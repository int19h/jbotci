use std::cell::Cell;
use std::collections::{BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[allow(unused_imports)]
use bityzba::{contract_trait, ensures, invariant, new, requires};
use serde_json::{Value, json};
use xarsnu::openrouter::ModelTurnData;
use xarsnu::protocol::{
    ListenerFlowAbandonReasonData, ProtocolEventData, ProtocolRunOutcomeData, TurnForfeitReasonData,
};
use xarsnu::{
    CapsConfig, ListenerMode, ModelTurn, ParticipantConfig, ProtocolEvent, ProtocolModel,
    ProtocolModelError, ProtocolRunner, ProtocolTool, ProviderToolChoice, ReferenceToolDispatcher,
    RunAccounting, RunConfig, RunHeader, ScenarioInstance, TaskStatus, TersmuFormat, ToolCall,
    ToolChoice, ToolDefinition, ToolDispatchError, ToolDispatcher, read_transcript, report_file,
};

const REFERENCE_TOOLS: [&str; 5] = ["vlacku", "gentufa", "tersmu", "jvozba", "cukta"];

#[requires(!name.trim().is_empty())]
#[ensures(ret.file_name().is_some())]
fn temp_path(name: &str) -> PathBuf {
    let executable = std::env::current_exe().expect("current test executable");
    let target_directory = executable
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .expect("Cargo target directory");
    let directory = target_directory.join("xarsnu-test-tmp");
    fs::create_dir_all(&directory).expect("create target temporary directory");
    directory.join(format!("xarsnu-{name}-{}.jsonl", std::process::id()))
}

#[invariant(!tool_name.trim().is_empty())]
#[invariant(arguments.is_object())]
#[invariant(expected_protocol_tools.iter().all(|name| !name.trim().is_empty()))]
#[derive(Debug)]
struct ScriptStep {
    expected_protocol_tools: Vec<&'static str>,
    reference_tools_expected: bool,
    tool_name: &'static str,
    arguments: Value,
}

#[invariant(!tool_name.trim().is_empty())]
#[invariant(!content.is_empty())]
#[derive(Debug)]
struct RecordedToolResult {
    tool_name: String,
    content: String,
}

#[invariant(!result.is_empty())]
#[derive(Debug, Clone)]
struct CountingDispatcher {
    calls: Rc<Cell<usize>>,
    result: String,
}

impl CountingDispatcher {
    #[requires(!result.is_empty())]
    #[ensures(ret.calls.get() == 0)]
    fn new(result: &str) -> Self {
        new!(CountingDispatcher {
            calls: Rc::new(Cell::new(0)),
            result: result.to_owned(),
        })
    }
}

#[contract_trait]
impl ToolDispatcher for CountingDispatcher {
    fn dispatch(&mut self, _call: &ToolCall) -> Result<String, ToolDispatchError> {
        self.calls.set(self.calls.get() + 1);
        Ok(self.result.clone())
    }
}

#[invariant(true, "constructed with a nonempty name and test-owned script")]
#[derive(Debug)]
struct ScriptedModel {
    name: String,
    tool_choice: ProviderToolChoice,
    supports_prefill: bool,
    max_tool_reprompts: usize,
    prose_responses: VecDeque<String>,
    steps: VecDeque<ScriptStep>,
    user_messages: Vec<String>,
    request_user_messages: Vec<Vec<String>>,
    request_tool_choices: Vec<ProviderToolChoice>,
    assistant_prefills: Vec<String>,
    tool_results: Vec<RecordedToolResult>,
    calls_made: usize,
}

impl ScriptedModel {
    #[requires(!name.trim().is_empty())]
    #[ensures(ret.name == name)]
    fn new(name: &str, steps: Vec<ScriptStep>) -> Self {
        Self {
            name: name.to_owned(),
            tool_choice: ProviderToolChoice::Required,
            supports_prefill: false,
            max_tool_reprompts: 0,
            prose_responses: VecDeque::new(),
            steps: steps.into(),
            user_messages: Vec::new(),
            request_user_messages: Vec::new(),
            request_tool_choices: Vec::new(),
            assistant_prefills: Vec::new(),
            tool_results: Vec::new(),
            calls_made: 0,
        }
    }

    #[requires(!name.trim().is_empty())]
    #[requires(prose_responses.iter().all(|content| !content.is_empty()))]
    #[ensures(ret.name == name)]
    fn auto(
        name: &str,
        max_tool_reprompts: usize,
        prose_responses: Vec<String>,
        steps: Vec<ScriptStep>,
    ) -> Self {
        Self {
            name: name.to_owned(),
            tool_choice: ProviderToolChoice::Auto,
            supports_prefill: false,
            max_tool_reprompts,
            prose_responses: prose_responses.into(),
            steps: steps.into(),
            user_messages: Vec::new(),
            request_user_messages: Vec::new(),
            request_tool_choices: Vec::new(),
            assistant_prefills: Vec::new(),
            tool_results: Vec::new(),
            calls_made: 0,
        }
    }

    #[requires(true)]
    #[ensures(ret == (self.steps.is_empty() && self.prose_responses.is_empty()))]
    fn is_complete(&self) -> bool {
        self.steps.is_empty() && self.prose_responses.is_empty()
    }

    #[requires(true)]
    #[ensures(ret.supports_prefill)]
    fn with_prefill(mut self) -> Self {
        self.supports_prefill = true;
        self
    }
}

#[contract_trait]
impl ProtocolModel for ScriptedModel {
    fn participant_name(&self) -> &str {
        &self.name
    }

    fn tool_choice(&self) -> ProviderToolChoice {
        self.tool_choice
    }

    fn max_tool_reprompts(&self) -> usize {
        self.max_tool_reprompts
    }

    fn push_user(&mut self, content: String) {
        self.user_messages.push(content);
    }

    fn push_tool_correction(&mut self, tools: &[ToolDefinition]) {
        if self.supports_prefill {
            let names = tools
                .iter()
                .map(ToolDefinition::name)
                .collect::<Vec<_>>()
                .join(", ");
            self.assistant_prefills.push(format!(
                "Actually, I must use one of the following tools: {names}."
            ));
        } else {
            self.user_messages.push(
                "You must respond by calling one of the provided tools. Do not answer with prose."
                    .to_owned(),
            );
        }
    }

    fn request(
        &mut self,
        tools: &[ToolDefinition],
        tool_choice: ProviderToolChoice,
        _accounting: &mut RunAccounting,
    ) -> Result<ModelTurn, ProtocolModelError> {
        assert_eq!(tool_choice, self.tool_choice);
        self.request_user_messages.push(self.user_messages.clone());
        self.request_tool_choices.push(tool_choice);
        self.calls_made += 1;
        if let Some(content) = self.prose_responses.pop_front() {
            return Ok(new!(ModelTurn::Message { content }));
        }
        let step = self
            .steps
            .pop_front()
            .unwrap_or_else(|| panic!("{} had no scripted response left", self.name));
        let bityzba::data!(ScriptStep {
            expected_protocol_tools,
            reference_tools_expected,
            tool_name,
            arguments,
        }) = step.into_data();
        let actual = tools
            .iter()
            .map(ToolDefinition::name)
            .collect::<BTreeSet<_>>();
        let expected = expected_protocol_tools
            .iter()
            .copied()
            .chain(
                reference_tools_expected
                    .then_some(REFERENCE_TOOLS)
                    .into_iter()
                    .flatten(),
            )
            .collect::<BTreeSet<_>>();
        assert_eq!(actual, expected, "dynamic tools for {}", self.name);
        Ok(new!(ModelTurn::ToolCalls {
            calls: vec![tool_call(self.calls_made, tool_name, arguments)],
        }))
    }

    fn push_tool_result(&mut self, call: &ToolCall, content: String) {
        self.tool_results.push(new!(RecordedToolResult {
            tool_name: call.function.name.clone(),
            content,
        }));
    }
}

#[requires(!expected_protocol_tools.is_empty())]
#[requires(!tool_name.trim().is_empty())]
#[requires(arguments.is_object())]
#[ensures(ret.tool_name == tool_name)]
fn step(
    expected_protocol_tools: &[&'static str],
    tool_name: &'static str,
    arguments: Value,
) -> ScriptStep {
    new!(ScriptStep {
        expected_protocol_tools: expected_protocol_tools.to_vec(),
        reference_tools_expected: true,
        tool_name,
        arguments,
    })
}

#[requires(!expected_protocol_tools.is_empty())]
#[requires(!tool_name.trim().is_empty())]
#[requires(arguments.is_object())]
#[ensures(ret.tool_name == tool_name)]
fn step_without_reference_tools(
    expected_protocol_tools: &[&'static str],
    tool_name: &'static str,
    arguments: Value,
) -> ScriptStep {
    new!(ScriptStep {
        expected_protocol_tools: expected_protocol_tools.to_vec(),
        reference_tools_expected: false,
        tool_name,
        arguments,
    })
}

#[requires(!meaning_en.trim().is_empty())]
#[requires(!paraphrase_en.trim().is_empty())]
#[ensures(ret.len() == 3)]
fn posted_message_steps(meaning_en: &str, paraphrase_en: &str) -> Vec<ScriptStep> {
    vec![
        step(
            &["register_intent"],
            "register_intent",
            json!({ "meaning_en": meaning_en }),
        ),
        step(
            &["register_intent", "submit_lojban"],
            "submit_lojban",
            json!({ "text": "mi klama" }),
        ),
        step(
            &["register_intent", "confirm_meaning"],
            "confirm_meaning",
            json!({ "matches": true, "paraphrase_en": paraphrase_en }),
        ),
    ]
}

#[requires(!interpretation_en.trim().is_empty())]
#[ensures(ret.len() == 2)]
fn listener_steps(interpretation_en: &str) -> Vec<ScriptStep> {
    vec![
        step(
            &["interpret_blind"],
            "interpret_blind",
            json!({ "interpretation_en": interpretation_en }),
        ),
        step(
            &["acknowledge"],
            "acknowledge",
            json!({ "final_understanding_en": interpretation_en }),
        ),
    ]
}

#[requires(!understanding_en.trim().is_empty())]
#[ensures(ret.len() == 1)]
fn informed_listener_steps(understanding_en: &str) -> Vec<ScriptStep> {
    vec![step(
        &["acknowledge"],
        "acknowledge",
        json!({ "final_understanding_en": understanding_en }),
    )]
}

#[requires(id > 0)]
#[requires(!name.trim().is_empty())]
#[requires(arguments.is_object())]
#[ensures(ret.function.name == name)]
fn tool_call(id: usize, name: &str, arguments: Value) -> ToolCall {
    serde_json::from_value(json!({
        "id": format!("call-{id}-{name}"),
        "type": "function",
        "function": {
            "name": name,
            "arguments": arguments.to_string(),
        },
    }))
    .expect("valid scripted tool call")
}

#[requires(max_parse_attempts > 0)]
#[requires(max_intent_revisions > 0)]
#[requires(max_turns > 0)]
#[ensures(ret.max_turns == max_turns)]
fn caps(max_parse_attempts: usize, max_intent_revisions: usize, max_turns: usize) -> CapsConfig {
    new!(CapsConfig {
        max_parse_attempts_per_turn: max_parse_attempts,
        max_intent_revisions_per_turn: max_intent_revisions,
        max_turns,
        max_cost_usd: 10.0,
        max_reference_calls_per_phase: 16,
        reference_dedupe: true,
        reference_nudge_after: 6,
    })
}

#[requires(participants.len() >= 2)]
#[ensures(ret.as_ref().is_ok_and(|runner| runner.participants().len() >= 2) || ret.is_err())]
fn runner(
    participants: Vec<ScriptedModel>,
    caps: CapsConfig,
) -> Result<ProtocolRunner<ScriptedModel, ReferenceToolDispatcher>, xarsnu::ProtocolRunError> {
    ProtocolRunner::new(
        participants,
        caps,
        ListenerMode::BlindThenReveal,
        TersmuFormat::Smusni,
        ReferenceToolDispatcher,
    )
}

#[requires(true)]
#[ensures(ret <= events.len())]
fn count_protocol_errors(events: &[ProtocolEvent], tool: ProtocolTool) -> usize {
    events
        .iter()
        .filter(|event| {
            matches!(
                event.as_data(),
                bityzba::data!(ProtocolEvent::ProtocolError { tool_name, .. })
                    if tool_name == tool.name()
            )
        })
        .count()
}

#[requires(true)]
#[ensures(ret <= events.len())]
fn count_rejections(events: &[ProtocolEvent]) -> usize {
    events
        .iter()
        .filter(|event| {
            matches!(
                event.as_data(),
                bityzba::data!(ProtocolEvent::CandidateRejected { .. })
            )
        })
        .count()
}

#[test]
#[requires(true)]
#[ensures(true)]
fn reference_budget_withdraws_tools_on_the_wire_and_preserves_turn_forfeit() {
    let speaker = ScriptedModel::new(
        "alice",
        vec![
            step(
                &["register_intent"],
                "register_intent",
                json!({ "meaning_en": "I go." }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "vlacku",
                json!({ "word": "klama" }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "vlacku",
                json!({ "word": "cadzu" }),
            ),
            // Anti-no-op: this request asserts that all reference definitions
            // disappeared while the legal protocol tools remained.
            step_without_reference_tools(
                &["register_intent", "submit_lojban"],
                "submit_lojban",
                json!({ "text": "mi cu" }),
            ),
        ],
    );
    let listener = ScriptedModel::new("bob", Vec::new());
    let caps = caps(1, 1, 1).with_data(bityzba::data! {
        max_reference_calls_per_phase: 2,
        reference_nudge_after: 1,
    });
    let dispatcher = CountingDispatcher::new("reference payload\n");
    let invocation_count = dispatcher.calls.clone();
    let mut runner = ProtocolRunner::new(
        vec![speaker, listener],
        caps,
        ListenerMode::BlindThenReveal,
        TersmuFormat::Smusni,
        dispatcher,
    )
    .expect("valid runner");

    runner.run().expect("bounded reference run");

    assert_eq!(invocation_count.get(), 2);
    assert_eq!(
        runner
            .events()
            .iter()
            .filter(|event| matches!(
                event.as_data(),
                bityzba::data!(ProtocolEvent::ReferenceCallBudgetExhausted { maximum: 2, .. })
            ))
            .count(),
        1
    );
    assert!(runner.events().iter().any(|event| matches!(
        event.as_data(),
        bityzba::data!(ProtocolEvent::TurnForfeited {
            reason,
            ..
        }) if matches!(reason.as_data(), bityzba::data!(TurnForfeitReason::ParseAttempts { maximum: 1 }))
    )));
    let final_request = runner.participants()[0]
        .request_user_messages
        .last()
        .expect("post-budget request captured");
    assert!(final_request.iter().any(|message| {
        message.contains("reference-call budget for this phase is spent (2/2)")
            && message.contains("composition or interpretation must proceed")
    }));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn duplicate_reference_lookup_reuses_prior_payload_byte_for_byte() {
    let arguments = json!({ "word": "klama" });
    let speaker = ScriptedModel::new(
        "alice",
        vec![
            step(
                &["register_intent"],
                "register_intent",
                json!({ "meaning_en": "I go." }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "vlacku",
                arguments.clone(),
            ),
            step(&["register_intent", "submit_lojban"], "vlacku", arguments),
            step(
                &["register_intent", "submit_lojban"],
                "submit_lojban",
                json!({ "text": "mi cu" }),
            ),
        ],
    );
    let caps = caps(1, 1, 1).with_data(bityzba::data! {
        max_reference_calls_per_phase: 4,
        reference_nudge_after: 3,
    });
    let dispatcher = CountingDispatcher::new("first line\nsecond line\n");
    let invocation_count = dispatcher.calls.clone();
    let mut runner = ProtocolRunner::new(
        vec![speaker, ScriptedModel::new("bob", Vec::new())],
        caps,
        ListenerMode::BlindThenReveal,
        TersmuFormat::Smusni,
        dispatcher,
    )
    .expect("valid runner");

    runner.run().expect("memoized reference run");

    assert_eq!(invocation_count.get(), 1, "repeat must not invoke jbotci");
    let results = runner.participants()[0]
        .tool_results
        .iter()
        .filter(|result| result.tool_name == "vlacku")
        .collect::<Vec<_>>();
    assert_eq!(results.len(), 2);
    let expected_note = "(repeat lookup #2 of this exact query this phase — the result has not changed; you have 2 reference calls left)";
    let repeated_payload = results[1]
        .content
        .strip_prefix(expected_note)
        .and_then(|content| content.strip_prefix('\n'))
        .expect("exact corrective note prefix");
    assert_eq!(
        repeated_payload.as_bytes(),
        results[0].content.as_bytes(),
        "stripping only the note line must recover the prior bytes"
    );
    assert!(runner.events().iter().any(|event| matches!(
        event.as_data(),
        bityzba::data!(ProtocolEvent::ReferenceLookupRepeated {
            repeat_number: 2,
            remaining_calls: 2,
            ..
        })
    )));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn disabling_reference_dedupe_reexecutes_exact_repeats() {
    let arguments = json!({ "word": "klama" });
    let speaker = ScriptedModel::new(
        "alice",
        vec![
            step(
                &["register_intent"],
                "register_intent",
                json!({ "meaning_en": "I go." }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "vlacku",
                arguments.clone(),
            ),
            step(&["register_intent", "submit_lojban"], "vlacku", arguments),
            step(
                &["register_intent", "submit_lojban"],
                "submit_lojban",
                json!({ "text": "mi cu" }),
            ),
        ],
    );
    let caps = caps(1, 1, 1).with_data(bityzba::data! {
        max_reference_calls_per_phase: 4,
        reference_dedupe: false,
        reference_nudge_after: 3,
    });
    let dispatcher = CountingDispatcher::new("reference payload");
    let invocation_count = dispatcher.calls.clone();
    let mut runner = ProtocolRunner::new(
        vec![speaker, ScriptedModel::new("bob", Vec::new())],
        caps,
        ListenerMode::BlindThenReveal,
        TersmuFormat::Smusni,
        dispatcher,
    )
    .expect("valid runner");

    runner.run().expect("non-deduped reference run");

    assert_eq!(invocation_count.get(), 2);
    assert!(!runner.events().iter().any(|event| matches!(
        event.as_data(),
        bityzba::data!(ProtocolEvent::ReferenceLookupRepeated { .. })
    )));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn idle_reference_nudge_fires_once_at_threshold_in_each_phase() {
    let speaker = ScriptedModel::new(
        "alice",
        vec![
            step(&["register_intent"], "vlacku", json!({ "word": "pa" })),
            step(&["register_intent"], "vlacku", json!({ "word": "re" })),
            step(
                &["register_intent"],
                "register_intent",
                json!({ "meaning_en": "I go." }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "vlacku",
                json!({ "word": "ci" }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "vlacku",
                json!({ "word": "vo" }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "vlacku",
                json!({ "word": "mu" }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "submit_lojban",
                json!({ "text": "mi cu" }),
            ),
        ],
    );
    let caps = caps(1, 1, 1).with_data(bityzba::data! {
        max_reference_calls_per_phase: 5,
        reference_nudge_after: 2,
    });
    let dispatcher = CountingDispatcher::new("reference payload");
    let mut runner = ProtocolRunner::new(
        vec![speaker, ScriptedModel::new("bob", Vec::new())],
        caps,
        ListenerMode::BlindThenReveal,
        TersmuFormat::Smusni,
        dispatcher,
    )
    .expect("valid runner");

    runner.run().expect("nudged reference run");

    let nudges = runner
        .events()
        .iter()
        .filter_map(|event| match event.as_data() {
            bityzba::data!(ProtocolEvent::ReferenceResearchNudge {
                phase,
                consecutive_calls,
                message,
                ..
            }) => Some((*phase, *consecutive_calls, message)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(nudges.len(), 2);
    assert_eq!(nudges[0].1, 2);
    assert_eq!(nudges[1].1, 2);
    assert_ne!(nudges[0].0, nudges[1].0, "nudge allowance resets by phase");
    for (_, _, message) in &nudges {
        assert!(message.contains("Current phase protocol tools:"));
        assert!(message.contains("Current phase intent:"));
    }

    // Anti-no-op: each corrective message was present before a later request,
    // not merely appended after the scripted run ended.
    for (_, _, message) in nudges {
        assert!(
            runner.participants()[0]
                .request_user_messages
                .iter()
                .any(|request| request.contains(message))
        );
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn happy_path_posts_then_completes_two_blind_listener_flows() {
    let speaker = ScriptedModel::new(
        "alice",
        vec![
            step(
                &["register_intent"],
                "register_intent",
                json!({ "meaning_en": "I am going." }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "submit_lojban",
                json!({ "text": "mi klama" }),
            ),
            step(
                &["register_intent", "confirm_meaning"],
                "confirm_meaning",
                json!({
                    "matches": true,
                    "paraphrase_en": "I go."
                }),
            ),
        ],
    );
    let listener_steps = || {
        vec![
            step(
                &["interpret_blind"],
                "interpret_blind",
                json!({ "interpretation_en": "The speaker goes." }),
            ),
            step(
                &["acknowledge"],
                "acknowledge",
                json!({ "final_understanding_en": "The speaker goes." }),
            ),
        ]
    };
    let mut runner = runner(
        vec![
            speaker,
            ScriptedModel::new("bob", listener_steps()),
            ScriptedModel::new("carol", listener_steps()),
        ],
        caps(3, 2, 1),
    )
    .expect("valid runner");

    runner.run().expect("happy protocol run");

    assert_eq!(runner.visible_chat().len(), 1);
    assert!(runner.participants().iter().all(|participant| {
        participant
            .request_tool_choices
            .iter()
            .all(|choice| *choice == ProviderToolChoice::Required)
    }));
    assert!(runner.participants().iter().all(|participant| {
        participant
            .user_messages
            .iter()
            .all(|message| message != "You must respond by calling one of the provided tools. Do not answer with prose.")
    }));
    assert!(!runner.events().iter().any(|event| matches!(
        event.as_data(),
        bityzba::data!(ProtocolEvent::ProseRejected { .. })
    )));
    let posted = &runner.visible_chat()[0];
    assert_eq!(posted.text, "mi klama");
    assert!(!posted.tersmu_rendering.is_empty());
    let blind_events = runner
        .events()
        .iter()
        .filter(|event| {
            matches!(
                event.as_data(),
                bityzba::data!(ProtocolEvent::BlindInterpretationRecorded { .. })
            )
        })
        .count();
    let acknowledgements = runner
        .events()
        .iter()
        .filter(|event| {
            matches!(
                event.as_data(),
                bityzba::data!(ProtocolEvent::Acknowledged { .. })
            )
        })
        .count();
    assert_eq!(blind_events, 2);
    assert_eq!(acknowledgements, 2);
    assert!(runner.participants().iter().all(ScriptedModel::is_complete));

    // Anti-no-op check 3: the first listener prompt is built from BlindMessage,
    // and the rendering appears only in the later reveal prompt.
    let rendering = std::str::from_utf8(&posted.tersmu_rendering).expect("UTF-8 tersmu");
    for listener in &runner.participants()[1..] {
        assert_eq!(listener.user_messages.len(), 2);
        assert!(!listener.user_messages[0].contains(rendering));
        assert!(listener.user_messages[1].contains(rendering));
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn informed_listener_receives_rendering_from_the_start_and_acknowledges_once() {
    let speaker = ScriptedModel::new("alice", posted_message_steps("I am going.", "I go."));
    let listener = ScriptedModel::new("bob", informed_listener_steps("Alice goes."));
    let mut runner = ProtocolRunner::new(
        vec![speaker, listener],
        caps(3, 2, 1),
        ListenerMode::Informed,
        TersmuFormat::Smusni,
        ReferenceToolDispatcher,
    )
    .expect("valid informed runner");

    runner.run().expect("informed listener run");

    let posted = &runner.visible_chat()[0];
    let rendering = std::str::from_utf8(&posted.tersmu_rendering).expect("UTF-8 tersmu");
    let listener = &runner.participants()[1];
    assert!(listener.is_complete());
    assert_eq!(listener.calls_made, 1);
    assert_eq!(listener.user_messages.len(), 1);
    assert!(listener.user_messages[0].contains("mi klama"));
    assert!(listener.user_messages[0].contains(rendering));
    assert!(
        listener.user_messages[0]
            .contains("tersmu rendering and definitions available from the start")
    );
    assert!(runner.events().iter().any(|event| matches!(
        event.as_data(),
        bityzba::data!(ProtocolEvent::ListenerFlowStarted {
            listener,
            mode: ListenerMode::Informed,
            message,
            ..
        }) if listener == "bob" && message == posted
    )));
    assert!(!runner.events().iter().any(|event| matches!(
        event.as_data(),
        bityzba::data!(ProtocolEvent::BlindInterpretationRecorded { .. })
            | bityzba::data!(ProtocolEvent::TersmuRevealed { .. })
    )));
    assert_eq!(
        runner
            .events()
            .iter()
            .filter(|event| matches!(
                event.as_data(),
                bityzba::data!(ProtocolEvent::Acknowledged { listener, .. })
                    if listener == "bob"
            ))
            .count(),
        1
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn automatic_tool_choice_rejects_prose_then_recovers_within_the_existing_cap() {
    let speaker = ScriptedModel::auto(
        "alice",
        2,
        vec![
            "I will explain first.".to_owned(),
            "One more prose answer.".to_owned(),
        ],
        vec![
            step(
                &["register_intent"],
                "register_intent",
                json!({ "meaning_en": "I go." }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "submit_lojban",
                json!({ "text": "mi klama" }),
            ),
            step(
                &["register_intent", "confirm_meaning"],
                "confirm_meaning",
                json!({ "matches": true, "paraphrase_en": "I go." }),
            ),
        ],
    )
    .with_prefill();
    let listener = ScriptedModel::new(
        "bob",
        vec![
            step(
                &["interpret_blind"],
                "interpret_blind",
                json!({ "interpretation_en": "Alice goes." }),
            ),
            step(
                &["acknowledge"],
                "acknowledge",
                json!({ "final_understanding_en": "Alice goes." }),
            ),
        ],
    );
    let mut runner = runner(vec![speaker, listener], caps(3, 2, 1)).expect("valid runner");

    runner.run().expect("automatic prose recovery run");

    assert_eq!(runner.visible_chat().len(), 1);
    assert!(runner.participants().iter().all(ScriptedModel::is_complete));
    let prose_rejections = runner
        .events()
        .iter()
        .filter_map(|event| match event.as_data() {
            bityzba::data!(ProtocolEvent::ProseRejected {
                participant,
                attempt,
                maximum_attempts,
                ..
            }) => Some((participant.as_str(), *attempt, *maximum_attempts)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(prose_rejections, [("alice", 1, 3), ("alice", 2, 3)]);
    let alice = &runner.participants()[0];
    assert!(
        alice
            .request_tool_choices
            .iter()
            .all(|choice| *choice == ProviderToolChoice::Auto)
    );
    assert_eq!(
        alice.assistant_prefills,
        [
            "Actually, I must use one of the following tools: register_intent, vlacku, gentufa, tersmu, jvozba, cukta.",
            "Actually, I must use one of the following tools: register_intent, vlacku, gentufa, tersmu, jvozba, cukta.",
        ]
    );
    assert!(
        runner.participants()[1]
            .request_tool_choices
            .iter()
            .all(|choice| *choice == ProviderToolChoice::Required)
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn automatic_listener_rejects_prose_then_completes_the_blind_flow() {
    let speaker = ScriptedModel::new(
        "alice",
        vec![
            step(
                &["register_intent"],
                "register_intent",
                json!({ "meaning_en": "I go." }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "submit_lojban",
                json!({ "text": "mi klama" }),
            ),
            step(
                &["register_intent", "confirm_meaning"],
                "confirm_meaning",
                json!({ "matches": true, "paraphrase_en": "I go." }),
            ),
        ],
    );
    let listener = ScriptedModel::auto(
        "bob",
        2,
        vec![
            "The sentence probably means going.".to_owned(),
            "I should explain my interpretation.".to_owned(),
        ],
        vec![
            step(
                &["interpret_blind"],
                "interpret_blind",
                json!({ "interpretation_en": "Alice goes." }),
            ),
            step(
                &["acknowledge"],
                "acknowledge",
                json!({ "final_understanding_en": "Alice goes." }),
            ),
        ],
    );
    let mut runner = runner(vec![speaker, listener], caps(3, 2, 1)).expect("valid runner");

    runner.run().expect("automatic listener recovery run");

    assert_eq!(runner.visible_chat().len(), 1);
    assert!(runner.participants().iter().all(ScriptedModel::is_complete));
    let prose_rejections = runner
        .events()
        .iter()
        .filter_map(|event| match event.as_data() {
            bityzba::data!(ProtocolEvent::ProseRejected {
                participant,
                attempt,
                maximum_attempts,
                ..
            }) => Some((participant.as_str(), *attempt, *maximum_attempts)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(prose_rejections, [("bob", 1, 3), ("bob", 2, 3)]);
    let bob = &runner.participants()[1];
    let correction =
        "You must respond by calling one of the provided tools. Do not answer with prose.";
    assert!(bob.assistant_prefills.is_empty());
    assert_eq!(
        bob.user_messages
            .iter()
            .filter(|message| message.as_str() == correction)
            .count(),
        2
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn automatic_listener_exhaustion_abandons_only_that_listener_flow() {
    let speaker = ScriptedModel::new(
        "alice",
        vec![
            step(
                &["register_intent"],
                "register_intent",
                json!({ "meaning_en": "I go." }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "submit_lojban",
                json!({ "text": "mi klama" }),
            ),
            step(
                &["register_intent", "confirm_meaning"],
                "confirm_meaning",
                json!({ "matches": true, "paraphrase_en": "I go." }),
            ),
        ],
    );
    let abandoned = ScriptedModel::auto(
        "bob",
        1,
        vec!["first prose".to_owned(), "second prose".to_owned()],
        Vec::new(),
    );
    let continuing = ScriptedModel::new(
        "carol",
        vec![
            step(
                &["interpret_blind"],
                "interpret_blind",
                json!({ "interpretation_en": "Alice goes." }),
            ),
            step(
                &["acknowledge"],
                "acknowledge",
                json!({ "final_understanding_en": "Alice goes." }),
            ),
        ],
    );
    let mut runner =
        runner(vec![speaker, abandoned, continuing], caps(3, 2, 1)).expect("valid runner");

    runner.run().expect("bounded listener abandonment run");

    assert_eq!(runner.visible_chat().len(), 1);
    assert!(runner.participants().iter().all(ScriptedModel::is_complete));
    assert!(runner.events().iter().any(|event| matches!(
        event.as_data(),
        bityzba::data!(ProtocolEvent::ListenerFlowAbandoned {
            listener,
            reason,
            ..
        }) if listener == "bob" && matches!(
            reason.as_data(),
            bityzba::data!(ListenerFlowAbandonReason::ProtocolProseResponses {
                maximum_attempts: 2,
            })
        )
    )));
    assert!(!runner.events().iter().any(|event| matches!(
        event.as_data(),
        bityzba::data!(ProtocolEvent::TurnForfeited { .. })
    )));
    assert!(runner.events().iter().any(|event| matches!(
        event.as_data(),
        bityzba::data!(ProtocolEvent::Acknowledged { listener, .. }) if listener == "carol"
    )));
    assert!(!runner.events().iter().any(|event| matches!(
        event.as_data(),
        bityzba::data!(ProtocolEvent::BlindInterpretationRecorded { listener, .. })
            if listener == "bob"
    )));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn automatic_prose_exhaustion_forfeits_the_turn() {
    let speaker = ScriptedModel::auto(
        "alice",
        1,
        vec!["first prose".to_owned(), "second prose".to_owned()],
        Vec::new(),
    );
    let mut runner = runner(
        vec![speaker, ScriptedModel::new("bob", Vec::new())],
        caps(3, 2, 1),
    )
    .expect("valid runner");

    runner.run().expect("bounded automatic prose run");

    assert!(runner.visible_chat().is_empty());
    assert!(runner.participants().iter().all(ScriptedModel::is_complete));
    assert_eq!(
        runner
            .events()
            .iter()
            .filter(|event| matches!(
                event.as_data(),
                bityzba::data!(ProtocolEvent::ProseRejected { .. })
            ))
            .count(),
        2
    );
    assert!(runner.events().iter().any(|event| matches!(
        event.as_data(),
        bityzba::data!(ProtocolEvent::TurnForfeited { reason, .. })
            if matches!(
                reason.as_data(),
                bityzba::data!(TurnForfeitReason::ProtocolProseResponses {
                    maximum_attempts: 2,
                })
            )
    )));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn parse_failures_keep_composing_and_record_diagnostics_before_success() {
    let speaker = ScriptedModel::new(
        "alice",
        vec![
            step(
                &["register_intent"],
                "register_intent",
                json!({ "meaning_en": "I am going." }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "submit_lojban",
                json!({ "text": "mi cu" }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "submit_lojban",
                json!({ "text": "mi @ klama" }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "submit_lojban",
                json!({ "text": "mi klama" }),
            ),
            step(
                &["register_intent", "confirm_meaning"],
                "confirm_meaning",
                json!({ "matches": true, "paraphrase_en": "I go." }),
            ),
        ],
    );
    let listener = ScriptedModel::new(
        "bob",
        vec![
            step(
                &["interpret_blind"],
                "interpret_blind",
                json!({ "interpretation_en": "Alice goes." }),
            ),
            step(
                &["acknowledge"],
                "acknowledge",
                json!({ "final_understanding_en": "Alice goes." }),
            ),
        ],
    );
    let mut runner = runner(vec![speaker, listener], caps(4, 2, 1)).expect("valid runner");

    runner.run().expect("parse recovery run");

    assert_eq!(count_rejections(runner.events()), 2);
    assert_eq!(runner.visible_chat().len(), 1);
    let rejected = runner
        .events()
        .iter()
        .filter_map(|event| match event.as_data() {
            bityzba::data!(ProtocolEvent::CandidateRejected {
                diagnostics,
                attempt,
                ..
            }) => Some((diagnostics, *attempt)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(rejected.len(), 2);
    assert_eq!(rejected[0].1, 1);
    assert_eq!(rejected[1].1, 2);
    assert!(
        rejected
            .iter()
            .all(|(diagnostics, _)| !diagnostics.is_empty())
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn confirmation_mismatch_returns_to_composing_then_posts_new_candidate() {
    let speaker = ScriptedModel::new(
        "alice",
        vec![
            step(
                &["register_intent"],
                "register_intent",
                json!({ "meaning_en": "I am going." }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "submit_lojban",
                json!({ "text": "mi klama" }),
            ),
            step(
                &["register_intent", "confirm_meaning"],
                "confirm_meaning",
                json!({
                    "matches": false,
                    "paraphrase_en": "I travel.",
                    "discrepancies": "The destination is underspecified."
                }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "submit_lojban",
                json!({ "text": "mi klama" }),
            ),
            step(
                &["register_intent", "confirm_meaning"],
                "confirm_meaning",
                json!({ "matches": true, "paraphrase_en": "I go." }),
            ),
        ],
    );
    let listener = ScriptedModel::new(
        "bob",
        vec![
            step(
                &["interpret_blind"],
                "interpret_blind",
                json!({ "interpretation_en": "Alice goes." }),
            ),
            step(
                &["acknowledge"],
                "acknowledge",
                json!({ "final_understanding_en": "Alice goes." }),
            ),
        ],
    );
    let mut runner = runner(vec![speaker, listener], caps(3, 2, 1)).expect("valid runner");

    runner.run().expect("mismatch recovery run");

    let confirmations = runner
        .events()
        .iter()
        .filter_map(|event| match event.as_data() {
            bityzba::data!(ProtocolEvent::MeaningConfirmed { matches, .. }) => Some(*matches),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(confirmations, [false, true]);
    assert_eq!(runner.visible_chat().len(), 1);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn intent_revision_is_recorded_without_resetting_the_turn() {
    let speaker = ScriptedModel::new(
        "alice",
        vec![
            step(
                &["register_intent"],
                "register_intent",
                json!({ "meaning_en": "I travel." }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "register_intent",
                json!({ "meaning_en": "I go." }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "submit_lojban",
                json!({ "text": "mi klama" }),
            ),
            step(
                &["register_intent", "confirm_meaning"],
                "confirm_meaning",
                json!({ "matches": true, "paraphrase_en": "I go." }),
            ),
        ],
    );
    let listener = ScriptedModel::new(
        "bob",
        vec![
            step(
                &["interpret_blind"],
                "interpret_blind",
                json!({ "interpretation_en": "Alice goes." }),
            ),
            step(
                &["acknowledge"],
                "acknowledge",
                json!({ "final_understanding_en": "Alice goes." }),
            ),
        ],
    );
    let mut runner = runner(vec![speaker, listener], caps(3, 2, 1)).expect("valid runner");

    runner.run().expect("revision run");

    let registrations = runner
        .events()
        .iter()
        .filter_map(|event| match event.as_data() {
            bityzba::data!(ProtocolEvent::IntentRegistered {
                revision,
                revision_number,
                ..
            }) => Some((*revision, *revision_number)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(registrations, [(false, 0), (true, 1)]);
    assert_eq!(runner.visible_chat().len(), 1);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn submit_without_registered_intent_is_rejected_before_gate() {
    let speaker = ScriptedModel::new(
        "alice",
        vec![
            // Deliberately call an unoffered tool to model a provider ignoring
            // the tools array. This is anti-no-op check 2.
            step(
                &["register_intent"],
                "submit_lojban",
                json!({ "text": "mi klama" }),
            ),
            step(
                &["register_intent"],
                "register_intent",
                json!({ "meaning_en": "I go." }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "submit_lojban",
                json!({ "text": "mi klama" }),
            ),
            step(
                &["register_intent", "confirm_meaning"],
                "confirm_meaning",
                json!({ "matches": true, "paraphrase_en": "I go." }),
            ),
        ],
    );
    let listener = ScriptedModel::new(
        "bob",
        vec![
            step(
                &["interpret_blind"],
                "interpret_blind",
                json!({ "interpretation_en": "Alice goes." }),
            ),
            step(
                &["acknowledge"],
                "acknowledge",
                json!({ "final_understanding_en": "Alice goes." }),
            ),
        ],
    );
    let mut runner = runner(vec![speaker, listener], caps(3, 2, 1)).expect("valid runner");

    runner.run().expect("illegal submit recovery run");

    assert_eq!(
        count_protocol_errors(runner.events(), ProtocolTool::SubmitLojban),
        1
    );
    assert!(runner.participants()[0].tool_results.iter().any(|result| {
        result.tool_name == "submit_lojban" && result.content.contains("is not available")
    }));
    let submissions = runner
        .events()
        .iter()
        .filter(|event| {
            matches!(
                event.as_data(),
                bityzba::data!(ProtocolEvent::CandidateSubmitted { .. })
            )
        })
        .count();
    assert_eq!(submissions, 1, "illegal submit must not reach the gate");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn confirmation_without_current_success_is_rejected_before_posting() {
    let speaker = ScriptedModel::new(
        "alice",
        vec![
            step(
                &["register_intent"],
                "register_intent",
                json!({ "meaning_en": "I go." }),
            ),
            // Deliberately call an unoffered tool while Composing. This is
            // anti-no-op check 1.
            step(
                &["register_intent", "submit_lojban"],
                "confirm_meaning",
                json!({ "matches": true, "paraphrase_en": "I go." }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "submit_lojban",
                json!({ "text": "mi klama" }),
            ),
            step(
                &["register_intent", "confirm_meaning"],
                "confirm_meaning",
                json!({ "matches": true, "paraphrase_en": "I go." }),
            ),
        ],
    );
    let listener = ScriptedModel::new(
        "bob",
        vec![
            step(
                &["interpret_blind"],
                "interpret_blind",
                json!({ "interpretation_en": "Alice goes." }),
            ),
            step(
                &["acknowledge"],
                "acknowledge",
                json!({ "final_understanding_en": "Alice goes." }),
            ),
        ],
    );
    let mut runner = runner(vec![speaker, listener], caps(3, 2, 1)).expect("valid runner");

    runner.run().expect("illegal confirmation recovery run");

    assert_eq!(
        count_protocol_errors(runner.events(), ProtocolTool::ConfirmMeaning),
        1
    );
    assert!(runner.participants()[0].tool_results.iter().any(|result| {
        result.tool_name == "confirm_meaning" && result.content.contains("is not available")
    }));
    let accepted_index = runner
        .events()
        .iter()
        .position(|event| {
            matches!(
                event.as_data(),
                bityzba::data!(ProtocolEvent::CandidateAccepted { .. })
            )
        })
        .expect("legal candidate accepted");
    let posted_index = runner
        .events()
        .iter()
        .position(|event| {
            matches!(
                event.as_data(),
                bityzba::data!(ProtocolEvent::MessagePosted { .. })
            )
        })
        .expect("message posted");
    assert!(accepted_index < posted_index);
    assert_eq!(runner.visible_chat().len(), 1);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn parse_attempt_cap_forfeits_turn_and_does_not_start_listeners() {
    let speaker = ScriptedModel::new(
        "alice",
        vec![
            step(
                &["register_intent"],
                "register_intent",
                json!({ "meaning_en": "I go." }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "submit_lojban",
                json!({ "text": "mi cu" }),
            ),
        ],
    );
    let listener = ScriptedModel::new("bob", Vec::new());
    let mut runner = runner(vec![speaker, listener], caps(1, 1, 1)).expect("valid runner");

    runner.run().expect("parse-cap run");

    assert!(runner.visible_chat().is_empty());
    assert!(runner.participants().iter().all(ScriptedModel::is_complete));
    assert!(runner.events().iter().any(|event| {
        matches!(
            event.as_data(),
            bityzba::data!(ProtocolEvent::TurnForfeited { reason, .. })
                if matches!(
                    reason.as_data(),
                    bityzba::data!(TurnForfeitReason::ParseAttempts { maximum: 1 })
                )
        )
    }));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn intent_revision_cap_forfeits_only_when_an_extra_revision_is_attempted() {
    let speaker = ScriptedModel::new(
        "alice",
        vec![
            step(
                &["register_intent"],
                "register_intent",
                json!({ "meaning_en": "First intent." }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "register_intent",
                json!({ "meaning_en": "Allowed revision." }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "register_intent",
                json!({ "meaning_en": "One revision too many." }),
            ),
        ],
    );
    let listener = ScriptedModel::new("bob", Vec::new());
    let mut runner = runner(vec![speaker, listener], caps(2, 1, 1)).expect("valid runner");

    runner.run().expect("revision-cap run");

    assert!(runner.visible_chat().is_empty());
    assert!(runner.events().iter().any(|event| {
        matches!(
            event.as_data(),
            bityzba::data!(ProtocolEvent::TurnForfeited { reason, .. })
                if matches!(
                    reason.as_data(),
                    bityzba::data!(TurnForfeitReason::IntentRevisions { maximum: 1 })
                )
        )
    }));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn round_robin_starts_next_speaker_only_after_listener_acknowledges() {
    let alice = ScriptedModel::new(
        "alice",
        vec![
            step(
                &["register_intent"],
                "register_intent",
                json!({ "meaning_en": "I go." }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "submit_lojban",
                json!({ "text": "mi klama" }),
            ),
            step(
                &["register_intent", "confirm_meaning"],
                "confirm_meaning",
                json!({ "matches": true, "paraphrase_en": "I go." }),
            ),
            step(
                &["interpret_blind"],
                "interpret_blind",
                json!({ "interpretation_en": "Bob goes." }),
            ),
            step(
                &["acknowledge"],
                "acknowledge",
                json!({ "final_understanding_en": "Bob goes." }),
            ),
        ],
    );
    let bob = ScriptedModel::new(
        "bob",
        vec![
            step(
                &["interpret_blind"],
                "interpret_blind",
                json!({ "interpretation_en": "Alice goes." }),
            ),
            step(
                &["acknowledge"],
                "acknowledge",
                json!({ "final_understanding_en": "Alice goes." }),
            ),
            step(
                &["register_intent"],
                "register_intent",
                json!({ "meaning_en": "I go too." }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "submit_lojban",
                json!({ "text": "mi ji'a klama" }),
            ),
            step(
                &["register_intent", "confirm_meaning"],
                "confirm_meaning",
                json!({ "matches": true, "paraphrase_en": "I also go." }),
            ),
        ],
    );
    let mut runner = runner(vec![alice, bob], caps(3, 2, 2)).expect("valid runner");

    runner.run().expect("two-turn round robin");

    assert_eq!(runner.visible_chat().len(), 2);
    let events = runner.events();
    let first_ack = events
        .iter()
        .position(|event| {
            matches!(
                event.as_data(),
                bityzba::data!(ProtocolEvent::Acknowledged { turn_number: 1, .. })
            )
        })
        .expect("turn-one acknowledgement");
    let second_turn = events
        .iter()
        .position(|event| {
            matches!(
                event.as_data(),
                bityzba::data!(ProtocolEvent::TurnStarted {
                    turn_number: 2,
                    speaker,
                }) if speaker == "bob"
            )
        })
        .expect("bob starts turn two");
    assert!(first_ack < second_turn);
    assert!(runner.participants().iter().all(ScriptedModel::is_complete));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn referential_answers_close_dialog_and_are_collected_from_one_frozen_boundary() {
    let scenario =
        ScenarioInstance::from_toml(include_str!("../scenarios/referential-game-1.toml"))
            .expect("referential scenario");
    assert!(scenario.answers_close_dialog());

    let mut speaker_steps = posted_message_steps("I describe the target.", "I go.");
    speaker_steps.extend(listener_steps("Listener A goes."));
    speaker_steps.extend(listener_steps("Listener B goes."));

    let mut listener_a_steps = listener_steps("The speaker goes.");
    listener_a_steps.extend(posted_message_steps("I refine the description.", "I go."));
    listener_a_steps.extend(listener_steps("Listener B goes."));
    listener_a_steps.push(step_without_reference_tools(
        &["submit_answer"],
        "submit_answer",
        json!({ "scene_index": 1 }),
    ));

    let mut listener_b_steps = listener_steps("The speaker goes.");
    listener_b_steps.extend(listener_steps("Listener A goes."));
    listener_b_steps.extend(posted_message_steps(
        "I give the final description.",
        "I go.",
    ));
    listener_b_steps.push(step_without_reference_tools(
        &["submit_answer"],
        "submit_answer",
        json!({ "scene_index": 1 }),
    ));

    let mut runner = ProtocolRunner::new_with_scenario(
        vec![
            ScriptedModel::new("speaker", speaker_steps),
            ScriptedModel::new("listener-a", listener_a_steps),
            ScriptedModel::new("listener-b", listener_b_steps),
        ],
        caps(2, 2, 8),
        ListenerMode::BlindThenReveal,
        TersmuFormat::Smusni,
        ReferenceToolDispatcher,
        scenario,
    )
    .expect("scenario runner");

    let outcome = runner.run().expect("closed-dialog answer phase");

    assert!(matches!(
        outcome.as_data(),
        ProtocolRunOutcomeData::ScenarioCompleted { turns: 3 }
    ));
    assert_eq!(runner.visible_chat().len(), 3);
    assert_eq!(runner.answers().len(), 2);
    assert_eq!(
        runner.task_outcome().expect("checker outcome").status,
        TaskStatus::Success
    );

    let events = runner.events();
    let closure_index = events
        .iter()
        .position(|event| {
            matches!(
                event.as_data(),
                bityzba::data!(ProtocolEvent::DialogClosedForAnswers {
                    turn_number: 3,
                    round_number: 1,
                })
            )
        })
        .expect("typed dialog closure");
    assert!(events[closure_index + 1..].iter().all(|event| !matches!(
        event.as_data(),
        bityzba::data!(ProtocolEvent::TurnStarted { .. })
            | bityzba::data!(ProtocolEvent::MessagePosted { .. })
            | bityzba::data!(ProtocolEvent::DialogClosedForAnswers { .. })
    )));
    let submitted = events[closure_index + 1..]
        .iter()
        .filter_map(|event| match event.as_data() {
            bityzba::data!(ProtocolEvent::AnswerSubmitted {
                turn_number: 3,
                participant,
                ..
            }) => Some(participant.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(submitted, ["listener-a", "listener-b"]);

    let participants = runner.participants();
    assert!(participants.iter().all(ScriptedModel::is_complete));
    let answer_prompts = participants[1..]
        .iter()
        .map(|participant| {
            participant
                .request_user_messages
                .last()
                .and_then(|messages| messages.last())
                .expect("answer-phase request has a final user instruction")
        })
        .collect::<Vec<_>>();
    assert_eq!(answer_prompts[0], answer_prompts[1]);
    assert!(answer_prompts[0].contains("visible-channel dialog is now closed"));
    assert!(
        answer_prompts[0].contains("based only on the dialog through the final posted description")
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn debate_runs_full_round_robin_to_instance_turn_cap_without_answer_or_checker_events() {
    let scenario =
        ScenarioInstance::from_toml(include_str!("../scenarios/debate-consciousness-1.toml"))
            .expect("debate scenario");
    let participant_names = ["alice", "bob", "carol"];
    let participants = participant_names
        .iter()
        .map(|participant| {
            let mut steps = Vec::new();
            for turn_number in 1..=scenario.maximum_turns() {
                let speaker = participant_names[(turn_number - 1) % participant_names.len()];
                if speaker == *participant {
                    steps.extend(posted_message_steps("I continue the debate.", "I go."));
                } else {
                    steps.extend(listener_steps("The speaker goes."));
                }
            }
            ScriptedModel::new(participant, steps)
        })
        .collect::<Vec<_>>();

    let run_config = new!(RunConfig {
        participants: participant_names
            .into_iter()
            .map(|name| new!(ParticipantConfig {
                name: name.to_owned(),
                model: format!("example/{name}"),
                provider: None,
                prompt_caching: xarsnu::PromptCaching::Auto,
                tool_choice: ToolChoice::Required,
                reasoning: None,
                temperature: 0.7,
                system_prompt: "Use the gated protocol.".to_owned(),
            }))
            .collect(),
        scenario: "debate-consciousness-1.toml".to_owned(),
        caps: caps(3, 2, 12),
        client: xarsnu::ClientConfig::default(),
        tersmu_format: TersmuFormat::Smusni,
        listener_mode: ListenerMode::BlindThenReveal,
        allow_degraded_search: false,
    });
    let header = RunHeader::new(run_config.clone(), &scenario).expect("transcript header");
    let transcript_path = temp_path("debate-turn-cap");
    let mut runner = ProtocolRunner::new_with_scenario(
        participants,
        run_config.caps.clone(),
        run_config.listener_mode,
        TersmuFormat::Smusni,
        ReferenceToolDispatcher,
        scenario,
    )
    .expect("debate runner");
    runner
        .attach_transcript(&transcript_path, header)
        .expect("attach debate transcript");

    let outcome = runner.run().expect("debate protocol run");

    assert!(matches!(
        outcome.as_data(),
        ProtocolRunOutcomeData::Completed { turns: 10 }
    ));
    assert_eq!(runner.visible_chat().len(), 10);
    assert!(runner.answers().is_empty());
    assert_eq!(runner.task_outcome(), None);
    assert!(runner.participants().iter().all(ScriptedModel::is_complete));
    assert!(runner.events().iter().all(|event| !matches!(
        event.as_data(),
        bityzba::data!(ProtocolEvent::AnswerSubmitted { .. })
            | bityzba::data!(ProtocolEvent::CheckerOutcome { .. })
            | bityzba::data!(ProtocolEvent::DialogClosedForAnswers { .. })
    )));

    let records = read_transcript(&transcript_path).expect("debate transcript validates");
    assert_eq!(records.len(), runner.events().len());
    let report = report_file(&transcript_path).expect("debate report renders");
    assert!(report.contains("Outcome: **dialog completed** after 10 turn(s)."));
    assert!(report.contains("Aggregate: **not scored**"));
    assert!(!report.contains("### Scenario checker"));
    fs::remove_file(transcript_path).expect("remove debate transcript");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn submit_answer_unlocks_after_minimum_rounds_and_finishes_after_all_required_answers() {
    let scenario =
        ScenarioInstance::from_toml(include_str!("../scenarios/schedule-negotiation-1.toml"))
            .expect("schedule scenario");
    let correct = json!({
        "day": "tuesday",
        "start_minute": 660,
        "duration_minutes": 60
    });
    let alice = ScriptedModel::new(
        "alice",
        vec![
            step(
                &["register_intent"],
                "register_intent",
                json!({ "meaning_en": "I can meet on Tuesday." }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "submit_lojban",
                json!({ "text": "mi klama" }),
            ),
            step(
                &["register_intent", "confirm_meaning"],
                "confirm_meaning",
                json!({ "matches": true, "paraphrase_en": "I go." }),
            ),
            step(
                &["interpret_blind"],
                "interpret_blind",
                json!({ "interpretation_en": "Bob goes." }),
            ),
            step(
                &["acknowledge"],
                "acknowledge",
                json!({ "final_understanding_en": "Bob goes." }),
            ),
            step(
                &["register_intent", "submit_answer"],
                "submit_answer",
                correct.clone(),
            ),
            step(
                &["register_intent"],
                "register_intent",
                json!({ "meaning_en": "The meeting is agreed." }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "submit_lojban",
                json!({ "text": "mi klama" }),
            ),
            step(
                &["register_intent", "confirm_meaning"],
                "confirm_meaning",
                json!({ "matches": true, "paraphrase_en": "I go." }),
            ),
        ],
    );
    let bob = ScriptedModel::new(
        "bob",
        vec![
            step(
                &["interpret_blind"],
                "interpret_blind",
                json!({ "interpretation_en": "Alice goes." }),
            ),
            step(
                &["acknowledge"],
                "acknowledge",
                json!({ "final_understanding_en": "Alice goes." }),
            ),
            step(
                &["register_intent"],
                "register_intent",
                json!({ "meaning_en": "I can meet later Tuesday." }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "submit_lojban",
                json!({ "text": "mi klama" }),
            ),
            step(
                &["register_intent", "confirm_meaning"],
                "confirm_meaning",
                json!({ "matches": true, "paraphrase_en": "I go." }),
            ),
            step(
                &["interpret_blind", "submit_answer"],
                "submit_answer",
                correct,
            ),
        ],
    );
    let header = RunHeader::new(
        new!(RunConfig {
            participants: ["alice", "bob"]
                .into_iter()
                .map(|name| new!(ParticipantConfig {
                    name: name.to_owned(),
                    model: format!("example/{name}"),
                    provider: None,
                    prompt_caching: xarsnu::PromptCaching::Auto,
                    tool_choice: ToolChoice::Required,
                    reasoning: None,
                    temperature: 0.25,
                    system_prompt: "Use the gated protocol.".to_owned(),
                }))
                .collect(),
            scenario: "schedule-negotiation-1.toml".to_owned(),
            caps: caps(3, 2, 6),
            client: xarsnu::ClientConfig::default(),
            tersmu_format: TersmuFormat::Smusni,
            listener_mode: ListenerMode::BlindThenReveal,
            allow_degraded_search: false,
        }),
        &scenario,
    )
    .expect("transcript header");
    let transcript_path = temp_path("protocol-transcript");
    let mut runner = ProtocolRunner::new_with_scenario(
        vec![alice, bob],
        caps(3, 2, 6),
        ListenerMode::BlindThenReveal,
        TersmuFormat::Smusni,
        ReferenceToolDispatcher,
        scenario,
    )
    .expect("scenario runner");
    runner
        .attach_transcript(&transcript_path, header)
        .expect("attach transcript");

    let outcome = runner.run().expect("scenario protocol run");

    assert!(matches!(
        outcome.as_data(),
        ProtocolRunOutcomeData::ScenarioCompleted { turns: 3 }
    ));
    assert_eq!(runner.answers().len(), 2);
    assert_eq!(
        runner.task_outcome().expect("checker outcome").status,
        TaskStatus::Success
    );
    assert_eq!(
        runner
            .events()
            .iter()
            .filter(|event| matches!(
                event.as_data(),
                bityzba::data!(ProtocolEvent::AnswerSubmitted { .. })
            ))
            .count(),
        2
    );
    assert!(matches!(
        runner
            .events()
            .iter()
            .rev()
            .find(|event| matches!(
                event.as_data(),
                bityzba::data!(ProtocolEvent::CheckerOutcome { .. })
            ))
            .expect("checker outcome event")
            .as_data(),
        bityzba::data!(ProtocolEvent::CheckerOutcome { outcome, .. })
            if outcome.status == TaskStatus::Success
    ));
    let records = read_transcript(&transcript_path).expect("runtime transcript validates");
    assert_eq!(records.len(), runner.events().len());
    assert_eq!(
        records.last().expect("terminal record").sequence_number,
        u64::try_from(records.len() - 1).unwrap()
    );
    std::fs::remove_file(transcript_path).expect("remove runtime transcript");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn redeclaring_intent_while_awaiting_confirmation_retains_candidate_and_records_sequence() {
    // Legitimate retreat, NOT post-hoc laundering: the original intent is a
    // two-part message ("go to the market" AND "to buy fish"); the speaker can only
    // get the parser to accept the first conjunct, so it drops the purpose clause it
    // could not express and keeps a sub-message it independently wanted to send. The
    // retained candidate genuinely expresses that narrowed goal. The wording — not
    // the harness — is what forbids the laundering variant, where a speaker would
    // instead read the revised intent off the rendering to force a match; the state
    // machine cannot tell the two apart, so this test only exercises the mechanical
    // retain-and-confirm path.
    let speaker = ScriptedModel::new(
        "alice",
        vec![
            step(
                &["register_intent"],
                "register_intent",
                json!({ "meaning_en": "I am going to the market to buy fish." }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "submit_lojban",
                json!({ "text": "mi klama le zarci" }),
            ),
            // Register ONLY the narrowed message the retained candidate actually
            // expresses. The rationale for the retreat (dropping the "to buy fish"
            // purpose clause that could not be parsed) stays in this comment, never in
            // the declared intent, so the confirm is a genuine match — not a mismatch
            // against unexpressed meta-text.
            step(
                &["register_intent", "confirm_meaning"],
                "register_intent",
                json!({ "meaning_en": "I am going to the market." }),
            ),
            step(
                &["register_intent", "confirm_meaning"],
                "confirm_meaning",
                json!({ "matches": true, "paraphrase_en": "I am going to the market." }),
            ),
        ],
    );
    let listener = ScriptedModel::new(
        "bob",
        vec![
            step(
                &["interpret_blind"],
                "interpret_blind",
                json!({ "interpretation_en": "Alice goes to the market." }),
            ),
            step(
                &["acknowledge"],
                "acknowledge",
                json!({ "final_understanding_en": "Alice goes to the market." }),
            ),
        ],
    );
    let mut runner = runner(vec![speaker, listener], caps(3, 2, 1)).expect("valid runner");

    runner.run().expect("re-declaration run");

    // Re-declaring intent while awaiting confirmation is legal, not an error.
    assert_eq!(
        count_protocol_errors(runner.events(), ProtocolTool::RegisterIntent),
        0
    );

    // The candidate was submitted and accepted exactly once: the re-declaration
    // retained it rather than forcing a re-submit.
    let submissions = runner
        .events()
        .iter()
        .filter(|event| {
            matches!(
                event.as_data(),
                bityzba::data!(ProtocolEvent::CandidateSubmitted { .. })
            )
        })
        .count();
    assert_eq!(submissions, 1);
    let accepted = runner
        .events()
        .iter()
        .filter(|event| {
            matches!(
                event.as_data(),
                bityzba::data!(ProtocolEvent::CandidateAccepted { .. })
            )
        })
        .count();
    assert_eq!(accepted, 1);

    // Original declaration (sequence 0) then the revised one (sequence 1).
    let registrations = runner
        .events()
        .iter()
        .filter_map(|event| match event.as_data() {
            bityzba::data!(ProtocolEvent::IntentRegistered {
                revision,
                revision_number,
                ..
            }) => Some((*revision, *revision_number)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(registrations, [(false, 0), (true, 1)]);

    // The confirm was measured against the revised intent (sequence 1) and posted.
    let confirm = runner
        .events()
        .iter()
        .find_map(|event| match event.as_data() {
            bityzba::data!(ProtocolEvent::MeaningConfirmed {
                matches,
                intent_sequence,
                ..
            }) => Some((*matches, *intent_sequence)),
            _ => None,
        })
        .expect("a confirm event");
    assert_eq!(confirm, (true, Some(1)));
    assert_eq!(runner.visible_chat().len(), 1);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn submit_lojban_while_awaiting_confirmation_names_state_and_legal_calls() {
    let speaker = ScriptedModel::new(
        "alice",
        vec![
            step(
                &["register_intent"],
                "register_intent",
                json!({ "meaning_en": "I go." }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "submit_lojban",
                json!({ "text": "mi klama" }),
            ),
            // Deliberately submit again while awaiting confirmation: the classic
            // out-of-order trip point from issue #609.
            step(
                &["register_intent", "confirm_meaning"],
                "submit_lojban",
                json!({ "text": "mi klama" }),
            ),
            step(
                &["register_intent", "confirm_meaning"],
                "confirm_meaning",
                json!({ "matches": true, "paraphrase_en": "I go." }),
            ),
        ],
    );
    let listener = ScriptedModel::new(
        "bob",
        vec![
            step(
                &["interpret_blind"],
                "interpret_blind",
                json!({ "interpretation_en": "Alice goes." }),
            ),
            step(
                &["acknowledge"],
                "acknowledge",
                json!({ "final_understanding_en": "Alice goes." }),
            ),
        ],
    );
    let mut runner = runner(vec![speaker, listener], caps(3, 2, 1)).expect("valid runner");

    runner.run().expect("out-of-order submit recovery run");

    assert_eq!(
        count_protocol_errors(runner.events(), ProtocolTool::SubmitLojban),
        1
    );
    // Two submit_lojban results exist: the accepted candidate's tersmu rendering
    // and the out-of-order rejection. Select the rejection by its diagnostic text.
    let error = runner.participants()[0]
        .tool_results
        .iter()
        .find(|result| {
            result.tool_name == "submit_lojban" && result.content.contains("is not available")
        })
        .expect("out-of-order submit_lojban rejection");
    assert!(error.content.contains("speaker/awaiting-confirmation"));
    assert!(error.content.contains("`register_intent`"));
    assert!(error.content.contains("`confirm_meaning`"));

    // The out-of-order submit did not fabricate a new candidate.
    let submissions = runner
        .events()
        .iter()
        .filter(|event| {
            matches!(
                event.as_data(),
                bityzba::data!(ProtocolEvent::CandidateSubmitted { .. })
            )
        })
        .count();
    assert_eq!(submissions, 1);
    assert_eq!(runner.visible_chat().len(), 1);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn repeated_redeclaration_while_awaiting_confirmation_bumps_sequence_and_retains_candidate() {
    let speaker = ScriptedModel::new(
        "alice",
        vec![
            step(
                &["register_intent"],
                "register_intent",
                json!({ "meaning_en": "I am going to the market to buy fish for the festival." }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "submit_lojban",
                json!({ "text": "mi klama le zarci" }),
            ),
            step(
                &["register_intent", "confirm_meaning"],
                "register_intent",
                json!({ "meaning_en": "I am going to the market to buy fish." }),
            ),
            step(
                &["register_intent", "confirm_meaning"],
                "register_intent",
                json!({ "meaning_en": "I am going to the market." }),
            ),
            step(
                &["register_intent", "confirm_meaning"],
                "confirm_meaning",
                json!({ "matches": true, "paraphrase_en": "I am going to the market." }),
            ),
        ],
    );
    let listener = ScriptedModel::new(
        "bob",
        vec![
            step(
                &["interpret_blind"],
                "interpret_blind",
                json!({ "interpretation_en": "Alice goes to the market." }),
            ),
            step(
                &["acknowledge"],
                "acknowledge",
                json!({ "final_understanding_en": "Alice goes to the market." }),
            ),
        ],
    );
    let mut runner = runner(vec![speaker, listener], caps(3, 3, 1)).expect("valid runner");

    runner.run().expect("repeated re-declaration run");

    let submissions = runner
        .events()
        .iter()
        .filter(|event| {
            matches!(
                event.as_data(),
                bityzba::data!(ProtocolEvent::CandidateSubmitted { .. })
            )
        })
        .count();
    assert_eq!(
        submissions, 1,
        "the candidate is retained across both re-declarations"
    );

    let registrations = runner
        .events()
        .iter()
        .filter_map(|event| match event.as_data() {
            bityzba::data!(ProtocolEvent::IntentRegistered {
                revision,
                revision_number,
                ..
            }) => Some((*revision, *revision_number)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(registrations, [(false, 0), (true, 1), (true, 2)]);

    let confirm = runner
        .events()
        .iter()
        .find_map(|event| match event.as_data() {
            bityzba::data!(ProtocolEvent::MeaningConfirmed {
                intent_sequence,
                ..
            }) => Some(*intent_sequence),
            _ => None,
        })
        .expect("a confirm event");
    assert_eq!(
        confirm,
        Some(2),
        "the confirm names the latest re-declaration"
    );
    assert_eq!(runner.visible_chat().len(), 1);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn intent_revision_cap_forfeits_at_awaiting_confirmation_after_the_exact_cap_is_used() {
    // Cap = 1: the first re-declaration (revision 1) is allowed even though a
    // candidate awaits confirmation; the second (which would be revision 2) forfeits.
    let speaker = ScriptedModel::new(
        "alice",
        vec![
            step(
                &["register_intent"],
                "register_intent",
                json!({ "meaning_en": "First intent." }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "submit_lojban",
                json!({ "text": "mi klama le zarci" }),
            ),
            step(
                &["register_intent", "confirm_meaning"],
                "register_intent",
                json!({ "meaning_en": "Allowed revision at the cap." }),
            ),
            step(
                &["register_intent", "confirm_meaning"],
                "register_intent",
                json!({ "meaning_en": "One revision too many." }),
            ),
        ],
    );
    let listener = ScriptedModel::new("bob", Vec::new());
    let mut runner = runner(vec![speaker, listener], caps(3, 1, 1)).expect("valid runner");

    runner
        .run()
        .expect("awaiting-confirmation revision-cap run");

    assert!(runner.visible_chat().is_empty());
    // The candidate was accepted before the forfeit, and nothing was posted.
    assert!(runner.events().iter().any(|event| matches!(
        event.as_data(),
        bityzba::data!(ProtocolEvent::CandidateAccepted { .. })
    )));
    assert!(!runner.events().iter().any(|event| matches!(
        event.as_data(),
        bityzba::data!(ProtocolEvent::MessagePosted { .. })
    )));
    assert!(runner.events().iter().any(|event| {
        matches!(
            event.as_data(),
            bityzba::data!(ProtocolEvent::TurnForfeited { reason, .. })
                if matches!(
                    reason.as_data(),
                    bityzba::data!(TurnForfeitReason::IntentRevisions { maximum: 1 })
                )
        )
    }));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn redeclare_then_mismatch_resubmits_and_confirms_against_the_revised_intent() {
    let speaker = ScriptedModel::new(
        "alice",
        vec![
            step(
                &["register_intent"],
                "register_intent",
                json!({ "meaning_en": "I am going to the market to buy fish." }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "submit_lojban",
                json!({ "text": "mi klama le zarci" }),
            ),
            step(
                &["register_intent", "confirm_meaning"],
                "register_intent",
                json!({ "meaning_en": "I am going home." }),
            ),
            // The retained candidate ("to the market") does not express the revised
            // intent ("going home"), so this is an honest mismatch, not a match.
            step(
                &["register_intent", "confirm_meaning"],
                "confirm_meaning",
                json!({ "matches": false, "paraphrase_en": "I am going to the market." }),
            ),
            step(
                &["register_intent", "submit_lojban"],
                "submit_lojban",
                json!({ "text": "mi klama le zdani" }),
            ),
            step(
                &["register_intent", "confirm_meaning"],
                "confirm_meaning",
                json!({ "matches": true, "paraphrase_en": "I am going home." }),
            ),
        ],
    );
    let listener = ScriptedModel::new(
        "bob",
        vec![
            step(
                &["interpret_blind"],
                "interpret_blind",
                json!({ "interpretation_en": "Alice goes home." }),
            ),
            step(
                &["acknowledge"],
                "acknowledge",
                json!({ "final_understanding_en": "Alice goes home." }),
            ),
        ],
    );
    let mut runner = runner(vec![speaker, listener], caps(3, 2, 1)).expect("valid runner");

    runner.run().expect("re-declare then mismatch run");

    let submissions = runner
        .events()
        .iter()
        .filter(|event| {
            matches!(
                event.as_data(),
                bityzba::data!(ProtocolEvent::CandidateSubmitted { .. })
            )
        })
        .count();
    assert_eq!(submissions, 2, "the mismatch forced a fresh candidate");

    // Both confirms were measured against the revised intent (revision 1), which
    // survived the mismatch-driven return to composing.
    let confirms = runner
        .events()
        .iter()
        .filter_map(|event| match event.as_data() {
            bityzba::data!(ProtocolEvent::MeaningConfirmed {
                matches,
                intent_sequence,
                ..
            }) => Some((*matches, *intent_sequence)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(confirms, [(false, Some(1)), (true, Some(1))]);
    assert_eq!(runner.visible_chat().len(), 1);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn legacy_meaning_confirmed_without_intent_sequence_deserializes_as_none() {
    // A transcript event written before the intent_sequence field existed must
    // still deserialize, with the field distinctly absent (None), never Some(0).
    let legacy = json!({
        "kind": "meaning-confirmed",
        "turn_number": 1,
        "speaker": "alice",
        "matches": true,
        "paraphrase_en": "I go.",
        "discrepancies": null
    });
    let event: ProtocolEvent =
        serde_json::from_value(legacy).expect("legacy confirm event deserializes");
    match event.as_data() {
        bityzba::data!(ProtocolEvent::MeaningConfirmed {
            intent_sequence,
            ..
        }) => assert_eq!(*intent_sequence, None),
        _ => panic!("expected a meaning-confirmed event"),
    }
}

use std::collections::{BTreeSet, VecDeque};

#[allow(unused_imports)]
use bityzba::{contract_trait, ensures, invariant, new, requires};
use serde_json::{Value, json};
use xarsnu::openrouter::ModelTurnData;
use xarsnu::protocol::{ProtocolEventData, ProtocolRunOutcomeData, TurnForfeitReasonData};
use xarsnu::{
    CapsConfig, ModelTurn, ParticipantConfig, ProtocolEvent, ProtocolModel, ProtocolModelError,
    ProtocolRunner, ProtocolTool, ReferenceToolDispatcher, RunAccounting, RunConfig, RunHeader,
    ScenarioInstance, TaskStatus, TersmuFormat, ToolCall, ToolChoice, ToolDefinition,
    read_transcript,
};

const REFERENCE_TOOLS: [&str; 5] = ["vlacku", "gentufa", "tersmu", "jvozba", "cukta"];

#[invariant(!tool_name.trim().is_empty())]
#[invariant(arguments.is_object())]
#[invariant(expected_protocol_tools.iter().all(|name| !name.trim().is_empty()))]
#[derive(Debug)]
struct ScriptStep {
    expected_protocol_tools: Vec<&'static str>,
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

#[invariant(true, "constructed with a nonempty name and test-owned script")]
#[derive(Debug)]
struct ScriptedModel {
    name: String,
    steps: VecDeque<ScriptStep>,
    user_messages: Vec<String>,
    tool_results: Vec<RecordedToolResult>,
    calls_made: usize,
}

impl ScriptedModel {
    #[requires(!name.trim().is_empty())]
    #[ensures(ret.name == name)]
    fn new(name: &str, steps: Vec<ScriptStep>) -> Self {
        Self {
            name: name.to_owned(),
            steps: steps.into(),
            user_messages: Vec::new(),
            tool_results: Vec::new(),
            calls_made: 0,
        }
    }

    #[requires(true)]
    #[ensures(ret == self.steps.is_empty())]
    fn is_complete(&self) -> bool {
        self.steps.is_empty()
    }
}

#[contract_trait]
impl ProtocolModel for ScriptedModel {
    fn participant_name(&self) -> &str {
        &self.name
    }

    fn push_user(&mut self, content: String) {
        self.user_messages.push(content);
    }

    fn request(
        &mut self,
        tools: &[ToolDefinition],
        tool_choice: ToolChoice,
        _accounting: &mut RunAccounting,
    ) -> Result<ModelTurn, ProtocolModelError> {
        assert_eq!(tool_choice, ToolChoice::Required);
        let step = self
            .steps
            .pop_front()
            .unwrap_or_else(|| panic!("{} had no scripted response left", self.name));
        let bityzba::data!(ScriptStep {
            expected_protocol_tools,
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
            .chain(REFERENCE_TOOLS)
            .collect::<BTreeSet<_>>();
        assert_eq!(actual, expected, "dynamic tools for {}", self.name);
        self.calls_made += 1;
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
        tool_name,
        arguments,
    })
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
        TersmuFormat::TreeProj,
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
                &["confirm_meaning"],
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
                &["confirm_meaning"],
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
                &["confirm_meaning"],
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
                &["confirm_meaning"],
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
                &["confirm_meaning"],
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
                &["confirm_meaning"],
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
        result.tool_name == "submit_lojban" && result.content.contains("not legal")
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
                &["confirm_meaning"],
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
        result.tool_name == "confirm_meaning" && result.content.contains("not legal")
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
                &["confirm_meaning"],
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
                &["confirm_meaning"],
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
                &["confirm_meaning"],
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
                &["confirm_meaning"],
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
                &["confirm_meaning"],
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
                    temperature: 0.25,
                    system_prompt: "Use the gated protocol.".to_owned(),
                    private_brief: format!("Private English brief for {name}."),
                }))
                .collect(),
            scenario: "schedule-negotiation-1.toml".to_owned(),
            caps: caps(3, 2, 6),
            tersmu_format: TersmuFormat::TreeProj,
        }),
        &scenario,
    )
    .expect("transcript header");
    let transcript_path = std::env::temp_dir().join(format!(
        "xarsnu-protocol-transcript-{}.jsonl",
        std::process::id()
    ));
    let mut runner = ProtocolRunner::new_with_scenario(
        vec![alice, bob],
        caps(3, 2, 6),
        TersmuFormat::TreeProj,
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

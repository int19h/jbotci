use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use serde_json::{Value, json};
use xarsnu::{ScenarioAnswer, ScenarioInstance, ScenarioKind, TaskStatus};

#[requires(true)]
#[ensures(!ret.is_empty())]
fn fixture_instances() -> Vec<ScenarioInstance> {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scenarios");
    let mut paths = fs::read_dir(directory)
        .expect("scenario fixture directory")
        .map(|entry| entry.expect("fixture directory entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "toml")
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path).expect("read scenario fixture");
            ScenarioInstance::from_toml(&source)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
        })
        .collect()
}

#[requires(value.is_object())]
#[ensures(true)]
fn typed(instance: &ScenarioInstance, value: Value) -> ScenarioAnswer {
    instance.parse_answer(value).expect("typed fixture answer")
}

#[requires(answer_kind_matches(instance, &answer))]
#[ensures(ret.len() >= 1)]
fn all_required_answers(
    instance: &ScenarioInstance,
    answer: ScenarioAnswer,
) -> BTreeMap<String, ScenarioAnswer> {
    instance
        .participants()
        .iter()
        .filter(|participant| participant.answer_required)
        .map(|participant| (participant.name.clone(), answer.clone()))
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn answer_kind_matches(instance: &ScenarioInstance, answer: &ScenarioAnswer) -> bool {
    matches!(
        (instance.kind(), answer),
        (
            ScenarioKind::ScheduleNegotiation,
            ScenarioAnswer::Schedule { .. }
        ) | (
            ScenarioKind::DistributedClueDeduction,
            ScenarioAnswer::Deduction { .. }
        ) | (
            ScenarioKind::ReferentialGame,
            ScenarioAnswer::Referential { .. }
        )
    )
}

#[requires(true)]
#[ensures(ret.is_object())]
fn correct_answer(instance: &ScenarioInstance) -> Value {
    match instance.id() {
        "schedule-negotiation-1" => {
            json!({ "day": "tuesday", "start_minute": 660, "duration_minutes": 60 })
        }
        "schedule-negotiation-2" => {
            json!({ "day": "wednesday", "start_minute": 840, "duration_minutes": 60 })
        }
        "distributed-clue-deduction-1" => json!({
            "assignments": [
                { "person": "Ada", "profession": "doctor", "city": "Lima" },
                { "person": "Ben", "profession": "engineer", "city": "Oslo" },
                { "person": "Cy", "profession": "teacher", "city": "Rome" }
            ]
        }),
        "distributed-clue-deduction-2" => json!({
            "assignments": [
                { "person": "Inez", "profession": "pilot", "city": "Quito" },
                { "person": "Jun", "profession": "scientist", "city": "Kyoto" },
                { "person": "Kofi", "profession": "baker", "city": "Accra" }
            ]
        }),
        "referential-game-1" => json!({ "scene_index": 1 }),
        "referential-game-2" => json!({ "scene_index": 2 }),
        "referential-game-3" => json!({ "scene_index": 3 }),
        "referential-game-abstraction-1" => json!({ "scene_index": 2 }),
        "referential-game-abstraction-2" => json!({ "scene_index": 4 }),
        other => panic!("unknown fixture {other}"),
    }
}

#[requires(true)]
#[ensures(ret.is_object())]
fn perturbed_answer(instance: &ScenarioInstance) -> Value {
    match instance.id() {
        "schedule-negotiation-1" => {
            json!({ "day": "tuesday", "start_minute": 661, "duration_minutes": 60 })
        }
        "schedule-negotiation-2" => {
            json!({ "day": "wednesday", "start_minute": 841, "duration_minutes": 60 })
        }
        "distributed-clue-deduction-1" => json!({
            "assignments": [
                { "person": "Ada", "profession": "engineer", "city": "Lima" },
                { "person": "Ben", "profession": "doctor", "city": "Oslo" },
                { "person": "Cy", "profession": "teacher", "city": "Rome" }
            ]
        }),
        "distributed-clue-deduction-2" => json!({
            "assignments": [
                { "person": "Inez", "profession": "scientist", "city": "Quito" },
                { "person": "Jun", "profession": "pilot", "city": "Kyoto" },
                { "person": "Kofi", "profession": "baker", "city": "Accra" }
            ]
        }),
        "referential-game-1" => json!({ "scene_index": 2 }),
        "referential-game-2" => json!({ "scene_index": 3 }),
        "referential-game-3" => json!({ "scene_index": 2 }),
        "referential-game-abstraction-1" => json!({ "scene_index": 1 }),
        "referential-game-abstraction-2" => json!({ "scene_index": 3 }),
        other => panic!("unknown fixture {other}"),
    }
}

#[requires(instance.kind() == ScenarioKind::ReferentialGame)]
#[ensures(ret == 3 || ret == 4)]
fn referential_scene_count(instance: &ScenarioInstance) -> usize {
    match instance.id() {
        "referential-game-1" | "referential-game-2" | "referential-game-3" => 3,
        "referential-game-abstraction-1" | "referential-game-abstraction-2" => 4,
        other => panic!("unknown referential fixture {other}"),
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn all_scenario_toml_fixtures_round_trip() {
    let instances = fixture_instances();
    assert_eq!(instances.len(), 10);
    for instance in instances {
        let encoded = instance.to_toml().expect("serialize scenario");
        let decoded = ScenarioInstance::from_toml(&encoded).expect("reparse scenario");
        assert_eq!(decoded, instance, "round trip for {}", instance.id());
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn debate_fixture_preserves_authored_briefs_without_scoring_fields_or_school_labels() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scenarios/debate-consciousness-1.toml");
    let source = fs::read_to_string(path).expect("debate fixture");
    let lowercase = source.to_lowercase();
    for forbidden_label in [
        "consequentialism",
        "consequentialist",
        "deontology",
        "deontological",
        "utilitarian",
        "utilitarianism",
        "virtue",
    ] {
        assert!(
            !lowercase.contains(forbidden_label),
            "forbidden school label `{forbidden_label}` appeared in raw scenario TOML"
        );
    }

    let debate = ScenarioInstance::from_toml(&source).expect("debate fixture");
    assert_eq!(debate.kind(), ScenarioKind::Debate);
    assert_eq!(
        debate.title(),
        "What do we owe to minds? The hard problem and its ethics"
    );
    assert_eq!(debate.maximum_turns(), 10);
    assert!(!debate.is_scored());
    assert_eq!(debate.answer_schema(), None);
    assert_eq!(debate.minimum_rounds(), None);
    assert_eq!(debate.maximum_rounds(), None);
    assert!(!debate.answers_close_dialog());
    assert!(
        debate
            .participants()
            .iter()
            .all(|participant| !participant.answer_required)
    );

    let norm = "You find genuine disagreement clarifying. When another speaker's reasoning rests on premises you reject, say so plainly and press the point in Lojban; do not accommodate for the sake of harmony. Engage the strongest version of what they said.";
    let expected_briefs = [
        (
            "alice",
            "When you weigh any ethical question, what ultimately moves you is how much better or worse experience becomes for those who have it: suffering prevented, wellbeing produced. Rules and character matter to you only insofar as they change what actually happens to minds. You are skeptical of moral claims that cannot be cashed out as someone's experience going differently.",
        ),
        (
            "bob",
            "When you weigh any ethical question, what ultimately moves you is what is owed: duties and constraints that bind regardless of how the totals come out. Some things may not be done to a mind even for better overall outcomes, and some things are owed to it even when nobody benefits. You are skeptical of arguments that make right and wrong hostage to consequences.",
        ),
        (
            "carol",
            "When you weigh any ethical question, what ultimately moves you is what the choice reveals about and does to the character of the one choosing: whether it is what a wise, just, and compassionate agent would do, and what doing it cultivates or corrodes. You are skeptical of both rule-tallies and outcome-tallies detached from the kind of agent acting.",
        ),
    ];
    for (name, bias) in expected_briefs {
        let participant = debate
            .participants()
            .iter()
            .find(|participant| participant.name == name)
            .expect("named debate participant");
        assert_eq!(participant.private_brief, format!("{bias}\n\n{norm}"));
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn answer_dialog_closure_defaults_by_scenario_family_and_can_be_overridden() {
    let referential_source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scenarios/referential-game-1.toml"),
    )
    .expect("referential fixture");
    let referential =
        ScenarioInstance::from_toml(&referential_source).expect("referential default");
    assert!(referential.answers_close_dialog());

    let referential_open = referential_source.replace(
        "minimum-rounds = 1",
        "answers-close-dialog = false\nminimum-rounds = 1",
    );
    let referential_open =
        ScenarioInstance::from_toml(&referential_open).expect("referential override");
    assert!(!referential_open.answers_close_dialog());
    assert!(
        referential_open
            .to_toml()
            .expect("serialize override")
            .contains("answers-close-dialog = false")
    );

    let schedule_source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scenarios/schedule-negotiation-1.toml"),
    )
    .expect("schedule fixture");
    let schedule = ScenarioInstance::from_toml(&schedule_source).expect("schedule default");
    assert!(!schedule.answers_close_dialog());

    let schedule_closed = schedule_source
        .replace(
            "minimum-rounds = 1",
            "answers-close-dialog = true\nminimum-rounds = 1",
        )
        .replace("maximum-turns = 5", "maximum-turns = 2");
    let schedule_closed =
        ScenarioInstance::from_toml(&schedule_closed).expect("closed schedule override");
    assert!(schedule_closed.answers_close_dialog());
}

#[test]
#[requires(true)]
#[ensures(true)]
fn every_checker_accepts_truth_and_rejects_a_minimal_perturbation() {
    for instance in fixture_instances()
        .into_iter()
        .filter(ScenarioInstance::is_scored)
    {
        let correct = typed(&instance, correct_answer(&instance));
        let outcome = instance.check_answers(&all_required_answers(&instance, correct));
        assert_eq!(outcome.status, TaskStatus::Success, "{}", instance.id());
        assert!(
            outcome
                .participants
                .iter()
                .filter(|participant| participant.required)
                .all(|participant| participant.correct == Some(true)),
            "{}",
            instance.id()
        );

        let perturbed = typed(&instance, perturbed_answer(&instance));
        let outcome = instance.check_answers(&all_required_answers(&instance, perturbed));
        assert_eq!(outcome.status, TaskStatus::Failure, "{}", instance.id());
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn invalid_config_errors_name_the_offending_field() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scenarios/schedule-negotiation-1.toml");
    let source = fs::read_to_string(path).expect("schedule fixture");
    let invalid = source.replace("minimum-rounds = 1", "minimum-rounds = 0");
    let error = ScenarioInstance::from_toml(&invalid).expect_err("zero minimum rounds");
    assert_eq!(error.field(), Some("minimum-rounds"));
    assert!(error.to_string().contains("minimum-rounds"));

    let invalid = source.replace("\"start_minute\", ", "");
    let error = ScenarioInstance::from_toml(&invalid).expect_err("schema drift");
    assert_eq!(error.field(), Some("answer-schema"));
    assert!(error.to_string().contains("answer-schema"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn referential_no_description_guesser_stays_at_chance_rate() {
    let mut instances_by_scene_count = BTreeMap::<usize, Vec<ScenarioInstance>>::new();
    for instance in fixture_instances()
        .into_iter()
        .filter(|instance| instance.kind() == ScenarioKind::ReferentialGame)
    {
        instances_by_scene_count
            .entry(referential_scene_count(&instance))
            .or_default()
            .push(instance);
    }
    assert_eq!(
        instances_by_scene_count
            .values()
            .map(Vec::len)
            .sum::<usize>(),
        5
    );

    for (scene_count, instances) in instances_by_scene_count {
        let chance_successes = instances.len().div_ceil(scene_count);
        for fixed_guess in 1..=scene_count {
            let successes = instances
                .iter()
                .filter(|instance| {
                    let answer = typed(instance, json!({ "scene_index": fixed_guess }));
                    instance
                        .check_answers(&all_required_answers(instance, answer))
                        .status
                        == TaskStatus::Success
                })
                .count();
            assert!(
                successes <= chance_successes,
                "fixed guess {fixed_guess} among {scene_count}-scene instances"
            );
        }
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn scenario_prompts_do_not_cross_private_briefs() {
    let instances = fixture_instances();
    let schedule = instances
        .iter()
        .find(|instance| instance.id() == "schedule-negotiation-1")
        .expect("schedule fixture");
    let alice = schedule.prompt_for("alice").expect("alice prompt");
    let bob = schedule.prompt_for("bob").expect("bob prompt");
    assert!(!alice.contains("12:30"));
    assert!(!bob.contains("09:00"));

    let deduction = instances
        .iter()
        .find(|instance| instance.id() == "distributed-clue-deduction-1")
        .expect("deduction fixture");
    let alice = deduction.prompt_for("alice").expect("alice prompt");
    assert!(!alice.contains("Cy's profession is teacher"));

    let referential = instances
        .iter()
        .find(|instance| instance.id() == "referential-game-2")
        .expect("referential fixture");
    let listener = referential
        .prompt_for("listener-a")
        .expect("listener prompt");
    assert!(!listener.contains("The hidden target is"));
}

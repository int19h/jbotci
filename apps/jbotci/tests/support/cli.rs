use super::*;
use base64::Engine as _;
use clap::CommandFactory;
use clap::error::ErrorKind;
use jbotci_dialect::DialectFeature;
use jbotci_embeddings::{EMBEDDING_INDEX_DIR_ENV, EMBEDDING_MODEL_DIR_ENV};
use jbotci_search::vlacku::INVALID_LOJBAN_WORD_MESSAGE_PREFIX;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

#[test]
#[requires(true)]
#[ensures(true)]
fn parses_canonical_and_english_aliases() {
    assert!(matches!(
        Cli::try_parse_from(["jbotci", "vlasei", "coi"])
            .expect("canonical command")
            .command,
        Command::Vlasei(_)
    ));
    assert!(matches!(
        Cli::try_parse_from(["jbotci", "lex", "coi"])
            .expect("alias command")
            .command,
        Command::Vlasei(_)
    ));
    assert!(matches!(
        Cli::try_parse_from(["jbotci", "vlatai", "coi"])
            .expect("vlatai command")
            .command,
        Command::Vlatai(_)
    ));
    assert!(Cli::try_parse_from(["jbotci", "server"]).is_err());
    assert!(Cli::try_parse_from(["jbotci", "selfu"]).is_err());
}

#[test]
#[requires(true)]
#[ensures(true)]
fn lsp_accepts_optional_stdio_and_rejects_other_transports() {
    assert!(matches!(
        Cli::try_parse_from(["jbotci", "lsp"])
            .expect("bare lsp command")
            .command,
        Command::Lsp { stdio: false }
    ));
    assert!(matches!(
        Cli::try_parse_from(["jbotci", "lsp", "--stdio"])
            .expect("editor-compatible stdio flag")
            .command,
        Command::Lsp { stdio: true }
    ));
    assert!(Cli::try_parse_from(["jbotci", "lsp", "--socket"]).is_err());
    assert!(Cli::try_parse_from(["jbotci", "lsp", "--tcp"]).is_err());
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlatai_text_reports_lujvo_split_and_stdout_diagnostics() {
    let run = run_cli_capture(
        &["jbotci", "vlatai", "jetcybolxada", "coibroda", "aa"],
        false,
    );

    assert_eq!(run.status, CliStatus::Failure);
    assert!(run.stderr.is_empty());
    assert_in_order(
        &run.stdout,
        &[
            "valsi: jetcybolxada\n",
            "status: valid\n",
            "category: lujvo\n",
            "phonemes: jetcybolxáda\n",
            "split: jetc-y-bolxáda\n",
            "rafsi: jetc",
            "hyphen: y",
            "rafsi: bolxáda",
            "valsi: coibroda\n",
            "status: not-single-word\n",
            "vlatai.not-single-word",
            "words:",
            "valsi: aa\n",
            "status: invalid\n",
            "morphology.vowel-hiatus",
        ],
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlatai_json_reports_machine_readable_statuses() {
    let run = run_cli_capture(
        &[
            "jbotci",
            "vlatai",
            "--format",
            "json",
            "jetcybolxada",
            "coibroda",
        ],
        false,
    );

    assert_eq!(run.status, CliStatus::Failure);
    assert!(run.stderr.is_empty());
    let json: serde_json::Value = serde_json::from_str(&run.stdout).expect("valid JSON");
    assert_eq!(json[0]["status"], "valid");
    assert_eq!(json[0]["classification"]["category"], "lujvo");
    assert_eq!(json[0]["classification"]["split"], "jetc-y-bolxáda");
    assert_eq!(json[1]["status"], "not-single-word");
    assert!(
        json[1]["diagnostics"]
            .as_array()
            .is_some_and(|value| !value.is_empty())
    );
    assert!(json[1]["words"].is_array());
}

#[test]
#[requires(true)]
#[ensures(true)]
fn parses_benchmark_before_and_after_subcommand() {
    let before_cli = Cli::try_parse_from(["jbotci", "--benchmark", "3", "vlasei", "coi"])
        .expect("benchmark before subcommand");
    assert_eq!(before_cli.benchmark.map(NonZeroUsize::get), Some(3));
    assert!(matches!(before_cli.command, Command::Vlasei(_)));

    let after_cli = Cli::try_parse_from(["jbotci", "vlasei", "--benchmark", "4", "coi"])
        .expect("benchmark after subcommand");
    assert_eq!(after_cli.benchmark.map(NonZeroUsize::get), Some(4));
    assert!(matches!(after_cli.command, Command::Vlasei(_)));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn rejects_zero_benchmark_iterations() {
    let error = Cli::try_parse_from(["jbotci", "vlasei", "--benchmark", "0", "coi"])
        .expect_err("zero benchmark iteration count is rejected");
    assert_eq!(error.kind(), ErrorKind::ValueValidation);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn tool_vlacku_semantic_uses_supplied_embedding_context_error() {
    let mut context =
        ToolExecutionContext::embedding_search_unavailable("cached embedding load failed".into());
    let output = run_tool_vlacku_with_context(
        ToolVlackuRequest {
            mode: ToolVlackuMode::Meaning,
            query: "goer".to_owned(),
            count: Some(1),
            word_types: Vec::new(),
            min_votes: None,
            min_similarity: None,
            decompose_lujvo: true,
            show_etymology: false,
        },
        &mut context,
    )
    .expect("tool output");

    assert_eq!(output.status, ToolStatus::InvalidInput);
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr, "vlacku: cached embedding load failed\n");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn tool_vlacku_punctuation_only_query_reports_invalid_input_status() {
    let output = run_tool_vlacku(ToolVlackuRequest {
        mode: ToolVlackuMode::Word,
        query: "!!!".to_owned(),
        count: Some(1),
        word_types: Vec::new(),
        min_votes: None,
        min_similarity: None,
        decompose_lujvo: true,
        show_etymology: false,
    })
    .expect("tool output");

    assert_eq!(output.status, ToolStatus::InvalidInput);
    assert!(output.stdout.is_empty());
    assert_eq!(
        output.stderr,
        format!("vlacku: {INVALID_LOJBAN_WORD_MESSAGE_PREFIX}!!!\n")
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn tool_cukta_semantic_uses_supplied_embedding_context_error() {
    let mut context =
        ToolExecutionContext::embedding_search_unavailable("cached embedding load failed".into());
    let output = run_tool_cukta_with_context(
        ToolCuktaRequest {
            mode: ToolCuktaMode::Meaning,
            query: Some("goer".to_owned()),
            count: Some(1),
            search_result_kinds: Vec::new(),
            format: ToolCuktaFormat::Markdown,
        },
        &mut context,
    )
    .expect("tool output");

    assert_eq!(output.status, ToolStatus::InvalidInput);
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr, "cached embedding load failed\n");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn tool_cukta_section_output_uses_plain_links() {
    let output = jbotci_cli::run_tool_cukta(ToolCuktaRequest {
        mode: ToolCuktaMode::Section,
        query: Some("9.6".to_owned()),
        count: None,
        search_result_kinds: Vec::new(),
        format: ToolCuktaFormat::Markdown,
    })
    .expect("tool output");

    assert_eq!(output.status, ToolStatus::Success);
    assert!(output.stderr.is_empty(), "{}", output.stderr);
    let stdout = String::from_utf8(output.stdout).expect("cukta Markdown should be UTF-8");
    assert!(!stdout.contains("]("), "{stdout}");
    assert!(!stdout.contains("Parse"), "{stdout}");
    assert!(stdout.contains("| mi | viska | do | sepi'o |"), "{stdout}");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn benchmark_repeats_stdout_and_reports_success_metrics() {
    let once = run_cli_capture(&["jbotci", "vlasei", "--format", "brackets", "coi"], false);
    assert_eq!(once.status, CliStatus::Success);
    assert!(once.stderr.is_empty());

    let benchmark = run_cli_capture(
        &[
            "jbotci",
            "vlasei",
            "--benchmark",
            "2",
            "--format",
            "brackets",
            "coi",
        ],
        false,
    );
    assert_eq!(benchmark.status, CliStatus::Success);
    assert_eq!(benchmark.stdout, format!("{}{}", once.stdout, once.stdout));
    assert_benchmark_report_contains(
        &benchmark.stderr,
        "iterations: 2",
        "statuses: success=2 failure=0 valid-missing=0 invalid-input=0",
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn benchmark_continues_failure_statuses_and_appends_metrics_after_stderr() {
    let once = run_cli_capture(&["jbotci", "vlasei", "aa"], false);
    assert_eq!(once.status, CliStatus::Failure);
    assert!(!once.stderr.is_empty());

    let benchmark = run_cli_capture(&["jbotci", "vlasei", "--benchmark", "2", "aa"], false);
    assert_eq!(benchmark.status, CliStatus::Failure);
    let benchmark_start = benchmark
        .stderr
        .rfind("benchmark:\n")
        .expect("benchmark report");
    assert_eq!(
        &benchmark.stderr[..benchmark_start],
        format!("{}{}", once.stderr, once.stderr)
    );
    assert_benchmark_report_contains(
        &benchmark.stderr[benchmark_start..],
        "iterations: 2",
        "statuses: success=0 failure=2 valid-missing=0 invalid-input=0",
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn benchmark_rejects_unsupported_commands() {
    let cli = Cli::try_parse_from(["jbotci", "jvozba", "--benchmark", "2", "lojbo", "bangu"])
        .expect("benchmark flag parses globally");
    let error = run_cli(cli, &mut Vec::new(), &mut Vec::new(), false)
        .expect_err("benchmark rejects unsupported command");
    assert!(
        error
            .to_string()
            .contains("only supported with vlasei, gentufa, vlacku, and cukta")
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn parses_vlacku_primary_name_and_dict_alias() {
    let Command::Vlacku(primary_input) =
        Cli::try_parse_from(["jbotci", "vlacku", "--valsi", "klama"])
            .expect("primary vlacku command")
            .command
    else {
        panic!("expected vlacku command");
    };
    assert_eq!(
        primary_input.requests,
        vec![VlackuRequest::valsi("klama".to_owned())]
    );
    assert_eq!(primary_input.sumti_places, CliSumtiPlaces::Index);

    let Command::Vlacku(alias_input) =
        Cli::try_parse_from(["jbotci", "dict", "--sumti-places", "raw", "--rafsi", "kla"])
            .expect("dict alias command")
            .command
    else {
        panic!("expected vlacku command");
    };
    assert_eq!(
        alias_input.requests,
        vec![VlackuRequest::rafsi("kla".to_owned())]
    );
    assert_eq!(alias_input.sumti_places, CliSumtiPlaces::Raw);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn parses_jvozba_command() {
    let Command::Jvozba(input) =
        Cli::try_parse_from(["jbotci", "jvozba", "--cmevla", "lojbo", "--rafsi", "bau"])
            .expect("jvozba command")
            .command
    else {
        panic!("expected jvozba command");
    };
    assert!(input.cmevla);
    assert_eq!(
        input.sources,
        vec![
            JvozbaSourceInput::Word("lojbo".to_owned()),
            JvozbaSourceInput::FixedRafsi("bau".to_owned()),
        ]
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn parses_jvozba_word_and_rafsi_order() {
    let Command::Jvozba(input) = Cli::try_parse_from([
        "jbotci", "jvozba", "--rafsi", "jbo", "bangu", "--rafsi", "bau",
    ])
    .expect("jvozba command")
    .command
    else {
        panic!("expected jvozba command");
    };
    assert_eq!(
        input.sources,
        vec![
            JvozbaSourceInput::FixedRafsi("jbo".to_owned()),
            JvozbaSourceInput::Word("bangu".to_owned()),
            JvozbaSourceInput::FixedRafsi("bau".to_owned()),
        ]
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn parses_gimfihi_command_and_apostrophe_alias() {
    let Command::Gimfihi(primary_input) = Cli::try_parse_from([
        "jbotci",
        "gimfihi",
        "--preset",
        "1995",
        "--source",
        "eng::ekspekt",
    ])
    .expect("gimfihi command")
    .command
    else {
        panic!("expected gimfihi command");
    };
    assert_eq!(primary_input.preset.as_deref(), Some("1995"));
    assert_eq!(
        primary_input.sources,
        vec![GimfihiSourceInput {
            language: "eng".to_owned(),
            explicit_weight: None,
            word: "ekspekt".to_owned(),
            ipa: None,
        }]
    );

    let Command::Gimfihi(alias_input) =
        Cli::try_parse_from(["jbotci", "gimfi'i", "--source", "eng:1:ekspekt"])
            .expect("gimfi'i alias command")
            .command
    else {
        panic!("expected gimfihi command");
    };
    assert_eq!(alias_input.preset, None);
    assert_eq!(
        alias_input.sources,
        vec![GimfihiSourceInput {
            language: "eng".to_owned(),
            explicit_weight: Some(1),
            word: "ekspekt".to_owned(),
            ipa: None,
        }]
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gimfihi_rejects_old_spellings() {
    assert!(Cli::try_parse_from(["jbotci", "gimfihe"]).is_err());
    assert!(Cli::try_parse_from(["jbotci", "gimfi'e"]).is_err());
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gimfihi_parameter_defaults_match_phonetic_crate_and_help_lists_every_knob() {
    let defaults = jbotci_phonetic::AlineParameters::default();
    let Command::Gimfihi(input) =
        Cli::try_parse_from(["jbotci", "gimfihi", "--source", "eng:1:klama"])
            .expect("gimfihi defaults")
            .command
    else {
        panic!("expected gimfihi command");
    };
    assert_eq!(input.scorer, GimfihiCliScorer::Classic);
    assert_eq!(input.c_sub, defaults.c_sub);
    assert_eq!(input.c_exp, defaults.c_exp);
    assert_eq!(input.c_skip, defaults.c_skip);
    assert_eq!(input.c_vwl, defaults.c_vwl);
    assert_eq!(input.c_flank, defaults.c_flank);
    assert_eq!(input.normalizer, GimfihiCliNormalizer::SourceSide);
    assert!(input.saliences.is_empty());

    let help = Cli::command()
        .find_subcommand_mut("gimfihi")
        .expect("gimfihi subcommand")
        .render_long_help()
        .to_string();
    for knob in [
        "--scorer",
        "--c-sub",
        "--c-exp",
        "--c-skip",
        "--c-vwl",
        "--c-flank",
        "--normalizer",
        "--salience",
    ] {
        assert!(help.contains(knob), "missing {knob} in {help}");
    }
    for default_text in [
        "default: classic",
        "default: 35",
        "default: 45",
        "default: -10",
        "default: 10",
        "default: 0",
        "default: source-side",
        "manner=50",
        "long=1",
    ] {
        assert!(
            help.contains(default_text),
            "missing {default_text} in {help}"
        );
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gimfihi_rejects_invalid_scorer_salience_and_nonfinite_coefficient_by_name() {
    let scorer_error = Cli::try_parse_from([
        "jbotci",
        "gimfihi",
        "--scorer",
        "mystery",
        "--source",
        "eng:1:klama",
    ])
    .expect_err("unknown scorer");
    assert!(scorer_error.to_string().contains("scorer"));

    let salience_error = Cli::try_parse_from([
        "jbotci",
        "gimfihi",
        "--salience",
        "mystery=5",
        "--source",
        "eng:1:klama",
    ])
    .expect_err("unknown salience");
    assert!(salience_error.to_string().contains("mystery"));

    let cli = Cli::try_parse_from([
        "jbotci",
        "gimfihi",
        "--c-sub",
        "NaN",
        "--source",
        "eng:1:klama",
    ])
    .expect("clap accepts an f64 spelling");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let error = run_cli(cli, &mut stdout, &mut stderr, false).expect_err("nonfinite c-sub");
    assert!(error.to_string().contains("c-sub"), "{error}");

    let tool_error = run_tool_gimfihi(
        ToolGimfihiRequest {
            sources: vec![ToolGimfihiSource {
                language: "eng".to_owned(),
                word: "klama".to_owned(),
                weight: Some(1),
            }],
            scorer: ToolGimfihiScorer::Phonetic,
            c_sub: f64::INFINITY,
            ..ToolGimfihiRequest::default()
        },
        GimfihiSourceWordKind::Ipa,
    )
    .expect_err("typed tool rejects nonfinite c-sub");
    assert!(tool_error.to_string().contains("c-sub"), "{tool_error}");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gimfihi_cli_and_mcp_bridge_produce_identical_phonetic_ranking() {
    let cli = run_cli_capture(
        &[
            "jbotci",
            "gimfihi",
            "--scorer",
            "phonetic",
            "--c-vwl",
            "8",
            "--source",
            "src:700:[qalma]",
            "--source",
            "support:300:[alma]",
            "--shape",
            "cvccv",
            "--check-collisions",
            "none",
            "--count",
            "5",
            "--format",
            "json",
        ],
        false,
    );
    assert_eq!(cli.status, CliStatus::Success);
    let cli_output: GimfihiOutput =
        serde_json::from_str(&cli.stdout).expect("typed CLI gimfihi output");

    let tool_output = run_tool_gimfihi(
        ToolGimfihiRequest {
            sources: vec![
                ToolGimfihiSource {
                    language: "src".to_owned(),
                    word: "qalma".to_owned(),
                    weight: Some(700),
                },
                ToolGimfihiSource {
                    language: "support".to_owned(),
                    word: "alma".to_owned(),
                    weight: Some(300),
                },
            ],
            scorer: ToolGimfihiScorer::Phonetic,
            c_vwl: 8.0,
            shapes: vec!["cvccv".to_owned()],
            check_collisions: ToolCollisionScope::None,
            count: Some(5),
            format: ToolGimfihiFormat::Json,
            ..ToolGimfihiRequest::default()
        },
        GimfihiSourceWordKind::Ipa,
    )
    .expect("MCP bridge output");
    assert_eq!(tool_output.status, ToolStatus::Success);
    let mcp_output: GimfihiOutput =
        serde_json::from_str(tool_output.stdout_text().expect("MCP bridge UTF-8 output"))
            .expect("typed MCP gimfihi output");

    assert_eq!(
        cli_output
            .candidates
            .iter()
            .map(|candidate| candidate.word.as_str())
            .collect::<Vec<_>>(),
        mcp_output
            .candidates
            .iter()
            .map(|candidate| candidate.word.as_str())
            .collect::<Vec<_>>()
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn parses_vlacku_mixed_repeated_request_order() {
    let Command::Vlacku(input) = Cli::try_parse_from([
        "jbotci",
        "vlacku",
        "--valsi",
        "a",
        "--rafsi",
        "bau",
        "--valsi",
        "klama",
        "--lujvo",
        "mivyselbai",
    ])
    .expect("mixed vlacku requests")
    .command
    else {
        panic!("expected vlacku command");
    };

    assert_eq!(
        input.requests,
        vec![
            VlackuRequest::valsi("a".to_owned()),
            VlackuRequest::rafsi("bau".to_owned()),
            VlackuRequest::valsi("klama".to_owned()),
            VlackuRequest::lujvo("mivyselbai".to_owned()),
        ]
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn parses_embedding_setup_precomputed_policy_and_validation_skip() {
    let Command::Setup(input) = Cli::try_parse_from([
        "jbotci",
        "setup",
        "--embedding",
        "--use-precomputed",
        "always",
        "--skip-validation",
    ])
    .expect("setup command")
    .command
    else {
        panic!("expected setup command");
    };

    assert_eq!(input.use_precomputed, CliUsePrecomputed::Always);
    assert!(input.skip_validation);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn rejects_removed_vlacku_min_match_switch() {
    let error = Cli::try_parse_from(["jbotci", "vlacku", "--min-match", "80", "--valsi", "klama"])
        .expect_err("min-match is no longer accepted");
    assert_eq!(error.kind(), ErrorKind::UnknownArgument);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn rejects_removed_embedding_index_switches() {
    let vlacku_error = Cli::try_parse_from(["jbotci", "vlacku", "--index", "--valsi", "klama"])
        .expect_err("vlacku index is no longer accepted");
    assert_eq!(vlacku_error.kind(), ErrorKind::UnknownArgument);

    let cukta_error = Cli::try_parse_from(["jbotci", "cukta", "--index"])
        .expect_err("cukta index is no longer accepted");
    assert_eq!(cukta_error.kind(), ErrorKind::UnknownArgument);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn rejects_removed_camxes_switches() {
    for args in [
        ["jbotci", "vlasei", "--camxes", "coi"],
        ["jbotci", "gentufa", "--camxes", "coi"],
        ["jbotci", "mulgau", "--camxes", "coi"],
        ["jbotci", "tersmu", "--camxes", "coi"],
        ["jbotci", "zbasu", "--camxes", "coi"],
    ] {
        let error = Cli::try_parse_from(args).expect_err("camxes flag is no longer accepted");
        assert_eq!(error.kind(), ErrorKind::UnknownArgument);
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_positional_query_uses_semantic_search() {
    let run = run_cli_capture_with_embedding_dirs(
        &["jbotci", "vlacku", "going somewhere"],
        false,
        &unique_embedding_test_path("vlacku-query-model-missing"),
        &unique_embedding_test_path("vlacku-query-index-missing"),
    );
    assert_eq!(run.status, CliStatus::InvalidInput);
    assert!(run.stderr.contains("jbotci setup --embedding"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn rejects_vlacku_sound_exclusive_combinations() {
    let cli = Cli::try_parse_from(["jbotci", "vlacku", "--sound", "klama", "--valsi", "klama"])
        .expect("sound combination parses");
    let error = run_cli(cli, &mut Vec::new(), &mut Vec::new(), false)
        .expect_err("sound cannot combine with exact modes");
    assert!(error.to_string().contains("cannot be combined"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn rejects_vlacku_min_similarity_outside_sound_mode() {
    let cli = Cli::try_parse_from([
        "jbotci",
        "vlacku",
        "--min-similarity",
        "80",
        "--valsi",
        "klama",
    ])
    .expect("min-similarity with valsi parses");
    let error = run_cli(cli, &mut Vec::new(), &mut Vec::new(), false)
        .expect_err("min-similarity is sound-only");
    assert!(error.to_string().contains("only valid with `--sound`"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn stable_cli_omits_gerna() {
    assert!(Cli::try_parse_from(["jbotci", "gerna"]).is_err());
    let help = Cli::command().render_long_help().to_string();
    assert!(!help.contains("gerna"));
    assert!(!help.contains("grammar"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn parses_gentufa_formats_and_flags() {
    let Command::Gentufa(default_input) = Cli::try_parse_from(["jbotci", "gentufa", "coi"])
        .expect("default gentufa")
        .command
    else {
        panic!("expected gentufa command")
    };
    assert_eq!(default_input.format, GentufaFormat::Brackets);

    let Command::Gentufa(brackets_input) =
        Cli::try_parse_from(["jbotci", "gentufa", "--turtai", "brackets", "coi"])
            .expect("turtai brackets")
            .command
    else {
        panic!("expected gentufa command")
    };
    assert_eq!(brackets_input.format, GentufaFormat::Brackets);

    let Command::Gentufa(alias_input) =
        Cli::try_parse_from(["jbotci", "gentufa", "--format", "brackets", "coi"])
            .expect("format alias")
            .command
    else {
        panic!("expected gentufa command")
    };
    assert_eq!(alias_input.format, GentufaFormat::Brackets);

    let Command::Gentufa(raw_input) =
        Cli::try_parse_from(["jbotci", "gentufa", "--turtai", "raw", "--show-defs", "coi"])
            .expect("raw with show-defs parses")
            .command
    else {
        panic!("expected gentufa command")
    };
    assert_eq!(raw_input.format, GentufaFormat::Raw);
    assert!(raw_input.show_defs);

    let Command::Gentufa(tree_input) =
        Cli::try_parse_from(["jbotci", "gentufa", "--turtai", "tree", "coi"])
            .expect("tree parses")
            .command
    else {
        panic!("expected gentufa command")
    };
    assert_eq!(tree_input.format, GentufaFormat::Tree);

    let Command::Gentufa(vipcihe_input) =
        Cli::try_parse_from(["jbotci", "gentufa", "--turtai", "vipcihe", "coi"])
            .expect("vipcihe parses")
            .command
    else {
        panic!("expected gentufa command")
    };
    assert_eq!(vipcihe_input.format, GentufaFormat::Tree);

    let Command::Gentufa(defs_input) =
        Cli::try_parse_from(["jbotci", "gentufa", "--show-defs", "coi"])
            .expect("show-defs flag")
            .command
    else {
        panic!("expected gentufa command")
    };
    assert!(defs_input.show_defs);

    let Command::Gentufa(dialect_input) = Cli::try_parse_from([
        "jbotci",
        "gentufa",
        "--dialect",
        "(+ZANTUFA-CONNECTIVES)",
        "coi",
    ])
    .expect("dialect flag parses")
    .command
    else {
        panic!("expected gentufa command")
    };
    assert!(
        dialect_input
            .dialect_definition()
            .expect("dialect definition")
            .features
            .contains(&DialectFeature::ZantufaConnectives)
    );

    let Command::Gentufa(bare_dialect_input) =
        Cli::try_parse_from(["jbotci", "gentufa", "--dialect", "gadganzu", "coi"])
            .expect("bare dialect name parses")
            .command
    else {
        panic!("expected gentufa command")
    };
    assert!(
        bare_dialect_input
            .dialect_definition()
            .expect("bare dialect definition")
            .features
            .contains(&DialectFeature::Gadganzu)
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn tersmu_smusni_cli_output_has_a_single_trailing_newline() {
    // Round-1 review (Codex 3): the delivered CLI surface must be
    // oracle-identical — `render_smusni` already ends in one newline, and the
    // command must not double it.
    let run = run_cli_capture(
        &["jbotci", "tersmu", "--format", "smusni", "mi klama"],
        false,
    );
    assert_eq!(run.status, CliStatus::Success);
    assert!(
        run.stdout.starts_with("SEMANTIC DOCUMENT document_1 {\n"),
        "smusni CLI output should be the notation document, got: {:?}",
        &run.stdout[..run.stdout.len().min(48)]
    );
    assert!(run.stdout.contains("ID PREFIXES: r=reference"));
    // Exactly one trailing newline (the closing `}` then a single `\n`).
    assert!(
        run.stdout.ends_with("}\n"),
        "must end with the closing brace and one newline"
    );
    assert!(
        !run.stdout.ends_with("}\n\n"),
        "must not double the renderer's trailing newline"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn parses_tersmu_formats_with_smusni_default() {
    let Command::Tersmu(default_input) = Cli::try_parse_from(["jbotci", "tersmu", "coi"])
        .expect("default tersmu")
        .command
    else {
        panic!("expected tersmu command")
    };
    assert_eq!(default_input.format, TersmuFormat::Smusni);
    assert!(!default_input.show_defs);

    for (name, expected) in [
        ("json", TersmuFormat::Json),
        ("smusni", TersmuFormat::Smusni),
    ] {
        let Command::Tersmu(input) =
            Cli::try_parse_from(["jbotci", "tersmu", "--format", name, "coi"])
                .expect("supported tersmu format")
                .command
        else {
            panic!("expected tersmu command")
        };
        assert_eq!(input.format, expected);
    }

    // The `lean3` working name was renamed to `smusni`, and the legacy `tree` /
    // `tree+proj` renderers were removed, all with no deprecated alias, so the
    // CLI must reject each retired value as an unknown format.
    for removed in ["lean3", "tree", "tree+proj", "claims", "combined"] {
        assert!(
            Cli::try_parse_from(["jbotci", "tersmu", "--format", removed, "coi"]).is_err(),
            "removed format {removed:?} must be rejected"
        );
    }

    let Command::Tersmu(defs_input) = Cli::try_parse_from([
        "jbotci",
        "tersmu",
        "--show-defs",
        "--format",
        "smusni",
        "coi",
    ])
    .expect("tersmu definitions")
    .command
    else {
        panic!("expected tersmu command")
    };
    assert!(defs_input.show_defs);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn tersmu_help_describes_the_smusni_default() {
    let error = Cli::try_parse_from(["jbotci", "tersmu", "--help"]).expect_err("help");
    assert_eq!(error.kind(), ErrorKind::DisplayHelp);
    let help = error.to_string();
    for marker in [
        "default `smusni` format",
        "flat, self-describing declaration listing",
        "ID-prefix legend",
        "NOT COMPUTED",
        "canonical interchange graph",
    ] {
        assert!(
            help.contains(marker),
            "missing help contract marker {marker:?}"
        );
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn tersmu_show_defs_rejects_json_cli_output() {
    let cli = Cli::try_parse_from([
        "jbotci",
        "tersmu",
        "--show-defs",
        "--format",
        "json",
        "mi",
        "klama",
    ])
    .expect("tersmu JSON definitions parse");
    let error = run_cli(cli, &mut Vec::new(), &mut Vec::new(), false)
        .expect_err("JSON definitions must be rejected");
    assert_eq!(
        error.to_string(),
        "`--show-defs` is not supported with `--format json`"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn tersmu_show_defs_prepends_definitions_before_the_smusni_document() {
    let output = run_success_stdout(&[
        "jbotci",
        "tersmu",
        "--show-defs",
        "--format",
        "smusni",
        "--color=never",
        "ti",
        "klupe",
    ]);

    // The content-word dictionary definitions are prepended ahead of the
    // smusni semantic document that the default format renders.
    let (definitions, document) = output
        .split_once("SEMANTIC DOCUMENT ")
        .expect("smusni document follows the prepended definitions");
    assert!(
        definitions.starts_with("1. klupe | by: officialdata | gismu"),
        "definitions must lead: {definitions:?}"
    );
    assert!(
        !definitions.contains("cmavo:"),
        "tersmu definitions must not define cmavo: {definitions:?}"
    );
    assert!(
        document.contains("ID PREFIXES: r=reference"),
        "the smusni document legend must follow the definitions"
    );
    assert!(document.contains("RELATION: klupe"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn tersmu_outputs_smusni_by_default() {
    let run = run_cli_capture(&["jbotci", "tersmu", "mi", "klama"], false);
    assert_eq!(run.status, CliStatus::Success);
    assert!(run.stderr.is_empty());
    assert!(run.stdout.starts_with("SEMANTIC DOCUMENT "));
    assert!(run.stdout.contains("ID PREFIXES: r=reference"));
    assert!(run.stdout.contains("RELATION: klama"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn tersmu_accepts_explicit_json_format() {
    let run = run_cli_capture(
        &["jbotci", "tersmu", "--format", "json", "ma", "klama"],
        false,
    );
    assert_eq!(run.status, CliStatus::Success);
    assert!(run.stderr.is_empty());
    let json: serde_json::Value = serde_json::from_str(&run.stdout).expect("semantic json");
    let question = json["objects"]
        .as_object()
        .expect("semantic objects")
        .values()
        .find(|object| object["type"] == "question")
        .expect("question");
    assert_eq!(question["kind"], "argument");
    assert!(
        question["slots"][0]["parameter"]
            .as_str()
            .is_some_and(|id| id.starts_with("parameter:"))
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn parses_vlasei_formats_and_rejects_unknown_values() {
    let Command::Vlasei(default_input) = Cli::try_parse_from(["jbotci", "vlasei", "coi"])
        .expect("default vlasei")
        .command
    else {
        panic!("expected vlasei command")
    };
    assert_eq!(default_input.format, VlaseiFormat::Brackets);

    let Command::Vlasei(json_input) =
        Cli::try_parse_from(["jbotci", "vlasei", "--turtai", "json", "coi"])
            .expect("vlasei json")
            .command
    else {
        panic!("expected vlasei command")
    };
    assert_eq!(json_input.format, VlaseiFormat::Json);

    let Command::Vlasei(raw_input) =
        Cli::try_parse_from(["jbotci", "vlasei", "--format", "raw", "coi"])
            .expect("vlasei raw")
            .command
    else {
        panic!("expected vlasei command")
    };
    assert_eq!(raw_input.format, VlaseiFormat::Raw);

    let Command::Vlasei(alias_input) =
        Cli::try_parse_from(["jbotci", "vlasei", "--format", "djeisone", "coi"])
            .expect("vlasei format alias")
            .command
    else {
        panic!("expected vlasei command")
    };
    assert_eq!(alias_input.format, VlaseiFormat::Json);

    let Command::Vlasei(brackets_input) =
        Cli::try_parse_from(["jbotci", "vlasei", "--format", "brackets", "coi"])
            .expect("vlasei brackets")
            .command
    else {
        panic!("expected vlasei command")
    };
    assert_eq!(brackets_input.format, VlaseiFormat::Brackets);

    let Command::Vlasei(tree_input) =
        Cli::try_parse_from(["jbotci", "vlasei", "--format", "tree", "coi"])
            .expect("vlasei tree")
            .command
    else {
        panic!("expected vlasei command")
    };
    assert_eq!(tree_input.format, VlaseiFormat::Tree);

    let Command::Vlasei(ipa_input) =
        Cli::try_parse_from(["jbotci", "vlasei", "--format", "ipa", "coi"])
            .expect("vlasei IPA")
            .command
    else {
        panic!("expected vlasei command")
    };
    assert_eq!(ipa_input.format, VlaseiFormat::Ipa);

    let Command::Vlasei(bare_dialect_input) =
        Cli::try_parse_from(["jbotci", "vlasei", "--dialect", "gadganzu", "coi"])
            .expect("bare vlasei dialect")
            .command
    else {
        panic!("expected vlasei command")
    };
    assert!(
        bare_dialect_input
            .dialect_definition()
            .expect("bare vlasei dialect definition")
            .features
            .contains(&DialectFeature::Gadganzu)
    );

    assert_eq!(
        Cli::try_parse_from(["jbotci", "vlasei", "--turtai", "xml", "coi"])
            .expect_err("unknown vlasei format")
            .kind(),
        ErrorKind::InvalidValue
    );
    assert_eq!(
        Cli::try_parse_from(["jbotci", "vlasei", "--termoha", "json", "coi"])
            .expect_err("old vlasei format option")
            .kind(),
        ErrorKind::UnknownArgument
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn parses_color_policy_values() {
    let default_cli = Cli::try_parse_from(["jbotci", "gentufa", "coi"]).expect("default color");
    assert_eq!(default_cli.color, concolor_clap::ColorChoice::Auto);
    let Command::Gentufa(default_input) = default_cli.command else {
        panic!("expected gentufa command");
    };
    assert!(!default_input.ascii);
    assert!(!default_input.detailed_errors);
    assert_eq!(default_input.error_context, 1);

    let bare_cli =
        Cli::try_parse_from(["jbotci", "gentufa", "--color", "coi"]).expect("bare color");
    assert_eq!(bare_cli.color, concolor_clap::ColorChoice::Always);

    let always_cli =
        Cli::try_parse_from(["jbotci", "gentufa", "--color=always", "coi"]).expect("always color");
    assert_eq!(always_cli.color, concolor_clap::ColorChoice::Always);

    let never_cli =
        Cli::try_parse_from(["jbotci", "gentufa", "--color=never", "coi"]).expect("never color");
    assert_eq!(never_cli.color, concolor_clap::ColorChoice::Never);

    let detailed_cli = Cli::try_parse_from(["jbotci", "gentufa", "--detailed-errors", "coi"])
        .expect("detailed errors");
    let Command::Gentufa(detailed_input) = detailed_cli.command else {
        panic!("expected gentufa command");
    };
    assert!(detailed_input.detailed_errors);

    let context_cli = Cli::try_parse_from(["jbotci", "gentufa", "--error-context", "3", "coi"])
        .expect("error context");
    let Command::Gentufa(context_input) = context_cli.command else {
        panic!("expected gentufa command");
    };
    assert_eq!(context_input.error_context, 3);

    let ascii_cli =
        Cli::try_parse_from(["jbotci", "gentufa", "--ascii", "coi"]).expect("ascii flag");
    let Command::Gentufa(ascii_input) = ascii_cli.command else {
        panic!("expected gentufa command");
    };
    assert!(ascii_input.ascii);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn parses_trace_options_and_aliases() {
    let cli = Cli::try_parse_from([
        "jbotci",
        "gentufa",
        "--trace-phase",
        "all",
        "--trace-limit",
        "7",
        "--trace",
        "argument:3",
        "mi",
        "klama",
    ])
    .expect("trace options");
    let Command::Gentufa(input) = cli.command else {
        panic!("expected gentufa command")
    };
    assert_eq!(input.trace_phase, Some(CliTracePhase::All));
    assert_eq!(input.trace_limit, Some(7));
    assert_eq!(input.trace, Some(Some("argument:3".to_owned())));
    assert_eq!(input.text, vec!["mi".to_owned(), "klama".to_owned()]);

    let alias_cli =
        Cli::try_parse_from(["jbotci", "vlasei", "--plivei", "2", "coi"]).expect("alias");
    let Command::Vlasei(input) = alias_cli.command else {
        panic!("expected vlasei command")
    };
    assert_eq!(input.trace, Some(Some("2".to_owned())));
    assert_eq!(input.text, vec!["coi".to_owned()]);

    let bare = trace_options(&Some(None), TracePhase::Syntax, 7).expect("bare trace");
    assert!(bare.enabled);
    assert_eq!(bare.level, TraceLevel::Top);
    assert_eq!(bare.phase, TracePhase::Syntax);
    assert_eq!(bare.limit, 7);
    assert!(trace_options(&Some(Some("5".to_owned())), TracePhase::Syntax, 7).is_err());
}

#[test]
#[requires(true)]
#[ensures(true)]
fn trace_list_prints_known_filters() {
    let cli =
        Cli::try_parse_from(["jbotci", "gentufa", "--trace-list"]).expect("trace list parses");
    let mut output = Vec::new();
    let mut error = Vec::new();
    let status = run_cli(cli, &mut output, &mut error, false).expect("trace list run");

    assert_eq!(status, CliStatus::Success);
    assert!(error.is_empty());
    let stdout = String::from_utf8(output).expect("stdout utf8");
    assert!(stdout.contains("syntax:"));
    assert!(stdout.contains("- sumti"));
    assert!(stdout.contains("- free modifier"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn trace_context_flags_require_trace_or_trace_list() {
    let cases = [
        (
            ["jbotci", "gentufa", "--trace-phase", "syntax", "coi"].as_slice(),
            "`--trace-phase` requires `--trace` or `--trace-list`",
        ),
        (
            ["jbotci", "gentufa", "--trace-limit", "3", "coi"].as_slice(),
            "`--trace-limit` requires `--trace`",
        ),
        (
            [
                "jbotci",
                "gentufa",
                "--trace-list",
                "--trace",
                "argument:3",
                "coi",
            ]
            .as_slice(),
            "`--trace-list` cannot be combined with `--trace`",
        ),
    ];
    for (args, message) in cases {
        let cli = Cli::try_parse_from(args).expect("trace context parses");
        let error = run_cli(cli, &mut Vec::new(), &mut Vec::new(), false)
            .expect_err("trace context rejected");
        assert!(error.to_string().contains(message), "{error}");
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn trace_phase_is_validated_for_command() {
    let cli = Cli::try_parse_from([
        "jbotci",
        "vlasei",
        "--trace-phase",
        "syntax",
        "--trace",
        "coi",
    ])
    .expect("vlasei trace parses");
    let error = run_cli(cli, &mut Vec::new(), &mut Vec::new(), false)
        .expect_err("syntax trace rejected for vlasei");
    assert!(
        error
            .to_string()
            .contains("`--trace-phase syntax` is not supported with `vlasei`"),
        "{error}"
    );

    let cli = Cli::try_parse_from([
        "jbotci",
        "gentufa",
        "--trace-phase",
        "morphology",
        "--trace-list",
    ])
    .expect("trace list phase parses");
    let mut output = Vec::new();
    let mut error = Vec::new();
    let status = run_cli(cli, &mut output, &mut error, false).expect("trace list run");
    assert_eq!(status, CliStatus::Success);
    assert!(error.is_empty());
    let stdout = String::from_utf8(output).expect("stdout utf8");
    assert!(stdout.contains("morphology:"));
    assert!(!stdout.contains("syntax:"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn rejects_unknown_gentufa_format_and_word_kind_flag() {
    assert_eq!(
        Cli::try_parse_from(["jbotci", "gentufa", "--turtai", "xml", "coi"])
            .expect_err("unknown format")
            .kind(),
        ErrorKind::InvalidValue
    );
    assert_eq!(
        Cli::try_parse_from(["jbotci", "gentufa", "--format", "ipa", "coi"])
            .expect_err("IPA is only a vlasei format")
            .kind(),
        ErrorKind::InvalidValue
    );
    assert_eq!(
        Cli::try_parse_from(["jbotci", "gentufa", "--turtau", "raw", "coi"])
            .expect_err("old gentufa format option")
            .kind(),
        ErrorKind::UnknownArgument
    );
    assert_eq!(
        Cli::try_parse_from(["jbotci", "gentufa", "--wordKind", "coi"])
            .expect_err("wordKind is not supported")
            .kind(),
        ErrorKind::UnknownArgument
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_help_lists_formats_and_brackets_flags() {
    let error = Cli::try_parse_from(["jbotci", "gentufa", "--help"]).expect_err("help");
    assert_eq!(error.kind(), ErrorKind::DisplayHelp);
    let help = error.to_string();
    assert!(help.contains("--turtai"));
    assert!(help.contains("--format"));
    assert!(help.contains("brackets"));
    assert!(help.contains("blocks"));
    assert!(help.contains("tree"));
    assert!(help.contains("vipcihe"));
    assert!(!help.contains("compact"));
    assert!(help.contains("raw"));
    assert!(help.contains("--show-defs"));
    assert!(!help.contains("--skicu"));
    assert!(!help.contains("--defs"));
    assert!(help.contains("--indent"));
    assert!(help.contains("--output-type"));
    assert!(help.contains("--output-file"));
    assert!(!help.contains("--wordKind"));
    assert!(!help.contains("--turtau"));
    assert!(!help.contains("--termoha"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlasei_help_lists_restricted_formats() {
    let error = Cli::try_parse_from(["jbotci", "vlasei", "--help"]).expect_err("help");
    assert_eq!(error.kind(), ErrorKind::DisplayHelp);
    let help = error.to_string();
    assert!(help.contains("--turtai"));
    assert!(help.contains("--format"));
    assert!(!help.contains("plain"));
    assert!(help.contains("brackets"));
    assert!(help.contains("tree"));
    assert!(help.contains("raw"));
    assert!(help.contains("ipa"));
    assert!(help.contains("json"));
    assert!(!help.contains("--turtau"));
    assert!(!help.contains("--termoha"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_default_output_shows_generated_brackets() {
    run_on_normal_stack(|| {
        let cli =
            Cli::try_parse_from(["jbotci", "gentufa", "mi", "klama"]).expect("gentufa default");
        let mut output = Vec::new();
        let mut error = Vec::new();
        run_cli(cli, &mut output, &mut error, false).expect("gentufa run");
        assert!(error.is_empty());
        let output = String::from_utf8(output).expect("utf8");
        assert_eq!(output.trim_end(), "(mi kláma)");
    });
}

const EMPTY_ERASURE_RAW: &str = "RegularText(\n    RegularTextSyntax {\n        leading_nai: [],\n        leading_cmevla: [],\n        leading_indicators: [],\n        leading_free_modifiers: [],\n        leading_connective: None,\n        leading_i_statements: [],\n        paragraphs: None,\n    },\n)\n";

const EMPTY_ERASURE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="68" height="56" viewBox="0 0 68 56" role="img"><title>jbotci gentufa generated syntax</title><style>
@font-face {
  font-family: "Noto Sans";
  src: url("https://cdn.jsdelivr.net/fontsource/fonts/noto-sans:vf@5.2.10/latin-wght-normal.woff2") format("woff2-variations");
  font-weight: 100 900;
  font-style: normal;
}
@font-face {
  font-family: "Noto Sans";
  src: url("https://cdn.jsdelivr.net/fontsource/fonts/noto-sans:vf@5.2.10/latin-wght-italic.woff2") format("woff2-variations");
  font-weight: 100 900;
  font-style: italic;
}
@font-face {
  font-family: "STIX Two Math";
  src: url("https://fonts.gstatic.com/s/stixtwomath/v12/pONg1hwwL_6M9EkZySr_yteUi1o.ttf") format("truetype");
  font-weight: 400;
  font-style: normal;
}
@font-face {
  font-family: "STIX Two Text";
  src: url("https://fonts.gstatic.com/s/stixtwotext/v18/YA9Gr02F12Xkf5whdwKf11l0jbKkeidMTtZ5Yihg2SOY.ttf") format("truetype");
  font-weight: 400;
  font-style: normal;
}
@font-face {
  font-family: "STIX Two Text";
  src: url("https://fonts.gstatic.com/s/stixtwotext/v18/YA9Gr02F12Xkf5whdwKf11l0jbKkeidMTtZ5YiiH3iOY.ttf") format("truetype");
  font-weight: 700;
  font-style: normal;
}</style><rect x="0" y="0" width="68" height="56" fill="#ffffff"/></svg>"##;

const EMPTY_ERASURE_PNG_BASE64: &str = "iVBORw0KGgoAAAANSUhEUgAAAIgAAABwCAYAAADFezgmAAABCklEQVR4nO3SAQ0AIBAAoZ+zf2W1gJcAMrDPM/CxBoIgJEFIgpAEIQlCEoQkCEkQkiAkQUiCkAQhCUIShCQISRCSICRBSIKQBCEJQhKEJAhJEJIgJEFIgpAEIQlCEoQkCEkQkiAkQUiCkAQhCUIShCQISRCSICRBSIKQBCEJQhKEJAhJEJIgJEFIgpAEIQlCEoQkCEkQkiAkQUiCkAQhCUIShCQISRCSICRBSIKQBCEJQhKEJAhJEJIgJEFIgpAEIQlCEoQkCEkQkiAkQUiCkAQhCUIShCQISRCSICRBSIKQBCEJQhKEJAhJEJIgJEFIgpAEIQlCEoQkCEkQkiAkQUiCkAQhCUIShHQBRQgE3zTiLRgAAAAASUVORK5CYII=";

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_empty_erasure_has_exact_output_in_every_format() {
    run_on_normal_stack(|| {
        let source = "le broda sa le si";

        assert_eq!(
            run_success_bytes(&["jbotci", "gentufa", source]),
            b"Text {}\n"
        );
        assert_eq!(
            run_success_bytes(&["jbotci", "gentufa", "--turtai", "brackets", source]),
            b"Text {}\n"
        );
        assert_eq!(
            run_success_bytes(&["jbotci", "gentufa", "--turtai", "tree", source]),
            b"Text {}\n"
        );
        assert_eq!(
            run_success_bytes(&["jbotci", "gentufa", "--turtai", "raw", source]),
            EMPTY_ERASURE_RAW.as_bytes()
        );
        assert_eq!(
            run_success_bytes(&["jbotci", "gentufa", "--turtai", "json", source]),
            b"{\"RegularText\": {}}\n"
        );
        assert_eq!(
            run_success_bytes(&["jbotci", "gentufa", "--turtai", "blocks", source]),
            EMPTY_ERASURE_SVG.as_bytes()
        );
        assert_eq!(
            run_success_bytes(&[
                "jbotci",
                "gentufa",
                "--turtai",
                "blocks",
                "--output-type",
                "png",
                source,
            ]),
            base64::engine::general_purpose::STANDARD
                .decode(EMPTY_ERASURE_PNG_BASE64)
                .expect("empty-text PNG expectation is valid base64")
        );
    });
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_adjacent_empty_derivations_use_the_explicit_representation() {
    run_on_normal_stack(|| {
        for source in [
            "si",
            "sa",
            "su",
            "si si",
            "sa si",
            "le si   ",
            "le broda su",
        ] {
            assert_eq!(
                run_success_bytes(&["jbotci", "gentufa", source]),
                b"Text {}\n",
                "{source:?}"
            );
        }
    });
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_su_preserves_niho_and_lu_boundaries_in_exact_output() {
    run_on_normal_stack(|| {
        for (args, expected_stdout) in [
            (
                [
                    "jbotci", "gentufa", "--format", "brackets", "mi", "klama", "ni'o", "do",
                    "tavla", "su", "do", "cusku",
                ]
                .as_slice(),
                "([mi kláma] [ni'o {do cúsku}])\n",
            ),
            (
                [
                    "jbotci", "gentufa", "--format", "brackets", "lu", "mi", "klama", "su", "do",
                    "cusku", "li'u",
                ]
                .as_slice(),
                "(lu [do cúsku] li'u)\n",
            ),
        ] {
            let cli = Cli::try_parse_from(args).expect("gentufa SU boundary input");
            let mut output = Vec::new();
            let mut error = Vec::new();
            run_cli(cli, &mut output, &mut error, false).expect("gentufa SU boundary run");

            assert!(error.is_empty());
            assert_eq!(
                String::from_utf8(output).expect("stdout utf8"),
                expected_stdout
            );
        }
    });
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_mahoi_quotes_have_exact_bracket_output() {
    run_on_normal_stack(|| {
        for (args, expected_stdout) in [
            (
                ["jbotci", "gentufa", "--turtai", "brackets", "ma'oi", "ba"].as_slice(),
                "(ma'oĭ ba)\n",
            ),
            (
                ["jbotci", "gentufa", "--turtai", "brackets", "ma'oi", "pu"].as_slice(),
                "(ma'oĭ pu)\n",
            ),
            (
                [
                    "jbotci", "gentufa", "--turtai", "brackets", "mi", "cusku", "ma'oi", "ba",
                ]
                .as_slice(),
                "(mi [cúsku {ma'oĭ ba}])\n",
            ),
        ] {
            let run = run_cli_capture(args, false);
            assert_eq!(run.status, CliStatus::Success, "{args:?}");
            assert_eq!(run.stdout, expected_stdout, "{args:?}");
            // `ma'oi` is experimental syntax: a valid parse stays successful but
            // must still surface the experimental-cmavo warning on stderr.
            assert!(
                run.stderr.contains("syntax.warning.experimental-cmavo"),
                "{args:?}: {}",
                run.stderr
            );
            assert!(run.stderr.contains("ma'oi"), "{args:?}: {}", run.stderr);
        }
    });
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_blocks_stdout_defaults_to_svg() {
    run_on_normal_stack(|| {
        let output = run_success_bytes(&["jbotci", "gentufa", "--format", "blocks", "mi", "klama"]);
        let svg = String::from_utf8(output).expect("SVG is UTF-8");
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("<text"));
        assert!(svg.contains("@font-face"));
        assert!(!svg.ends_with('\n'));
    });
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_blocks_svg_renders_composite_morphology_components() {
    run_on_normal_stack(|| {
        let quote = run_success_bytes(&[
            "jbotci",
            "gentufa",
            "--format",
            "blocks",
            "mi klama zoi gy house gy",
        ]);
        let quote = String::from_utf8(quote).expect("SVG is UTF-8");
        for text in ["mi", "kláma", "zoĭ", "gy", "house", "quote"] {
            assert!(
                quote.contains(&format!(">{text}</text>")),
                "{text}: {quote}"
            );
        }

        let compound = run_success_bytes(&[
            "jbotci",
            "gentufa",
            "--format",
            "blocks",
            "mi bakni zei kanla",
        ]);
        let compound = String::from_utf8(compound).expect("SVG is UTF-8");
        for text in ["mi", "bákni", "zeĭ", "kánla", "tanru unit"] {
            assert!(
                compound.contains(&format!(">{text}</text>")),
                "{text}: {compound}"
            );
        }
    });
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_blocks_output_file_extension_infers_svg_and_png() {
    run_on_normal_stack(|| {
        let svg_path = unique_cli_output_path("gentufa-blocks-inferred-svg", "svg");
        let png_path = unique_cli_output_path("gentufa-blocks-inferred-png", "png");
        let _ = fs::remove_file(&svg_path);
        let _ = fs::remove_file(&png_path);

        let svg_arg = svg_path.to_string_lossy().into_owned();
        let png_arg = png_path.to_string_lossy().into_owned();
        let mut svg_stdout = Vec::new();
        let mut svg_stderr = Vec::new();
        let svg_cli = Cli::try_parse_from([
            "jbotci",
            "gentufa",
            "--format",
            "blocks",
            "--output-file",
            svg_arg.as_str(),
            "mi",
            "klama",
        ])
        .expect("SVG output-file args parse");
        let svg_status =
            run_cli(svg_cli, &mut svg_stdout, &mut svg_stderr, false).expect("SVG run");
        assert_eq!(svg_status, CliStatus::Success);
        assert!(svg_stdout.is_empty());
        assert!(
            svg_stderr.is_empty(),
            "{}",
            String::from_utf8_lossy(&svg_stderr)
        );
        let svg = fs::read_to_string(&svg_path).expect("SVG output file");
        assert!(svg.starts_with("<svg"));

        let mut png_stdout = Vec::new();
        let mut png_stderr = Vec::new();
        let png_cli = Cli::try_parse_from([
            "jbotci",
            "gentufa",
            "--format",
            "blocks",
            "--output-file",
            png_arg.as_str(),
            "mi",
            "klama",
        ])
        .expect("PNG output-file args parse");
        let png_status =
            run_cli(png_cli, &mut png_stdout, &mut png_stderr, false).expect("PNG run");
        assert_eq!(png_status, CliStatus::Success);
        assert!(png_stdout.is_empty());
        assert!(
            png_stderr.is_empty(),
            "{}",
            String::from_utf8_lossy(&png_stderr)
        );
        let png = fs::read(&png_path).expect("PNG output file");
        assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));

        let _ = fs::remove_file(svg_path);
        let _ = fs::remove_file(png_path);
    });
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_blocks_explicit_output_type_wins_over_extension() {
    run_on_normal_stack(|| {
        let path = unique_cli_output_path("gentufa-blocks-explicit-png", "svg");
        let _ = fs::remove_file(&path);
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let cli = Cli::try_parse_from([
            "jbotci",
            "gentufa",
            "--format",
            "blocks",
            "--output-type",
            "png",
            "--output-file",
            path_arg.as_str(),
            "mi",
            "klama",
        ])
        .expect("explicit PNG args parse");
        let status = run_cli(cli, &mut stdout, &mut stderr, false).expect("explicit PNG run");
        assert_eq!(status, CliStatus::Success);
        assert!(stdout.is_empty());
        assert!(stderr.is_empty(), "{}", String::from_utf8_lossy(&stderr));
        let png = fs::read(&path).expect("PNG output file");
        assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
        let _ = fs::remove_file(path);
    });
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_blocks_png_stdout_is_binary_without_added_newline() {
    run_on_normal_stack(|| {
        let output = run_success_bytes(&[
            "jbotci",
            "gentufa",
            "--format",
            "blocks",
            "--output-type",
            "png",
            "mi",
            "klama",
        ]);
        assert!(output.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert_ne!(output.last().copied(), Some(b'\n'));
    });
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_blocks_unknown_extension_requires_explicit_output_type() {
    let path = unique_cli_output_path("gentufa-blocks-unknown-extension", "dat");
    let path_arg = path.to_string_lossy().into_owned();
    let cli = Cli::try_parse_from([
        "jbotci",
        "gentufa",
        "--format",
        "blocks",
        "--output-file",
        path_arg.as_str(),
        "mi",
        "klama",
    ])
    .expect("unknown extension args parse");
    let error = run_cli(cli, &mut Vec::new(), &mut Vec::new(), false)
        .expect_err("unknown extension rejected");
    assert!(
        error
            .to_string()
            .contains("cannot infer gentufa blocks output type")
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_blocks_rejects_text_only_options() {
    assert_gentufa_error_contains(
        &[
            "jbotci",
            "gentufa",
            "--format",
            "blocks",
            "--show-defs",
            "mi",
            "klama",
        ],
        "`--show-defs`",
    );
    assert_gentufa_error_contains(
        &[
            "jbotci", "gentufa", "--format", "blocks", "--indent", "2", "mi", "klama",
        ],
        "`--indent`",
    );
    assert_gentufa_error_contains(
        &[
            "jbotci",
            "gentufa",
            "--format",
            "blocks",
            "--show-spans",
            "mi",
            "klama",
        ],
        "`--show-spans`",
    );
    assert_gentufa_error_contains(
        &[
            "jbotci",
            "gentufa",
            "--format",
            "blocks",
            "--show-refs",
            "mi",
            "klama",
        ],
        "`--show-refs`",
    );
    assert_gentufa_error_contains(
        &[
            "jbotci",
            "gentufa",
            "--format",
            "blocks",
            "--decompose-lujvo",
            "mi",
            "klama",
        ],
        "`--decompose-lujvo`",
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlasei_json_outputs_compact_morphology() {
    let cli =
        Cli::try_parse_from(["jbotci", "vlasei", "--turtai", "json", "coi"]).expect("vlasei json");
    let mut output = Vec::new();
    let mut error = Vec::new();
    run_cli(cli, &mut output, &mut error, false).expect("vlasei json run");
    assert!(error.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&output).expect("valid uncolored JSON");

    assert_eq!(value[0]["PlainWord"]["Cmavo"]["phonemes"], "coĭ");
    assert_eq!(
        value[0]["PlainWord"]["Cmavo"]["span"],
        serde_json::json!([0, 3])
    );
    assert!(
        String::from_utf8(output)
            .expect("utf8")
            .contains("\"PlainWord\"")
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlasei_ipa_outputs_pronunciation_surface() {
    let cli = Cli::try_parse_from([
        "jbotci", "vlasei", "--format", "ipa", "mi", "klama", "le", "zarci",
    ])
    .expect("vlasei IPA");
    let mut output = Vec::new();
    let mut error = Vec::new();
    run_cli(cli, &mut output, &mut error, false).expect("vlasei IPA run");

    assert!(error.is_empty());
    assert_eq!(
        String::from_utf8(output).expect("stdout utf8"),
        "mi ˈkla.ma le ˈzar.ʃi\n"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlasei_cgv_warning_keeps_json_stdout_clean() {
    let cli = Cli::try_parse_from(["jbotci", "vlasei", "--format", "json", "siatl."])
        .expect("vlasei json");
    let mut output = Vec::new();
    let mut error = Vec::new();
    let status = run_cli(cli, &mut output, &mut error, false).expect("vlasei run");

    assert_eq!(status, CliStatus::Success);
    let stdout = String::from_utf8(output).expect("stdout utf8");
    let _json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    let stderr = String::from_utf8(error).expect("stderr utf8");
    assert!(stderr.contains("morphology.warning.experimental-cgv"));
    assert!(stderr.contains("experimental morphology"));
    assert!(!stdout.contains("morphology.warning.experimental-cgv"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlasei_mz_warning_keeps_json_stdout_clean() {
    let cli = Cli::try_parse_from(["jbotci", "vlasei", "--format", "json", "namzi"])
        .expect("vlasei json");
    let mut output = Vec::new();
    let mut error = Vec::new();
    let status = run_cli(cli, &mut output, &mut error, false).expect("vlasei run");

    assert_eq!(status, CliStatus::Success);
    let stdout = String::from_utf8(output).expect("stdout utf8");
    let _json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    let stderr = String::from_utf8(error).expect("stderr utf8");
    assert!(stderr.contains("morphology.warning.experimental-mz"));
    assert!(stderr.contains("experimental morphology: MZ consonant pair"));
    assert!(!stdout.contains("morphology.warning.experimental-mz"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlasei_detailed_error_reports_xlaglymlu_lujvo_progress() {
    let run = run_cli_capture(
        &["jbotci", "vlasei", "--detailed-errors", "xlaglymlu"],
        false,
    );

    assert_eq!(run.status, CliStatus::Failure);
    assert!(run.stdout.contains('‼'), "{}", run.stdout);
    assert!(run.stderr.contains("morphology.slinkuhi"));
    assert!(run.stderr.contains("slinku'i"));
    assert!(run.stderr.contains("{xlaglymlu}"));
    assert!(run.stderr.contains("while parsing fu'ivla"));
    assert!(
        !run.stderr
            .contains("reason: word is not a valid Lojban word")
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlasei_detailed_error_reports_zoi_delimiter_reason() {
    let run = run_cli_capture(&["jbotci", "vlasei", "--detailed-errors", "zoi"], false);

    assert_eq!(run.status, CliStatus::Failure);
    assert!(run.stdout.contains('‼'), "{}", run.stdout);
    assert!(run.stderr.contains("morphology.invalid-zoi-delimiter"));
    assert!(run.stderr.contains("ZOI requires an"));
    let compact_stderr = run.stderr.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(compact_stderr.contains("opening delimiter word after the quote marker"));
    assert!(!compact_stderr.contains("reason: ZOI delimiter must be a single non-y word"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlasei_raw_output_is_debug_morphology() {
    let cli =
        Cli::try_parse_from(["jbotci", "vlasei", "--format", "raw", "coi"]).expect("vlasei raw");
    let mut output = Vec::new();
    let mut error = Vec::new();
    run_cli(cli, &mut output, &mut error, false).expect("vlasei raw run");
    assert!(error.is_empty());
    let output = String::from_utf8(output).expect("utf8");

    assert!(output.starts_with("[\n"));
    assert!(output.contains("PlainWord("));
    assert!(output.contains("Cmavo"));
    assert!(output.contains("Phonemes"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlasei_raw_indent_zero_uses_compact_debug() {
    let cli = Cli::try_parse_from([
        "jbotci", "vlasei", "--format", "raw", "--indent", "0", "coi",
    ])
    .expect("vlasei raw indent zero");
    let mut output = Vec::new();
    let mut error = Vec::new();
    run_cli(cli, &mut output, &mut error, false).expect("vlasei raw run");
    assert!(error.is_empty());
    let output = String::from_utf8(output).expect("utf8");

    assert!(!output.trim_end().contains('\n'));
    assert!(output.starts_with("[PlainWord("));
    assert!(output.contains("PlainWord("));
    assert!(output.contains("Cmavo"));
    assert!(output.contains("Phonemes"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlasei_raw_rejects_nonzero_indent() {
    let cli = Cli::try_parse_from([
        "jbotci", "vlasei", "--format", "raw", "--indent", "2", "coi",
    ])
    .expect("vlasei raw indent parses");
    let error = run_cli(cli, &mut Vec::new(), &mut Vec::new(), false)
        .expect_err("raw nonzero indent rejected");
    assert!(error.to_string().contains("only supports `0`"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlasei_projection_flags_affect_non_raw_output() {
    let cli = Cli::try_parse_from([
        "jbotci",
        "vlasei",
        "--format",
        "tree",
        "--mark-stress",
        "none",
        "--mark-glides",
        "none",
        "coi",
        "klama",
    ])
    .expect("vlasei projection flags parse");
    let mut output = Vec::new();
    let mut error = Vec::new();
    run_cli(cli, &mut output, &mut error, false).expect("vlasei tree run");
    assert!(error.is_empty());
    let output = String::from_utf8(output).expect("utf8");
    assert!(output.contains("Cmavo \"coi\""));
    assert!(output.contains("Gismu \"klama\""));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlasei_morphology_errors_go_to_stderr() {
    let cli = Cli::try_parse_from(["jbotci", "vlasei", "aa"]).expect("vlasei parses");
    let mut output = Vec::new();
    let mut error = Vec::new();
    let status = run_cli(cli, &mut output, &mut error, false).expect("vlasei run");

    assert_eq!(status, CliStatus::Failure);
    let stdout = std::str::from_utf8(&output).expect("stdout utf8");
    assert!(stdout.contains('‼'), "{stdout}");
    let stderr = String::from_utf8(error).expect("stderr utf8");
    assert!(stderr.contains("morphology.vowel-hiatus"));
    assert!(stderr.contains("vowels in hiatus are not allowed"));
    assert!(stderr.contains("aa"));
    assert!(!stderr.contains("jbotci:"));
    assert!(!stderr.contains("\x1b["));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn raw_rejects_projection_flags() {
    let cli = Cli::try_parse_from([
        "jbotci",
        "gentufa",
        "--format",
        "raw",
        "--mark-stress",
        "none",
        "mi",
        "klama",
    ])
    .expect("gentufa raw projection flag parses");
    let error = run_cli(cli, &mut Vec::new(), &mut Vec::new(), false)
        .expect_err("raw projection flags rejected");
    assert!(error.to_string().contains("not supported with raw output"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn ascii_rejects_incompatible_diacritic_flags() {
    let stress_cli = Cli::try_parse_from([
        "jbotci",
        "gentufa",
        "--ascii",
        "--format",
        "tree",
        "--mark-stress",
        "acute",
        "mi",
        "klama",
    ])
    .expect("ASCII stress conflict parses");
    let error = run_cli(stress_cli, &mut Vec::new(), &mut Vec::new(), false)
        .expect_err("ASCII stress conflict rejected");
    assert!(error.to_string().contains("`--ascii`"));
    assert!(error.to_string().contains("`--mark-stress acute`"));

    let glide_cli = Cli::try_parse_from([
        "jbotci",
        "vlasei",
        "--ascii",
        "--format",
        "tree",
        "--mark-glides",
        "breve",
        "coi",
    ])
    .expect("ASCII glide conflict parses");
    let error = run_cli(glide_cli, &mut Vec::new(), &mut Vec::new(), false)
        .expect_err("ASCII glide conflict rejected");
    assert!(error.to_string().contains("`--mark-glides breve`"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlasei_ipa_rejects_ascii_output() {
    let cli = Cli::try_parse_from([
        "jbotci", "vlasei", "--ascii", "--format", "ipa", "mi", "klama",
    ])
    .expect("vlasei IPA ASCII parses");
    let error =
        run_cli(cli, &mut Vec::new(), &mut Vec::new(), false).expect_err("ASCII IPA rejected");

    assert!(error.to_string().contains("`--ascii`"));
    assert!(error.to_string().contains("`--turtai ipa`"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlasei_ipa_rejects_phoneme_projection_flags() {
    let cli = Cli::try_parse_from([
        "jbotci",
        "vlasei",
        "--format",
        "ipa",
        "--mark-stress",
        "none",
        "mi",
        "klama",
    ])
    .expect("vlasei IPA projection flag parses");
    let error = run_cli(cli, &mut Vec::new(), &mut Vec::new(), false)
        .expect_err("IPA projection flags rejected");

    assert!(error.to_string().contains("`--mark-stress`"));
    assert!(error.to_string().contains("IPA output"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn ascii_accepts_compatible_diacritic_flags() {
    let output = run_success_stdout(&[
        "jbotci",
        "gentufa",
        "--ascii",
        "--format",
        "tree",
        "--mark-stress",
        "none",
        "--mark-glides",
        "none",
        "mi",
        "klama",
    ]);

    assert!(output.contains("Gismu \"klama\""));
    assert!(!output.contains("kláma"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn ascii_affects_human_and_json_outputs() {
    let gentufa_tree = run_success_stdout(&[
        "jbotci",
        "gentufa",
        "--ascii",
        "--format",
        "tree",
        "--show-spans",
        "--show-refs",
        "mi",
        "klama",
        "do",
    ]);
    assert!(gentufa_tree.contains("Cmavo @[0..2) \"mi\""));
    assert!(gentufa_tree.contains("Gismu @[3..8) \"klama\""));
    assert!(!gentufa_tree.contains('→'));
    assert!(!gentufa_tree.contains('‥'));
    assert!(!gentufa_tree.contains('á'));

    let gentufa_brackets = run_success_stdout(&[
        "jbotci",
        "gentufa",
        "--ascii",
        "--format",
        "brackets",
        "--decompose-lujvo",
        "mivyselbai",
    ]);
    assert!(gentufa_brackets.contains("miv~y~sel~bai"));

    let gentufa_json = run_success_stdout(&[
        "jbotci", "gentufa", "--ascii", "--format", "json", "coi", "klama",
    ]);
    assert!(gentufa_json.contains("\"phonemes\": \"coi\""));
    assert!(gentufa_json.contains("\"phonemes\": \"klama\""));

    let vlasei_tree = run_success_stdout(&[
        "jbotci",
        "vlasei",
        "--ascii",
        "--format",
        "tree",
        "--show-spans",
        "coi",
        "klama",
    ]);
    assert!(vlasei_tree.contains("Cmavo @[0..3) \"coi\""));
    assert!(vlasei_tree.contains("Gismu @[4..9) \"klama\""));

    let vlasei_brackets = run_success_stdout(&[
        "jbotci",
        "vlasei",
        "--ascii",
        "--format",
        "brackets",
        "--decompose-lujvo",
        "mivyselbai",
    ]);
    assert!(vlasei_brackets.contains("miv~y~sel~bai"));

    let vlasei_json = run_success_stdout(&[
        "jbotci", "vlasei", "--ascii", "--format", "json", "coi", "klama",
    ]);
    assert!(vlasei_json.contains("\"phonemes\": \"coi\""));
    assert!(vlasei_json.contains("\"phonemes\": \"klama\""));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn show_refs_is_tree_only() {
    let cli = Cli::try_parse_from([
        "jbotci",
        "gentufa",
        "--format",
        "brackets",
        "--show-refs",
        "mi",
        "klama",
    ])
    .expect("gentufa show refs flag parses");
    let error = run_cli(cli, &mut Vec::new(), &mut Vec::new(), false)
        .expect_err("show refs rejected for non-tree output");
    assert!(error.to_string().contains("`--show-refs`"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn tree_show_spans_and_lujvo_decomposition() {
    let cli = Cli::try_parse_from([
        "jbotci",
        "vlasei",
        "--format",
        "tree",
        "--show-spans",
        "--decompose-lujvo",
        "mivyselbai",
    ])
    .expect("vlasei tree span flags parse");
    let mut output = Vec::new();
    let mut error = Vec::new();
    run_cli(cli, &mut output, &mut error, false).expect("vlasei tree run");
    assert!(error.is_empty());
    let output = String::from_utf8(output).expect("utf8");
    assert!(output.contains("Lujvo @[0‥10)"));
    assert!(output.contains("miv·y·sél·baĭ"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_json_outputs_typed_syntax_tree() {
    run_on_normal_stack(|| {
        let cli = Cli::try_parse_from(["jbotci", "gentufa", "--format", "djeisone", "mi", "klama"])
            .expect("gentufa json");
        let mut output = Vec::new();
        let mut error = Vec::new();
        run_cli(cli, &mut output, &mut error, false).expect("gentufa json run");
        assert!(error.is_empty());
        let text = String::from_utf8(output).expect("utf8");
        let value: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");

        assert!(value.get("leading_nai").is_none());
        assert!(value["RegularText"]["paragraphs"].as_object().is_some());
        assert!(text.contains("\"BridiStatement\""));
        assert!(!text.contains("\"constructor\""));
        assert!(!text.contains("\"kind\": \"node\""));
        assert!(!text.contains("\"leadingNai\""));
    });
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_morphology_warnings_go_to_stderr() {
    run_on_normal_stack(|| {
        let cli = Cli::try_parse_from([
            "jbotci", "gentufa", "--format", "json", "la", "siatl.", "cu", "klama",
        ])
        .expect("gentufa json");
        let mut output = Vec::new();
        let mut error = Vec::new();
        run_cli(cli, &mut output, &mut error, false).expect("gentufa run");

        let stdout = String::from_utf8(output).expect("stdout utf8");
        let _json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
        let stderr = String::from_utf8(error).expect("stderr utf8");
        assert!(stderr.contains("morphology.warning.experimental-cgv"));
        assert!(stderr.contains("experimental morphology"));
        assert!(!stdout.contains("morphology.warning.experimental-cgv"));
    });
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_tree_outputs_generated_syntax_tree() {
    run_on_normal_stack(|| {
        let cli = Cli::try_parse_from(["jbotci", "gentufa", "--format", "tree", "mi", "klama"])
            .expect("gentufa tree");
        let mut output = Vec::new();
        let mut error = Vec::new();
        run_cli(cli, &mut output, &mut error, false).expect("gentufa tree run");
        assert!(error.is_empty());
        let output = String::from_utf8(output).expect("utf8");

        assert!(output.starts_with("BridiWithLeadingTerms {\n"));
        assert!(output.contains("leading_terms: ["));
        assert!(output.contains("Cmavo \"mi\""));
        assert!(output.contains("bridi_tail: Gismu \"kláma\""));
    });
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_tree_preserves_source_order_for_selbri_connection() {
    run_on_normal_stack(|| {
        let cli = Cli::try_parse_from([
            "jbotci", "gentufa", "--format", "tree", "gleki", "je", "klama",
        ])
        .expect("gentufa tree");
        let mut output = Vec::new();
        let mut error = Vec::new();
        run_cli(cli, &mut output, &mut error, false).expect("gentufa tree run");
        assert!(error.is_empty());
        let output = String::from_utf8(output).expect("utf8");

        let leading = output.find("Gismu \"gléki\"").expect("leading selbri");
        let connective = output.find("Cmavo \"je\"").expect("connective");
        let trailing = output.find("Gismu \"kláma\"").expect("trailing selbri");
        assert!(leading < connective);
        assert!(connective < trailing);
    });
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_tree_preserves_source_order_for_binary_math() {
    run_on_normal_stack(|| {
        let cli = Cli::try_parse_from([
            "jbotci", "gentufa", "--format", "tree", "li", "pa", "su'i", "re",
        ])
        .expect("gentufa tree");
        let mut output = Vec::new();
        let mut error = Vec::new();
        run_cli(cli, &mut output, &mut error, false).expect("gentufa tree run");
        assert!(error.is_empty());
        let output = String::from_utf8(output).expect("utf8");

        let left = output.find("Cmavo \"pa\"").expect("left expression");
        let operator = output.find("Cmavo \"su'i\"").expect("operator");
        let right = output.find("Cmavo \"re\"").expect("right expression");
        assert!(left < operator);
        assert!(operator < right);
    });
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_indent_zero_makes_tree_single_line() {
    run_on_normal_stack(|| {
        let cli = Cli::try_parse_from([
            "jbotci", "gentufa", "--format", "tree", "--indent", "0", "mi", "klama",
        ])
        .expect("gentufa tree indent zero");
        let mut output = Vec::new();
        let mut error = Vec::new();
        run_cli(cli, &mut output, &mut error, false).expect("gentufa tree run");
        assert!(error.is_empty());
        let output = String::from_utf8(output).expect("utf8");
        assert_eq!(
            output.trim_end(),
            r#"BridiWithLeadingTerms{leading_terms:[Cmavo "mi"],bridi_tail:Gismu "kláma"}"#
        );
    });
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_indent_zero_makes_json_single_line() {
    run_on_normal_stack(|| {
        let cli = Cli::try_parse_from([
            "jbotci", "gentufa", "--format", "json", "--indent", "0", "mi", "klama",
        ])
        .expect("gentufa json indent zero");
        let mut output = Vec::new();
        let mut error = Vec::new();
        run_cli(cli, &mut output, &mut error, false).expect("gentufa json run");
        assert!(error.is_empty());
        let output = String::from_utf8(output).expect("utf8");
        assert!(!output.trim_end().contains('\n'));
        let _: serde_json::Value = serde_json::from_str(&output).expect("valid JSON");
    });
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_warnings_go_to_stderr() {
    run_on_normal_stack(|| {
        let cli = Cli::try_parse_from([
            "jbotci", "gentufa", "--format", "djeisone", "mi", "klama", "fi'oi", "broda",
        ])
        .expect("gentufa warning parse");
        let mut output = Vec::new();
        let mut error = Vec::new();
        run_cli(cli, &mut output, &mut error, false).expect("gentufa warning run");

        let stdout = String::from_utf8(output).expect("stdout utf8");
        let stderr = String::from_utf8(error).expect("stderr utf8");
        assert!(stdout.starts_with('{'));
        assert!(!stdout.contains("warning:"));
        assert!(stderr.contains("experimental syntax"), "{stderr}");
        assert!(stderr.contains("syntax.warning.experimental-fihoi-adverbial"));
        assert!(stderr.contains("FIhOI bridi/subbridi adverbial term"));
        assert!(stderr.contains("fi'oi"));
    });
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_bare_nahe_sumti_without_bo_warning_goes_to_stderr() {
    run_on_normal_stack(|| {
        // Bare `na'e <sumti>` without `bo` is a valid parse (Success) that carries the
        // experimental without-`bo` warning; the warning must surface on stderr.
        let cli = Cli::try_parse_from([
            "jbotci", "gentufa", "--format", "brackets", "mi", "viska", "na'e", "lo", "mlatu",
        ])
        .expect("gentufa nahe warning parse");
        let mut output = Vec::new();
        let mut error = Vec::new();
        let status = run_cli(cli, &mut output, &mut error, false).expect("gentufa nahe run");
        assert_eq!(status, CliStatus::Success);

        let stdout = String::from_utf8(output).expect("stdout utf8");
        let stderr = String::from_utf8(error).expect("stderr utf8");
        assert!(!stdout.contains("warning:"));
        assert!(stderr.contains("experimental syntax"), "{stderr}");
        assert!(
            stderr.contains("syntax.warning.experimental-nahe-sumti-without-bo"),
            "{stderr}"
        );
        assert!(stderr.contains("NAhE before sumti without BO"), "{stderr}");
        assert!(stderr.contains("na'e"), "{stderr}");
    });
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_syntax_errors_go_to_stderr() {
    run_on_normal_stack(|| {
        let cli =
            Cli::try_parse_from(["jbotci", "gentufa", "gleki", "ku", "klama", "zei", "klama"])
                .expect("gentufa parses");
        let mut output = Vec::new();
        let mut error = Vec::new();
        let status = run_cli(cli, &mut output, &mut error, false).expect("gentufa run");

        assert_eq!(status, CliStatus::Failure);
        let stdout = std::str::from_utf8(&output).expect("stdout utf8");
        assert!(stdout.contains('‼'), "{stdout}");
        let stderr = String::from_utf8(error).expect("stderr utf8");
        assert!(stderr.contains("syntax.unexpected-cmavo"), "{stderr}");
        assert!(stderr.contains("unexpected cmavo"));
        assert!(
            stderr.contains(
                "expected: free modifier, joik, linked arguments, forethought selbri connective",
            ),
            "{stderr}"
        );
        assert!(
            stderr.contains("while parsing BO-grouped tanru unit"),
            "{stderr}"
        );
        assert!(!stderr.contains("expected one of:"));
        assert!(!stderr.contains("needs one of:"));
        assert!(!stderr.contains("{be}"));
        assert!(!stderr.contains("BRIVLA"));
        assert!(stderr.contains("ku"));
        assert!(!stderr.contains("jbotci:"));
        assert!(!stderr.contains("\x1b["));
    });
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_syntax_error_uses_explicit_diagnostic_width() {
    run_on_normal_stack(|| {
        let cli = Cli::try_parse_from(["jbotci", "gentufa", "--detailed-errors", "mi", "cu"])
            .expect("gentufa parses");
        let mut output = Vec::new();
        let mut error = Vec::new();
        let status = run_cli_with_color_policy_and_width(
            cli,
            &mut output,
            &mut error,
            CliColorPolicy::same(false),
            40,
        )
        .expect("gentufa run");

        assert_eq!(status, CliStatus::Failure);
        let stdout = std::str::from_utf8(&output).expect("stdout utf8");
        assert!(stdout.contains('‼'), "{stdout}");
        let stderr = String::from_utf8(error).expect("stderr utf8");
        assert!(stderr.contains("expected: free modifier, terms"));
        assert!(stderr.contains("bridi tail"));
        assert!(stderr.contains("while parsing bridi"));
        assert!(!stderr.contains("expected one of:"));
        assert!(stderr.contains("\n            "));
        assert!(!stderr.contains("\x1b["));
    });
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_detailed_syntax_errors_use_specific_codes() {
    run_on_normal_stack(|| {
        for (source, code, message) in [
            (&["ku"][..], "syntax.unexpected-cmavo", "unexpected cmavo"),
            (&["lo"][..], "syntax.incomplete-sumti", "incomplete sumti"),
            (
                &["ga", "lo", "mlatu", "gi"][..],
                "syntax.incomplete-forethought-connection",
                "incomplete forethought connection",
            ),
        ] {
            let mut args = vec!["jbotci", "gentufa", "--detailed-errors"];
            args.extend_from_slice(source);
            let cli = Cli::try_parse_from(args).expect("gentufa detailed parses");
            let mut output = Vec::new();
            let mut error = Vec::new();
            let status = run_cli(cli, &mut output, &mut error, false).expect("gentufa run");

            assert_eq!(status, CliStatus::Failure);
            let stdout = std::str::from_utf8(&output).expect("stdout utf8");
            assert!(stdout.contains('‼'), "{stdout}");
            let stderr = String::from_utf8(error).expect("stderr utf8");
            assert!(stderr.contains(code), "{stderr}");
            assert!(stderr.contains(message), "{stderr}");
            assert!(!stderr.contains("syntax.parse"), "{stderr}");
            assert!(!stderr.contains("syntax parse failed"), "{stderr}");
        }
    });
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_detailed_syntax_errors_show_expectation_breakdown() {
    run_on_normal_stack(|| {
        let cli = Cli::try_parse_from(["jbotci", "gentufa", "--detailed-errors", "mi", "cu"])
            .expect("gentufa detailed parses");
        let mut output = Vec::new();
        let mut error = Vec::new();
        let status = run_cli(cli, &mut output, &mut error, false).expect("gentufa run");

        assert_eq!(status, CliStatus::Failure);
        let stdout = std::str::from_utf8(&output).expect("stdout utf8");
        assert!(stdout.contains('‼'), "{stdout}");
        let stderr = String::from_utf8(error).expect("stderr utf8");
        assert!(stderr.contains("needs one of:"));
        assert!(stderr.contains("replacement phrase"));
        assert!(stderr.contains("tag"));
        assert!(stderr.contains("NA, NAhE, SE"));
        assert!(stderr.contains("{nu'i}"));
        assert!(stderr.contains("{pe'o}"));
        assert!(stderr.contains("bridi"));
        let compact_stderr = stderr.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(compact_stderr.contains("[continues bridi]"));
        assert!(!stderr.contains("end of input (end of input)"));
        assert!(!stderr.contains("\x1b["));
    });
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_syntax_error_labels_unique_current_construct() {
    run_on_normal_stack(|| {
        let cli = Cli::try_parse_from(["jbotci", "gentufa", "--detailed-errors", "mi", "cu"])
            .expect("gentufa detailed parses");
        let mut output = Vec::new();
        let mut error = Vec::new();
        let status = run_cli(cli, &mut output, &mut error, false).expect("gentufa run");

        assert_eq!(status, CliStatus::Failure);
        let stdout = std::str::from_utf8(&output).expect("stdout utf8");
        assert!(stdout.contains('‼'), "{stdout}");
        let stderr = String::from_utf8(error).expect("stderr utf8");
        assert!(stderr.contains("mi cu"), "{stderr}");
        assert!(stderr.contains("syntax.incomplete-bridi"), "{stderr}");
        assert!(stderr.contains("needs one of:"), "{stderr}");
        assert!(stderr.contains("replacement phrase"), "{stderr}");
        assert!(stderr.contains("{nu'i}"), "{stderr}");
        assert!(stderr.contains("{pe'o}"), "{stderr}");
        assert!(stderr.contains("while parsing bridi"), "{stderr}");
        assert_eq!(stderr.matches("while parsing bridi").count(), 1, "{stderr}");
    });
}

#[test]
#[ignore = "generated syntax CLI output temporarily has no syntax trace stream"]
#[requires(true)]
#[ensures(true)]
fn gentufa_trace_writes_to_stderr_and_keeps_json_stdout_clean() {
    run_on_normal_stack(|| {
        let cli = Cli::try_parse_from([
            "jbotci", "gentufa", "--trace", "1", "--turtai", "json", "mi", "klama",
        ])
        .expect("gentufa trace parses");
        let mut output = Vec::new();
        let mut error = Vec::new();
        let status = run_cli(cli, &mut output, &mut error, false).expect("gentufa run");

        assert_eq!(status, CliStatus::Success);
        let stdout = String::from_utf8(output).expect("stdout utf8");
        let _: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
        assert!(!stdout.contains("trace["));
        let stderr = String::from_utf8(error).expect("stderr utf8");
        assert!(stderr.contains("trace[syntax]"), "{stderr}");
        assert!(!stderr.contains("\x1b["));
    });
}

#[test]
#[ignore = "generated syntax CLI output temporarily disables elided terminator rendering"]
#[requires(true)]
#[ensures(true)]
fn gentufa_show_elided_renders_tree_and_json_terminators() {
    run_on_normal_stack(|| {
        let tree_cli = Cli::try_parse_from([
            "jbotci",
            "gentufa",
            "--show-elided",
            "--turtai",
            "tree",
            "--show-spans",
            "mi",
            "klama",
        ])
        .expect("gentufa tree parses");
        let mut tree_output = Vec::new();
        let mut tree_error = Vec::new();
        let tree_status =
            run_cli(tree_cli, &mut tree_output, &mut tree_error, false).expect("gentufa tree");

        assert_eq!(tree_status, CliStatus::Success);
        assert!(tree_error.is_empty());
        let tree_stdout = String::from_utf8(tree_output).expect("tree stdout utf8");
        assert!(tree_stdout.contains("vau: Cmavo @[8‥8) \"vau\""));

        let json_cli = Cli::try_parse_from([
            "jbotci",
            "gentufa",
            "--show-elided",
            "--turtai",
            "json",
            "mi",
            "klama",
        ])
        .expect("gentufa json parses");
        let mut json_output = Vec::new();
        let mut json_error = Vec::new();
        let json_status =
            run_cli(json_cli, &mut json_output, &mut json_error, false).expect("gentufa json");

        assert_eq!(json_status, CliStatus::Success);
        assert!(json_error.is_empty());
        let json_stdout = String::from_utf8(json_output).expect("json stdout utf8");
        assert!(json_stdout.contains("\"phonemes\": \"vau\""));
        assert!(json_stdout.contains("\"span\": [8, 8]"));
        assert!(json_stdout.contains("\"elided\": true"));
    });
}

#[test]
#[ignore = "generated syntax CLI output temporarily has no syntax trace stream"]
#[requires(true)]
#[ensures(true)]
fn bare_trace_before_text_uses_default_trace_level() {
    run_on_normal_stack(|| {
        let cli = Cli::try_parse_from(["jbotci", "gentufa", "--trace", "gleki ku klama zei klama"])
            .expect("bare trace parses");
        let mut output = Vec::new();
        let mut error = Vec::new();
        let status = run_cli(cli, &mut output, &mut error, false).expect("gentufa run");

        assert_eq!(status, CliStatus::Failure);
        let stdout = std::str::from_utf8(&output).expect("stdout utf8");
        assert!(stdout.contains('‼'), "{stdout}");
        let stderr = String::from_utf8(error).expect("stderr utf8");
        assert!(stderr.contains("trace[syntax]"), "{stderr}");
        assert!(stderr.contains("syntax.unexpected-cmavo"), "{stderr}");
        assert!(!stderr.contains("syntax worker panicked"), "{stderr}");
    });
}

#[test]
#[ignore = "generated syntax CLI output temporarily has no syntax trace stream"]
#[requires(true)]
#[ensures(true)]
fn trace_color_policy_controls_ansi() {
    run_on_normal_stack(|| {
        let always_cli = Cli::try_parse_from([
            "jbotci",
            "gentufa",
            "--color=always",
            "--trace",
            "argument:3",
            "gleki",
            "ku",
            "klama",
            "zei",
            "klama",
        ])
        .expect("always color trace parses");
        let mut output = Vec::new();
        let mut error = Vec::new();
        let status =
            run_cli(always_cli, &mut output, &mut error, false).expect("always color trace run");
        assert_eq!(status, CliStatus::Failure);
        let stdout = std::str::from_utf8(&output).expect("stdout utf8");
        assert!(stdout.contains('‼'), "{stdout}");
        let stderr = String::from_utf8(error).expect("stderr utf8");
        assert!(stderr.contains("\x1b["), "{stderr}");

        let never_cli = Cli::try_parse_from([
            "jbotci",
            "gentufa",
            "--color=never",
            "--trace",
            "argument:3",
            "gleki",
            "ku",
            "klama",
            "zei",
            "klama",
        ])
        .expect("never color trace parses");
        let mut output = Vec::new();
        let mut error = Vec::new();
        let status =
            run_cli(never_cli, &mut output, &mut error, true).expect("never color trace run");
        assert_eq!(status, CliStatus::Failure);
        let stdout = std::str::from_utf8(&output).expect("stdout utf8");
        assert!(stdout.contains('‼'), "{stdout}");
        let stderr = String::from_utf8(error).expect("stderr utf8");
        assert!(stderr.contains("trace[syntax]"), "{stderr}");
        assert!(!stderr.contains("\x1b["), "{stderr}");
    });
}

#[test]
#[requires(true)]
#[ensures(true)]
fn detailed_syntax_error_color_controls_word_braces() {
    run_on_normal_stack(|| {
        let cli = Cli::try_parse_from([
            "jbotci",
            "gentufa",
            "--color=always",
            "--detailed-errors",
            "mi",
            "cu",
        ])
        .expect("gentufa color parses");
        let mut output = Vec::new();
        let mut error = Vec::new();
        let status = run_cli(cli, &mut output, &mut error, false).expect("gentufa run");

        assert_eq!(status, CliStatus::Failure);
        let stdout = std::str::from_utf8(&output).expect("stdout utf8");
        assert!(stdout.contains('‼'), "{stdout}");
        let stderr = String::from_utf8(error).expect("stderr utf8");
        assert!(stderr.contains("\x1b["));
        assert!(stderr.contains("lo'ai"));
        assert!(!stderr.contains("{lo'ai}"));
    });
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlasei_detailed_morphology_errors_show_detail_note() {
    let cli = Cli::try_parse_from(["jbotci", "vlasei", "--detailed-errors", "aa"])
        .expect("vlasei detailed parses");
    let mut output = Vec::new();
    let mut error = Vec::new();
    let status = run_cli(cli, &mut output, &mut error, false).expect("vlasei run");

    assert_eq!(status, CliStatus::Failure);
    let stdout = std::str::from_utf8(&output).expect("stdout utf8");
    assert!(stdout.contains('‼'), "{stdout}");
    let stderr = String::from_utf8(error).expect("stderr utf8");
    assert!(stderr.contains("morphology detail:"));
    assert!(stderr.contains("vowels in hiatus are not allowed"));
    assert!(stderr.contains("while parsing fu'ivla"));
    assert!(stderr.contains("reason"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlasei_trace_writes_morphology_stderr() {
    let cli = Cli::try_parse_from(["jbotci", "vlasei", "--trace", "1", "melxi,or."])
        .expect("vlasei trace parses");
    let mut output = Vec::new();
    let mut error = Vec::new();
    let status = run_cli(cli, &mut output, &mut error, false).expect("vlasei run");

    assert_eq!(status, CliStatus::Success);
    assert!(!output.is_empty());
    let stderr = String::from_utf8(error).expect("stderr utf8");
    assert!(stderr.contains("trace[morphology]"), "{stderr}");
    assert!(
        stderr.contains("morphology.warning.experimental-cgv"),
        "{stderr}"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn warning_context_includes_verbatim_quote_text() {
    run_on_normal_stack(|| {
        let cli = Cli::try_parse_from(["jbotci", "gentufa", "zo'oi", "gleki"])
            .expect("zo'oi warning parse");
        let mut output = Vec::new();
        let mut error = Vec::new();
        run_cli(cli, &mut output, &mut error, false).expect("gentufa warning run");

        let stderr = String::from_utf8(error).expect("stderr utf8");
        assert!(stderr.contains("ZOhOI single-word foreign quote"));
        assert!(stderr.contains("zo'oi gleki"));
        assert!(stderr.contains("syntax.warning.experimental-zoh-oi-quote"));
        assert!(!stderr.contains("<5 chars>"));
    });
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_raw_output_is_debug_syntax_parse() {
    run_on_normal_stack(|| {
        let cli = Cli::try_parse_from(["jbotci", "gentufa", "--turtai", "raw", "mi", "klama"])
            .expect("gentufa raw");
        let mut output = Vec::new();
        let mut error = Vec::new();
        run_cli(cli, &mut output, &mut error, false).expect("gentufa run");
        assert!(error.is_empty());
        let output = String::from_utf8(output).expect("utf8");
        assert!(output.contains("Regular"));
        assert!(output.contains("BridiStatementSyntax"));
        assert!(!output.contains("SyntaxValue"));
    });
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_raw_indent_zero_uses_compact_debug() {
    run_on_normal_stack(|| {
        let cli = Cli::try_parse_from([
            "jbotci", "gentufa", "--turtai", "raw", "--indent", "0", "mi", "klama",
        ])
        .expect("gentufa raw indent zero");
        let mut output = Vec::new();
        let mut error = Vec::new();
        run_cli(cli, &mut output, &mut error, false).expect("gentufa raw run");
        assert!(error.is_empty());
        let output = String::from_utf8(output).expect("utf8");
        assert!(!output.trim_end().contains('\n'));
        assert!(output.starts_with("Regular"));
        assert!(output.contains("BridiStatementSyntax"));
    });
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_raw_rejects_nonzero_indent() {
    let cli = Cli::try_parse_from([
        "jbotci", "gentufa", "--turtai", "raw", "--indent", "2", "mi", "klama",
    ])
    .expect("gentufa raw indent parses");
    let error = run_cli(cli, &mut Vec::new(), &mut Vec::new(), false)
        .expect_err("raw nonzero indent rejected");
    assert!(error.to_string().contains("only supports `0`"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_show_defs_prepends_dictionary_cards() {
    let output = run_success_stdout(&[
        "jbotci",
        "gentufa",
        "--show-defs",
        "--color=never",
        "mi",
        "klama",
    ]);
    assert!(output.starts_with("1. mi | by: officialdata | cmavo: KOhA3"));
    assert!(output.contains("\n2. klama | by: officialdata | gismu"));
    assert!(output.contains("  definitions:"));
    assert!(output.contains("\n\n(mi kláma)"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_show_defs_works_for_non_bracket_formats() {
    for format in ["raw", "tree", "json"] {
        let output = run_success_stdout(&[
            "jbotci",
            "gentufa",
            "--show-defs",
            "--format",
            format,
            "--color=never",
            "mi",
            "klama",
        ]);
        assert!(
            output.starts_with("1. mi | by: officialdata | cmavo: KOhA3"),
            "{format}"
        );
        assert!(
            output.contains("\n2. klama | by: officialdata | gismu"),
            "{format}"
        );
        assert!(output.contains("\n\n"), "{format}");
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_old_definition_flags_are_removed() {
    assert_eq!(
        Cli::try_parse_from(["jbotci", "gentufa", "--defs", "mi", "klama"])
            .expect_err("defs flag removed")
            .kind(),
        ErrorKind::UnknownArgument
    );
    assert_eq!(
        Cli::try_parse_from(["jbotci", "gentufa", "--skicu", "mi", "klama"])
            .expect_err("skicu flag removed")
            .kind(),
        ErrorKind::UnknownArgument
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_color_flag_forces_ansi_bracket_output() {
    run_on_normal_stack(|| {
        let cli = Cli::try_parse_from(["jbotci", "gentufa", "--color", "mi", "klama"])
            .expect("gentufa color");
        assert_eq!(cli.color, concolor_clap::ColorChoice::Always);
        let mut output = Vec::new();
        let mut error = Vec::new();
        run_cli(cli, &mut output, &mut error, false).expect("gentufa color run");
        assert!(error.is_empty());
        let output = String::from_utf8(output).expect("utf8");
        assert!(output.contains("\x1b["));
    });
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_color_flag_forces_ansi_tree_output() {
    run_on_normal_stack(|| {
        let cli = Cli::try_parse_from([
            "jbotci", "gentufa", "--color", "--format", "vipcihe", "mi", "klama",
        ])
        .expect("gentufa tree color");
        let mut output = Vec::new();
        let mut error = Vec::new();
        run_cli(cli, &mut output, &mut error, false).expect("gentufa tree color run");
        assert!(error.is_empty());
        let output = String::from_utf8(output).expect("utf8");
        assert!(output.contains("\x1b[94mBridiWithLeadingTerms\x1b[39m"));
        assert!(output.contains("\x1b[94mGismu\x1b[39m"));
        assert!(output.contains("\x1b[94mCmavo\x1b[39m"));
        assert!(output.contains("\x1b[33m\"mi\"\x1b[39m"));
    });
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_runs_reported_color_case_on_normal_cli_stack() {
    run_on_normal_stack(|| {
        let cli = Cli::try_parse_from([
            "jbotci", "gentufa", "--color", "gleki", "je", "klama", "zei", "klama",
        ])
        .expect("gentufa color");
        let mut output = Vec::new();
        let mut error = Vec::new();
        run_cli(cli, &mut output, &mut error, false).expect("gentufa color run");
        assert!(error.is_empty());
        let output = String::from_utf8(output).expect("utf8");
        assert!(output.contains("\x1b["));
        assert!(output.contains("gléki"));
    });
}

#[test]
#[requires(true)]
#[ensures(true)]
fn color_never_disables_ansi_output() {
    run_on_normal_stack(|| {
        let cli = Cli::try_parse_from(["jbotci", "gentufa", "--color=never", "mi", "klama"])
            .expect("gentufa color never");
        assert_eq!(cli.color, concolor_clap::ColorChoice::Never);

        let mut output = Vec::new();
        let mut error = Vec::new();
        run_cli(cli, &mut output, &mut error, true).expect("gentufa color never run");

        let output = String::from_utf8(output).expect("utf8");
        assert!(!output.contains("\x1b["));
        assert!(error.is_empty());
    });
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_exact_found_outputs_dictionary_card() {
    let run = run_cli_capture(&["jbotci", "vlacku", "--valsi", "klama"], false);

    assert_eq!(run.status, CliStatus::Success);
    assert!(run.stderr.is_empty(), "{}", run.stderr);
    assert!(
        run.stdout
            .contains("1. klama | by: officialdata | gismu | similarity: 100% | votes: ∞")
    );
    assert!(run.stdout.contains("  rafsi: "));
    assert!(run.stdout.contains("  glosses:"));
    assert!(run.stdout.contains("  definitions:"));

    for query in ["шой", "\u{ed86}\u{eda8}"] {
        let run = run_cli_capture(&["jbotci", "vlacku", "--valsi", query], false);

        assert_eq!(run.status, CliStatus::Success, "{query}");
        assert!(run.stderr.is_empty(), "{}", run.stderr);
        assert!(
            run.stdout
                .contains("1. coi | by: officialdata | cmavo: COI | similarity: 100%"),
            "{query}: {}",
            run.stdout
        );
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_uses_one_definition_place_map_for_definitions_and_notes() {
    let cases = [
        (
            "baldakyxa'i",
            "⟨1⟩ is a great sword for use against ⟨2⟩ by ⟨3⟩.",
        ),
        ("bircidni", "⟨1⟩ is an elbow of body ⟨2⟩."),
        ("barku'a", "⟨2⟩ = bartu₂. For \"balcony\", see {balni}."),
        ("brivla", "Derived from {bridi} and {valsi}, deleting b₃,"),
    ];

    for (word, expected) in cases {
        let run = run_cli_capture(&["jbotci", "vlacku", "--valsi", word], false);

        assert_eq!(run.status, CliStatus::Success, "{word}: {}", run.stderr);
        assert!(run.stderr.is_empty(), "{word}: {}", run.stderr);
        assert!(run.stdout.contains(expected), "{word}: {}", run.stdout);
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_glosses_reference_indexed_definition_places() {
    let run = run_cli_capture(&["jbotci", "vlacku", "--valsi", "seltictra"], false);

    assert_eq!(run.status, CliStatus::Success);
    assert!(run.stderr.is_empty(), "{}", run.stderr);
    assert!(
        run.stdout.lines().any(|line| {
            line == "    known falsehood (⟨3⟩ may be aware of the falsehood, however the intended target of deception is ⟨4⟩)"
        }),
        "{}",
        run.stdout
    );
    assert!(!run.stdout.contains("$x_3$"), "{}", run.stdout);
    assert!(!run.stdout.contains("$x_4$"), "{}", run.stdout);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_etymology_references_indexed_definition_places() {
    let run = run_cli_capture(
        &["jbotci", "vlacku", "--show-etymology", "--valsi", "bu'ivla"],
        false,
    );

    assert_eq!(run.status, CliStatus::Success);
    assert!(run.stderr.is_empty(), "{}", run.stderr);
    assert!(
        run.stdout.lines().any(|line| {
            line == "    bu + valsi. “valsi” was chosen because ⟨1⟩ quotes words and a “bu” also takes a single word."
        }),
        "{}",
        run.stdout
    );
    assert!(!run.stdout.contains("$x_1$"), "{}", run.stdout);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn cukta_section_fetch_outputs_default_section() {
    let run = run_cli_capture(
        &["jbotci", "cukta", "--section", "section-what-is-lojban"],
        false,
    );

    assert_eq!(run.status, CliStatus::Success);
    assert!(run.stderr.is_empty(), "{}", run.stderr);
    assert!(run.stdout.starts_with("# 1.1. What is Lojban?"));
    assert!(run.stdout.contains("Lojban (pronounced"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn cukta_section_fetch_uses_plain_links() {
    let run = run_cli_capture(&["jbotci", "cukta", "--section", "9.6"], false);

    assert_eq!(run.status, CliStatus::Success);
    assert!(run.stderr.is_empty(), "{}", run.stderr);
    assert!(!run.stdout.contains("]("), "{}", run.stdout);
    assert!(!run.stdout.contains("Parse"), "{}", run.stdout);
    assert!(
        run.stdout.contains("| mi | viska | do | sepi'o |"),
        "{}",
        run.stdout
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn cukta_book_alias_exact_word_search_uses_tagged_content() {
    let run = run_cli_capture(&["jbotci", "book", "--valsi", "lojban", "-n", "3"], false);

    assert_eq!(run.status, CliStatus::Success);
    assert!(run.stderr.is_empty(), "{}", run.stderr);
    assert_in_order(
        &run.stdout,
        &[
            "### 1. 4.3. brivla",
            "### 2. Paragraph in 4.3. brivla",
            "### 3.",
        ],
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn cukta_exact_word_search_accepts_non_latin_query() {
    for query in ["шой", "\u{ed86}\u{eda8}"] {
        let run = run_cli_capture(&["jbotci", "cukta", "--valsi", query, "-n", "3"], false);

        assert_eq!(run.status, CliStatus::Success, "{query}");
        assert!(run.stderr.is_empty(), "{}", run.stderr);
        assert!(
            run.stdout.contains("the cmavo coi means hello"),
            "{query}: {}",
            run.stdout
        );
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn cukta_toc_outputs_table_of_contents() {
    let run = run_cli_capture(&["jbotci", "cukta", "--toc"], false);

    assert_eq!(run.status, CliStatus::Success);
    assert!(run.stderr.is_empty(), "{}", run.stderr);
    assert!(run.stdout.starts_with("# Table of Contents"));
    assert!(run.stdout.contains("1.1. What is Lojban?"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn cukta_semantic_search_reports_missing_setup() {
    let run = run_cli_capture_with_embedding_dirs(
        &["jbotci", "cukta", "lojban"],
        false,
        &unique_embedding_test_path("cukta-model-missing"),
        &unique_embedding_test_path("cukta-index-missing"),
    );

    assert_eq!(run.status, CliStatus::InvalidInput);
    assert!(run.stdout.is_empty(), "{}", run.stdout);
    assert!(run.stderr.contains("jbotci setup --embedding"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_semantic_search_reports_missing_setup() {
    let run = run_cli_capture_with_embedding_dirs(
        &["jbotci", "vlacku", "language"],
        false,
        &unique_embedding_test_path("vlacku-model-missing"),
        &unique_embedding_test_path("vlacku-index-missing"),
    );

    assert_eq!(run.status, CliStatus::InvalidInput);
    assert!(run.stdout.is_empty(), "{}", run.stdout);
    assert!(run.stderr.contains("jbotci setup --embedding"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn setup_embedding_requires_a_setup_task() {
    let cli = Cli::try_parse_from(["jbotci", "setup"]).expect("setup parses");
    let mut output = Vec::new();
    let mut error = Vec::new();
    let result = run_cli(cli, &mut output, &mut error, false);

    assert!(result.is_err());
    assert!(output.is_empty());
    assert!(error.is_empty());
    assert!(
        result
            .expect_err("setup without task fails")
            .to_string()
            .contains("jbotci setup --embedding")
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn setup_embedding_rejects_unknown_model_without_download() {
    let cli = Cli::try_parse_from(["jbotci", "setup", "--embedding", "--model", "unknown-model"])
        .expect("setup parses");
    let mut output = Vec::new();
    let mut error = Vec::new();
    let result = run_cli(cli, &mut output, &mut error, false);

    assert!(result.is_err());
    assert!(output.is_empty());
    assert!(error.is_empty());
    assert!(
        result
            .expect_err("unknown model fails")
            .to_string()
            .contains("unsupported embedding model")
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn jvozba_outputs_best_lujvo_word() {
    let run = run_cli_capture(&["jbotci", "jvozba", "lojbo", "bangu"], false);

    assert_eq!(run.status, CliStatus::Success);
    assert!(run.stderr.is_empty(), "{}", run.stderr);
    assert_eq!(run.stdout, "jbobau\n");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn jvozba_accepts_fixed_rafsi_and_cmevla_mode() {
    let run = run_cli_capture(
        &["jbotci", "jvozba", "--cmevla", "lojbo", "--rafsi", "bau"],
        false,
    );

    assert_eq!(run.status, CliStatus::Success);
    assert!(run.stderr.is_empty(), "{}", run.stderr);
    assert_eq!(run.stdout, "jbobaus\n");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn jvozba_rejects_option_like_positional_rafsi() {
    let error = Cli::try_parse_from(["jbotci", "jvozba", "lojbo", "-bau-"])
        .expect_err("fixed rafsi marker is not positional syntax");

    assert_eq!(error.kind(), ErrorKind::UnknownArgument);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn jvozba_rejects_unsupported_flags_with_clap() {
    let error = Cli::try_parse_from(["jbotci", "jvozba", "--detailed-errors", "lojbo"])
        .expect_err("jvozba does not expose detailed errors");

    assert_eq!(error.kind(), ErrorKind::UnknownArgument);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn jvozba_help_only_lists_supported_options() {
    let error = Cli::try_parse_from(["jbotci", "jvozba", "--help"]).expect_err("help");
    assert_eq!(error.kind(), ErrorKind::DisplayHelp);
    let help = error.to_string();

    assert!(help.contains("--cmevla"));
    assert!(help.contains("--rafsi"));
    assert!(help.contains("--color"));
    assert!(!help.contains("--detailed-errors"));
    assert!(!help.contains("--trace-phase"));
    assert!(!help.contains("--trace-list"));
    assert!(!help.contains("--ascii"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn jvozba_colorizes_segments_when_requested() {
    let run = run_cli_capture(&["jbotci", "jvozba", "--color", "lojbo", "bangu"], false);

    assert_eq!(run.status, CliStatus::Success);
    assert!(run.stderr.is_empty(), "{}", run.stderr);
    assert!(run.stdout.contains("\x1b[32mjbo\x1b[39m"));
    assert!(run.stdout.contains("\x1b[35mbau\x1b[39m"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn jvozba_colorizes_cmevla_suffix_like_hyphen() {
    let run = run_cli_capture(
        &[
            "jbotci", "jvozba", "--color", "--cmevla", "birti", "--rafsi", "zba",
        ],
        false,
    );

    assert_eq!(run.status, CliStatus::Success);
    assert!(run.stderr.is_empty(), "{}", run.stderr);
    assert!(
        run.stdout.contains("\x1b[32mbit\x1b[39m"),
        "{:?}",
        run.stdout
    );
    assert!(run.stdout.contains("\x1b[90my\x1b[39m"), "{:?}", run.stdout);
    assert!(
        run.stdout.contains("\x1b[35mzba\x1b[39m"),
        "{:?}",
        run.stdout
    );
    assert!(run.stdout.contains("\x1b[90ms\x1b[39m"), "{:?}", run.stdout);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn jvozba_colorizes_cmevla_final_consonant_rafsi_as_rafsi() {
    let run = run_cli_capture(
        &["jbotci", "jvozba", "--color", "--cmevla", "cmene", "valsi"],
        false,
    );

    assert_eq!(run.status, CliStatus::Success);
    assert!(run.stderr.is_empty(), "{}", run.stderr);
    assert!(run.stdout.contains("\x1b[32mcme\x1b[39m"));
    assert!(run.stdout.contains("\x1b[35mval\x1b[39m"));
    assert!(!run.stdout.contains("\x1b[35mva\x1b[39m\x1b[90ml\x1b[39m"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn jvozba_errors_match_v0_text() {
    let cli = Cli::try_parse_from(["jbotci", "jvozba", "lojbo"]).expect("jvozba args");
    let error = run_cli(cli, &mut Vec::new(), &mut Vec::new(), false).expect_err("jvozba error");

    assert_eq!(
        error.to_string(),
        "jvozba requires at least two rafsi-producing inputs."
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gimfihi_outputs_table_for_canonical_command() {
    let run = run_cli_capture(&gimfihi_1995_sample_args("gimfihi", &[]), false);

    assert_eq!(run.status, CliStatus::Success);
    assert!(run.stderr.is_empty(), "{}", run.stderr);
    assert!(run.stdout.contains("sources: cmn:uan → uan"));
    assert!(run.stdout.contains("sources: eng:ekspekt → ekspekt"));
    assert!(run.stdout.contains("winner:"));
    assert!(run.stdout.contains("mark  gismu  score"));
    assert!(run.stdout.contains("*"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gimfihi_table_shows_bracketed_ipa_and_its_resolved_source() {
    let run = run_cli_capture(
        &[
            "jbotci",
            "gimfihi",
            "--source",
            "eng:100:[kæt]",
            "--check-collisions",
            "none",
            "--count",
            "1",
        ],
        false,
    );

    assert_eq!(run.status, CliStatus::Success);
    assert!(run.stderr.is_empty(), "{}", run.stderr);
    assert!(run.stdout.contains("sources: eng:[kæt] → kat"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gimfihi_json_uses_preset_weights_and_explicit_overrides() {
    let run = run_cli_capture(
        &gimfihi_1995_sample_args_with_eng("gimfihi", "eng:250:ekspekt", &["--format", "json"]),
        false,
    );

    assert_eq!(run.status, CliStatus::Success);
    assert!(run.stderr.is_empty(), "{}", run.stderr);
    let json: serde_json::Value = serde_json::from_str(&run.stdout).expect("json output");
    let sources = json["resolved-sources"].as_array().expect("sources");
    let cmn = sources
        .iter()
        .find(|source| source["language"] == "cmn")
        .expect("cmn source");
    assert_eq!(cmn["weight"], 347);
    let eng = sources
        .iter()
        .find(|source| source["language"] == "eng")
        .expect("eng source");
    assert_eq!(eng["weight"], 250);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gimfihi_check_collisions_changes_filtered_winner() {
    let without_collisions = run_cli_capture(
        &gimfihi_1995_sample_args("gimfihi", &["--check-collisions", "none"]),
        false,
    );
    let with_collisions = run_cli_capture(
        &gimfihi_1995_sample_args("gimfihi", &["--check-collisions", "all"]),
        false,
    );

    assert_eq!(without_collisions.status, CliStatus::Success);
    assert_eq!(with_collisions.status, CliStatus::Success);
    assert!(without_collisions.stdout.contains("winner: kanpe"));
    assert!(!with_collisions.stdout.contains("winner: kanpe"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gimfihi_json_includes_highlighted_candidate_outside_count() {
    let run = run_cli_capture(
        &gimfihi_1995_sample_args(
            "gimfihi",
            &[
                "--check-collisions",
                "none",
                "--count",
                "1",
                "--highlight",
                "nanpe",
                "--format",
                "json",
            ],
        ),
        false,
    );

    assert_eq!(run.status, CliStatus::Success);
    assert!(run.stderr.is_empty(), "{}", run.stderr);
    let json: serde_json::Value = serde_json::from_str(&run.stdout).expect("json output");
    assert_eq!(json["highlighted-word"], "nanpe");
    let candidates = json["candidates"].as_array().expect("candidates");
    assert!(candidates.len() >= 2);
    assert!(
        candidates
            .iter()
            .any(|candidate| candidate["word"] == "nanpe" && candidate["highlighted"] == true)
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gimfihi_rejects_incomplete_preset_language_set() {
    let cli = Cli::try_parse_from([
        "jbotci",
        "gimfihi",
        "--preset",
        "1995",
        "--source",
        "eng::ekspekt",
    ])
    .expect("gimfihi args");
    let error = run_cli(cli, &mut Vec::new(), &mut Vec::new(), false).expect_err("gimfihi error");
    assert!(
        error
            .to_string()
            .contains("preset source language `cmn` is missing")
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_exact_valid_missing_outputs_classification_card() {
    let run = run_cli_capture(&["jbotci", "vlacku", "--valsi", "brodax"], false);

    assert_eq!(run.status, CliStatus::ValidMissing);
    assert!(run.stderr.is_empty(), "{}", run.stderr);
    assert!(run.stdout.contains("1. brodax | cmevla"));
    assert!(!run.stdout.contains("  rafsi:"));
    assert!(!run.stdout.contains("  glosses:"));
    assert!(!run.stdout.contains("  definitions:"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_exact_invalid_word_reports_invalid_input_status() {
    let run = run_cli_capture(&["jbotci", "vlacku", "--valsi", "aa"], false);

    assert_eq!(run.status, CliStatus::InvalidInput);
    assert!(run.stdout.is_empty(), "{}", run.stdout);
    assert!(
        run.stderr
            .contains(&format!("{INVALID_LOJBAN_WORD_MESSAGE_PREFIX}aa"))
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_punctuation_only_query_reports_invalid_input_status() {
    let run = run_cli_capture(&["jbotci", "vlacku", "--valsi", "!!!"], false);

    assert_eq!(run.status, CliStatus::InvalidInput);
    assert!(run.stdout.is_empty(), "{}", run.stdout);
    assert!(
        run.stderr
            .contains(&format!("{INVALID_LOJBAN_WORD_MESSAGE_PREFIX}!!!"))
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_rafsi_lookup_returns_source_entry() {
    let run = run_cli_capture(&["jbotci", "vlacku", "--rafsi", "kla"], false);

    assert_eq!(run.status, CliStatus::Success);
    assert!(run.stderr.is_empty(), "{}", run.stderr);
    assert!(
        run.stdout
            .contains("1. klama | by: officialdata | gismu | similarity: 100% | votes: ∞")
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_lujvo_outputs_headword_decomposition_then_sources() {
    let run = run_cli_capture(
        &["jbotci", "vlacku", "--ascii", "--lujvo", "mivyselbai"],
        false,
    );

    assert_eq!(run.status, CliStatus::ValidMissing);
    assert!(run.stderr.is_empty(), "{}", run.stderr);
    assert!(run.stdout.contains("1. mivyselbai | lujvo"));
    assert!(run.stdout.contains("  decomposition: miv~y~sel~bai"));
    assert_in_order(
        &run.stdout,
        &[
            "1. mivyselbai | lujvo",
            "2. jmive | by: officialdata | gismu",
            "3. se | by: officialdata | cmavo: SE",
            "4. bapli | by: officialdata | gismu",
        ],
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_lujvo_outputs_unresolved_final_part_and_exact_word_card() {
    let run = run_cli_capture(
        &["jbotci", "vlacku", "--ascii", "--lujvo", "jetcybolxada"],
        false,
    );

    assert_eq!(run.status, CliStatus::ValidMissing);
    assert!(run.stderr.is_empty(), "{}", run.stderr);
    assert!(run.stdout.contains("1. jetcybolxada | lujvo"));
    assert!(run.stdout.contains("  decomposition: jetc~y~bolxada"));
    assert_in_order(
        &run.stdout,
        &[
            "1. jetcybolxada | lujvo",
            "2. jetce | by: officialdata | gismu",
            "3. bolxada | by: Ilmen | fu'ivla",
        ],
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_lujvo_outputs_unknown_final_full_word_card() {
    let run = run_cli_capture(
        &["jbotci", "vlacku", "--ascii", "--lujvo", "jetcyblorvuku"],
        false,
    );

    assert_eq!(run.status, CliStatus::ValidMissing);
    assert!(run.stderr.is_empty(), "{}", run.stderr);
    assert!(run.stdout.contains("1. jetcyblorvuku | lujvo"));
    assert!(run.stdout.contains("  decomposition: jetc~y~blorvuku"));
    assert_in_order(
        &run.stdout,
        &[
            "1. jetcyblorvuku | lujvo",
            "2. jetce | by: officialdata | gismu",
            "3. blorvuku | fu'ivla",
        ],
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_decompose_lujvo_adds_decomposition_to_exact_lujvo_cards() {
    let run = run_cli_capture(
        &[
            "jbotci",
            "vlacku",
            "--ascii",
            "--decompose-lujvo",
            "--valsi",
            "mivyselbai",
        ],
        false,
    );

    assert_eq!(run.status, CliStatus::ValidMissing);
    assert!(run.stderr.is_empty(), "{}", run.stderr);
    assert!(run.stdout.contains("1. mivyselbai | lujvo"));
    assert!(run.stdout.contains("  decomposition: miv~y~sel~bai"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_exact_word_glob_matches_through_valsi() {
    let found = run_cli_capture(
        &["jbotci", "vlacku", "--valsi", "klam@", "--count", "1"],
        false,
    );
    assert_eq!(found.status, CliStatus::Success);
    assert!(found.stdout.contains("1. klama | by: officialdata | gismu"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_exact_rafsi_glob_matches_through_rafsi() {
    let found = run_cli_capture(
        &["jbotci", "vlacku", "--rafsi", "kl@", "--count", "5"],
        false,
    );
    assert_eq!(found.status, CliStatus::Success);
    assert!(found.stdout.contains("klama | by: officialdata | gismu"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_filters_can_turn_hits_into_no_hit_status() {
    let run = run_cli_capture(
        &[
            "jbotci",
            "vlacku",
            "--valsi",
            "klama",
            "--word-type",
            "cmavo",
        ],
        false,
    );

    assert_eq!(run.status, CliStatus::ValidMissing);
    assert_eq!(run.stdout, "No matches found.\n");
    assert!(run.stderr.is_empty(), "{}", run.stderr);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_official_author_low_score_renders_official_marker() {
    let run = run_cli_capture(&["jbotci", "vlacku", "--valsi", "birka"], false);

    assert_eq!(run.status, CliStatus::Success);
    assert!(run.stderr.is_empty(), "{}", run.stderr);
    assert!(
        run.stdout
            .contains("1. birka | by: officialdata | gismu | similarity: 100% | votes: ∞"),
        "{}",
        run.stdout
    );
    assert!(!run.stdout.contains("votes: +10000"), "{}", run.stdout);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_ascii_renders_official_author_marker_as_ascii() {
    let run = run_cli_capture(&["jbotci", "vlacku", "--ascii", "--valsi", "birka"], false);

    assert_eq!(run.status, CliStatus::Success);
    assert!(run.stderr.is_empty(), "{}", run.stderr);
    assert!(run.stdout.contains("votes: official"), "{}", run.stdout);
    assert!(!run.stdout.contains('∞'), "{}", run.stdout);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_hides_etymology_by_default() {
    let run = run_cli_capture(&["jbotci", "vlacku", "--valsi", "abniena"], false);

    assert_eq!(run.status, CliStatus::Success);
    assert!(run.stderr.is_empty(), "{}", run.stderr);
    assert!(!run.stdout.contains("etymology:"), "{}", run.stdout);
    assert!(run.stdout.contains("Guaraní in aspect"), "{}", run.stdout);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_show_etymology_renders_etymology_section() {
    let run = run_cli_capture(
        &["jbotci", "vlacku", "--show-etymology", "--valsi", "abniena"],
        false,
    );

    assert_eq!(run.status, CliStatus::Success);
    assert!(run.stderr.is_empty(), "{}", run.stderr);
    assert!(run.stdout.contains("  etymology:"), "{}", run.stdout);
    assert!(run.stdout.contains("ava, people"), "{}", run.stdout);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_sound_search_accepts_bracketed_ipa_and_orders_by_similarity() {
    let run = run_cli_capture(
        &[
            "jbotci",
            "vlacku",
            "--sound",
            "[ˈkla.ma]",
            "--count",
            "3",
            "--min-similarity",
            "90",
        ],
        false,
    );

    assert_eq!(run.status, CliStatus::Success);
    assert!(run.stderr.is_empty(), "{}", run.stderr);
    assert!(
        run.stdout
            .contains("1. klama | by: officialdata | gismu | similarity: 100%")
    );
    assert!(
        run.stdout
            .contains("2. klani | by: officialdata | gismu | similarity: 92%")
    );
    assert!(
        run.stdout
            .contains("3. klina | by: officialdata | gismu | similarity: 92%")
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_colors_card_labels_dividers_and_rich_text() {
    let output = render_vlacku_output_with_options(
        &VlackuSearchOutput {
            cards: vec![new!(VlackuCard {
                word: "klama".to_owned(),
                word_type: "gismu".to_owned(),
                selmaho: None,
                author: None,
                is_official: false,
                similarity: Some(1.0),
                votes: Some(7),
                rafsi: vec!["kla".to_owned()],
                glosses: vec!["come".to_owned()],
                definition: "references {cadzu} at $x_1$; malformed {bad link}.".to_owned(),
                notes: "unmatched $ remains plain".to_owned(),
                etymology: None,
                decomposition: Vec::new(),
            })],
            outcome: VlackuOutcome::Found,
            diagnostics: Vec::new(),
        },
        new!(VlackuRenderOptions {
            color: true,
            glyphs: GlyphStyle::Unicode,
            output_terminal_width: None,
            sumti_places: CliSumtiPlaces::Index,
            show_etymology: false,
        }),
    );

    assert!(output.contains("\x1b[90m1.\x1b[39m"));
    assert!(output.contains("\x1b[4m\x1b[33mklama"), "{output}");
    assert!(output.contains("\x1b[90m | \x1b[39m"));
    assert!(output.contains("\x1b[90msimilarity: \x1b[39m\x1b[35m100%\x1b[39m"));
    assert!(output.contains("\x1b[90mvotes: \x1b[39m\x1b[32m+7\x1b[39m"));
    assert!(output.contains("\x1b[90mrafsi: \x1b[39m\x1b[31mkla\x1b[39m"));
    assert!(output.contains("\x1b[90m{\x1b[39m\x1b[33mcadzu\x1b[39m\x1b[90m}\x1b[39m"));
    assert!(!output.contains("\x1b[4mcadzu"), "{output}");
    assert!(
        output.contains("\x1b[90m⟨\x1b[39m\x1b[36m1\x1b[39m\x1b[90m⟩\x1b[39m"),
        "{output}"
    );
    assert!(output.contains("\x1b[37m{bad link}\x1b[39m"));
    assert!(output.contains("\x1b[37munmatched $ remains plain\x1b[39m"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_raw_sumti_places_keep_dollar_spans_and_color_equals() {
    let output = render_vlacku_output_with_options(
        &VlackuSearchOutput {
            cards: vec![new!(VlackuCard {
                word: "klama".to_owned(),
                word_type: "gismu".to_owned(),
                selmaho: None,
                author: None,
                is_official: false,
                similarity: Some(1.0),
                votes: Some(7),
                rafsi: Vec::new(),
                glosses: Vec::new(),
                definition: "$x_2=b_1$ moves to $x_3$.".to_owned(),
                notes: String::new(),
                etymology: None,
                decomposition: Vec::new(),
            })],
            outcome: VlackuOutcome::Found,
            diagnostics: Vec::new(),
        },
        new!(VlackuRenderOptions {
            color: true,
            glyphs: GlyphStyle::Unicode,
            output_terminal_width: None,
            sumti_places: CliSumtiPlaces::Raw,
            show_etymology: false,
        }),
    );

    assert!(output.contains(
        "\x1b[90m$\x1b[39m\x1b[36mx_2\x1b[39m\x1b[90m=\x1b[39m\x1b[36mb_1\x1b[39m\x1b[90m$\x1b[39m"
    ));
    assert!(output.contains("\x1b[90m$\x1b[39m\x1b[36mx_3\x1b[39m\x1b[90m$\x1b[39m"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_raw_sumti_places_keep_gloss_and_etymology_references() {
    let output = render_vlacku_output_with_options(
        &VlackuSearchOutput {
            cards: vec![new!(VlackuCard {
                word: "example".to_owned(),
                word_type: "lujvo".to_owned(),
                selmaho: None,
                author: None,
                is_official: false,
                similarity: Some(1.0),
                votes: None,
                rafsi: Vec::new(),
                glosses: vec!["gloss reference $x_3$".to_owned()],
                definition: "$x_1$ defines the place map.".to_owned(),
                notes: String::new(),
                etymology: Some("etymology reference $x_1$".to_owned()),
                decomposition: Vec::new(),
            })],
            outcome: VlackuOutcome::Found,
            diagnostics: Vec::new(),
        },
        new!(VlackuRenderOptions {
            color: false,
            glyphs: GlyphStyle::Unicode,
            output_terminal_width: None,
            sumti_places: CliSumtiPlaces::Raw,
            show_etymology: true,
        }),
    );

    assert!(
        output
            .lines()
            .any(|line| line == "    gloss reference $x_3$"),
        "{output}"
    );
    assert!(
        output
            .lines()
            .any(|line| line == "    etymology reference $x_1$"),
        "{output}"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_terminal_width_wraps_long_detail_lines_with_indent() {
    let output = render_vlacku_output_with_width(
        &VlackuSearchOutput {
            cards: vec![new!(VlackuCard {
                word: "cmevla".to_owned(),
                word_type: "lujvo".to_owned(),
                selmaho: None,
                author: None,
                is_official: false,
                similarity: Some(1.0),
                votes: Some(4),
                rafsi: Vec::new(),
                glosses: Vec::new(),
                definition:
                    "$x_1$ is a morphologically defined name word meaning $x_2$ in language $x_3$."
                        .to_owned(),
                notes: "In Lojban, such words are characterized by ending with a consonant."
                    .to_owned(),
                etymology: None,
                decomposition: Vec::new(),
            })],
            outcome: VlackuOutcome::Found,
            diagnostics: Vec::new(),
        },
        false,
        GlyphStyle::Unicode,
        Some(48),
    );

    assert!(
        output.contains(
            "    ⟨1⟩ is a morphologically defined name word\n    meaning ⟨2⟩ in language ⟨3⟩."
        ),
        "{output}"
    );
    assert!(
        output.contains(
            "    In Lojban, such words are characterized by\n    ending with a consonant."
        ),
        "{output}"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_official_author_renders_infinity() {
    let output = render_vlacku_output(
        &VlackuSearchOutput {
            cards: vec![new!(VlackuCard {
                word: "birka".to_owned(),
                word_type: "gismu".to_owned(),
                selmaho: None,
                author: Some(new!(VlackuAuthor {
                    username: "officialdata".to_owned(),
                    realname: Some("Official Data".to_owned()),
                })),
                is_official: true,
                similarity: Some(1.0),
                votes: Some(10000),
                rafsi: Vec::new(),
                glosses: Vec::new(),
                definition: String::new(),
                notes: String::new(),
                etymology: None,
                decomposition: Vec::new(),
            })],
            outcome: VlackuOutcome::Found,
            diagnostics: Vec::new(),
        },
        false,
        GlyphStyle::Unicode,
    );

    assert!(output.contains("votes: ∞"));
    assert!(!output.contains("votes: +10000"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_ascii_renders_index_places_and_official_votes_as_ascii() {
    let output = render_vlacku_output_with_options(
        &VlackuSearchOutput {
            cards: vec![new!(VlackuCard {
                word: "fuhivla".to_owned(),
                word_type: "fu'ivla".to_owned(),
                selmaho: None,
                author: Some(new!(VlackuAuthor {
                    username: "officialdata".to_owned(),
                    realname: Some("Official Data".to_owned()),
                })),
                is_official: true,
                similarity: Some(1.0),
                votes: Some(10000),
                rafsi: Vec::new(),
                glosses: Vec::new(),
                definition: "$x_1$ is a loanword meaning $x_2$.".to_owned(),
                notes: String::new(),
                etymology: None,
                decomposition: Vec::new(),
            })],
            outcome: VlackuOutcome::Found,
            diagnostics: Vec::new(),
        },
        new!(VlackuRenderOptions {
            color: false,
            glyphs: GlyphStyle::Ascii,
            output_terminal_width: None,
            sumti_places: CliSumtiPlaces::Index,
            show_etymology: false,
        }),
    );

    assert!(output.contains("votes: official"));
    assert!(output.contains("<1> is a loanword meaning <2>."));
    assert!(!output.contains('∞'));
    assert!(!output.contains('⟨'));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn joins_positional_text() {
    let input = TextInput {
        file: None,
        trace: None,
        dialect: None,
        indent: None,
        text: vec!["coi".into(), "rodo".into()],
    };
    assert_eq!(input.read_text().expect("text"), "coi rodo");
}

#[requires(!command.is_empty())]
#[ensures(ret.iter().any(|arg| *arg == "--preset"))]
fn gimfihi_1995_sample_args(
    command: &'static str,
    extra_args: &[&'static str],
) -> Vec<&'static str> {
    gimfihi_1995_sample_args_with_eng(command, "eng::ekspekt", extra_args)
}

#[requires(!command.is_empty())]
#[requires(!eng_source.is_empty())]
#[ensures(ret.iter().any(|arg| *arg == eng_source))]
fn gimfihi_1995_sample_args_with_eng(
    command: &'static str,
    eng_source: &'static str,
    extra_args: &[&'static str],
) -> Vec<&'static str> {
    let mut args = vec![
        "jbotci",
        command,
        "--preset",
        "1995",
        "--source",
        "cmn::uan",
        "--source",
        "hin::rakan",
        "--source",
        eng_source,
        "--source",
        "spa::esper",
        "--source",
        "rus::predpologa",
        "--source",
        "ara::mulud",
    ];
    args.extend_from_slice(extra_args);
    args
}

#[derive(Debug)]
#[invariant(true)]
struct CapturedCliRun {
    status: CliStatus,
    stdout: String,
    stderr: String,
}

static EMBEDDING_ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[requires(true)]
#[ensures(true)]
fn embedding_env_lock() -> &'static Mutex<()> {
    EMBEDDING_ENV_LOCK.get_or_init(|| Mutex::new(()))
}

#[requires(!suffix.is_empty())]
#[ensures(!ret.as_os_str().is_empty())]
fn unique_embedding_test_path(suffix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "jbotci-embedding-test-{}-{}",
        std::process::id(),
        suffix
    ))
}

#[requires(true)]
#[ensures(true)]
fn run_cli_capture_with_embedding_dirs(
    args: &[&str],
    color_enabled: bool,
    model_dir: &Path,
    index_dir: &Path,
) -> CapturedCliRun {
    let _guard = embedding_env_lock()
        .lock()
        .expect("embedding env lock is not poisoned");
    let old_model_dir = std::env::var_os(EMBEDDING_MODEL_DIR_ENV);
    let old_index_dir = std::env::var_os(EMBEDDING_INDEX_DIR_ENV);
    set_embedding_test_env(EMBEDDING_MODEL_DIR_ENV, Some(model_dir.as_os_str()));
    set_embedding_test_env(EMBEDDING_INDEX_DIR_ENV, Some(index_dir.as_os_str()));
    let run = run_cli_capture(args, color_enabled);
    set_embedding_test_env(EMBEDDING_MODEL_DIR_ENV, old_model_dir.as_deref());
    set_embedding_test_env(EMBEDDING_INDEX_DIR_ENV, old_index_dir.as_deref());
    run
}

#[requires(!name.is_empty())]
#[ensures(true)]
fn set_embedding_test_env(name: &str, value: Option<&std::ffi::OsStr>) {
    // The embedding env vars are process-global; tests that mutate them hold
    // EMBEDDING_ENV_LOCK so concurrent semantic-search tests cannot observe a
    // half-updated pair.
    unsafe {
        if let Some(value) = value {
            std::env::set_var(name, value);
        } else {
            std::env::remove_var(name);
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn run_cli_capture(args: &[&str], color_enabled: bool) -> CapturedCliRun {
    let cli = Cli::try_parse_from(args).expect("CLI args parse");
    let mut output = Vec::new();
    let mut error = Vec::new();
    let status = run_cli(cli, &mut output, &mut error, color_enabled).expect("CLI run succeeds");

    CapturedCliRun {
        status,
        stdout: String::from_utf8(output).expect("stdout utf8"),
        stderr: String::from_utf8(error).expect("stderr utf8"),
    }
}

#[requires(!needles.is_empty())]
#[ensures(true)]
fn assert_in_order(haystack: &str, needles: &[&str]) {
    let mut start_index = 0;
    for needle in needles {
        let Some(relative_index) = haystack[start_index..].find(needle) else {
            panic!("missing `{needle}` after byte {start_index} in:\n{haystack}");
        };
        start_index += relative_index + needle.len();
    }
}

#[requires(!expected_statuses.is_empty())]
#[requires(!expected_iterations.is_empty())]
#[ensures(true)]
fn assert_benchmark_report_contains(
    stderr: &str,
    expected_iterations: &str,
    expected_statuses: &str,
) {
    assert_in_order(
        stderr,
        &[
            "benchmark:\n",
            expected_iterations,
            expected_statuses,
            "wall: total=",
            "throughput=",
            "cpu: ",
            "memory: ",
            "page-faults: ",
            "context-switches: ",
            "block-io: ",
        ],
    );
}

#[requires(true)]
#[ensures(true)]
fn run_success_stdout(args: &[&str]) -> String {
    let cli = Cli::try_parse_from(args).expect("CLI args parse");
    let mut output = Vec::new();
    let mut error = Vec::new();
    let status = run_cli(cli, &mut output, &mut error, false).expect("CLI run succeeds");

    assert_eq!(status, CliStatus::Success);
    assert!(error.is_empty(), "{}", String::from_utf8_lossy(&error));
    String::from_utf8(output).expect("stdout utf8")
}

#[requires(true)]
#[ensures(true)]
fn run_success_bytes(args: &[&str]) -> Vec<u8> {
    let cli = Cli::try_parse_from(args).expect("CLI args parse");
    let mut output = Vec::new();
    let mut error = Vec::new();
    let status = run_cli(cli, &mut output, &mut error, false).expect("CLI run succeeds");

    assert_eq!(status, CliStatus::Success);
    assert!(error.is_empty(), "{}", String::from_utf8_lossy(&error));
    output
}

#[requires(!stem.is_empty())]
#[requires(!extension.is_empty())]
#[ensures(ret.extension().is_some())]
fn unique_cli_output_path(stem: &str, extension: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "jbotci-{stem}-{}.{}",
        std::process::id(),
        extension
    ))
}

#[requires(!expected.is_empty())]
#[ensures(true)]
fn assert_gentufa_error_contains(args: &[&str], expected: &str) {
    let cli = Cli::try_parse_from(args).expect("CLI args parse");
    let error =
        run_cli(cli, &mut Vec::new(), &mut Vec::new(), false).expect_err("CLI run rejects args");
    assert!(
        error.to_string().contains(expected),
        "expected `{expected}` in `{error}`"
    );
}

#[requires(true)]
#[ensures(true)]
fn run_on_normal_stack(test: impl FnOnce()) {
    test();
}

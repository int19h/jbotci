#![recursion_limit = "1024"]
use bityzba::{ensures, requires};
use jbotci_dialect::{DialectDefinition, parse_dialect_definition};
use jbotci_morphology::{
    MorphologyOptions, segment_words_with_modifiers_with_options_and_source_id_attempt,
};
use jbotci_output::{BracketRenderOptions, JsonRenderOptions, TreeRenderOptions};
use jbotci_source::{SourceId, SourceSpan};
use jbotci_syntax::{
    ParseOptions, parse_syntax_tree_recovered_with_source_and_options,
    parse_syntax_tree_with_source_and_options,
};
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};
#[requires(true)]
#[ensures(true)]
fn main() {
    let mut output = io::BufWriter::new(io::stdout().lock());
    for line in io::stdin().lock().lines() {
        let request: Value = serde_json::from_str(&line.unwrap()).unwrap();
        let text = request["text"].as_str().unwrap();
        let dialect = request["dialect"]
            .as_str()
            .map(|s| parse_dialect_definition(s).unwrap())
            .unwrap_or_else(DialectDefinition::default);
        let morph = MorphologyOptions::default().with_dialect_definition(&dialect);
        let attempt = segment_words_with_modifiers_with_options_and_source_id_attempt(
            text,
            &morph,
            Some(SourceId("<fixture>".into())),
        )
        .into_data();
        let mut result = json!({"path": request["path"], "morphology": format!("{:?}", attempt.result), "morphology_warnings": format!("{:?}", attempt.warnings)});
        if let Ok(words) = attempt.result {
            let mut options = ParseOptions::default().with_dialect_definition(&dialect);
            if let Some(max) = request["max_errors"]
                .as_u64()
                .and_then(|n| std::num::NonZeroUsize::new(n as usize))
            {
                options = options.with_max_recovery_errors(max.get());
            }
            match parse_syntax_tree_with_source_and_options(&words, text, &options) {
                Ok(parsed) => {
                    let tree = parsed.parse_tree.as_ref();
                    result["raw"] = json!(format!("{tree:?}"));
                    result["json"] = json!(
                        jbotci_output::compact_generated_model_json_string_with_options(
                            tree,
                            JsonRenderOptions::default()
                        )
                        .unwrap()
                    );
                    result["tree"] = json!(
                        jbotci_output::pretty_generated_model_tree_with_options(
                            tree,
                            text,
                            TreeRenderOptions::default()
                        )
                        .unwrap()
                    );
                    result["brackets"] = json!(
                        jbotci_output::pretty_generated_model_brackets_with_options(
                            tree,
                            text,
                            BracketRenderOptions::default()
                        )
                        .unwrap()
                    );
                    result["refs"] = json!(
                        jbotci_semantics::references::analyze_generated_references(tree)
                            .unwrap()
                            .fixture_projection_json()
                            .unwrap()
                    );
                    result["blocks"] = json!(format!(
                        "{:?}",
                        jbotci_web_core::generated_model_gentufa_blocks_projection(
                            tree,
                            text,
                            &words,
                            &jbotci_web_core::GentufaBlocksProjectionOptions {
                                blocks: jbotci_web_core::GentufaBlockOptions::default(),
                                show_compounds: true
                            }
                        )
                        .unwrap()
                    ));
                    result["warnings"] = json!(format!("{:?}", parsed.warnings));
                }
                Err(error) => {
                    result["error"] = json!(format!("{error:?}"));
                }
            }
            let recovered =
                parse_syntax_tree_recovered_with_source_and_options(&words, text, &options);
            if !recovered.errors.is_empty() {
                result["recovered_tree"] = json!(format!(
                    "{:?}",
                    jbotci_output::pretty_recovered_syntax_tree_with_options(
                        &recovered,
                        text,
                        TreeRenderOptions::default()
                    )
                ));
                result["recovered_json"] = json!(
                    jbotci_output::compact_recovered_syntax_json_string_with_options(
                        &recovered,
                        text,
                        JsonRenderOptions::default()
                    )
                    .unwrap()
                );
                result["recovered_brackets"] = json!(format!(
                    "{:?}",
                    jbotci_output::pretty_recovered_syntax_brackets_with_options(
                        &recovered,
                        text,
                        BracketRenderOptions::default()
                    )
                ));
                result["recovered_blocks"] = json!(format!(
                    "{:?}",
                    jbotci_web_core::recovered_gentufa_blocks_projection(
                        &recovered,
                        text,
                        &words,
                        &jbotci_web_core::GentufaBlocksProjectionOptions {
                            blocks: jbotci_web_core::GentufaBlockOptions::default(),
                            show_compounds: true
                        }
                    )
                ));
            }
            result["recovered_raw"] = json!(format!("{:?}", recovered.parse_tree.as_ref()));
            result["recovered_errors"] = json!(format!("{:?}", recovered.errors));
            result["recovered_warnings"] = json!(format!("{:?}", recovered.warnings));
        }
        let snapshot = jbotci_ide::DocumentSnapshot::new(text.into(), 1);
        let span = SourceSpan::new(None, 0, text.len(), 0, text.chars().count()).unwrap();
        result["ide"] = json!({
            "diagnostics": snapshot.diagnostics,
            "tokens": format!("{:?}", snapshot.semantic_tokens().collect::<Vec<_>>()),
            "folds": format!("{:?}", snapshot.folding_ranges()),
            "selection": format!("{:?}", snapshot.selection_ranges(&(0..=text.chars().count()).collect::<Vec<_>>())),
            "structure_inlays": snapshot.structure_inlays(&jbotci_ide::DecorationProfile::default(), &span),
            "inlays": format!("{:?}", snapshot.inlays(&jbotci_ide::InlayOptions::default(), &span)),
        });
        serde_json::to_writer(&mut output, &result).unwrap();
        writeln!(output).unwrap();
        output.flush().unwrap();
    }
}

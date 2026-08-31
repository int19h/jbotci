use bityzba::requires;
use serde::Serialize;

use super::{BracketExpectations, CommandOutputExpectation, Expectations, Provenance, TestCase};

#[requires(test_case.is_valid_fixture_metadata())]
#[bityzba::ensures(ret.is_err() || ret.as_ref().is_ok_and(|text| !text.is_empty()))]
pub(super) fn format_test_case_toml(test_case: &TestCase) -> Result<String, toml::ser::Error> {
    let mut output = String::new();
    push_field(&mut output, "id", &test_case.id)?;
    if let Some(filename) = &test_case.lojban_filename {
        push_field(
            &mut output,
            "lojban-filename",
            &filename.to_string_lossy().replace('\\', "/"),
        )?;
    } else {
        push_field(&mut output, "lojban", &test_case.lojban)?;
    }
    push_optional_field(&mut output, "dialect", &test_case.dialect)?;
    push_optional_field(&mut output, "translation-en", &test_case.translation_en)?;
    push_optional_field(&mut output, "gloss-en", &test_case.gloss_en)?;
    if !test_case.tags.is_empty() {
        push_field(&mut output, "tags", &test_case.tags)?;
    }
    if test_case
        .tags
        .iter()
        .any(|tag| tag == "regression-baseline")
    {
        output.push_str(
            "# Initial expectations are parser-output regression baselines, not semantically verified truth.\n",
        );
    }
    for provenance in &test_case.provenance {
        push_provenance_toml(&mut output, provenance)?;
    }
    push_expectations_toml(&mut output, &test_case.expectations)?;
    Ok(output)
}

#[requires(true)]
#[bityzba::ensures(true)]
fn push_provenance_toml(
    output: &mut String,
    provenance: &Provenance,
) -> Result<(), toml::ser::Error> {
    output.push_str("\n[[provenance]]\n");
    push_field(output, "kind", provenance.kind_name())?;
    match provenance {
        Provenance::Cll {
            chapter,
            appendix,
            section_number,
            section_id,
            example_number,
            example_id,
            source_path,
        } => {
            push_optional_field(output, "chapter", chapter)?;
            push_optional_field(output, "appendix", appendix)?;
            push_optional_field(output, "section-number", section_number)?;
            push_field(output, "section-id", section_id)?;
            push_optional_field(output, "example-number", example_number)?;
            push_optional_field(output, "example-id", example_id)?;
            push_optional_field(output, "source-path", source_path)?;
        }
        Provenance::Muplis {
            collection_id,
            item_id,
            form,
            url,
        } => {
            push_field(output, "collection-id", collection_id)?;
            push_optional_field(output, "item-id", item_id)?;
            push_optional_field(output, "form", form)?;
            push_optional_field(output, "url", url)?;
        }
        Provenance::Corpus {
            corpus,
            entry_id,
            md5,
        } => {
            push_field(output, "corpus", corpus)?;
            push_optional_field(output, "entry-id", entry_id)?;
            push_optional_field(output, "md5", md5)?;
        }
        Provenance::Adhoc { description } => {
            push_optional_field(output, "description", description)?;
        }
        Provenance::Other {
            name,
            url,
            description,
        } => {
            push_field(output, "name", name)?;
            push_optional_field(output, "url", url)?;
            push_optional_field(output, "description", description)?;
        }
    }
    Ok(())
}

#[requires(true)]
#[bityzba::ensures(true)]
fn push_expectations_toml(
    output: &mut String,
    expectations: &Expectations,
) -> Result<(), toml::ser::Error> {
    if let Some(morphology) = &expectations.morphology {
        output.push_str("\n[expectations.morphology]\n");
        push_field(output, "status", &morphology.status)?;
        push_optional_field(output, "raw", &morphology.raw)?;
        if let Some(diagnostics) = &morphology.diagnostics {
            push_field(output, "diagnostics", diagnostics)?;
        }
        if let Some(recovered) = &morphology.recovered {
            output.push_str("\n[expectations.morphology.recovered]\n");
            push_field(output, "status", &recovered.status)?;
            push_optional_field(output, "max-errors", &recovered.max_errors)?;
            if !recovered.diagnostics.is_empty() {
                push_field(output, "diagnostics", &recovered.diagnostics)?;
            }
            push_optional_field(output, "tree", &recovered.tree)?;
        }
    }
    if let Some(jvozba) = &expectations.jvozba {
        output.push_str("\n[expectations.jvozba]\n");
        push_field(output, "status", &jvozba.status)?;
        push_field(output, "mode", &jvozba.mode)?;
        push_field(output, "inputs", &jvozba.inputs)?;
        push_optional_field(output, "output", &jvozba.output)?;
        push_optional_field(output, "error", &jvozba.error)?;
    }
    if let Some(syntax) = &expectations.syntax {
        output.push_str("\n[expectations.syntax]\n");
        push_field(output, "status", &syntax.status)?;
        push_optional_field(output, "raw", &syntax.raw)?;
        if let Some(diagnostics) = &syntax.diagnostics {
            push_field(output, "diagnostics", diagnostics)?;
        }
        push_optional_field(output, "xfail", &syntax.xfail)?;
        if let Some(recovered) = &syntax.recovered {
            output.push_str("\n[expectations.syntax.recovered]\n");
            push_field(output, "status", &recovered.status)?;
            push_optional_field(output, "max-errors", &recovered.max_errors)?;
            if !recovered.diagnostics.is_empty() {
                push_field(output, "diagnostics", &recovered.diagnostics)?;
            }
            push_optional_field(output, "tree", &recovered.tree)?;
        }
    }
    if let Some(semantics) = &expectations.semantics
        && let Some(refs) = &semantics.refs
    {
        output.push_str("\n[expectations.semantics.refs]\n");
        push_field(output, "status", &refs.status)?;
        push_optional_field(output, "raw", &refs.raw)?;
        push_optional_field(output, "error", &refs.error)?;
    }
    if let Some(output_expectation) = &expectations.output {
        if let Some(vlasei) = &output_expectation.vlasei {
            let has_inline_brackets =
                matches!(vlasei.brackets, Some(BracketExpectations::Legacy(_)));
            if has_inline_brackets || vlasei.tree.is_some() || vlasei.json.is_some() {
                output.push_str("\n[expectations.output.vlasei]\n");
                if let Some(BracketExpectations::Legacy(brackets)) = &vlasei.brackets {
                    push_field(output, "brackets", brackets)?;
                }
                if let Some(tree) = &vlasei.tree {
                    push_field(output, "tree", tree)?;
                }
                if let Some(json) = &vlasei.json {
                    push_field(output, "json", json)?;
                }
            }
            if let Some(BracketExpectations::Scripts(brackets)) = &vlasei.brackets
                && (brackets.latin.is_some()
                    || brackets.cyrillic.is_some()
                    || brackets.zbalermorna.is_some())
            {
                output.push_str("\n[expectations.output.vlasei.brackets]\n");
                push_optional_field(output, "latin", &brackets.latin)?;
                push_optional_field(output, "cyrillic", &brackets.cyrillic)?;
                push_optional_field(output, "zbalermorna", &brackets.zbalermorna)?;
            }
        }
        if let Some(gentufa) = &output_expectation.gentufa {
            if gentufa.brackets.is_some() || gentufa.tree.is_some() || gentufa.json.is_some() {
                output.push_str("\n[expectations.output.gentufa]\n");
                push_optional_field(output, "brackets", &gentufa.brackets)?;
                push_optional_field(output, "tree", &gentufa.tree)?;
                push_optional_field(output, "json", &gentufa.json)?;
            }
            if let Some(show_elided) = &gentufa.show_elided
                && (show_elided.brackets.is_some()
                    || show_elided.tree.is_some()
                    || show_elided.json.is_some())
            {
                output.push_str("\n[expectations.output.gentufa.show-elided]\n");
                push_command_output_fields(output, show_elided)?;
            }
        }
    }
    Ok(())
}

#[requires(true)]
#[bityzba::ensures(true)]
fn push_command_output_fields(
    output: &mut String,
    expectation: &CommandOutputExpectation,
) -> Result<(), toml::ser::Error> {
    if let Some(brackets) = &expectation.brackets {
        push_field(output, "brackets", brackets)?;
    }
    if let Some(tree) = &expectation.tree {
        push_field(output, "tree", tree)?;
    }
    if let Some(json) = &expectation.json {
        push_field(output, "json", json)?;
    }
    Ok(())
}

#[requires(true)]
#[bityzba::ensures(true)]
fn push_field<T: Serialize + ?Sized>(
    output: &mut String,
    key: &str,
    value: &T,
) -> Result<(), toml::ser::Error> {
    output.push_str(key);
    output.push_str(" = ");
    output.push_str(&format_toml_value(value)?);
    output.push('\n');
    Ok(())
}

#[requires(true)]
#[bityzba::ensures(true)]
fn push_optional_field<T: Serialize>(
    output: &mut String,
    key: &str,
    value: &Option<T>,
) -> Result<(), toml::ser::Error> {
    if let Some(value) = value {
        push_field(output, key, value)?;
    }
    Ok(())
}

#[requires(true)]
#[bityzba::ensures(true)]
fn format_toml_value<T: Serialize + ?Sized>(value: &T) -> Result<String, toml::ser::Error> {
    let mut output = String::new();
    value.serialize(toml::ser::ValueSerializer::new(&mut output))?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::format_test_case_toml;
    use crate::fixtures::{TestCase, load_fixture_path};
    use bityzba::requires;

    #[test]
    #[requires(true)]
    #[bityzba::ensures(true)]
    fn recovered_syntax_max_errors_survives_fixture_formatting() {
        let fixture =
            load_repo_fixture("tests/fixtures/adhoc/recovery/syntax/error-cap-truncation.toml");
        let formatted = format_test_case_toml(&fixture).unwrap();
        let reparsed: TestCase = toml::from_str(&formatted).unwrap();

        assert_eq!(
            reparsed
                .expectations
                .syntax
                .as_ref()
                .and_then(|expectation| expectation.recovered.as_ref())
                .and_then(|recovered| recovered.max_errors),
            Some(3)
        );
    }

    #[test]
    #[requires(true)]
    #[bityzba::ensures(true)]
    fn recovered_syntax_tree_survives_fixture_formatting() {
        let fixture = load_repo_fixture(
            "tests/fixtures/adhoc/recovery/syntax/mid-input-token-conservation.toml",
        );
        let expected_tree = fixture
            .expectations
            .syntax
            .as_ref()
            .and_then(|expectation| expectation.recovered.as_ref())
            .and_then(|recovered| recovered.tree.clone());
        let formatted = format_test_case_toml(&fixture).unwrap();
        let reparsed: TestCase = toml::from_str(&formatted).unwrap();

        assert_eq!(
            reparsed
                .expectations
                .syntax
                .as_ref()
                .and_then(|expectation| expectation.recovered.as_ref())
                .and_then(|recovered| recovered.tree.clone()),
            expected_tree
        );
    }

    #[test]
    #[requires(true)]
    #[bityzba::ensures(true)]
    fn explicit_empty_success_diagnostics_survive_fixture_formatting() {
        let fixture: TestCase = toml::from_str(
            r#"
id = "format.explicit-empty-diagnostics"
lojban = "coi"

[expectations.morphology]
status = "success"
diagnostics = []

[expectations.syntax]
status = "success"
diagnostics = []
"#,
        )
        .unwrap();

        assert_eq!(
            fixture
                .expectations
                .morphology
                .as_ref()
                .and_then(|expectation| expectation.diagnostics.as_ref()),
            Some(&Vec::new())
        );
        assert_eq!(
            fixture
                .expectations
                .syntax
                .as_ref()
                .and_then(|expectation| expectation.diagnostics.as_ref()),
            Some(&Vec::new())
        );
        let formatted = format_test_case_toml(&fixture).unwrap();
        assert!(formatted.contains("diagnostics = []"));
    }

    #[test]
    #[requires(true)]
    #[bityzba::ensures(true)]
    fn omitted_success_diagnostics_remain_unspecified_after_formatting() {
        let fixture: TestCase = toml::from_str(
            r#"
id = "format.omitted-diagnostics"
lojban = "coi"

[expectations.morphology]
status = "success"

[expectations.syntax]
status = "success"
"#,
        )
        .unwrap();

        assert!(
            fixture
                .expectations
                .morphology
                .as_ref()
                .is_some_and(|expectation| expectation.diagnostics.is_none())
        );
        assert!(
            fixture
                .expectations
                .syntax
                .as_ref()
                .is_some_and(|expectation| expectation.diagnostics.is_none())
        );
        let formatted = format_test_case_toml(&fixture).unwrap();
        assert!(!formatted.contains("diagnostics ="));
    }

    #[requires(!Path::new(relative).is_absolute())]
    #[bityzba::ensures(ret.is_valid_fixture_metadata())]
    fn load_repo_fixture(relative: &str) -> TestCase {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask-common must be in the workspace root")
            .join(relative);
        load_fixture_path(&path).unwrap().test_case
    }
}

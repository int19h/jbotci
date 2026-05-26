use bityzba::requires;
use serde::Serialize;

use super::{Expectations, Provenance, TestCase};

#[requires(test_case.is_valid_fixture_metadata())]
#[bityzba::ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(|text| !text.is_empty()))]
pub(super) fn format_test_case_toml(test_case: &TestCase) -> Result<String, toml::ser::Error> {
    let mut output = String::new();
    push_field(&mut output, "id", &test_case.id)?;
    push_field(&mut output, "lojban", &test_case.lojban)?;
    push_optional_field(&mut output, "dialect", &test_case.dialect)?;
    push_optional_field(&mut output, "translation-en", &test_case.translation_en)?;
    push_optional_field(&mut output, "gloss-en", &test_case.gloss_en)?;
    if !test_case.tags.is_empty() {
        push_field(&mut output, "tags", &test_case.tags)?;
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
            section_number,
            section_id,
            example_number,
            example_id,
            source_path,
        } => {
            push_field(output, "chapter", chapter)?;
            push_field(output, "section-number", section_number)?;
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
        if !morphology.diagnostics.is_empty() {
            push_field(output, "diagnostics", &morphology.diagnostics)?;
        }
    }
    if let Some(syntax) = &expectations.syntax {
        output.push_str("\n[expectations.syntax]\n");
        push_field(output, "status", &syntax.status)?;
        push_optional_field(output, "raw", &syntax.raw)?;
        if !syntax.diagnostics.is_empty() {
            push_field(output, "diagnostics", &syntax.diagnostics)?;
        }
        push_optional_field(output, "xfail", &syntax.xfail)?;
    }
    if let Some(semantics) = &expectations.semantics
        && let Some(refs) = &semantics.refs
    {
        output.push_str("\n[expectations.semantics.refs]\n");
        push_field(output, "status", &refs.status)?;
        push_optional_field(output, "raw", &refs.raw)?;
    }
    if let Some(output_expectation) = &expectations.output {
        if let Some(vlasei) = &output_expectation.vlasei
            && (vlasei.brackets.is_some() || vlasei.tree.is_some() || vlasei.json.is_some())
        {
            output.push_str("\n[expectations.output.vlasei]\n");
            if let Some(brackets) = &vlasei.brackets {
                push_field(output, "brackets", brackets)?;
            }
            if let Some(tree) = &vlasei.tree {
                push_field(output, "tree", tree)?;
            }
            if let Some(json) = &vlasei.json {
                push_field(output, "json", json)?;
            }
        }
        if let Some(gentufa) = &output_expectation.gentufa
            && (gentufa.brackets.is_some() || gentufa.tree.is_some() || gentufa.json.is_some())
        {
            output.push_str("\n[expectations.output.gentufa]\n");
            if let Some(brackets) = &gentufa.brackets {
                push_field(output, "brackets", brackets)?;
            }
            if let Some(tree) = &gentufa.tree {
                push_field(output, "tree", tree)?;
            }
            if let Some(json) = &gentufa.json {
                push_field(output, "json", json)?;
            }
        }
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

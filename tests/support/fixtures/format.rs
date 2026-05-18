use bityzba::{data, requires};
use jbotci_syntax::{SyntaxValue, SyntaxValueData};
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
    if let Some(output_expectation) = &expectations.output
        && output_expectation.brackets.is_some()
    {
        output.push_str("\n[expectations.output]\n");
        if let Some(brackets) = &output_expectation.brackets {
            push_field(output, "brackets", brackets)?;
        }
    }
    if let Some(morphology) = &expectations.morphology {
        output.push_str("\n[expectations.morphology]\n");
        push_field(output, "status", &morphology.status)?;
        if !morphology.words.is_empty() {
            output.push_str("words = [\n");
            for word in &morphology.words {
                output.push_str("    ");
                output.push_str(&format_toml_value(word)?);
                output.push_str(",\n");
            }
            output.push_str("]\n");
        }
        push_optional_field(output, "error", &morphology.error)?;
    }
    if let Some(syntax) = &expectations.syntax {
        output.push_str("\n[expectations.syntax]\n");
        push_field(output, "status", &syntax.status)?;
        if let Some(parse_tree) = &syntax.parse_tree {
            output.push_str("parse-tree = ");
            output.push_str(&format_syntax_value_toml(parse_tree, 0)?);
            output.push('\n');
        }
        push_optional_field(output, "error", &syntax.error)?;
        push_optional_field(output, "xfail", &syntax.xfail)?;
    }
    if let Some(syntax_refs) = &expectations.syntax_refs {
        output.push_str("\n[expectations.syntax-refs]\n");
        push_field(output, "status", &syntax_refs.status)?;
        push_optional_field(output, "value", &syntax_refs.value)?;
    }
    if let Some(warnings) = &expectations.warnings {
        output.push_str("\n[expectations.warnings]\n");
        push_field(output, "status", &warnings.status)?;
        push_optional_field(output, "value", &warnings.value)?;
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
fn format_syntax_value_toml(
    value: &SyntaxValue,
    indent: usize,
) -> Result<String, toml::ser::Error> {
    match value.as_data() {
        data!(SyntaxValue::Null) => Ok(r#"{ kind = "null" }"#.to_owned()),
        data!(SyntaxValue::Bool { value }) => {
            Ok(format!(r#"{{ kind = "bool", value = {value} }}"#))
        }
        data!(SyntaxValue::Integer { value }) => {
            Ok(format!(r#"{{ kind = "integer", value = {value} }}"#))
        }
        data!(SyntaxValue::Text { value }) => Ok(format!(
            r#"{{ kind = "text", value = {} }}"#,
            format_toml_value(value)?
        )),
        data!(SyntaxValue::Word { word }) => Ok(format!(
            r#"{{ kind = "word", word = {} }}"#,
            format_toml_value(word.as_ref())?
        )),
        data!(SyntaxValue::Json { value }) => Ok(format!(
            r#"{{ kind = "json", value = {} }}"#,
            format_toml_value(value)?
        )),
        data!(SyntaxValue::List { items }) => format_syntax_list_toml(items, indent),
        data!(SyntaxValue::Node { node }) => {
            let child = indent + 4;
            let field_indent = indent + 8;
            let mut output = String::new();
            output.push_str("{\n");
            output.push_str(&spaces(child));
            output.push_str(r#"kind = "node","#);
            output.push('\n');
            output.push_str(&spaces(child));
            output.push_str("node = {\n");
            output.push_str(&spaces(field_indent));
            output.push_str("constructor = ");
            output.push_str(&format_toml_value(&node.constructor)?);
            output.push_str(",\n");
            output.push_str(&spaces(field_indent));
            output.push_str("fields = ");
            output.push_str(&format_syntax_fields_toml(&node.fields, field_indent)?);
            output.push('\n');
            output.push_str(&spaces(child));
            output.push_str("}\n");
            output.push_str(&spaces(indent));
            output.push('}');
            Ok(output)
        }
    }
}

#[requires(true)]
#[bityzba::ensures(true)]
fn format_syntax_list_toml(
    items: &[SyntaxValue],
    indent: usize,
) -> Result<String, toml::ser::Error> {
    if items.is_empty() {
        return Ok(r#"{ kind = "list", items = [] }"#.to_owned());
    }
    let child = indent + 4;
    let item_indent = indent + 8;
    let mut output = String::new();
    output.push_str("{\n");
    output.push_str(&spaces(child));
    output.push_str(r#"kind = "list","#);
    output.push('\n');
    output.push_str(&spaces(child));
    output.push_str("items = [\n");
    for (index, item) in items.iter().enumerate() {
        output.push_str(&spaces(item_indent));
        output.push_str(&format_syntax_value_toml(item, item_indent)?);
        if index + 1 != items.len() {
            output.push(',');
        }
        output.push('\n');
    }
    output.push_str(&spaces(child));
    output.push_str("]\n");
    output.push_str(&spaces(indent));
    output.push('}');
    Ok(output)
}

#[requires(true)]
#[bityzba::ensures(true)]
fn format_syntax_fields_toml(
    fields: &[jbotci_syntax::SyntaxField],
    indent: usize,
) -> Result<String, toml::ser::Error> {
    if fields.is_empty() {
        return Ok("[]".to_owned());
    }
    let item_indent = indent + 4;
    let mut output = String::new();
    output.push_str("[\n");
    for (index, field) in fields.iter().enumerate() {
        output.push_str(&spaces(item_indent));
        output.push_str("{\n");
        if let Some(name) = &field.name {
            output.push_str(&spaces(item_indent + 4));
            output.push_str("name = ");
            output.push_str(&format_toml_value(name)?);
            output.push_str(",\n");
        }
        output.push_str(&spaces(item_indent + 4));
        output.push_str("value = ");
        output.push_str(&format_syntax_value_toml(&field.value, item_indent + 4)?);
        output.push('\n');
        output.push_str(&spaces(item_indent));
        output.push('}');
        if index + 1 != fields.len() {
            output.push(',');
        }
        output.push('\n');
    }
    output.push_str(&spaces(indent));
    output.push(']');
    Ok(output)
}

#[requires(true)]
#[bityzba::ensures(true)]
fn format_toml_value<T: Serialize + ?Sized>(value: &T) -> Result<String, toml::ser::Error> {
    let mut output = String::new();
    value.serialize(toml::ser::ValueSerializer::new(&mut output))?;
    Ok(output)
}

#[requires(true)]
#[bityzba::ensures(true)]
fn spaces(count: usize) -> String {
    " ".repeat(count)
}

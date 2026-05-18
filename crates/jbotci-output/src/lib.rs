//! Output format selection and render facade.

mod sexpr;
mod surface;
mod walker;

use bityzba::{invariant, requires};
use jbotci_syntax::{SyntaxValue, SyntaxValueData};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub enum OutputBase {
    Compact,
    Ipa,
    Tree,
    Raw,
    Camxes,
    Svg,
    Gloss,
    Xml,
    MermaidFlowchart,
    MermaidBlock,
    Markdown,
    Lean,
    Paraphrase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub enum OutputFeature {
    WordKind,
    Definitions,
    Color,
    CompactXml,
    Gloss,
    LeanPrelude,
    LeanUnicode,
    LeanSyntheticNames,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct OutputFormat {
    pub base: OutputBase,
    pub features: Vec<OutputFeature>,
}

impl Default for OutputFormat {
    #[requires(true)]
    #[ensures(ret.base == OutputBase::Compact && ret.features.is_empty())]
    fn default() -> Self {
        Self {
            base: OutputBase::Compact,
            features: Vec::new(),
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[invariant(true)]
pub enum OutputError {
    #[error("output rendering is not implemented yet")]
    NotImplemented,
    #[error("invalid syntax tree for bracket rendering: {0}")]
    InvalidSyntaxTree(String),
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || matches!(tree.as_data(), bityzba::data!(SyntaxValue::Null)))]
pub fn pretty_brackets(tree: &SyntaxValue, source: &str) -> Result<String, OutputError> {
    let sexpr = walker::to_sexpr(tree, source)?;
    Ok(sexpr::render_bracketed(&sexpr::flatten(sexpr)))
}

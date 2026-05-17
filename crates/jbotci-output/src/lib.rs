//! Output format selection and render facade.

use bityzba::{data, invariant, requires};
use jbotci_morphology::{
    Word, WordKind, WordLike, WordLikeData, WordWithModifiers, WordWithModifiersData,
};
use jbotci_source::SourceSpan;
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
    #[ensures(true)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
enum SurfaceChunk {
    Word(String),
    QuotedWords(Vec<Word>),
    QuotedText(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
enum SExpr {
    Leaf(String),
    Node(Vec<SExpr>),
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || matches!(tree.as_data(), data!(SyntaxValue::Null)))]
pub fn pretty_brackets(tree: &SyntaxValue, source: &str) -> Result<String, OutputError> {
    Ok(render_bracketed(0, &flatten(to_sexpr(tree, source)?)))
}

#[requires(true)]
#[ensures(true)]
fn to_sexpr(value: &SyntaxValue, source: &str) -> Result<SExpr, OutputError> {
    match value.as_data() {
        data!(SyntaxValue::Null)
        | data!(SyntaxValue::Bool { .. })
        | data!(SyntaxValue::Integer { .. })
        | data!(SyntaxValue::Json { .. }) => Ok(empty_node()),
        data!(SyntaxValue::Text { value }) => Ok(SExpr::Leaf(value.clone())),
        data!(SyntaxValue::Word { word }) => Ok(word_leaf(word, source)),
        data!(SyntaxValue::List { items }) => list_sexpr(items, source),
        data!(SyntaxValue::Node { node: syntax_node }) => {
            let constructor = syntax_node.constructor.as_str();
            if constructor == "[]" || constructor == "Nothing" {
                return Ok(empty_node());
            }
            if constructor == "(:)" {
                return cons_node_sexpr(value, source);
            }
            if is_compound_quote_node(constructor, value) {
                return compound_quote_sexpr(value, source);
            }
            syntax_node
                .fields
                .iter()
                .map(|field| {
                    field_value_sexpr(constructor, field.name.as_deref(), &field.value, source)
                })
                .collect::<Result<Vec<_>, _>>()
                .map(node)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn list_sexpr(items: &[SyntaxValue], source: &str) -> Result<SExpr, OutputError> {
    let mut children = Vec::new();
    for item in items {
        let sexpr = flatten(to_sexpr(item, source)?);
        match sexpr {
            SExpr::Node(items) => children.extend(items),
            leaf => children.push(leaf),
        }
    }
    Ok(node(children))
}

#[requires(true)]
#[ensures(true)]
fn cons_node_sexpr(value: &SyntaxValue, source: &str) -> Result<SExpr, OutputError> {
    let data!(SyntaxValue::Node { node: syntax_node }) = value.as_data() else {
        return Ok(empty_node());
    };
    let mut children = Vec::new();
    for field in &syntax_node.fields {
        let sexpr = flatten(to_sexpr(&field.value, source)?);
        match sexpr {
            SExpr::Node(items) => children.extend(items),
            leaf => children.push(leaf),
        }
    }
    Ok(node(children))
}

#[requires(true)]
#[ensures(true)]
fn field_value_sexpr(
    constructor: &str,
    field_name: Option<&str>,
    value: &SyntaxValue,
    source: &str,
) -> Result<SExpr, OutputError> {
    match (constructor, field_name, value.as_data()) {
        (
            "ZoiQuote" | "LahoQuote" | "MuhoiRelationUnit",
            Some("quotedText"),
            data!(SyntaxValue::Text { value }),
        ) => Ok(SExpr::Leaf(format!("\"{value}\""))),
        (
            "ZohOiQuote" | "MehoiRelationUnit" | "GohoiRelationUnit",
            Some("quotedText"),
            data!(SyntaxValue::Text { value }),
        ) => Ok(SExpr::Leaf(format!("«{value}»"))),
        _ => to_sexpr(value, source),
    }
}

#[requires(true)]
#[ensures(true)]
fn is_compound_quote_node(constructor: &str, value: &SyntaxValue) -> bool {
    matches!(
        constructor,
        "ZoQuote" | "ZohOiQuote" | "ZoiQuote" | "LahoQuote" | "LohuQuote"
    ) && first_word_field(value).is_some_and(is_compound_word_with_modifiers)
}

#[requires(true)]
#[ensures(true)]
fn compound_quote_sexpr(value: &SyntaxValue, source: &str) -> Result<SExpr, OutputError> {
    let word = first_word_field(value).ok_or_else(|| {
        OutputError::InvalidSyntaxTree("compound quote node has no leading word".to_owned())
    })?;
    let mut children = vec![word_leaf(word, source)];
    if let Some(free_modifiers) = named_field(value, "freeModifiers") {
        children.push(to_sexpr(free_modifiers, source)?);
    } else if let Some(free_modifiers) = named_field(value, "lehuFreeModifiers") {
        children.push(to_sexpr(free_modifiers, source)?);
    }
    Ok(node(children))
}

#[requires(true)]
#[ensures(true)]
fn first_word_field(value: &SyntaxValue) -> Option<&WordWithModifiers> {
    let data!(SyntaxValue::Node { node }) = value.as_data() else {
        return None;
    };
    node.fields.iter().find_map(|field| {
        let data!(SyntaxValue::Word { word }) = field.value.as_data() else {
            return None;
        };
        Some(word.as_ref())
    })
}

#[requires(!name.is_empty())]
#[ensures(true)]
fn named_field<'tree>(value: &'tree SyntaxValue, name: &str) -> Option<&'tree SyntaxValue> {
    let data!(SyntaxValue::Node { node }) = value.as_data() else {
        return None;
    };
    node.fields
        .iter()
        .find(|field| field.name.as_deref() == Some(name))
        .map(|field| &field.value)
}

#[requires(true)]
#[ensures(true)]
fn word_leaf(word: &WordWithModifiers, source: &str) -> SExpr {
    let text = format_word_with_modifiers(word, source);
    if text.is_empty() {
        empty_node()
    } else {
        SExpr::Leaf(text)
    }
}

#[requires(true)]
#[ensures(true)]
fn empty_node() -> SExpr {
    SExpr::Node(Vec::new())
}

#[requires(true)]
#[ensures(true)]
fn node(children: Vec<SExpr>) -> SExpr {
    SExpr::Node(
        children
            .into_iter()
            .filter(|child| !sexpr_is_empty(child))
            .collect(),
    )
}

#[requires(true)]
#[ensures(true)]
fn sexpr_is_empty(expr: &SExpr) -> bool {
    match expr {
        SExpr::Leaf(text) => text.is_empty(),
        SExpr::Node(children) => children.is_empty(),
    }
}

#[requires(true)]
#[ensures(true)]
fn flatten(expr: SExpr) -> SExpr {
    match expr {
        SExpr::Leaf(text) => SExpr::Leaf(text),
        SExpr::Node(children) => {
            let mut flattened = children
                .into_iter()
                .map(flatten)
                .filter(|child| !sexpr_is_empty(child))
                .collect::<Vec<_>>();
            if flattened.len() == 1 {
                flattened.remove(0)
            } else {
                SExpr::Node(flattened)
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_bracketed(depth: usize, expr: &SExpr) -> String {
    match expr {
        SExpr::Leaf(text) => text.clone(),
        SExpr::Node(children) => {
            let rendered = children
                .iter()
                .map(|child| render_bracketed(depth + 1, child))
                .filter(|text| !text.is_empty())
                .collect::<Vec<_>>();
            match rendered.as_slice() {
                [] => String::new(),
                [single] => single.clone(),
                _ => {
                    let (open, close) = bracket_pair(depth);
                    format!("{open}{}{close}", rendered.join(" "))
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(!ret.0.is_empty() && !ret.1.is_empty())]
fn bracket_pair(depth: usize) -> (&'static str, &'static str) {
    match depth % 3 {
        0 => ("(", ")"),
        1 => ("[", "]"),
        _ => ("{", "}"),
    }
}

#[requires(true)]
#[ensures(true)]
fn is_compound_word_with_modifiers(word: &WordWithModifiers) -> bool {
    match word.as_data() {
        data!(WordWithModifiers::StandaloneIndicator { .. })
        | data!(WordWithModifiers::Emphasized { .. })
        | data!(WordWithModifiers::WithIndicator { .. }) => true,
        data!(WordWithModifiers::BaseWord { word_like }) => match word_like.as_data() {
            data!(WordLike::Bare { .. }) => false,
            data!(WordLike::ZoQuote { .. })
            | data!(WordLike::ZoiQuote { .. })
            | data!(WordLike::LohuQuote { .. })
            | data!(WordLike::SingleWordQuote { .. })
            | data!(WordLike::Letter { .. })
            | data!(WordLike::ZeiLujvo { .. }) => true,
        },
        data!(WordWithModifiers::NotEof) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn format_word_with_modifiers(word: &WordWithModifiers, source: &str) -> String {
    render_surface_chunks(flatten_word_with_modifiers_surface(word, source))
}

#[requires(true)]
#[ensures(true)]
fn flatten_word_with_modifiers_surface(
    word: &WordWithModifiers,
    source: &str,
) -> Vec<SurfaceChunk> {
    match word.as_data() {
        data!(WordWithModifiers::BaseWord { word_like }) => {
            flatten_word_like_surface(word_like, source)
        }
        data!(WordWithModifiers::StandaloneIndicator { indicator, nai }) => {
            let mut chunks = vec![SurfaceChunk::Word(render_word(indicator))];
            if let Some(nai) = nai {
                chunks.push(SurfaceChunk::Word(render_word(nai)));
            }
            chunks
        }
        data!(WordWithModifiers::Emphasized { bahe, word_like }) => {
            let mut chunks = vec![SurfaceChunk::Word(render_word(bahe))];
            chunks.extend(flatten_word_like_surface(word_like, source));
            chunks
        }
        data!(WordWithModifiers::WithIndicator {
            base,
            indicator,
            nai,
        }) => {
            let mut chunks = flatten_word_with_modifiers_surface(base, source);
            chunks.push(SurfaceChunk::Word(render_word(indicator)));
            if let Some(nai) = nai {
                chunks.push(SurfaceChunk::Word(render_word(nai)));
            }
            chunks
        }
        data!(WordWithModifiers::NotEof) => Vec::new(),
    }
}

#[requires(true)]
#[ensures(true)]
fn flatten_word_like_surface(word_like: &WordLike, source: &str) -> Vec<SurfaceChunk> {
    match word_like.as_data() {
        data!(WordLike::Bare { word }) => vec![SurfaceChunk::Word(render_word(word))],
        data!(WordLike::ZoQuote { zo, word }) => vec![
            SurfaceChunk::Word(render_word(zo)),
            SurfaceChunk::QuotedWords(vec![(**word).clone()]),
        ],
        data!(WordLike::ZoiQuote {
            zoi,
            opening_delimiter,
            quoted_text,
            closing_delimiter,
        }) => vec![
            SurfaceChunk::Word(render_word(zoi)),
            SurfaceChunk::Word(render_word_without_pause(opening_delimiter)),
            SurfaceChunk::QuotedText(drop_leading_zoi_separator(source_slice(
                source,
                quoted_text,
            ))),
            SurfaceChunk::Word(render_word_without_pause(closing_delimiter)),
        ],
        data!(WordLike::LohuQuote {
            lohu,
            quoted_words,
            lehu,
        }) => vec![
            SurfaceChunk::Word(render_word(lohu)),
            SurfaceChunk::QuotedWords(quoted_words.clone()),
            SurfaceChunk::Word(render_word(lehu)),
        ],
        data!(WordLike::SingleWordQuote {
            marker,
            quoted_text,
        }) => vec![
            SurfaceChunk::Word(render_word(marker)),
            SurfaceChunk::QuotedText(source_slice(source, quoted_text)),
        ],
        data!(WordLike::Letter { base, bu }) => {
            let mut chunks = flatten_word_like_surface(base, source);
            chunks.push(SurfaceChunk::Word(render_word(bu)));
            chunks
        }
        data!(WordLike::ZeiLujvo { left, zei, right }) => {
            let mut chunks = flatten_word_like_surface(left, source);
            chunks.push(SurfaceChunk::Word(render_word(zei)));
            chunks.push(SurfaceChunk::Word(render_word(right)));
            chunks
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn source_slice(source: &str, span: &SourceSpan) -> String {
    source
        .get(span.byte_start..span.byte_end)
        .unwrap_or_default()
        .to_owned()
}

#[requires(true)]
#[ensures(true)]
fn drop_leading_zoi_separator(text: String) -> String {
    text.strip_prefix(char::is_whitespace)
        .unwrap_or(&text)
        .to_owned()
}

#[requires(true)]
#[ensures(true)]
fn render_surface_chunks(chunks: Vec<SurfaceChunk>) -> String {
    let rendered = chunks
        .into_iter()
        .map(render_surface_chunk)
        .filter(|chunk| !chunk.is_empty())
        .collect::<Vec<_>>();
    let Some((first, rest)) = rendered.split_first() else {
        return String::new();
    };
    rest.iter().fold(first.clone(), |mut acc, next| {
        if !ends_with_visible_pause_dot(&acc) && !starts_with_visible_pause_dot(next) {
            acc.push('-');
        }
        acc.push_str(next);
        acc
    })
}

#[requires(true)]
#[ensures(true)]
fn render_surface_chunk(chunk: SurfaceChunk) -> String {
    match chunk {
        SurfaceChunk::Word(word) => word,
        SurfaceChunk::QuotedWords(words) => format!(
            "«{}»",
            words.iter().map(render_word).collect::<Vec<_>>().join(" ")
        ),
        SurfaceChunk::QuotedText(text) => format!("«{text}»"),
    }
}

#[requires(true)]
#[ensures(true)]
fn starts_with_visible_pause_dot(text: &str) -> bool {
    text.chars().next().is_some_and(is_visible_pause_dot)
}

#[requires(true)]
#[ensures(true)]
fn ends_with_visible_pause_dot(text: &str) -> bool {
    text.chars().next_back().is_some_and(is_visible_pause_dot)
}

#[requires(true)]
#[ensures(true)]
fn is_visible_pause_dot(ch: char) -> bool {
    ch == '.'
}

#[requires(true)]
#[ensures(true)]
fn render_word(word: &Word) -> String {
    if let Some(surface_override) = &word.surface_override {
        return surface_override.clone();
    }
    render_visible_word_surface(word)
}

#[requires(true)]
#[ensures(true)]
fn render_word_without_pause(word: &Word) -> String {
    if let Some(surface_override) = &word.surface_override {
        return surface_override.clone();
    }
    match word.kind {
        WordKind::Cmavo | WordKind::Cmevla => strip_stress_accents(&add_diacritics(&word.phonemes)),
        WordKind::Gismu | WordKind::Lujvo | WordKind::Fuhivla => add_diacritics(&word.phonemes),
    }
}

#[requires(true)]
#[ensures(true)]
fn render_visible_word_surface(word: &Word) -> String {
    let mut rendered = match word.kind {
        WordKind::Cmavo | WordKind::Cmevla => strip_stress_accents(&add_diacritics(&word.phonemes)),
        WordKind::Gismu | WordKind::Lujvo | WordKind::Fuhivla => add_diacritics(&word.phonemes),
    };
    if needs_leading_pause(word) {
        rendered.insert(0, '.');
    }
    if word.kind == WordKind::Cmevla {
        rendered.push('.');
    }
    rendered
}

#[requires(true)]
#[ensures(true)]
fn needs_leading_pause(word: &Word) -> bool {
    word.kind == WordKind::Cmevla
        || strip_diacritics(&word.phonemes)
            .chars()
            .next()
            .is_some_and(|ch| matches!(ch, 'a' | 'e' | 'i' | 'o' | 'u'))
}

#[requires(true)]
#[ensures(true)]
fn add_diacritics(text: &str) -> String {
    mark_stress(&normalize_uppercase_stress(text))
}

#[requires(true)]
#[ensures(true)]
fn normalize_uppercase_stress(text: &str) -> String {
    text.chars()
        .map(|ch| match ch {
            'A' | 'Á' | 'À' | 'à' => 'á',
            'E' | 'É' | 'È' | 'è' => 'é',
            'I' | 'Í' | 'Ì' | 'ì' => 'í',
            'O' | 'Ó' | 'Ò' | 'ò' => 'ó',
            'U' | 'Ú' | 'Ù' | 'ù' => 'ú',
            'Y' | 'Ý' | 'Ỳ' | 'ỳ' => 'ý',
            other => other,
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn mark_stress(text: &str) -> String {
    if has_explicit_stress(text) {
        return text.to_owned();
    }
    let stressable = text
        .char_indices()
        .filter_map(|(index, ch)| is_full_vowel(ch).then_some(index))
        .collect::<Vec<_>>();
    let Some(&stress_index) = stressable.iter().rev().nth(1) else {
        return text.to_owned();
    };
    let mut rendered = String::with_capacity(text.len() + 1);
    for (index, ch) in text.char_indices() {
        if index == stress_index {
            rendered.push(acute_vowel(ch));
        } else {
            rendered.push(ch);
        }
    }
    rendered
}

#[requires(true)]
#[ensures(true)]
fn has_explicit_stress(text: &str) -> bool {
    text.chars().any(|ch| {
        matches!(
            ch,
            'á' | 'é' | 'í' | 'ó' | 'ú' | 'ý' | 'à' | 'è' | 'ì' | 'ò' | 'ù' | 'ỳ'
        )
    })
}

#[requires(true)]
#[ensures(true)]
fn is_full_vowel(ch: char) -> bool {
    matches!(
        ch,
        'a' | 'e' | 'i' | 'o' | 'u' | 'á' | 'é' | 'í' | 'ó' | 'ú' | 'à' | 'è' | 'ì' | 'ò' | 'ù'
    )
}

#[requires(true)]
#[ensures(true)]
fn acute_vowel(ch: char) -> char {
    match ch {
        'a' | 'á' | 'à' => 'á',
        'e' | 'é' | 'è' => 'é',
        'i' | 'í' | 'ì' => 'í',
        'o' | 'ó' | 'ò' => 'ó',
        'u' | 'ú' | 'ù' => 'ú',
        other => other,
    }
}

#[requires(true)]
#[ensures(true)]
fn strip_stress_accents(text: &str) -> String {
    text.chars()
        .map(|ch| match ch {
            'á' | 'à' => 'a',
            'é' | 'è' => 'e',
            'í' | 'ì' => 'i',
            'ó' | 'ò' => 'o',
            'ú' | 'ù' => 'u',
            'ý' | 'ỳ' => 'y',
            other => other,
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn strip_diacritics(text: &str) -> String {
    text.chars()
        .filter_map(|ch| match ch {
            'á' | 'à' | 'Á' | 'À' => Some('a'),
            'é' | 'è' | 'É' | 'È' => Some('e'),
            'í' | 'ì' | 'ĭ' | 'Ĭ' | 'Í' | 'Ì' => Some('i'),
            'ó' | 'ò' | 'Ó' | 'Ò' => Some('o'),
            'ú' | 'ù' | 'ŭ' | 'Ŭ' | 'Ú' | 'Ù' => Some('u'),
            'ý' | 'ỳ' | 'Ý' | 'Ỳ' => Some('y'),
            '\u{0301}' | '\u{0300}' | '\u{0306}' => None,
            other => Some(other),
        })
        .collect()
}

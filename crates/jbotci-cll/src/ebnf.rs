use std::collections::BTreeSet;

use bityzba::{data, invariant, new, requires};
use roxmltree::Node;
use serde::{Deserialize, Serialize};

use super::{
    AnchorMode, CllAnchor, CllBlock, SectionParseContext, attr_string, block_anchor_id_for,
    child_element, cll_import_metadata, normalized_plain_text, section_href, visible_text,
    visible_text_raw, xml_id,
};

#[invariant(!id.is_empty())]
#[invariant(!label.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingEbnfAnchor {
    id: String,
    label: String,
}

#[invariant(!rule_name.is_empty())]
#[invariant(!anchor_id.is_empty())]
#[invariant(rule_href.as_ref().is_none_or(|href| !href.is_empty()))]
#[invariant(!rhs.is_empty())]
#[invariant(source_anchor_ids.iter().all(|id| !id.is_empty()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllEbnfEntry {
    pub rule_name: String,
    pub anchor_id: String,
    pub rule_href: Option<String>,
    pub rhs: Vec<CllEbnfToken>,
    pub source_anchor_ids: Vec<String>,
}

#[invariant(true)]
#[invariant(::Text { .. } => true)]
#[invariant(::Operator { .. } => true)]
#[invariant(::Hash { .. } => true)]
#[invariant(::Terminal { .. } => true)]
#[invariant(::ElidableTerminator { .. } => true)]
#[invariant(::Nonterminal { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CllEbnfToken {
    Text { body: String },
    Operator { body: String },
    Hash { body: String },
    Terminal { body: String, href: Option<String> },
    ElidableTerminator { body: String, href: Option<String> },
    Nonterminal { body: String, href: Option<String> },
}

#[requires(node.is_element())]
#[ensures(true)]
pub(super) fn parse_ebnf_block(
    node: Node<'_, '_>,
    context: &SectionParseContext,
    anchors: &mut Vec<(String, CllAnchor)>,
) -> Option<CllBlock> {
    let entry_nodes = node
        .children()
        .filter(|child| child.is_element() && child.has_tag_name("varlistentry"))
        .collect::<Vec<_>>();
    let defined_rules = entry_nodes
        .iter()
        .filter_map(|entry| child_element(*entry, "term"))
        .map(|term| extract_ebnf_rule_name(&visible_text(term)))
        .filter(|rule| !rule.is_empty())
        .collect::<BTreeSet<_>>();
    let mut entries = Vec::with_capacity(entry_nodes.len());
    let mut pending_source_anchors = Vec::new();
    for entry in entry_nodes {
        let Some(term) = child_element(entry, "term") else {
            continue;
        };
        pending_source_anchors.extend(parse_ebnf_source_anchors(term));
        let source_anchor_ids = pending_source_anchors
            .iter()
            .map(|anchor| anchor.id.clone())
            .collect();
        let Some(parsed) = parse_ebnf_entry(entry, &defined_rules, source_anchor_ids) else {
            pending_source_anchors = Vec::new();
            continue;
        };
        for source_anchor in pending_source_anchors {
            let data!(PendingEbnfAnchor { id, label }) = source_anchor.into_data();
            anchors.push((
                id,
                new!(CllAnchor {
                    section_id: context.section_id.clone(),
                    label,
                }),
            ));
        }
        pending_source_anchors = child_element(entry, "listitem")
            .into_iter()
            .flat_map(parse_ebnf_source_anchors)
            .collect();
        entries.push(parsed);
    }
    (!entries.is_empty()).then_some(CllBlock::Ebnf {
        id: block_anchor_id_for("ebnf", AnchorMode::TopLevel, context, node),
        entries,
    })
}

#[requires(entry.is_element())]
#[ensures(true)]
fn parse_ebnf_entry(
    entry: Node<'_, '_>,
    defined_rules: &BTreeSet<String>,
    source_anchor_ids: Vec<String>,
) -> Option<CllEbnfEntry> {
    let term = child_element(entry, "term")?;
    let listitem = child_element(entry, "listitem")?;
    let para = child_element(listitem, "para")?;
    let rule_name = extract_ebnf_rule_name(&visible_text(term));
    let rhs_text = normalized_plain_text(&visible_text_raw(para));
    if rule_name.is_empty() || rhs_text.is_empty() {
        return None;
    }
    Some(new!(CllEbnfEntry {
        anchor_id: ebnf_rule_anchor_id(&rule_name),
        rule_href: ebnf_symbol_href(&rule_name),
        rhs: tokenize_ebnf_rule_rhs(&rule_name, defined_rules, &rhs_text),
        rule_name,
        source_anchor_ids,
    }))
}

#[requires(node.is_element())]
#[ensures(ret.iter().all(|anchor| !anchor.id.is_empty() && !anchor.label.is_empty()))]
fn parse_ebnf_source_anchors(node: Node<'_, '_>) -> Vec<PendingEbnfAnchor> {
    node.descendants()
        .filter(|descendant| descendant.is_element() && descendant.has_tag_name("anchor"))
        .filter_map(|anchor| {
            let id = xml_id(anchor)?;
            let label = attr_string(anchor, "xreflabel").unwrap_or_else(|| id.clone());
            Some(new!(PendingEbnfAnchor { id, label }))
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn extract_ebnf_rule_name(text: &str) -> String {
    text.trim()
        .chars()
        .take_while(|character| {
            character.is_ascii_alphanumeric() || *character == '-' || *character == '\''
        })
        .collect()
}

#[requires(!rule_name.is_empty())]
#[ensures(ret.starts_with("ebnf-rule-"))]
pub fn ebnf_rule_anchor_id(rule_name: &str) -> String {
    let slug = rule_name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    format!("ebnf-rule-{slug}")
}

#[requires(true)]
#[ensures(true)]
fn tokenize_ebnf_rule_rhs(
    rule_name: &str,
    defined_rules: &BTreeSet<String>,
    rhs_text: &str,
) -> Vec<CllEbnfToken> {
    if rule_name == "any-word" || rule_name == "anything" {
        return vec![CllEbnfToken::Text {
            body: rhs_text.to_owned(),
        }];
    }
    tokenize_ebnf_tokens(defined_rules, rhs_text)
}

#[requires(true)]
#[ensures(true)]
fn tokenize_ebnf_tokens(defined_rules: &BTreeSet<String>, rhs_text: &str) -> Vec<CllEbnfToken> {
    let chars = rhs_text.chars().collect::<Vec<_>>();
    let mut index = 0usize;
    let mut tokens = Vec::new();
    while index < chars.len() {
        let character = chars[index];
        if character.is_whitespace() {
            let start = index;
            while index < chars.len() && chars[index].is_whitespace() {
                index += 1;
            }
            tokens.push(CllEbnfToken::Text {
                body: chars[start..index].iter().collect(),
            });
        } else if character == '\u{201c}' {
            let start = index;
            index += 1;
            while index < chars.len() && chars[index] != '\u{201d}' {
                index += 1;
            }
            if index < chars.len() {
                index += 1;
            }
            tokens.push(CllEbnfToken::Text {
                body: chars[start..index].iter().collect(),
            });
        } else if character == '/' {
            if let Some((body, symbol, next_index)) = parse_ebnf_elidable(&chars, index) {
                tokens.push(CllEbnfToken::ElidableTerminator {
                    body,
                    href: ebnf_symbol_href(&symbol),
                });
                index = next_index;
            } else {
                tokens.push(CllEbnfToken::Operator {
                    body: character.to_string(),
                });
                index += 1;
            }
        } else if index + 3 <= chars.len() && chars[index..index + 3] == ['.', '.', '.'] {
            tokens.push(CllEbnfToken::Operator {
                body: "...".to_owned(),
            });
            index += 3;
        } else if is_ebnf_boundary(character) {
            let body = character.to_string();
            if character == '#' {
                tokens.push(CllEbnfToken::Hash { body });
            } else {
                tokens.push(CllEbnfToken::Operator { body });
            }
            index += 1;
        } else if is_ebnf_identifier_char(character) {
            let start = index;
            while index < chars.len() && is_ebnf_identifier_char(chars[index]) {
                index += 1;
            }
            let body = chars[start..index].iter().collect::<String>();
            tokens.push(classify_ebnf_identifier(&body, defined_rules));
        } else {
            tokens.push(CllEbnfToken::Text {
                body: character.to_string(),
            });
            index += 1;
        }
    }
    tokens
}

#[requires(index < chars.len())]
#[ensures(true)]
fn parse_ebnf_elidable(chars: &[char], index: usize) -> Option<(String, String, usize)> {
    let mut cursor = index + 1;
    let symbol_start = cursor;
    while cursor < chars.len() && is_ebnf_identifier_char(chars[cursor]) {
        cursor += 1;
    }
    if cursor == symbol_start {
        return None;
    }
    let symbol = chars[symbol_start..cursor].iter().collect::<String>();
    if cursor < chars.len() && chars[cursor] == '/' {
        cursor += 1;
        return Some((chars[index..cursor].iter().collect(), symbol, cursor));
    }
    if cursor + 1 < chars.len() && chars[cursor] == '#' && chars[cursor + 1] == '/' {
        cursor += 2;
        return Some((chars[index..cursor].iter().collect(), symbol, cursor));
    }
    None
}

#[requires(true)]
#[ensures(true)]
fn classify_ebnf_identifier(body: &str, defined_rules: &BTreeSet<String>) -> CllEbnfToken {
    if let Some(href) = ebnf_symbol_href(body) {
        return CllEbnfToken::Terminal {
            body: body.to_owned(),
            href: Some(href),
        };
    }
    if defined_rules.contains(body) {
        return CllEbnfToken::Nonterminal {
            body: body.to_owned(),
            href: Some(format!("#{}", ebnf_rule_anchor_id(body))),
        };
    }
    let letters = body
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<Vec<_>>();
    if !letters.is_empty()
        && letters
            .iter()
            .all(|character| !character.is_ascii_lowercase())
    {
        return CllEbnfToken::Terminal {
            body: body.to_owned(),
            href: ebnf_symbol_href(body),
        };
    }
    if letters
        .iter()
        .any(|character| character.is_ascii_lowercase())
    {
        return CllEbnfToken::Nonterminal {
            body: body.to_owned(),
            href: None,
        };
    }
    CllEbnfToken::Text {
        body: body.to_owned(),
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn ebnf_symbol_href(symbol: &str) -> Option<String> {
    if let Some(section_id) = cll_import_metadata().ebnf_symbols.get(symbol) {
        return Some(section_href(section_id));
    }
    if symbol
        .chars()
        .any(|character| character.is_ascii_uppercase())
    {
        Some(format!("{}#{symbol}", section_href("section-index")))
    } else {
        None
    }
}

#[requires(true)]
#[ensures(true)]
fn is_ebnf_boundary(character: char) -> bool {
    matches!(character, '|' | '&' | '[' | ']' | '(' | ')' | '=' | '#')
}

#[requires(true)]
#[ensures(true)]
fn is_ebnf_identifier_char(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '-' || character == '\''
}

#[requires(true)]
#[ensures(true)]
pub fn wrap_ebnf_choice_lines(tokens: &[CllEbnfToken]) -> Vec<Vec<CllEbnfToken>> {
    let mut lines = Vec::new();
    let mut current = Vec::new();
    let mut depth = 0usize;
    for token in tokens {
        if depth == 0 && matches!(token, CllEbnfToken::Operator { body } if body == "|") {
            current.push(token.clone());
            push_trimmed_ebnf_line(&mut lines, &mut current);
        } else {
            depth = next_ebnf_depth(depth, token);
            current.push(token.clone());
        }
    }
    push_trimmed_ebnf_line(&mut lines, &mut current);
    if lines.len() <= 1 {
        vec![tokens.to_vec()]
    } else {
        lines
    }
}

#[requires(true)]
#[ensures(current.is_empty())]
fn push_trimmed_ebnf_line(lines: &mut Vec<Vec<CllEbnfToken>>, current: &mut Vec<CllEbnfToken>) {
    let line = std::mem::take(current);
    let start = line
        .iter()
        .position(|token| !ebnf_token_is_whitespace(token))
        .unwrap_or(line.len());
    let end = line
        .iter()
        .rposition(|token| !ebnf_token_is_whitespace(token))
        .map(|index| index + 1)
        .unwrap_or(start);
    if start < end {
        lines.push(line[start..end].to_vec());
    }
}

#[requires(true)]
#[ensures(true)]
fn ebnf_token_is_whitespace(token: &CllEbnfToken) -> bool {
    matches!(token, CllEbnfToken::Text { body } if body.chars().all(char::is_whitespace))
}

#[requires(true)]
#[ensures(true)]
fn next_ebnf_depth(depth: usize, token: &CllEbnfToken) -> usize {
    match token {
        CllEbnfToken::Operator { body } if body == "[" || body == "(" => depth + 1,
        CllEbnfToken::Operator { body } if body == "]" || body == ")" => depth.saturating_sub(1),
        _ => depth,
    }
}

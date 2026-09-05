//! Dictionary attestation and explicit presentation partitioning over morphology words.
//!
//! Blocks select longest intervals globally, breaking equal-length ties to the left.
//! Cursor documentation instead selects the longest interval containing that cursor.

use std::collections::BTreeMap;

use bityzba::{data, invariant, new, requires};
use jbotci_dictionary::{Dictionary, EntryIndex, is_compound_separator};
use jbotci_morphology::{Word, WordKind, WordLike, WordLikeData};
use jbotci_source::SourceSpan;

use crate::vlacku::{VlackuCard, dictionary_entry_card, word_like_lookup_text};

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParsedCompoundKind {
    CmavoSequence,
    Zei,
}

/// An unavailable source region. A zero-width barrier splits an enclosing interval.
#[invariant(byte_start <= byte_end)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompoundBarrier {
    pub byte_start: usize,
    pub byte_end: usize,
}

impl CompoundBarrier {
    #[requires(start <= end)]
    #[ensures(true)]
    pub fn intersects(&self, start: usize, end: usize) -> bool {
        self.byte_start < end && start < self.byte_end
            || (self.byte_start == self.byte_end
                && start < self.byte_start
                && self.byte_start < end)
    }
}

#[invariant(members.len() >= 2 && !entry_indices.is_empty() && !lookup_text.is_empty())]
#[invariant(span.byte_start == members[0].byte_start && span.byte_end == members[members.len() - 1].byte_end)]
#[invariant(span.char_start == members[0].char_start && span.char_end == members[members.len() - 1].char_end)]
#[invariant(match kind { ParsedCompoundKind::CmavoSequence => components.len() == members.len(), ParsedCompoundKind::Zei => members.len() >= 3 && components.is_empty() })]
#[expensive_invariant(members.iter().all(|member| member.byte_start < member.byte_end && member.source_id == span.source_id))]
#[expensive_invariant(members.windows(2).all(|pair| pair[0].byte_end <= pair[1].byte_start && pair[0].char_end <= pair[1].char_start))]
#[expensive_invariant(entry_indices.windows(2).all(|pair| pair[0] < pair[1]))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCompoundMatch {
    pub kind: ParsedCompoundKind,
    pub span: SourceSpan,
    pub members: Vec<SourceSpan>,
    pub components: Vec<String>,
    pub lookup_text: String,
    pub entry_indices: Vec<EntryIndex>,
}

/// Construct only real entry cards; callers choose their presentation's first-card policy.
#[requires(compound.entry_indices.iter().all(|index| dictionary.entry_for_index(*index).is_some_and(|entry| entry.has_definition())))]
#[ensures(ret.len() == compound.entry_indices.len())]
pub fn compound_cards(
    dictionary: &Dictionary<'_>,
    compound: &ParsedCompoundMatch,
) -> Vec<VlackuCard> {
    compound
        .entry_indices
        .iter()
        .map(|index| {
            dictionary_entry_card(
                dictionary,
                dictionary
                    .entry_for_index(*index)
                    .expect("attested entry index"),
                None,
                false,
            )
        })
        .collect()
}

#[invariant(words.len() == components.len())]
#[derive(Default)]
struct CmavoRun<'a> {
    words: Vec<&'a Word>,
    components: Vec<String>,
}

#[requires(true)]
#[ensures(ret.is_none_or(|word| word.kind() == WordKind::Cmavo))]
fn plain_cmavo(word: &WordLike) -> Option<&Word> {
    word.bare_word()
        .filter(|word| word.kind() == WordKind::Cmavo)
}

#[requires(true)]
#[ensures(true)]
fn adjacent(
    source: &str,
    left: &SourceSpan,
    right: &SourceSpan,
    barriers: &[CompoundBarrier],
) -> bool {
    left.source_id == right.source_id
        && source
            .get(left.byte_end..right.byte_start)
            .is_some_and(|gap| gap.chars().all(is_compound_separator))
        && !barriers
            .iter()
            .any(|barrier| barrier.intersects(left.byte_start, right.byte_end))
}

/// Recognize complete attested groups, with recovery excluded before overlap selection.
#[requires(true)]
#[ensures(ret.windows(2).all(|pair| pair[0].span.byte_end <= pair[1].span.byte_start))]
#[expensive_ensures(ret.iter().all(|item| !barriers.iter().any(|barrier| barrier.intersects(item.span.byte_start, item.span.byte_end))))]
pub fn recognize_compounds(
    dictionary: &Dictionary<'_>,
    words: &[WordLike],
    source: &str,
    barriers: &[CompoundBarrier],
) -> Vec<ParsedCompoundMatch> {
    let mut result = Vec::new();
    let mut run = CmavoRun::default();
    for word_like in words {
        let word = plain_cmavo(word_like).filter(|word| {
            !barriers
                .iter()
                .any(|barrier| barrier.intersects(word.span().byte_start, word.span().byte_end))
        });
        if word.is_none()
            || run
                .words
                .last()
                .zip(word)
                .is_some_and(|(left, right)| !adjacent(source, left.span(), right.span(), barriers))
        {
            append_partition(dictionary, std::mem::take(&mut run), &mut result);
        }
        if let Some(word) = word {
            let mut run_data = run.into_data();
            run_data.words.push(word);
            run_data.components.push(word.canonical_phonemes());
            run = CmavoRun::from_data(run_data);
        } else if matches!(word_like.as_data(), data!(WordLike::ZeiCompound { .. })) {
            // This entire typed value is reserved even when no exact entry exists.
            if let Some(compound) = attested_zei(dictionary, word_like, source, barriers) {
                result.push(compound);
            }
        }
    }
    append_partition(dictionary, run, &mut result);
    result
}

#[requires(true)]
#[ensures(true)]
fn append_partition(
    dictionary: &Dictionary<'_>,
    run: CmavoRun<'_>,
    out: &mut Vec<ParsedCompoundMatch>,
) {
    let mut accepted = BTreeMap::<usize, usize>::new();
    for len in (2..=run.words.len().min(dictionary.max_cmavo_sequence_len())).rev() {
        for start in 0..=run.words.len() - len {
            let end = start + len;
            if accepted
                .range(..end)
                .next_back()
                .is_some_and(|(_, previous_end)| *previous_end > start)
            {
                continue;
            }
            if !dictionary
                .lookup_cmavo_sequence(&run.components[start..end])
                .is_empty()
            {
                accepted.insert(start, end);
            }
        }
    }
    for (start, end) in accepted {
        out.push(cmavo_match(dictionary, &run, start, end));
    }
}

#[requires(start < end && end <= run.words.len())]
#[requires(!dictionary.lookup_cmavo_sequence(&run.components[start..end]).is_empty())]
#[ensures(ret.kind == ParsedCompoundKind::CmavoSequence)]
fn cmavo_match(
    dictionary: &Dictionary<'_>,
    run: &CmavoRun<'_>,
    start: usize,
    end: usize,
) -> ParsedCompoundMatch {
    let targets = dictionary.lookup_cmavo_sequence(&run.components[start..end]);
    let first = run.words[start].span();
    let last = run.words[end - 1].span();
    new!(ParsedCompoundMatch {
        kind: ParsedCompoundKind::CmavoSequence,
        span: SourceSpan::new(
            first.source_id.clone(),
            first.byte_start,
            last.byte_end,
            first.char_start,
            last.char_end
        )
        .expect("ordered morphology run"),
        members: run.words[start..end]
            .iter()
            .map(|word| word.span().clone())
            .collect(),
        components: run.components[start..end].to_vec(),
        lookup_text: dictionary
            .entry_for_index(targets[0])
            .expect("generated index target")
            .word
            .to_owned(),
        entry_indices: targets.to_vec(),
    })
}

#[requires(matches!(word.as_data(), data!(WordLike::ZeiCompound { .. })))]
#[ensures(ret.as_ref().is_none_or(|item| item.kind == ParsedCompoundKind::Zei))]
fn attested_zei(
    dictionary: &Dictionary<'_>,
    word: &WordLike,
    source: &str,
    barriers: &[CompoundBarrier],
) -> Option<ParsedCompoundMatch> {
    let lookup_text = word_like_lookup_text(word)?;
    let targets: Vec<_> = dictionary.exact_definition_indices(&lookup_text).collect();
    if targets.is_empty() {
        return None;
    }
    let mut spans = Vec::new();
    word.source_spans_into(&mut spans);
    let first = *spans.first()?;
    let last = *spans.last()?;
    if barriers
        .iter()
        .any(|barrier| barrier.intersects(first.byte_start, last.byte_end))
        || !spans
            .windows(2)
            .all(|pair| adjacent(source, pair[0], pair[1], barriers))
    {
        return None;
    }
    let span = SourceSpan::new(
        first.source_id.clone(),
        first.byte_start,
        last.byte_end,
        first.char_start,
        last.char_end,
    )
    .ok()?;
    Some(new!(ParsedCompoundMatch {
        kind: ParsedCompoundKind::Zei,
        span,
        members: spans.into_iter().cloned().collect(),
        components: Vec::new(),
        lookup_text,
        entry_indices: targets,
    }))
}

/// Cursor-local counterpart, intentionally independent of the Blocks partition.
#[requires(index < words.len())]
#[ensures(ret.as_ref().is_none_or(|item| item.kind == ParsedCompoundKind::CmavoSequence))]
pub fn cmavo_sequence_containing(
    dictionary: &Dictionary<'_>,
    words: &[WordLike],
    source: &str,
    index: usize,
) -> Option<ParsedCompoundMatch> {
    plain_cmavo(&words[index])?;
    let mut start = index;
    while start > 0
        && plain_cmavo(&words[start - 1])
            .zip(plain_cmavo(&words[start]))
            .is_some_and(|(left, right)| adjacent(source, left.span(), right.span(), &[]))
    {
        start -= 1;
    }
    let mut end = index + 1;
    while end < words.len()
        && plain_cmavo(&words[end - 1])
            .zip(plain_cmavo(&words[end]))
            .is_some_and(|(left, right)| adjacent(source, left.span(), right.span(), &[]))
    {
        end += 1;
    }
    let run_words: Vec<_> = words[start..end]
        .iter()
        .map(|word| plain_cmavo(word).expect("plain cmavo run"))
        .collect();
    let components = run_words
        .iter()
        .map(|word| word.canonical_phonemes())
        .collect();
    let run = new!(CmavoRun {
        words: run_words,
        components
    });
    let cursor = index - start;
    for len in (2..=run.words.len().min(dictionary.max_cmavo_sequence_len())).rev() {
        for offset in (cursor + 1).saturating_sub(len)..=cursor.min(run.words.len() - len) {
            if !dictionary
                .lookup_cmavo_sequence(&run.components[offset..offset + len])
                .is_empty()
            {
                return Some(cmavo_match(dictionary, &run, offset, offset + len));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use jbotci_morphology::segment_words_with_modifiers;

    #[requires(true)]
    #[ensures(true)]
    fn matches(source: &str, barriers: &[CompoundBarrier]) -> Vec<ParsedCompoundMatch> {
        let words = segment_words_with_modifiers(source).unwrap();
        recognize_compounds(jbotci_dictionary_data::english(), &words, source, barriers)
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn longest_first_then_leftmost_is_a_global_partition() {
        let crossing = matches("ba pu ba", &[]);
        assert_eq!(crossing.len(), 1);
        assert_eq!(crossing[0].components, ["ba", "pu"]);
        let nested = matches("bi no no vo", &[]);
        assert_eq!(nested.len(), 1);
        assert_eq!(nested[0].members.len(), 4);
        let disjoint = matches("ba pu klama pu ba", &[]);
        assert_eq!(disjoint.len(), 2);
        assert_eq!(matches(".i je", &[])[0].components, ["i", "je"]);
        let words = segment_words_with_modifiers("ba pu ba").unwrap();
        let hover =
            cmavo_sequence_containing(jbotci_dictionary_data::english(), &words, "ba pu ba", 2)
                .unwrap();
        assert_eq!(hover.components, ["pu", "ba"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovery_and_source_gaps_are_barriers_before_selection() {
        let shorter = matches(
            "bi no no vo",
            &[new!(CompoundBarrier {
                byte_start: 6,
                byte_end: 8
            })],
        );
        assert_eq!(shorter.len(), 1);
        assert_eq!(shorter[0].components, ["bi", "no"]);
        for source in ["ba ! pu", "ba klama pu", "ba zo pu", "ba bu pu"] {
            assert!(matches(source, &[]).is_empty(), "{source}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn zei_requires_an_exact_complete_entry_and_reserves_its_members() {
        for source in ["batke zei uidje", "denpa bu zei sance"] {
            let selected = matches(source, &[]);
            assert_eq!(selected.len(), 1, "{source}");
            assert_eq!(selected[0].kind, ParsedCompoundKind::Zei);
            assert_eq!(selected[0].span.byte_end, source.len());
            assert!(!compound_cards(jbotci_dictionary_data::english(), &selected[0]).is_empty());
        }
        for source in [
            "ba pu zei ba",
            "ba zei pu ba",
            "klama zei tavla",
            "batke zei uidje zei klama",
            "zo pu zei ba",
        ] {
            assert!(matches(source, &[]).is_empty(), "{source}");
        }
    }
}

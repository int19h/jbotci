//! Dictionary attestation and explicit presentation partitioning over morphology words.
//!
//! Blocks select longest intervals globally, breaking equal-length ties to the left.
//! Cursor documentation instead selects the longest interval containing that cursor.

use std::collections::BTreeMap;

use bityzba::{data, expensive_invariant, invariant, new, requires};
use jbotci_dictionary::{Dictionary, EntryIndex, LookupKey, is_compound_separator};
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

/// Prefix maxima make recovery checks logarithmic even for many skipped regions.
#[expensive_invariant(end_by_start.iter().all(|(start, end)| start <= end))]
#[expensive_invariant(end_by_start.values().zip(end_by_start.values().skip(1)).all(|(left, right)| left <= right))]
#[derive(Default)]
struct BarrierIndex {
    end_by_start: BTreeMap<usize, usize>,
}

impl BarrierIndex {
    #[requires(true)]
    #[ensures(ret.end_by_start.len() <= barriers.len())]
    fn from_barriers(barriers: &[CompoundBarrier]) -> Self {
        let mut end_by_start = BTreeMap::<usize, usize>::new();
        for barrier in barriers {
            end_by_start
                .entry(barrier.byte_start)
                .and_modify(|end| *end = (*end).max(barrier.byte_end))
                .or_insert(barrier.byte_end);
        }
        let mut maximum = 0;
        for end in end_by_start.values_mut() {
            maximum = maximum.max(*end);
            *end = maximum;
        }
        new!(BarrierIndex { end_by_start })
    }

    #[requires(start <= end)]
    #[ensures(true)]
    fn intersects(&self, start: usize, end: usize) -> bool {
        self.end_by_start
            .range(..end)
            .next_back()
            .is_some_and(|(_, maximum)| start < *maximum)
    }
}

#[invariant(members.len() >= 2 && !entry_indices.is_empty() && !lookup_text.is_empty())]
#[invariant(span.byte_start == members[0].byte_start && span.byte_end == members[members.len() - 1].byte_end)]
#[invariant(span.char_start == members[0].char_start && span.char_end == members[members.len() - 1].char_end)]
#[invariant(match kind { ParsedCompoundKind::CmavoSequence => components.len() == members.len(), ParsedCompoundKind::Zei => members.len() >= 3 && components.is_empty() })]
#[expensive_invariant(members.iter().all(|member| member.byte_start < member.byte_end && member.source_id == span.source_id))]
#[expensive_invariant(members.windows(2).all(|pair| pair[0].byte_end <= pair[1].byte_start && pair[0].char_end <= pair[1].char_start))]
#[expensive_invariant(entry_indices.iter().enumerate().all(|(left, value)| !entry_indices[..left].contains(value)))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCompoundMatch {
    pub kind: ParsedCompoundKind,
    pub span: SourceSpan,
    pub members: Vec<SourceSpan>,
    pub components: Vec<String>,
    pub lookup_text: String,
    /// Attested entries, highest score first. Equal scores stay in source order.
    pub entry_indices: Vec<EntryIndex>,
}

/// Return one card for each attested entry, in `entry_indices` order.
#[requires(compound.entry_indices.iter().all(|index| dictionary.entry_for_index(*index).is_some_and(|entry| entry.has_definition())))]
#[ensures(ret.len() == compound.entry_indices.len())]
#[bityzba::expensive_ensures(ret.iter().zip(&compound.entry_indices).all(|(card, index)| {
    dictionary
        .entry_for_index(*index)
        .is_some_and(|entry| card.word == entry.word)
}))]
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
                true,
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
#[ensures(!ret || (left.byte_end <= right.byte_start && left.char_end <= right.char_start))]
fn adjacent(source: &str, left: &SourceSpan, right: &SourceSpan, barriers: &BarrierIndex) -> bool {
    left.source_id == right.source_id
        // External span mappings must preserve both coordinate orders. Checking
        // before selection keeps invalid spans out of cmavo and ZEI candidates.
        && left.char_end <= right.char_start
        && source
            .get(left.byte_end..right.byte_start)
            .is_some_and(|gap| gap.chars().all(is_compound_separator))
        && !barriers.intersects(left.byte_start, right.byte_end)
}

/// Recognize complete attested groups, with recovery excluded before overlap selection.
#[requires(true)]
#[ensures(true)]
#[expensive_ensures(ret.windows(2).all(|pair| pair[0].span.byte_end <= pair[1].span.byte_start))]
#[expensive_ensures(ret.iter().all(|item| !barriers.iter().any(|barrier| barrier.intersects(item.span.byte_start, item.span.byte_end))))]
#[expensive_ensures(ret.iter().all(|item| item.entry_indices.iter().all(|index| dictionary.entry_for_index(*index).is_some_and(|entry| entry.has_definition()))))]
pub fn recognize_compounds(
    dictionary: &Dictionary<'_>,
    words: &[WordLike],
    source: &str,
    barriers: &[CompoundBarrier],
) -> Vec<ParsedCompoundMatch> {
    let barrier_index = BarrierIndex::from_barriers(barriers);
    let mut result = Vec::new();
    let mut run = CmavoRun::default();
    for word_like in words {
        let word = plain_cmavo(word_like)
            .filter(|word| !barrier_index.intersects(word.span().byte_start, word.span().byte_end));
        if word.is_none()
            || run.words.last().zip(word).is_some_and(|(left, right)| {
                !adjacent(source, left.span(), right.span(), &barrier_index)
            })
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
            if let Some(compound) = attested_zei(dictionary, word_like, source, &barrier_index) {
                result.push(compound);
            }
        }
    }
    append_partition(dictionary, run, &mut result);
    result
}

#[requires(true)]
#[ensures(true)]
#[expensive_ensures((2..=run.words.len().min(dictionary.max_cmavo_sequence_len())).all(|len|
    (0..=run.words.len() - len).all(|start|
!dictionary.has_cmavo_sequence(&run.components[start..start + len])
        || out.iter().any(|selected|
            selected.kind == ParsedCompoundKind::CmavoSequence
                && selected.span.byte_start < run.words[start + len - 1].span().byte_end
                && run.words[start].span().byte_start < selected.span.byte_end
                && (selected.members.len() > len || (selected.members.len() == len
                    && selected.span.byte_start <= run.words[start].span().byte_start))))),
    "every candidate is selected or overlaps a selected candidate with earlier priority")]
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
            if dictionary.has_cmavo_sequence(&run.components[start..end]) {
                accepted.insert(start, end);
            }
        }
    }
    for (start, end) in accepted {
        out.push(cmavo_match(dictionary, &run, start, end));
    }
}

#[requires(start < end && end <= run.words.len())]
#[requires(run.words[start].span().byte_start <= run.words[end - 1].span().byte_end)]
#[requires(run.words[start].span().char_start <= run.words[end - 1].span().char_end)]
#[requires(dictionary.has_cmavo_sequence(&run.components[start..end]))]
#[ensures(ret.kind == ParsedCompoundKind::CmavoSequence)]
#[bityzba::expensive_ensures({
    let ranked = dictionary.lookup_cmavo_sequence(&run.components[start..end]);
    ret.entry_indices == ranked
        && dictionary
            .entry_for_index(ranked[0])
            .is_some_and(|entry| ret.lookup_text == entry.word)
})]
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
        .expect("adjacency preserves byte and character order throughout the run"),
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
        entry_indices: targets,
    })
}

#[requires(matches!(word.as_data(), data!(WordLike::ZeiCompound { .. })))]
#[ensures(ret.as_ref().is_none_or(|item| item.kind == ParsedCompoundKind::Zei))]
#[bityzba::expensive_ensures(ret.as_ref().is_none_or(|item| {
    item.entry_indices == dictionary.exact_definition_indices(&LookupKey::from_stored(&item.lookup_text))
}))]
fn attested_zei(
    dictionary: &Dictionary<'_>,
    word: &WordLike,
    source: &str,
    barriers: &BarrierIndex,
) -> Option<ParsedCompoundMatch> {
    let lookup_text = word_like_lookup_text(word)?;
    let targets = dictionary.exact_definition_indices(&LookupKey::from_stored(&lookup_text));
    if targets.is_empty() {
        return None;
    }
    let mut spans = Vec::new();
    word.source_spans_into(&mut spans);
    let first = *spans.first()?;
    let last = *spans.last()?;
    if barriers.intersects(first.byte_start, last.byte_end)
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
    let barriers = BarrierIndex::default();
    let mut start = index;
    while start > 0
        && plain_cmavo(&words[start - 1])
            .zip(plain_cmavo(&words[start]))
            .is_some_and(|(left, right)| adjacent(source, left.span(), right.span(), &barriers))
    {
        start -= 1;
    }
    let mut end = index + 1;
    while end < words.len()
        && plain_cmavo(&words[end - 1])
            .zip(plain_cmavo(&words[end]))
            .is_some_and(|(left, right)| adjacent(source, left.span(), right.span(), &barriers))
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
            if dictionary.has_cmavo_sequence(&run.components[offset..offset + len]) {
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
    fn zero_width_barrier_partitions_cmavo_runs_before_selection() {
        let selected = matches(
            "ba pu ba pu",
            &[new!(CompoundBarrier {
                byte_start: 5,
                byte_end: 5,
            })],
        );
        assert_eq!(selected.len(), 2);
        assert!(selected.iter().all(|item| item.components == ["ba", "pu"]));
        assert_eq!(selected[0].span.byte_end, 5);
        assert_eq!(selected[1].span.byte_start, 6);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn barrier_inside_a_cmavo_excludes_its_entire_word() {
        let source = "pa moi klama ba pu";
        assert_eq!(matches(source, &[]).len(), 2);
        let selected = matches(
            source,
            &[new!(CompoundBarrier {
                byte_start: 4,
                byte_end: 5,
            })],
        );
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].components, ["ba", "pu"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn barriers_over_zei_members_or_gaps_exclude_the_complete_group() {
        let source = "batke zei uidje ba pu";
        assert_eq!(matches(source, &[]).len(), 2);
        for barrier in [
            new!(CompoundBarrier {
                byte_start: 1,
                byte_end: 2
            }),
            new!(CompoundBarrier {
                byte_start: 5,
                byte_end: 5
            }),
            new!(CompoundBarrier {
                byte_start: 6,
                byte_end: 9
            }),
            new!(CompoundBarrier {
                byte_start: 11,
                byte_end: 12
            }),
        ] {
            let selected = matches(source, &[barrier]);
            assert_eq!(selected.len(), 1, "{barrier:?}");
            assert_eq!(selected[0].kind, ParsedCompoundKind::CmavoSequence);
            assert_eq!(selected[0].components, ["ba", "pu"]);
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn inverted_character_order_is_rejected_before_compound_selection() {
        for source in ["ba pu", "batke zei uidje"] {
            let words =
                segment_words_with_modifiers(source)
                    .unwrap()
                    .into_iter()
                    .map(|word| {
                        jbotci_morphology::map_word_like_spans(word, &|span| {
                            if span.byte_start == 0 {
                                let char_start = span.char_start + source.chars().count();
                                let char_end = span.char_end + source.chars().count();
                                Ok(span.with_data(
                                    data! { char_start: char_start, char_end: char_end },
                                ))
                            } else {
                                Ok(span)
                            }
                        })
                        .unwrap()
                    })
                    .collect::<Vec<_>>();
            let dictionary = jbotci_dictionary_data::english();
            assert!(recognize_compounds(dictionary, &words, source, &[]).is_empty());
            for index in 0..words.len() {
                assert!(cmavo_sequence_containing(dictionary, &words, source, index).is_none());
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn indexed_barriers_preserve_nested_duplicate_and_zero_width_boundaries() {
        let barriers = [
            new!(CompoundBarrier {
                byte_start: 5,
                byte_end: 8
            }),
            new!(CompoundBarrier {
                byte_start: 2,
                byte_end: 6
            }),
            new!(CompoundBarrier {
                byte_start: 2,
                byte_end: 3
            }),
            new!(CompoundBarrier {
                byte_start: 10,
                byte_end: 10
            }),
        ];
        let index = BarrierIndex::from_barriers(&barriers);
        for start in 0..=12 {
            for end in start..=12 {
                assert_eq!(
                    index.intersects(start, end),
                    barriers
                        .iter()
                        .any(|barrier| barrier.intersects(start, end)),
                    "{start}..{end}"
                );
            }
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

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn multilink_zei_attestation_distinguishes_exact_missing_and_empty_entries() {
        let source = "batke zei uidje zei klama";
        let words = segment_words_with_modifiers(source).unwrap();
        let mut entry = *jbotci_dictionary_data::english()
            .lookup_words(&LookupKey::from_stored("batke zei uidje"))
            .next()
            .unwrap();
        entry.word = source;
        let key = jbotci_dictionary::normalize_lookup_query(source);
        let rows = [jbotci_dictionary::WordIndexEntry {
            key: &key,
            targets: &[EntryIndex(0)],
        }];
        for definition in ["an attested multi-link entry", ""] {
            entry.definition = definition;
            let entries = [entry];
            // Deliberately exercise the low-level attestation boundary with an empty
            // definition, which a fully validated dictionary would already reject.
            let dictionary =
                Dictionary::from_static_slices(&entries, &rows, &[], &[], &[], &[], &[], 0);
            let selected = recognize_compounds(&dictionary, &words, source, &[]);
            if definition.is_empty() {
                assert!(selected.is_empty());
            } else {
                assert_eq!(selected.len(), 1);
                assert_eq!(selected[0].kind, ParsedCompoundKind::Zei);
                assert_eq!(selected[0].members.len(), 5);
                assert_eq!(selected[0].lookup_text, source);
                assert_eq!(selected[0].entry_indices, [EntryIndex(0)]);
            }
        }
        let missing = Dictionary::from_static_slices(&[], &[], &[], &[], &[], &[], &[], 0);
        assert!(recognize_compounds(&missing, &words, source, &[]).is_empty());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn shared_key_zei_and_cmavo_sequence_rank_the_higher_score() {
        use jbotci_dictionary::{
            CmavoSequenceIndexEntry, DefinitionId, DictionaryPatternEntry, Keyword, Score,
            WordIndexEntry, WordType, build_owned_indexes,
        };

        static WORSE_ZEI_GLOSS: [Keyword<'static>; 1] = [Keyword {
            word: "worse-zei",
            meaning: None,
        }];
        static BETTER_ZEI_GLOSS: [Keyword<'static>; 1] = [Keyword {
            word: "better-zei",
            meaning: None,
        }];
        static WORSE_CMAVO_GLOSS: [Keyword<'static>; 1] = [Keyword {
            word: "worse-cmavo",
            meaning: None,
        }];
        static BETTER_CMAVO_GLOSS: [Keyword<'static>; 1] = [Keyword {
            word: "better-cmavo",
            meaning: None,
        }];

        let base_zei = *jbotci_dictionary_data::english()
            .lookup_words(&LookupKey::from_stored("batke zei uidje"))
            .next()
            .unwrap();
        let mut worse_zei = base_zei;
        worse_zei.word = "nuhempcr zei klama";
        worse_zei.word_type = WordType::ZeiLujvo;
        worse_zei.definition_id = DefinitionId(101);
        worse_zei.score = Score(-2.0);
        worse_zei.gloss_keywords = &WORSE_ZEI_GLOSS;
        worse_zei.rafsi = &[];

        let mut better_zei = base_zei;
        better_zei.word = "nu'empcr zei klama";
        better_zei.word_type = WordType::ZeiLujvo;
        better_zei.definition_id = DefinitionId(102);
        better_zei.score = Score(1.0);
        better_zei.gloss_keywords = &BETTER_ZEI_GLOSS;
        better_zei.rafsi = &[];

        let base_cmavo = *jbotci_dictionary_data::english()
            .lookup_words(&LookupKey::from_stored("na'e"))
            .next()
            .unwrap();
        let mut worse_cmavo = base_cmavo;
        worse_cmavo.word = "nahe bo";
        worse_cmavo.definition_id = DefinitionId(103);
        worse_cmavo.score = Score(-2.0);
        worse_cmavo.gloss_keywords = &WORSE_CMAVO_GLOSS;
        worse_cmavo.rafsi = &[];

        let mut better_cmavo = base_cmavo;
        better_cmavo.word = "na'e bo";
        better_cmavo.definition_id = DefinitionId(104);
        better_cmavo.score = Score(3.0);
        better_cmavo.gloss_keywords = &BETTER_CMAVO_GLOSS;
        better_cmavo.rafsi = &[];

        let entries = [worse_zei, better_zei, worse_cmavo, better_cmavo];
        let owned = build_owned_indexes(&entries);
        let word_rows: Vec<WordIndexEntry<'_>> = owned
            .word_index
            .iter()
            .map(|row| WordIndexEntry {
                key: row.key.as_str(),
                targets: row.targets.as_slice(),
            })
            .collect();
        let pattern_rows: Vec<DictionaryPatternEntry<'_>> = owned
            .pattern_index
            .iter()
            .map(|row| DictionaryPatternEntry {
                entry_index: row.entry_index,
                word_key: row.word_key.as_str(),
                rafsi_keys: &[],
            })
            .collect();
        let component_rows: Vec<Vec<&str>> = owned
            .cmavo_sequence_index
            .iter()
            .map(|row| row.components.iter().map(String::as_str).collect())
            .collect();
        let cmavo_rows: Vec<CmavoSequenceIndexEntry<'_>> = component_rows
            .iter()
            .zip(&owned.cmavo_sequence_index)
            .map(|(components, row)| {
                CmavoSequenceIndexEntry::from_static_parts(
                    components.as_slice(),
                    row.targets.as_slice(),
                )
            })
            .collect();
        let dictionary = Dictionary::from_static_slices(
            &entries,
            &word_rows,
            &[],
            &pattern_rows,
            &[],
            &[],
            &cmavo_rows,
            owned.max_cmavo_sequence_len,
        );
        assert_eq!(dictionary.validate(), Ok(()));
        assert_eq!(owned.cmavo_sequence_index.len(), 1);
        assert_eq!(
            owned.cmavo_sequence_index[0].targets,
            [EntryIndex(2), EntryIndex(3)]
        );
        assert_eq!(
            dictionary.lookup_cmavo_sequence(&owned.cmavo_sequence_index[0].components),
            [EntryIndex(3), EntryIndex(2)]
        );

        for zei_src in [
            "nu'empcr zei klama",
            "nu\u{02bc}empcr zei klama",
            "nuhempcr zei klama",
        ] {
            let words = segment_words_with_modifiers(zei_src).unwrap();
            let recognized = recognize_compounds(&dictionary, &words, zei_src, &[]);
            assert_eq!(recognized.len(), 1, "{zei_src}");
            assert_eq!(recognized[0].kind, ParsedCompoundKind::Zei, "{zei_src}");
            assert_eq!(
                recognized[0].entry_indices,
                [EntryIndex(1), EntryIndex(0)],
                "{zei_src}"
            );
            assert_eq!(
                recognized[0].lookup_text,
                crate::vlacku::word_like_lookup_text(&words[0]).unwrap(),
                "{zei_src}"
            );
            let cards = compound_cards(&dictionary, &recognized[0]);
            assert_eq!(cards[0].word, "nu'empcr zei klama", "{zei_src}");
            assert_eq!(cards[1].word, "nuhempcr zei klama", "{zei_src}");
            assert_eq!(
                dictionary.exact_definition_indices(&LookupKey::from_stored(zei_src)),
                [EntryIndex(1), EntryIndex(0)],
                "{zei_src}"
            );
        }

        for cmavo_src in ["na'e bo", "na\u{02bc}e bo", "nahe bo", "na\u{02bc}ebo"] {
            let words = segment_words_with_modifiers(cmavo_src).unwrap();
            let recognized = recognize_compounds(&dictionary, &words, cmavo_src, &[]);
            assert_eq!(recognized.len(), 1, "{cmavo_src}");
            assert_eq!(recognized[0].lookup_text, "na'e bo", "{cmavo_src}");
            assert_eq!(
                recognized[0].entry_indices,
                [EntryIndex(3), EntryIndex(2)],
                "{cmavo_src}"
            );
            let cards = compound_cards(&dictionary, &recognized[0]);
            assert_eq!(cards[0].word, "na'e bo", "{cmavo_src}");
            assert_eq!(cards[1].word, "nahe bo", "{cmavo_src}");
            let hover = cmavo_sequence_containing(&dictionary, &words, cmavo_src, 0).unwrap();
            assert_eq!(hover.lookup_text, "na'e bo", "{cmavo_src}");
            assert_eq!(
                compound_cards(&dictionary, &hover)[0].word,
                "na'e bo",
                "{cmavo_src}"
            );
        }
    }
}

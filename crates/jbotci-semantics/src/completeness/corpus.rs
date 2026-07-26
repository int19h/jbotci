//! Manifest of the frozen Phase-B corpus (research repo `FREEZE-PHASE-B.md`).
//!
//! This is the canonical list of witness-document names the inventory's
//! [`super::model::Witness::Corpus`] pointers may reference. The fixture files
//! themselves (`<name>.lojban`, `<name>.frozen.json`) live under
//! `tests/phaseb_corpus/`; the completeness tests iterate this list and load
//! them. Keeping the names here — contract-checked for uniqueness — means a
//! typo in a witness pointer is caught rather than silently unverified.

#[allow(unused_imports)]
use bityzba::{ensures, requires};

/// The 44 frozen corpus documents: 33 battery docs, 4 renderer samples, and seven
/// question witnesses. The first 37 are frozen at
/// oracle commit `28c7d5f`; `ti-mo` was added at oracle commit `7e9c722`
/// (jbotci#620) — the first corpus document exercising a predication whose
/// relation is a bound parameter (`mo`, `relationParameter`) rather than a
/// lexical word. `mi-klama-fia` (source `mi klama fi'a`) was added for the
/// jbotci#620 round-1 review (B3): the first corpus document exercising a
/// predication `placeQuestions` binding (`fi'a`, a place-structure question),
/// now rendered as a first-class `PLACE QUESTIONS` binding record. The five
/// jbotci#622 additions witness typed multi-domain slots (including connective,
/// truth, tense, quantity, argument, and relation kinds), a math-operator
/// question, and an indirect `kau` question with focus and a presupposed answer.
/// All 44 render from oracle commit `d5d1dfd`. See
/// `tests/phaseb_corpus/PROVENANCE.md`.
pub const CORPUS_DOCS: &[&str] = &[
    "b13", "b14", "b15", "b16", "b17", "b18", "b19", "b21", "b22", "b23", "b24", "b25", "b26",
    "b27", "b28", "b29", "b30", "b31", "b32", "b33", "b34", "b35", "b36", "b37", "b38", "b39",
    "b40", "b41", "b42", "b43", "nd1", "nd2", "nd3", "medium-quantified", "numeral-price",
    "paragraph-narrative", "small-mi-klama", "ti-mo", "mi-klama-fia",
    "question-multiple-domains", "question-connective", "question-tense",
    "question-math-operator", "question-indirect-presupposed",
];

/// Whether `doc` is a known frozen corpus document.
#[requires(true)]
#[ensures(ret == CORPUS_DOCS.contains(&doc))]
pub fn is_corpus_doc(doc: &str) -> bool {
    CORPUS_DOCS.contains(&doc)
}

/// True when every corpus name is distinct (no duplicate fixture references).
#[requires(true)]
#[ensures(true)]
pub fn corpus_docs_are_unique() -> bool {
    let mut seen = std::collections::BTreeSet::new();
    CORPUS_DOCS.iter().all(|doc| seen.insert(*doc))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn manifest_is_the_frozen_corpus() {
        // 37 documents frozen at oracle commit 28c7d5f, plus the `ti-mo`
        // relation-question witness (jbotci#620, oracle 7e9c722) and the
        // `mi-klama-fia` place-question witness (jbotci#620 round-1 review B3),
        // plus the five discriminant-rich question witnesses added for
        // jbotci#622.
        assert_eq!(CORPUS_DOCS.len(), 44);
        assert!(corpus_docs_are_unique());
        assert!(is_corpus_doc("small-mi-klama"));
        assert!(is_corpus_doc("ti-mo"));
        assert!(is_corpus_doc("mi-klama-fia"));
        assert!(is_corpus_doc("question-multiple-domains"));
        assert!(is_corpus_doc("question-connective"));
        assert!(is_corpus_doc("question-tense"));
        assert!(is_corpus_doc("question-math-operator"));
        assert!(is_corpus_doc("question-indirect-presupposed"));
        assert!(!is_corpus_doc("b20"));
    }
}

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

/// The 38 frozen corpus documents: 33 battery docs, 4 renderer samples, and one
/// relation-question witness (`ti-mo`). The first 37 are frozen at oracle commit
/// `28c7d5f`; `ti-mo` was added at oracle commit `7e9c722` (jbotci#620) — the
/// first corpus document exercising a predication whose relation is a bound
/// parameter (`mo`, `relationParameter`) rather than a lexical word. See
/// `tests/phaseb_corpus/PROVENANCE.md`.
pub const CORPUS_DOCS: &[&str] = &[
    "b13", "b14", "b15", "b16", "b17", "b18", "b19", "b21", "b22", "b23", "b24", "b25", "b26",
    "b27", "b28", "b29", "b30", "b31", "b32", "b33", "b34", "b35", "b36", "b37", "b38", "b39",
    "b40", "b41", "b42", "b43", "nd1", "nd2", "nd3", "medium-quantified", "numeral-price",
    "paragraph-narrative", "small-mi-klama", "ti-mo",
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
        // relation-question witness added for jbotci#620 (oracle 7e9c722).
        assert_eq!(CORPUS_DOCS.len(), 38);
        assert!(corpus_docs_are_unique());
        assert!(is_corpus_doc("small-mi-klama"));
        assert!(is_corpus_doc("ti-mo"));
        assert!(!is_corpus_doc("b20"));
    }
}

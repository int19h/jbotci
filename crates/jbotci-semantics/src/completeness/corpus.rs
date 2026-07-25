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

/// The 37 frozen corpus documents: 33 battery docs plus 4 renderer samples.
pub const CORPUS_DOCS: &[&str] = &[
    "b13", "b14", "b15", "b16", "b17", "b18", "b19", "b21", "b22", "b23", "b24", "b25", "b26",
    "b27", "b28", "b29", "b30", "b31", "b32", "b33", "b34", "b35", "b36", "b37", "b38", "b39",
    "b40", "b41", "b42", "b43", "nd1", "nd2", "nd3", "medium-quantified", "numeral-price",
    "paragraph-narrative", "small-mi-klama",
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
    fn manifest_is_the_frozen_37() {
        assert_eq!(CORPUS_DOCS.len(), 37);
        assert!(corpus_docs_are_unique());
        assert!(is_corpus_doc("small-mi-klama"));
        assert!(!is_corpus_doc("b20"));
    }
}

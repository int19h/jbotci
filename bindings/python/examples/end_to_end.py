"""Parse, match typed syntax, analyze references, and look up source words."""

from __future__ import annotations

import jbotci
from jbotci import dictionary, morphology
from jbotci.syntax import strict


def main() -> None:
    """Run the public typed pipeline without serialization."""
    analyzed = jbotci.analyze("mi tavla do .i ri cadzu", source_id="end-to-end")

    match analyzed.parse_tree:
        case strict.TextSyntaxRegularText(regular_text=regular_text):
            assert isinstance(regular_text, strict.RegularTextSyntax)
        case unexpected:
            raise AssertionError(f"unexpected text variant: {type(unexpected).__name__}")

    assignments = analyzed.reference_analysis.place_analysis.assignments()
    edges = analyzed.reference_analysis.discourse_references.edges()
    assert assignments
    assert edges
    for assignment in assignments:
        sumti = analyzed.reference_analysis.syntax_index.node(
            assignment.sumti.raw_id
        )
        assert isinstance(sumti, strict.SumtiSyntax)

    source_words: list[str] = []
    for word_like in analyzed.words:
        match word_like:
            case morphology.PlainWord(word=word):
                source_words.append(word.canonical_phonemes)
            case unexpected_word:
                raise AssertionError(
                    "unexpected morphology variant: "
                    f"{type(unexpected_word).__name__}"
                )
    entries = tuple(dictionary.english.lookup_word(word) for word in source_words)
    assert all(entry is not None for entry in entries)
    assert any(entry is not None and entry.word == "tavla" for entry in entries)

    print(
        f"{type(analyzed.parse_tree).__name__}: "
        f"{len(assignments)} assignments, {len(edges)} reference edges, "
        f"{len(entries)} dictionary lookups"
    )


if __name__ == "__main__":
    main()

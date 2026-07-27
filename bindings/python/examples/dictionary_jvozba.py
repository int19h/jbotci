"""Classify a word, query the embedded dictionary, and build a lujvo."""

from __future__ import annotations

from jbotci import dictionary, jvozba, morphology


def main() -> None:
    """Exercise dictionary reuse by the morphology and jvozba APIs."""
    analysis = morphology.analyze_valsi("tavla")
    assert analysis.result.is_valid
    match analysis.result.word:
        case morphology.PlainWord(word=word):
            dictionary_key = word.canonical_phonemes
        case unexpected:
            raise AssertionError(
                f"unexpected classification value: {type(unexpected).__name__}"
            )
    entry = dictionary.english.lookup_word(dictionary_key)
    assert entry is not None
    assert entry.word == "tavla"

    built = jvozba.build(
        (jvozba.Word("lojbo"), jvozba.Word("bangu")),
        dictionary=dictionary.english,
    )
    assert built.word == "jbobau"
    print(f"{entry.word}: {built.word}")


if __name__ == "__main__":
    main()

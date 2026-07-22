"""Strict-type-check smoke coverage for packaged public declarations."""

from jbotci import Sample, SampleMode, dictionary, sample_mode, semantics, smoke


def sample_text(value: str | None) -> tuple[str, str | None]:
    """Exercise class, tuple, optional, and namespace annotations."""
    sample = Sample(value or "")
    mode: SampleMode = sample_mode(advanced=value is not None)
    assert semantics.references.__all__ == ()
    return (f"{smoke()}:{mode.value}", sample.value if value is not None else None)


def dictionary_values(word: str) -> tuple[str | None, int | None, str | None]:
    """Exercise typed dictionary records, enums, tuples, and optional results."""
    entry = dictionary.english.lookup_word(word)
    if entry is None:
        return (None, None, None)
    index = dictionary.english.entry_index_for_entry(entry)
    first_rafsi = entry.rafsi[0].value if entry.rafsi else None
    assert isinstance(entry.word_type, dictionary.WordType)
    _ = entry.word_type.is_gismu_like() or entry.word_type.is_lujvo_like()
    return (entry.word, index.value if index is not None else None, first_rafsi)


def dictionary_batch(words: tuple[str, ...]) -> tuple[str | None, ...]:
    """Exercise the strict batch and bounded-result declarations."""
    return dictionary.english.first_gloss_keywords_for_words(words)

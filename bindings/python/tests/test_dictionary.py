from __future__ import annotations

import enum
import gc
import inspect
import math
import subprocess
import sys
from collections.abc import Sequence

import pytest

import jbotci._native as native
import jbotci.dictionary as dictionary


def required_entry(word: str) -> dictionary.DictionaryEntry:
    """Return a required snapshot witness with a useful assertion failure."""
    entry = dictionary.english.lookup_word(word)
    assert entry is not None
    return entry


def entry_index(word: str) -> dictionary.EntryIndex:
    """Return the stable typed index of a required snapshot witness."""
    index = dictionary.english.entry_index_for_entry(required_entry(word))
    assert index is not None
    return index


def required_index_for_entry(
    entry: dictionary.DictionaryEntry,
) -> dictionary.EntryIndex:
    """Return the typed index of an entry known to belong to English."""
    index = dictionary.english.entry_index_for_entry(entry)
    assert index is not None
    return index


def test_english_objects_have_stable_identity_and_metadata() -> None:
    assert dictionary.english is native._dictionary_english
    assert dictionary.english_metadata is native._dictionary_english_metadata
    assert len(dictionary.english) == 17_415
    assert dictionary.english_metadata.entry_count == len(dictionary.english)
    assert dictionary.english_metadata.language_tag == "en"
    assert dictionary.english_metadata.language_realname == "English"
    assert dictionary.english_metadata.format == "json"
    assert dictionary.english_metadata.filename == "dictionary-en.json"
    assert dictionary.english_metadata.lensisku_created_at == (
        "2026-05-23T00:00:42.298977Z"
    )
    assert dictionary.english_metadata.sha256 == (
        "515c3fbf56f65a904cdcf28a6ddd411768363c322f4d33d3ac5bc74882314187"
    )
    assert repr(dictionary.english) == "jbotci.dictionary.english"
    assert dictionary.Dictionary.__name__ == "Dictionary"
    assert dictionary.WordType.__name__ == "WordType"
    assert not hasattr(native, "Dictionary")
    assert all(hasattr(dictionary, name) for name in dictionary.__all__)


def test_import_does_not_materialize_python_entry_objects() -> None:
    assert isinstance(dictionary.english.entries, dictionary.DictionaryEntries)
    assert not isinstance(dictionary.english.entries, tuple)
    code = """
import gc
import jbotci.dictionary as dictionary
assert not any(isinstance(value, dictionary.DictionaryEntry) for value in gc.get_objects())
assert not isinstance(dictionary.english.entries, tuple)
"""
    subprocess.run([sys.executable, "-c", code], check=True)


def test_source_order_sequence_supports_iteration_indices_and_slices() -> None:
    entries = dictionary.english.entries
    assert not isinstance(dictionary.english, Sequence)
    assert not isinstance(entries, Sequence)
    assert not hasattr(dictionary.english, "count")
    assert not hasattr(dictionary.english, "index")
    assert not hasattr(entries, "count")
    assert not hasattr(entries, "index")
    assert len(entries) == 17_415
    assert entries[0].word == dictionary.english[0].word
    assert entries[dictionary.EntryIndex(0)].word == entries[0].word
    assert entries[-1].word == entries[len(entries) - 1].word
    assert tuple(entry.word for entry in entries[:3]) == tuple(
        entry.word for entry in dictionary.english[:3]
    )
    assert tuple(entry.word for entry in entries[4:0:-2]) == (
        entries[4].word,
        entries[2].word,
    )
    assert tuple(entry.word for entry in entries[1::sys.maxsize]) == (
        entries[1].word,
    )
    assert next(iter(dictionary.english)).word == entries[0].word
    assert sum(1 for _ in dictionary.english) == len(entries)
    with pytest.raises(IndexError):
        _ = entries[-len(entries) - 1]
    with pytest.raises(IndexError):
        _ = entries[len(entries)]
    with pytest.raises(IndexError):
        _ = entries[10**100]
    with pytest.raises(TypeError):
        _ = entries["0"]  # type: ignore[call-overload]


def test_typed_indices_do_not_gain_contextual_invariants() -> None:
    assert dictionary.DefinitionId(0).value == 0
    assert int(dictionary.DefinitionId(0)) == 0
    assert dictionary.EntryIndex(10**8).value == 10**8
    assert dictionary.english.entry_for_index(dictionary.EntryIndex(10**8)) is None
    with pytest.raises(OverflowError):
        dictionary.EntryIndex(-1)

    nan = dictionary.Score(math.nan)
    infinity = dictionary.Score(math.inf)
    assert math.isnan(nan.value)
    assert infinity.value == math.inf
    assert dictionary.Score(1.0) < dictionary.Score(2.0)
    assert nan != nan


def test_exact_word_lookup_preserves_collision_and_normalization() -> None:
    assert [entry.word for entry in dictionary.english.lookup_words("internet")] == [
        "INternet",
        "internet",
    ]
    assert required_entry(" . BÁNGU . ").word == "bangu"
    assert required_entry("  daʼoi  ").word == "da'oi"
    assert dictionary.normalize_lookup_query(" .Án,iis. ") == "aniis"
    assert dictionary.normalize_lookup_query("lo  brodá") == "lo broda"
    assert dictionary.normalize_pattern_lookup_key("dahoi") == "da'oi"
    assert dictionary.english.lookup_word("definitely missing") is None


def test_prefix_lookup_is_normalized_ordered_and_handles_empty_prefix() -> None:
    matches = dictionary.english.entries_by_word_prefix("BÁ")
    normalized = tuple(dictionary.normalize_lookup_query(entry.word) for entry in matches)
    assert normalized == tuple(sorted(normalized))
    assert normalized
    assert all(word.startswith("ba") for word in normalized)
    ordered_keys = tuple(
        (
            dictionary.normalize_lookup_query(entry.word),
            required_index_for_entry(entry).value,
        )
        for entry in matches
    )
    assert ordered_keys == tuple(sorted(ordered_keys))

    all_entries = dictionary.english.entries_by_word_prefix("")
    assert isinstance(all_entries, tuple)
    assert len(all_entries) == len(dictionary.english)
    assert all_entries[0].word == dictionary.english[0].word
    assert all_entries[-1].word == dictionary.english[-1].word


def test_rafsi_queries_preserve_provenance_and_helpers_are_typed() -> None:
    listed = dictionary.english.lookup_rafsi("bau")
    assert any(
        match.entry.word == "bangu" and match.source is dictionary.RafsiSource.LISTED
        for match in listed
    )
    short = dictionary.english.lookup_rafsi("banl")
    assert any(
        match.entry.word == "banli"
        and match.source is dictionary.RafsiSource.UNIVERSAL_SHORT
        for match in short
    )
    long = dictionary.english.lookup_rafsi("banli")
    assert any(
        match.entry.word == "banli"
        and match.source is dictionary.RafsiSource.UNIVERSAL_LONG
        for match in long
    )

    forms = dictionary.universal_gismu_rafsi_forms("banli")
    assert tuple((form.value, source) for form, source in forms) == (
        ("banl", dictionary.RafsiSource.UNIVERSAL_SHORT),
        ("banli", dictionary.RafsiSource.UNIVERSAL_LONG),
    )
    assert dictionary.universal_gismu_rafsi_forms("broda") == (
        (dictionary.Rafsi("broda"), dictionary.RafsiSource.UNIVERSAL_LONG),
    )


def test_selmaho_and_batch_gloss_queries_use_real_indexes() -> None:
    coi_words = tuple(entry.word for entry in dictionary.english.entries_by_selmaho("COI"))
    assert "coi" in coi_words
    assert required_entry("coi").selmaho == dictionary.RawSelmaho("COI")
    assert dictionary.english.entries_by_selmaho("coi") == ()

    assert dictionary.english.first_gloss_keywords_for_words(
        ("bangu", "missing", "internet")
    ) == ("language", None, "Internet")
    assert dictionary.first_gloss_keywords_for_words(["klama", "coi"]) == (
        "come",
        "greetings",
    )
    with pytest.raises(TypeError, match="bare string"):
        dictionary.english.first_gloss_keywords_for_words("klama")


def test_entry_records_expose_optional_and_repeated_typed_values() -> None:
    a = required_entry("a")
    assert a.word_type is dictionary.WordType.CMAVO
    assert not a.word_type.is_gismu_like()
    assert not a.word_type.is_lujvo_like()
    assert a.definition_id == dictionary.DefinitionId(1339)
    assert len(a.gloss_keywords) == 3
    assert a.gloss_keywords[0] == dictionary.Keyword("and/or", "inclusive or")
    assert a.gloss_keywords[2].meaning is None
    assert a.place_keywords == ()
    assert a.etymology is None
    assert a.jargon is None
    assert a.user == dictionary.DictionaryUser("officialdata", "Official Data")

    aasna = required_entry("a'asna")
    assert len(aasna.gloss_keywords) == 4
    assert len(aasna.place_keywords) == 2
    assert aasna.jargon == "linguistics"
    adzau = required_entry("adzau")
    assert adzau.etymology is not None
    assert adzau.jargon == "Internet"
    assert required_entry("bafygau").user.realname is None
    assert [rafsi.value for rafsi in required_entry("bangu").rafsi] == ["ban", "bau"]
    assert required_entry("bangu").word_type.is_gismu_like()
    assert required_entry("jbobau").word_type.is_lujvo_like()


def test_word_type_predicates_delegate_through_exact_native_enum_conversion() -> None:
    assert dictionary.WordType.GISMU.is_gismu_like()
    assert dictionary.WordType.EXPERIMENTAL_GISMU.is_gismu_like()
    assert not dictionary.WordType.CMAVO.is_gismu_like()
    assert dictionary.WordType.LUJVO.is_lujvo_like()
    assert dictionary.WordType.ZEI_LUJVO.is_lujvo_like()
    assert dictionary.WordType.OBSOLETE_ZEI_LUJVO.is_lujvo_like()
    assert not dictionary.WordType.GISMU.is_lujvo_like()
    with pytest.raises(TypeError):
        native._dictionary_word_type_is_gismu_like("gismu")  # type: ignore[arg-type]


def test_sound_records_expose_exact_ipa_and_typed_segments_without_search() -> None:
    sounds = dictionary.english.sound_index
    assert isinstance(sounds, tuple)
    assert tuple(sound.entry_index.value for sound in sounds) == tuple(
        sorted(sound.entry_index.value for sound in sounds)
    )
    by_index = {sound.entry_index: sound for sound in sounds}
    klama = by_index[entry_index("klama")]
    coi = by_index[entry_index("coi")]
    assert klama.ipa == "ˈkla.ma"
    assert klama.token_sequence.segment_count() == 5
    assert len(klama.token_sequence.segments) == 5
    assert all(segment.symbol for segment in klama.token_sequence.segments)
    assert coi.ipa == "ʃoj"
    assert coi.token_sequence.segment_count() == 3
    assert not hasattr(dictionary.english, "search_sound")
    assert not hasattr(dictionary.english, "lookup_sound")


def test_lujvo_decomposition_records_preserve_segments_and_source_words() -> None:
    decompositions = dictionary.english.lujvo_index
    assert isinstance(decompositions, tuple)
    jbobau = dictionary.english.lujvo_decomposition_for_entry_index(entry_index("jbobau"))
    assert jbobau is not None
    assert {value.entry_index: value for value in decompositions}[
        entry_index("jbobau")
    ] == jbobau
    assert tuple(
        (segment.kind, segment.surface, segment.source_word)
        for segment in jbobau.segments
    ) == (
        (dictionary.DictionaryLujvoSegmentKind.RAFSI, "jbó", "lojbo"),
        (dictionary.DictionaryLujvoSegmentKind.RAFSI, "baŭ", "bangu"),
    )
    assert jbobau.source_words == ("lojbo", "bangu")

    ciartai = dictionary.english.lujvo_decomposition_for_entry_index(
        entry_index("ci'artai")
    )
    assert ciartai is not None
    assert tuple(
        (segment.kind, segment.surface, segment.source_word)
        for segment in ciartai.segments
    ) == (
        (dictionary.DictionaryLujvoSegmentKind.RAFSI, "ci'á", "ciska"),
        (dictionary.DictionaryLujvoSegmentKind.HYPHEN, "r", None),
        (dictionary.DictionaryLujvoSegmentKind.RAFSI, "taĭ", "tarmi"),
    )
    assert ciartai.source_words == ("ciska", "tarmi")


def test_pattern_records_match_source_order_and_normalized_keys() -> None:
    patterns = dictionary.english.pattern_index
    assert len(patterns) == len(dictionary.english)
    klama_index = entry_index("klama")
    klama = patterns[klama_index.value]
    assert klama.entry_index == klama_index
    assert klama.word_key == "klama"
    assert "kla" in klama.rafsi_keys
    assert "klam" in klama.rafsi_keys
    assert "klama" in klama.rafsi_keys


def test_child_records_retain_owner_after_parent_and_results_are_dropped() -> None:
    def retain_children() -> tuple[
        dictionary.DictionaryEntry,
        dictionary.Keyword,
        dictionary.Rafsi,
        dictionary.DictionaryUser,
        dictionary.IpaTokenSequenceView,
        dictionary.DictionaryLujvoSegment,
    ]:
        owner = dictionary.english
        entry = owner.lookup_word("bangu")
        assert entry is not None
        sound = {value.entry_index: value for value in owner.sound_index}[
            entry_index("klama")
        ]
        decomposition = owner.lujvo_decomposition_for_entry_index(entry_index("jbobau"))
        assert decomposition is not None
        return (
            entry,
            entry.gloss_keywords[0],
            entry.rafsi[0],
            entry.user,
            sound.token_sequence,
            decomposition.segments[0],
        )

    entry, keyword, rafsi, user, sequence, segment = retain_children()
    gc.collect()
    assert entry.word == "bangu"
    assert keyword.word == "language"
    assert rafsi.value == "ban"
    assert user.username == "officialdata"
    assert sequence.segment_count() == 5
    assert segment.source_word == "lojbo"


def test_public_data_classes_are_frozen_final_and_in_public_module() -> None:
    public_data_classes = (
        dictionary.DefinitionId,
        dictionary.Score,
        dictionary.EntryIndex,
        dictionary.Keyword,
        dictionary.Rafsi,
        dictionary.RawSelmaho,
        dictionary.DictionaryUser,
        dictionary.Dictionary,
        dictionary.DictionaryEntries,
        dictionary.DictionaryEntry,
        dictionary.RafsiMatch,
        dictionary.DictionarySoundEntry,
        dictionary.IpaTokenSequenceView,
        dictionary.IpaSegmentId,
        dictionary.DictionaryLujvoEntry,
        dictionary.DictionaryLujvoSegment,
        dictionary.DictionaryPatternEntry,
        dictionary.DictionarySnapshotMetadata,
        dictionary.InvalidEntryValidationDetail,
        dictionary.WordIndexMismatchValidationDetail,
        dictionary.RafsiIndexMismatchValidationDetail,
        dictionary.SelmahoIndexMismatchValidationDetail,
        dictionary.PatternIndexMismatchValidationDetail,
        dictionary.InvalidSoundIndexEntryValidationDetail,
        dictionary.InvalidLujvoIndexEntryValidationDetail,
    )
    for data_class in public_data_classes:
        assert data_class.__module__ == "jbotci.dictionary"
        with pytest.raises(TypeError):
            type("Derived", (data_class,), {})

    returned_only_classes = (
        dictionary.Dictionary,
        dictionary.DictionaryEntries,
        dictionary.DictionaryEntry,
        dictionary.RafsiMatch,
        dictionary.DictionarySoundEntry,
        dictionary.IpaTokenSequenceView,
        dictionary.IpaSegmentId,
        dictionary.DictionaryLujvoEntry,
        dictionary.DictionaryLujvoSegment,
        dictionary.DictionaryPatternEntry,
        dictionary.DictionarySnapshotMetadata,
        dictionary.InvalidEntryValidationDetail,
        dictionary.WordIndexMismatchValidationDetail,
        dictionary.RafsiIndexMismatchValidationDetail,
        dictionary.SelmahoIndexMismatchValidationDetail,
        dictionary.PatternIndexMismatchValidationDetail,
        dictionary.InvalidSoundIndexEntryValidationDetail,
        dictionary.InvalidLujvoIndexEntryValidationDetail,
    )
    for returned_only_class in returned_only_classes:
        with pytest.raises(TypeError):
            returned_only_class()  # type: ignore[call-arg]

    first_sound = dictionary.english.sound_index[0]
    first_pattern = dictionary.english.pattern_index[0]
    values: tuple[object, ...] = (
        dictionary.DefinitionId(0),
        dictionary.Score(0.0),
        dictionary.EntryIndex(0),
        dictionary.Keyword("", None),
        dictionary.Rafsi(""),
        dictionary.RawSelmaho(""),
        dictionary.DictionaryUser("", None),
        dictionary.english,
        dictionary.english.entries,
        dictionary.english_metadata,
        required_entry("a"),
        dictionary.english.lookup_rafsi("bau")[0],
        first_sound,
        first_sound.token_sequence,
        first_sound.token_sequence.segments[0],
        first_pattern,
    )
    for value in values:
        assert type(value).__module__ == "jbotci.dictionary"
        with pytest.raises((AttributeError, TypeError)):
            setattr(value, "unexpected", True)

    enum_types = (
        dictionary.WordType,
        dictionary.RafsiSource,
        dictionary.DictionaryLujvoSegmentKind,
    )
    for enum_type in enum_types:
        assert issubclass(enum_type, str)
        assert issubclass(enum_type, enum.Enum)
        assert enum_type.__module__ == "jbotci.dictionary"
        with pytest.raises(TypeError):
            type("DerivedEnum", (enum_type,), {})


def test_public_dictionary_api_has_complete_runtime_docstrings() -> None:
    documented_classes = (
        dictionary.DefinitionId,
        dictionary.Score,
        dictionary.EntryIndex,
        dictionary.Keyword,
        dictionary.Rafsi,
        dictionary.RawSelmaho,
        dictionary.DictionaryUser,
        dictionary.Dictionary,
        dictionary.DictionaryEntries,
        dictionary.DictionaryEntry,
        dictionary.RafsiMatch,
        dictionary.DictionarySoundEntry,
        dictionary.IpaTokenSequenceView,
        dictionary.IpaSegmentId,
        dictionary.DictionaryLujvoEntry,
        dictionary.DictionaryLujvoSegment,
        dictionary.DictionaryPatternEntry,
        dictionary.DictionarySnapshotMetadata,
        dictionary.InvalidEntryValidationDetail,
        dictionary.WordIndexMismatchValidationDetail,
        dictionary.RafsiIndexMismatchValidationDetail,
        dictionary.SelmahoIndexMismatchValidationDetail,
        dictionary.PatternIndexMismatchValidationDetail,
        dictionary.InvalidSoundIndexEntryValidationDetail,
        dictionary.InvalidLujvoIndexEntryValidationDetail,
        dictionary.WordType,
        dictionary.RafsiSource,
        dictionary.DictionaryLujvoSegmentKind,
        dictionary.DictionaryValidationError,
    )
    for documented_class in documented_classes:
        assert inspect.getdoc(documented_class), documented_class.__name__
        for member_name, member in vars(documented_class).items():
            if member_name.startswith("_"):
                continue
            if inspect.isroutine(member) or inspect.isdatadescriptor(member):
                assert inspect.getdoc(member), (
                    f"{documented_class.__name__}.{member_name}"
                )

    for export_name in dictionary.__all__:
        exported = getattr(dictionary, export_name)
        if inspect.isroutine(exported) or isinstance(exported, type):
            assert inspect.getdoc(exported), export_name


def test_embedded_dictionary_validates() -> None:
    assert dictionary.english.validate() is None

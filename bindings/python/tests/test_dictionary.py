from __future__ import annotations

import ast
import enum
import gc
import inspect
import math
import operator
import subprocess
import sys
import types
from collections.abc import Sequence
from pathlib import Path

import pytest

import jbotci._native as native
import jbotci.dictionary as dictionary
import jbotci.morphology as morphology

PACKAGE_ROOT = Path(__file__).resolve().parents[1]
EXPECTED_DICTIONARY_EXPORTS = (
    "DefinitionId",
    "Dictionary",
    "DictionaryEntries",
    "DictionaryEntry",
    "DictionaryLujvoEntry",
    "DictionaryLujvoSegment",
    "DictionaryLujvoSegmentKind",
    "DictionaryPatternEntry",
    "DictionarySnapshotMetadata",
    "DictionarySoundEntry",
    "DictionaryUser",
    "DictionaryValidationDetail",
    "DictionaryValidationError",
    "EntryIndex",
    "FreeRafsiAvailability",
    "InvalidEntryValidationDetail",
    "InvalidLujvoIndexEntryValidationDetail",
    "InvalidSoundIndexEntryValidationDetail",
    "IpaSegmentId",
    "IpaTokenSequenceView",
    "Keyword",
    "PatternIndexMismatchValidationDetail",
    "PronunciationTargetId",
    "PronunciationTargetSequenceView",
    "Rafsi",
    "RafsiAvailability",
    "RafsiCandidate",
    "RafsiClaimKind",
    "RafsiIndexMismatchValidationDetail",
    "RafsiMatch",
    "RafsiSource",
    "RawSelmaho",
    "Score",
    "SelmahoIndexMismatchValidationDetail",
    "TakenRafsiAvailability",
    "WordIndexMismatchValidationDetail",
    "WordType",
    "english",
    "english_metadata",
    "first_gloss_keywords_for_words",
    "normalize_lookup_query",
    "normalize_pattern_lookup_key",
    "rafsi_claimants",
    "short_rafsi_candidates",
    "universal_gismu_rafsi_forms",
)


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


def required_sound(word: str) -> dictionary.DictionarySoundEntry:
    """Return the generated sound record for a required public word witness."""
    owner = dictionary.english
    entry = owner.lookup_word(word)
    assert entry is not None
    index = owner.entry_index_for_entry(entry)
    assert index is not None
    sound = next((value for value in owner.sound_index if value.entry_index == index), None)
    assert sound is not None
    return sound


def test_english_objects_have_stable_identity_and_metadata() -> None:
    assert dictionary.english is native._dictionary_english
    assert dictionary.english_metadata is native._dictionary_english_metadata
    assert (
        dictionary.PronunciationTargetId
        is native._dictionary_PronunciationTargetId
    )
    assert (
        dictionary.PronunciationTargetSequenceView
        is native._dictionary_PronunciationTargetSequenceView
    )
    assert len(dictionary.english) == 17_536
    assert dictionary.english_metadata.entry_count == len(dictionary.english)
    assert dictionary.english_metadata.language_tag == "en"
    assert dictionary.english_metadata.language_realname == "English"
    assert dictionary.english_metadata.format == "json"
    assert dictionary.english_metadata.filename == "dictionary-en.json"
    assert dictionary.english_metadata.lensisku_created_at == (
        "2026-07-27T07:10:51.776063Z"
    )
    assert dictionary.english_metadata.sha256 == (
        "ba268ad701f8f44656ea4b17a1fd9539cfc1a3c523d0bdf581a44e3e93bb412f"
    )
    assert repr(dictionary.english) == "jbotci.dictionary.english"
    assert dictionary.Dictionary.__name__ == "Dictionary"
    assert dictionary.WordType.__name__ == "WordType"
    assert not hasattr(native, "Dictionary")
    assert dictionary.__all__ == EXPECTED_DICTIONARY_EXPORTS
    assert all(hasattr(dictionary, name) for name in EXPECTED_DICTIONARY_EXPORTS)


def test_import_keeps_entry_sequence_lazy_and_has_no_tracked_entry_wrappers() -> None:
    assert isinstance(dictionary.english.entries, dictionary.DictionaryEntries)
    assert not isinstance(dictionary.english.entries, tuple)
    # `gc.get_objects()` only observes GC-tracked wrappers. Keep this as the
    # narrow regression signal it is; the sequence-shape checks prove laziness.
    code = """
import gc
import jbotci.dictionary as dictionary
import jbotci.morphology as morphology
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
    assert len(entries) == 17_536
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
    all_ordered_keys = tuple(
        (
            dictionary.normalize_lookup_query(entry.word),
            required_index_for_entry(entry).value,
        )
        for entry in all_entries
    )
    # Reconstruct the Rust word index independently from the source-order
    # sequence: normalized keys sort globally, and collision targets retain
    # their source indexes in ascending order.
    expected_word_index_order = tuple(
        sorted(
            (
                dictionary.normalize_lookup_query(entry.word),
                source_index,
            )
            for source_index, entry in enumerate(dictionary.english)
        )
    )
    assert all_ordered_keys == expected_word_index_order
    assert sorted(index for _, index in all_ordered_keys) == list(
        range(len(dictionary.english))
    )


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


def test_short_rafsi_candidates_report_dictionary_claims() -> None:
    candidates = dictionary.short_rafsi_candidates("sakli")
    assert tuple(candidate.form for candidate in candidates) == (
        "kli",
        "sa'i",
        "sai",
        "sak",
        "sal",
        "ska",
    )
    assert candidates == dictionary.english.short_rafsi_candidates("sakli")
    assert candidates[0].shape is morphology.ShortRafsiShape.CCV

    # Every rafsi `sakli` could claim is already held, `sal` by `sakli` itself.
    for candidate in candidates:
        match candidate.availability:
            case dictionary.TakenRafsiAvailability(kind, words):
                assert kind is dictionary.RafsiClaimKind.OFFICIAL
                assert words
            case _:  # pragma: no cover - structural match must succeed
                pytest.fail(f"{candidate.form} should be taken")
    assert candidates[4].availability == dictionary.TakenRafsiAvailability(
        dictionary.RafsiClaimKind.OFFICIAL, ("sakli",)
    )

    # Cmavo hold rafsi too, so `kam` is not available to an invented `kacma`.
    kacma = dictionary.short_rafsi_candidates("kacma")
    assert tuple(
        (candidate.form, candidate.availability) for candidate in kacma
    ) == tuple(
        (
            form,
            dictionary.TakenRafsiAvailability(
                dictionary.RafsiClaimKind.OFFICIAL, (holder,)
            ),
        )
        for form, holder in (
            ("cma", "cmalu"),
            ("ka'a", "katna"),
            ("kac", "kancu"),
            ("kam", "ka"),
        )
    )

    assert dictionary.short_rafsi_candidates("coi") == ()
    assert any(
        candidate.availability == dictionary.FreeRafsiAvailability()
        for candidate in dictionary.short_rafsi_candidates("nanpe")
    )

    assert dictionary.rafsi_claimants("sal") == (
        ("sakli", dictionary.WordType.GISMU),
    )
    assert dictionary.rafsi_claimants("kam") == (("ka", dictionary.WordType.CMAVO),)
    assert dictionary.rafsi_claimants("zzz") == ()
    with pytest.raises(TypeError):
        dictionary.TakenRafsiAvailability(
            dictionary.RafsiClaimKind.OFFICIAL, ()
        )
    # Candidates only ever arrive from the validated Rust derivation, so there
    # is no Python path that could construct an ill-formed one.
    with pytest.raises(TypeError):
        dictionary.RafsiCandidate()


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
    # Only experimental and obsolete types make provisional rafsi claims.
    for word_type in dictionary.WordType:
        assert word_type.rafsi_claim_kind() is (
            dictionary.RafsiClaimKind.EXPERIMENTAL
            if word_type.startswith(("experimental ", "obsolete "))
            else dictionary.RafsiClaimKind.OFFICIAL
        )
    with pytest.raises(TypeError):
        native._dictionary_word_type_is_gismu_like("gismu")  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        native._dictionary_word_type_rafsi_claim_kind("gismu")  # type: ignore[arg-type]


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


def test_sound_records_preserve_pronunciation_targets_and_rhotic_realizations() -> None:
    sounds = {sound.entry_index: sound for sound in dictionary.english.sound_index}
    klama = sounds[entry_index("klama")]
    targets = klama.pronunciation_targets
    assert targets.target_count() == len(targets) == len(targets.targets)
    assert targets.target_count() == klama.token_sequence.segment_count()
    singleton = next(target for target in targets.targets if target.realization_count == 1)
    realization = singleton.realization(0)
    assert isinstance(realization, dictionary.IpaSegmentId)
    assert singleton.realizations == (realization,)

    # Keep this public lookup witness literal. The embedded Rust parity test
    # independently derives all expected target and realization IDs from the
    # Rust `prami` sound record, so this test never copies inventory numbers.
    prami_entry = required_entry("prami")
    prami = sounds[required_index_for_entry(prami_entry)]
    rhotics = tuple(
        target
        for target in prami.pronunciation_targets.targets
        if target.realization_count > 1
    )
    assert len(rhotics) == 1
    rhotic = rhotics[0]
    assert rhotic.realizations == tuple(
        rhotic.realization(index) for index in range(rhotic.realization_count)
    )
    assert all(isinstance(value, dictionary.IpaSegmentId) for value in rhotic.realizations)
    assert len(set(rhotic.realizations)) == rhotic.realization_count

    assert rhotic.realization(rhotic.realization_count) is None
    assert rhotic.realization(rhotic.realization_count + 100) is None
    assert rhotic.realization(10**100) is None
    assert rhotic.realization(-1) is None
    assert rhotic.realization(-(10**100)) is None
    assert rhotic.realizations[-1] == rhotic.realizations[rhotic.realization_count - 1]
    with pytest.raises(TypeError):
        rhotic.realization(0.0)  # type: ignore[arg-type]


def test_pronunciation_target_protocols_follow_existing_id_and_view_policy() -> None:
    sounds = {sound.entry_index: sound for sound in dictionary.english.sound_index}
    prami = sounds[entry_index("prami")]
    first_view = prami.pronunciation_targets
    second_view = prami.pronunciation_targets
    other_view = sounds[entry_index("klama")].pronunciation_targets
    assert first_view == second_view
    assert first_view != other_view
    assert repr(first_view) == (
        "jbotci.dictionary.PronunciationTargetSequenceView("
        f"target_count={first_view.target_count()}, "
        f"self_similarity={first_view.self_similarity!r})"
    )
    with pytest.raises(TypeError):
        hash(first_view)
    with pytest.raises(TypeError):
        _ = first_view < second_view  # type: ignore[operator]

    first_targets = {target.value: target for target in first_view.targets}
    assert len(first_targets) >= 2
    lower_value, higher_value = sorted(first_targets)[:2]
    lower = first_targets[lower_value]
    higher = first_targets[higher_value]
    equal_lower = next(
        target for target in second_view.targets if target.value == lower_value
    )

    assert lower == equal_lower
    assert lower != higher
    assert len({lower, equal_lower, higher}) == 2
    assert hash(lower) == hash(equal_lower)
    snapshot_targets = {
        target.value: target
        for sound in dictionary.english.sound_index
        for target in sound.pronunciation_targets.targets
    }
    assert len(snapshot_targets) > 1
    assert len({hash(target) for target in snapshot_targets.values()}) > 1
    assert int(lower) == lower.value
    assert operator.index(lower) == lower.value
    assert repr(lower) == (
        "jbotci.dictionary.PronunciationTargetId("
        f"value={lower.value}, "
        f"realization_count={lower.realization_count})"
    )

    assert lower < higher
    assert lower <= higher
    assert higher > lower
    assert higher >= lower
    assert not higher < lower
    assert not higher <= lower
    assert not lower > higher
    assert not lower >= higher
    assert not lower < equal_lower
    assert lower <= equal_lower
    assert not lower > equal_lower
    assert lower >= equal_lower
    assert tuple(sorted(first_view.targets)) == tuple(
        sorted(first_view.targets, key=lambda target: target.value)
    )


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


def test_pronunciation_target_sequence_survives_gc_independently() -> None:
    expected_count = required_sound("klama").token_sequence.segment_count()

    def retain_sequence() -> dictionary.PronunciationTargetSequenceView:
        sound = required_sound("klama")
        return sound.pronunciation_targets

    sequence = retain_sequence()
    gc.collect()
    assert sequence.target_count() == expected_count
    assert len(sequence.targets) == expected_count


def test_pronunciation_target_id_survives_gc_independently() -> None:
    def retain_target() -> dictionary.PronunciationTargetId:
        sound = required_sound("prami")
        sequence = sound.pronunciation_targets
        return next(target for target in sequence.targets if target.realization_count > 1)

    target = retain_target()
    gc.collect()
    assert target.value == int(target)
    assert len(target.realizations) == target.realization_count


def test_pronunciation_realization_survives_gc_independently() -> None:
    def retain_realization() -> dictionary.IpaSegmentId:
        sound = required_sound("prami")
        sequence = sound.pronunciation_targets
        target = next(
            value for value in sequence.targets if value.realization_count > 1
        )
        return target.realizations[0]

    realization = retain_realization()
    gc.collect()
    assert realization.value == int(realization)
    assert realization.symbol


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
        dictionary.PronunciationTargetSequenceView,
        dictionary.PronunciationTargetId,
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

    first_sound = dictionary.english.sound_index[0]
    returned_only_classes = (
        dictionary.Dictionary,
        dictionary.DictionaryEntries,
        dictionary.DictionaryEntry,
        dictionary.RafsiMatch,
        dictionary.DictionarySoundEntry,
        dictionary.IpaTokenSequenceView,
        dictionary.IpaSegmentId,
        dictionary.PronunciationTargetSequenceView,
        dictionary.PronunciationTargetId,
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
    with pytest.raises(TypeError):
        dictionary.PronunciationTargetId(0)  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        dictionary.PronunciationTargetSequenceView(first_sound)  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        dictionary.PronunciationTargetId(0, 1)  # type: ignore[call-arg, arg-type]
    with pytest.raises(TypeError):
        dictionary.PronunciationTargetId(value=0)  # type: ignore[call-arg]
    with pytest.raises(TypeError):
        dictionary.PronunciationTargetId(  # type: ignore[call-arg]
            value=0,
            realization_count=1,
        )
    with pytest.raises(TypeError):
        dictionary.PronunciationTargetId(  # type: ignore[call-arg]
            value=0,
            realizations=(),
        )
    with pytest.raises(TypeError):
        dictionary.PronunciationTargetSequenceView(  # type: ignore[call-arg]
            first_sound,  # type: ignore[arg-type]
            0,
        )
    with pytest.raises(TypeError):
        dictionary.PronunciationTargetSequenceView(  # type: ignore[call-arg]
            sound=first_sound
        )
    with pytest.raises(TypeError):
        dictionary.PronunciationTargetSequenceView(  # type: ignore[call-arg]
            owner=dictionary.english,
            position=0,
        )
    with pytest.raises(TypeError):
        dictionary.PronunciationTargetSequenceView(  # type: ignore[call-arg]
            targets=(),
            self_similarity=0.0,
        )

    target_sequence = first_sound.pronunciation_targets
    target = target_sequence.targets[0]
    for property_owner, property_names in (
        (target, ("value", "realization_count", "realizations")),
        (target_sequence, ("targets", "self_similarity")),
    ):
        for property_name in property_names:
            with pytest.raises(AttributeError):
                setattr(property_owner, property_name, None)

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
        first_sound.pronunciation_targets,
        first_sound.pronunciation_targets.targets[0],
        first_sound.pronunciation_targets.targets[0].realizations[0],
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
            types.new_class("DerivedEnum", (enum_type,))


def test_pronunciation_target_runtime_and_stub_shape_is_exact() -> None:
    target_members = {
        name for name in vars(dictionary.PronunciationTargetId) if not name.startswith("_")
    }
    assert target_members == {
        "realization",
        "realization_count",
        "realizations",
        "value",
    }
    sequence_members = {
        name
        for name in vars(dictionary.PronunciationTargetSequenceView)
        if not name.startswith("_")
    }
    assert sequence_members == {"self_similarity", "target_count", "targets"}
    assert str(inspect.signature(dictionary.PronunciationTargetId.realization)) == (
        "(self, /, index)"
    )
    assert str(
        inspect.signature(dictionary.PronunciationTargetSequenceView.target_count)
    ) == "(self, /)"
    assert not hasattr(dictionary.PronunciationTargetId, "__match_args__")
    assert not hasattr(dictionary.PronunciationTargetSequenceView, "__match_args__")

    stub_path = PACKAGE_ROOT / "stubs" / "_native" / "dictionary.pyi"
    tree = ast.parse(stub_path.read_text(encoding="utf-8"), filename=str(stub_path))
    classes = {
        declaration.name: declaration
        for declaration in tree.body
        if isinstance(declaration, ast.ClassDef)
    }
    expected_functions = {
        "_dictionary_PronunciationTargetId": {
            "__eq__",
            "__ge__",
            "__gt__",
            "__hash__",
            "__index__",
            "__int__",
            "__le__",
            "__lt__",
            "__new__",
            "__repr__",
            "realization",
            "realization_count",
            "realizations",
            "value",
        },
        "_dictionary_PronunciationTargetSequenceView": {
            "__eq__",
            "__len__",
            "__new__",
            "__repr__",
            "self_similarity",
            "target_count",
            "targets",
        },
    }
    for class_name, function_names in expected_functions.items():
        declaration = classes[class_name]
        assert len(declaration.decorator_list) == 1
        final_decorator = declaration.decorator_list[0]
        assert isinstance(final_decorator, ast.Name)
        assert final_decorator.id == "final"
        functions = {
            statement.name: statement
            for statement in declaration.body
            if isinstance(statement, ast.FunctionDef)
        }
        assert set(functions) == function_names
        constructor = functions["__new__"]
        assert len(constructor.args.args) == 2
        sentinel = constructor.args.args[1]
        assert sentinel.arg == "_nonconstructible"
        assert isinstance(sentinel.annotation, ast.Name)
        assert sentinel.annotation.id == "Never"
        assert not any(
            isinstance(statement, ast.AnnAssign)
            and isinstance(statement.target, ast.Name)
            and statement.target.id == "__match_args__"
            for statement in declaration.body
        )

    target_realization = next(
        statement
        for statement in classes["_dictionary_PronunciationTargetId"].body
        if isinstance(statement, ast.FunctionDef) and statement.name == "realization"
    )
    assert [argument.arg for argument in target_realization.args.posonlyargs] == [
        "self"
    ]
    assert [argument.arg for argument in target_realization.args.args] == ["index"]
    index_annotation = target_realization.args.args[0].annotation
    assert isinstance(index_annotation, ast.Name)
    assert index_annotation.id == "int"
    return_annotation = target_realization.returns
    assert isinstance(return_annotation, ast.BinOp)
    assert isinstance(return_annotation.op, ast.BitOr)
    assert isinstance(return_annotation.left, ast.Name)
    assert return_annotation.left.id == "_dictionary_IpaSegmentId"
    assert isinstance(return_annotation.right, ast.Constant)
    assert return_annotation.right.value is None
    sequence_count = next(
        statement
        for statement in classes["_dictionary_PronunciationTargetSequenceView"].body
        if isinstance(statement, ast.FunctionDef) and statement.name == "target_count"
    )
    assert [argument.arg for argument in sequence_count.args.posonlyargs] == ["self"]
    assert sequence_count.args.args == []

    sound_class = classes["_dictionary_DictionarySoundEntry"]
    sound_properties = {
        statement.name
        for statement in sound_class.body
        if isinstance(statement, ast.FunctionDef)
        and any(
            isinstance(decorator, ast.Name) and decorator.id == "property"
            for decorator in statement.decorator_list
        )
    }
    assert sound_properties == {
        "entry_index",
        "ipa",
        "pronunciation_targets",
        "token_sequence",
    }

    composed = (PACKAGE_ROOT / "python" / "jbotci" / "_native.pyi").read_text(
        encoding="utf-8"
    )
    for class_name in expected_functions:
        assert composed.count(f"class {class_name}:") == 1


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
        dictionary.PronunciationTargetSequenceView,
        dictionary.PronunciationTargetId,
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
    dictionary.english.validate()

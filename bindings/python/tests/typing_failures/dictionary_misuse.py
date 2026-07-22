"""Intentional dictionary API type errors exercised by a subprocess test."""

from jbotci import dictionary

first = dictionary.english[0]
dictionary.english.count(first)
dictionary.english.entries.index(first)
dictionary.Dictionary()
dictionary.DictionaryEntry()
dictionary.IpaSegmentId()
dictionary.PronunciationTargetId()
dictionary.PronunciationTargetSequenceView()

sound = dictionary.english.sound_index[0]
sequence = sound.pronunciation_targets
target = sequence.targets[0]

target.realization(0.0)
dictionary.PronunciationTargetId(0)
dictionary.PronunciationTargetSequenceView(sound)


class InvalidPronunciationTargetId(dictionary.PronunciationTargetId):
    pass


class InvalidPronunciationTargetSequenceView(
    dictionary.PronunciationTargetSequenceView
):
    pass


target.value = 0
target.realization_count = 1
target.realizations = ()
sequence.targets = ()
sequence.self_similarity = 0.0

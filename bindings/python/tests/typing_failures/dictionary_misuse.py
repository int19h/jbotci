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

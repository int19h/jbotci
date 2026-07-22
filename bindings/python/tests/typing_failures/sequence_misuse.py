"""Intentional unordered collection inputs for Sequence-only APIs."""

from jbotci import dialect, morphology, source

entry = dialect.CmavoSwap("ce'u", "ce")
dialect.DialectDefinition({entry})

span = source.SourceSpan(0, 1, 0, 1)
part = morphology.LujvoRafsi(morphology.Phonemes("jbo"))
morphology.LujvoWord({part}, span)

word = morphology.CmavoWord(morphology.Phonemes("mi"), span)
morphology.QuotedWords(word, {word}, word)

build_part = morphology.LujvoRafsiBuildPart("jbo")
morphology.choose_best_lujvo_candidate_from_parts(
    morphology.LujvoBuildMode.LUJVO, {build_part}
)

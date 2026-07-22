"""Intentional construction attempts for parser-returned morphology classes."""

from jbotci import dialect, morphology

dialect.BuiltinDialect()

morphology.CompiledDialectWord()
morphology.CompiledDialectSwap()
morphology.CompiledDialectExpansion()
morphology.MorphologySegmentAttempt()
morphology.RecoveredMorphologySegmentation()
morphology.RecoveredMorphologySegmentAttempt()
morphology.ValsiAnalysis()

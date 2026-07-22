"""Pre-alpha Python bindings for jbotci's unstable Rust API."""

from . import diagnostics, dialect, dictionary, jvozba, morphology, semantics, source, syntax
from ._native import (
    InvalidInputError,
    JbotciError,
    _root_Sample as Sample,
    _root_SampleMode as SampleMode,
    __version__,
    raise_sample_error,
    sample_mode,
    smoke,
)

__all__: tuple[str, ...] = (
    "__version__",
    "dictionary",
    "diagnostics",
    "dialect",
    "jvozba",
    "morphology",
    "semantics",
    "source",
    "syntax",
    "InvalidInputError",
    "JbotciError",
    "Sample",
    "SampleMode",
    "raise_sample_error",
    "sample_mode",
    "smoke",
)

"""Strict-type-check smoke coverage for packaged public declarations."""

from jbotci import Sample, SampleMode, sample_mode, semantics, smoke


def sample_text(value: str | None) -> tuple[str, str | None]:
    """Exercise class, tuple, optional, and namespace annotations."""
    sample = Sample(value or "")
    mode: SampleMode = sample_mode(advanced=value is not None)
    assert semantics.references.__all__ == ()
    return (f"{smoke()}:{mode.value}", sample.value if value is not None else None)

"""Strict-type-check smoke coverage for packaged public declarations."""

from jbotci import Sample, semantics, smoke


def sample_text(value: str | None) -> tuple[str, str | None]:
    """Exercise class, tuple, optional, and namespace annotations."""
    sample = Sample(value or "")
    assert semantics.references.__all__ == ()
    return (smoke(), sample.value if value is not None else None)

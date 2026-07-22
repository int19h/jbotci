from enum import StrEnum
from typing import Final, final

__version__: Final[str]
__all__: tuple[str, ...]

class JbotciError(Exception):
    """Base exception for errors reported by jbotci."""

class InvalidInputError(JbotciError):
    """The supplied value is not valid input for an operation."""

"""Intentional omission from the closed jvozba error union."""

from typing import assert_never

from jbotci import jvozba


def render_error(value: jvozba.JvozbaErrorValue) -> str:
    if isinstance(value, jvozba.RequiresAtLeastTwoInputs):
        return str(value)
    if isinstance(value, jvozba.FixedRafsiEmpty):
        return str(value)
    if isinstance(value, jvozba.NonFinalUniversalLongRafsi):
        return value.offending
    if isinstance(value, jvozba.FinalConsonant):
        return value.offending
    if isinstance(value, jvozba.NoRafsiAvailable):
        return value.offending
    if isinstance(value, jvozba.NoDictionaryEntry):
        return value.offending
    if isinstance(value, jvozba.CouldNotBuildLujvo):
        return str(value)
    assert_never(value)

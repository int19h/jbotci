"""A missing parser payload variant must fail closed-union exhaustiveness."""

from typing import assert_never

from jbotci import syntax


def incomplete_expected_token(value: syntax.SyntaxExpectedToken) -> str:
    match value:
        case syntax.SyntaxExpectedTokenCmavo():
            return "cmavo"
        case syntax.SyntaxExpectedTokenSelmaho():
            return "selmaho"
        case syntax.SyntaxExpectedTokenWordCategory():
            return "category"
        case syntax.SyntaxExpectedTokenEndOfInput():
            return "end"
    assert_never(value)

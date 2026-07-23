"""A missing generated syntax variant must fail closed-union exhaustiveness."""

from typing import assert_never

from jbotci import syntax


def incomplete_linked_sumti(value: syntax.strict.LinkedSumtiSyntax) -> str:
    match value:
        case syntax.strict.LinkedSumtiSyntaxPlaceTaggedLinkedSumti():
            return "place"
        case syntax.strict.LinkedSumtiSyntaxTenseTaggedLinkedSumti():
            return "tense"
        case syntax.strict.LinkedSumtiSyntaxPlainLinkedSumti():
            return "plain"
    assert_never(value)

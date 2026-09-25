"""A missing generated syntax variant must fail closed-union exhaustiveness."""

from typing import assert_never

from jbotci import syntax


def incomplete_linked_term(value: syntax.strict.LinkedTermSyntax) -> str:
    match value:
        case syntax.strict.LinkedTermSyntaxPlaceTaggedLinkedSumti():
            return "place"
        case syntax.strict.LinkedTermSyntaxTenseTaggedLinkedSumti():
            return "tense"
        case syntax.strict.LinkedTermSyntaxPlainLinkedSumti():
            return "plain"
        case syntax.strict.LinkedTermSyntaxConnectedLinkedTerm():
            return "connected"
        case syntax.strict.LinkedTermSyntaxBoundLinkedTermConnection():
            return "bound"
    assert_never(value)

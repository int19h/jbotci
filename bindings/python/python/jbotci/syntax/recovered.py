# Generated from the canonical syntax binding schema; do not edit.
'Generated recovered syntax model. Exhaustive variant checking relies on the packaged type hints and a type checker.'
from __future__ import annotations

from collections.abc import Sequence
from typing import TypeAlias, cast, final

from jbotci.morphology import Cmavo, Selmaho, Word, WordLike
from jbotci.source import SourceId, SourceSpan
from jbotci.syntax import Chain, RecoveredField, Token, WithFreeModifiers, WithIndicators
from jbotci.syntax._runtime import _SyntaxNode

@final
class LeadingIndicatorSyntax(_SyntaxNode):
    'A UI/CAI indicator together with its optional attached NAI word.'
    __slots__ = ()
    _schema_id = 0
    __match_args__ = ('indicator', 'nai')
    def __new__(cls, indicator: RecoveredField[Token], nai: RecoveredField[Token] | None) -> LeadingIndicatorSyntax:
        return cls._from_fields((indicator, nai))
    def __init__(self, indicator: RecoveredField[Token], nai: RecoveredField[Token] | None) -> None:
        pass
    @property
    def indicator(self) -> RecoveredField[Token]:
        'The UI or CAI indicator word.'
        return cast(RecoveredField[Token], self._field(0))
    @property
    def nai(self) -> RecoveredField[Token] | None:
        'The optional NAI word attached to the indicator.'
        return cast(RecoveredField[Token] | None, self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('LeadingIndicatorSyntax is final')

@final
class TextSyntaxExplicitXauhaLohoiText(_SyntaxNode):
    'Text introduced by XAUhA and closed by KUhAU; the payload retains the framed paragraphs.'
    __slots__ = ()
    _schema_id = 1
    __match_args__ = ('explicit_xauha_lohoi_text',)
    def __new__(cls, explicit_xauha_lohoi_text: RecoveredField[ExplicitXauhaLohoiTextSyntax]) -> TextSyntaxExplicitXauhaLohoiText:
        return cls._from_fields((explicit_xauha_lohoi_text,))
    def __init__(self, explicit_xauha_lohoi_text: RecoveredField[ExplicitXauhaLohoiTextSyntax]) -> None:
        pass
    @property
    def explicit_xauha_lohoi_text(self) -> RecoveredField[ExplicitXauhaLohoiTextSyntax]:
        'Text introduced by XAUhA and closed by KUhAU; the payload retains the framed paragraphs.'
        return cast(RecoveredField[ExplicitXauhaLohoiTextSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TextSyntaxExplicitXauhaLohoiText is final')

@final
class TextSyntaxRegularText(_SyntaxNode):
    'Ordinary text, retaining its leading material and optional paragraph tree.'
    __slots__ = ()
    _schema_id = 2
    __match_args__ = ('regular_text',)
    def __new__(cls, regular_text: RecoveredField[RegularTextSyntax]) -> TextSyntaxRegularText:
        return cls._from_fields((regular_text,))
    def __init__(self, regular_text: RecoveredField[RegularTextSyntax]) -> None:
        pass
    @property
    def regular_text(self) -> RecoveredField[RegularTextSyntax]:
        'Ordinary text, retaining its leading material and optional paragraph tree.'
        return cast(RecoveredField[RegularTextSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TextSyntaxRegularText is final')

TextSyntax: TypeAlias = TextSyntaxExplicitXauhaLohoiText | TextSyntaxRegularText

@final
class ExplicitXauhaLohoiTextSyntax(_SyntaxNode):
    'XAUhA…KUhAU-framed text; framing words are consumed while paragraphs remain public.'
    __slots__ = ()
    _schema_id = 3
    __match_args__ = ('paragraphs',)
    def __new__(cls, paragraphs: RecoveredField[TextParagraphWithAdditionalNihoSyntax]) -> ExplicitXauhaLohoiTextSyntax:
        return cls._from_fields((paragraphs,))
    def __init__(self, paragraphs: RecoveredField[TextParagraphWithAdditionalNihoSyntax]) -> None:
        pass
    @property
    def paragraphs(self) -> RecoveredField[TextParagraphWithAdditionalNihoSyntax]:
        'The paragraphs enclosed by the ignored XAUhA…KUhAU framing sequence.'
        return cast(RecoveredField[TextParagraphWithAdditionalNihoSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ExplicitXauhaLohoiTextSyntax is final')

@final
class RegularTextSyntax(_SyntaxNode):
    'Ordinary text with source-ordered leading material and an optional paragraph tree.'
    __slots__ = ()
    _schema_id = 4
    __match_args__ = ('leading_nai', 'leading_cmevla', 'leading_indicators', 'leading_free_modifiers', 'leading_connective', 'leading_i_statements', 'paragraphs')
    def __new__(cls, leading_nai: Sequence[RecoveredField[Token]], leading_cmevla: Sequence[RecoveredField[Token]], leading_indicators: Sequence[RecoveredField[LeadingIndicatorSyntax]], leading_free_modifiers: Sequence[RecoveredField[FreeModifierSyntax]], leading_connective: RecoveredField[TextLeadingConnectiveSyntax] | None, leading_i_statements: Sequence[RecoveredField[LeadingIStatementSyntax]], paragraphs: RecoveredField[TextParagraphsSyntax] | None) -> RegularTextSyntax:
        return cls._from_fields((leading_nai, leading_cmevla, leading_indicators, leading_free_modifiers, leading_connective, leading_i_statements, paragraphs))
    def __init__(self, leading_nai: Sequence[RecoveredField[Token]], leading_cmevla: Sequence[RecoveredField[Token]], leading_indicators: Sequence[RecoveredField[LeadingIndicatorSyntax]], leading_free_modifiers: Sequence[RecoveredField[FreeModifierSyntax]], leading_connective: RecoveredField[TextLeadingConnectiveSyntax] | None, leading_i_statements: Sequence[RecoveredField[LeadingIStatementSyntax]], paragraphs: RecoveredField[TextParagraphsSyntax] | None) -> None:
        pass
    @property
    def leading_nai(self) -> tuple[RecoveredField[Token], ...]:
        'NAI words that precede the first formal text construct.'
        return cast(tuple[RecoveredField[Token], ...], self._field(0))
    @property
    def leading_cmevla(self) -> tuple[RecoveredField[Token], ...]:
        'CMEVLA words accepted before the first formal text construct.'
        return cast(tuple[RecoveredField[Token], ...], self._field(1))
    @property
    def leading_indicators(self) -> tuple[RecoveredField[LeadingIndicatorSyntax], ...]:
        'UI/CAI indicators accepted before the first formal text construct.'
        return cast(tuple[RecoveredField[LeadingIndicatorSyntax], ...], self._field(2))
    @property
    def leading_free_modifiers(self) -> tuple[RecoveredField[FreeModifierSyntax], ...]:
        'Free modifiers accepted before the first formal text construct.'
        return cast(tuple[RecoveredField[FreeModifierSyntax], ...], self._field(3))
    @property
    def leading_connective(self) -> RecoveredField[TextLeadingConnectiveSyntax] | None:
        'A text-leading connective when it is not the start of a modal forethought connective.'
        return cast(RecoveredField[TextLeadingConnectiveSyntax] | None, self._field(4))
    @property
    def leading_i_statements(self) -> tuple[RecoveredField[LeadingIStatementSyntax], ...]:
        'I-led statement prefixes that occur before the paragraph tree.'
        return cast(tuple[RecoveredField[LeadingIStatementSyntax], ...], self._field(5))
    @property
    def paragraphs(self) -> RecoveredField[TextParagraphsSyntax] | None:
        'The primary paragraph subtree, absent when the text contains only leading material.'
        return cast(RecoveredField[TextParagraphsSyntax] | None, self._field(6))
    def __init_subclass__(cls) -> None:
        raise TypeError('RegularTextSyntax is final')

@final
class TextParagraphsSyntaxTextParagraphWithAdditionalNiho(_SyntaxNode):
    'Uses the `text_paragraph_with_additional_niho` product form, whose payload preserves `first` and `additional_niho`.'
    __slots__ = ()
    _schema_id = 5
    __match_args__ = ('text_paragraph_with_additional_niho',)
    def __new__(cls, text_paragraph_with_additional_niho: RecoveredField[TextParagraphWithAdditionalNihoSyntax]) -> TextParagraphsSyntaxTextParagraphWithAdditionalNiho:
        return cls._from_fields((text_paragraph_with_additional_niho,))
    def __init__(self, text_paragraph_with_additional_niho: RecoveredField[TextParagraphWithAdditionalNihoSyntax]) -> None:
        pass
    @property
    def text_paragraph_with_additional_niho(self) -> RecoveredField[TextParagraphWithAdditionalNihoSyntax]:
        'Uses the `text_paragraph_with_additional_niho` product form, whose payload preserves `first` and `additional_niho`.'
        return cast(RecoveredField[TextParagraphWithAdditionalNihoSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TextParagraphsSyntaxTextParagraphWithAdditionalNiho is final')

@final
class TextParagraphsSyntaxTextNihoParagraphs(_SyntaxNode):
    'Uses the `text_niho_paragraphs` product form, whose payload preserves `paragraphs`.'
    __slots__ = ()
    _schema_id = 6
    __match_args__ = ('text_niho_paragraphs',)
    def __new__(cls, text_niho_paragraphs: RecoveredField[TextNihoParagraphsSyntax]) -> TextParagraphsSyntaxTextNihoParagraphs:
        return cls._from_fields((text_niho_paragraphs,))
    def __init__(self, text_niho_paragraphs: RecoveredField[TextNihoParagraphsSyntax]) -> None:
        pass
    @property
    def text_niho_paragraphs(self) -> RecoveredField[TextNihoParagraphsSyntax]:
        'Uses the `text_niho_paragraphs` product form, whose payload preserves `paragraphs`.'
        return cast(RecoveredField[TextNihoParagraphsSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TextParagraphsSyntaxTextNihoParagraphs is final')

TextParagraphsSyntax: TypeAlias = TextParagraphsSyntaxTextParagraphWithAdditionalNiho | TextParagraphsSyntaxTextNihoParagraphs

@final
class TextParagraphWithAdditionalNihoSyntax(_SyntaxNode):
    'Product node for paragraphs; preserves `first` and `additional_niho` in source order.'
    __slots__ = ()
    _schema_id = 7
    __match_args__ = ('first', 'additional_niho')
    def __new__(cls, first: RecoveredField[ParagraphSyntax], additional_niho: Sequence[RecoveredField[NihoParagraphSyntax]]) -> TextParagraphWithAdditionalNihoSyntax:
        return cls._from_fields((first, additional_niho))
    def __init__(self, first: RecoveredField[ParagraphSyntax], additional_niho: Sequence[RecoveredField[NihoParagraphSyntax]]) -> None:
        pass
    @property
    def first(self) -> RecoveredField[ParagraphSyntax]:
        'The initial paragraph before zero or more NIhO-led paragraph continuations.'
        return cast(RecoveredField[ParagraphSyntax], self._field(0))
    @property
    def additional_niho(self) -> tuple[RecoveredField[NihoParagraphSyntax], ...]:
        'Ordered sequence of zero or more additional niho components.'
        return cast(tuple[RecoveredField[NihoParagraphSyntax], ...], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('TextParagraphWithAdditionalNihoSyntax is final')

@final
class TextNihoParagraphsSyntax(_SyntaxNode):
    'Transparent product node for paragraphs; preserves the `paragraphs` component.'
    __slots__ = ()
    _schema_id = 8
    __match_args__ = ('paragraphs',)
    def __new__(cls, paragraphs: Sequence[RecoveredField[NihoParagraphSyntax]]) -> TextNihoParagraphsSyntax:
        return cls._from_fields((paragraphs,))
    def __init__(self, paragraphs: Sequence[RecoveredField[NihoParagraphSyntax]]) -> None:
        pass
    @property
    def paragraphs(self) -> tuple[RecoveredField[NihoParagraphSyntax], ...]:
        'Non-empty ordered sequence of paragraphs components.'
        return cast(tuple[RecoveredField[NihoParagraphSyntax], ...], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TextNihoParagraphsSyntax is final')

@final
class LeadingIStatementSyntax(_SyntaxNode):
    'Product node for paragraph statement; preserves `i`, `connective`, and `free_modifiers` in source order.'
    __slots__ = ()
    _schema_id = 9
    __match_args__ = ('i', 'connective', 'free_modifiers')
    def __new__(cls, i: RecoveredField[Token], connective: RecoveredField[IParagraphStatementConnectiveSyntax] | None, free_modifiers: Sequence[RecoveredField[FreeModifierSyntax]]) -> LeadingIStatementSyntax:
        return cls._from_fields((i, connective, free_modifiers))
    def __init__(self, i: RecoveredField[Token], connective: RecoveredField[IParagraphStatementConnectiveSyntax] | None, free_modifiers: Sequence[RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def i(self) -> RecoveredField[Token]:
        'The `I` cmavo marker.'
        return cast(RecoveredField[Token], self._field(0))
    @property
    def connective(self) -> RecoveredField[IParagraphStatementConnectiveSyntax] | None:
        'The optional connective component.'
        return cast(RecoveredField[IParagraphStatementConnectiveSyntax] | None, self._field(1))
    @property
    def free_modifiers(self) -> tuple[RecoveredField[FreeModifierSyntax], ...]:
        'Ordered sequence of zero or more free modifiers components.'
        return cast(tuple[RecoveredField[FreeModifierSyntax], ...], self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('LeadingIStatementSyntax is final')

@final
class ParagraphSyntaxINihoParagraph(_SyntaxNode):
    'Uses the `i_niho_paragraph` product form, whose payload preserves `i`, `niho`, `free_modifiers`, and `statements`.'
    __slots__ = ()
    _schema_id = 10
    __match_args__ = ('i_niho_paragraph',)
    def __new__(cls, i_niho_paragraph: RecoveredField[INihoParagraphSyntax]) -> ParagraphSyntaxINihoParagraph:
        return cls._from_fields((i_niho_paragraph,))
    def __init__(self, i_niho_paragraph: RecoveredField[INihoParagraphSyntax]) -> None:
        pass
    @property
    def i_niho_paragraph(self) -> RecoveredField[INihoParagraphSyntax]:
        'Uses the `i_niho_paragraph` product form, whose payload preserves `i`, `niho`, `free_modifiers`, and `statements`.'
        return cast(RecoveredField[INihoParagraphSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ParagraphSyntaxINihoParagraph is final')

@final
class ParagraphSyntaxSimpleParagraph(_SyntaxNode):
    'Uses the `simple_paragraph` product form, whose payload preserves `statements`.'
    __slots__ = ()
    _schema_id = 11
    __match_args__ = ('simple_paragraph',)
    def __new__(cls, simple_paragraph: RecoveredField[SimpleParagraphSyntax]) -> ParagraphSyntaxSimpleParagraph:
        return cls._from_fields((simple_paragraph,))
    def __init__(self, simple_paragraph: RecoveredField[SimpleParagraphSyntax]) -> None:
        pass
    @property
    def simple_paragraph(self) -> RecoveredField[SimpleParagraphSyntax]:
        'Uses the `simple_paragraph` product form, whose payload preserves `statements`.'
        return cast(RecoveredField[SimpleParagraphSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ParagraphSyntaxSimpleParagraph is final')

ParagraphSyntax: TypeAlias = ParagraphSyntaxINihoParagraph | ParagraphSyntaxSimpleParagraph

@final
class SimpleParagraphSyntax(_SyntaxNode):
    'Transparent product node for paragraph; preserves the `statements` component.'
    __slots__ = ()
    _schema_id = 12
    __match_args__ = ('statements',)
    def __new__(cls, statements: RecoveredField[ParagraphStatementSequenceSyntax]) -> SimpleParagraphSyntax:
        return cls._from_fields((statements,))
    def __init__(self, statements: RecoveredField[ParagraphStatementSequenceSyntax]) -> None:
        pass
    @property
    def statements(self) -> RecoveredField[ParagraphStatementSequenceSyntax]:
        'The paragraph primary statement sequence.'
        return cast(RecoveredField[ParagraphStatementSequenceSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleParagraphSyntax is final')

@final
class ParagraphStatementSequenceSyntax(_SyntaxNode):
    'Product node for paragraph statement sequence; preserves `initial`, `following`, and `trailing` in source order.'
    __slots__ = ()
    _schema_id = 13
    __match_args__ = ('initial', 'following', 'trailing')
    def __new__(cls, initial: RecoveredField[InitialParagraphStatementSyntax], following: Sequence[RecoveredField[FollowingParagraphStatementSyntax]], trailing: Sequence[RecoveredField[TrailingIjekParagraphStatementSyntax]]) -> ParagraphStatementSequenceSyntax:
        return cls._from_fields((initial, following, trailing))
    def __init__(self, initial: RecoveredField[InitialParagraphStatementSyntax], following: Sequence[RecoveredField[FollowingParagraphStatementSyntax]], trailing: Sequence[RecoveredField[TrailingIjekParagraphStatementSyntax]]) -> None:
        pass
    @property
    def initial(self) -> RecoveredField[InitialParagraphStatementSyntax]:
        'The initial paragraph statement before following I-led or trailing-connective entries.'
        return cast(RecoveredField[InitialParagraphStatementSyntax], self._field(0))
    @property
    def following(self) -> tuple[RecoveredField[FollowingParagraphStatementSyntax], ...]:
        'Ordered sequence of zero or more following components.'
        return cast(tuple[RecoveredField[FollowingParagraphStatementSyntax], ...], self._field(1))
    @property
    def trailing(self) -> tuple[RecoveredField[TrailingIjekParagraphStatementSyntax], ...]:
        'Ordered sequence of zero or more trailing components.'
        return cast(tuple[RecoveredField[TrailingIjekParagraphStatementSyntax], ...], self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('ParagraphStatementSequenceSyntax is final')

@final
class INihoParagraphSyntax(_SyntaxNode):
    'Product node for paragraph; preserves `i`, `niho`, `free_modifiers`, and `statements` in source order.'
    __slots__ = ()
    _schema_id = 14
    __match_args__ = ('i', 'niho', 'free_modifiers', 'statements')
    def __new__(cls, i: RecoveredField[Token], niho: Sequence[RecoveredField[Token]], free_modifiers: Sequence[RecoveredField[FreeModifierSyntax]], statements: RecoveredField[ParagraphStatementSequenceSyntax] | None) -> INihoParagraphSyntax:
        return cls._from_fields((i, niho, free_modifiers, statements))
    def __init__(self, i: RecoveredField[Token], niho: Sequence[RecoveredField[Token]], free_modifiers: Sequence[RecoveredField[FreeModifierSyntax]], statements: RecoveredField[ParagraphStatementSequenceSyntax] | None) -> None:
        pass
    @property
    def i(self) -> RecoveredField[Token]:
        'The `I` cmavo marker.'
        return cast(RecoveredField[Token], self._field(0))
    @property
    def niho(self) -> tuple[RecoveredField[Token], ...]:
        'Non-empty ordered sequence of niho components.'
        return cast(tuple[RecoveredField[Token], ...], self._field(1))
    @property
    def free_modifiers(self) -> tuple[RecoveredField[FreeModifierSyntax], ...]:
        'Ordered sequence of zero or more free modifiers components.'
        return cast(tuple[RecoveredField[FreeModifierSyntax], ...], self._field(2))
    @property
    def statements(self) -> RecoveredField[ParagraphStatementSequenceSyntax] | None:
        'The optional statements component.'
        return cast(RecoveredField[ParagraphStatementSequenceSyntax] | None, self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('INihoParagraphSyntax is final')

@final
class NihoParagraphSyntax(_SyntaxNode):
    'Product node for paragraph; preserves `niho`, `free_modifiers`, and `statements` in source order.'
    __slots__ = ()
    _schema_id = 15
    __match_args__ = ('niho', 'free_modifiers', 'statements')
    def __new__(cls, niho: Sequence[RecoveredField[Token]], free_modifiers: Sequence[RecoveredField[FreeModifierSyntax]], statements: RecoveredField[ParagraphStatementSequenceSyntax] | None) -> NihoParagraphSyntax:
        return cls._from_fields((niho, free_modifiers, statements))
    def __init__(self, niho: Sequence[RecoveredField[Token]], free_modifiers: Sequence[RecoveredField[FreeModifierSyntax]], statements: RecoveredField[ParagraphStatementSequenceSyntax] | None) -> None:
        pass
    @property
    def niho(self) -> tuple[RecoveredField[Token], ...]:
        'Non-empty ordered sequence of niho components.'
        return cast(tuple[RecoveredField[Token], ...], self._field(0))
    @property
    def free_modifiers(self) -> tuple[RecoveredField[FreeModifierSyntax], ...]:
        'Ordered sequence of zero or more free modifiers components.'
        return cast(tuple[RecoveredField[FreeModifierSyntax], ...], self._field(1))
    @property
    def statements(self) -> RecoveredField[ParagraphStatementSequenceSyntax] | None:
        'The optional statements component.'
        return cast(RecoveredField[ParagraphStatementSequenceSyntax] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('NihoParagraphSyntax is final')

@final
class InitialParagraphStatementSyntax(_SyntaxNode):
    'Transparent product node for paragraph statement; preserves the `statement` component.'
    __slots__ = ()
    _schema_id = 16
    __match_args__ = ('statement',)
    def __new__(cls, statement: RecoveredField[StatementOrFragmentSyntax]) -> InitialParagraphStatementSyntax:
        return cls._from_fields((statement,))
    def __init__(self, statement: RecoveredField[StatementOrFragmentSyntax]) -> None:
        pass
    @property
    def statement(self) -> RecoveredField[StatementOrFragmentSyntax]:
        'The shared statement child syntax node.'
        return cast(RecoveredField[StatementOrFragmentSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('InitialParagraphStatementSyntax is final')

@final
class FollowingParagraphStatementSyntax(_SyntaxNode):
    'Product node for paragraph statement; preserves `i`, `free_modifiers`, and `statement` in source order.'
    __slots__ = ()
    _schema_id = 17
    __match_args__ = ('i', 'free_modifiers', 'statement')
    def __new__(cls, i: RecoveredField[Token], free_modifiers: Sequence[RecoveredField[FreeModifierSyntax]], statement: RecoveredField[StatementOrFragmentSyntax] | None) -> FollowingParagraphStatementSyntax:
        return cls._from_fields((i, free_modifiers, statement))
    def __init__(self, i: RecoveredField[Token], free_modifiers: Sequence[RecoveredField[FreeModifierSyntax]], statement: RecoveredField[StatementOrFragmentSyntax] | None) -> None:
        pass
    @property
    def i(self) -> RecoveredField[Token]:
        'The `I` cmavo marker.'
        return cast(RecoveredField[Token], self._field(0))
    @property
    def free_modifiers(self) -> tuple[RecoveredField[FreeModifierSyntax], ...]:
        'Ordered sequence of zero or more free modifiers components.'
        return cast(tuple[RecoveredField[FreeModifierSyntax], ...], self._field(1))
    @property
    def statement(self) -> RecoveredField[StatementOrFragmentSyntax] | None:
        'The optional statement component.'
        return cast(RecoveredField[StatementOrFragmentSyntax] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('FollowingParagraphStatementSyntax is final')

@final
class TrailingIjekParagraphStatementSyntax(_SyntaxNode):
    'Product node for paragraph statement; preserves `i` and `connective` in source order.'
    __slots__ = ()
    _schema_id = 18
    __match_args__ = ('i', 'connective')
    def __new__(cls, i: RecoveredField[Token], connective: RecoveredField[StatementConnectiveSyntax]) -> TrailingIjekParagraphStatementSyntax:
        return cls._from_fields((i, connective))
    def __init__(self, i: RecoveredField[Token], connective: RecoveredField[StatementConnectiveSyntax]) -> None:
        pass
    @property
    def i(self) -> RecoveredField[Token]:
        'The `I` cmavo marker.'
        return cast(RecoveredField[Token], self._field(0))
    @property
    def connective(self) -> RecoveredField[StatementConnectiveSyntax]:
        'The statement connective after I, retained for the following paragraph statement.'
        return cast(RecoveredField[StatementConnectiveSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('TrailingIjekParagraphStatementSyntax is final')

@final
class StatementSyntaxIStatementConnection(_SyntaxNode):
    'Uses the `i_statement_connection` product form, whose payload preserves `leading_statement` and `continuations`.'
    __slots__ = ()
    _schema_id = 19
    __match_args__ = ('i_statement_connection',)
    def __new__(cls, i_statement_connection: RecoveredField[IStatementConnectionSyntax]) -> StatementSyntaxIStatementConnection:
        return cls._from_fields((i_statement_connection,))
    def __init__(self, i_statement_connection: RecoveredField[IStatementConnectionSyntax]) -> None:
        pass
    @property
    def i_statement_connection(self) -> RecoveredField[IStatementConnectionSyntax]:
        'Uses the `i_statement_connection` product form, whose payload preserves `leading_statement` and `continuations`.'
        return cast(RecoveredField[IStatementConnectionSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('StatementSyntaxIStatementConnection is final')

@final
class StatementSyntaxPreposedIStatementConnection(_SyntaxNode):
    'Uses the `preposed_i_statement_connection` product form, whose payload preserves `leading_statement`, `connective`, `i`, and `trailing_statement`.'
    __slots__ = ()
    _schema_id = 20
    __match_args__ = ('preposed_i_statement_connection',)
    def __new__(cls, preposed_i_statement_connection: RecoveredField[PreposedIStatementConnectionSyntax]) -> StatementSyntaxPreposedIStatementConnection:
        return cls._from_fields((preposed_i_statement_connection,))
    def __init__(self, preposed_i_statement_connection: RecoveredField[PreposedIStatementConnectionSyntax]) -> None:
        pass
    @property
    def preposed_i_statement_connection(self) -> RecoveredField[PreposedIStatementConnectionSyntax]:
        'Uses the `preposed_i_statement_connection` product form, whose payload preserves `leading_statement`, `connective`, `i`, and `trailing_statement`.'
        return cast(RecoveredField[PreposedIStatementConnectionSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('StatementSyntaxPreposedIStatementConnection is final')

@final
class StatementSyntaxStatementBase(_SyntaxNode):
    'Uses the nested `statement_base` sum form and preserves its selected alternative.'
    __slots__ = ()
    _schema_id = 21
    __match_args__ = ('statement_base',)
    def __new__(cls, statement_base: RecoveredField[StatementBaseSyntax]) -> StatementSyntaxStatementBase:
        return cls._from_fields((statement_base,))
    def __init__(self, statement_base: RecoveredField[StatementBaseSyntax]) -> None:
        pass
    @property
    def statement_base(self) -> RecoveredField[StatementBaseSyntax]:
        'Uses the nested `statement_base` sum form and preserves its selected alternative.'
        return cast(RecoveredField[StatementBaseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('StatementSyntaxStatementBase is final')

StatementSyntax: TypeAlias = StatementSyntaxIStatementConnection | StatementSyntaxPreposedIStatementConnection | StatementSyntaxStatementBase

@final
class StatementBaseSyntaxPrenexStatement(_SyntaxNode):
    'Uses the `prenex_statement` product form, whose payload preserves `prenex_terms`, `zohu`, and `inner_statement`.'
    __slots__ = ()
    _schema_id = 22
    __match_args__ = ('prenex_statement',)
    def __new__(cls, prenex_statement: RecoveredField[PrenexStatementSyntax]) -> StatementBaseSyntaxPrenexStatement:
        return cls._from_fields((prenex_statement,))
    def __init__(self, prenex_statement: RecoveredField[PrenexStatementSyntax]) -> None:
        pass
    @property
    def prenex_statement(self) -> RecoveredField[PrenexStatementSyntax]:
        'Uses the `prenex_statement` product form, whose payload preserves `prenex_terms`, `zohu`, and `inner_statement`.'
        return cast(RecoveredField[PrenexStatementSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('StatementBaseSyntaxPrenexStatement is final')

@final
class StatementBaseSyntaxForethoughtStatement(_SyntaxNode):
    'Uses the `forethought_statement` product form, whose payload preserves `gek`, `first`, `first_branch`, `additional_branches`, and `gihi`.'
    __slots__ = ()
    _schema_id = 23
    __match_args__ = ('forethought_statement',)
    def __new__(cls, forethought_statement: RecoveredField[ForethoughtStatementSyntax]) -> StatementBaseSyntaxForethoughtStatement:
        return cls._from_fields((forethought_statement,))
    def __init__(self, forethought_statement: RecoveredField[ForethoughtStatementSyntax]) -> None:
        pass
    @property
    def forethought_statement(self) -> RecoveredField[ForethoughtStatementSyntax]:
        'Uses the `forethought_statement` product form, whose payload preserves `gek`, `first`, `first_branch`, `additional_branches`, and `gihi`.'
        return cast(RecoveredField[ForethoughtStatementSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('StatementBaseSyntaxForethoughtStatement is final')

@final
class StatementBaseSyntaxBridiStatement(_SyntaxNode):
    'Uses the `bridi_statement` product form, whose payload preserves `bridi` and `continuations`.'
    __slots__ = ()
    _schema_id = 24
    __match_args__ = ('bridi_statement',)
    def __new__(cls, bridi_statement: RecoveredField[BridiStatementSyntax]) -> StatementBaseSyntaxBridiStatement:
        return cls._from_fields((bridi_statement,))
    def __init__(self, bridi_statement: RecoveredField[BridiStatementSyntax]) -> None:
        pass
    @property
    def bridi_statement(self) -> RecoveredField[BridiStatementSyntax]:
        'Uses the `bridi_statement` product form, whose payload preserves `bridi` and `continuations`.'
        return cast(RecoveredField[BridiStatementSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('StatementBaseSyntaxBridiStatement is final')

@final
class StatementBaseSyntaxTextGroupStatement(_SyntaxNode):
    'Uses the `text_group_statement` product form, whose payload preserves `tense_modal`, `tuhe`, `text`, and `tuhu`.'
    __slots__ = ()
    _schema_id = 25
    __match_args__ = ('text_group_statement',)
    def __new__(cls, text_group_statement: RecoveredField[TextGroupStatementSyntax]) -> StatementBaseSyntaxTextGroupStatement:
        return cls._from_fields((text_group_statement,))
    def __init__(self, text_group_statement: RecoveredField[TextGroupStatementSyntax]) -> None:
        pass
    @property
    def text_group_statement(self) -> RecoveredField[TextGroupStatementSyntax]:
        'Uses the `text_group_statement` product form, whose payload preserves `tense_modal`, `tuhe`, `text`, and `tuhu`.'
        return cast(RecoveredField[TextGroupStatementSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('StatementBaseSyntaxTextGroupStatement is final')

StatementBaseSyntax: TypeAlias = StatementBaseSyntaxPrenexStatement | StatementBaseSyntaxForethoughtStatement | StatementBaseSyntaxBridiStatement | StatementBaseSyntaxTextGroupStatement

@final
class StatementOrFragmentSyntaxZantufaStatementTermsStatement(_SyntaxNode):
    'Uses the `zantufa_statement_terms_statement` product form, whose payload preserves `statement` and `tail`.'
    __slots__ = ()
    _schema_id = 26
    __match_args__ = ('zantufa_statement_terms_statement',)
    def __new__(cls, zantufa_statement_terms_statement: RecoveredField[ZantufaStatementTermsStatementSyntax]) -> StatementOrFragmentSyntaxZantufaStatementTermsStatement:
        return cls._from_fields((zantufa_statement_terms_statement,))
    def __init__(self, zantufa_statement_terms_statement: RecoveredField[ZantufaStatementTermsStatementSyntax]) -> None:
        pass
    @property
    def zantufa_statement_terms_statement(self) -> RecoveredField[ZantufaStatementTermsStatementSyntax]:
        'Uses the `zantufa_statement_terms_statement` product form, whose payload preserves `statement` and `tail`.'
        return cast(RecoveredField[ZantufaStatementTermsStatementSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('StatementOrFragmentSyntaxZantufaStatementTermsStatement is final')

@final
class StatementOrFragmentSyntaxStatementOrFragmentStatement(_SyntaxNode):
    'Uses the `statement_or_fragment_statement` product form, whose payload preserves `statement`.'
    __slots__ = ()
    _schema_id = 27
    __match_args__ = ('statement_or_fragment_statement',)
    def __new__(cls, statement_or_fragment_statement: RecoveredField[StatementOrFragmentStatementSyntax]) -> StatementOrFragmentSyntaxStatementOrFragmentStatement:
        return cls._from_fields((statement_or_fragment_statement,))
    def __init__(self, statement_or_fragment_statement: RecoveredField[StatementOrFragmentStatementSyntax]) -> None:
        pass
    @property
    def statement_or_fragment_statement(self) -> RecoveredField[StatementOrFragmentStatementSyntax]:
        'Uses the `statement_or_fragment_statement` product form, whose payload preserves `statement`.'
        return cast(RecoveredField[StatementOrFragmentStatementSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('StatementOrFragmentSyntaxStatementOrFragmentStatement is final')

@final
class StatementOrFragmentSyntaxFragmentStatement(_SyntaxNode):
    'Uses the nested `fragment_statement` sum form and preserves its selected alternative.'
    __slots__ = ()
    _schema_id = 28
    __match_args__ = ('fragment_statement',)
    def __new__(cls, fragment_statement: RecoveredField[FragmentStatementSyntax]) -> StatementOrFragmentSyntaxFragmentStatement:
        return cls._from_fields((fragment_statement,))
    def __init__(self, fragment_statement: RecoveredField[FragmentStatementSyntax]) -> None:
        pass
    @property
    def fragment_statement(self) -> RecoveredField[FragmentStatementSyntax]:
        'Uses the nested `fragment_statement` sum form and preserves its selected alternative.'
        return cast(RecoveredField[FragmentStatementSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('StatementOrFragmentSyntaxFragmentStatement is final')

StatementOrFragmentSyntax: TypeAlias = StatementOrFragmentSyntaxZantufaStatementTermsStatement | StatementOrFragmentSyntaxStatementOrFragmentStatement | StatementOrFragmentSyntaxFragmentStatement

@final
class ZantufaStatementTermsStatementSyntax(_SyntaxNode):
    'Product node for paragraph statement; preserves `statement` and `tail` in source order.'
    __slots__ = ()
    _schema_id = 29
    __match_args__ = ('statement', 'tail')
    def __new__(cls, statement: RecoveredField[StatementSyntax], tail: RecoveredField[ZantufaStatementTermsTailSyntax]) -> ZantufaStatementTermsStatementSyntax:
        return cls._from_fields((statement, tail))
    def __init__(self, statement: RecoveredField[StatementSyntax], tail: RecoveredField[ZantufaStatementTermsTailSyntax]) -> None:
        pass
    @property
    def statement(self) -> RecoveredField[StatementSyntax]:
        'The shared statement child syntax node.'
        return cast(RecoveredField[StatementSyntax], self._field(0))
    @property
    def tail(self) -> RecoveredField[ZantufaStatementTermsTailSyntax]:
        'The `zantufa_statement_terms_tail` grammar result in the `tail` structural role of the `zantufa_statement_terms_statement` production.'
        return cast(RecoveredField[ZantufaStatementTermsTailSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaStatementTermsStatementSyntax is final')

@final
class ZantufaStatementTermsTailSyntaxZantufaIauStatementTermsTail(_SyntaxNode):
    'Uses the `zantufa_iau_statement_terms_tail` product form, whose payload preserves `iau` and `terms`.'
    __slots__ = ()
    _schema_id = 30
    __match_args__ = ('zantufa_iau_statement_terms_tail',)
    def __new__(cls, zantufa_iau_statement_terms_tail: RecoveredField[ZantufaIauStatementTermsTailSyntax]) -> ZantufaStatementTermsTailSyntaxZantufaIauStatementTermsTail:
        return cls._from_fields((zantufa_iau_statement_terms_tail,))
    def __init__(self, zantufa_iau_statement_terms_tail: RecoveredField[ZantufaIauStatementTermsTailSyntax]) -> None:
        pass
    @property
    def zantufa_iau_statement_terms_tail(self) -> RecoveredField[ZantufaIauStatementTermsTailSyntax]:
        'Uses the `zantufa_iau_statement_terms_tail` product form, whose payload preserves `iau` and `terms`.'
        return cast(RecoveredField[ZantufaIauStatementTermsTailSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaStatementTermsTailSyntaxZantufaIauStatementTermsTail is final')

@final
class ZantufaStatementTermsTailSyntaxZantufaBareStatementTermsTail(_SyntaxNode):
    'Uses the `zantufa_bare_statement_terms_tail` product form, whose payload preserves `terms`.'
    __slots__ = ()
    _schema_id = 31
    __match_args__ = ('zantufa_bare_statement_terms_tail',)
    def __new__(cls, zantufa_bare_statement_terms_tail: RecoveredField[ZantufaBareStatementTermsTailSyntax]) -> ZantufaStatementTermsTailSyntaxZantufaBareStatementTermsTail:
        return cls._from_fields((zantufa_bare_statement_terms_tail,))
    def __init__(self, zantufa_bare_statement_terms_tail: RecoveredField[ZantufaBareStatementTermsTailSyntax]) -> None:
        pass
    @property
    def zantufa_bare_statement_terms_tail(self) -> RecoveredField[ZantufaBareStatementTermsTailSyntax]:
        'Uses the `zantufa_bare_statement_terms_tail` product form, whose payload preserves `terms`.'
        return cast(RecoveredField[ZantufaBareStatementTermsTailSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaStatementTermsTailSyntaxZantufaBareStatementTermsTail is final')

ZantufaStatementTermsTailSyntax: TypeAlias = ZantufaStatementTermsTailSyntaxZantufaIauStatementTermsTail | ZantufaStatementTermsTailSyntaxZantufaBareStatementTermsTail

@final
class ZantufaIauStatementTermsTailSyntax(_SyntaxNode):
    'Product node for paragraph statement; preserves `iau` and `terms` in source order.'
    __slots__ = ()
    _schema_id = 32
    __match_args__ = ('iau', 'terms')
    def __new__(cls, iau: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], terms: Sequence[RecoveredField[TermSyntax]]) -> ZantufaIauStatementTermsTailSyntax:
        return cls._from_fields((iau, terms))
    def __init__(self, iau: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], terms: Sequence[RecoveredField[TermSyntax]]) -> None:
        pass
    @property
    def iau(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Ihau` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def terms(self) -> tuple[RecoveredField[TermSyntax], ...]:
        'Ordered sequence of zero or more terms components.'
        return cast(tuple[RecoveredField[TermSyntax], ...], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaIauStatementTermsTailSyntax is final')

@final
class ZantufaBareStatementTermsTailSyntax(_SyntaxNode):
    'Transparent product node for paragraph statement; preserves the `terms` component.'
    __slots__ = ()
    _schema_id = 33
    __match_args__ = ('terms',)
    def __new__(cls, terms: Sequence[RecoveredField[TermSyntax]]) -> ZantufaBareStatementTermsTailSyntax:
        return cls._from_fields((terms,))
    def __init__(self, terms: Sequence[RecoveredField[TermSyntax]]) -> None:
        pass
    @property
    def terms(self) -> tuple[RecoveredField[TermSyntax], ...]:
        'Non-empty ordered sequence of terms components.'
        return cast(tuple[RecoveredField[TermSyntax], ...], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaBareStatementTermsTailSyntax is final')

@final
class StatementOrFragmentStatementSyntax(_SyntaxNode):
    'Transparent product node for paragraph statement; preserves the `statement` component.'
    __slots__ = ()
    _schema_id = 34
    __match_args__ = ('statement',)
    def __new__(cls, statement: RecoveredField[StatementSyntax]) -> StatementOrFragmentStatementSyntax:
        return cls._from_fields((statement,))
    def __init__(self, statement: RecoveredField[StatementSyntax]) -> None:
        pass
    @property
    def statement(self) -> RecoveredField[StatementSyntax]:
        'The `statement` grammar result in the `statement` structural role of the `statement_or_fragment_statement` production.'
        return cast(RecoveredField[StatementSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('StatementOrFragmentStatementSyntax is final')

@final
class FragmentStatementSyntaxPrenexFragment(_SyntaxNode):
    'Uses the `prenex_fragment` product form, whose payload preserves `terms` and `zohu`.'
    __slots__ = ()
    _schema_id = 35
    __match_args__ = ('prenex_fragment',)
    def __new__(cls, prenex_fragment: RecoveredField[PrenexFragmentSyntax]) -> FragmentStatementSyntaxPrenexFragment:
        return cls._from_fields((prenex_fragment,))
    def __init__(self, prenex_fragment: RecoveredField[PrenexFragmentSyntax]) -> None:
        pass
    @property
    def prenex_fragment(self) -> RecoveredField[PrenexFragmentSyntax]:
        'Uses the `prenex_fragment` product form, whose payload preserves `terms` and `zohu`.'
        return cast(RecoveredField[PrenexFragmentSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('FragmentStatementSyntaxPrenexFragment is final')

@final
class FragmentStatementSyntaxSelbriFragment(_SyntaxNode):
    'Uses the `selbri_fragment` product form, whose payload preserves `selbri`.'
    __slots__ = ()
    _schema_id = 36
    __match_args__ = ('selbri_fragment',)
    def __new__(cls, selbri_fragment: RecoveredField[SelbriFragmentSyntax]) -> FragmentStatementSyntaxSelbriFragment:
        return cls._from_fields((selbri_fragment,))
    def __init__(self, selbri_fragment: RecoveredField[SelbriFragmentSyntax]) -> None:
        pass
    @property
    def selbri_fragment(self) -> RecoveredField[SelbriFragmentSyntax]:
        'Uses the `selbri_fragment` product form, whose payload preserves `selbri`.'
        return cast(RecoveredField[SelbriFragmentSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('FragmentStatementSyntaxSelbriFragment is final')

@final
class FragmentStatementSyntaxEkFragment(_SyntaxNode):
    'Uses the `ek_fragment` product form, whose payload preserves `connective`.'
    __slots__ = ()
    _schema_id = 37
    __match_args__ = ('ek_fragment',)
    def __new__(cls, ek_fragment: RecoveredField[EkFragmentSyntax]) -> FragmentStatementSyntaxEkFragment:
        return cls._from_fields((ek_fragment,))
    def __init__(self, ek_fragment: RecoveredField[EkFragmentSyntax]) -> None:
        pass
    @property
    def ek_fragment(self) -> RecoveredField[EkFragmentSyntax]:
        'Uses the `ek_fragment` product form, whose payload preserves `connective`.'
        return cast(RecoveredField[EkFragmentSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('FragmentStatementSyntaxEkFragment is final')

@final
class FragmentStatementSyntaxGihekFragment(_SyntaxNode):
    'Uses the `gihek_fragment` product form, whose payload preserves `connective`.'
    __slots__ = ()
    _schema_id = 38
    __match_args__ = ('gihek_fragment',)
    def __new__(cls, gihek_fragment: RecoveredField[GihekFragmentSyntax]) -> FragmentStatementSyntaxGihekFragment:
        return cls._from_fields((gihek_fragment,))
    def __init__(self, gihek_fragment: RecoveredField[GihekFragmentSyntax]) -> None:
        pass
    @property
    def gihek_fragment(self) -> RecoveredField[GihekFragmentSyntax]:
        'Uses the `gihek_fragment` product form, whose payload preserves `connective`.'
        return cast(RecoveredField[GihekFragmentSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('FragmentStatementSyntaxGihekFragment is final')

@final
class FragmentStatementSyntaxMultipleNaFragment(_SyntaxNode):
    'Uses the `multiple_na_fragment` product form, whose payload preserves `first_na`, `second_na`, and `additional_na`.'
    __slots__ = ()
    _schema_id = 39
    __match_args__ = ('multiple_na_fragment',)
    def __new__(cls, multiple_na_fragment: RecoveredField[MultipleNaFragmentSyntax]) -> FragmentStatementSyntaxMultipleNaFragment:
        return cls._from_fields((multiple_na_fragment,))
    def __init__(self, multiple_na_fragment: RecoveredField[MultipleNaFragmentSyntax]) -> None:
        pass
    @property
    def multiple_na_fragment(self) -> RecoveredField[MultipleNaFragmentSyntax]:
        'Uses the `multiple_na_fragment` product form, whose payload preserves `first_na`, `second_na`, and `additional_na`.'
        return cast(RecoveredField[MultipleNaFragmentSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('FragmentStatementSyntaxMultipleNaFragment is final')

@final
class FragmentStatementSyntaxSingleNaFragment(_SyntaxNode):
    'Uses the `single_na_fragment` product form, whose payload preserves `na`.'
    __slots__ = ()
    _schema_id = 40
    __match_args__ = ('single_na_fragment',)
    def __new__(cls, single_na_fragment: RecoveredField[SingleNaFragmentSyntax]) -> FragmentStatementSyntaxSingleNaFragment:
        return cls._from_fields((single_na_fragment,))
    def __init__(self, single_na_fragment: RecoveredField[SingleNaFragmentSyntax]) -> None:
        pass
    @property
    def single_na_fragment(self) -> RecoveredField[SingleNaFragmentSyntax]:
        'Uses the `single_na_fragment` product form, whose payload preserves `na`.'
        return cast(RecoveredField[SingleNaFragmentSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('FragmentStatementSyntaxSingleNaFragment is final')

@final
class FragmentStatementSyntaxTermsFragment(_SyntaxNode):
    'Uses the `terms_fragment` product form, whose payload preserves `terms` and `vau`.'
    __slots__ = ()
    _schema_id = 41
    __match_args__ = ('terms_fragment',)
    def __new__(cls, terms_fragment: RecoveredField[TermsFragmentSyntax]) -> FragmentStatementSyntaxTermsFragment:
        return cls._from_fields((terms_fragment,))
    def __init__(self, terms_fragment: RecoveredField[TermsFragmentSyntax]) -> None:
        pass
    @property
    def terms_fragment(self) -> RecoveredField[TermsFragmentSyntax]:
        'Uses the `terms_fragment` product form, whose payload preserves `terms` and `vau`.'
        return cast(RecoveredField[TermsFragmentSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('FragmentStatementSyntaxTermsFragment is final')

@final
class FragmentStatementSyntaxMeksoFragment(_SyntaxNode):
    'Uses the `mekso_fragment` product form, whose payload preserves `quantifier`.'
    __slots__ = ()
    _schema_id = 42
    __match_args__ = ('mekso_fragment',)
    def __new__(cls, mekso_fragment: RecoveredField[MeksoFragmentSyntax]) -> FragmentStatementSyntaxMeksoFragment:
        return cls._from_fields((mekso_fragment,))
    def __init__(self, mekso_fragment: RecoveredField[MeksoFragmentSyntax]) -> None:
        pass
    @property
    def mekso_fragment(self) -> RecoveredField[MeksoFragmentSyntax]:
        'Uses the `mekso_fragment` product form, whose payload preserves `quantifier`.'
        return cast(RecoveredField[MeksoFragmentSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('FragmentStatementSyntaxMeksoFragment is final')

@final
class FragmentStatementSyntaxRelativeClauseFragment(_SyntaxNode):
    'Uses the `relative_clause_fragment` product form, whose payload preserves `relative_clauses`.'
    __slots__ = ()
    _schema_id = 43
    __match_args__ = ('relative_clause_fragment',)
    def __new__(cls, relative_clause_fragment: RecoveredField[RelativeClauseFragmentSyntax]) -> FragmentStatementSyntaxRelativeClauseFragment:
        return cls._from_fields((relative_clause_fragment,))
    def __init__(self, relative_clause_fragment: RecoveredField[RelativeClauseFragmentSyntax]) -> None:
        pass
    @property
    def relative_clause_fragment(self) -> RecoveredField[RelativeClauseFragmentSyntax]:
        'Uses the `relative_clause_fragment` product form, whose payload preserves `relative_clauses`.'
        return cast(RecoveredField[RelativeClauseFragmentSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('FragmentStatementSyntaxRelativeClauseFragment is final')

@final
class FragmentStatementSyntaxLinkedSumtiContinuationFragment(_SyntaxNode):
    'Uses the `linked_sumti_continuation_fragment` product form, whose payload preserves `bei_links`.'
    __slots__ = ()
    _schema_id = 44
    __match_args__ = ('linked_sumti_continuation_fragment',)
    def __new__(cls, linked_sumti_continuation_fragment: RecoveredField[LinkedSumtiContinuationFragmentSyntax]) -> FragmentStatementSyntaxLinkedSumtiContinuationFragment:
        return cls._from_fields((linked_sumti_continuation_fragment,))
    def __init__(self, linked_sumti_continuation_fragment: RecoveredField[LinkedSumtiContinuationFragmentSyntax]) -> None:
        pass
    @property
    def linked_sumti_continuation_fragment(self) -> RecoveredField[LinkedSumtiContinuationFragmentSyntax]:
        'Uses the `linked_sumti_continuation_fragment` product form, whose payload preserves `bei_links`.'
        return cast(RecoveredField[LinkedSumtiContinuationFragmentSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('FragmentStatementSyntaxLinkedSumtiContinuationFragment is final')

@final
class FragmentStatementSyntaxLinkedSumtiFragment(_SyntaxNode):
    'Uses the `linked_sumti_fragment` product form, whose payload preserves `linkargs`.'
    __slots__ = ()
    _schema_id = 45
    __match_args__ = ('linked_sumti_fragment',)
    def __new__(cls, linked_sumti_fragment: RecoveredField[LinkedSumtiFragmentSyntax]) -> FragmentStatementSyntaxLinkedSumtiFragment:
        return cls._from_fields((linked_sumti_fragment,))
    def __init__(self, linked_sumti_fragment: RecoveredField[LinkedSumtiFragmentSyntax]) -> None:
        pass
    @property
    def linked_sumti_fragment(self) -> RecoveredField[LinkedSumtiFragmentSyntax]:
        'Uses the `linked_sumti_fragment` product form, whose payload preserves `linkargs`.'
        return cast(RecoveredField[LinkedSumtiFragmentSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('FragmentStatementSyntaxLinkedSumtiFragment is final')

@final
class FragmentStatementSyntaxZantufaMeksoFragment(_SyntaxNode):
    'Uses the `zantufa_mekso_fragment` product form, whose payload preserves `expression`.'
    __slots__ = ()
    _schema_id = 46
    __match_args__ = ('zantufa_mekso_fragment',)
    def __new__(cls, zantufa_mekso_fragment: RecoveredField[ZantufaMeksoFragmentSyntax]) -> FragmentStatementSyntaxZantufaMeksoFragment:
        return cls._from_fields((zantufa_mekso_fragment,))
    def __init__(self, zantufa_mekso_fragment: RecoveredField[ZantufaMeksoFragmentSyntax]) -> None:
        pass
    @property
    def zantufa_mekso_fragment(self) -> RecoveredField[ZantufaMeksoFragmentSyntax]:
        'Uses the `zantufa_mekso_fragment` product form, whose payload preserves `expression`.'
        return cast(RecoveredField[ZantufaMeksoFragmentSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('FragmentStatementSyntaxZantufaMeksoFragment is final')

FragmentStatementSyntax: TypeAlias = FragmentStatementSyntaxPrenexFragment | FragmentStatementSyntaxSelbriFragment | FragmentStatementSyntaxEkFragment | FragmentStatementSyntaxGihekFragment | FragmentStatementSyntaxMultipleNaFragment | FragmentStatementSyntaxSingleNaFragment | FragmentStatementSyntaxTermsFragment | FragmentStatementSyntaxMeksoFragment | FragmentStatementSyntaxRelativeClauseFragment | FragmentStatementSyntaxLinkedSumtiContinuationFragment | FragmentStatementSyntaxLinkedSumtiFragment | FragmentStatementSyntaxZantufaMeksoFragment

@final
class StatementAfterIConnectiveSyntaxForethoughtStatement(_SyntaxNode):
    'Uses the `forethought_statement` product form, whose payload preserves `gek`, `first`, `first_branch`, `additional_branches`, and `gihi`.'
    __slots__ = ()
    _schema_id = 47
    __match_args__ = ('forethought_statement',)
    def __new__(cls, forethought_statement: RecoveredField[ForethoughtStatementSyntax]) -> StatementAfterIConnectiveSyntaxForethoughtStatement:
        return cls._from_fields((forethought_statement,))
    def __init__(self, forethought_statement: RecoveredField[ForethoughtStatementSyntax]) -> None:
        pass
    @property
    def forethought_statement(self) -> RecoveredField[ForethoughtStatementSyntax]:
        'Uses the `forethought_statement` product form, whose payload preserves `gek`, `first`, `first_branch`, `additional_branches`, and `gihi`.'
        return cast(RecoveredField[ForethoughtStatementSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('StatementAfterIConnectiveSyntaxForethoughtStatement is final')

@final
class StatementAfterIConnectiveSyntaxBridiStatement(_SyntaxNode):
    'Uses the `bridi_statement` product form, whose payload preserves `bridi` and `continuations`.'
    __slots__ = ()
    _schema_id = 48
    __match_args__ = ('bridi_statement',)
    def __new__(cls, bridi_statement: RecoveredField[BridiStatementSyntax]) -> StatementAfterIConnectiveSyntaxBridiStatement:
        return cls._from_fields((bridi_statement,))
    def __init__(self, bridi_statement: RecoveredField[BridiStatementSyntax]) -> None:
        pass
    @property
    def bridi_statement(self) -> RecoveredField[BridiStatementSyntax]:
        'Uses the `bridi_statement` product form, whose payload preserves `bridi` and `continuations`.'
        return cast(RecoveredField[BridiStatementSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('StatementAfterIConnectiveSyntaxBridiStatement is final')

@final
class StatementAfterIConnectiveSyntaxTextGroupStatement(_SyntaxNode):
    'Uses the `text_group_statement` product form, whose payload preserves `tense_modal`, `tuhe`, `text`, and `tuhu`.'
    __slots__ = ()
    _schema_id = 49
    __match_args__ = ('text_group_statement',)
    def __new__(cls, text_group_statement: RecoveredField[TextGroupStatementSyntax]) -> StatementAfterIConnectiveSyntaxTextGroupStatement:
        return cls._from_fields((text_group_statement,))
    def __init__(self, text_group_statement: RecoveredField[TextGroupStatementSyntax]) -> None:
        pass
    @property
    def text_group_statement(self) -> RecoveredField[TextGroupStatementSyntax]:
        'Uses the `text_group_statement` product form, whose payload preserves `tense_modal`, `tuhe`, `text`, and `tuhu`.'
        return cast(RecoveredField[TextGroupStatementSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('StatementAfterIConnectiveSyntaxTextGroupStatement is final')

StatementAfterIConnectiveSyntax: TypeAlias = StatementAfterIConnectiveSyntaxForethoughtStatement | StatementAfterIConnectiveSyntaxBridiStatement | StatementAfterIConnectiveSyntaxTextGroupStatement

@final
class MultipleNaFragmentSyntax(_SyntaxNode):
    'Product node for fragment; preserves `first_na`, `second_na`, and `additional_na` in source order.'
    __slots__ = ()
    _schema_id = 50
    __match_args__ = ('first_na', 'second_na', 'additional_na')
    def __new__(cls, first_na: RecoveredField[Token], second_na: RecoveredField[Token], additional_na: Sequence[RecoveredField[Token]]) -> MultipleNaFragmentSyntax:
        return cls._from_fields((first_na, second_na, additional_na))
    def __init__(self, first_na: RecoveredField[Token], second_na: RecoveredField[Token], additional_na: Sequence[RecoveredField[Token]]) -> None:
        pass
    @property
    def first_na(self) -> RecoveredField[Token]:
        'A word from selmaho `Na`.'
        return cast(RecoveredField[Token], self._field(0))
    @property
    def second_na(self) -> RecoveredField[Token]:
        'A word from selmaho `Na`.'
        return cast(RecoveredField[Token], self._field(1))
    @property
    def additional_na(self) -> tuple[RecoveredField[Token], ...]:
        'Ordered sequence of zero or more additional na components.'
        return cast(tuple[RecoveredField[Token], ...], self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('MultipleNaFragmentSyntax is final')

@final
class SingleNaFragmentSyntax(_SyntaxNode):
    'Transparent product node for fragment; preserves the `na` component.'
    __slots__ = ()
    _schema_id = 51
    __match_args__ = ('na',)
    def __new__(cls, na: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> SingleNaFragmentSyntax:
        return cls._from_fields((na,))
    def __init__(self, na: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def na(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Na`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SingleNaFragmentSyntax is final')

@final
class EkFragmentSyntax(_SyntaxNode):
    'Transparent product node for fragment; preserves the `connective` component.'
    __slots__ = ()
    _schema_id = 52
    __match_args__ = ('connective',)
    def __new__(cls, connective: RecoveredField[EkConnectiveSyntax]) -> EkFragmentSyntax:
        return cls._from_fields((connective,))
    def __init__(self, connective: RecoveredField[EkConnectiveSyntax]) -> None:
        pass
    @property
    def connective(self) -> RecoveredField[EkConnectiveSyntax]:
        'The standalone `ek_connective` connective represented by the `ek_fragment` fragment.'
        return cast(RecoveredField[EkConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('EkFragmentSyntax is final')

@final
class GihekFragmentSyntax(_SyntaxNode):
    'Transparent product node for fragment; preserves the `connective` component.'
    __slots__ = ()
    _schema_id = 53
    __match_args__ = ('connective',)
    def __new__(cls, connective: RecoveredField[GihekConnectiveSyntax]) -> GihekFragmentSyntax:
        return cls._from_fields((connective,))
    def __init__(self, connective: RecoveredField[GihekConnectiveSyntax]) -> None:
        pass
    @property
    def connective(self) -> RecoveredField[GihekConnectiveSyntax]:
        'The standalone `gihek_connective` connective represented by the `gihek_fragment` fragment.'
        return cast(RecoveredField[GihekConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('GihekFragmentSyntax is final')

@final
class IStatementConnectionSyntax(_SyntaxNode):
    'Product node for statement connection; preserves `leading_statement` and `continuations` in source order.'
    __slots__ = ()
    _schema_id = 54
    __match_args__ = ('leading_statement', 'continuations')
    def __new__(cls, leading_statement: RecoveredField[StatementBaseSyntax], continuations: Sequence[RecoveredField[IStatementConnectionTailSyntax]]) -> IStatementConnectionSyntax:
        return cls._from_fields((leading_statement, continuations))
    def __init__(self, leading_statement: RecoveredField[StatementBaseSyntax], continuations: Sequence[RecoveredField[IStatementConnectionTailSyntax]]) -> None:
        pass
    @property
    def leading_statement(self) -> RecoveredField[StatementBaseSyntax]:
        'The shared leading statement child syntax node.'
        return cast(RecoveredField[StatementBaseSyntax], self._field(0))
    @property
    def continuations(self) -> tuple[RecoveredField[IStatementConnectionTailSyntax], ...]:
        'Non-empty ordered sequence of continuations components.'
        return cast(tuple[RecoveredField[IStatementConnectionTailSyntax], ...], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('IStatementConnectionSyntax is final')

@final
class PendingIConnectiveSyntax(_SyntaxNode):
    'Product node for statement connective; preserves `i` and `connective` in source order.'
    __slots__ = ()
    _schema_id = 55
    __match_args__ = ('i', 'connective')
    def __new__(cls, i: RecoveredField[Token], connective: RecoveredField[StatementConnectiveSyntax]) -> PendingIConnectiveSyntax:
        return cls._from_fields((i, connective))
    def __init__(self, i: RecoveredField[Token], connective: RecoveredField[StatementConnectiveSyntax]) -> None:
        pass
    @property
    def i(self) -> RecoveredField[Token]:
        'The `I` cmavo marker.'
        return cast(RecoveredField[Token], self._field(0))
    @property
    def connective(self) -> RecoveredField[StatementConnectiveSyntax]:
        'The `statement_connective` connective retained while its following statement remains pending.'
        return cast(RecoveredField[StatementConnectiveSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('PendingIConnectiveSyntax is final')

@final
class IStatementConnectionTailSyntaxChainedIConnectiveStatementTail(_SyntaxNode):
    'Uses the `chained_i_connective_statement_tail` product form, whose payload preserves `pending`, `i`, `connective`, and `trailing_statement`.'
    __slots__ = ()
    _schema_id = 56
    __match_args__ = ('chained_i_connective_statement_tail',)
    def __new__(cls, chained_i_connective_statement_tail: RecoveredField[ChainedIConnectiveStatementTailSyntax]) -> IStatementConnectionTailSyntaxChainedIConnectiveStatementTail:
        return cls._from_fields((chained_i_connective_statement_tail,))
    def __init__(self, chained_i_connective_statement_tail: RecoveredField[ChainedIConnectiveStatementTailSyntax]) -> None:
        pass
    @property
    def chained_i_connective_statement_tail(self) -> RecoveredField[ChainedIConnectiveStatementTailSyntax]:
        'Uses the `chained_i_connective_statement_tail` product form, whose payload preserves `pending`, `i`, `connective`, and `trailing_statement`.'
        return cast(RecoveredField[ChainedIConnectiveStatementTailSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('IStatementConnectionTailSyntaxChainedIConnectiveStatementTail is final')

@final
class IStatementConnectionTailSyntaxSimpleIConnectiveStatementTail(_SyntaxNode):
    'Uses the `simple_i_connective_statement_tail` product form, whose payload preserves `i`, `connective`, and `trailing_statement`.'
    __slots__ = ()
    _schema_id = 57
    __match_args__ = ('simple_i_connective_statement_tail',)
    def __new__(cls, simple_i_connective_statement_tail: RecoveredField[SimpleIConnectiveStatementTailSyntax]) -> IStatementConnectionTailSyntaxSimpleIConnectiveStatementTail:
        return cls._from_fields((simple_i_connective_statement_tail,))
    def __init__(self, simple_i_connective_statement_tail: RecoveredField[SimpleIConnectiveStatementTailSyntax]) -> None:
        pass
    @property
    def simple_i_connective_statement_tail(self) -> RecoveredField[SimpleIConnectiveStatementTailSyntax]:
        'Uses the `simple_i_connective_statement_tail` product form, whose payload preserves `i`, `connective`, and `trailing_statement`.'
        return cast(RecoveredField[SimpleIConnectiveStatementTailSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('IStatementConnectionTailSyntaxSimpleIConnectiveStatementTail is final')

IStatementConnectionTailSyntax: TypeAlias = IStatementConnectionTailSyntaxChainedIConnectiveStatementTail | IStatementConnectionTailSyntaxSimpleIConnectiveStatementTail

@final
class ChainedIConnectiveStatementTailSyntax(_SyntaxNode):
    'Product node for statement connection; preserves `pending`, `i`, `connective`, and `trailing_statement` in source order.'
    __slots__ = ()
    _schema_id = 58
    __match_args__ = ('pending', 'i', 'connective', 'trailing_statement')
    def __new__(cls, pending: Sequence[RecoveredField[PendingIConnectiveSyntax]], i: RecoveredField[Token], connective: RecoveredField[IStatementConnectiveSyntax], trailing_statement: RecoveredField[StatementAfterIConnectiveSyntax]) -> ChainedIConnectiveStatementTailSyntax:
        return cls._from_fields((pending, i, connective, trailing_statement))
    def __init__(self, pending: Sequence[RecoveredField[PendingIConnectiveSyntax]], i: RecoveredField[Token], connective: RecoveredField[IStatementConnectiveSyntax], trailing_statement: RecoveredField[StatementAfterIConnectiveSyntax]) -> None:
        pass
    @property
    def pending(self) -> tuple[RecoveredField[PendingIConnectiveSyntax], ...]:
        'Non-empty ordered sequence of pending components.'
        return cast(tuple[RecoveredField[PendingIConnectiveSyntax], ...], self._field(0))
    @property
    def i(self) -> RecoveredField[Token]:
        'The `I` cmavo marker.'
        return cast(RecoveredField[Token], self._field(1))
    @property
    def connective(self) -> RecoveredField[IStatementConnectiveSyntax]:
        'The `i_statement_connective` connective joining the adjacent constituents of the `chained_i_connective_statement_tail` production.'
        return cast(RecoveredField[IStatementConnectiveSyntax], self._field(2))
    @property
    def trailing_statement(self) -> RecoveredField[StatementAfterIConnectiveSyntax]:
        'The shared trailing statement child syntax node.'
        return cast(RecoveredField[StatementAfterIConnectiveSyntax], self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('ChainedIConnectiveStatementTailSyntax is final')

@final
class SimpleIConnectiveStatementTailSyntax(_SyntaxNode):
    'Product node for statement connection; preserves `i`, `connective`, and `trailing_statement` in source order.'
    __slots__ = ()
    _schema_id = 59
    __match_args__ = ('i', 'connective', 'trailing_statement')
    def __new__(cls, i: RecoveredField[Token], connective: RecoveredField[IStatementConnectiveSyntax], trailing_statement: RecoveredField[StatementAfterIConnectiveSyntax]) -> SimpleIConnectiveStatementTailSyntax:
        return cls._from_fields((i, connective, trailing_statement))
    def __init__(self, i: RecoveredField[Token], connective: RecoveredField[IStatementConnectiveSyntax], trailing_statement: RecoveredField[StatementAfterIConnectiveSyntax]) -> None:
        pass
    @property
    def i(self) -> RecoveredField[Token]:
        'The `I` cmavo marker.'
        return cast(RecoveredField[Token], self._field(0))
    @property
    def connective(self) -> RecoveredField[IStatementConnectiveSyntax]:
        'The `i_statement_connective` connective joining the adjacent constituents of the `simple_i_connective_statement_tail` production.'
        return cast(RecoveredField[IStatementConnectiveSyntax], self._field(1))
    @property
    def trailing_statement(self) -> RecoveredField[StatementAfterIConnectiveSyntax]:
        'The shared trailing statement child syntax node.'
        return cast(RecoveredField[StatementAfterIConnectiveSyntax], self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleIConnectiveStatementTailSyntax is final')

@final
class PreposedIStatementConnectionSyntax(_SyntaxNode):
    'Product node for statement connection; preserves `leading_statement`, `connective`, `i`, and `trailing_statement` in source order.'
    __slots__ = ()
    _schema_id = 60
    __match_args__ = ('leading_statement', 'connective', 'i', 'trailing_statement')
    def __new__(cls, leading_statement: RecoveredField[StatementBaseSyntax], connective: RecoveredField[StatementConnectiveSyntax], i: RecoveredField[Token], trailing_statement: RecoveredField[StatementAfterIConnectiveSyntax]) -> PreposedIStatementConnectionSyntax:
        return cls._from_fields((leading_statement, connective, i, trailing_statement))
    def __init__(self, leading_statement: RecoveredField[StatementBaseSyntax], connective: RecoveredField[StatementConnectiveSyntax], i: RecoveredField[Token], trailing_statement: RecoveredField[StatementAfterIConnectiveSyntax]) -> None:
        pass
    @property
    def leading_statement(self) -> RecoveredField[StatementBaseSyntax]:
        'The shared leading statement child syntax node.'
        return cast(RecoveredField[StatementBaseSyntax], self._field(0))
    @property
    def connective(self) -> RecoveredField[StatementConnectiveSyntax]:
        'The `statement_connective` connective joining the adjacent constituents of the `preposed_i_statement_connection` production.'
        return cast(RecoveredField[StatementConnectiveSyntax], self._field(1))
    @property
    def i(self) -> RecoveredField[Token]:
        'The `I` cmavo marker.'
        return cast(RecoveredField[Token], self._field(2))
    @property
    def trailing_statement(self) -> RecoveredField[StatementAfterIConnectiveSyntax]:
        'The shared trailing statement child syntax node.'
        return cast(RecoveredField[StatementAfterIConnectiveSyntax], self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('PreposedIStatementConnectionSyntax is final')

@final
class TextGroupStatementSyntax(_SyntaxNode):
    'Product node for text group; preserves `tense_modal`, `tuhe`, `text`, and `tuhu` in source order.'
    __slots__ = ()
    _schema_id = 61
    __match_args__ = ('tense_modal', 'tuhe', 'text', 'tuhu')
    def __new__(cls, tense_modal: RecoveredField[TenseModalSyntax] | None, tuhe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], text: RecoveredField[TextSyntax], tuhu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> TextGroupStatementSyntax:
        return cls._from_fields((tense_modal, tuhe, text, tuhu))
    def __init__(self, tense_modal: RecoveredField[TenseModalSyntax] | None, tuhe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], text: RecoveredField[TextSyntax], tuhu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def tense_modal(self) -> RecoveredField[TenseModalSyntax] | None:
        'The optional tense modal component.'
        return cast(RecoveredField[TenseModalSyntax] | None, self._field(0))
    @property
    def tuhe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Tuhe` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(1))
    @property
    def text(self) -> RecoveredField[TextSyntax]:
        'The shared text child syntax node.'
        return cast(RecoveredField[TextSyntax], self._field(2))
    @property
    def tuhu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Tuhu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('TextGroupStatementSyntax is final')

@final
class PrenexFragmentSyntax(_SyntaxNode):
    'Product node for prenex; preserves `terms` and `zohu` in source order.'
    __slots__ = ()
    _schema_id = 62
    __match_args__ = ('terms', 'zohu')
    def __new__(cls, terms: Sequence[RecoveredField[TermSyntax]], zohu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> PrenexFragmentSyntax:
        return cls._from_fields((terms, zohu))
    def __init__(self, terms: Sequence[RecoveredField[TermSyntax]], zohu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def terms(self) -> tuple[RecoveredField[TermSyntax], ...]:
        'Ordered sequence of zero or more terms components.'
        return cast(tuple[RecoveredField[TermSyntax], ...], self._field(0))
    @property
    def zohu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Zohu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('PrenexFragmentSyntax is final')

@final
class PrenexStatementSyntax(_SyntaxNode):
    'Product node for prenex; preserves `prenex_terms`, `zohu`, and `inner_statement` in source order.'
    __slots__ = ()
    _schema_id = 63
    __match_args__ = ('prenex_terms', 'zohu', 'inner_statement')
    def __new__(cls, prenex_terms: Sequence[RecoveredField[TermSyntax]], zohu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_statement: RecoveredField[StatementSyntax]) -> PrenexStatementSyntax:
        return cls._from_fields((prenex_terms, zohu, inner_statement))
    def __init__(self, prenex_terms: Sequence[RecoveredField[TermSyntax]], zohu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_statement: RecoveredField[StatementSyntax]) -> None:
        pass
    @property
    def prenex_terms(self) -> tuple[RecoveredField[TermSyntax], ...]:
        'Ordered sequence of zero or more prenex terms components.'
        return cast(tuple[RecoveredField[TermSyntax], ...], self._field(0))
    @property
    def zohu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Zohu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(1))
    @property
    def inner_statement(self) -> RecoveredField[StatementSyntax]:
        'The shared inner statement child syntax node.'
        return cast(RecoveredField[StatementSyntax], self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('PrenexStatementSyntax is final')

@final
class ForethoughtStatementSyntax(_SyntaxNode):
    'Product node for statement; preserves `gek`, `first`, `first_branch`, `additional_branches`, and `gihi` in source order.'
    __slots__ = ()
    _schema_id = 64
    __match_args__ = ('gek', 'first', 'first_branch', 'additional_branches', 'gihi')
    def __new__(cls, gek: RecoveredField[ModalForethoughtConnectiveSyntax], first: RecoveredField[StatementSyntax], first_branch: RecoveredField[ForethoughtStatementBranchSyntax], additional_branches: Sequence[RecoveredField[ZantufaForethoughtStatementBranchSyntax]], gihi: RecoveredField[Token] | None) -> ForethoughtStatementSyntax:
        return cls._from_fields((gek, first, first_branch, additional_branches, gihi))
    def __init__(self, gek: RecoveredField[ModalForethoughtConnectiveSyntax], first: RecoveredField[StatementSyntax], first_branch: RecoveredField[ForethoughtStatementBranchSyntax], additional_branches: Sequence[RecoveredField[ZantufaForethoughtStatementBranchSyntax]], gihi: RecoveredField[Token] | None) -> None:
        pass
    @property
    def gek(self) -> RecoveredField[ModalForethoughtConnectiveSyntax]:
        'The forethought connective that opens the statement and determines how its branches combine.'
        return cast(RecoveredField[ModalForethoughtConnectiveSyntax], self._field(0))
    @property
    def first(self) -> RecoveredField[StatementSyntax]:
        'The first statement branch, which appears immediately after the opening forethought connective.'
        return cast(RecoveredField[StatementSyntax], self._field(1))
    @property
    def first_branch(self) -> RecoveredField[ForethoughtStatementBranchSyntax]:
        'The first GIK connective together with the statement branch that follows it.'
        return cast(RecoveredField[ForethoughtStatementBranchSyntax], self._field(2))
    @property
    def additional_branches(self) -> tuple[RecoveredField[ZantufaForethoughtStatementBranchSyntax], ...]:
        'Additional Zantufa GIK-led statement branches in their source order.'
        return cast(tuple[RecoveredField[ZantufaForethoughtStatementBranchSyntax], ...], self._field(3))
    @property
    def gihi(self) -> RecoveredField[Token] | None:
        'The optional experimental GIhI terminator following all statement branches.'
        return cast(RecoveredField[Token] | None, self._field(4))
    def __init_subclass__(cls) -> None:
        raise TypeError('ForethoughtStatementSyntax is final')

@final
class ForethoughtStatementBranchSyntax(_SyntaxNode):
    'Product node for statement branch; preserves `gik` and `statement` in source order.'
    __slots__ = ()
    _schema_id = 65
    __match_args__ = ('gik', 'statement')
    def __new__(cls, gik: RecoveredField[GikConnectiveSyntax], statement: RecoveredField[StatementSyntax]) -> ForethoughtStatementBranchSyntax:
        return cls._from_fields((gik, statement))
    def __init__(self, gik: RecoveredField[GikConnectiveSyntax], statement: RecoveredField[StatementSyntax]) -> None:
        pass
    @property
    def gik(self) -> RecoveredField[GikConnectiveSyntax]:
        'The GI-family `gik_connective` connective separating the forethought branches of the `forethought_statement_branch` production.'
        return cast(RecoveredField[GikConnectiveSyntax], self._field(0))
    @property
    def statement(self) -> RecoveredField[StatementSyntax]:
        'The shared statement child syntax node.'
        return cast(RecoveredField[StatementSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ForethoughtStatementBranchSyntax is final')

@final
class ZantufaForethoughtStatementBranchSyntax(_SyntaxNode):
    'Product node for statement branch; preserves `gik` and `statement` in source order.'
    __slots__ = ()
    _schema_id = 66
    __match_args__ = ('gik', 'statement')
    def __new__(cls, gik: RecoveredField[ZantufaExtraGikConnectiveSyntax], statement: RecoveredField[StatementSyntax]) -> ZantufaForethoughtStatementBranchSyntax:
        return cls._from_fields((gik, statement))
    def __init__(self, gik: RecoveredField[ZantufaExtraGikConnectiveSyntax], statement: RecoveredField[StatementSyntax]) -> None:
        pass
    @property
    def gik(self) -> RecoveredField[ZantufaExtraGikConnectiveSyntax]:
        'The GI-family `zantufa_extra_gik_connective` connective separating the forethought branches of the `zantufa_forethought_statement_branch` production.'
        return cast(RecoveredField[ZantufaExtraGikConnectiveSyntax], self._field(0))
    @property
    def statement(self) -> RecoveredField[StatementSyntax]:
        'The shared statement child syntax node.'
        return cast(RecoveredField[StatementSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaForethoughtStatementBranchSyntax is final')

@final
class BridiStatementSyntax(_SyntaxNode):
    'Product node for statement; preserves `bridi` and `continuations` in source order.'
    __slots__ = ()
    _schema_id = 67
    __match_args__ = ('bridi', 'continuations')
    def __new__(cls, bridi: RecoveredField[BridiSyntax], continuations: Sequence[RecoveredField[BridiStatementContinuationSyntax]]) -> BridiStatementSyntax:
        return cls._from_fields((bridi, continuations))
    def __init__(self, bridi: RecoveredField[BridiSyntax], continuations: Sequence[RecoveredField[BridiStatementContinuationSyntax]]) -> None:
        pass
    @property
    def bridi(self) -> RecoveredField[BridiSyntax]:
        'The shared bridi child syntax node.'
        return cast(RecoveredField[BridiSyntax], self._field(0))
    @property
    def continuations(self) -> tuple[RecoveredField[BridiStatementContinuationSyntax], ...]:
        'Ordered sequence of zero or more continuations components.'
        return cast(tuple[RecoveredField[BridiStatementContinuationSyntax], ...], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('BridiStatementSyntax is final')

@final
class BridiStatementContinuationSyntaxBoBridiStatementContinuation(_SyntaxNode):
    'Uses the `bo_bridi_statement_continuation` product form, whose payload preserves `connective`, `tense_modal`, `bo`, and `trailing_subbridi`.'
    __slots__ = ()
    _schema_id = 68
    __match_args__ = ('bo_bridi_statement_continuation',)
    def __new__(cls, bo_bridi_statement_continuation: RecoveredField[BoBridiStatementContinuationSyntax]) -> BridiStatementContinuationSyntaxBoBridiStatementContinuation:
        return cls._from_fields((bo_bridi_statement_continuation,))
    def __init__(self, bo_bridi_statement_continuation: RecoveredField[BoBridiStatementContinuationSyntax]) -> None:
        pass
    @property
    def bo_bridi_statement_continuation(self) -> RecoveredField[BoBridiStatementContinuationSyntax]:
        'Uses the `bo_bridi_statement_continuation` product form, whose payload preserves `connective`, `tense_modal`, `bo`, and `trailing_subbridi`.'
        return cast(RecoveredField[BoBridiStatementContinuationSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('BridiStatementContinuationSyntaxBoBridiStatementContinuation is final')

@final
class BridiStatementContinuationSyntaxKeBridiStatementContinuation(_SyntaxNode):
    'Uses the `ke_bridi_statement_continuation` product form, whose payload preserves `connective`, `tense_modal`, `ke`, `trailing_subbridi`, and `kehe`.'
    __slots__ = ()
    _schema_id = 69
    __match_args__ = ('ke_bridi_statement_continuation',)
    def __new__(cls, ke_bridi_statement_continuation: RecoveredField[KeBridiStatementContinuationSyntax]) -> BridiStatementContinuationSyntaxKeBridiStatementContinuation:
        return cls._from_fields((ke_bridi_statement_continuation,))
    def __init__(self, ke_bridi_statement_continuation: RecoveredField[KeBridiStatementContinuationSyntax]) -> None:
        pass
    @property
    def ke_bridi_statement_continuation(self) -> RecoveredField[KeBridiStatementContinuationSyntax]:
        'Uses the `ke_bridi_statement_continuation` product form, whose payload preserves `connective`, `tense_modal`, `ke`, `trailing_subbridi`, and `kehe`.'
        return cast(RecoveredField[KeBridiStatementContinuationSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('BridiStatementContinuationSyntaxKeBridiStatementContinuation is final')

BridiStatementContinuationSyntax: TypeAlias = BridiStatementContinuationSyntaxBoBridiStatementContinuation | BridiStatementContinuationSyntaxKeBridiStatementContinuation

@final
class BoBridiStatementContinuationSyntax(_SyntaxNode):
    'Product node for bridi continuation; preserves `connective`, `tense_modal`, `bo`, and `trailing_subbridi` in source order.'
    __slots__ = ()
    _schema_id = 70
    __match_args__ = ('connective', 'tense_modal', 'bo', 'trailing_subbridi')
    def __new__(cls, connective: RecoveredField[BridiTailConnectiveSyntax], tense_modal: RecoveredField[TenseModalSyntax] | None, bo: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], trailing_subbridi: RecoveredField[SubbridiSyntax]) -> BoBridiStatementContinuationSyntax:
        return cls._from_fields((connective, tense_modal, bo, trailing_subbridi))
    def __init__(self, connective: RecoveredField[BridiTailConnectiveSyntax], tense_modal: RecoveredField[TenseModalSyntax] | None, bo: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], trailing_subbridi: RecoveredField[SubbridiSyntax]) -> None:
        pass
    @property
    def connective(self) -> RecoveredField[BridiTailConnectiveSyntax]:
        'The `bridi_tail_connective` connective joining the adjacent constituents of the `bo_bridi_statement_continuation` production.'
        return cast(RecoveredField[BridiTailConnectiveSyntax], self._field(0))
    @property
    def tense_modal(self) -> RecoveredField[TenseModalSyntax] | None:
        'The optional tense modal component.'
        return cast(RecoveredField[TenseModalSyntax] | None, self._field(1))
    @property
    def bo(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Bo` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(2))
    @property
    def trailing_subbridi(self) -> RecoveredField[SubbridiSyntax]:
        'The shared trailing subbridi child syntax node.'
        return cast(RecoveredField[SubbridiSyntax], self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('BoBridiStatementContinuationSyntax is final')

@final
class KeBridiStatementContinuationSyntax(_SyntaxNode):
    'Product node for bridi continuation; preserves `connective`, `tense_modal`, `ke`, `trailing_subbridi`, and `kehe` in source order.'
    __slots__ = ()
    _schema_id = 71
    __match_args__ = ('connective', 'tense_modal', 'ke', 'trailing_subbridi', 'kehe')
    def __new__(cls, connective: RecoveredField[RelationAfterthoughtConnectiveSyntax], tense_modal: RecoveredField[TenseModalSyntax] | None, ke: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], trailing_subbridi: RecoveredField[SubbridiSyntax], kehe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> KeBridiStatementContinuationSyntax:
        return cls._from_fields((connective, tense_modal, ke, trailing_subbridi, kehe))
    def __init__(self, connective: RecoveredField[RelationAfterthoughtConnectiveSyntax], tense_modal: RecoveredField[TenseModalSyntax] | None, ke: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], trailing_subbridi: RecoveredField[SubbridiSyntax], kehe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def connective(self) -> RecoveredField[RelationAfterthoughtConnectiveSyntax]:
        'The `relation_afterthought_connective` connective joining the adjacent constituents of the `ke_bridi_statement_continuation` production.'
        return cast(RecoveredField[RelationAfterthoughtConnectiveSyntax], self._field(0))
    @property
    def tense_modal(self) -> RecoveredField[TenseModalSyntax] | None:
        'The optional tense modal component.'
        return cast(RecoveredField[TenseModalSyntax] | None, self._field(1))
    @property
    def ke(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Ke` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(2))
    @property
    def trailing_subbridi(self) -> RecoveredField[SubbridiSyntax]:
        'The shared trailing subbridi child syntax node.'
        return cast(RecoveredField[SubbridiSyntax], self._field(3))
    @property
    def kehe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Kehe` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(4))
    def __init_subclass__(cls) -> None:
        raise TypeError('KeBridiStatementContinuationSyntax is final')

@final
class SelbriFragmentSyntax(_SyntaxNode):
    'Transparent product node for selbri; preserves the `selbri` component.'
    __slots__ = ()
    _schema_id = 72
    __match_args__ = ('selbri',)
    def __new__(cls, selbri: RecoveredField[SelbriSyntax]) -> SelbriFragmentSyntax:
        return cls._from_fields((selbri,))
    def __init__(self, selbri: RecoveredField[SelbriSyntax]) -> None:
        pass
    @property
    def selbri(self) -> RecoveredField[SelbriSyntax]:
        'The shared selbri child syntax node.'
        return cast(RecoveredField[SelbriSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SelbriFragmentSyntax is final')

@final
class TermsFragmentSyntax(_SyntaxNode):
    'Product node for terms; preserves `terms` and `vau` in source order.'
    __slots__ = ()
    _schema_id = 73
    __match_args__ = ('terms', 'vau')
    def __new__(cls, terms: Sequence[RecoveredField[TermSyntax]], vau: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> TermsFragmentSyntax:
        return cls._from_fields((terms, vau))
    def __init__(self, terms: Sequence[RecoveredField[TermSyntax]], vau: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def terms(self) -> tuple[RecoveredField[TermSyntax], ...]:
        'Non-empty ordered sequence of terms components.'
        return cast(tuple[RecoveredField[TermSyntax], ...], self._field(0))
    @property
    def vau(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Vau` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('TermsFragmentSyntax is final')

@final
class MeksoFragmentSyntax(_SyntaxNode):
    'Transparent product node for mex; preserves the `quantifier` component.'
    __slots__ = ()
    _schema_id = 74
    __match_args__ = ('quantifier',)
    def __new__(cls, quantifier: RecoveredField[QuantifierSyntax]) -> MeksoFragmentSyntax:
        return cls._from_fields((quantifier,))
    def __init__(self, quantifier: RecoveredField[QuantifierSyntax]) -> None:
        pass
    @property
    def quantifier(self) -> RecoveredField[QuantifierSyntax]:
        'The shared quantifier child syntax node.'
        return cast(RecoveredField[QuantifierSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('MeksoFragmentSyntax is final')

@final
class ZantufaMeksoFragmentSyntax(_SyntaxNode):
    'Transparent product node for mex; preserves the `expression` component.'
    __slots__ = ()
    _schema_id = 75
    __match_args__ = ('expression',)
    def __new__(cls, expression: RecoveredField[MeksoSyntax]) -> ZantufaMeksoFragmentSyntax:
        return cls._from_fields((expression,))
    def __init__(self, expression: RecoveredField[MeksoSyntax]) -> None:
        pass
    @property
    def expression(self) -> RecoveredField[MeksoSyntax]:
        'The shared expression child syntax node.'
        return cast(RecoveredField[MeksoSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaMeksoFragmentSyntax is final')

@final
class RelativeClauseListSyntax(_SyntaxNode):
    'Product node for relative clauses; preserves `first` and `additional` in source order.'
    __slots__ = ()
    _schema_id = 76
    __match_args__ = ('first', 'additional')
    def __new__(cls, first: RecoveredField[RelativeClauseAtomSyntax], additional: Sequence[RecoveredField[RelativeClauseTailSyntax]]) -> RelativeClauseListSyntax:
        return cls._from_fields((first, additional))
    def __init__(self, first: RecoveredField[RelativeClauseAtomSyntax], additional: Sequence[RecoveredField[RelativeClauseTailSyntax]]) -> None:
        pass
    @property
    def first(self) -> RecoveredField[RelativeClauseAtomSyntax]:
        'The initial `relative_clause_atom` constituent before the continuations of the `relative_clause_list` production.'
        return cast(RecoveredField[RelativeClauseAtomSyntax], self._field(0))
    @property
    def additional(self) -> tuple[RecoveredField[RelativeClauseTailSyntax], ...]:
        'Ordered sequence of zero or more additional components.'
        return cast(tuple[RecoveredField[RelativeClauseTailSyntax], ...], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('RelativeClauseListSyntax is final')

@final
class RelativeClauseFragmentSyntax(_SyntaxNode):
    'Transparent product node for relative clauses; preserves the `relative_clauses` component.'
    __slots__ = ()
    _schema_id = 77
    __match_args__ = ('relative_clauses',)
    def __new__(cls, relative_clauses: RecoveredField[RelativeClauseListSyntax]) -> RelativeClauseFragmentSyntax:
        return cls._from_fields((relative_clauses,))
    def __init__(self, relative_clauses: RecoveredField[RelativeClauseListSyntax]) -> None:
        pass
    @property
    def relative_clauses(self) -> RecoveredField[RelativeClauseListSyntax]:
        'The `relative_clause_list` grammar result in the `relative_clauses` structural role of the `relative_clause_fragment` production.'
        return cast(RecoveredField[RelativeClauseListSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('RelativeClauseFragmentSyntax is final')

@final
class LinkedSumtiContinuationFragmentSyntax(_SyntaxNode):
    'Transparent product node for linked arguments; preserves the `bei_links` component.'
    __slots__ = ()
    _schema_id = 78
    __match_args__ = ('bei_links',)
    def __new__(cls, bei_links: Sequence[RecoveredField[BeiLinkSyntax]]) -> LinkedSumtiContinuationFragmentSyntax:
        return cls._from_fields((bei_links,))
    def __init__(self, bei_links: Sequence[RecoveredField[BeiLinkSyntax]]) -> None:
        pass
    @property
    def bei_links(self) -> tuple[RecoveredField[BeiLinkSyntax], ...]:
        'Non-empty ordered sequence of bei links components.'
        return cast(tuple[RecoveredField[BeiLinkSyntax], ...], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('LinkedSumtiContinuationFragmentSyntax is final')

@final
class LinkedSumtiFragmentSyntax(_SyntaxNode):
    'Transparent product node for linked arguments; preserves the `linkargs` component.'
    __slots__ = ()
    _schema_id = 79
    __match_args__ = ('linkargs',)
    def __new__(cls, linkargs: RecoveredField[LinkargsSyntax]) -> LinkedSumtiFragmentSyntax:
        return cls._from_fields((linkargs,))
    def __init__(self, linkargs: RecoveredField[LinkargsSyntax]) -> None:
        pass
    @property
    def linkargs(self) -> RecoveredField[LinkargsSyntax]:
        'The `linkargs` grammar result in the `linkargs` structural role of the `linked_sumti_fragment` production.'
        return cast(RecoveredField[LinkargsSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('LinkedSumtiFragmentSyntax is final')

@final
class BridiSyntaxBridiWithLeadingTerms(_SyntaxNode):
    'Uses the `bridi_with_leading_terms` product form, whose payload preserves `leading_terms`, `cu`, and `bridi_tail`.'
    __slots__ = ()
    _schema_id = 80
    __match_args__ = ('bridi_with_leading_terms',)
    def __new__(cls, bridi_with_leading_terms: RecoveredField[BridiWithLeadingTermsSyntax]) -> BridiSyntaxBridiWithLeadingTerms:
        return cls._from_fields((bridi_with_leading_terms,))
    def __init__(self, bridi_with_leading_terms: RecoveredField[BridiWithLeadingTermsSyntax]) -> None:
        pass
    @property
    def bridi_with_leading_terms(self) -> RecoveredField[BridiWithLeadingTermsSyntax]:
        'Uses the `bridi_with_leading_terms` product form, whose payload preserves `leading_terms`, `cu`, and `bridi_tail`.'
        return cast(RecoveredField[BridiWithLeadingTermsSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('BridiSyntaxBridiWithLeadingTerms is final')

@final
class BridiSyntaxBridiWithPostCuTerms(_SyntaxNode):
    'Uses the `bridi_with_post_cu_terms` product form, whose payload preserves `leading_terms`, `cu`, and `bridi_tail`.'
    __slots__ = ()
    _schema_id = 81
    __match_args__ = ('bridi_with_post_cu_terms',)
    def __new__(cls, bridi_with_post_cu_terms: RecoveredField[BridiWithPostCuTermsSyntax]) -> BridiSyntaxBridiWithPostCuTerms:
        return cls._from_fields((bridi_with_post_cu_terms,))
    def __init__(self, bridi_with_post_cu_terms: RecoveredField[BridiWithPostCuTermsSyntax]) -> None:
        pass
    @property
    def bridi_with_post_cu_terms(self) -> RecoveredField[BridiWithPostCuTermsSyntax]:
        'Uses the `bridi_with_post_cu_terms` product form, whose payload preserves `leading_terms`, `cu`, and `bridi_tail`.'
        return cast(RecoveredField[BridiWithPostCuTermsSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('BridiSyntaxBridiWithPostCuTerms is final')

@final
class BridiSyntaxBareCuBridi(_SyntaxNode):
    'Uses the `bare_cu_bridi` product form, whose payload preserves `cu` and `bridi_tail`.'
    __slots__ = ()
    _schema_id = 82
    __match_args__ = ('bare_cu_bridi',)
    def __new__(cls, bare_cu_bridi: RecoveredField[BareCuBridiSyntax]) -> BridiSyntaxBareCuBridi:
        return cls._from_fields((bare_cu_bridi,))
    def __init__(self, bare_cu_bridi: RecoveredField[BareCuBridiSyntax]) -> None:
        pass
    @property
    def bare_cu_bridi(self) -> RecoveredField[BareCuBridiSyntax]:
        'Uses the `bare_cu_bridi` product form, whose payload preserves `cu` and `bridi_tail`.'
        return cast(RecoveredField[BareCuBridiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('BridiSyntaxBareCuBridi is final')

@final
class BridiSyntaxBareCuTermsBridi(_SyntaxNode):
    'Uses the `bare_cu_terms_bridi` product form, whose payload preserves `cu` and `bridi_tail`.'
    __slots__ = ()
    _schema_id = 83
    __match_args__ = ('bare_cu_terms_bridi',)
    def __new__(cls, bare_cu_terms_bridi: RecoveredField[BareCuTermsBridiSyntax]) -> BridiSyntaxBareCuTermsBridi:
        return cls._from_fields((bare_cu_terms_bridi,))
    def __init__(self, bare_cu_terms_bridi: RecoveredField[BareCuTermsBridiSyntax]) -> None:
        pass
    @property
    def bare_cu_terms_bridi(self) -> RecoveredField[BareCuTermsBridiSyntax]:
        'Uses the `bare_cu_terms_bridi` product form, whose payload preserves `cu` and `bridi_tail`.'
        return cast(RecoveredField[BareCuTermsBridiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('BridiSyntaxBareCuTermsBridi is final')

@final
class BridiSyntaxRelationOnlyBridi(_SyntaxNode):
    'Uses the `relation_only_bridi` product form, whose payload preserves `bridi_tail`.'
    __slots__ = ()
    _schema_id = 84
    __match_args__ = ('relation_only_bridi',)
    def __new__(cls, relation_only_bridi: RecoveredField[RelationOnlyBridiSyntax]) -> BridiSyntaxRelationOnlyBridi:
        return cls._from_fields((relation_only_bridi,))
    def __init__(self, relation_only_bridi: RecoveredField[RelationOnlyBridiSyntax]) -> None:
        pass
    @property
    def relation_only_bridi(self) -> RecoveredField[RelationOnlyBridiSyntax]:
        'Uses the `relation_only_bridi` product form, whose payload preserves `bridi_tail`.'
        return cast(RecoveredField[RelationOnlyBridiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('BridiSyntaxRelationOnlyBridi is final')

BridiSyntax: TypeAlias = BridiSyntaxBridiWithLeadingTerms | BridiSyntaxBridiWithPostCuTerms | BridiSyntaxBareCuBridi | BridiSyntaxBareCuTermsBridi | BridiSyntaxRelationOnlyBridi

@final
class BridiWithLeadingTermsSyntax(_SyntaxNode):
    'Product node for bridi; preserves `leading_terms`, `cu`, and `bridi_tail` in source order.'
    __slots__ = ()
    _schema_id = 85
    __match_args__ = ('leading_terms', 'cu', 'bridi_tail')
    def __new__(cls, leading_terms: Sequence[RecoveredField[TermSyntax]], cu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, bridi_tail: RecoveredField[BridiTailSyntax]) -> BridiWithLeadingTermsSyntax:
        return cls._from_fields((leading_terms, cu, bridi_tail))
    def __init__(self, leading_terms: Sequence[RecoveredField[TermSyntax]], cu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, bridi_tail: RecoveredField[BridiTailSyntax]) -> None:
        pass
    @property
    def leading_terms(self) -> tuple[RecoveredField[TermSyntax], ...]:
        'Non-empty ordered sequence of leading terms components.'
        return cast(tuple[RecoveredField[TermSyntax], ...], self._field(0))
    @property
    def cu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Cu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(1))
    @property
    def bridi_tail(self) -> RecoveredField[BridiTailSyntax]:
        'The shared bridi tail child syntax node.'
        return cast(RecoveredField[BridiTailSyntax], self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('BridiWithLeadingTermsSyntax is final')

@final
class BridiWithPostCuTermsSyntax(_SyntaxNode):
    'Product node for bridi; preserves `leading_terms`, `cu`, and `bridi_tail` in source order.'
    __slots__ = ()
    _schema_id = 86
    __match_args__ = ('leading_terms', 'cu', 'bridi_tail')
    def __new__(cls, leading_terms: Sequence[RecoveredField[TermSyntax]], cu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], bridi_tail: RecoveredField[CuTermsBridiTailSyntax]) -> BridiWithPostCuTermsSyntax:
        return cls._from_fields((leading_terms, cu, bridi_tail))
    def __init__(self, leading_terms: Sequence[RecoveredField[TermSyntax]], cu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], bridi_tail: RecoveredField[CuTermsBridiTailSyntax]) -> None:
        pass
    @property
    def leading_terms(self) -> tuple[RecoveredField[TermSyntax], ...]:
        'Non-empty ordered sequence of leading terms components.'
        return cast(tuple[RecoveredField[TermSyntax], ...], self._field(0))
    @property
    def cu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Cu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(1))
    @property
    def bridi_tail(self) -> RecoveredField[CuTermsBridiTailSyntax]:
        'The shared bridi tail child syntax node.'
        return cast(RecoveredField[CuTermsBridiTailSyntax], self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('BridiWithPostCuTermsSyntax is final')

@final
class BareCuBridiSyntax(_SyntaxNode):
    'Product node for bridi; preserves `cu` and `bridi_tail` in source order.'
    __slots__ = ()
    _schema_id = 87
    __match_args__ = ('cu', 'bridi_tail')
    def __new__(cls, cu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], bridi_tail: RecoveredField[BridiTailSyntax]) -> BareCuBridiSyntax:
        return cls._from_fields((cu, bridi_tail))
    def __init__(self, cu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], bridi_tail: RecoveredField[BridiTailSyntax]) -> None:
        pass
    @property
    def cu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Cu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def bridi_tail(self) -> RecoveredField[BridiTailSyntax]:
        'The shared bridi tail child syntax node.'
        return cast(RecoveredField[BridiTailSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('BareCuBridiSyntax is final')

@final
class BareCuTermsBridiSyntax(_SyntaxNode):
    'Product node for bridi; preserves `cu` and `bridi_tail` in source order.'
    __slots__ = ()
    _schema_id = 88
    __match_args__ = ('cu', 'bridi_tail')
    def __new__(cls, cu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], bridi_tail: RecoveredField[CuTermsBridiTailSyntax]) -> BareCuTermsBridiSyntax:
        return cls._from_fields((cu, bridi_tail))
    def __init__(self, cu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], bridi_tail: RecoveredField[CuTermsBridiTailSyntax]) -> None:
        pass
    @property
    def cu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Cu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def bridi_tail(self) -> RecoveredField[CuTermsBridiTailSyntax]:
        'The shared bridi tail child syntax node.'
        return cast(RecoveredField[CuTermsBridiTailSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('BareCuTermsBridiSyntax is final')

@final
class RelationOnlyBridiSyntax(_SyntaxNode):
    'Transparent product node for bridi; preserves the `bridi_tail` component.'
    __slots__ = ()
    _schema_id = 89
    __match_args__ = ('bridi_tail',)
    def __new__(cls, bridi_tail: RecoveredField[BridiTailSyntax]) -> RelationOnlyBridiSyntax:
        return cls._from_fields((bridi_tail,))
    def __init__(self, bridi_tail: RecoveredField[BridiTailSyntax]) -> None:
        pass
    @property
    def bridi_tail(self) -> RecoveredField[BridiTailSyntax]:
        'The shared bridi tail child syntax node.'
        return cast(RecoveredField[BridiTailSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('RelationOnlyBridiSyntax is final')

@final
class CuTermsBridiTailSyntax(_SyntaxNode):
    'Product node for bridi tail; preserves `terms` and `bridi_tail` in source order.'
    __slots__ = ()
    _schema_id = 90
    __match_args__ = ('terms', 'bridi_tail')
    def __new__(cls, terms: Sequence[RecoveredField[TermSyntax]], bridi_tail: RecoveredField[BridiTailSyntax]) -> CuTermsBridiTailSyntax:
        return cls._from_fields((terms, bridi_tail))
    def __init__(self, terms: Sequence[RecoveredField[TermSyntax]], bridi_tail: RecoveredField[BridiTailSyntax]) -> None:
        pass
    @property
    def terms(self) -> tuple[RecoveredField[TermSyntax], ...]:
        'Non-empty ordered sequence of terms components.'
        return cast(tuple[RecoveredField[TermSyntax], ...], self._field(0))
    @property
    def bridi_tail(self) -> RecoveredField[BridiTailSyntax]:
        'The shared bridi tail child syntax node.'
        return cast(RecoveredField[BridiTailSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('CuTermsBridiTailSyntax is final')

@final
class BridiTailSyntaxZantufaGroupedBridiTail(_SyntaxNode):
    'Uses the `zantufa_grouped_bridi_tail` product form, whose payload preserves `ke`, `bridi_tail`, `kehe`, `tail_terms`, and `vau`.'
    __slots__ = ()
    _schema_id = 91
    __match_args__ = ('zantufa_grouped_bridi_tail',)
    def __new__(cls, zantufa_grouped_bridi_tail: RecoveredField[ZantufaGroupedBridiTailSyntax]) -> BridiTailSyntaxZantufaGroupedBridiTail:
        return cls._from_fields((zantufa_grouped_bridi_tail,))
    def __init__(self, zantufa_grouped_bridi_tail: RecoveredField[ZantufaGroupedBridiTailSyntax]) -> None:
        pass
    @property
    def zantufa_grouped_bridi_tail(self) -> RecoveredField[ZantufaGroupedBridiTailSyntax]:
        'Uses the `zantufa_grouped_bridi_tail` product form, whose payload preserves `ke`, `bridi_tail`, `kehe`, `tail_terms`, and `vau`.'
        return cast(RecoveredField[ZantufaGroupedBridiTailSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('BridiTailSyntaxZantufaGroupedBridiTail is final')

@final
class BridiTailSyntaxBridiTailWithPossibleTailTerms(_SyntaxNode):
    'Uses the `bridi_tail_with_possible_tail_terms` product form, whose payload preserves `first` and `ke_continuation`.'
    __slots__ = ()
    _schema_id = 92
    __match_args__ = ('bridi_tail_with_possible_tail_terms',)
    def __new__(cls, bridi_tail_with_possible_tail_terms: RecoveredField[BridiTailWithPossibleTailTermsSyntax]) -> BridiTailSyntaxBridiTailWithPossibleTailTerms:
        return cls._from_fields((bridi_tail_with_possible_tail_terms,))
    def __init__(self, bridi_tail_with_possible_tail_terms: RecoveredField[BridiTailWithPossibleTailTermsSyntax]) -> None:
        pass
    @property
    def bridi_tail_with_possible_tail_terms(self) -> RecoveredField[BridiTailWithPossibleTailTermsSyntax]:
        'Uses the `bridi_tail_with_possible_tail_terms` product form, whose payload preserves `first` and `ke_continuation`.'
        return cast(RecoveredField[BridiTailWithPossibleTailTermsSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('BridiTailSyntaxBridiTailWithPossibleTailTerms is final')

@final
class BridiTailSyntaxBridiTailWithoutTailTerms(_SyntaxNode):
    'Uses the `bridi_tail_without_tail_terms` product form, whose payload preserves `first` and `ke_continuation`.'
    __slots__ = ()
    _schema_id = 93
    __match_args__ = ('bridi_tail_without_tail_terms',)
    def __new__(cls, bridi_tail_without_tail_terms: RecoveredField[BridiTailWithoutTailTermsSyntax]) -> BridiTailSyntaxBridiTailWithoutTailTerms:
        return cls._from_fields((bridi_tail_without_tail_terms,))
    def __init__(self, bridi_tail_without_tail_terms: RecoveredField[BridiTailWithoutTailTermsSyntax]) -> None:
        pass
    @property
    def bridi_tail_without_tail_terms(self) -> RecoveredField[BridiTailWithoutTailTermsSyntax]:
        'Uses the `bridi_tail_without_tail_terms` product form, whose payload preserves `first` and `ke_continuation`.'
        return cast(RecoveredField[BridiTailWithoutTailTermsSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('BridiTailSyntaxBridiTailWithoutTailTerms is final')

BridiTailSyntax: TypeAlias = BridiTailSyntaxZantufaGroupedBridiTail | BridiTailSyntaxBridiTailWithPossibleTailTerms | BridiTailSyntaxBridiTailWithoutTailTerms

@final
class ZantufaGroupedBridiTailSyntax(_SyntaxNode):
    'Product node for bridi tail; preserves `ke`, `bridi_tail`, `kehe`, `tail_terms`, and `vau` in source order.'
    __slots__ = ()
    _schema_id = 94
    __match_args__ = ('ke', 'bridi_tail', 'kehe', 'tail_terms', 'vau')
    def __new__(cls, ke: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], bridi_tail: RecoveredField[BridiTailSyntax], kehe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, tail_terms: Sequence[RecoveredField[TermSyntax]], vau: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> ZantufaGroupedBridiTailSyntax:
        return cls._from_fields((ke, bridi_tail, kehe, tail_terms, vau))
    def __init__(self, ke: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], bridi_tail: RecoveredField[BridiTailSyntax], kehe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, tail_terms: Sequence[RecoveredField[TermSyntax]], vau: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def ke(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Ke` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def bridi_tail(self) -> RecoveredField[BridiTailSyntax]:
        'The shared bridi tail child syntax node.'
        return cast(RecoveredField[BridiTailSyntax], self._field(1))
    @property
    def kehe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Kehe` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    @property
    def tail_terms(self) -> tuple[RecoveredField[TermSyntax], ...]:
        'Ordered sequence of zero or more tail terms components.'
        return cast(tuple[RecoveredField[TermSyntax], ...], self._field(3))
    @property
    def vau(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Vau` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(4))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaGroupedBridiTailSyntax is final')

@final
class BridiTailWithoutTailTermsSyntax(_SyntaxNode):
    'Product node for bridi tail; preserves `first` and `ke_continuation` in source order.'
    __slots__ = ()
    _schema_id = 95
    __match_args__ = ('first', 'ke_continuation')
    def __new__(cls, first: RecoveredField[AfterthoughtBridiTailWithoutTailTermsSyntax], ke_continuation: RecoveredField[BridiTailKeContinuationSyntax] | None) -> BridiTailWithoutTailTermsSyntax:
        return cls._from_fields((first, ke_continuation))
    def __init__(self, first: RecoveredField[AfterthoughtBridiTailWithoutTailTermsSyntax], ke_continuation: RecoveredField[BridiTailKeContinuationSyntax] | None) -> None:
        pass
    @property
    def first(self) -> RecoveredField[AfterthoughtBridiTailWithoutTailTermsSyntax]:
        'The shared first child syntax node.'
        return cast(RecoveredField[AfterthoughtBridiTailWithoutTailTermsSyntax], self._field(0))
    @property
    def ke_continuation(self) -> RecoveredField[BridiTailKeContinuationSyntax] | None:
        'The optional ke continuation component.'
        return cast(RecoveredField[BridiTailKeContinuationSyntax] | None, self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('BridiTailWithoutTailTermsSyntax is final')

@final
class BridiTailWithPossibleTailTermsSyntax(_SyntaxNode):
    'Product node for bridi tail; preserves `first` and `ke_continuation` in source order.'
    __slots__ = ()
    _schema_id = 96
    __match_args__ = ('first', 'ke_continuation')
    def __new__(cls, first: RecoveredField[AfterthoughtBridiTailSyntax], ke_continuation: RecoveredField[GihekBridiTailKeContinuationSyntax] | None) -> BridiTailWithPossibleTailTermsSyntax:
        return cls._from_fields((first, ke_continuation))
    def __init__(self, first: RecoveredField[AfterthoughtBridiTailSyntax], ke_continuation: RecoveredField[GihekBridiTailKeContinuationSyntax] | None) -> None:
        pass
    @property
    def first(self) -> RecoveredField[AfterthoughtBridiTailSyntax]:
        'The shared first child syntax node.'
        return cast(RecoveredField[AfterthoughtBridiTailSyntax], self._field(0))
    @property
    def ke_continuation(self) -> RecoveredField[GihekBridiTailKeContinuationSyntax] | None:
        'The optional ke continuation component.'
        return cast(RecoveredField[GihekBridiTailKeContinuationSyntax] | None, self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('BridiTailWithPossibleTailTermsSyntax is final')

@final
class AfterthoughtBridiTailWithoutTailTermsSyntax(_SyntaxNode):
    'Transparent product node for bridi tail; preserves the `bridi_tails` component.'
    __slots__ = ()
    _schema_id = 97
    __match_args__ = ('bridi_tails',)
    def __new__(cls, bridi_tails: Chain[RecoveredField[BoGroupedBridiTailWithoutTailTermsSyntax], RecoveredField[BridiTailContinuationWithoutTailTermsSyntax]]) -> AfterthoughtBridiTailWithoutTailTermsSyntax:
        return cls._from_fields((bridi_tails,))
    def __init__(self, bridi_tails: Chain[RecoveredField[BoGroupedBridiTailWithoutTailTermsSyntax], RecoveredField[BridiTailContinuationWithoutTailTermsSyntax]]) -> None:
        pass
    @property
    def bridi_tails(self) -> Chain[RecoveredField[BoGroupedBridiTailWithoutTailTermsSyntax], RecoveredField[BridiTailContinuationWithoutTailTermsSyntax]]:
        'The source-ordered `bridi_tails` chain assembled by the `afterthought_bridi_tail_without_tail_terms` production.'
        return cast(Chain[RecoveredField[BoGroupedBridiTailWithoutTailTermsSyntax], RecoveredField[BridiTailContinuationWithoutTailTermsSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('AfterthoughtBridiTailWithoutTailTermsSyntax is final')

@final
class AfterthoughtBridiTailSyntax(_SyntaxNode):
    'Transparent product node for bridi tail; preserves the `bridi_tails` component.'
    __slots__ = ()
    _schema_id = 98
    __match_args__ = ('bridi_tails',)
    def __new__(cls, bridi_tails: Chain[RecoveredField[BoGroupedBridiTailSyntax], RecoveredField[BridiTailContinuationSyntax]]) -> AfterthoughtBridiTailSyntax:
        return cls._from_fields((bridi_tails,))
    def __init__(self, bridi_tails: Chain[RecoveredField[BoGroupedBridiTailSyntax], RecoveredField[BridiTailContinuationSyntax]]) -> None:
        pass
    @property
    def bridi_tails(self) -> Chain[RecoveredField[BoGroupedBridiTailSyntax], RecoveredField[BridiTailContinuationSyntax]]:
        'The source-ordered `bridi_tails` chain assembled by the `afterthought_bridi_tail` production.'
        return cast(Chain[RecoveredField[BoGroupedBridiTailSyntax], RecoveredField[BridiTailContinuationSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('AfterthoughtBridiTailSyntax is final')

@final
class BoGroupedBridiTailWithoutTailTermsSyntax(_SyntaxNode):
    'Product node for bridi tail; preserves `first` and `bo_continuation` in source order.'
    __slots__ = ()
    _schema_id = 99
    __match_args__ = ('first', 'bo_continuation')
    def __new__(cls, first: RecoveredField[SimpleBridiTailWithoutTailTermsSyntax], bo_continuation: RecoveredField[BridiTailBoContinuationWithoutTailTermsSyntax] | None) -> BoGroupedBridiTailWithoutTailTermsSyntax:
        return cls._from_fields((first, bo_continuation))
    def __init__(self, first: RecoveredField[SimpleBridiTailWithoutTailTermsSyntax], bo_continuation: RecoveredField[BridiTailBoContinuationWithoutTailTermsSyntax] | None) -> None:
        pass
    @property
    def first(self) -> RecoveredField[SimpleBridiTailWithoutTailTermsSyntax]:
        'The shared first child syntax node.'
        return cast(RecoveredField[SimpleBridiTailWithoutTailTermsSyntax], self._field(0))
    @property
    def bo_continuation(self) -> RecoveredField[BridiTailBoContinuationWithoutTailTermsSyntax] | None:
        'The optional bo continuation component.'
        return cast(RecoveredField[BridiTailBoContinuationWithoutTailTermsSyntax] | None, self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('BoGroupedBridiTailWithoutTailTermsSyntax is final')

@final
class BoGroupedBridiTailSyntax(_SyntaxNode):
    'Product node for bridi tail; preserves `first` and `bo_continuation` in source order.'
    __slots__ = ()
    _schema_id = 100
    __match_args__ = ('first', 'bo_continuation')
    def __new__(cls, first: RecoveredField[SimpleBridiTailSyntax], bo_continuation: RecoveredField[BridiTailBoContinuationSyntax] | None) -> BoGroupedBridiTailSyntax:
        return cls._from_fields((first, bo_continuation))
    def __init__(self, first: RecoveredField[SimpleBridiTailSyntax], bo_continuation: RecoveredField[BridiTailBoContinuationSyntax] | None) -> None:
        pass
    @property
    def first(self) -> RecoveredField[SimpleBridiTailSyntax]:
        'The shared first child syntax node.'
        return cast(RecoveredField[SimpleBridiTailSyntax], self._field(0))
    @property
    def bo_continuation(self) -> RecoveredField[BridiTailBoContinuationSyntax] | None:
        'The optional bo continuation component.'
        return cast(RecoveredField[BridiTailBoContinuationSyntax] | None, self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('BoGroupedBridiTailSyntax is final')

@final
class SimpleBridiTailWithoutTailTermsSyntaxForethoughtSimpleBridiTailWithoutTailTerms(_SyntaxNode):
    'Uses the `forethought_simple_bridi_tail_without_tail_terms` product form, whose payload preserves `connection`.'
    __slots__ = ()
    _schema_id = 101
    __match_args__ = ('forethought_simple_bridi_tail_without_tail_terms',)
    def __new__(cls, forethought_simple_bridi_tail_without_tail_terms: RecoveredField[ForethoughtSimpleBridiTailWithoutTailTermsSyntax]) -> SimpleBridiTailWithoutTailTermsSyntaxForethoughtSimpleBridiTailWithoutTailTerms:
        return cls._from_fields((forethought_simple_bridi_tail_without_tail_terms,))
    def __init__(self, forethought_simple_bridi_tail_without_tail_terms: RecoveredField[ForethoughtSimpleBridiTailWithoutTailTermsSyntax]) -> None:
        pass
    @property
    def forethought_simple_bridi_tail_without_tail_terms(self) -> RecoveredField[ForethoughtSimpleBridiTailWithoutTailTermsSyntax]:
        'Uses the `forethought_simple_bridi_tail_without_tail_terms` product form, whose payload preserves `connection`.'
        return cast(RecoveredField[ForethoughtSimpleBridiTailWithoutTailTermsSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleBridiTailWithoutTailTermsSyntaxForethoughtSimpleBridiTailWithoutTailTerms is final')

@final
class SimpleBridiTailWithoutTailTermsSyntaxSelbriSimpleBridiTailWithoutTailTerms(_SyntaxNode):
    'Uses the `selbri_simple_bridi_tail_without_tail_terms` product form, whose payload preserves `selbri` and `vau`.'
    __slots__ = ()
    _schema_id = 102
    __match_args__ = ('selbri_simple_bridi_tail_without_tail_terms',)
    def __new__(cls, selbri_simple_bridi_tail_without_tail_terms: RecoveredField[SelbriSimpleBridiTailWithoutTailTermsSyntax]) -> SimpleBridiTailWithoutTailTermsSyntaxSelbriSimpleBridiTailWithoutTailTerms:
        return cls._from_fields((selbri_simple_bridi_tail_without_tail_terms,))
    def __init__(self, selbri_simple_bridi_tail_without_tail_terms: RecoveredField[SelbriSimpleBridiTailWithoutTailTermsSyntax]) -> None:
        pass
    @property
    def selbri_simple_bridi_tail_without_tail_terms(self) -> RecoveredField[SelbriSimpleBridiTailWithoutTailTermsSyntax]:
        'Uses the `selbri_simple_bridi_tail_without_tail_terms` product form, whose payload preserves `selbri` and `vau`.'
        return cast(RecoveredField[SelbriSimpleBridiTailWithoutTailTermsSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleBridiTailWithoutTailTermsSyntaxSelbriSimpleBridiTailWithoutTailTerms is final')

SimpleBridiTailWithoutTailTermsSyntax: TypeAlias = SimpleBridiTailWithoutTailTermsSyntaxForethoughtSimpleBridiTailWithoutTailTerms | SimpleBridiTailWithoutTailTermsSyntaxSelbriSimpleBridiTailWithoutTailTerms

@final
class SimpleBridiTailSyntaxForethoughtSimpleBridiTail(_SyntaxNode):
    'Uses the `forethought_simple_bridi_tail` product form, whose payload preserves `connection`.'
    __slots__ = ()
    _schema_id = 103
    __match_args__ = ('forethought_simple_bridi_tail',)
    def __new__(cls, forethought_simple_bridi_tail: RecoveredField[ForethoughtSimpleBridiTailSyntax]) -> SimpleBridiTailSyntaxForethoughtSimpleBridiTail:
        return cls._from_fields((forethought_simple_bridi_tail,))
    def __init__(self, forethought_simple_bridi_tail: RecoveredField[ForethoughtSimpleBridiTailSyntax]) -> None:
        pass
    @property
    def forethought_simple_bridi_tail(self) -> RecoveredField[ForethoughtSimpleBridiTailSyntax]:
        'Uses the `forethought_simple_bridi_tail` product form, whose payload preserves `connection`.'
        return cast(RecoveredField[ForethoughtSimpleBridiTailSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleBridiTailSyntaxForethoughtSimpleBridiTail is final')

@final
class SimpleBridiTailSyntaxSelbriSimpleBridiTail(_SyntaxNode):
    'Uses the `selbri_simple_bridi_tail` product form, whose payload preserves `selbri`, `terms`, and `vau`.'
    __slots__ = ()
    _schema_id = 104
    __match_args__ = ('selbri_simple_bridi_tail',)
    def __new__(cls, selbri_simple_bridi_tail: RecoveredField[SelbriSimpleBridiTailSyntax]) -> SimpleBridiTailSyntaxSelbriSimpleBridiTail:
        return cls._from_fields((selbri_simple_bridi_tail,))
    def __init__(self, selbri_simple_bridi_tail: RecoveredField[SelbriSimpleBridiTailSyntax]) -> None:
        pass
    @property
    def selbri_simple_bridi_tail(self) -> RecoveredField[SelbriSimpleBridiTailSyntax]:
        'Uses the `selbri_simple_bridi_tail` product form, whose payload preserves `selbri`, `terms`, and `vau`.'
        return cast(RecoveredField[SelbriSimpleBridiTailSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleBridiTailSyntaxSelbriSimpleBridiTail is final')

SimpleBridiTailSyntax: TypeAlias = SimpleBridiTailSyntaxForethoughtSimpleBridiTail | SimpleBridiTailSyntaxSelbriSimpleBridiTail

@final
class ForethoughtSimpleBridiTailWithoutTailTermsSyntax(_SyntaxNode):
    'Transparent product node for forethought bridi connection; preserves the `connection` component.'
    __slots__ = ()
    _schema_id = 105
    __match_args__ = ('connection',)
    def __new__(cls, connection: RecoveredField[ForethoughtBridiConnectionWithoutTailTermsSyntax]) -> ForethoughtSimpleBridiTailWithoutTailTermsSyntax:
        return cls._from_fields((connection,))
    def __init__(self, connection: RecoveredField[ForethoughtBridiConnectionWithoutTailTermsSyntax]) -> None:
        pass
    @property
    def connection(self) -> RecoveredField[ForethoughtBridiConnectionWithoutTailTermsSyntax]:
        'The shared connection child syntax node.'
        return cast(RecoveredField[ForethoughtBridiConnectionWithoutTailTermsSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ForethoughtSimpleBridiTailWithoutTailTermsSyntax is final')

@final
class ForethoughtSimpleBridiTailSyntax(_SyntaxNode):
    'Transparent product node for forethought bridi connection; preserves the `connection` component.'
    __slots__ = ()
    _schema_id = 106
    __match_args__ = ('connection',)
    def __new__(cls, connection: RecoveredField[ForethoughtBridiConnectionSyntax]) -> ForethoughtSimpleBridiTailSyntax:
        return cls._from_fields((connection,))
    def __init__(self, connection: RecoveredField[ForethoughtBridiConnectionSyntax]) -> None:
        pass
    @property
    def connection(self) -> RecoveredField[ForethoughtBridiConnectionSyntax]:
        'The shared connection child syntax node.'
        return cast(RecoveredField[ForethoughtBridiConnectionSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ForethoughtSimpleBridiTailSyntax is final')

@final
class SelbriSimpleBridiTailWithoutTailTermsSyntax(_SyntaxNode):
    'Product node for bridi tail; preserves `selbri` and `vau` in source order.'
    __slots__ = ()
    _schema_id = 107
    __match_args__ = ('selbri', 'vau')
    def __new__(cls, selbri: RecoveredField[SelbriSyntax], vau: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> SelbriSimpleBridiTailWithoutTailTermsSyntax:
        return cls._from_fields((selbri, vau))
    def __init__(self, selbri: RecoveredField[SelbriSyntax], vau: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def selbri(self) -> RecoveredField[SelbriSyntax]:
        'The shared selbri child syntax node.'
        return cast(RecoveredField[SelbriSyntax], self._field(0))
    @property
    def vau(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Vau` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('SelbriSimpleBridiTailWithoutTailTermsSyntax is final')

@final
class SelbriSimpleBridiTailSyntax(_SyntaxNode):
    'Product node for bridi tail; preserves `selbri`, `terms`, and `vau` in source order.'
    __slots__ = ()
    _schema_id = 108
    __match_args__ = ('selbri', 'terms', 'vau')
    def __new__(cls, selbri: RecoveredField[SelbriSyntax], terms: Sequence[RecoveredField[TermSyntax]], vau: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> SelbriSimpleBridiTailSyntax:
        return cls._from_fields((selbri, terms, vau))
    def __init__(self, selbri: RecoveredField[SelbriSyntax], terms: Sequence[RecoveredField[TermSyntax]], vau: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def selbri(self) -> RecoveredField[SelbriSyntax]:
        'The shared selbri child syntax node.'
        return cast(RecoveredField[SelbriSyntax], self._field(0))
    @property
    def terms(self) -> tuple[RecoveredField[TermSyntax], ...]:
        'Ordered sequence of zero or more terms components.'
        return cast(tuple[RecoveredField[TermSyntax], ...], self._field(1))
    @property
    def vau(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Vau` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('SelbriSimpleBridiTailSyntax is final')

@final
class ForethoughtBridiConnectionSyntaxDirectForethoughtBridiConnection(_SyntaxNode):
    'Uses the `direct_forethought_bridi_connection` product form, whose payload preserves `gek`, `first`, `first_branch`, and 4 other fields.'
    __slots__ = ()
    _schema_id = 109
    __match_args__ = ('direct_forethought_bridi_connection',)
    def __new__(cls, direct_forethought_bridi_connection: RecoveredField[DirectForethoughtBridiConnectionSyntax]) -> ForethoughtBridiConnectionSyntaxDirectForethoughtBridiConnection:
        return cls._from_fields((direct_forethought_bridi_connection,))
    def __init__(self, direct_forethought_bridi_connection: RecoveredField[DirectForethoughtBridiConnectionSyntax]) -> None:
        pass
    @property
    def direct_forethought_bridi_connection(self) -> RecoveredField[DirectForethoughtBridiConnectionSyntax]:
        'Uses the `direct_forethought_bridi_connection` product form, whose payload preserves `gek`, `first`, `first_branch`, and 4 other fields.'
        return cast(RecoveredField[DirectForethoughtBridiConnectionSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ForethoughtBridiConnectionSyntaxDirectForethoughtBridiConnection is final')

@final
class ForethoughtBridiConnectionSyntaxGroupedForethoughtBridiConnection(_SyntaxNode):
    'Uses the `grouped_forethought_bridi_connection` product form, whose payload preserves `tense_modal`, `ke`, `inner`, and `kehe`.'
    __slots__ = ()
    _schema_id = 110
    __match_args__ = ('grouped_forethought_bridi_connection',)
    def __new__(cls, grouped_forethought_bridi_connection: RecoveredField[GroupedForethoughtBridiConnectionSyntax]) -> ForethoughtBridiConnectionSyntaxGroupedForethoughtBridiConnection:
        return cls._from_fields((grouped_forethought_bridi_connection,))
    def __init__(self, grouped_forethought_bridi_connection: RecoveredField[GroupedForethoughtBridiConnectionSyntax]) -> None:
        pass
    @property
    def grouped_forethought_bridi_connection(self) -> RecoveredField[GroupedForethoughtBridiConnectionSyntax]:
        'Uses the `grouped_forethought_bridi_connection` product form, whose payload preserves `tense_modal`, `ke`, `inner`, and `kehe`.'
        return cast(RecoveredField[GroupedForethoughtBridiConnectionSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ForethoughtBridiConnectionSyntaxGroupedForethoughtBridiConnection is final')

@final
class ForethoughtBridiConnectionSyntaxNegatedForethoughtBridiConnection(_SyntaxNode):
    'Uses the `negated_forethought_bridi_connection` product form, whose payload preserves `na` and `inner`.'
    __slots__ = ()
    _schema_id = 111
    __match_args__ = ('negated_forethought_bridi_connection',)
    def __new__(cls, negated_forethought_bridi_connection: RecoveredField[NegatedForethoughtBridiConnectionSyntax]) -> ForethoughtBridiConnectionSyntaxNegatedForethoughtBridiConnection:
        return cls._from_fields((negated_forethought_bridi_connection,))
    def __init__(self, negated_forethought_bridi_connection: RecoveredField[NegatedForethoughtBridiConnectionSyntax]) -> None:
        pass
    @property
    def negated_forethought_bridi_connection(self) -> RecoveredField[NegatedForethoughtBridiConnectionSyntax]:
        'Uses the `negated_forethought_bridi_connection` product form, whose payload preserves `na` and `inner`.'
        return cast(RecoveredField[NegatedForethoughtBridiConnectionSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ForethoughtBridiConnectionSyntaxNegatedForethoughtBridiConnection is final')

ForethoughtBridiConnectionSyntax: TypeAlias = ForethoughtBridiConnectionSyntaxDirectForethoughtBridiConnection | ForethoughtBridiConnectionSyntaxGroupedForethoughtBridiConnection | ForethoughtBridiConnectionSyntaxNegatedForethoughtBridiConnection

@final
class ForethoughtBridiConnectionWithoutTailTermsSyntaxDirectForethoughtBridiConnectionWithoutTailTerms(_SyntaxNode):
    'Uses the `direct_forethought_bridi_connection_without_tail_terms` product form, whose payload preserves `gek`, `first`, `first_branch`, and 3 other fields.'
    __slots__ = ()
    _schema_id = 112
    __match_args__ = ('direct_forethought_bridi_connection_without_tail_terms',)
    def __new__(cls, direct_forethought_bridi_connection_without_tail_terms: RecoveredField[DirectForethoughtBridiConnectionWithoutTailTermsSyntax]) -> ForethoughtBridiConnectionWithoutTailTermsSyntaxDirectForethoughtBridiConnectionWithoutTailTerms:
        return cls._from_fields((direct_forethought_bridi_connection_without_tail_terms,))
    def __init__(self, direct_forethought_bridi_connection_without_tail_terms: RecoveredField[DirectForethoughtBridiConnectionWithoutTailTermsSyntax]) -> None:
        pass
    @property
    def direct_forethought_bridi_connection_without_tail_terms(self) -> RecoveredField[DirectForethoughtBridiConnectionWithoutTailTermsSyntax]:
        'Uses the `direct_forethought_bridi_connection_without_tail_terms` product form, whose payload preserves `gek`, `first`, `first_branch`, and 3 other fields.'
        return cast(RecoveredField[DirectForethoughtBridiConnectionWithoutTailTermsSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ForethoughtBridiConnectionWithoutTailTermsSyntaxDirectForethoughtBridiConnectionWithoutTailTerms is final')

@final
class ForethoughtBridiConnectionWithoutTailTermsSyntaxGroupedForethoughtBridiConnectionWithoutTailTerms(_SyntaxNode):
    'Uses the `grouped_forethought_bridi_connection_without_tail_terms` product form, whose payload preserves `tense_modal`, `ke`, `inner`, and `kehe`.'
    __slots__ = ()
    _schema_id = 113
    __match_args__ = ('grouped_forethought_bridi_connection_without_tail_terms',)
    def __new__(cls, grouped_forethought_bridi_connection_without_tail_terms: RecoveredField[GroupedForethoughtBridiConnectionWithoutTailTermsSyntax]) -> ForethoughtBridiConnectionWithoutTailTermsSyntaxGroupedForethoughtBridiConnectionWithoutTailTerms:
        return cls._from_fields((grouped_forethought_bridi_connection_without_tail_terms,))
    def __init__(self, grouped_forethought_bridi_connection_without_tail_terms: RecoveredField[GroupedForethoughtBridiConnectionWithoutTailTermsSyntax]) -> None:
        pass
    @property
    def grouped_forethought_bridi_connection_without_tail_terms(self) -> RecoveredField[GroupedForethoughtBridiConnectionWithoutTailTermsSyntax]:
        'Uses the `grouped_forethought_bridi_connection_without_tail_terms` product form, whose payload preserves `tense_modal`, `ke`, `inner`, and `kehe`.'
        return cast(RecoveredField[GroupedForethoughtBridiConnectionWithoutTailTermsSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ForethoughtBridiConnectionWithoutTailTermsSyntaxGroupedForethoughtBridiConnectionWithoutTailTerms is final')

@final
class ForethoughtBridiConnectionWithoutTailTermsSyntaxNegatedForethoughtBridiConnectionWithoutTailTerms(_SyntaxNode):
    'Uses the `negated_forethought_bridi_connection_without_tail_terms` product form, whose payload preserves `na` and `inner`.'
    __slots__ = ()
    _schema_id = 114
    __match_args__ = ('negated_forethought_bridi_connection_without_tail_terms',)
    def __new__(cls, negated_forethought_bridi_connection_without_tail_terms: RecoveredField[NegatedForethoughtBridiConnectionWithoutTailTermsSyntax]) -> ForethoughtBridiConnectionWithoutTailTermsSyntaxNegatedForethoughtBridiConnectionWithoutTailTerms:
        return cls._from_fields((negated_forethought_bridi_connection_without_tail_terms,))
    def __init__(self, negated_forethought_bridi_connection_without_tail_terms: RecoveredField[NegatedForethoughtBridiConnectionWithoutTailTermsSyntax]) -> None:
        pass
    @property
    def negated_forethought_bridi_connection_without_tail_terms(self) -> RecoveredField[NegatedForethoughtBridiConnectionWithoutTailTermsSyntax]:
        'Uses the `negated_forethought_bridi_connection_without_tail_terms` product form, whose payload preserves `na` and `inner`.'
        return cast(RecoveredField[NegatedForethoughtBridiConnectionWithoutTailTermsSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ForethoughtBridiConnectionWithoutTailTermsSyntaxNegatedForethoughtBridiConnectionWithoutTailTerms is final')

ForethoughtBridiConnectionWithoutTailTermsSyntax: TypeAlias = ForethoughtBridiConnectionWithoutTailTermsSyntaxDirectForethoughtBridiConnectionWithoutTailTerms | ForethoughtBridiConnectionWithoutTailTermsSyntaxGroupedForethoughtBridiConnectionWithoutTailTerms | ForethoughtBridiConnectionWithoutTailTermsSyntaxNegatedForethoughtBridiConnectionWithoutTailTerms

@final
class DirectForethoughtBridiConnectionSyntax(_SyntaxNode):
    'Product node for forethought bridi connection; preserves `gek`, `first`, `first_branch`, and 4 other fields in source order.'
    __slots__ = ()
    _schema_id = 115
    __match_args__ = ('gek', 'first', 'first_branch', 'additional_branches', 'gihi', 'tail_terms', 'vau')
    def __new__(cls, gek: RecoveredField[ModalForethoughtConnectiveSyntax], first: RecoveredField[SubbridiSyntax], first_branch: RecoveredField[ForethoughtBridiBranchSyntax], additional_branches: Sequence[RecoveredField[ZantufaForethoughtBridiBranchSyntax]], gihi: RecoveredField[Token] | None, tail_terms: Sequence[RecoveredField[TermSyntax]], vau: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> DirectForethoughtBridiConnectionSyntax:
        return cls._from_fields((gek, first, first_branch, additional_branches, gihi, tail_terms, vau))
    def __init__(self, gek: RecoveredField[ModalForethoughtConnectiveSyntax], first: RecoveredField[SubbridiSyntax], first_branch: RecoveredField[ForethoughtBridiBranchSyntax], additional_branches: Sequence[RecoveredField[ZantufaForethoughtBridiBranchSyntax]], gihi: RecoveredField[Token] | None, tail_terms: Sequence[RecoveredField[TermSyntax]], vau: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def gek(self) -> RecoveredField[ModalForethoughtConnectiveSyntax]:
        'The opening forethought connective that determines how the subbridi branches are combined.'
        return cast(RecoveredField[ModalForethoughtConnectiveSyntax], self._field(0))
    @property
    def first(self) -> RecoveredField[SubbridiSyntax]:
        'The first subbridi branch, which follows the opening connective without an intervening GIK.'
        return cast(RecoveredField[SubbridiSyntax], self._field(1))
    @property
    def first_branch(self) -> RecoveredField[ForethoughtBridiBranchSyntax]:
        'The first GIK-led subbridi branch paired with the opening connective.'
        return cast(RecoveredField[ForethoughtBridiBranchSyntax], self._field(2))
    @property
    def additional_branches(self) -> tuple[RecoveredField[ZantufaForethoughtBridiBranchSyntax], ...]:
        'Additional Zantufa GIK-led subbridi branches, retained in source order.'
        return cast(tuple[RecoveredField[ZantufaForethoughtBridiBranchSyntax], ...], self._field(3))
    @property
    def gihi(self) -> RecoveredField[Token] | None:
        'The optional experimental GIhI terminator following the complete branch sequence.'
        return cast(RecoveredField[Token] | None, self._field(4))
    @property
    def tail_terms(self) -> tuple[RecoveredField[TermSyntax], ...]:
        'Terms attached to the completed forethought bridi after its connected subbridi branches.'
        return cast(tuple[RecoveredField[TermSyntax], ...], self._field(5))
    @property
    def vau(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional elidable VAU terminator for the bridi tail.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(6))
    def __init_subclass__(cls) -> None:
        raise TypeError('DirectForethoughtBridiConnectionSyntax is final')

@final
class DirectForethoughtBridiConnectionWithoutTailTermsSyntax(_SyntaxNode):
    'Product node for forethought bridi connection; preserves `gek`, `first`, `first_branch`, and 3 other fields in source order.'
    __slots__ = ()
    _schema_id = 116
    __match_args__ = ('gek', 'first', 'first_branch', 'additional_branches', 'gihi', 'vau')
    def __new__(cls, gek: RecoveredField[ModalForethoughtConnectiveSyntax], first: RecoveredField[SubbridiSyntax], first_branch: RecoveredField[ForethoughtBridiBranchSyntax], additional_branches: Sequence[RecoveredField[ZantufaForethoughtBridiBranchSyntax]], gihi: RecoveredField[Token] | None, vau: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> DirectForethoughtBridiConnectionWithoutTailTermsSyntax:
        return cls._from_fields((gek, first, first_branch, additional_branches, gihi, vau))
    def __init__(self, gek: RecoveredField[ModalForethoughtConnectiveSyntax], first: RecoveredField[SubbridiSyntax], first_branch: RecoveredField[ForethoughtBridiBranchSyntax], additional_branches: Sequence[RecoveredField[ZantufaForethoughtBridiBranchSyntax]], gihi: RecoveredField[Token] | None, vau: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def gek(self) -> RecoveredField[ModalForethoughtConnectiveSyntax]:
        'The opening forethought connective that determines how the subbridi branches are combined.'
        return cast(RecoveredField[ModalForethoughtConnectiveSyntax], self._field(0))
    @property
    def first(self) -> RecoveredField[SubbridiSyntax]:
        'The first subbridi branch, which follows the opening connective without an intervening GIK.'
        return cast(RecoveredField[SubbridiSyntax], self._field(1))
    @property
    def first_branch(self) -> RecoveredField[ForethoughtBridiBranchSyntax]:
        'The first GIK-led subbridi branch paired with the opening connective.'
        return cast(RecoveredField[ForethoughtBridiBranchSyntax], self._field(2))
    @property
    def additional_branches(self) -> tuple[RecoveredField[ZantufaForethoughtBridiBranchSyntax], ...]:
        'Additional Zantufa GIK-led subbridi branches, retained in source order.'
        return cast(tuple[RecoveredField[ZantufaForethoughtBridiBranchSyntax], ...], self._field(3))
    @property
    def gihi(self) -> RecoveredField[Token] | None:
        'The optional experimental GIhI terminator following the complete branch sequence.'
        return cast(RecoveredField[Token] | None, self._field(4))
    @property
    def vau(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional elidable VAU terminator for the bridi tail.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(5))
    def __init_subclass__(cls) -> None:
        raise TypeError('DirectForethoughtBridiConnectionWithoutTailTermsSyntax is final')

@final
class ForethoughtBridiBranchSyntax(_SyntaxNode):
    'Product node for forethought bridi branch; preserves `gik` and `branch` in source order.'
    __slots__ = ()
    _schema_id = 117
    __match_args__ = ('gik', 'branch')
    def __new__(cls, gik: RecoveredField[GikConnectiveSyntax], branch: RecoveredField[SubbridiSyntax]) -> ForethoughtBridiBranchSyntax:
        return cls._from_fields((gik, branch))
    def __init__(self, gik: RecoveredField[GikConnectiveSyntax], branch: RecoveredField[SubbridiSyntax]) -> None:
        pass
    @property
    def gik(self) -> RecoveredField[GikConnectiveSyntax]:
        'The GIK connective that introduces this branch and pairs with the opening forethought connective.'
        return cast(RecoveredField[GikConnectiveSyntax], self._field(0))
    @property
    def branch(self) -> RecoveredField[SubbridiSyntax]:
        'The subbridi governed by this branch\'s GIK connective.'
        return cast(RecoveredField[SubbridiSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ForethoughtBridiBranchSyntax is final')

@final
class ZantufaForethoughtBridiBranchSyntax(_SyntaxNode):
    'Product node for forethought bridi branch; preserves `gik` and `branch` in source order.'
    __slots__ = ()
    _schema_id = 118
    __match_args__ = ('gik', 'branch')
    def __new__(cls, gik: RecoveredField[ZantufaExtraGikConnectiveSyntax], branch: RecoveredField[SubbridiSyntax]) -> ZantufaForethoughtBridiBranchSyntax:
        return cls._from_fields((gik, branch))
    def __init__(self, gik: RecoveredField[ZantufaExtraGikConnectiveSyntax], branch: RecoveredField[SubbridiSyntax]) -> None:
        pass
    @property
    def gik(self) -> RecoveredField[ZantufaExtraGikConnectiveSyntax]:
        'The additional Zantufa GIK connective that introduces this branch.'
        return cast(RecoveredField[ZantufaExtraGikConnectiveSyntax], self._field(0))
    @property
    def branch(self) -> RecoveredField[SubbridiSyntax]:
        'The subbridi governed by this additional branch\'s GIK connective.'
        return cast(RecoveredField[SubbridiSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaForethoughtBridiBranchSyntax is final')

@final
class GroupedForethoughtBridiConnectionSyntax(_SyntaxNode):
    'Product node for forethought bridi connection; preserves `tense_modal`, `ke`, `inner`, and `kehe` in source order.'
    __slots__ = ()
    _schema_id = 119
    __match_args__ = ('tense_modal', 'ke', 'inner', 'kehe')
    def __new__(cls, tense_modal: RecoveredField[TenseModalSyntax] | None, ke: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner: RecoveredField[ForethoughtBridiConnectionSyntax], kehe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> GroupedForethoughtBridiConnectionSyntax:
        return cls._from_fields((tense_modal, ke, inner, kehe))
    def __init__(self, tense_modal: RecoveredField[TenseModalSyntax] | None, ke: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner: RecoveredField[ForethoughtBridiConnectionSyntax], kehe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def tense_modal(self) -> RecoveredField[TenseModalSyntax] | None:
        'The optional tense modal component.'
        return cast(RecoveredField[TenseModalSyntax] | None, self._field(0))
    @property
    def ke(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Ke` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(1))
    @property
    def inner(self) -> RecoveredField[ForethoughtBridiConnectionSyntax]:
        'The shared inner child syntax node.'
        return cast(RecoveredField[ForethoughtBridiConnectionSyntax], self._field(2))
    @property
    def kehe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Kehe` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('GroupedForethoughtBridiConnectionSyntax is final')

@final
class GroupedForethoughtBridiConnectionWithoutTailTermsSyntax(_SyntaxNode):
    'Product node for forethought bridi connection; preserves `tense_modal`, `ke`, `inner`, and `kehe` in source order.'
    __slots__ = ()
    _schema_id = 120
    __match_args__ = ('tense_modal', 'ke', 'inner', 'kehe')
    def __new__(cls, tense_modal: RecoveredField[TenseModalSyntax] | None, ke: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner: RecoveredField[ForethoughtBridiConnectionWithoutTailTermsSyntax], kehe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> GroupedForethoughtBridiConnectionWithoutTailTermsSyntax:
        return cls._from_fields((tense_modal, ke, inner, kehe))
    def __init__(self, tense_modal: RecoveredField[TenseModalSyntax] | None, ke: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner: RecoveredField[ForethoughtBridiConnectionWithoutTailTermsSyntax], kehe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def tense_modal(self) -> RecoveredField[TenseModalSyntax] | None:
        'The optional tense modal component.'
        return cast(RecoveredField[TenseModalSyntax] | None, self._field(0))
    @property
    def ke(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Ke` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(1))
    @property
    def inner(self) -> RecoveredField[ForethoughtBridiConnectionWithoutTailTermsSyntax]:
        'The shared inner child syntax node.'
        return cast(RecoveredField[ForethoughtBridiConnectionWithoutTailTermsSyntax], self._field(2))
    @property
    def kehe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Kehe` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('GroupedForethoughtBridiConnectionWithoutTailTermsSyntax is final')

@final
class NegatedForethoughtBridiConnectionSyntax(_SyntaxNode):
    'Product node for forethought bridi connection; preserves `na` and `inner` in source order.'
    __slots__ = ()
    _schema_id = 121
    __match_args__ = ('na', 'inner')
    def __new__(cls, na: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner: RecoveredField[ForethoughtBridiConnectionSyntax]) -> NegatedForethoughtBridiConnectionSyntax:
        return cls._from_fields((na, inner))
    def __init__(self, na: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner: RecoveredField[ForethoughtBridiConnectionSyntax]) -> None:
        pass
    @property
    def na(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Na`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def inner(self) -> RecoveredField[ForethoughtBridiConnectionSyntax]:
        'The shared inner child syntax node.'
        return cast(RecoveredField[ForethoughtBridiConnectionSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('NegatedForethoughtBridiConnectionSyntax is final')

@final
class NegatedForethoughtBridiConnectionWithoutTailTermsSyntax(_SyntaxNode):
    'Product node for forethought bridi connection; preserves `na` and `inner` in source order.'
    __slots__ = ()
    _schema_id = 122
    __match_args__ = ('na', 'inner')
    def __new__(cls, na: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner: RecoveredField[ForethoughtBridiConnectionWithoutTailTermsSyntax]) -> NegatedForethoughtBridiConnectionWithoutTailTermsSyntax:
        return cls._from_fields((na, inner))
    def __init__(self, na: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner: RecoveredField[ForethoughtBridiConnectionWithoutTailTermsSyntax]) -> None:
        pass
    @property
    def na(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Na`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def inner(self) -> RecoveredField[ForethoughtBridiConnectionWithoutTailTermsSyntax]:
        'The shared inner child syntax node.'
        return cast(RecoveredField[ForethoughtBridiConnectionWithoutTailTermsSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('NegatedForethoughtBridiConnectionWithoutTailTermsSyntax is final')

@final
class BridiTailKeContinuationSyntax(_SyntaxNode):
    'Product node for bridi tail connective; preserves `connective`, `tense_modal`, `ke`, and 4 other fields in source order.'
    __slots__ = ()
    _schema_id = 123
    __match_args__ = ('connective', 'tense_modal', 'ke', 'bridi_tail', 'kehe', 'tail_terms', 'vau')
    def __new__(cls, connective: RecoveredField[BridiTailConnectiveSyntax], tense_modal: RecoveredField[TenseModalSyntax] | None, ke: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], bridi_tail: RecoveredField[BridiTailSyntax], kehe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, tail_terms: Sequence[RecoveredField[TermSyntax]], vau: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> BridiTailKeContinuationSyntax:
        return cls._from_fields((connective, tense_modal, ke, bridi_tail, kehe, tail_terms, vau))
    def __init__(self, connective: RecoveredField[BridiTailConnectiveSyntax], tense_modal: RecoveredField[TenseModalSyntax] | None, ke: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], bridi_tail: RecoveredField[BridiTailSyntax], kehe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, tail_terms: Sequence[RecoveredField[TermSyntax]], vau: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def connective(self) -> RecoveredField[BridiTailConnectiveSyntax]:
        'The `bridi_tail_connective` connective joining the adjacent constituents of the `bridi_tail_ke_continuation` production.'
        return cast(RecoveredField[BridiTailConnectiveSyntax], self._field(0))
    @property
    def tense_modal(self) -> RecoveredField[TenseModalSyntax] | None:
        'The optional tense modal component.'
        return cast(RecoveredField[TenseModalSyntax] | None, self._field(1))
    @property
    def ke(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Ke` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(2))
    @property
    def bridi_tail(self) -> RecoveredField[BridiTailSyntax]:
        'The shared bridi tail child syntax node.'
        return cast(RecoveredField[BridiTailSyntax], self._field(3))
    @property
    def kehe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Kehe` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(4))
    @property
    def tail_terms(self) -> tuple[RecoveredField[TermSyntax], ...]:
        'Ordered sequence of zero or more tail terms components.'
        return cast(tuple[RecoveredField[TermSyntax], ...], self._field(5))
    @property
    def vau(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Vau` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(6))
    def __init_subclass__(cls) -> None:
        raise TypeError('BridiTailKeContinuationSyntax is final')

@final
class GihekBridiTailKeContinuationSyntax(_SyntaxNode):
    'Product node for bridi tail connective; preserves `connective`, `tense_modal`, `ke`, and 4 other fields in source order.'
    __slots__ = ()
    _schema_id = 124
    __match_args__ = ('connective', 'tense_modal', 'ke', 'bridi_tail', 'kehe', 'tail_terms', 'vau')
    def __new__(cls, connective: RecoveredField[GihekConnectiveSyntax], tense_modal: RecoveredField[TenseModalSyntax] | None, ke: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], bridi_tail: RecoveredField[BridiTailSyntax], kehe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, tail_terms: Sequence[RecoveredField[TermSyntax]], vau: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> GihekBridiTailKeContinuationSyntax:
        return cls._from_fields((connective, tense_modal, ke, bridi_tail, kehe, tail_terms, vau))
    def __init__(self, connective: RecoveredField[GihekConnectiveSyntax], tense_modal: RecoveredField[TenseModalSyntax] | None, ke: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], bridi_tail: RecoveredField[BridiTailSyntax], kehe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, tail_terms: Sequence[RecoveredField[TermSyntax]], vau: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def connective(self) -> RecoveredField[GihekConnectiveSyntax]:
        'The `gihek_connective` connective joining the adjacent constituents of the `gihek_bridi_tail_ke_continuation` production.'
        return cast(RecoveredField[GihekConnectiveSyntax], self._field(0))
    @property
    def tense_modal(self) -> RecoveredField[TenseModalSyntax] | None:
        'The optional tense modal component.'
        return cast(RecoveredField[TenseModalSyntax] | None, self._field(1))
    @property
    def ke(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Ke` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(2))
    @property
    def bridi_tail(self) -> RecoveredField[BridiTailSyntax]:
        'The shared bridi tail child syntax node.'
        return cast(RecoveredField[BridiTailSyntax], self._field(3))
    @property
    def kehe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Kehe` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(4))
    @property
    def tail_terms(self) -> tuple[RecoveredField[TermSyntax], ...]:
        'Ordered sequence of zero or more tail terms components.'
        return cast(tuple[RecoveredField[TermSyntax], ...], self._field(5))
    @property
    def vau(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Vau` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(6))
    def __init_subclass__(cls) -> None:
        raise TypeError('GihekBridiTailKeContinuationSyntax is final')

@final
class BridiTailBoContinuationWithoutTailTermsSyntax(_SyntaxNode):
    'Product node for bridi tail connective; preserves `connective`, `tense_modal`, `bo`, `cu`, and `bridi_tail` in source order.'
    __slots__ = ()
    _schema_id = 125
    __match_args__ = ('connective', 'tense_modal', 'bo', 'cu', 'bridi_tail')
    def __new__(cls, connective: RecoveredField[BridiTailConnectiveSyntax], tense_modal: RecoveredField[TenseModalSyntax] | None, bo: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], cu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, bridi_tail: RecoveredField[BoGroupedBridiTailWithoutTailTermsSyntax]) -> BridiTailBoContinuationWithoutTailTermsSyntax:
        return cls._from_fields((connective, tense_modal, bo, cu, bridi_tail))
    def __init__(self, connective: RecoveredField[BridiTailConnectiveSyntax], tense_modal: RecoveredField[TenseModalSyntax] | None, bo: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], cu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, bridi_tail: RecoveredField[BoGroupedBridiTailWithoutTailTermsSyntax]) -> None:
        pass
    @property
    def connective(self) -> RecoveredField[BridiTailConnectiveSyntax]:
        'The `bridi_tail_connective` connective joining the adjacent constituents of the `bridi_tail_bo_continuation_without_tail_terms` production.'
        return cast(RecoveredField[BridiTailConnectiveSyntax], self._field(0))
    @property
    def tense_modal(self) -> RecoveredField[TenseModalSyntax] | None:
        'The optional tense modal component.'
        return cast(RecoveredField[TenseModalSyntax] | None, self._field(1))
    @property
    def bo(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Bo` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(2))
    @property
    def cu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Cu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(3))
    @property
    def bridi_tail(self) -> RecoveredField[BoGroupedBridiTailWithoutTailTermsSyntax]:
        'The shared bridi tail child syntax node.'
        return cast(RecoveredField[BoGroupedBridiTailWithoutTailTermsSyntax], self._field(4))
    def __init_subclass__(cls) -> None:
        raise TypeError('BridiTailBoContinuationWithoutTailTermsSyntax is final')

@final
class BridiTailBoContinuationSyntax(_SyntaxNode):
    'Product node for bridi tail connective; preserves `connective`, `tense_modal`, `bo`, and 4 other fields in source order.'
    __slots__ = ()
    _schema_id = 126
    __match_args__ = ('connective', 'tense_modal', 'bo', 'cu', 'bridi_tail', 'tail_terms', 'vau')
    def __new__(cls, connective: RecoveredField[BridiTailConnectiveSyntax], tense_modal: RecoveredField[TenseModalSyntax] | None, bo: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], cu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, bridi_tail: RecoveredField[BoGroupedBridiTailSyntax], tail_terms: Sequence[RecoveredField[TermSyntax]], vau: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> BridiTailBoContinuationSyntax:
        return cls._from_fields((connective, tense_modal, bo, cu, bridi_tail, tail_terms, vau))
    def __init__(self, connective: RecoveredField[BridiTailConnectiveSyntax], tense_modal: RecoveredField[TenseModalSyntax] | None, bo: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], cu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, bridi_tail: RecoveredField[BoGroupedBridiTailSyntax], tail_terms: Sequence[RecoveredField[TermSyntax]], vau: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def connective(self) -> RecoveredField[BridiTailConnectiveSyntax]:
        'The `bridi_tail_connective` connective joining the adjacent constituents of the `bridi_tail_bo_continuation` production.'
        return cast(RecoveredField[BridiTailConnectiveSyntax], self._field(0))
    @property
    def tense_modal(self) -> RecoveredField[TenseModalSyntax] | None:
        'The optional tense modal component.'
        return cast(RecoveredField[TenseModalSyntax] | None, self._field(1))
    @property
    def bo(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Bo` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(2))
    @property
    def cu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Cu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(3))
    @property
    def bridi_tail(self) -> RecoveredField[BoGroupedBridiTailSyntax]:
        'The shared bridi tail child syntax node.'
        return cast(RecoveredField[BoGroupedBridiTailSyntax], self._field(4))
    @property
    def tail_terms(self) -> tuple[RecoveredField[TermSyntax], ...]:
        'Ordered sequence of zero or more tail terms components.'
        return cast(tuple[RecoveredField[TermSyntax], ...], self._field(5))
    @property
    def vau(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Vau` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(6))
    def __init_subclass__(cls) -> None:
        raise TypeError('BridiTailBoContinuationSyntax is final')

@final
class BridiTailContinuationWithoutTailTermsSyntax(_SyntaxNode):
    'Product node for bridi tail connective; preserves `connective`, `cu`, and `bridi_tail` in source order.'
    __slots__ = ()
    _schema_id = 127
    __match_args__ = ('connective', 'cu', 'bridi_tail')
    def __new__(cls, connective: RecoveredField[BridiTailConnectiveSyntax], cu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, bridi_tail: RecoveredField[BoGroupedBridiTailWithoutTailTermsSyntax]) -> BridiTailContinuationWithoutTailTermsSyntax:
        return cls._from_fields((connective, cu, bridi_tail))
    def __init__(self, connective: RecoveredField[BridiTailConnectiveSyntax], cu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, bridi_tail: RecoveredField[BoGroupedBridiTailWithoutTailTermsSyntax]) -> None:
        pass
    @property
    def connective(self) -> RecoveredField[BridiTailConnectiveSyntax]:
        'The `bridi_tail_connective` connective joining the adjacent constituents of the `bridi_tail_continuation_without_tail_terms` production.'
        return cast(RecoveredField[BridiTailConnectiveSyntax], self._field(0))
    @property
    def cu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Cu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(1))
    @property
    def bridi_tail(self) -> RecoveredField[BoGroupedBridiTailWithoutTailTermsSyntax]:
        'The shared bridi tail child syntax node.'
        return cast(RecoveredField[BoGroupedBridiTailWithoutTailTermsSyntax], self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('BridiTailContinuationWithoutTailTermsSyntax is final')

@final
class BridiTailContinuationSyntax(_SyntaxNode):
    'Product node for bridi tail connective; preserves `connective`, `cu`, `bridi_tail`, `tail_terms`, and `vau` in source order.'
    __slots__ = ()
    _schema_id = 128
    __match_args__ = ('connective', 'cu', 'bridi_tail', 'tail_terms', 'vau')
    def __new__(cls, connective: RecoveredField[BridiTailConnectiveSyntax], cu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, bridi_tail: RecoveredField[BoGroupedBridiTailSyntax], tail_terms: Sequence[RecoveredField[TermSyntax]], vau: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> BridiTailContinuationSyntax:
        return cls._from_fields((connective, cu, bridi_tail, tail_terms, vau))
    def __init__(self, connective: RecoveredField[BridiTailConnectiveSyntax], cu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, bridi_tail: RecoveredField[BoGroupedBridiTailSyntax], tail_terms: Sequence[RecoveredField[TermSyntax]], vau: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def connective(self) -> RecoveredField[BridiTailConnectiveSyntax]:
        'The `bridi_tail_connective` connective joining the adjacent constituents of the `bridi_tail_continuation` production.'
        return cast(RecoveredField[BridiTailConnectiveSyntax], self._field(0))
    @property
    def cu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Cu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(1))
    @property
    def bridi_tail(self) -> RecoveredField[BoGroupedBridiTailSyntax]:
        'The shared bridi tail child syntax node.'
        return cast(RecoveredField[BoGroupedBridiTailSyntax], self._field(2))
    @property
    def tail_terms(self) -> tuple[RecoveredField[TermSyntax], ...]:
        'Ordered sequence of zero or more tail terms components.'
        return cast(tuple[RecoveredField[TermSyntax], ...], self._field(3))
    @property
    def vau(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Vau` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(4))
    def __init_subclass__(cls) -> None:
        raise TypeError('BridiTailContinuationSyntax is final')

@final
class SubbridiSyntaxPrenexSubbridi(_SyntaxNode):
    'Uses the `prenex_subbridi` product form, whose payload preserves `prenex_terms`, `zohu`, and `inner_subbridi`.'
    __slots__ = ()
    _schema_id = 129
    __match_args__ = ('prenex_subbridi',)
    def __new__(cls, prenex_subbridi: RecoveredField[PrenexSubbridiSyntax]) -> SubbridiSyntaxPrenexSubbridi:
        return cls._from_fields((prenex_subbridi,))
    def __init__(self, prenex_subbridi: RecoveredField[PrenexSubbridiSyntax]) -> None:
        pass
    @property
    def prenex_subbridi(self) -> RecoveredField[PrenexSubbridiSyntax]:
        'Uses the `prenex_subbridi` product form, whose payload preserves `prenex_terms`, `zohu`, and `inner_subbridi`.'
        return cast(RecoveredField[PrenexSubbridiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SubbridiSyntaxPrenexSubbridi is final')

@final
class SubbridiSyntaxBridiSubbridi(_SyntaxNode):
    'Uses the `bridi_subbridi` product form, whose payload preserves `bridi`.'
    __slots__ = ()
    _schema_id = 130
    __match_args__ = ('bridi_subbridi',)
    def __new__(cls, bridi_subbridi: RecoveredField[BridiSubbridiSyntax]) -> SubbridiSyntaxBridiSubbridi:
        return cls._from_fields((bridi_subbridi,))
    def __init__(self, bridi_subbridi: RecoveredField[BridiSubbridiSyntax]) -> None:
        pass
    @property
    def bridi_subbridi(self) -> RecoveredField[BridiSubbridiSyntax]:
        'Uses the `bridi_subbridi` product form, whose payload preserves `bridi`.'
        return cast(RecoveredField[BridiSubbridiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SubbridiSyntaxBridiSubbridi is final')

SubbridiSyntax: TypeAlias = SubbridiSyntaxPrenexSubbridi | SubbridiSyntaxBridiSubbridi

@final
class BridiSubbridiSyntax(_SyntaxNode):
    'Transparent product node for subbridi; preserves the `bridi` component.'
    __slots__ = ()
    _schema_id = 131
    __match_args__ = ('bridi',)
    def __new__(cls, bridi: RecoveredField[BridiSyntax]) -> BridiSubbridiSyntax:
        return cls._from_fields((bridi,))
    def __init__(self, bridi: RecoveredField[BridiSyntax]) -> None:
        pass
    @property
    def bridi(self) -> RecoveredField[BridiSyntax]:
        'The shared bridi child syntax node.'
        return cast(RecoveredField[BridiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('BridiSubbridiSyntax is final')

@final
class PrenexSubbridiSyntax(_SyntaxNode):
    'Product node for prenex; preserves `prenex_terms`, `zohu`, and `inner_subbridi` in source order.'
    __slots__ = ()
    _schema_id = 132
    __match_args__ = ('prenex_terms', 'zohu', 'inner_subbridi')
    def __new__(cls, prenex_terms: Sequence[RecoveredField[TermSyntax]], zohu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_subbridi: RecoveredField[SubbridiSyntax]) -> PrenexSubbridiSyntax:
        return cls._from_fields((prenex_terms, zohu, inner_subbridi))
    def __init__(self, prenex_terms: Sequence[RecoveredField[TermSyntax]], zohu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_subbridi: RecoveredField[SubbridiSyntax]) -> None:
        pass
    @property
    def prenex_terms(self) -> tuple[RecoveredField[TermSyntax], ...]:
        'Ordered sequence of zero or more prenex terms components.'
        return cast(tuple[RecoveredField[TermSyntax], ...], self._field(0))
    @property
    def zohu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Zohu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(1))
    @property
    def inner_subbridi(self) -> RecoveredField[SubbridiSyntax]:
        'The shared inner subbridi child syntax node.'
        return cast(RecoveredField[SubbridiSyntax], self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('PrenexSubbridiSyntax is final')

@final
class TermSyntaxPeheTermsetConnection(_SyntaxNode):
    'Uses the `pehe_termset_connection` product form, whose payload preserves `leading_term` and `continuations`.'
    __slots__ = ()
    _schema_id = 133
    __match_args__ = ('pehe_termset_connection',)
    def __new__(cls, pehe_termset_connection: RecoveredField[PeheTermsetConnectionSyntax]) -> TermSyntaxPeheTermsetConnection:
        return cls._from_fields((pehe_termset_connection,))
    def __init__(self, pehe_termset_connection: RecoveredField[PeheTermsetConnectionSyntax]) -> None:
        pass
    @property
    def pehe_termset_connection(self) -> RecoveredField[PeheTermsetConnectionSyntax]:
        'Uses the `pehe_termset_connection` product form, whose payload preserves `leading_term` and `continuations`.'
        return cast(RecoveredField[PeheTermsetConnectionSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TermSyntaxPeheTermsetConnection is final')

@final
class TermSyntaxBoundTermConnection(_SyntaxNode):
    'Uses the `bound_term_connection` product form, whose payload preserves `leading_term`, `connective`, `bo`, and `trailing_term`.'
    __slots__ = ()
    _schema_id = 134
    __match_args__ = ('bound_term_connection',)
    def __new__(cls, bound_term_connection: RecoveredField[BoundTermConnectionSyntax]) -> TermSyntaxBoundTermConnection:
        return cls._from_fields((bound_term_connection,))
    def __init__(self, bound_term_connection: RecoveredField[BoundTermConnectionSyntax]) -> None:
        pass
    @property
    def bound_term_connection(self) -> RecoveredField[BoundTermConnectionSyntax]:
        'Uses the `bound_term_connection` product form, whose payload preserves `leading_term`, `connective`, `bo`, and `trailing_term`.'
        return cast(RecoveredField[BoundTermConnectionSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TermSyntaxBoundTermConnection is final')

@final
class TermSyntaxTermsetGroup(_SyntaxNode):
    'Uses the `termset_group` product form, whose payload preserves `leading_term` and `continuations`.'
    __slots__ = ()
    _schema_id = 135
    __match_args__ = ('termset_group',)
    def __new__(cls, termset_group: RecoveredField[TermsetGroupSyntax]) -> TermSyntaxTermsetGroup:
        return cls._from_fields((termset_group,))
    def __init__(self, termset_group: RecoveredField[TermsetGroupSyntax]) -> None:
        pass
    @property
    def termset_group(self) -> RecoveredField[TermsetGroupSyntax]:
        'Uses the `termset_group` product form, whose payload preserves `leading_term` and `continuations`.'
        return cast(RecoveredField[TermsetGroupSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TermSyntaxTermsetGroup is final')

@final
class TermSyntaxConnectedTerm(_SyntaxNode):
    'Uses the `connected_term` product form, whose payload preserves `leading_term` and `continuations`.'
    __slots__ = ()
    _schema_id = 136
    __match_args__ = ('connected_term',)
    def __new__(cls, connected_term: RecoveredField[ConnectedTermSyntax]) -> TermSyntaxConnectedTerm:
        return cls._from_fields((connected_term,))
    def __init__(self, connected_term: RecoveredField[ConnectedTermSyntax]) -> None:
        pass
    @property
    def connected_term(self) -> RecoveredField[ConnectedTermSyntax]:
        'Uses the `connected_term` product form, whose payload preserves `leading_term` and `continuations`.'
        return cast(RecoveredField[ConnectedTermSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TermSyntaxConnectedTerm is final')

@final
class TermSyntaxSimpleTerm(_SyntaxNode):
    'Uses the nested `simple_term` sum form and preserves its selected alternative.'
    __slots__ = ()
    _schema_id = 137
    __match_args__ = ('simple_term',)
    def __new__(cls, simple_term: RecoveredField[SimpleTermSyntax]) -> TermSyntaxSimpleTerm:
        return cls._from_fields((simple_term,))
    def __init__(self, simple_term: RecoveredField[SimpleTermSyntax]) -> None:
        pass
    @property
    def simple_term(self) -> RecoveredField[SimpleTermSyntax]:
        'Uses the nested `simple_term` sum form and preserves its selected alternative.'
        return cast(RecoveredField[SimpleTermSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TermSyntaxSimpleTerm is final')

TermSyntax: TypeAlias = TermSyntaxPeheTermsetConnection | TermSyntaxBoundTermConnection | TermSyntaxTermsetGroup | TermSyntaxConnectedTerm | TermSyntaxSimpleTerm

@final
class PeheTermsetConnectionSyntax(_SyntaxNode):
    'Product node for termset connection; preserves `leading_term` and `continuations` in source order.'
    __slots__ = ()
    _schema_id = 138
    __match_args__ = ('leading_term', 'continuations')
    def __new__(cls, leading_term: RecoveredField[PeheTermsetOperandSyntax], continuations: Sequence[RecoveredField[PeheTermsetConnectionContinuationSyntax]]) -> PeheTermsetConnectionSyntax:
        return cls._from_fields((leading_term, continuations))
    def __init__(self, leading_term: RecoveredField[PeheTermsetOperandSyntax], continuations: Sequence[RecoveredField[PeheTermsetConnectionContinuationSyntax]]) -> None:
        pass
    @property
    def leading_term(self) -> RecoveredField[PeheTermsetOperandSyntax]:
        'The shared leading term child syntax node.'
        return cast(RecoveredField[PeheTermsetOperandSyntax], self._field(0))
    @property
    def continuations(self) -> tuple[RecoveredField[PeheTermsetConnectionContinuationSyntax], ...]:
        'Non-empty ordered sequence of continuations components.'
        return cast(tuple[RecoveredField[PeheTermsetConnectionContinuationSyntax], ...], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('PeheTermsetConnectionSyntax is final')

@final
class PeheTermsetConnectionContinuationSyntax(_SyntaxNode):
    'Product node for termset connection continuation; preserves `pehe`, `connective`, and `trailing_term` in source order.'
    __slots__ = ()
    _schema_id = 139
    __match_args__ = ('pehe', 'connective', 'trailing_term')
    def __new__(cls, pehe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], connective: RecoveredField[StatementConnectiveSyntax], trailing_term: RecoveredField[PeheTermsetOperandSyntax]) -> PeheTermsetConnectionContinuationSyntax:
        return cls._from_fields((pehe, connective, trailing_term))
    def __init__(self, pehe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], connective: RecoveredField[StatementConnectiveSyntax], trailing_term: RecoveredField[PeheTermsetOperandSyntax]) -> None:
        pass
    @property
    def pehe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Pehe` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def connective(self) -> RecoveredField[StatementConnectiveSyntax]:
        'The `statement_connective` connective joining the adjacent constituents of the `pehe_termset_connection_continuation` production.'
        return cast(RecoveredField[StatementConnectiveSyntax], self._field(1))
    @property
    def trailing_term(self) -> RecoveredField[PeheTermsetOperandSyntax]:
        'The shared trailing term child syntax node.'
        return cast(RecoveredField[PeheTermsetOperandSyntax], self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('PeheTermsetConnectionContinuationSyntax is final')

@final
class PeheTermsetOperandSyntaxBoundTermConnection(_SyntaxNode):
    'Uses the `bound_term_connection` product form, whose payload preserves `leading_term`, `connective`, `bo`, and `trailing_term`.'
    __slots__ = ()
    _schema_id = 140
    __match_args__ = ('bound_term_connection',)
    def __new__(cls, bound_term_connection: RecoveredField[BoundTermConnectionSyntax]) -> PeheTermsetOperandSyntaxBoundTermConnection:
        return cls._from_fields((bound_term_connection,))
    def __init__(self, bound_term_connection: RecoveredField[BoundTermConnectionSyntax]) -> None:
        pass
    @property
    def bound_term_connection(self) -> RecoveredField[BoundTermConnectionSyntax]:
        'Uses the `bound_term_connection` product form, whose payload preserves `leading_term`, `connective`, `bo`, and `trailing_term`.'
        return cast(RecoveredField[BoundTermConnectionSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('PeheTermsetOperandSyntaxBoundTermConnection is final')

@final
class PeheTermsetOperandSyntaxTermsetGroup(_SyntaxNode):
    'Uses the `termset_group` product form, whose payload preserves `leading_term` and `continuations`.'
    __slots__ = ()
    _schema_id = 141
    __match_args__ = ('termset_group',)
    def __new__(cls, termset_group: RecoveredField[TermsetGroupSyntax]) -> PeheTermsetOperandSyntaxTermsetGroup:
        return cls._from_fields((termset_group,))
    def __init__(self, termset_group: RecoveredField[TermsetGroupSyntax]) -> None:
        pass
    @property
    def termset_group(self) -> RecoveredField[TermsetGroupSyntax]:
        'Uses the `termset_group` product form, whose payload preserves `leading_term` and `continuations`.'
        return cast(RecoveredField[TermsetGroupSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('PeheTermsetOperandSyntaxTermsetGroup is final')

@final
class PeheTermsetOperandSyntaxSimpleTerm(_SyntaxNode):
    'Uses the nested `simple_term` sum form and preserves its selected alternative.'
    __slots__ = ()
    _schema_id = 142
    __match_args__ = ('simple_term',)
    def __new__(cls, simple_term: RecoveredField[SimpleTermSyntax]) -> PeheTermsetOperandSyntaxSimpleTerm:
        return cls._from_fields((simple_term,))
    def __init__(self, simple_term: RecoveredField[SimpleTermSyntax]) -> None:
        pass
    @property
    def simple_term(self) -> RecoveredField[SimpleTermSyntax]:
        'Uses the nested `simple_term` sum form and preserves its selected alternative.'
        return cast(RecoveredField[SimpleTermSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('PeheTermsetOperandSyntaxSimpleTerm is final')

PeheTermsetOperandSyntax: TypeAlias = PeheTermsetOperandSyntaxBoundTermConnection | PeheTermsetOperandSyntaxTermsetGroup | PeheTermsetOperandSyntaxSimpleTerm

@final
class SimpleTermSyntaxPlaceTaggedSumtiTerm(_SyntaxNode):
    'Uses the `place_tagged_sumti_term` product form, whose payload preserves `fa` and `sumti`.'
    __slots__ = ()
    _schema_id = 143
    __match_args__ = ('place_tagged_sumti_term',)
    def __new__(cls, place_tagged_sumti_term: RecoveredField[PlaceTaggedSumtiTermSyntax]) -> SimpleTermSyntaxPlaceTaggedSumtiTerm:
        return cls._from_fields((place_tagged_sumti_term,))
    def __init__(self, place_tagged_sumti_term: RecoveredField[PlaceTaggedSumtiTermSyntax]) -> None:
        pass
    @property
    def place_tagged_sumti_term(self) -> RecoveredField[PlaceTaggedSumtiTermSyntax]:
        'Uses the `place_tagged_sumti_term` product form, whose payload preserves `fa` and `sumti`.'
        return cast(RecoveredField[PlaceTaggedSumtiTermSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleTermSyntaxPlaceTaggedSumtiTerm is final')

@final
class SimpleTermSyntaxJaiTaggedSumtiTerm(_SyntaxNode):
    'Uses the `jai_tagged_sumti_term` product form, whose payload preserves `jai`, `tag`, and `sumti`.'
    __slots__ = ()
    _schema_id = 144
    __match_args__ = ('jai_tagged_sumti_term',)
    def __new__(cls, jai_tagged_sumti_term: RecoveredField[JaiTaggedSumtiTermSyntax]) -> SimpleTermSyntaxJaiTaggedSumtiTerm:
        return cls._from_fields((jai_tagged_sumti_term,))
    def __init__(self, jai_tagged_sumti_term: RecoveredField[JaiTaggedSumtiTermSyntax]) -> None:
        pass
    @property
    def jai_tagged_sumti_term(self) -> RecoveredField[JaiTaggedSumtiTermSyntax]:
        'Uses the `jai_tagged_sumti_term` product form, whose payload preserves `jai`, `tag`, and `sumti`.'
        return cast(RecoveredField[JaiTaggedSumtiTermSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleTermSyntaxJaiTaggedSumtiTerm is final')

@final
class SimpleTermSyntaxTaggedSumtiBeforeTagTerm(_SyntaxNode):
    'Uses the `tagged_sumti_before_tag_term` product form, whose payload preserves `tense_modal`.'
    __slots__ = ()
    _schema_id = 145
    __match_args__ = ('tagged_sumti_before_tag_term',)
    def __new__(cls, tagged_sumti_before_tag_term: RecoveredField[TaggedSumtiBeforeTagTermSyntax]) -> SimpleTermSyntaxTaggedSumtiBeforeTagTerm:
        return cls._from_fields((tagged_sumti_before_tag_term,))
    def __init__(self, tagged_sumti_before_tag_term: RecoveredField[TaggedSumtiBeforeTagTermSyntax]) -> None:
        pass
    @property
    def tagged_sumti_before_tag_term(self) -> RecoveredField[TaggedSumtiBeforeTagTermSyntax]:
        'Uses the `tagged_sumti_before_tag_term` product form, whose payload preserves `tense_modal`.'
        return cast(RecoveredField[TaggedSumtiBeforeTagTermSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleTermSyntaxTaggedSumtiBeforeTagTerm is final')

@final
class SimpleTermSyntaxTaggedSumtiTerm(_SyntaxNode):
    'Uses the `tagged_sumti_term` product form, whose payload preserves `tense_modal` and `sumti`.'
    __slots__ = ()
    _schema_id = 146
    __match_args__ = ('tagged_sumti_term',)
    def __new__(cls, tagged_sumti_term: RecoveredField[TaggedSumtiTermSyntax]) -> SimpleTermSyntaxTaggedSumtiTerm:
        return cls._from_fields((tagged_sumti_term,))
    def __init__(self, tagged_sumti_term: RecoveredField[TaggedSumtiTermSyntax]) -> None:
        pass
    @property
    def tagged_sumti_term(self) -> RecoveredField[TaggedSumtiTermSyntax]:
        'Uses the `tagged_sumti_term` product form, whose payload preserves `tense_modal` and `sumti`.'
        return cast(RecoveredField[TaggedSumtiTermSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleTermSyntaxTaggedSumtiTerm is final')

@final
class SimpleTermSyntaxNoihaAdverbialTerm(_SyntaxNode):
    'Uses the nested `noiha_adverbial_term` sum form and preserves its selected alternative.'
    __slots__ = ()
    _schema_id = 147
    __match_args__ = ('noiha_adverbial_term',)
    def __new__(cls, noiha_adverbial_term: RecoveredField[NoihaAdverbialTermSyntax]) -> SimpleTermSyntaxNoihaAdverbialTerm:
        return cls._from_fields((noiha_adverbial_term,))
    def __init__(self, noiha_adverbial_term: RecoveredField[NoihaAdverbialTermSyntax]) -> None:
        pass
    @property
    def noiha_adverbial_term(self) -> RecoveredField[NoihaAdverbialTermSyntax]:
        'Uses the nested `noiha_adverbial_term` sum form and preserves its selected alternative.'
        return cast(RecoveredField[NoihaAdverbialTermSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleTermSyntaxNoihaAdverbialTerm is final')

@final
class SimpleTermSyntaxFihoiAdverbialTerm(_SyntaxNode):
    'Uses the `fihoi_adverbial_term` product form, whose payload preserves `fihoi`, `statement`, and `fihau`.'
    __slots__ = ()
    _schema_id = 148
    __match_args__ = ('fihoi_adverbial_term',)
    def __new__(cls, fihoi_adverbial_term: RecoveredField[FihoiAdverbialTermSyntax]) -> SimpleTermSyntaxFihoiAdverbialTerm:
        return cls._from_fields((fihoi_adverbial_term,))
    def __init__(self, fihoi_adverbial_term: RecoveredField[FihoiAdverbialTermSyntax]) -> None:
        pass
    @property
    def fihoi_adverbial_term(self) -> RecoveredField[FihoiAdverbialTermSyntax]:
        'Uses the `fihoi_adverbial_term` product form, whose payload preserves `fihoi`, `statement`, and `fihau`.'
        return cast(RecoveredField[FihoiAdverbialTermSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleTermSyntaxFihoiAdverbialTerm is final')

@final
class SimpleTermSyntaxSoiAdverbialTerm(_SyntaxNode):
    'Uses the `soi_adverbial_term` product form, whose payload preserves `soi`, `statement`, and `sehu`.'
    __slots__ = ()
    _schema_id = 149
    __match_args__ = ('soi_adverbial_term',)
    def __new__(cls, soi_adverbial_term: RecoveredField[SoiAdverbialTermSyntax]) -> SimpleTermSyntaxSoiAdverbialTerm:
        return cls._from_fields((soi_adverbial_term,))
    def __init__(self, soi_adverbial_term: RecoveredField[SoiAdverbialTermSyntax]) -> None:
        pass
    @property
    def soi_adverbial_term(self) -> RecoveredField[SoiAdverbialTermSyntax]:
        'Uses the `soi_adverbial_term` product form, whose payload preserves `soi`, `statement`, and `sehu`.'
        return cast(RecoveredField[SoiAdverbialTermSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleTermSyntaxSoiAdverbialTerm is final')

@final
class SimpleTermSyntaxNaKuTerm(_SyntaxNode):
    'Uses the `na_ku_term` product form, whose payload preserves `na` and `na_ku`.'
    __slots__ = ()
    _schema_id = 150
    __match_args__ = ('na_ku_term',)
    def __new__(cls, na_ku_term: RecoveredField[NaKuTermSyntax]) -> SimpleTermSyntaxNaKuTerm:
        return cls._from_fields((na_ku_term,))
    def __init__(self, na_ku_term: RecoveredField[NaKuTermSyntax]) -> None:
        pass
    @property
    def na_ku_term(self) -> RecoveredField[NaKuTermSyntax]:
        'Uses the `na_ku_term` product form, whose payload preserves `na` and `na_ku`.'
        return cast(RecoveredField[NaKuTermSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleTermSyntaxNaKuTerm is final')

@final
class SimpleTermSyntaxSumtiTerm(_SyntaxNode):
    'Uses the `sumti_term` product form, whose payload preserves `sumti`.'
    __slots__ = ()
    _schema_id = 151
    __match_args__ = ('sumti_term',)
    def __new__(cls, sumti_term: RecoveredField[SumtiTermSyntax]) -> SimpleTermSyntaxSumtiTerm:
        return cls._from_fields((sumti_term,))
    def __init__(self, sumti_term: RecoveredField[SumtiTermSyntax]) -> None:
        pass
    @property
    def sumti_term(self) -> RecoveredField[SumtiTermSyntax]:
        'Uses the `sumti_term` product form, whose payload preserves `sumti`.'
        return cast(RecoveredField[SumtiTermSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleTermSyntaxSumtiTerm is final')

@final
class SimpleTermSyntaxBareNaTerm(_SyntaxNode):
    'Uses the `bare_na_term` product form, whose payload preserves `na`.'
    __slots__ = ()
    _schema_id = 152
    __match_args__ = ('bare_na_term',)
    def __new__(cls, bare_na_term: RecoveredField[BareNaTermSyntax]) -> SimpleTermSyntaxBareNaTerm:
        return cls._from_fields((bare_na_term,))
    def __init__(self, bare_na_term: RecoveredField[BareNaTermSyntax]) -> None:
        pass
    @property
    def bare_na_term(self) -> RecoveredField[BareNaTermSyntax]:
        'Uses the `bare_na_term` product form, whose payload preserves `na`.'
        return cast(RecoveredField[BareNaTermSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleTermSyntaxBareNaTerm is final')

@final
class SimpleTermSyntaxForethoughtTermset(_SyntaxNode):
    'Uses the `forethought_termset` product form, whose payload preserves `m_nuhi`, `gek`, `terms`, and 4 other fields.'
    __slots__ = ()
    _schema_id = 153
    __match_args__ = ('forethought_termset',)
    def __new__(cls, forethought_termset: RecoveredField[ForethoughtTermsetSyntax]) -> SimpleTermSyntaxForethoughtTermset:
        return cls._from_fields((forethought_termset,))
    def __init__(self, forethought_termset: RecoveredField[ForethoughtTermsetSyntax]) -> None:
        pass
    @property
    def forethought_termset(self) -> RecoveredField[ForethoughtTermsetSyntax]:
        'Uses the `forethought_termset` product form, whose payload preserves `m_nuhi`, `gek`, `terms`, and 4 other fields.'
        return cast(RecoveredField[ForethoughtTermsetSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleTermSyntaxForethoughtTermset is final')

@final
class SimpleTermSyntaxNuhiTermset(_SyntaxNode):
    'Uses the `nuhi_termset` product form, whose payload preserves `nuhi`, `termset`, and `nuhu`.'
    __slots__ = ()
    _schema_id = 154
    __match_args__ = ('nuhi_termset',)
    def __new__(cls, nuhi_termset: RecoveredField[NuhiTermsetSyntax]) -> SimpleTermSyntaxNuhiTermset:
        return cls._from_fields((nuhi_termset,))
    def __init__(self, nuhi_termset: RecoveredField[NuhiTermsetSyntax]) -> None:
        pass
    @property
    def nuhi_termset(self) -> RecoveredField[NuhiTermsetSyntax]:
        'Uses the `nuhi_termset` product form, whose payload preserves `nuhi`, `termset`, and `nuhu`.'
        return cast(RecoveredField[NuhiTermsetSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleTermSyntaxNuhiTermset is final')

@final
class SimpleTermSyntaxKeTermset(_SyntaxNode):
    'Uses the `ke_termset` product form, whose payload preserves `ke`, `termset`, and `kehe`.'
    __slots__ = ()
    _schema_id = 155
    __match_args__ = ('ke_termset',)
    def __new__(cls, ke_termset: RecoveredField[KeTermsetSyntax]) -> SimpleTermSyntaxKeTermset:
        return cls._from_fields((ke_termset,))
    def __init__(self, ke_termset: RecoveredField[KeTermsetSyntax]) -> None:
        pass
    @property
    def ke_termset(self) -> RecoveredField[KeTermsetSyntax]:
        'Uses the `ke_termset` product form, whose payload preserves `ke`, `termset`, and `kehe`.'
        return cast(RecoveredField[KeTermsetSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleTermSyntaxKeTermset is final')

SimpleTermSyntax: TypeAlias = SimpleTermSyntaxPlaceTaggedSumtiTerm | SimpleTermSyntaxJaiTaggedSumtiTerm | SimpleTermSyntaxTaggedSumtiBeforeTagTerm | SimpleTermSyntaxTaggedSumtiTerm | SimpleTermSyntaxNoihaAdverbialTerm | SimpleTermSyntaxFihoiAdverbialTerm | SimpleTermSyntaxSoiAdverbialTerm | SimpleTermSyntaxNaKuTerm | SimpleTermSyntaxSumtiTerm | SimpleTermSyntaxBareNaTerm | SimpleTermSyntaxForethoughtTermset | SimpleTermSyntaxNuhiTermset | SimpleTermSyntaxKeTermset

@final
class BoundTermConnectionSyntax(_SyntaxNode):
    'Product node for term connection; preserves `leading_term`, `connective`, `bo`, and `trailing_term` in source order.'
    __slots__ = ()
    _schema_id = 156
    __match_args__ = ('leading_term', 'connective', 'bo', 'trailing_term')
    def __new__(cls, leading_term: RecoveredField[SimpleTermSyntax], connective: RecoveredField[BoundTermConnectiveSyntax], bo: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], trailing_term: RecoveredField[SimpleTermSyntax]) -> BoundTermConnectionSyntax:
        return cls._from_fields((leading_term, connective, bo, trailing_term))
    def __init__(self, leading_term: RecoveredField[SimpleTermSyntax], connective: RecoveredField[BoundTermConnectiveSyntax], bo: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], trailing_term: RecoveredField[SimpleTermSyntax]) -> None:
        pass
    @property
    def leading_term(self) -> RecoveredField[SimpleTermSyntax]:
        'The shared leading term child syntax node.'
        return cast(RecoveredField[SimpleTermSyntax], self._field(0))
    @property
    def connective(self) -> RecoveredField[BoundTermConnectiveSyntax]:
        'The shared connective child syntax node.'
        return cast(RecoveredField[BoundTermConnectiveSyntax], self._field(1))
    @property
    def bo(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Bo` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(2))
    @property
    def trailing_term(self) -> RecoveredField[SimpleTermSyntax]:
        'The shared trailing term child syntax node.'
        return cast(RecoveredField[SimpleTermSyntax], self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('BoundTermConnectionSyntax is final')

@final
class BoundTermConnectiveSyntaxJoikConnective(_SyntaxNode):
    'Uses the nested `joik_connective` sum form and preserves its selected alternative.'
    __slots__ = ()
    _schema_id = 157
    __match_args__ = ('joik_connective',)
    def __new__(cls, joik_connective: RecoveredField[JoikConnectiveSyntax]) -> BoundTermConnectiveSyntaxJoikConnective:
        return cls._from_fields((joik_connective,))
    def __init__(self, joik_connective: RecoveredField[JoikConnectiveSyntax]) -> None:
        pass
    @property
    def joik_connective(self) -> RecoveredField[JoikConnectiveSyntax]:
        'Uses the nested `joik_connective` sum form and preserves its selected alternative.'
        return cast(RecoveredField[JoikConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('BoundTermConnectiveSyntaxJoikConnective is final')

@final
class BoundTermConnectiveSyntaxEkConnective(_SyntaxNode):
    'Uses the `ek_connective` product form, whose payload preserves `na`, `se`, `a`, and `nai`.'
    __slots__ = ()
    _schema_id = 158
    __match_args__ = ('ek_connective',)
    def __new__(cls, ek_connective: RecoveredField[EkConnectiveSyntax]) -> BoundTermConnectiveSyntaxEkConnective:
        return cls._from_fields((ek_connective,))
    def __init__(self, ek_connective: RecoveredField[EkConnectiveSyntax]) -> None:
        pass
    @property
    def ek_connective(self) -> RecoveredField[EkConnectiveSyntax]:
        'Uses the `ek_connective` product form, whose payload preserves `na`, `se`, `a`, and `nai`.'
        return cast(RecoveredField[EkConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('BoundTermConnectiveSyntaxEkConnective is final')

BoundTermConnectiveSyntax: TypeAlias = BoundTermConnectiveSyntaxJoikConnective | BoundTermConnectiveSyntaxEkConnective

@final
class ConnectedTermSyntax(_SyntaxNode):
    'Product node for term connection; preserves `leading_term` and `continuations` in source order.'
    __slots__ = ()
    _schema_id = 159
    __match_args__ = ('leading_term', 'continuations')
    def __new__(cls, leading_term: RecoveredField[SimpleTermSyntax], continuations: Sequence[RecoveredField[ConnectedTermContinuationSyntax]]) -> ConnectedTermSyntax:
        return cls._from_fields((leading_term, continuations))
    def __init__(self, leading_term: RecoveredField[SimpleTermSyntax], continuations: Sequence[RecoveredField[ConnectedTermContinuationSyntax]]) -> None:
        pass
    @property
    def leading_term(self) -> RecoveredField[SimpleTermSyntax]:
        'The shared leading term child syntax node.'
        return cast(RecoveredField[SimpleTermSyntax], self._field(0))
    @property
    def continuations(self) -> tuple[RecoveredField[ConnectedTermContinuationSyntax], ...]:
        'Ordered sequence of zero or more continuations components.'
        return cast(tuple[RecoveredField[ConnectedTermContinuationSyntax], ...], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ConnectedTermSyntax is final')

@final
class ConnectedTermContinuationSyntax(_SyntaxNode):
    'Product node for term connection continuation; preserves `connective` and `trailing_term` in source order.'
    __slots__ = ()
    _schema_id = 160
    __match_args__ = ('connective', 'trailing_term')
    def __new__(cls, connective: RecoveredField[ConnectedTermConnectiveSyntax], trailing_term: RecoveredField[SimpleTermSyntax]) -> ConnectedTermContinuationSyntax:
        return cls._from_fields((connective, trailing_term))
    def __init__(self, connective: RecoveredField[ConnectedTermConnectiveSyntax], trailing_term: RecoveredField[SimpleTermSyntax]) -> None:
        pass
    @property
    def connective(self) -> RecoveredField[ConnectedTermConnectiveSyntax]:
        'The `connected_term_connective` connective joining the adjacent constituents of the `connected_term_continuation` production.'
        return cast(RecoveredField[ConnectedTermConnectiveSyntax], self._field(0))
    @property
    def trailing_term(self) -> RecoveredField[SimpleTermSyntax]:
        'The shared trailing term child syntax node.'
        return cast(RecoveredField[SimpleTermSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ConnectedTermContinuationSyntax is final')

@final
class ConnectedTermConnectiveSyntaxJoikConnective(_SyntaxNode):
    'Uses the nested `joik_connective` sum form and preserves its selected alternative.'
    __slots__ = ()
    _schema_id = 161
    __match_args__ = ('joik_connective',)
    def __new__(cls, joik_connective: RecoveredField[JoikConnectiveSyntax]) -> ConnectedTermConnectiveSyntaxJoikConnective:
        return cls._from_fields((joik_connective,))
    def __init__(self, joik_connective: RecoveredField[JoikConnectiveSyntax]) -> None:
        pass
    @property
    def joik_connective(self) -> RecoveredField[JoikConnectiveSyntax]:
        'Uses the nested `joik_connective` sum form and preserves its selected alternative.'
        return cast(RecoveredField[JoikConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ConnectedTermConnectiveSyntaxJoikConnective is final')

@final
class ConnectedTermConnectiveSyntaxJekConnective(_SyntaxNode):
    'Uses the `jek_connective` product form, whose payload preserves `na`, `se`, `ja`, and `nai`.'
    __slots__ = ()
    _schema_id = 162
    __match_args__ = ('jek_connective',)
    def __new__(cls, jek_connective: RecoveredField[JekConnectiveSyntax]) -> ConnectedTermConnectiveSyntaxJekConnective:
        return cls._from_fields((jek_connective,))
    def __init__(self, jek_connective: RecoveredField[JekConnectiveSyntax]) -> None:
        pass
    @property
    def jek_connective(self) -> RecoveredField[JekConnectiveSyntax]:
        'Uses the `jek_connective` product form, whose payload preserves `na`, `se`, `ja`, and `nai`.'
        return cast(RecoveredField[JekConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ConnectedTermConnectiveSyntaxJekConnective is final')

@final
class ConnectedTermConnectiveSyntaxEkConnective(_SyntaxNode):
    'Uses the `ek_connective` product form, whose payload preserves `na`, `se`, `a`, and `nai`.'
    __slots__ = ()
    _schema_id = 163
    __match_args__ = ('ek_connective',)
    def __new__(cls, ek_connective: RecoveredField[EkConnectiveSyntax]) -> ConnectedTermConnectiveSyntaxEkConnective:
        return cls._from_fields((ek_connective,))
    def __init__(self, ek_connective: RecoveredField[EkConnectiveSyntax]) -> None:
        pass
    @property
    def ek_connective(self) -> RecoveredField[EkConnectiveSyntax]:
        'Uses the `ek_connective` product form, whose payload preserves `na`, `se`, `a`, and `nai`.'
        return cast(RecoveredField[EkConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ConnectedTermConnectiveSyntaxEkConnective is final')

@final
class ConnectedTermConnectiveSyntaxVuhuNonlogicalConnective(_SyntaxNode):
    'Uses the `vuhu_nonlogical_connective` product form, whose payload preserves `vuhu`.'
    __slots__ = ()
    _schema_id = 164
    __match_args__ = ('vuhu_nonlogical_connective',)
    def __new__(cls, vuhu_nonlogical_connective: RecoveredField[VuhuNonlogicalConnectiveSyntax]) -> ConnectedTermConnectiveSyntaxVuhuNonlogicalConnective:
        return cls._from_fields((vuhu_nonlogical_connective,))
    def __init__(self, vuhu_nonlogical_connective: RecoveredField[VuhuNonlogicalConnectiveSyntax]) -> None:
        pass
    @property
    def vuhu_nonlogical_connective(self) -> RecoveredField[VuhuNonlogicalConnectiveSyntax]:
        'Uses the `vuhu_nonlogical_connective` product form, whose payload preserves `vuhu`.'
        return cast(RecoveredField[VuhuNonlogicalConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ConnectedTermConnectiveSyntaxVuhuNonlogicalConnective is final')

ConnectedTermConnectiveSyntax: TypeAlias = ConnectedTermConnectiveSyntaxJoikConnective | ConnectedTermConnectiveSyntaxJekConnective | ConnectedTermConnectiveSyntaxEkConnective | ConnectedTermConnectiveSyntaxVuhuNonlogicalConnective

@final
class TermsetGroupSyntax(_SyntaxNode):
    'Product node for termset; preserves `leading_term` and `continuations` in source order.'
    __slots__ = ()
    _schema_id = 165
    __match_args__ = ('leading_term', 'continuations')
    def __new__(cls, leading_term: RecoveredField[SimpleTermSyntax], continuations: Sequence[RecoveredField[TermsetGroupContinuationSyntax]]) -> TermsetGroupSyntax:
        return cls._from_fields((leading_term, continuations))
    def __init__(self, leading_term: RecoveredField[SimpleTermSyntax], continuations: Sequence[RecoveredField[TermsetGroupContinuationSyntax]]) -> None:
        pass
    @property
    def leading_term(self) -> RecoveredField[SimpleTermSyntax]:
        'The shared leading term child syntax node.'
        return cast(RecoveredField[SimpleTermSyntax], self._field(0))
    @property
    def continuations(self) -> tuple[RecoveredField[TermsetGroupContinuationSyntax], ...]:
        'Non-empty ordered sequence of continuations components.'
        return cast(tuple[RecoveredField[TermsetGroupContinuationSyntax], ...], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('TermsetGroupSyntax is final')

@final
class TermsetGroupContinuationSyntax(_SyntaxNode):
    'Product node for termset continuation; preserves `cehe` and `trailing_term` in source order.'
    __slots__ = ()
    _schema_id = 166
    __match_args__ = ('cehe', 'trailing_term')
    def __new__(cls, cehe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], trailing_term: RecoveredField[SimpleTermSyntax]) -> TermsetGroupContinuationSyntax:
        return cls._from_fields((cehe, trailing_term))
    def __init__(self, cehe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], trailing_term: RecoveredField[SimpleTermSyntax]) -> None:
        pass
    @property
    def cehe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Cehe` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def trailing_term(self) -> RecoveredField[SimpleTermSyntax]:
        'The shared trailing term child syntax node.'
        return cast(RecoveredField[SimpleTermSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('TermsetGroupContinuationSyntax is final')

@final
class ForethoughtTermsetSyntax(_SyntaxNode):
    'Product node for termset; preserves `m_nuhi`, `gek`, `terms`, and 4 other fields in source order.'
    __slots__ = ()
    _schema_id = 167
    __match_args__ = ('m_nuhi', 'gek', 'terms', 'nuhu', 'first_branch', 'additional_branches', 'gihi')
    def __new__(cls, m_nuhi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, gek: RecoveredField[ModalForethoughtConnectiveSyntax], terms: Sequence[RecoveredField[TermSyntax]], nuhu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, first_branch: RecoveredField[ForethoughtTermsetBranchSyntax], additional_branches: Sequence[RecoveredField[ZantufaForethoughtTermsetBranchSyntax]], gihi: RecoveredField[Token] | None) -> ForethoughtTermsetSyntax:
        return cls._from_fields((m_nuhi, gek, terms, nuhu, first_branch, additional_branches, gihi))
    def __init__(self, m_nuhi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, gek: RecoveredField[ModalForethoughtConnectiveSyntax], terms: Sequence[RecoveredField[TermSyntax]], nuhu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, first_branch: RecoveredField[ForethoughtTermsetBranchSyntax], additional_branches: Sequence[RecoveredField[ZantufaForethoughtTermsetBranchSyntax]], gihi: RecoveredField[Token] | None) -> None:
        pass
    @property
    def m_nuhi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'An optional NUhI marker introducing the forethought termset before its connective.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(0))
    @property
    def gek(self) -> RecoveredField[ModalForethoughtConnectiveSyntax]:
        'The opening forethought connective that determines how the term sequences are combined.'
        return cast(RecoveredField[ModalForethoughtConnectiveSyntax], self._field(1))
    @property
    def terms(self) -> tuple[RecoveredField[TermSyntax], ...]:
        'The initial nonempty term sequence following the opening connective.'
        return cast(tuple[RecoveredField[TermSyntax], ...], self._field(2))
    @property
    def nuhu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional elidable NUhU terminator closing the initial term sequence.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(3))
    @property
    def first_branch(self) -> RecoveredField[ForethoughtTermsetBranchSyntax]:
        'The first GIK-led term-sequence branch paired with the opening connective.'
        return cast(RecoveredField[ForethoughtTermsetBranchSyntax], self._field(4))
    @property
    def additional_branches(self) -> tuple[RecoveredField[ZantufaForethoughtTermsetBranchSyntax], ...]:
        'Additional Zantufa GIK-led term-sequence branches, retained in source order.'
        return cast(tuple[RecoveredField[ZantufaForethoughtTermsetBranchSyntax], ...], self._field(5))
    @property
    def gihi(self) -> RecoveredField[Token] | None:
        'The optional experimental GIhI terminator following the complete branch sequence.'
        return cast(RecoveredField[Token] | None, self._field(6))
    def __init_subclass__(cls) -> None:
        raise TypeError('ForethoughtTermsetSyntax is final')

@final
class ForethoughtTermsetBranchSyntax(_SyntaxNode):
    'Product node for termset; preserves `gik`, `terms`, and `nuhu` in source order.'
    __slots__ = ()
    _schema_id = 168
    __match_args__ = ('gik', 'terms', 'nuhu')
    def __new__(cls, gik: RecoveredField[GikConnectiveSyntax], terms: Sequence[RecoveredField[TermSyntax]], nuhu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> ForethoughtTermsetBranchSyntax:
        return cls._from_fields((gik, terms, nuhu))
    def __init__(self, gik: RecoveredField[GikConnectiveSyntax], terms: Sequence[RecoveredField[TermSyntax]], nuhu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def gik(self) -> RecoveredField[GikConnectiveSyntax]:
        'The GIK connective that introduces this branch and pairs with the opening forethought connective.'
        return cast(RecoveredField[GikConnectiveSyntax], self._field(0))
    @property
    def terms(self) -> tuple[RecoveredField[TermSyntax], ...]:
        'The nonempty term sequence governed by this branch\'s GIK connective.'
        return cast(tuple[RecoveredField[TermSyntax], ...], self._field(1))
    @property
    def nuhu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional elidable NUhU terminator closing this branch\'s term sequence.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('ForethoughtTermsetBranchSyntax is final')

@final
class ZantufaForethoughtTermsetBranchSyntax(_SyntaxNode):
    'Product node for termset; preserves `gik`, `terms`, and `nuhu` in source order.'
    __slots__ = ()
    _schema_id = 169
    __match_args__ = ('gik', 'terms', 'nuhu')
    def __new__(cls, gik: RecoveredField[ZantufaExtraGikConnectiveSyntax], terms: Sequence[RecoveredField[TermSyntax]], nuhu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> ZantufaForethoughtTermsetBranchSyntax:
        return cls._from_fields((gik, terms, nuhu))
    def __init__(self, gik: RecoveredField[ZantufaExtraGikConnectiveSyntax], terms: Sequence[RecoveredField[TermSyntax]], nuhu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def gik(self) -> RecoveredField[ZantufaExtraGikConnectiveSyntax]:
        'The additional Zantufa GIK connective that introduces this branch.'
        return cast(RecoveredField[ZantufaExtraGikConnectiveSyntax], self._field(0))
    @property
    def terms(self) -> tuple[RecoveredField[TermSyntax], ...]:
        'The nonempty term sequence governed by this additional branch\'s GIK connective.'
        return cast(tuple[RecoveredField[TermSyntax], ...], self._field(1))
    @property
    def nuhu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional elidable NUhU terminator closing this branch\'s term sequence.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaForethoughtTermsetBranchSyntax is final')

@final
class NuhiTermsetSyntax(_SyntaxNode):
    'Product node for termset; preserves `nuhi`, `termset`, and `nuhu` in source order.'
    __slots__ = ()
    _schema_id = 170
    __match_args__ = ('nuhi', 'termset', 'nuhu')
    def __new__(cls, nuhi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], termset: Sequence[RecoveredField[TermSyntax]], nuhu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> NuhiTermsetSyntax:
        return cls._from_fields((nuhi, termset, nuhu))
    def __init__(self, nuhi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], termset: Sequence[RecoveredField[TermSyntax]], nuhu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def nuhi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Nuhi` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def termset(self) -> tuple[RecoveredField[TermSyntax], ...]:
        'Non-empty ordered sequence of termset components.'
        return cast(tuple[RecoveredField[TermSyntax], ...], self._field(1))
    @property
    def nuhu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Nuhu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('NuhiTermsetSyntax is final')

@final
class KeTermsetSyntax(_SyntaxNode):
    'Product node for termset; preserves `ke`, `termset`, and `kehe` in source order.'
    __slots__ = ()
    _schema_id = 171
    __match_args__ = ('ke', 'termset', 'kehe')
    def __new__(cls, ke: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], termset: Sequence[RecoveredField[TermSyntax]], kehe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> KeTermsetSyntax:
        return cls._from_fields((ke, termset, kehe))
    def __init__(self, ke: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], termset: Sequence[RecoveredField[TermSyntax]], kehe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def ke(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Ke` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def termset(self) -> tuple[RecoveredField[TermSyntax], ...]:
        'Non-empty ordered sequence of termset components.'
        return cast(tuple[RecoveredField[TermSyntax], ...], self._field(1))
    @property
    def kehe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Kehe` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('KeTermsetSyntax is final')

@final
class NoihaAdverbialTermSyntaxNoihaVariableAdverbialTerm(_SyntaxNode):
    'Uses the `noiha_variable_adverbial_term` product form, whose payload preserves `poiha`, `free_modifiers`, `selbri`, and `brigahi_ku`.'
    __slots__ = ()
    _schema_id = 172
    __match_args__ = ('noiha_variable_adverbial_term',)
    def __new__(cls, noiha_variable_adverbial_term: RecoveredField[NoihaVariableAdverbialTermSyntax]) -> NoihaAdverbialTermSyntaxNoihaVariableAdverbialTerm:
        return cls._from_fields((noiha_variable_adverbial_term,))
    def __init__(self, noiha_variable_adverbial_term: RecoveredField[NoihaVariableAdverbialTermSyntax]) -> None:
        pass
    @property
    def noiha_variable_adverbial_term(self) -> RecoveredField[NoihaVariableAdverbialTermSyntax]:
        'Uses the `noiha_variable_adverbial_term` product form, whose payload preserves `poiha`, `free_modifiers`, `selbri`, and `brigahi_ku`.'
        return cast(RecoveredField[NoihaVariableAdverbialTermSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('NoihaAdverbialTermSyntaxNoihaVariableAdverbialTerm is final')

@final
class NoihaAdverbialTermSyntaxNoihaRelativeAdverbialTerm(_SyntaxNode):
    'Uses the `noiha_relative_adverbial_term` product form, whose payload preserves `noiha`, `selbri`, and `fehu`.'
    __slots__ = ()
    _schema_id = 173
    __match_args__ = ('noiha_relative_adverbial_term',)
    def __new__(cls, noiha_relative_adverbial_term: RecoveredField[NoihaRelativeAdverbialTermSyntax]) -> NoihaAdverbialTermSyntaxNoihaRelativeAdverbialTerm:
        return cls._from_fields((noiha_relative_adverbial_term,))
    def __init__(self, noiha_relative_adverbial_term: RecoveredField[NoihaRelativeAdverbialTermSyntax]) -> None:
        pass
    @property
    def noiha_relative_adverbial_term(self) -> RecoveredField[NoihaRelativeAdverbialTermSyntax]:
        'Uses the `noiha_relative_adverbial_term` product form, whose payload preserves `noiha`, `selbri`, and `fehu`.'
        return cast(RecoveredField[NoihaRelativeAdverbialTermSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('NoihaAdverbialTermSyntaxNoihaRelativeAdverbialTerm is final')

NoihaAdverbialTermSyntax: TypeAlias = NoihaAdverbialTermSyntaxNoihaVariableAdverbialTerm | NoihaAdverbialTermSyntaxNoihaRelativeAdverbialTerm

@final
class NoihaVariableAdverbialTermSyntax(_SyntaxNode):
    'Product node for NOIhA adverbial; preserves `poiha`, `free_modifiers`, `selbri`, and `brigahi_ku` in source order.'
    __slots__ = ()
    _schema_id = 174
    __match_args__ = ('poiha', 'free_modifiers', 'selbri', 'brigahi_ku')
    def __new__(cls, poiha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], free_modifiers: Sequence[RecoveredField[FreeModifierSyntax]], selbri: RecoveredField[SelbriSyntax], brigahi_ku: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> NoihaVariableAdverbialTermSyntax:
        return cls._from_fields((poiha, free_modifiers, selbri, brigahi_ku))
    def __init__(self, poiha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], free_modifiers: Sequence[RecoveredField[FreeModifierSyntax]], selbri: RecoveredField[SelbriSyntax], brigahi_ku: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def poiha(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Noiha`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def free_modifiers(self) -> tuple[RecoveredField[FreeModifierSyntax], ...]:
        'Ordered sequence of zero or more free modifiers components.'
        return cast(tuple[RecoveredField[FreeModifierSyntax], ...], self._field(1))
    @property
    def selbri(self) -> RecoveredField[SelbriSyntax]:
        'The shared selbri child syntax node.'
        return cast(RecoveredField[SelbriSyntax], self._field(2))
    @property
    def brigahi_ku(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Ku` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('NoihaVariableAdverbialTermSyntax is final')

@final
class NoihaRelativeAdverbialTermSyntax(_SyntaxNode):
    'Product node for NOIhA adverbial; preserves `noiha`, `selbri`, and `fehu` in source order.'
    __slots__ = ()
    _schema_id = 175
    __match_args__ = ('noiha', 'selbri', 'fehu')
    def __new__(cls, noiha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], selbri: RecoveredField[SelbriSyntax], fehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> NoihaRelativeAdverbialTermSyntax:
        return cls._from_fields((noiha, selbri, fehu))
    def __init__(self, noiha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], selbri: RecoveredField[SelbriSyntax], fehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def noiha(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Noiha`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def selbri(self) -> RecoveredField[SelbriSyntax]:
        'The shared selbri child syntax node.'
        return cast(RecoveredField[SelbriSyntax], self._field(1))
    @property
    def fehu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Fehu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('NoihaRelativeAdverbialTermSyntax is final')

@final
class FihoiAdverbialTermSyntax(_SyntaxNode):
    'Product node for FIhOI adverbial; preserves `fihoi`, `statement`, and `fihau` in source order.'
    __slots__ = ()
    _schema_id = 176
    __match_args__ = ('fihoi', 'statement', 'fihau')
    def __new__(cls, fihoi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], statement: RecoveredField[StatementSyntax], fihau: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> FihoiAdverbialTermSyntax:
        return cls._from_fields((fihoi, statement, fihau))
    def __init__(self, fihoi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], statement: RecoveredField[StatementSyntax], fihau: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def fihoi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Fihoi` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def statement(self) -> RecoveredField[StatementSyntax]:
        'The shared statement child syntax node.'
        return cast(RecoveredField[StatementSyntax], self._field(1))
    @property
    def fihau(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Fihau` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('FihoiAdverbialTermSyntax is final')

@final
class SoiAdverbialTermSyntax(_SyntaxNode):
    'Product node for SOI adverbial; preserves `soi`, `statement`, and `sehu` in source order.'
    __slots__ = ()
    _schema_id = 177
    __match_args__ = ('soi', 'statement', 'sehu')
    def __new__(cls, soi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], statement: RecoveredField[StatementSyntax], sehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> SoiAdverbialTermSyntax:
        return cls._from_fields((soi, statement, sehu))
    def __init__(self, soi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], statement: RecoveredField[StatementSyntax], sehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def soi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Soi`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def statement(self) -> RecoveredField[StatementSyntax]:
        'The shared statement child syntax node.'
        return cast(RecoveredField[StatementSyntax], self._field(1))
    @property
    def sehu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Sehu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('SoiAdverbialTermSyntax is final')

@final
class SumtiTermSyntax(_SyntaxNode):
    'Transparent product node for term; preserves the `sumti` component.'
    __slots__ = ()
    _schema_id = 178
    __match_args__ = ('sumti',)
    def __new__(cls, sumti: RecoveredField[SumtiSyntax]) -> SumtiTermSyntax:
        return cls._from_fields((sumti,))
    def __init__(self, sumti: RecoveredField[SumtiSyntax]) -> None:
        pass
    @property
    def sumti(self) -> RecoveredField[SumtiSyntax]:
        'The shared sumti child syntax node.'
        return cast(RecoveredField[SumtiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SumtiTermSyntax is final')

@final
class PlaceTaggedSumtiTermSyntax(_SyntaxNode):
    'Product node for place tag; preserves `fa` and `sumti` in source order.'
    __slots__ = ()
    _schema_id = 179
    __match_args__ = ('fa', 'sumti')
    def __new__(cls, fa: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], sumti: RecoveredField[TaggedOrElidedSumtiSyntax]) -> PlaceTaggedSumtiTermSyntax:
        return cls._from_fields((fa, sumti))
    def __init__(self, fa: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], sumti: RecoveredField[TaggedOrElidedSumtiSyntax]) -> None:
        pass
    @property
    def fa(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Fa`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def sumti(self) -> RecoveredField[TaggedOrElidedSumtiSyntax]:
        'The shared sumti child syntax node.'
        return cast(RecoveredField[TaggedOrElidedSumtiSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('PlaceTaggedSumtiTermSyntax is final')

@final
class NaKuTermSyntax(_SyntaxNode):
    'Product node for NA KU term; preserves `na` and `na_ku` in source order.'
    __slots__ = ()
    _schema_id = 180
    __match_args__ = ('na', 'na_ku')
    def __new__(cls, na: RecoveredField[Token], na_ku: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> NaKuTermSyntax:
        return cls._from_fields((na, na_ku))
    def __init__(self, na: RecoveredField[Token], na_ku: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def na(self) -> RecoveredField[Token]:
        'A word from selmaho `Na`.'
        return cast(RecoveredField[Token], self._field(0))
    @property
    def na_ku(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Ku` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('NaKuTermSyntax is final')

@final
class BareNaTermSyntax(_SyntaxNode):
    'Transparent product node for NA term; preserves the `na` component.'
    __slots__ = ()
    _schema_id = 181
    __match_args__ = ('na',)
    def __new__(cls, na: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> BareNaTermSyntax:
        return cls._from_fields((na,))
    def __init__(self, na: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def na(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Na`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('BareNaTermSyntax is final')

@final
class TaggedSumtiBeforeTagTermSyntax(_SyntaxNode):
    'Transparent product node for tag; preserves the `tense_modal` component.'
    __slots__ = ()
    _schema_id = 182
    __match_args__ = ('tense_modal',)
    def __new__(cls, tense_modal: RecoveredField[LeadingTermTagTenseModalSyntax]) -> TaggedSumtiBeforeTagTermSyntax:
        return cls._from_fields((tense_modal,))
    def __init__(self, tense_modal: RecoveredField[LeadingTermTagTenseModalSyntax]) -> None:
        pass
    @property
    def tense_modal(self) -> RecoveredField[LeadingTermTagTenseModalSyntax]:
        'The shared tense modal child syntax node.'
        return cast(RecoveredField[LeadingTermTagTenseModalSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TaggedSumtiBeforeTagTermSyntax is final')

@final
class TaggedSumtiTermSyntax(_SyntaxNode):
    'Product node for tag; preserves `tense_modal` and `sumti` in source order.'
    __slots__ = ()
    _schema_id = 183
    __match_args__ = ('tense_modal', 'sumti')
    def __new__(cls, tense_modal: RecoveredField[LeadingTermTagTenseModalSyntax], sumti: RecoveredField[TaggedOrElidedSumtiSyntax]) -> TaggedSumtiTermSyntax:
        return cls._from_fields((tense_modal, sumti))
    def __init__(self, tense_modal: RecoveredField[LeadingTermTagTenseModalSyntax], sumti: RecoveredField[TaggedOrElidedSumtiSyntax]) -> None:
        pass
    @property
    def tense_modal(self) -> RecoveredField[LeadingTermTagTenseModalSyntax]:
        'The shared tense modal child syntax node.'
        return cast(RecoveredField[LeadingTermTagTenseModalSyntax], self._field(0))
    @property
    def sumti(self) -> RecoveredField[TaggedOrElidedSumtiSyntax]:
        'The shared sumti child syntax node.'
        return cast(RecoveredField[TaggedOrElidedSumtiSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('TaggedSumtiTermSyntax is final')

@final
class JaiTaggedSumtiTermSyntax(_SyntaxNode):
    'Product node for tag; preserves `jai`, `tag`, and `sumti` in source order.'
    __slots__ = ()
    _schema_id = 184
    __match_args__ = ('jai', 'tag', 'sumti')
    def __new__(cls, jai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], tag: RecoveredField[TenseModalSyntax] | None, sumti: RecoveredField[SumtiSyntax]) -> JaiTaggedSumtiTermSyntax:
        return cls._from_fields((jai, tag, sumti))
    def __init__(self, jai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], tag: RecoveredField[TenseModalSyntax] | None, sumti: RecoveredField[SumtiSyntax]) -> None:
        pass
    @property
    def jai(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Jai` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def tag(self) -> RecoveredField[TenseModalSyntax] | None:
        'The optional tag component.'
        return cast(RecoveredField[TenseModalSyntax] | None, self._field(1))
    @property
    def sumti(self) -> RecoveredField[SumtiSyntax]:
        'The shared sumti child syntax node.'
        return cast(RecoveredField[SumtiSyntax], self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('JaiTaggedSumtiTermSyntax is final')

@final
class LeadingTermTagTenseModalSyntaxPuBeforeNaheLeadingTermTagTense(_SyntaxNode):
    'Uses the `pu_before_nahe_leading_term_tag_tense` product form, whose payload preserves `pu` and `nai`.'
    __slots__ = ()
    _schema_id = 185
    __match_args__ = ('pu_before_nahe_leading_term_tag_tense',)
    def __new__(cls, pu_before_nahe_leading_term_tag_tense: RecoveredField[PuBeforeNaheLeadingTermTagTenseSyntax]) -> LeadingTermTagTenseModalSyntaxPuBeforeNaheLeadingTermTagTense:
        return cls._from_fields((pu_before_nahe_leading_term_tag_tense,))
    def __init__(self, pu_before_nahe_leading_term_tag_tense: RecoveredField[PuBeforeNaheLeadingTermTagTenseSyntax]) -> None:
        pass
    @property
    def pu_before_nahe_leading_term_tag_tense(self) -> RecoveredField[PuBeforeNaheLeadingTermTagTenseSyntax]:
        'Uses the `pu_before_nahe_leading_term_tag_tense` product form, whose payload preserves `pu` and `nai`.'
        return cast(RecoveredField[PuBeforeNaheLeadingTermTagTenseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('LeadingTermTagTenseModalSyntaxPuBeforeNaheLeadingTermTagTense is final')

@final
class LeadingTermTagTenseModalSyntaxPuDistanceBeforeTagLeadingTermTagTense(_SyntaxNode):
    'Uses the `pu_distance_before_tag_leading_term_tag_tense` product form, whose payload preserves `pu`, `nai`, and `distance`.'
    __slots__ = ()
    _schema_id = 186
    __match_args__ = ('pu_distance_before_tag_leading_term_tag_tense',)
    def __new__(cls, pu_distance_before_tag_leading_term_tag_tense: RecoveredField[PuDistanceBeforeTagLeadingTermTagTenseSyntax]) -> LeadingTermTagTenseModalSyntaxPuDistanceBeforeTagLeadingTermTagTense:
        return cls._from_fields((pu_distance_before_tag_leading_term_tag_tense,))
    def __init__(self, pu_distance_before_tag_leading_term_tag_tense: RecoveredField[PuDistanceBeforeTagLeadingTermTagTenseSyntax]) -> None:
        pass
    @property
    def pu_distance_before_tag_leading_term_tag_tense(self) -> RecoveredField[PuDistanceBeforeTagLeadingTermTagTenseSyntax]:
        'Uses the `pu_distance_before_tag_leading_term_tag_tense` product form, whose payload preserves `pu`, `nai`, and `distance`.'
        return cast(RecoveredField[PuDistanceBeforeTagLeadingTermTagTenseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('LeadingTermTagTenseModalSyntaxPuDistanceBeforeTagLeadingTermTagTense is final')

@final
class LeadingTermTagTenseModalSyntaxZiBeforeZiLeadingTermTagTense(_SyntaxNode):
    'Uses the `zi_before_zi_leading_term_tag_tense` product form, whose payload preserves `zi`.'
    __slots__ = ()
    _schema_id = 187
    __match_args__ = ('zi_before_zi_leading_term_tag_tense',)
    def __new__(cls, zi_before_zi_leading_term_tag_tense: RecoveredField[ZiBeforeZiLeadingTermTagTenseSyntax]) -> LeadingTermTagTenseModalSyntaxZiBeforeZiLeadingTermTagTense:
        return cls._from_fields((zi_before_zi_leading_term_tag_tense,))
    def __init__(self, zi_before_zi_leading_term_tag_tense: RecoveredField[ZiBeforeZiLeadingTermTagTenseSyntax]) -> None:
        pass
    @property
    def zi_before_zi_leading_term_tag_tense(self) -> RecoveredField[ZiBeforeZiLeadingTermTagTenseSyntax]:
        'Uses the `zi_before_zi_leading_term_tag_tense` product form, whose payload preserves `zi`.'
        return cast(RecoveredField[ZiBeforeZiLeadingTermTagTenseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('LeadingTermTagTenseModalSyntaxZiBeforeZiLeadingTermTagTense is final')

@final
class LeadingTermTagTenseModalSyntaxVaBeforeVaLeadingTermTagTense(_SyntaxNode):
    'Uses the `va_before_va_leading_term_tag_tense` product form, whose payload preserves `va`.'
    __slots__ = ()
    _schema_id = 188
    __match_args__ = ('va_before_va_leading_term_tag_tense',)
    def __new__(cls, va_before_va_leading_term_tag_tense: RecoveredField[VaBeforeVaLeadingTermTagTenseSyntax]) -> LeadingTermTagTenseModalSyntaxVaBeforeVaLeadingTermTagTense:
        return cls._from_fields((va_before_va_leading_term_tag_tense,))
    def __init__(self, va_before_va_leading_term_tag_tense: RecoveredField[VaBeforeVaLeadingTermTagTenseSyntax]) -> None:
        pass
    @property
    def va_before_va_leading_term_tag_tense(self) -> RecoveredField[VaBeforeVaLeadingTermTagTenseSyntax]:
        'Uses the `va_before_va_leading_term_tag_tense` product form, whose payload preserves `va`.'
        return cast(RecoveredField[VaBeforeVaLeadingTermTagTenseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('LeadingTermTagTenseModalSyntaxVaBeforeVaLeadingTermTagTense is final')

@final
class LeadingTermTagTenseModalSyntaxMohiBeforeMohiLeadingTermTagTense(_SyntaxNode):
    'Uses the `mohi_before_mohi_leading_term_tag_tense` product form, whose payload preserves `mohi`, `direction`, `nai`, and `distance`.'
    __slots__ = ()
    _schema_id = 189
    __match_args__ = ('mohi_before_mohi_leading_term_tag_tense',)
    def __new__(cls, mohi_before_mohi_leading_term_tag_tense: RecoveredField[MohiBeforeMohiLeadingTermTagTenseSyntax]) -> LeadingTermTagTenseModalSyntaxMohiBeforeMohiLeadingTermTagTense:
        return cls._from_fields((mohi_before_mohi_leading_term_tag_tense,))
    def __init__(self, mohi_before_mohi_leading_term_tag_tense: RecoveredField[MohiBeforeMohiLeadingTermTagTenseSyntax]) -> None:
        pass
    @property
    def mohi_before_mohi_leading_term_tag_tense(self) -> RecoveredField[MohiBeforeMohiLeadingTermTagTenseSyntax]:
        'Uses the `mohi_before_mohi_leading_term_tag_tense` product form, whose payload preserves `mohi`, `direction`, `nai`, and `distance`.'
        return cast(RecoveredField[MohiBeforeMohiLeadingTermTagTenseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('LeadingTermTagTenseModalSyntaxMohiBeforeMohiLeadingTermTagTense is final')

@final
class LeadingTermTagTenseModalSyntaxCahaBeforeTagLeadingTermTagTense(_SyntaxNode):
    'Uses the `caha_before_tag_leading_term_tag_tense` product form, whose payload preserves `caha`.'
    __slots__ = ()
    _schema_id = 190
    __match_args__ = ('caha_before_tag_leading_term_tag_tense',)
    def __new__(cls, caha_before_tag_leading_term_tag_tense: RecoveredField[CahaBeforeTagLeadingTermTagTenseSyntax]) -> LeadingTermTagTenseModalSyntaxCahaBeforeTagLeadingTermTagTense:
        return cls._from_fields((caha_before_tag_leading_term_tag_tense,))
    def __init__(self, caha_before_tag_leading_term_tag_tense: RecoveredField[CahaBeforeTagLeadingTermTagTenseSyntax]) -> None:
        pass
    @property
    def caha_before_tag_leading_term_tag_tense(self) -> RecoveredField[CahaBeforeTagLeadingTermTagTenseSyntax]:
        'Uses the `caha_before_tag_leading_term_tag_tense` product form, whose payload preserves `caha`.'
        return cast(RecoveredField[CahaBeforeTagLeadingTermTagTenseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('LeadingTermTagTenseModalSyntaxCahaBeforeTagLeadingTermTagTense is final')

@final
class LeadingTermTagTenseModalSyntaxIntervalPropertyLeadingTermTagTense(_SyntaxNode):
    'Uses the `interval_property_leading_term_tag_tense` product form, whose payload preserves `property`.'
    __slots__ = ()
    _schema_id = 191
    __match_args__ = ('interval_property_leading_term_tag_tense',)
    def __new__(cls, interval_property_leading_term_tag_tense: RecoveredField[IntervalPropertyLeadingTermTagTenseSyntax]) -> LeadingTermTagTenseModalSyntaxIntervalPropertyLeadingTermTagTense:
        return cls._from_fields((interval_property_leading_term_tag_tense,))
    def __init__(self, interval_property_leading_term_tag_tense: RecoveredField[IntervalPropertyLeadingTermTagTenseSyntax]) -> None:
        pass
    @property
    def interval_property_leading_term_tag_tense(self) -> RecoveredField[IntervalPropertyLeadingTermTagTenseSyntax]:
        'Uses the `interval_property_leading_term_tag_tense` product form, whose payload preserves `property`.'
        return cast(RecoveredField[IntervalPropertyLeadingTermTagTenseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('LeadingTermTagTenseModalSyntaxIntervalPropertyLeadingTermTagTense is final')

@final
class LeadingTermTagTenseModalSyntaxTenseModal(_SyntaxNode):
    'Uses the `tense_modal` product form, whose payload preserves `body`.'
    __slots__ = ()
    _schema_id = 192
    __match_args__ = ('tense_modal',)
    def __new__(cls, tense_modal: RecoveredField[TenseModalSyntax]) -> LeadingTermTagTenseModalSyntaxTenseModal:
        return cls._from_fields((tense_modal,))
    def __init__(self, tense_modal: RecoveredField[TenseModalSyntax]) -> None:
        pass
    @property
    def tense_modal(self) -> RecoveredField[TenseModalSyntax]:
        'Uses the `tense_modal` product form, whose payload preserves `body`.'
        return cast(RecoveredField[TenseModalSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('LeadingTermTagTenseModalSyntaxTenseModal is final')

LeadingTermTagTenseModalSyntax: TypeAlias = LeadingTermTagTenseModalSyntaxPuBeforeNaheLeadingTermTagTense | LeadingTermTagTenseModalSyntaxPuDistanceBeforeTagLeadingTermTagTense | LeadingTermTagTenseModalSyntaxZiBeforeZiLeadingTermTagTense | LeadingTermTagTenseModalSyntaxVaBeforeVaLeadingTermTagTense | LeadingTermTagTenseModalSyntaxMohiBeforeMohiLeadingTermTagTense | LeadingTermTagTenseModalSyntaxCahaBeforeTagLeadingTermTagTense | LeadingTermTagTenseModalSyntaxIntervalPropertyLeadingTermTagTense | LeadingTermTagTenseModalSyntaxTenseModal

@final
class PuBeforeNaheLeadingTermTagTenseSyntax(_SyntaxNode):
    'Product node for tag; preserves `pu` and `nai` in source order.'
    __slots__ = ()
    _schema_id = 193
    __match_args__ = ('pu', 'nai')
    def __new__(cls, pu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> PuBeforeNaheLeadingTermTagTenseSyntax:
        return cls._from_fields((pu, nai))
    def __init__(self, pu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def pu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Pu`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def nai(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Nai` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('PuBeforeNaheLeadingTermTagTenseSyntax is final')

@final
class PuDistanceBeforeTagLeadingTermTagTenseSyntax(_SyntaxNode):
    'Product node for tag; preserves `pu`, `nai`, and `distance` in source order.'
    __slots__ = ()
    _schema_id = 194
    __match_args__ = ('pu', 'nai', 'distance')
    def __new__(cls, pu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, distance: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> PuDistanceBeforeTagLeadingTermTagTenseSyntax:
        return cls._from_fields((pu, nai, distance))
    def __init__(self, pu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, distance: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def pu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Pu`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def nai(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Nai` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(1))
    @property
    def distance(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Zi`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('PuDistanceBeforeTagLeadingTermTagTenseSyntax is final')

@final
class ZiBeforeZiLeadingTermTagTenseSyntax(_SyntaxNode):
    'Transparent product node for tag; preserves the `zi` component.'
    __slots__ = ()
    _schema_id = 195
    __match_args__ = ('zi',)
    def __new__(cls, zi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> ZiBeforeZiLeadingTermTagTenseSyntax:
        return cls._from_fields((zi,))
    def __init__(self, zi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def zi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Zi`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZiBeforeZiLeadingTermTagTenseSyntax is final')

@final
class VaBeforeVaLeadingTermTagTenseSyntax(_SyntaxNode):
    'Transparent product node for tag; preserves the `va` component.'
    __slots__ = ()
    _schema_id = 196
    __match_args__ = ('va',)
    def __new__(cls, va: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> VaBeforeVaLeadingTermTagTenseSyntax:
        return cls._from_fields((va,))
    def __init__(self, va: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def va(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Va`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('VaBeforeVaLeadingTermTagTenseSyntax is final')

@final
class MohiBeforeMohiLeadingTermTagTenseSyntax(_SyntaxNode):
    'Product node for tag; preserves `mohi`, `direction`, `nai`, and `distance` in source order.'
    __slots__ = ()
    _schema_id = 197
    __match_args__ = ('mohi', 'direction', 'nai', 'distance')
    def __new__(cls, mohi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], direction: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, distance: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> MohiBeforeMohiLeadingTermTagTenseSyntax:
        return cls._from_fields((mohi, direction, nai, distance))
    def __init__(self, mohi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], direction: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, distance: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def mohi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Mohi`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def direction(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Faha`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(1))
    @property
    def nai(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Nai` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    @property
    def distance(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional distance component.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('MohiBeforeMohiLeadingTermTagTenseSyntax is final')

@final
class CahaBeforeTagLeadingTermTagTenseSyntax(_SyntaxNode):
    'Transparent product node for tag; preserves the `caha` component.'
    __slots__ = ()
    _schema_id = 198
    __match_args__ = ('caha',)
    def __new__(cls, caha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> CahaBeforeTagLeadingTermTagTenseSyntax:
        return cls._from_fields((caha,))
    def __init__(self, caha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def caha(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Caha`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('CahaBeforeTagLeadingTermTagTenseSyntax is final')

@final
class IntervalPropertyLeadingTermTagTenseSyntax(_SyntaxNode):
    'Transparent product node for interval property; preserves the `property` component.'
    __slots__ = ()
    _schema_id = 199
    __match_args__ = ('property',)
    def __new__(cls, property: RecoveredField[IntervalPropertyTenseSyntax]) -> IntervalPropertyLeadingTermTagTenseSyntax:
        return cls._from_fields((property,))
    def __init__(self, property: RecoveredField[IntervalPropertyTenseSyntax]) -> None:
        pass
    @property
    def property(self) -> RecoveredField[IntervalPropertyTenseSyntax]:
        'The shared property child syntax node.'
        return cast(RecoveredField[IntervalPropertyTenseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('IntervalPropertyLeadingTermTagTenseSyntax is final')

@final
class TaggedOrElidedSumtiSyntaxSumti(_SyntaxNode):
    'Uses the `sumti` product form, whose payload preserves `base_sumti` and `vuho_attachment`.'
    __slots__ = ()
    _schema_id = 200
    __match_args__ = ('sumti',)
    def __new__(cls, sumti: RecoveredField[SumtiSyntax]) -> TaggedOrElidedSumtiSyntaxSumti:
        return cls._from_fields((sumti,))
    def __init__(self, sumti: RecoveredField[SumtiSyntax]) -> None:
        pass
    @property
    def sumti(self) -> RecoveredField[SumtiSyntax]:
        'Uses the `sumti` product form, whose payload preserves `base_sumti` and `vuho_attachment`.'
        return cast(RecoveredField[SumtiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TaggedOrElidedSumtiSyntaxSumti is final')

@final
class TaggedOrElidedSumtiSyntaxTaggedElidedSumti(_SyntaxNode):
    'Uses the `tagged_elided_sumti` product form, whose payload preserves `maybe_ku`.'
    __slots__ = ()
    _schema_id = 201
    __match_args__ = ('tagged_elided_sumti',)
    def __new__(cls, tagged_elided_sumti: RecoveredField[TaggedElidedSumtiSyntax]) -> TaggedOrElidedSumtiSyntaxTaggedElidedSumti:
        return cls._from_fields((tagged_elided_sumti,))
    def __init__(self, tagged_elided_sumti: RecoveredField[TaggedElidedSumtiSyntax]) -> None:
        pass
    @property
    def tagged_elided_sumti(self) -> RecoveredField[TaggedElidedSumtiSyntax]:
        'Uses the `tagged_elided_sumti` product form, whose payload preserves `maybe_ku`.'
        return cast(RecoveredField[TaggedElidedSumtiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TaggedOrElidedSumtiSyntaxTaggedElidedSumti is final')

TaggedOrElidedSumtiSyntax: TypeAlias = TaggedOrElidedSumtiSyntaxSumti | TaggedOrElidedSumtiSyntaxTaggedElidedSumti

@final
class TaggedElidedSumtiSyntax(_SyntaxNode):
    'Transparent product node for elided sumti; preserves the `maybe_ku` component.'
    __slots__ = ()
    _schema_id = 202
    __match_args__ = ('maybe_ku',)
    def __new__(cls, maybe_ku: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> TaggedElidedSumtiSyntax:
        return cls._from_fields((maybe_ku,))
    def __init__(self, maybe_ku: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def maybe_ku(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Ku` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TaggedElidedSumtiSyntax is final')

@final
class SumtiSyntax(_SyntaxNode):
    'Product node for sumti; preserves `base_sumti` and `vuho_attachment` in source order.'
    __slots__ = ()
    _schema_id = 203
    __match_args__ = ('base_sumti', 'vuho_attachment')
    def __new__(cls, base_sumti: RecoveredField[SumtiGroupedSyntax], vuho_attachment: RecoveredField[VuhoSumtiAttachmentTailSyntax] | None) -> SumtiSyntax:
        return cls._from_fields((base_sumti, vuho_attachment))
    def __init__(self, base_sumti: RecoveredField[SumtiGroupedSyntax], vuho_attachment: RecoveredField[VuhoSumtiAttachmentTailSyntax] | None) -> None:
        pass
    @property
    def base_sumti(self) -> RecoveredField[SumtiGroupedSyntax]:
        'The shared base sumti child syntax node.'
        return cast(RecoveredField[SumtiGroupedSyntax], self._field(0))
    @property
    def vuho_attachment(self) -> RecoveredField[VuhoSumtiAttachmentTailSyntax] | None:
        'The optional vuho attachment component.'
        return cast(RecoveredField[VuhoSumtiAttachmentTailSyntax] | None, self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('SumtiSyntax is final')

@final
class SumtiGroupedSyntax(_SyntaxNode):
    'Product node for sumti connection; preserves `leading_sumti` and `grouped_tail` in source order.'
    __slots__ = ()
    _schema_id = 204
    __match_args__ = ('leading_sumti', 'grouped_tail')
    def __new__(cls, leading_sumti: RecoveredField[SumtiAfterthoughtSyntax], grouped_tail: RecoveredField[GroupedSumtiTailSyntax] | None) -> SumtiGroupedSyntax:
        return cls._from_fields((leading_sumti, grouped_tail))
    def __init__(self, leading_sumti: RecoveredField[SumtiAfterthoughtSyntax], grouped_tail: RecoveredField[GroupedSumtiTailSyntax] | None) -> None:
        pass
    @property
    def leading_sumti(self) -> RecoveredField[SumtiAfterthoughtSyntax]:
        'The shared leading sumti child syntax node.'
        return cast(RecoveredField[SumtiAfterthoughtSyntax], self._field(0))
    @property
    def grouped_tail(self) -> RecoveredField[GroupedSumtiTailSyntax] | None:
        'The optional grouped tail component.'
        return cast(RecoveredField[GroupedSumtiTailSyntax] | None, self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('SumtiGroupedSyntax is final')

@final
class SumtiAfterthoughtSyntax(_SyntaxNode):
    'Product node for sumti connection; preserves `leading_sumti` and `continuations` in source order.'
    __slots__ = ()
    _schema_id = 205
    __match_args__ = ('leading_sumti', 'continuations')
    def __new__(cls, leading_sumti: RecoveredField[SumtiBoundSyntax], continuations: Sequence[RecoveredField[SumtiAfterthoughtTailSyntax]]) -> SumtiAfterthoughtSyntax:
        return cls._from_fields((leading_sumti, continuations))
    def __init__(self, leading_sumti: RecoveredField[SumtiBoundSyntax], continuations: Sequence[RecoveredField[SumtiAfterthoughtTailSyntax]]) -> None:
        pass
    @property
    def leading_sumti(self) -> RecoveredField[SumtiBoundSyntax]:
        'The shared leading sumti child syntax node.'
        return cast(RecoveredField[SumtiBoundSyntax], self._field(0))
    @property
    def continuations(self) -> tuple[RecoveredField[SumtiAfterthoughtTailSyntax], ...]:
        'Ordered sequence of zero or more continuations components.'
        return cast(tuple[RecoveredField[SumtiAfterthoughtTailSyntax], ...], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('SumtiAfterthoughtSyntax is final')

@final
class SumtiBoundSyntax(_SyntaxNode):
    'Product node for sumti connection; preserves `leading_sumti` and `bound_tail` in source order.'
    __slots__ = ()
    _schema_id = 206
    __match_args__ = ('leading_sumti', 'bound_tail')
    def __new__(cls, leading_sumti: RecoveredField[SumtiForethoughtSyntax], bound_tail: RecoveredField[BoundSumtiTailSyntax] | None) -> SumtiBoundSyntax:
        return cls._from_fields((leading_sumti, bound_tail))
    def __init__(self, leading_sumti: RecoveredField[SumtiForethoughtSyntax], bound_tail: RecoveredField[BoundSumtiTailSyntax] | None) -> None:
        pass
    @property
    def leading_sumti(self) -> RecoveredField[SumtiForethoughtSyntax]:
        'The shared leading sumti child syntax node.'
        return cast(RecoveredField[SumtiForethoughtSyntax], self._field(0))
    @property
    def bound_tail(self) -> RecoveredField[BoundSumtiTailSyntax] | None:
        'The optional bound tail component.'
        return cast(RecoveredField[BoundSumtiTailSyntax] | None, self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('SumtiBoundSyntax is final')

@final
class SumtiForethoughtSyntaxForethoughtSumti(_SyntaxNode):
    'Uses the `forethought_sumti` product form, whose payload preserves `gek`, `leading_sumti`, `first_branch`, `additional_branches`, and `gihi`.'
    __slots__ = ()
    _schema_id = 207
    __match_args__ = ('forethought_sumti',)
    def __new__(cls, forethought_sumti: RecoveredField[ForethoughtSumtiSyntax]) -> SumtiForethoughtSyntaxForethoughtSumti:
        return cls._from_fields((forethought_sumti,))
    def __init__(self, forethought_sumti: RecoveredField[ForethoughtSumtiSyntax]) -> None:
        pass
    @property
    def forethought_sumti(self) -> RecoveredField[ForethoughtSumtiSyntax]:
        'Uses the `forethought_sumti` product form, whose payload preserves `gek`, `leading_sumti`, `first_branch`, `additional_branches`, and `gihi`.'
        return cast(RecoveredField[ForethoughtSumtiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SumtiForethoughtSyntaxForethoughtSumti is final')

@final
class SumtiForethoughtSyntaxSimpleSumti(_SyntaxNode):
    'Uses the `simple_sumti` product form, whose payload preserves `base_sumti` and `relative_clauses`.'
    __slots__ = ()
    _schema_id = 208
    __match_args__ = ('simple_sumti',)
    def __new__(cls, simple_sumti: RecoveredField[SimpleSumtiSyntax]) -> SumtiForethoughtSyntaxSimpleSumti:
        return cls._from_fields((simple_sumti,))
    def __init__(self, simple_sumti: RecoveredField[SimpleSumtiSyntax]) -> None:
        pass
    @property
    def simple_sumti(self) -> RecoveredField[SimpleSumtiSyntax]:
        'Uses the `simple_sumti` product form, whose payload preserves `base_sumti` and `relative_clauses`.'
        return cast(RecoveredField[SimpleSumtiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SumtiForethoughtSyntaxSimpleSumti is final')

SumtiForethoughtSyntax: TypeAlias = SumtiForethoughtSyntaxForethoughtSumti | SumtiForethoughtSyntaxSimpleSumti

@final
class ForethoughtSumtiSyntax(_SyntaxNode):
    'Product node for forethought sumti connection; preserves `gek`, `leading_sumti`, `first_branch`, `additional_branches`, and `gihi` in source order.'
    __slots__ = ()
    _schema_id = 209
    __match_args__ = ('gek', 'leading_sumti', 'first_branch', 'additional_branches', 'gihi')
    def __new__(cls, gek: RecoveredField[ModalForethoughtConnectiveSyntax], leading_sumti: RecoveredField[SumtiSyntax], first_branch: RecoveredField[ForethoughtSumtiBranchSyntax], additional_branches: Sequence[RecoveredField[ZantufaForethoughtSumtiBranchSyntax]], gihi: RecoveredField[Token] | None) -> ForethoughtSumtiSyntax:
        return cls._from_fields((gek, leading_sumti, first_branch, additional_branches, gihi))
    def __init__(self, gek: RecoveredField[ModalForethoughtConnectiveSyntax], leading_sumti: RecoveredField[SumtiSyntax], first_branch: RecoveredField[ForethoughtSumtiBranchSyntax], additional_branches: Sequence[RecoveredField[ZantufaForethoughtSumtiBranchSyntax]], gihi: RecoveredField[Token] | None) -> None:
        pass
    @property
    def gek(self) -> RecoveredField[ModalForethoughtConnectiveSyntax]:
        'The opening forethought connective that determines how the sumti branches are combined.'
        return cast(RecoveredField[ModalForethoughtConnectiveSyntax], self._field(0))
    @property
    def leading_sumti(self) -> RecoveredField[SumtiSyntax]:
        'The first sumti branch, which follows the opening connective without an intervening GIK.'
        return cast(RecoveredField[SumtiSyntax], self._field(1))
    @property
    def first_branch(self) -> RecoveredField[ForethoughtSumtiBranchSyntax]:
        'The first GIK-led sumti branch paired with the opening connective.'
        return cast(RecoveredField[ForethoughtSumtiBranchSyntax], self._field(2))
    @property
    def additional_branches(self) -> tuple[RecoveredField[ZantufaForethoughtSumtiBranchSyntax], ...]:
        'Additional Zantufa GIK-led sumti branches, retained in source order.'
        return cast(tuple[RecoveredField[ZantufaForethoughtSumtiBranchSyntax], ...], self._field(3))
    @property
    def gihi(self) -> RecoveredField[Token] | None:
        'The optional experimental GIhI terminator following the complete branch sequence.'
        return cast(RecoveredField[Token] | None, self._field(4))
    def __init_subclass__(cls) -> None:
        raise TypeError('ForethoughtSumtiSyntax is final')

@final
class ForethoughtSumtiBranchSyntax(_SyntaxNode):
    'Product node for forethought sumti connection; preserves `gik` and `sumti` in source order.'
    __slots__ = ()
    _schema_id = 210
    __match_args__ = ('gik', 'sumti')
    def __new__(cls, gik: RecoveredField[GikConnectiveSyntax], sumti: RecoveredField[SumtiForethoughtSyntax]) -> ForethoughtSumtiBranchSyntax:
        return cls._from_fields((gik, sumti))
    def __init__(self, gik: RecoveredField[GikConnectiveSyntax], sumti: RecoveredField[SumtiForethoughtSyntax]) -> None:
        pass
    @property
    def gik(self) -> RecoveredField[GikConnectiveSyntax]:
        'The GIK connective that introduces this branch and pairs with the opening forethought connective.'
        return cast(RecoveredField[GikConnectiveSyntax], self._field(0))
    @property
    def sumti(self) -> RecoveredField[SumtiForethoughtSyntax]:
        'The sumti governed by this branch\'s GIK connective.'
        return cast(RecoveredField[SumtiForethoughtSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ForethoughtSumtiBranchSyntax is final')

@final
class ZantufaForethoughtSumtiBranchSyntax(_SyntaxNode):
    'Product node for forethought sumti connection; preserves `gik` and `sumti` in source order.'
    __slots__ = ()
    _schema_id = 211
    __match_args__ = ('gik', 'sumti')
    def __new__(cls, gik: RecoveredField[ZantufaExtraGikConnectiveSyntax], sumti: RecoveredField[SumtiForethoughtSyntax]) -> ZantufaForethoughtSumtiBranchSyntax:
        return cls._from_fields((gik, sumti))
    def __init__(self, gik: RecoveredField[ZantufaExtraGikConnectiveSyntax], sumti: RecoveredField[SumtiForethoughtSyntax]) -> None:
        pass
    @property
    def gik(self) -> RecoveredField[ZantufaExtraGikConnectiveSyntax]:
        'The additional Zantufa GIK connective that introduces this branch.'
        return cast(RecoveredField[ZantufaExtraGikConnectiveSyntax], self._field(0))
    @property
    def sumti(self) -> RecoveredField[SumtiForethoughtSyntax]:
        'The sumti governed by this additional branch\'s GIK connective.'
        return cast(RecoveredField[SumtiForethoughtSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaForethoughtSumtiBranchSyntax is final')

@final
class BoundSumtiTailSyntax(_SyntaxNode):
    'Product node for sumti connection; preserves `connective`, `tense_modal`, `bo`, and `trailing_sumti` in source order.'
    __slots__ = ()
    _schema_id = 212
    __match_args__ = ('connective', 'tense_modal', 'bo', 'trailing_sumti')
    def __new__(cls, connective: RecoveredField[ArgumentConnectiveSyntax], tense_modal: RecoveredField[TenseModalSyntax] | None, bo: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], trailing_sumti: RecoveredField[SumtiBoundSyntax]) -> BoundSumtiTailSyntax:
        return cls._from_fields((connective, tense_modal, bo, trailing_sumti))
    def __init__(self, connective: RecoveredField[ArgumentConnectiveSyntax], tense_modal: RecoveredField[TenseModalSyntax] | None, bo: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], trailing_sumti: RecoveredField[SumtiBoundSyntax]) -> None:
        pass
    @property
    def connective(self) -> RecoveredField[ArgumentConnectiveSyntax]:
        'The shared connective child syntax node.'
        return cast(RecoveredField[ArgumentConnectiveSyntax], self._field(0))
    @property
    def tense_modal(self) -> RecoveredField[TenseModalSyntax] | None:
        'The optional tense modal component.'
        return cast(RecoveredField[TenseModalSyntax] | None, self._field(1))
    @property
    def bo(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Bo` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(2))
    @property
    def trailing_sumti(self) -> RecoveredField[SumtiBoundSyntax]:
        'The shared trailing sumti child syntax node.'
        return cast(RecoveredField[SumtiBoundSyntax], self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('BoundSumtiTailSyntax is final')

@final
class SumtiAfterthoughtTailSyntax(_SyntaxNode):
    'Product node for sumti connective; preserves `connective` and `sumti` in source order.'
    __slots__ = ()
    _schema_id = 213
    __match_args__ = ('connective', 'sumti')
    def __new__(cls, connective: RecoveredField[ArgumentConnectiveSyntax], sumti: RecoveredField[SumtiBoundSyntax]) -> SumtiAfterthoughtTailSyntax:
        return cls._from_fields((connective, sumti))
    def __init__(self, connective: RecoveredField[ArgumentConnectiveSyntax], sumti: RecoveredField[SumtiBoundSyntax]) -> None:
        pass
    @property
    def connective(self) -> RecoveredField[ArgumentConnectiveSyntax]:
        'The `argument_connective` connective joining the adjacent constituents of the `sumti_afterthought_tail` production.'
        return cast(RecoveredField[ArgumentConnectiveSyntax], self._field(0))
    @property
    def sumti(self) -> RecoveredField[SumtiBoundSyntax]:
        'The shared sumti child syntax node.'
        return cast(RecoveredField[SumtiBoundSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('SumtiAfterthoughtTailSyntax is final')

@final
class GroupedSumtiTailSyntax(_SyntaxNode):
    'Product node for sumti connection; preserves `connective`, `tense_modal`, `ke`, `inner_sumti`, and `kehe` in source order.'
    __slots__ = ()
    _schema_id = 214
    __match_args__ = ('connective', 'tense_modal', 'ke', 'inner_sumti', 'kehe')
    def __new__(cls, connective: RecoveredField[ArgumentConnectiveSyntax], tense_modal: RecoveredField[TenseModalSyntax] | None, ke: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_sumti: RecoveredField[SumtiSyntax], kehe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> GroupedSumtiTailSyntax:
        return cls._from_fields((connective, tense_modal, ke, inner_sumti, kehe))
    def __init__(self, connective: RecoveredField[ArgumentConnectiveSyntax], tense_modal: RecoveredField[TenseModalSyntax] | None, ke: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_sumti: RecoveredField[SumtiSyntax], kehe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def connective(self) -> RecoveredField[ArgumentConnectiveSyntax]:
        'The `argument_connective` connective joining the adjacent constituents of the `grouped_sumti_tail` production.'
        return cast(RecoveredField[ArgumentConnectiveSyntax], self._field(0))
    @property
    def tense_modal(self) -> RecoveredField[TenseModalSyntax] | None:
        'The optional tense modal component.'
        return cast(RecoveredField[TenseModalSyntax] | None, self._field(1))
    @property
    def ke(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Ke` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(2))
    @property
    def inner_sumti(self) -> RecoveredField[SumtiSyntax]:
        'The shared inner sumti child syntax node.'
        return cast(RecoveredField[SumtiSyntax], self._field(3))
    @property
    def kehe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Kehe` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(4))
    def __init_subclass__(cls) -> None:
        raise TypeError('GroupedSumtiTailSyntax is final')

@final
class VuhoSumtiAttachmentTailSyntaxVuhoRelativeSumtiAttachmentTail(_SyntaxNode):
    'Uses the `vuho_relative_sumti_attachment_tail` product form, whose payload preserves `vuho`, `relative_clauses`, and `sumti_connection`.'
    __slots__ = ()
    _schema_id = 215
    __match_args__ = ('vuho_relative_sumti_attachment_tail',)
    def __new__(cls, vuho_relative_sumti_attachment_tail: RecoveredField[VuhoRelativeSumtiAttachmentTailSyntax]) -> VuhoSumtiAttachmentTailSyntaxVuhoRelativeSumtiAttachmentTail:
        return cls._from_fields((vuho_relative_sumti_attachment_tail,))
    def __init__(self, vuho_relative_sumti_attachment_tail: RecoveredField[VuhoRelativeSumtiAttachmentTailSyntax]) -> None:
        pass
    @property
    def vuho_relative_sumti_attachment_tail(self) -> RecoveredField[VuhoRelativeSumtiAttachmentTailSyntax]:
        'Uses the `vuho_relative_sumti_attachment_tail` product form, whose payload preserves `vuho`, `relative_clauses`, and `sumti_connection`.'
        return cast(RecoveredField[VuhoRelativeSumtiAttachmentTailSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('VuhoSumtiAttachmentTailSyntaxVuhoRelativeSumtiAttachmentTail is final')

@final
class VuhoSumtiAttachmentTailSyntaxVuhoConnectedSumtiAttachmentTail(_SyntaxNode):
    'Uses the `vuho_connected_sumti_attachment_tail` product form, whose payload preserves `vuho` and `sumti_connection`.'
    __slots__ = ()
    _schema_id = 216
    __match_args__ = ('vuho_connected_sumti_attachment_tail',)
    def __new__(cls, vuho_connected_sumti_attachment_tail: RecoveredField[VuhoConnectedSumtiAttachmentTailSyntax]) -> VuhoSumtiAttachmentTailSyntaxVuhoConnectedSumtiAttachmentTail:
        return cls._from_fields((vuho_connected_sumti_attachment_tail,))
    def __init__(self, vuho_connected_sumti_attachment_tail: RecoveredField[VuhoConnectedSumtiAttachmentTailSyntax]) -> None:
        pass
    @property
    def vuho_connected_sumti_attachment_tail(self) -> RecoveredField[VuhoConnectedSumtiAttachmentTailSyntax]:
        'Uses the `vuho_connected_sumti_attachment_tail` product form, whose payload preserves `vuho` and `sumti_connection`.'
        return cast(RecoveredField[VuhoConnectedSumtiAttachmentTailSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('VuhoSumtiAttachmentTailSyntaxVuhoConnectedSumtiAttachmentTail is final')

VuhoSumtiAttachmentTailSyntax: TypeAlias = VuhoSumtiAttachmentTailSyntaxVuhoRelativeSumtiAttachmentTail | VuhoSumtiAttachmentTailSyntaxVuhoConnectedSumtiAttachmentTail

@final
class VuhoRelativeSumtiAttachmentTailSyntax(_SyntaxNode):
    'Product node for sumti relative phrase; preserves `vuho`, `relative_clauses`, and `sumti_connection` in source order.'
    __slots__ = ()
    _schema_id = 217
    __match_args__ = ('vuho', 'relative_clauses', 'sumti_connection')
    def __new__(cls, vuho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], relative_clauses: RecoveredField[RelativeClauseListSyntax], sumti_connection: RecoveredField[SumtiConnectionTailSyntax] | None) -> VuhoRelativeSumtiAttachmentTailSyntax:
        return cls._from_fields((vuho, relative_clauses, sumti_connection))
    def __init__(self, vuho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], relative_clauses: RecoveredField[RelativeClauseListSyntax], sumti_connection: RecoveredField[SumtiConnectionTailSyntax] | None) -> None:
        pass
    @property
    def vuho(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Vuho` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def relative_clauses(self) -> RecoveredField[RelativeClauseListSyntax]:
        'The `relative_clause_list` grammar result in the `relative_clauses` structural role of the `vuho_relative_sumti_attachment_tail` production.'
        return cast(RecoveredField[RelativeClauseListSyntax], self._field(1))
    @property
    def sumti_connection(self) -> RecoveredField[SumtiConnectionTailSyntax] | None:
        'The optional sumti connection component.'
        return cast(RecoveredField[SumtiConnectionTailSyntax] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('VuhoRelativeSumtiAttachmentTailSyntax is final')

@final
class VuhoConnectedSumtiAttachmentTailSyntax(_SyntaxNode):
    'Product node for sumti relative phrase; preserves `vuho` and `sumti_connection` in source order.'
    __slots__ = ()
    _schema_id = 218
    __match_args__ = ('vuho', 'sumti_connection')
    def __new__(cls, vuho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], sumti_connection: RecoveredField[SumtiConnectionTailSyntax]) -> VuhoConnectedSumtiAttachmentTailSyntax:
        return cls._from_fields((vuho, sumti_connection))
    def __init__(self, vuho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], sumti_connection: RecoveredField[SumtiConnectionTailSyntax]) -> None:
        pass
    @property
    def vuho(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Vuho` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def sumti_connection(self) -> RecoveredField[SumtiConnectionTailSyntax]:
        'The shared sumti connection child syntax node.'
        return cast(RecoveredField[SumtiConnectionTailSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('VuhoConnectedSumtiAttachmentTailSyntax is final')

@final
class SimpleSumtiSyntax(_SyntaxNode):
    'Product node for sumti; preserves `base_sumti` and `relative_clauses` in source order.'
    __slots__ = ()
    _schema_id = 219
    __match_args__ = ('base_sumti', 'relative_clauses')
    def __new__(cls, base_sumti: RecoveredField[SumtiAtomSyntax], relative_clauses: RecoveredField[RelativeClauseListSyntax] | None) -> SimpleSumtiSyntax:
        return cls._from_fields((base_sumti, relative_clauses))
    def __init__(self, base_sumti: RecoveredField[SumtiAtomSyntax], relative_clauses: RecoveredField[RelativeClauseListSyntax] | None) -> None:
        pass
    @property
    def base_sumti(self) -> RecoveredField[SumtiAtomSyntax]:
        'The shared base sumti child syntax node.'
        return cast(RecoveredField[SumtiAtomSyntax], self._field(0))
    @property
    def relative_clauses(self) -> RecoveredField[RelativeClauseListSyntax] | None:
        'The optional relative clauses component.'
        return cast(RecoveredField[RelativeClauseListSyntax] | None, self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleSumtiSyntax is final')

@final
class SumtiAtomSyntaxSumtiBase(_SyntaxNode):
    'Uses the nested `sumti_base` sum form and preserves its selected alternative.'
    __slots__ = ()
    _schema_id = 220
    __match_args__ = ('sumti_base',)
    def __new__(cls, sumti_base: RecoveredField[SumtiBaseSyntax]) -> SumtiAtomSyntaxSumtiBase:
        return cls._from_fields((sumti_base,))
    def __init__(self, sumti_base: RecoveredField[SumtiBaseSyntax]) -> None:
        pass
    @property
    def sumti_base(self) -> RecoveredField[SumtiBaseSyntax]:
        'Uses the nested `sumti_base` sum form and preserves its selected alternative.'
        return cast(RecoveredField[SumtiBaseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SumtiAtomSyntaxSumtiBase is final')

@final
class SumtiAtomSyntaxQuantifiedSumti(_SyntaxNode):
    'Uses the `quantified_sumti` product form, whose payload preserves `quantifier` and `inner_sumti`.'
    __slots__ = ()
    _schema_id = 221
    __match_args__ = ('quantified_sumti',)
    def __new__(cls, quantified_sumti: RecoveredField[QuantifiedSumtiSyntax]) -> SumtiAtomSyntaxQuantifiedSumti:
        return cls._from_fields((quantified_sumti,))
    def __init__(self, quantified_sumti: RecoveredField[QuantifiedSumtiSyntax]) -> None:
        pass
    @property
    def quantified_sumti(self) -> RecoveredField[QuantifiedSumtiSyntax]:
        'Uses the `quantified_sumti` product form, whose payload preserves `quantifier` and `inner_sumti`.'
        return cast(RecoveredField[QuantifiedSumtiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SumtiAtomSyntaxQuantifiedSumti is final')

SumtiAtomSyntax: TypeAlias = SumtiAtomSyntaxSumtiBase | SumtiAtomSyntaxQuantifiedSumti

@final
class SumtiBaseSyntaxScalarNegatedSumtiWithBo(_SyntaxNode):
    'Uses the `scalar_negated_sumti_with_bo` product form, whose payload preserves `nahe`, `bo`, `inner_sumti`, and `luhu`.'
    __slots__ = ()
    _schema_id = 222
    __match_args__ = ('scalar_negated_sumti_with_bo',)
    def __new__(cls, scalar_negated_sumti_with_bo: RecoveredField[ScalarNegatedSumtiWithBoSyntax]) -> SumtiBaseSyntaxScalarNegatedSumtiWithBo:
        return cls._from_fields((scalar_negated_sumti_with_bo,))
    def __init__(self, scalar_negated_sumti_with_bo: RecoveredField[ScalarNegatedSumtiWithBoSyntax]) -> None:
        pass
    @property
    def scalar_negated_sumti_with_bo(self) -> RecoveredField[ScalarNegatedSumtiWithBoSyntax]:
        'Uses the `scalar_negated_sumti_with_bo` product form, whose payload preserves `nahe`, `bo`, `inner_sumti`, and `luhu`.'
        return cast(RecoveredField[ScalarNegatedSumtiWithBoSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SumtiBaseSyntaxScalarNegatedSumtiWithBo is final')

@final
class SumtiBaseSyntaxScalarNegatedSumti(_SyntaxNode):
    'Uses the `scalar_negated_sumti` product form, whose payload preserves `nahe`, `inner_sumti`, and `luhu`.'
    __slots__ = ()
    _schema_id = 223
    __match_args__ = ('scalar_negated_sumti',)
    def __new__(cls, scalar_negated_sumti: RecoveredField[ScalarNegatedSumtiSyntax]) -> SumtiBaseSyntaxScalarNegatedSumti:
        return cls._from_fields((scalar_negated_sumti,))
    def __init__(self, scalar_negated_sumti: RecoveredField[ScalarNegatedSumtiSyntax]) -> None:
        pass
    @property
    def scalar_negated_sumti(self) -> RecoveredField[ScalarNegatedSumtiSyntax]:
        'Uses the `scalar_negated_sumti` product form, whose payload preserves `nahe`, `inner_sumti`, and `luhu`.'
        return cast(RecoveredField[ScalarNegatedSumtiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SumtiBaseSyntaxScalarNegatedSumti is final')

@final
class SumtiBaseSyntaxLaheSumti(_SyntaxNode):
    'Uses the `lahe_sumti` product form, whose payload preserves `lahe`, `relative_clauses`, `inner_sumti`, and `luhu`.'
    __slots__ = ()
    _schema_id = 224
    __match_args__ = ('lahe_sumti',)
    def __new__(cls, lahe_sumti: RecoveredField[LaheSumtiSyntax]) -> SumtiBaseSyntaxLaheSumti:
        return cls._from_fields((lahe_sumti,))
    def __init__(self, lahe_sumti: RecoveredField[LaheSumtiSyntax]) -> None:
        pass
    @property
    def lahe_sumti(self) -> RecoveredField[LaheSumtiSyntax]:
        'Uses the `lahe_sumti` product form, whose payload preserves `lahe`, `relative_clauses`, `inner_sumti`, and `luhu`.'
        return cast(RecoveredField[LaheSumtiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SumtiBaseSyntaxLaheSumti is final')

@final
class SumtiBaseSyntaxLaheTermWrapper(_SyntaxNode):
    'Uses the `lahe_term_wrapper` product form, whose payload preserves `lahe`, `inner_term`, and `luhu`.'
    __slots__ = ()
    _schema_id = 225
    __match_args__ = ('lahe_term_wrapper',)
    def __new__(cls, lahe_term_wrapper: RecoveredField[LaheTermWrapperSyntax]) -> SumtiBaseSyntaxLaheTermWrapper:
        return cls._from_fields((lahe_term_wrapper,))
    def __init__(self, lahe_term_wrapper: RecoveredField[LaheTermWrapperSyntax]) -> None:
        pass
    @property
    def lahe_term_wrapper(self) -> RecoveredField[LaheTermWrapperSyntax]:
        'Uses the `lahe_term_wrapper` product form, whose payload preserves `lahe`, `inner_term`, and `luhu`.'
        return cast(RecoveredField[LaheTermWrapperSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SumtiBaseSyntaxLaheTermWrapper is final')

@final
class SumtiBaseSyntaxScalarNegatedTermWrapperWithBo(_SyntaxNode):
    'Uses the `scalar_negated_term_wrapper_with_bo` product form, whose payload preserves `nahe`, `bo`, `inner_term`, and `luhu`.'
    __slots__ = ()
    _schema_id = 226
    __match_args__ = ('scalar_negated_term_wrapper_with_bo',)
    def __new__(cls, scalar_negated_term_wrapper_with_bo: RecoveredField[ScalarNegatedTermWrapperWithBoSyntax]) -> SumtiBaseSyntaxScalarNegatedTermWrapperWithBo:
        return cls._from_fields((scalar_negated_term_wrapper_with_bo,))
    def __init__(self, scalar_negated_term_wrapper_with_bo: RecoveredField[ScalarNegatedTermWrapperWithBoSyntax]) -> None:
        pass
    @property
    def scalar_negated_term_wrapper_with_bo(self) -> RecoveredField[ScalarNegatedTermWrapperWithBoSyntax]:
        'Uses the `scalar_negated_term_wrapper_with_bo` product form, whose payload preserves `nahe`, `bo`, `inner_term`, and `luhu`.'
        return cast(RecoveredField[ScalarNegatedTermWrapperWithBoSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SumtiBaseSyntaxScalarNegatedTermWrapperWithBo is final')

@final
class SumtiBaseSyntaxScalarNegatedTermWrapper(_SyntaxNode):
    'Uses the `scalar_negated_term_wrapper` product form, whose payload preserves `nahe`, `inner_term`, and `luhu`.'
    __slots__ = ()
    _schema_id = 227
    __match_args__ = ('scalar_negated_term_wrapper',)
    def __new__(cls, scalar_negated_term_wrapper: RecoveredField[ScalarNegatedTermWrapperSyntax]) -> SumtiBaseSyntaxScalarNegatedTermWrapper:
        return cls._from_fields((scalar_negated_term_wrapper,))
    def __init__(self, scalar_negated_term_wrapper: RecoveredField[ScalarNegatedTermWrapperSyntax]) -> None:
        pass
    @property
    def scalar_negated_term_wrapper(self) -> RecoveredField[ScalarNegatedTermWrapperSyntax]:
        'Uses the `scalar_negated_term_wrapper` product form, whose payload preserves `nahe`, `inner_term`, and `luhu`.'
        return cast(RecoveredField[ScalarNegatedTermWrapperSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SumtiBaseSyntaxScalarNegatedTermWrapper is final')

@final
class SumtiBaseSyntaxBridiDescriptionSumti(_SyntaxNode):
    'Uses the `bridi_description_sumti` product form, whose payload preserves `lohoi`, `additional_heads`, `statement`, and `kuhau`.'
    __slots__ = ()
    _schema_id = 228
    __match_args__ = ('bridi_description_sumti',)
    def __new__(cls, bridi_description_sumti: RecoveredField[BridiDescriptionSumtiSyntax]) -> SumtiBaseSyntaxBridiDescriptionSumti:
        return cls._from_fields((bridi_description_sumti,))
    def __init__(self, bridi_description_sumti: RecoveredField[BridiDescriptionSumtiSyntax]) -> None:
        pass
    @property
    def bridi_description_sumti(self) -> RecoveredField[BridiDescriptionSumtiSyntax]:
        'Uses the `bridi_description_sumti` product form, whose payload preserves `lohoi`, `additional_heads`, `statement`, and `kuhau`.'
        return cast(RecoveredField[BridiDescriptionSumtiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SumtiBaseSyntaxBridiDescriptionSumti is final')

@final
class SumtiBaseSyntaxNameSumti(_SyntaxNode):
    'Uses the `name_sumti` product form, whose payload preserves `la`, `relative_clauses`, and `names`.'
    __slots__ = ()
    _schema_id = 229
    __match_args__ = ('name_sumti',)
    def __new__(cls, name_sumti: RecoveredField[NameSumtiSyntax]) -> SumtiBaseSyntaxNameSumti:
        return cls._from_fields((name_sumti,))
    def __init__(self, name_sumti: RecoveredField[NameSumtiSyntax]) -> None:
        pass
    @property
    def name_sumti(self) -> RecoveredField[NameSumtiSyntax]:
        'Uses the `name_sumti` product form, whose payload preserves `la`, `relative_clauses`, and `names`.'
        return cast(RecoveredField[NameSumtiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SumtiBaseSyntaxNameSumti is final')

@final
class SumtiBaseSyntaxDescriptionConnectionSumti(_SyntaxNode):
    'Uses the `description_connection_sumti` product form, whose payload preserves `leading_description_head`, `connective`, `trailing_description_head`, `tail`, and `ku`.'
    __slots__ = ()
    _schema_id = 230
    __match_args__ = ('description_connection_sumti',)
    def __new__(cls, description_connection_sumti: RecoveredField[DescriptionConnectionSumtiSyntax]) -> SumtiBaseSyntaxDescriptionConnectionSumti:
        return cls._from_fields((description_connection_sumti,))
    def __init__(self, description_connection_sumti: RecoveredField[DescriptionConnectionSumtiSyntax]) -> None:
        pass
    @property
    def description_connection_sumti(self) -> RecoveredField[DescriptionConnectionSumtiSyntax]:
        'Uses the `description_connection_sumti` product form, whose payload preserves `leading_description_head`, `connective`, `trailing_description_head`, `tail`, and `ku`.'
        return cast(RecoveredField[DescriptionConnectionSumtiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SumtiBaseSyntaxDescriptionConnectionSumti is final')

@final
class SumtiBaseSyntaxDescriptorWithOuterQuantifierSumti(_SyntaxNode):
    'Uses the `descriptor_with_outer_quantifier_sumti` product form, whose payload preserves `outer_quantifier`, `description`, `tail`, and `ku`.'
    __slots__ = ()
    _schema_id = 231
    __match_args__ = ('descriptor_with_outer_quantifier_sumti',)
    def __new__(cls, descriptor_with_outer_quantifier_sumti: RecoveredField[DescriptorWithOuterQuantifierSumtiSyntax]) -> SumtiBaseSyntaxDescriptorWithOuterQuantifierSumti:
        return cls._from_fields((descriptor_with_outer_quantifier_sumti,))
    def __init__(self, descriptor_with_outer_quantifier_sumti: RecoveredField[DescriptorWithOuterQuantifierSumtiSyntax]) -> None:
        pass
    @property
    def descriptor_with_outer_quantifier_sumti(self) -> RecoveredField[DescriptorWithOuterQuantifierSumtiSyntax]:
        'Uses the `descriptor_with_outer_quantifier_sumti` product form, whose payload preserves `outer_quantifier`, `description`, `tail`, and `ku`.'
        return cast(RecoveredField[DescriptorWithOuterQuantifierSumtiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SumtiBaseSyntaxDescriptorWithOuterQuantifierSumti is final')

@final
class SumtiBaseSyntaxDescriptorWithGadriSumti(_SyntaxNode):
    'Uses the `descriptor_with_gadri_sumti` product form, whose payload preserves `description`, `tail`, and `ku`.'
    __slots__ = ()
    _schema_id = 232
    __match_args__ = ('descriptor_with_gadri_sumti',)
    def __new__(cls, descriptor_with_gadri_sumti: RecoveredField[DescriptorWithGadriSumtiSyntax]) -> SumtiBaseSyntaxDescriptorWithGadriSumti:
        return cls._from_fields((descriptor_with_gadri_sumti,))
    def __init__(self, descriptor_with_gadri_sumti: RecoveredField[DescriptorWithGadriSumtiSyntax]) -> None:
        pass
    @property
    def descriptor_with_gadri_sumti(self) -> RecoveredField[DescriptorWithGadriSumtiSyntax]:
        'Uses the `descriptor_with_gadri_sumti` product form, whose payload preserves `description`, `tail`, and `ku`.'
        return cast(RecoveredField[DescriptorWithGadriSumtiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SumtiBaseSyntaxDescriptorWithGadriSumti is final')

@final
class SumtiBaseSyntaxDescriptorWithoutGadriSumti(_SyntaxNode):
    'Uses the `descriptor_without_gadri_sumti` product form, whose payload preserves `quantifier`, `selbri`, `ku`, and `relative_clauses`.'
    __slots__ = ()
    _schema_id = 233
    __match_args__ = ('descriptor_without_gadri_sumti',)
    def __new__(cls, descriptor_without_gadri_sumti: RecoveredField[DescriptorWithoutGadriSumtiSyntax]) -> SumtiBaseSyntaxDescriptorWithoutGadriSumti:
        return cls._from_fields((descriptor_without_gadri_sumti,))
    def __init__(self, descriptor_without_gadri_sumti: RecoveredField[DescriptorWithoutGadriSumtiSyntax]) -> None:
        pass
    @property
    def descriptor_without_gadri_sumti(self) -> RecoveredField[DescriptorWithoutGadriSumtiSyntax]:
        'Uses the `descriptor_without_gadri_sumti` product form, whose payload preserves `quantifier`, `selbri`, `ku`, and `relative_clauses`.'
        return cast(RecoveredField[DescriptorWithoutGadriSumtiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SumtiBaseSyntaxDescriptorWithoutGadriSumti is final')

@final
class SumtiBaseSyntaxNumberSumti(_SyntaxNode):
    'Uses the `number_sumti` product form, whose payload preserves `li`, `expression`, and `loho`.'
    __slots__ = ()
    _schema_id = 234
    __match_args__ = ('number_sumti',)
    def __new__(cls, number_sumti: RecoveredField[NumberSumtiSyntax]) -> SumtiBaseSyntaxNumberSumti:
        return cls._from_fields((number_sumti,))
    def __init__(self, number_sumti: RecoveredField[NumberSumtiSyntax]) -> None:
        pass
    @property
    def number_sumti(self) -> RecoveredField[NumberSumtiSyntax]:
        'Uses the `number_sumti` product form, whose payload preserves `li`, `expression`, and `loho`.'
        return cast(RecoveredField[NumberSumtiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SumtiBaseSyntaxNumberSumti is final')

@final
class SumtiBaseSyntaxLerfuStringSumti(_SyntaxNode):
    'Uses the `lerfu_string_sumti` product form, whose payload preserves `words`, `boi`, and `free_modifiers`.'
    __slots__ = ()
    _schema_id = 235
    __match_args__ = ('lerfu_string_sumti',)
    def __new__(cls, lerfu_string_sumti: RecoveredField[LerfuStringSumtiSyntax]) -> SumtiBaseSyntaxLerfuStringSumti:
        return cls._from_fields((lerfu_string_sumti,))
    def __init__(self, lerfu_string_sumti: RecoveredField[LerfuStringSumtiSyntax]) -> None:
        pass
    @property
    def lerfu_string_sumti(self) -> RecoveredField[LerfuStringSumtiSyntax]:
        'Uses the `lerfu_string_sumti` product form, whose payload preserves `words`, `boi`, and `free_modifiers`.'
        return cast(RecoveredField[LerfuStringSumtiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SumtiBaseSyntaxLerfuStringSumti is final')

@final
class SumtiBaseSyntaxQuotedSumti(_SyntaxNode):
    'Uses the `quoted_sumti` product form, whose payload preserves `quote`.'
    __slots__ = ()
    _schema_id = 236
    __match_args__ = ('quoted_sumti',)
    def __new__(cls, quoted_sumti: RecoveredField[QuotedSumtiSyntax]) -> SumtiBaseSyntaxQuotedSumti:
        return cls._from_fields((quoted_sumti,))
    def __init__(self, quoted_sumti: RecoveredField[QuotedSumtiSyntax]) -> None:
        pass
    @property
    def quoted_sumti(self) -> RecoveredField[QuotedSumtiSyntax]:
        'Uses the `quoted_sumti` product form, whose payload preserves `quote`.'
        return cast(RecoveredField[QuotedSumtiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SumtiBaseSyntaxQuotedSumti is final')

@final
class SumtiBaseSyntaxProSumti(_SyntaxNode):
    'Uses the `pro_sumti` product form, whose payload preserves `koha`.'
    __slots__ = ()
    _schema_id = 237
    __match_args__ = ('pro_sumti',)
    def __new__(cls, pro_sumti: RecoveredField[ProSumtiSyntax]) -> SumtiBaseSyntaxProSumti:
        return cls._from_fields((pro_sumti,))
    def __init__(self, pro_sumti: RecoveredField[ProSumtiSyntax]) -> None:
        pass
    @property
    def pro_sumti(self) -> RecoveredField[ProSumtiSyntax]:
        'Uses the `pro_sumti` product form, whose payload preserves `koha`.'
        return cast(RecoveredField[ProSumtiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SumtiBaseSyntaxProSumti is final')

SumtiBaseSyntax: TypeAlias = SumtiBaseSyntaxScalarNegatedSumtiWithBo | SumtiBaseSyntaxScalarNegatedSumti | SumtiBaseSyntaxLaheSumti | SumtiBaseSyntaxLaheTermWrapper | SumtiBaseSyntaxScalarNegatedTermWrapperWithBo | SumtiBaseSyntaxScalarNegatedTermWrapper | SumtiBaseSyntaxBridiDescriptionSumti | SumtiBaseSyntaxNameSumti | SumtiBaseSyntaxDescriptionConnectionSumti | SumtiBaseSyntaxDescriptorWithOuterQuantifierSumti | SumtiBaseSyntaxDescriptorWithGadriSumti | SumtiBaseSyntaxDescriptorWithoutGadriSumti | SumtiBaseSyntaxNumberSumti | SumtiBaseSyntaxLerfuStringSumti | SumtiBaseSyntaxQuotedSumti | SumtiBaseSyntaxProSumti

@final
class QuantifiedSumtiSyntax(_SyntaxNode):
    'Product node for quantified sumti; preserves `quantifier` and `inner_sumti` in source order.'
    __slots__ = ()
    _schema_id = 238
    __match_args__ = ('quantifier', 'inner_sumti')
    def __new__(cls, quantifier: RecoveredField[QuantifierSyntax], inner_sumti: RecoveredField[SumtiBaseSyntax]) -> QuantifiedSumtiSyntax:
        return cls._from_fields((quantifier, inner_sumti))
    def __init__(self, quantifier: RecoveredField[QuantifierSyntax], inner_sumti: RecoveredField[SumtiBaseSyntax]) -> None:
        pass
    @property
    def quantifier(self) -> RecoveredField[QuantifierSyntax]:
        'The `quantifier` grammar result in the `quantifier` structural role of the `quantified_sumti` production.'
        return cast(RecoveredField[QuantifierSyntax], self._field(0))
    @property
    def inner_sumti(self) -> RecoveredField[SumtiBaseSyntax]:
        'The shared inner sumti child syntax node.'
        return cast(RecoveredField[SumtiBaseSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('QuantifiedSumtiSyntax is final')

@final
class SumtiConnectionTailSyntax(_SyntaxNode):
    'Product node for sumti connective; preserves `connective` and `sumti` in source order.'
    __slots__ = ()
    _schema_id = 239
    __match_args__ = ('connective', 'sumti')
    def __new__(cls, connective: RecoveredField[ArgumentConnectiveSyntax], sumti: RecoveredField[SumtiSyntax]) -> SumtiConnectionTailSyntax:
        return cls._from_fields((connective, sumti))
    def __init__(self, connective: RecoveredField[ArgumentConnectiveSyntax], sumti: RecoveredField[SumtiSyntax]) -> None:
        pass
    @property
    def connective(self) -> RecoveredField[ArgumentConnectiveSyntax]:
        'The `argument_connective` connective joining the adjacent constituents of the `sumti_connection_tail` production.'
        return cast(RecoveredField[ArgumentConnectiveSyntax], self._field(0))
    @property
    def sumti(self) -> RecoveredField[SumtiSyntax]:
        'The shared sumti child syntax node.'
        return cast(RecoveredField[SumtiSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('SumtiConnectionTailSyntax is final')

@final
class PaRunQuantifierSyntax(_SyntaxNode):
    'Product node for quantifier; preserves `number` and `boi` in source order.'
    __slots__ = ()
    _schema_id = 240
    __match_args__ = ('number', 'boi')
    def __new__(cls, number: WithFreeModifiers[RecoveredField[NumberWordsSyntax], RecoveredField[FreeModifierSyntax]], boi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> PaRunQuantifierSyntax:
        return cls._from_fields((number, boi))
    def __init__(self, number: WithFreeModifiers[RecoveredField[NumberWordsSyntax], RecoveredField[FreeModifierSyntax]], boi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def number(self) -> WithFreeModifiers[RecoveredField[NumberWordsSyntax], RecoveredField[FreeModifierSyntax]]:
        'The `number_words` grammar result in the `number` structural role of the `pa_run_quantifier` production.'
        return cast(WithFreeModifiers[RecoveredField[NumberWordsSyntax], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def boi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Boi` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('PaRunQuantifierSyntax is final')

@final
class MeksoQuantifierSyntax(_SyntaxNode):
    'Product node for quantifier; preserves `vei`, `mekso`, and `veho` in source order.'
    __slots__ = ()
    _schema_id = 241
    __match_args__ = ('vei', 'mekso', 'veho')
    def __new__(cls, vei: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], mekso: RecoveredField[MeksoSyntax], veho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> MeksoQuantifierSyntax:
        return cls._from_fields((vei, mekso, veho))
    def __init__(self, vei: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], mekso: RecoveredField[MeksoSyntax], veho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def vei(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Vei` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def mekso(self) -> RecoveredField[MeksoSyntax]:
        'The shared mekso child syntax node.'
        return cast(RecoveredField[MeksoSyntax], self._field(1))
    @property
    def veho(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Veho` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('MeksoQuantifierSyntax is final')

@final
class ZantufaRawMeksoQuantifierSyntax(_SyntaxNode):
    'Transparent product node for quantifier; preserves the `mekso` component.'
    __slots__ = ()
    _schema_id = 242
    __match_args__ = ('mekso',)
    def __new__(cls, mekso: RecoveredField[MeksoSyntax]) -> ZantufaRawMeksoQuantifierSyntax:
        return cls._from_fields((mekso,))
    def __init__(self, mekso: RecoveredField[MeksoSyntax]) -> None:
        pass
    @property
    def mekso(self) -> RecoveredField[MeksoSyntax]:
        'The shared mekso child syntax node.'
        return cast(RecoveredField[MeksoSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaRawMeksoQuantifierSyntax is final')

@final
class ZantufaPriorityRawMeksoQuantifierSyntax(_SyntaxNode):
    'Transparent product node for quantifier; preserves the `mekso` component.'
    __slots__ = ()
    _schema_id = 243
    __match_args__ = ('mekso',)
    def __new__(cls, mekso: RecoveredField[MeksoSyntax]) -> ZantufaPriorityRawMeksoQuantifierSyntax:
        return cls._from_fields((mekso,))
    def __init__(self, mekso: RecoveredField[MeksoSyntax]) -> None:
        pass
    @property
    def mekso(self) -> RecoveredField[MeksoSyntax]:
        'The shared mekso child syntax node.'
        return cast(RecoveredField[MeksoSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaPriorityRawMeksoQuantifierSyntax is final')

@final
class QuantifierSyntaxZantufaPriorityRawMeksoQuantifier(_SyntaxNode):
    'Uses the `zantufa_priority_raw_mekso_quantifier` product form, whose payload preserves `mekso`.'
    __slots__ = ()
    _schema_id = 244
    __match_args__ = ('zantufa_priority_raw_mekso_quantifier',)
    def __new__(cls, zantufa_priority_raw_mekso_quantifier: RecoveredField[ZantufaPriorityRawMeksoQuantifierSyntax]) -> QuantifierSyntaxZantufaPriorityRawMeksoQuantifier:
        return cls._from_fields((zantufa_priority_raw_mekso_quantifier,))
    def __init__(self, zantufa_priority_raw_mekso_quantifier: RecoveredField[ZantufaPriorityRawMeksoQuantifierSyntax]) -> None:
        pass
    @property
    def zantufa_priority_raw_mekso_quantifier(self) -> RecoveredField[ZantufaPriorityRawMeksoQuantifierSyntax]:
        'Uses the `zantufa_priority_raw_mekso_quantifier` product form, whose payload preserves `mekso`.'
        return cast(RecoveredField[ZantufaPriorityRawMeksoQuantifierSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('QuantifierSyntaxZantufaPriorityRawMeksoQuantifier is final')

@final
class QuantifierSyntaxMeksoQuantifier(_SyntaxNode):
    'Uses the `mekso_quantifier` product form, whose payload preserves `vei`, `mekso`, and `veho`.'
    __slots__ = ()
    _schema_id = 245
    __match_args__ = ('mekso_quantifier',)
    def __new__(cls, mekso_quantifier: RecoveredField[MeksoQuantifierSyntax]) -> QuantifierSyntaxMeksoQuantifier:
        return cls._from_fields((mekso_quantifier,))
    def __init__(self, mekso_quantifier: RecoveredField[MeksoQuantifierSyntax]) -> None:
        pass
    @property
    def mekso_quantifier(self) -> RecoveredField[MeksoQuantifierSyntax]:
        'Uses the `mekso_quantifier` product form, whose payload preserves `vei`, `mekso`, and `veho`.'
        return cast(RecoveredField[MeksoQuantifierSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('QuantifierSyntaxMeksoQuantifier is final')

@final
class QuantifierSyntaxPaRunQuantifier(_SyntaxNode):
    'Uses the `pa_run_quantifier` product form, whose payload preserves `number` and `boi`.'
    __slots__ = ()
    _schema_id = 246
    __match_args__ = ('pa_run_quantifier',)
    def __new__(cls, pa_run_quantifier: RecoveredField[PaRunQuantifierSyntax]) -> QuantifierSyntaxPaRunQuantifier:
        return cls._from_fields((pa_run_quantifier,))
    def __init__(self, pa_run_quantifier: RecoveredField[PaRunQuantifierSyntax]) -> None:
        pass
    @property
    def pa_run_quantifier(self) -> RecoveredField[PaRunQuantifierSyntax]:
        'Uses the `pa_run_quantifier` product form, whose payload preserves `number` and `boi`.'
        return cast(RecoveredField[PaRunQuantifierSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('QuantifierSyntaxPaRunQuantifier is final')

@final
class QuantifierSyntaxZantufaRawMeksoQuantifier(_SyntaxNode):
    'Uses the `zantufa_raw_mekso_quantifier` product form, whose payload preserves `mekso`.'
    __slots__ = ()
    _schema_id = 247
    __match_args__ = ('zantufa_raw_mekso_quantifier',)
    def __new__(cls, zantufa_raw_mekso_quantifier: RecoveredField[ZantufaRawMeksoQuantifierSyntax]) -> QuantifierSyntaxZantufaRawMeksoQuantifier:
        return cls._from_fields((zantufa_raw_mekso_quantifier,))
    def __init__(self, zantufa_raw_mekso_quantifier: RecoveredField[ZantufaRawMeksoQuantifierSyntax]) -> None:
        pass
    @property
    def zantufa_raw_mekso_quantifier(self) -> RecoveredField[ZantufaRawMeksoQuantifierSyntax]:
        'Uses the `zantufa_raw_mekso_quantifier` product form, whose payload preserves `mekso`.'
        return cast(RecoveredField[ZantufaRawMeksoQuantifierSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('QuantifierSyntaxZantufaRawMeksoQuantifier is final')

QuantifierSyntax: TypeAlias = QuantifierSyntaxZantufaPriorityRawMeksoQuantifier | QuantifierSyntaxMeksoQuantifier | QuantifierSyntaxPaRunQuantifier | QuantifierSyntaxZantufaRawMeksoQuantifier

@final
class NumberMeksoSyntax(_SyntaxNode):
    'Transparent product node for number mex; preserves the `quantifier` component.'
    __slots__ = ()
    _schema_id = 248
    __match_args__ = ('quantifier',)
    def __new__(cls, quantifier: RecoveredField[PaRunQuantifierSyntax]) -> NumberMeksoSyntax:
        return cls._from_fields((quantifier,))
    def __init__(self, quantifier: RecoveredField[PaRunQuantifierSyntax]) -> None:
        pass
    @property
    def quantifier(self) -> RecoveredField[PaRunQuantifierSyntax]:
        'The shared quantifier child syntax node.'
        return cast(RecoveredField[PaRunQuantifierSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('NumberMeksoSyntax is final')

@final
class PrimitiveMeksoOperatorSyntax(_SyntaxNode):
    'Transparent product node for VUhU operator; preserves the `vuhu` component.'
    __slots__ = ()
    _schema_id = 249
    __match_args__ = ('vuhu',)
    def __new__(cls, vuhu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> PrimitiveMeksoOperatorSyntax:
        return cls._from_fields((vuhu,))
    def __init__(self, vuhu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def vuhu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Vuhu`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('PrimitiveMeksoOperatorSyntax is final')

@final
class MeksoOperatorSyntaxAfterthoughtMeksoOperator(_SyntaxNode):
    'Uses the `afterthought_mekso_operator` product form, whose payload preserves `operators`.'
    __slots__ = ()
    _schema_id = 250
    __match_args__ = ('afterthought_mekso_operator',)
    def __new__(cls, afterthought_mekso_operator: RecoveredField[AfterthoughtMeksoOperatorSyntax]) -> MeksoOperatorSyntaxAfterthoughtMeksoOperator:
        return cls._from_fields((afterthought_mekso_operator,))
    def __init__(self, afterthought_mekso_operator: RecoveredField[AfterthoughtMeksoOperatorSyntax]) -> None:
        pass
    @property
    def afterthought_mekso_operator(self) -> RecoveredField[AfterthoughtMeksoOperatorSyntax]:
        'Uses the `afterthought_mekso_operator` product form, whose payload preserves `operators`.'
        return cast(RecoveredField[AfterthoughtMeksoOperatorSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('MeksoOperatorSyntaxAfterthoughtMeksoOperator is final')

@final
class MeksoOperatorSyntaxBoundMeksoOperator(_SyntaxNode):
    'Uses the `bound_mekso_operator` product form, whose payload preserves `left_operator`, `connective`, `bo`, and `right_operator`.'
    __slots__ = ()
    _schema_id = 251
    __match_args__ = ('bound_mekso_operator',)
    def __new__(cls, bound_mekso_operator: RecoveredField[BoundMeksoOperatorSyntax]) -> MeksoOperatorSyntaxBoundMeksoOperator:
        return cls._from_fields((bound_mekso_operator,))
    def __init__(self, bound_mekso_operator: RecoveredField[BoundMeksoOperatorSyntax]) -> None:
        pass
    @property
    def bound_mekso_operator(self) -> RecoveredField[BoundMeksoOperatorSyntax]:
        'Uses the `bound_mekso_operator` product form, whose payload preserves `left_operator`, `connective`, `bo`, and `right_operator`.'
        return cast(RecoveredField[BoundMeksoOperatorSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('MeksoOperatorSyntaxBoundMeksoOperator is final')

@final
class MeksoOperatorSyntaxSimpleMeksoOperator(_SyntaxNode):
    'Uses the nested `simple_mekso_operator` sum form and preserves its selected alternative.'
    __slots__ = ()
    _schema_id = 252
    __match_args__ = ('simple_mekso_operator',)
    def __new__(cls, simple_mekso_operator: RecoveredField[SimpleMeksoOperatorSyntax]) -> MeksoOperatorSyntaxSimpleMeksoOperator:
        return cls._from_fields((simple_mekso_operator,))
    def __init__(self, simple_mekso_operator: RecoveredField[SimpleMeksoOperatorSyntax]) -> None:
        pass
    @property
    def simple_mekso_operator(self) -> RecoveredField[SimpleMeksoOperatorSyntax]:
        'Uses the nested `simple_mekso_operator` sum form and preserves its selected alternative.'
        return cast(RecoveredField[SimpleMeksoOperatorSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('MeksoOperatorSyntaxSimpleMeksoOperator is final')

MeksoOperatorSyntax: TypeAlias = MeksoOperatorSyntaxAfterthoughtMeksoOperator | MeksoOperatorSyntaxBoundMeksoOperator | MeksoOperatorSyntaxSimpleMeksoOperator

@final
class AfterthoughtMeksoOperatorSyntax(_SyntaxNode):
    'Transparent product node for operator; preserves the `operators` component.'
    __slots__ = ()
    _schema_id = 253
    __match_args__ = ('operators',)
    def __new__(cls, operators: Chain[RecoveredField[BoundOrAtomMeksoOperatorSyntax], RecoveredField[AfterthoughtMeksoOperatorContinuationSyntax]]) -> AfterthoughtMeksoOperatorSyntax:
        return cls._from_fields((operators,))
    def __init__(self, operators: Chain[RecoveredField[BoundOrAtomMeksoOperatorSyntax], RecoveredField[AfterthoughtMeksoOperatorContinuationSyntax]]) -> None:
        pass
    @property
    def operators(self) -> Chain[RecoveredField[BoundOrAtomMeksoOperatorSyntax], RecoveredField[AfterthoughtMeksoOperatorContinuationSyntax]]:
        'The source-ordered `operators` chain assembled by the `afterthought_mekso_operator` production.'
        return cast(Chain[RecoveredField[BoundOrAtomMeksoOperatorSyntax], RecoveredField[AfterthoughtMeksoOperatorContinuationSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('AfterthoughtMeksoOperatorSyntax is final')

@final
class AfterthoughtMeksoOperatorContinuationSyntax(_SyntaxNode):
    'Product node for operator continuation; preserves `connective` and `trailing_operator` in source order.'
    __slots__ = ()
    _schema_id = 254
    __match_args__ = ('connective', 'trailing_operator')
    def __new__(cls, connective: RecoveredField[StandardStatementConnectiveSyntax], trailing_operator: RecoveredField[BoundOrAtomMeksoOperatorSyntax]) -> AfterthoughtMeksoOperatorContinuationSyntax:
        return cls._from_fields((connective, trailing_operator))
    def __init__(self, connective: RecoveredField[StandardStatementConnectiveSyntax], trailing_operator: RecoveredField[BoundOrAtomMeksoOperatorSyntax]) -> None:
        pass
    @property
    def connective(self) -> RecoveredField[StandardStatementConnectiveSyntax]:
        'The `standard_statement_connective` connective joining the adjacent constituents of the `afterthought_mekso_operator_continuation` production.'
        return cast(RecoveredField[StandardStatementConnectiveSyntax], self._field(0))
    @property
    def trailing_operator(self) -> RecoveredField[BoundOrAtomMeksoOperatorSyntax]:
        'The shared trailing operator child syntax node.'
        return cast(RecoveredField[BoundOrAtomMeksoOperatorSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('AfterthoughtMeksoOperatorContinuationSyntax is final')

@final
class BoundOrAtomMeksoOperatorSyntaxBoundMeksoOperator(_SyntaxNode):
    'Uses the `bound_mekso_operator` product form, whose payload preserves `left_operator`, `connective`, `bo`, and `right_operator`.'
    __slots__ = ()
    _schema_id = 255
    __match_args__ = ('bound_mekso_operator',)
    def __new__(cls, bound_mekso_operator: RecoveredField[BoundMeksoOperatorSyntax]) -> BoundOrAtomMeksoOperatorSyntaxBoundMeksoOperator:
        return cls._from_fields((bound_mekso_operator,))
    def __init__(self, bound_mekso_operator: RecoveredField[BoundMeksoOperatorSyntax]) -> None:
        pass
    @property
    def bound_mekso_operator(self) -> RecoveredField[BoundMeksoOperatorSyntax]:
        'Uses the `bound_mekso_operator` product form, whose payload preserves `left_operator`, `connective`, `bo`, and `right_operator`.'
        return cast(RecoveredField[BoundMeksoOperatorSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('BoundOrAtomMeksoOperatorSyntaxBoundMeksoOperator is final')

@final
class BoundOrAtomMeksoOperatorSyntaxSimpleMeksoOperator(_SyntaxNode):
    'Uses the nested `simple_mekso_operator` sum form and preserves its selected alternative.'
    __slots__ = ()
    _schema_id = 256
    __match_args__ = ('simple_mekso_operator',)
    def __new__(cls, simple_mekso_operator: RecoveredField[SimpleMeksoOperatorSyntax]) -> BoundOrAtomMeksoOperatorSyntaxSimpleMeksoOperator:
        return cls._from_fields((simple_mekso_operator,))
    def __init__(self, simple_mekso_operator: RecoveredField[SimpleMeksoOperatorSyntax]) -> None:
        pass
    @property
    def simple_mekso_operator(self) -> RecoveredField[SimpleMeksoOperatorSyntax]:
        'Uses the nested `simple_mekso_operator` sum form and preserves its selected alternative.'
        return cast(RecoveredField[SimpleMeksoOperatorSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('BoundOrAtomMeksoOperatorSyntaxSimpleMeksoOperator is final')

BoundOrAtomMeksoOperatorSyntax: TypeAlias = BoundOrAtomMeksoOperatorSyntaxBoundMeksoOperator | BoundOrAtomMeksoOperatorSyntaxSimpleMeksoOperator

@final
class BoundMeksoOperatorSyntax(_SyntaxNode):
    'Product node for operator; preserves `left_operator`, `connective`, `bo`, and `right_operator` in source order.'
    __slots__ = ()
    _schema_id = 257
    __match_args__ = ('left_operator', 'connective', 'bo', 'right_operator')
    def __new__(cls, left_operator: RecoveredField[SimpleMeksoOperatorSyntax], connective: RecoveredField[StandardStatementConnectiveSyntax], bo: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], right_operator: RecoveredField[MeksoOperatorSyntax]) -> BoundMeksoOperatorSyntax:
        return cls._from_fields((left_operator, connective, bo, right_operator))
    def __init__(self, left_operator: RecoveredField[SimpleMeksoOperatorSyntax], connective: RecoveredField[StandardStatementConnectiveSyntax], bo: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], right_operator: RecoveredField[MeksoOperatorSyntax]) -> None:
        pass
    @property
    def left_operator(self) -> RecoveredField[SimpleMeksoOperatorSyntax]:
        'The shared left operator child syntax node.'
        return cast(RecoveredField[SimpleMeksoOperatorSyntax], self._field(0))
    @property
    def connective(self) -> RecoveredField[StandardStatementConnectiveSyntax]:
        'The `standard_statement_connective` connective joining the adjacent constituents of the `bound_mekso_operator` production.'
        return cast(RecoveredField[StandardStatementConnectiveSyntax], self._field(1))
    @property
    def bo(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Bo` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(2))
    @property
    def right_operator(self) -> RecoveredField[MeksoOperatorSyntax]:
        'The shared right operator child syntax node.'
        return cast(RecoveredField[MeksoOperatorSyntax], self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('BoundMeksoOperatorSyntax is final')

@final
class SimpleMeksoOperatorSyntaxConvertedMeksoOperator(_SyntaxNode):
    'Uses the `converted_mekso_operator` product form, whose payload preserves `se` and `inner_operator`.'
    __slots__ = ()
    _schema_id = 258
    __match_args__ = ('converted_mekso_operator',)
    def __new__(cls, converted_mekso_operator: RecoveredField[ConvertedMeksoOperatorSyntax]) -> SimpleMeksoOperatorSyntaxConvertedMeksoOperator:
        return cls._from_fields((converted_mekso_operator,))
    def __init__(self, converted_mekso_operator: RecoveredField[ConvertedMeksoOperatorSyntax]) -> None:
        pass
    @property
    def converted_mekso_operator(self) -> RecoveredField[ConvertedMeksoOperatorSyntax]:
        'Uses the `converted_mekso_operator` product form, whose payload preserves `se` and `inner_operator`.'
        return cast(RecoveredField[ConvertedMeksoOperatorSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleMeksoOperatorSyntaxConvertedMeksoOperator is final')

@final
class SimpleMeksoOperatorSyntaxScalarNegatedMeksoOperator(_SyntaxNode):
    'Uses the `scalar_negated_mekso_operator` product form, whose payload preserves `nahe` and `inner_operator`.'
    __slots__ = ()
    _schema_id = 259
    __match_args__ = ('scalar_negated_mekso_operator',)
    def __new__(cls, scalar_negated_mekso_operator: RecoveredField[ScalarNegatedMeksoOperatorSyntax]) -> SimpleMeksoOperatorSyntaxScalarNegatedMeksoOperator:
        return cls._from_fields((scalar_negated_mekso_operator,))
    def __init__(self, scalar_negated_mekso_operator: RecoveredField[ScalarNegatedMeksoOperatorSyntax]) -> None:
        pass
    @property
    def scalar_negated_mekso_operator(self) -> RecoveredField[ScalarNegatedMeksoOperatorSyntax]:
        'Uses the `scalar_negated_mekso_operator` product form, whose payload preserves `nahe` and `inner_operator`.'
        return cast(RecoveredField[ScalarNegatedMeksoOperatorSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleMeksoOperatorSyntaxScalarNegatedMeksoOperator is final')

@final
class SimpleMeksoOperatorSyntaxForethoughtMeksoOperator(_SyntaxNode):
    'Uses the `forethought_mekso_operator` product form, whose payload preserves `guhek`, `left_operator`, `gik`, and `right_operator`.'
    __slots__ = ()
    _schema_id = 260
    __match_args__ = ('forethought_mekso_operator',)
    def __new__(cls, forethought_mekso_operator: RecoveredField[ForethoughtMeksoOperatorSyntax]) -> SimpleMeksoOperatorSyntaxForethoughtMeksoOperator:
        return cls._from_fields((forethought_mekso_operator,))
    def __init__(self, forethought_mekso_operator: RecoveredField[ForethoughtMeksoOperatorSyntax]) -> None:
        pass
    @property
    def forethought_mekso_operator(self) -> RecoveredField[ForethoughtMeksoOperatorSyntax]:
        'Uses the `forethought_mekso_operator` product form, whose payload preserves `guhek`, `left_operator`, `gik`, and `right_operator`.'
        return cast(RecoveredField[ForethoughtMeksoOperatorSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleMeksoOperatorSyntaxForethoughtMeksoOperator is final')

@final
class SimpleMeksoOperatorSyntaxGroupedMeksoOperator(_SyntaxNode):
    'Uses the `grouped_mekso_operator` product form, whose payload preserves `ke`, `inner_operator`, and `kehe`.'
    __slots__ = ()
    _schema_id = 261
    __match_args__ = ('grouped_mekso_operator',)
    def __new__(cls, grouped_mekso_operator: RecoveredField[GroupedMeksoOperatorSyntax]) -> SimpleMeksoOperatorSyntaxGroupedMeksoOperator:
        return cls._from_fields((grouped_mekso_operator,))
    def __init__(self, grouped_mekso_operator: RecoveredField[GroupedMeksoOperatorSyntax]) -> None:
        pass
    @property
    def grouped_mekso_operator(self) -> RecoveredField[GroupedMeksoOperatorSyntax]:
        'Uses the `grouped_mekso_operator` product form, whose payload preserves `ke`, `inner_operator`, and `kehe`.'
        return cast(RecoveredField[GroupedMeksoOperatorSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleMeksoOperatorSyntaxGroupedMeksoOperator is final')

@final
class SimpleMeksoOperatorSyntaxSelbriMeksoOperator(_SyntaxNode):
    'Uses the `selbri_mekso_operator` product form, whose payload preserves `nahu`, `selbri`, and `tehu`.'
    __slots__ = ()
    _schema_id = 262
    __match_args__ = ('selbri_mekso_operator',)
    def __new__(cls, selbri_mekso_operator: RecoveredField[SelbriMeksoOperatorSyntax]) -> SimpleMeksoOperatorSyntaxSelbriMeksoOperator:
        return cls._from_fields((selbri_mekso_operator,))
    def __init__(self, selbri_mekso_operator: RecoveredField[SelbriMeksoOperatorSyntax]) -> None:
        pass
    @property
    def selbri_mekso_operator(self) -> RecoveredField[SelbriMeksoOperatorSyntax]:
        'Uses the `selbri_mekso_operator` product form, whose payload preserves `nahu`, `selbri`, and `tehu`.'
        return cast(RecoveredField[SelbriMeksoOperatorSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleMeksoOperatorSyntaxSelbriMeksoOperator is final')

@final
class SimpleMeksoOperatorSyntaxOperandMeksoOperator(_SyntaxNode):
    'Uses the `operand_mekso_operator` product form, whose payload preserves `maho`, `mekso`, and `tehu`.'
    __slots__ = ()
    _schema_id = 263
    __match_args__ = ('operand_mekso_operator',)
    def __new__(cls, operand_mekso_operator: RecoveredField[OperandMeksoOperatorSyntax]) -> SimpleMeksoOperatorSyntaxOperandMeksoOperator:
        return cls._from_fields((operand_mekso_operator,))
    def __init__(self, operand_mekso_operator: RecoveredField[OperandMeksoOperatorSyntax]) -> None:
        pass
    @property
    def operand_mekso_operator(self) -> RecoveredField[OperandMeksoOperatorSyntax]:
        'Uses the `operand_mekso_operator` product form, whose payload preserves `maho`, `mekso`, and `tehu`.'
        return cast(RecoveredField[OperandMeksoOperatorSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleMeksoOperatorSyntaxOperandMeksoOperator is final')

@final
class SimpleMeksoOperatorSyntaxZantufaMahoSelbriMeksoOperator(_SyntaxNode):
    'Uses the `zantufa_maho_selbri_mekso_operator` product form, whose payload preserves `maho`, `selbri`, and `tehu`.'
    __slots__ = ()
    _schema_id = 264
    __match_args__ = ('zantufa_maho_selbri_mekso_operator',)
    def __new__(cls, zantufa_maho_selbri_mekso_operator: RecoveredField[ZantufaMahoSelbriMeksoOperatorSyntax]) -> SimpleMeksoOperatorSyntaxZantufaMahoSelbriMeksoOperator:
        return cls._from_fields((zantufa_maho_selbri_mekso_operator,))
    def __init__(self, zantufa_maho_selbri_mekso_operator: RecoveredField[ZantufaMahoSelbriMeksoOperatorSyntax]) -> None:
        pass
    @property
    def zantufa_maho_selbri_mekso_operator(self) -> RecoveredField[ZantufaMahoSelbriMeksoOperatorSyntax]:
        'Uses the `zantufa_maho_selbri_mekso_operator` product form, whose payload preserves `maho`, `selbri`, and `tehu`.'
        return cast(RecoveredField[ZantufaMahoSelbriMeksoOperatorSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleMeksoOperatorSyntaxZantufaMahoSelbriMeksoOperator is final')

@final
class SimpleMeksoOperatorSyntaxZantufaMahoSumtiMeksoOperator(_SyntaxNode):
    'Uses the `zantufa_maho_sumti_mekso_operator` product form, whose payload preserves `maho`, `sumti`, and `tehu`.'
    __slots__ = ()
    _schema_id = 265
    __match_args__ = ('zantufa_maho_sumti_mekso_operator',)
    def __new__(cls, zantufa_maho_sumti_mekso_operator: RecoveredField[ZantufaMahoSumtiMeksoOperatorSyntax]) -> SimpleMeksoOperatorSyntaxZantufaMahoSumtiMeksoOperator:
        return cls._from_fields((zantufa_maho_sumti_mekso_operator,))
    def __init__(self, zantufa_maho_sumti_mekso_operator: RecoveredField[ZantufaMahoSumtiMeksoOperatorSyntax]) -> None:
        pass
    @property
    def zantufa_maho_sumti_mekso_operator(self) -> RecoveredField[ZantufaMahoSumtiMeksoOperatorSyntax]:
        'Uses the `zantufa_maho_sumti_mekso_operator` product form, whose payload preserves `maho`, `sumti`, and `tehu`.'
        return cast(RecoveredField[ZantufaMahoSumtiMeksoOperatorSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleMeksoOperatorSyntaxZantufaMahoSumtiMeksoOperator is final')

@final
class SimpleMeksoOperatorSyntaxZantufaConnectiveMeksoOperator(_SyntaxNode):
    'Uses the `zantufa_connective_mekso_operator` product form, whose payload preserves `connective`.'
    __slots__ = ()
    _schema_id = 266
    __match_args__ = ('zantufa_connective_mekso_operator',)
    def __new__(cls, zantufa_connective_mekso_operator: RecoveredField[ZantufaConnectiveMeksoOperatorSyntax]) -> SimpleMeksoOperatorSyntaxZantufaConnectiveMeksoOperator:
        return cls._from_fields((zantufa_connective_mekso_operator,))
    def __init__(self, zantufa_connective_mekso_operator: RecoveredField[ZantufaConnectiveMeksoOperatorSyntax]) -> None:
        pass
    @property
    def zantufa_connective_mekso_operator(self) -> RecoveredField[ZantufaConnectiveMeksoOperatorSyntax]:
        'Uses the `zantufa_connective_mekso_operator` product form, whose payload preserves `connective`.'
        return cast(RecoveredField[ZantufaConnectiveMeksoOperatorSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleMeksoOperatorSyntaxZantufaConnectiveMeksoOperator is final')

@final
class SimpleMeksoOperatorSyntaxPrimitiveMeksoOperator(_SyntaxNode):
    'Uses the `primitive_mekso_operator` product form, whose payload preserves `vuhu`.'
    __slots__ = ()
    _schema_id = 267
    __match_args__ = ('primitive_mekso_operator',)
    def __new__(cls, primitive_mekso_operator: RecoveredField[PrimitiveMeksoOperatorSyntax]) -> SimpleMeksoOperatorSyntaxPrimitiveMeksoOperator:
        return cls._from_fields((primitive_mekso_operator,))
    def __init__(self, primitive_mekso_operator: RecoveredField[PrimitiveMeksoOperatorSyntax]) -> None:
        pass
    @property
    def primitive_mekso_operator(self) -> RecoveredField[PrimitiveMeksoOperatorSyntax]:
        'Uses the `primitive_mekso_operator` product form, whose payload preserves `vuhu`.'
        return cast(RecoveredField[PrimitiveMeksoOperatorSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleMeksoOperatorSyntaxPrimitiveMeksoOperator is final')

SimpleMeksoOperatorSyntax: TypeAlias = SimpleMeksoOperatorSyntaxConvertedMeksoOperator | SimpleMeksoOperatorSyntaxScalarNegatedMeksoOperator | SimpleMeksoOperatorSyntaxForethoughtMeksoOperator | SimpleMeksoOperatorSyntaxGroupedMeksoOperator | SimpleMeksoOperatorSyntaxSelbriMeksoOperator | SimpleMeksoOperatorSyntaxOperandMeksoOperator | SimpleMeksoOperatorSyntaxZantufaMahoSelbriMeksoOperator | SimpleMeksoOperatorSyntaxZantufaMahoSumtiMeksoOperator | SimpleMeksoOperatorSyntaxZantufaConnectiveMeksoOperator | SimpleMeksoOperatorSyntaxPrimitiveMeksoOperator

@final
class ConvertedMeksoOperatorSyntax(_SyntaxNode):
    'Product node for converted operator; preserves `se` and `inner_operator` in source order.'
    __slots__ = ()
    _schema_id = 268
    __match_args__ = ('se', 'inner_operator')
    def __new__(cls, se: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_operator: RecoveredField[MeksoOperatorSyntax]) -> ConvertedMeksoOperatorSyntax:
        return cls._from_fields((se, inner_operator))
    def __init__(self, se: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_operator: RecoveredField[MeksoOperatorSyntax]) -> None:
        pass
    @property
    def se(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Se`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def inner_operator(self) -> RecoveredField[MeksoOperatorSyntax]:
        'The shared inner operator child syntax node.'
        return cast(RecoveredField[MeksoOperatorSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ConvertedMeksoOperatorSyntax is final')

@final
class ScalarNegatedMeksoOperatorSyntax(_SyntaxNode):
    'Product node for converted operator; preserves `nahe` and `inner_operator` in source order.'
    __slots__ = ()
    _schema_id = 269
    __match_args__ = ('nahe', 'inner_operator')
    def __new__(cls, nahe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_operator: RecoveredField[MeksoOperatorSyntax]) -> ScalarNegatedMeksoOperatorSyntax:
        return cls._from_fields((nahe, inner_operator))
    def __init__(self, nahe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_operator: RecoveredField[MeksoOperatorSyntax]) -> None:
        pass
    @property
    def nahe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Nahe`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def inner_operator(self) -> RecoveredField[MeksoOperatorSyntax]:
        'The shared inner operator child syntax node.'
        return cast(RecoveredField[MeksoOperatorSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ScalarNegatedMeksoOperatorSyntax is final')

@final
class ForethoughtMeksoOperatorSyntax(_SyntaxNode):
    'Product node for operator; preserves `guhek`, `left_operator`, `gik`, and `right_operator` in source order.'
    __slots__ = ()
    _schema_id = 270
    __match_args__ = ('guhek', 'left_operator', 'gik', 'right_operator')
    def __new__(cls, guhek: RecoveredField[GuhekConnectiveSyntax], left_operator: RecoveredField[MeksoOperatorSyntax], gik: RecoveredField[GikConnectiveSyntax], right_operator: RecoveredField[MeksoOperatorSyntax]) -> ForethoughtMeksoOperatorSyntax:
        return cls._from_fields((guhek, left_operator, gik, right_operator))
    def __init__(self, guhek: RecoveredField[GuhekConnectiveSyntax], left_operator: RecoveredField[MeksoOperatorSyntax], gik: RecoveredField[GikConnectiveSyntax], right_operator: RecoveredField[MeksoOperatorSyntax]) -> None:
        pass
    @property
    def guhek(self) -> RecoveredField[GuhekConnectiveSyntax]:
        'The `guhek_connective` forethought connective opening the paired branches of the `forethought_mekso_operator` production.'
        return cast(RecoveredField[GuhekConnectiveSyntax], self._field(0))
    @property
    def left_operator(self) -> RecoveredField[MeksoOperatorSyntax]:
        'The shared left operator child syntax node.'
        return cast(RecoveredField[MeksoOperatorSyntax], self._field(1))
    @property
    def gik(self) -> RecoveredField[GikConnectiveSyntax]:
        'The GI-family `gik_connective` connective separating the forethought branches of the `forethought_mekso_operator` production.'
        return cast(RecoveredField[GikConnectiveSyntax], self._field(2))
    @property
    def right_operator(self) -> RecoveredField[MeksoOperatorSyntax]:
        'The shared right operator child syntax node.'
        return cast(RecoveredField[MeksoOperatorSyntax], self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('ForethoughtMeksoOperatorSyntax is final')

@final
class GroupedMeksoOperatorSyntax(_SyntaxNode):
    'Product node for grouped operator; preserves `ke`, `inner_operator`, and `kehe` in source order.'
    __slots__ = ()
    _schema_id = 271
    __match_args__ = ('ke', 'inner_operator', 'kehe')
    def __new__(cls, ke: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_operator: RecoveredField[MeksoOperatorSyntax], kehe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> GroupedMeksoOperatorSyntax:
        return cls._from_fields((ke, inner_operator, kehe))
    def __init__(self, ke: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_operator: RecoveredField[MeksoOperatorSyntax], kehe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def ke(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Ke` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def inner_operator(self) -> RecoveredField[MeksoOperatorSyntax]:
        'The shared inner operator child syntax node.'
        return cast(RecoveredField[MeksoOperatorSyntax], self._field(1))
    @property
    def kehe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Kehe` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('GroupedMeksoOperatorSyntax is final')

@final
class SelbriMeksoOperatorSyntax(_SyntaxNode):
    'Product node for selbri-to-operator; preserves `nahu`, `selbri`, and `tehu` in source order.'
    __slots__ = ()
    _schema_id = 272
    __match_args__ = ('nahu', 'selbri', 'tehu')
    def __new__(cls, nahu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], selbri: RecoveredField[SelbriSyntax], tehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> SelbriMeksoOperatorSyntax:
        return cls._from_fields((nahu, selbri, tehu))
    def __init__(self, nahu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], selbri: RecoveredField[SelbriSyntax], tehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def nahu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Nahu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def selbri(self) -> RecoveredField[SelbriSyntax]:
        'The shared selbri child syntax node.'
        return cast(RecoveredField[SelbriSyntax], self._field(1))
    @property
    def tehu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Tehu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('SelbriMeksoOperatorSyntax is final')

@final
class OperandMeksoOperatorSyntax(_SyntaxNode):
    'Product node for operand-to-operator; preserves `maho`, `mekso`, and `tehu` in source order.'
    __slots__ = ()
    _schema_id = 273
    __match_args__ = ('maho', 'mekso', 'tehu')
    def __new__(cls, maho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], mekso: RecoveredField[MeksoSyntax], tehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> OperandMeksoOperatorSyntax:
        return cls._from_fields((maho, mekso, tehu))
    def __init__(self, maho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], mekso: RecoveredField[MeksoSyntax], tehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def maho(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Maho` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def mekso(self) -> RecoveredField[MeksoSyntax]:
        'The shared mekso child syntax node.'
        return cast(RecoveredField[MeksoSyntax], self._field(1))
    @property
    def tehu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Tehu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('OperandMeksoOperatorSyntax is final')

@final
class ZantufaMahoSelbriMeksoOperatorSyntax(_SyntaxNode):
    'Product node for selbri-to-operator; preserves `maho`, `selbri`, and `tehu` in source order.'
    __slots__ = ()
    _schema_id = 274
    __match_args__ = ('maho', 'selbri', 'tehu')
    def __new__(cls, maho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], selbri: RecoveredField[SelbriSyntax], tehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> ZantufaMahoSelbriMeksoOperatorSyntax:
        return cls._from_fields((maho, selbri, tehu))
    def __init__(self, maho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], selbri: RecoveredField[SelbriSyntax], tehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def maho(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Maho` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def selbri(self) -> RecoveredField[SelbriSyntax]:
        'The shared selbri child syntax node.'
        return cast(RecoveredField[SelbriSyntax], self._field(1))
    @property
    def tehu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Tehu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaMahoSelbriMeksoOperatorSyntax is final')

@final
class ZantufaMahoSumtiMeksoOperatorSyntax(_SyntaxNode):
    'Product node for sumti-to-operator; preserves `maho`, `sumti`, and `tehu` in source order.'
    __slots__ = ()
    _schema_id = 275
    __match_args__ = ('maho', 'sumti', 'tehu')
    def __new__(cls, maho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], sumti: RecoveredField[SumtiSyntax], tehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> ZantufaMahoSumtiMeksoOperatorSyntax:
        return cls._from_fields((maho, sumti, tehu))
    def __init__(self, maho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], sumti: RecoveredField[SumtiSyntax], tehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def maho(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Maho` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def sumti(self) -> RecoveredField[SumtiSyntax]:
        'The shared sumti child syntax node.'
        return cast(RecoveredField[SumtiSyntax], self._field(1))
    @property
    def tehu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Tehu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaMahoSumtiMeksoOperatorSyntax is final')

@final
class ZantufaConnectiveMeksoOperatorSyntax(_SyntaxNode):
    'Transparent product node for connective operator; preserves the `connective` component.'
    __slots__ = ()
    _schema_id = 276
    __match_args__ = ('connective',)
    def __new__(cls, connective: RecoveredField[OperandConnectiveSyntax]) -> ZantufaConnectiveMeksoOperatorSyntax:
        return cls._from_fields((connective,))
    def __init__(self, connective: RecoveredField[OperandConnectiveSyntax]) -> None:
        pass
    @property
    def connective(self) -> RecoveredField[OperandConnectiveSyntax]:
        'The shared connective child syntax node.'
        return cast(RecoveredField[OperandConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaConnectiveMeksoOperatorSyntax is final')

@final
class MeksoOperandSyntaxAfterthoughtMeksoOperand(_SyntaxNode):
    'Uses the `afterthought_mekso_operand` product form, whose payload preserves `operands`.'
    __slots__ = ()
    _schema_id = 277
    __match_args__ = ('afterthought_mekso_operand',)
    def __new__(cls, afterthought_mekso_operand: RecoveredField[AfterthoughtMeksoOperandSyntax]) -> MeksoOperandSyntaxAfterthoughtMeksoOperand:
        return cls._from_fields((afterthought_mekso_operand,))
    def __init__(self, afterthought_mekso_operand: RecoveredField[AfterthoughtMeksoOperandSyntax]) -> None:
        pass
    @property
    def afterthought_mekso_operand(self) -> RecoveredField[AfterthoughtMeksoOperandSyntax]:
        'Uses the `afterthought_mekso_operand` product form, whose payload preserves `operands`.'
        return cast(RecoveredField[AfterthoughtMeksoOperandSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('MeksoOperandSyntaxAfterthoughtMeksoOperand is final')

@final
class MeksoOperandSyntaxBoundMeksoOperand(_SyntaxNode):
    'Uses the `bound_mekso_operand` product form, whose payload preserves `left_expression`, `operand_connective`, `tense_modal`, `bo`, and `right_expression`.'
    __slots__ = ()
    _schema_id = 278
    __match_args__ = ('bound_mekso_operand',)
    def __new__(cls, bound_mekso_operand: RecoveredField[BoundMeksoOperandSyntax]) -> MeksoOperandSyntaxBoundMeksoOperand:
        return cls._from_fields((bound_mekso_operand,))
    def __init__(self, bound_mekso_operand: RecoveredField[BoundMeksoOperandSyntax]) -> None:
        pass
    @property
    def bound_mekso_operand(self) -> RecoveredField[BoundMeksoOperandSyntax]:
        'Uses the `bound_mekso_operand` product form, whose payload preserves `left_expression`, `operand_connective`, `tense_modal`, `bo`, and `right_expression`.'
        return cast(RecoveredField[BoundMeksoOperandSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('MeksoOperandSyntaxBoundMeksoOperand is final')

@final
class MeksoOperandSyntaxSimpleMeksoOperand(_SyntaxNode):
    'Uses the nested `simple_mekso_operand` sum form and preserves its selected alternative.'
    __slots__ = ()
    _schema_id = 279
    __match_args__ = ('simple_mekso_operand',)
    def __new__(cls, simple_mekso_operand: RecoveredField[SimpleMeksoOperandSyntax]) -> MeksoOperandSyntaxSimpleMeksoOperand:
        return cls._from_fields((simple_mekso_operand,))
    def __init__(self, simple_mekso_operand: RecoveredField[SimpleMeksoOperandSyntax]) -> None:
        pass
    @property
    def simple_mekso_operand(self) -> RecoveredField[SimpleMeksoOperandSyntax]:
        'Uses the nested `simple_mekso_operand` sum form and preserves its selected alternative.'
        return cast(RecoveredField[SimpleMeksoOperandSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('MeksoOperandSyntaxSimpleMeksoOperand is final')

MeksoOperandSyntax: TypeAlias = MeksoOperandSyntaxAfterthoughtMeksoOperand | MeksoOperandSyntaxBoundMeksoOperand | MeksoOperandSyntaxSimpleMeksoOperand

@final
class AfterthoughtMeksoOperandSyntax(_SyntaxNode):
    'Transparent product node for operand connective; preserves the `operands` component.'
    __slots__ = ()
    _schema_id = 280
    __match_args__ = ('operands',)
    def __new__(cls, operands: Chain[RecoveredField[BoundOrSimpleMeksoOperandSyntax], RecoveredField[AfterthoughtMeksoOperandContinuationSyntax]]) -> AfterthoughtMeksoOperandSyntax:
        return cls._from_fields((operands,))
    def __init__(self, operands: Chain[RecoveredField[BoundOrSimpleMeksoOperandSyntax], RecoveredField[AfterthoughtMeksoOperandContinuationSyntax]]) -> None:
        pass
    @property
    def operands(self) -> Chain[RecoveredField[BoundOrSimpleMeksoOperandSyntax], RecoveredField[AfterthoughtMeksoOperandContinuationSyntax]]:
        'The source-ordered `operands` chain assembled by the `afterthought_mekso_operand` production.'
        return cast(Chain[RecoveredField[BoundOrSimpleMeksoOperandSyntax], RecoveredField[AfterthoughtMeksoOperandContinuationSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('AfterthoughtMeksoOperandSyntax is final')

@final
class AfterthoughtMeksoOperandContinuationSyntax(_SyntaxNode):
    'Product node for operand continuation; preserves `operand_connective` and `trailing_expression` in source order.'
    __slots__ = ()
    _schema_id = 281
    __match_args__ = ('operand_connective', 'trailing_expression')
    def __new__(cls, operand_connective: RecoveredField[OperandConnectiveSyntax], trailing_expression: RecoveredField[BoundOrSimpleMeksoOperandSyntax]) -> AfterthoughtMeksoOperandContinuationSyntax:
        return cls._from_fields((operand_connective, trailing_expression))
    def __init__(self, operand_connective: RecoveredField[OperandConnectiveSyntax], trailing_expression: RecoveredField[BoundOrSimpleMeksoOperandSyntax]) -> None:
        pass
    @property
    def operand_connective(self) -> RecoveredField[OperandConnectiveSyntax]:
        'The `operand_connective` connective joining the adjacent constituents of the `afterthought_mekso_operand_continuation` production.'
        return cast(RecoveredField[OperandConnectiveSyntax], self._field(0))
    @property
    def trailing_expression(self) -> RecoveredField[BoundOrSimpleMeksoOperandSyntax]:
        'The shared trailing expression child syntax node.'
        return cast(RecoveredField[BoundOrSimpleMeksoOperandSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('AfterthoughtMeksoOperandContinuationSyntax is final')

@final
class BoundOrSimpleMeksoOperandSyntaxBoundMeksoOperand(_SyntaxNode):
    'Uses the `bound_mekso_operand` product form, whose payload preserves `left_expression`, `operand_connective`, `tense_modal`, `bo`, and `right_expression`.'
    __slots__ = ()
    _schema_id = 282
    __match_args__ = ('bound_mekso_operand',)
    def __new__(cls, bound_mekso_operand: RecoveredField[BoundMeksoOperandSyntax]) -> BoundOrSimpleMeksoOperandSyntaxBoundMeksoOperand:
        return cls._from_fields((bound_mekso_operand,))
    def __init__(self, bound_mekso_operand: RecoveredField[BoundMeksoOperandSyntax]) -> None:
        pass
    @property
    def bound_mekso_operand(self) -> RecoveredField[BoundMeksoOperandSyntax]:
        'Uses the `bound_mekso_operand` product form, whose payload preserves `left_expression`, `operand_connective`, `tense_modal`, `bo`, and `right_expression`.'
        return cast(RecoveredField[BoundMeksoOperandSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('BoundOrSimpleMeksoOperandSyntaxBoundMeksoOperand is final')

@final
class BoundOrSimpleMeksoOperandSyntaxSimpleMeksoOperand(_SyntaxNode):
    'Uses the nested `simple_mekso_operand` sum form and preserves its selected alternative.'
    __slots__ = ()
    _schema_id = 283
    __match_args__ = ('simple_mekso_operand',)
    def __new__(cls, simple_mekso_operand: RecoveredField[SimpleMeksoOperandSyntax]) -> BoundOrSimpleMeksoOperandSyntaxSimpleMeksoOperand:
        return cls._from_fields((simple_mekso_operand,))
    def __init__(self, simple_mekso_operand: RecoveredField[SimpleMeksoOperandSyntax]) -> None:
        pass
    @property
    def simple_mekso_operand(self) -> RecoveredField[SimpleMeksoOperandSyntax]:
        'Uses the nested `simple_mekso_operand` sum form and preserves its selected alternative.'
        return cast(RecoveredField[SimpleMeksoOperandSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('BoundOrSimpleMeksoOperandSyntaxSimpleMeksoOperand is final')

BoundOrSimpleMeksoOperandSyntax: TypeAlias = BoundOrSimpleMeksoOperandSyntaxBoundMeksoOperand | BoundOrSimpleMeksoOperandSyntaxSimpleMeksoOperand

@final
class BoundMeksoOperandSyntax(_SyntaxNode):
    'Product node for operand connective; preserves `left_expression`, `operand_connective`, `tense_modal`, `bo`, and `right_expression` in source order.'
    __slots__ = ()
    _schema_id = 284
    __match_args__ = ('left_expression', 'operand_connective', 'tense_modal', 'bo', 'right_expression')
    def __new__(cls, left_expression: RecoveredField[SimpleMeksoOperandSyntax], operand_connective: RecoveredField[OperandConnectiveSyntax], tense_modal: RecoveredField[TenseModalSyntax] | None, bo: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], right_expression: RecoveredField[MeksoOperandSyntax]) -> BoundMeksoOperandSyntax:
        return cls._from_fields((left_expression, operand_connective, tense_modal, bo, right_expression))
    def __init__(self, left_expression: RecoveredField[SimpleMeksoOperandSyntax], operand_connective: RecoveredField[OperandConnectiveSyntax], tense_modal: RecoveredField[TenseModalSyntax] | None, bo: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], right_expression: RecoveredField[MeksoOperandSyntax]) -> None:
        pass
    @property
    def left_expression(self) -> RecoveredField[SimpleMeksoOperandSyntax]:
        'The shared left expression child syntax node.'
        return cast(RecoveredField[SimpleMeksoOperandSyntax], self._field(0))
    @property
    def operand_connective(self) -> RecoveredField[OperandConnectiveSyntax]:
        'The `operand_connective` connective joining the adjacent constituents of the `bound_mekso_operand` production.'
        return cast(RecoveredField[OperandConnectiveSyntax], self._field(1))
    @property
    def tense_modal(self) -> RecoveredField[TenseModalSyntax] | None:
        'The optional tense modal component.'
        return cast(RecoveredField[TenseModalSyntax] | None, self._field(2))
    @property
    def bo(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Bo` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(3))
    @property
    def right_expression(self) -> RecoveredField[MeksoOperandSyntax]:
        'The shared right expression child syntax node.'
        return cast(RecoveredField[MeksoOperandSyntax], self._field(4))
    def __init_subclass__(cls) -> None:
        raise TypeError('BoundMeksoOperandSyntax is final')

@final
class SimpleMeksoOperandSyntaxForethoughtMeksoOperand(_SyntaxNode):
    'Uses the `forethought_mekso_operand` product form, whose payload preserves `gek`, `left_expression`, `gik`, and `right_expression`.'
    __slots__ = ()
    _schema_id = 285
    __match_args__ = ('forethought_mekso_operand',)
    def __new__(cls, forethought_mekso_operand: RecoveredField[ForethoughtMeksoOperandSyntax]) -> SimpleMeksoOperandSyntaxForethoughtMeksoOperand:
        return cls._from_fields((forethought_mekso_operand,))
    def __init__(self, forethought_mekso_operand: RecoveredField[ForethoughtMeksoOperandSyntax]) -> None:
        pass
    @property
    def forethought_mekso_operand(self) -> RecoveredField[ForethoughtMeksoOperandSyntax]:
        'Uses the `forethought_mekso_operand` product form, whose payload preserves `gek`, `left_expression`, `gik`, and `right_expression`.'
        return cast(RecoveredField[ForethoughtMeksoOperandSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleMeksoOperandSyntaxForethoughtMeksoOperand is final')

@final
class SimpleMeksoOperandSyntaxQualifiedMeksoOperand(_SyntaxNode):
    'Uses the `qualified_mekso_operand` product form, whose payload preserves `nahe`, `bo`, `inner_expression`, and `luhu`.'
    __slots__ = ()
    _schema_id = 286
    __match_args__ = ('qualified_mekso_operand',)
    def __new__(cls, qualified_mekso_operand: RecoveredField[QualifiedMeksoOperandSyntax]) -> SimpleMeksoOperandSyntaxQualifiedMeksoOperand:
        return cls._from_fields((qualified_mekso_operand,))
    def __init__(self, qualified_mekso_operand: RecoveredField[QualifiedMeksoOperandSyntax]) -> None:
        pass
    @property
    def qualified_mekso_operand(self) -> RecoveredField[QualifiedMeksoOperandSyntax]:
        'Uses the `qualified_mekso_operand` product form, whose payload preserves `nahe`, `bo`, `inner_expression`, and `luhu`.'
        return cast(RecoveredField[QualifiedMeksoOperandSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleMeksoOperandSyntaxQualifiedMeksoOperand is final')

@final
class SimpleMeksoOperandSyntaxParenthesizedMeksoOperand(_SyntaxNode):
    'Uses the `parenthesized_mekso_operand` product form, whose payload preserves `vei`, `inner_expression`, and `veho`.'
    __slots__ = ()
    _schema_id = 287
    __match_args__ = ('parenthesized_mekso_operand',)
    def __new__(cls, parenthesized_mekso_operand: RecoveredField[ParenthesizedMeksoOperandSyntax]) -> SimpleMeksoOperandSyntaxParenthesizedMeksoOperand:
        return cls._from_fields((parenthesized_mekso_operand,))
    def __init__(self, parenthesized_mekso_operand: RecoveredField[ParenthesizedMeksoOperandSyntax]) -> None:
        pass
    @property
    def parenthesized_mekso_operand(self) -> RecoveredField[ParenthesizedMeksoOperandSyntax]:
        'Uses the `parenthesized_mekso_operand` product form, whose payload preserves `vei`, `inner_expression`, and `veho`.'
        return cast(RecoveredField[ParenthesizedMeksoOperandSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleMeksoOperandSyntaxParenthesizedMeksoOperand is final')

@final
class SimpleMeksoOperandSyntaxSumtiMeksoOperand(_SyntaxNode):
    'Uses the `sumti_mekso_operand` product form, whose payload preserves `mohe`, `sumti`, and `tehu`.'
    __slots__ = ()
    _schema_id = 288
    __match_args__ = ('sumti_mekso_operand',)
    def __new__(cls, sumti_mekso_operand: RecoveredField[SumtiMeksoOperandSyntax]) -> SimpleMeksoOperandSyntaxSumtiMeksoOperand:
        return cls._from_fields((sumti_mekso_operand,))
    def __init__(self, sumti_mekso_operand: RecoveredField[SumtiMeksoOperandSyntax]) -> None:
        pass
    @property
    def sumti_mekso_operand(self) -> RecoveredField[SumtiMeksoOperandSyntax]:
        'Uses the `sumti_mekso_operand` product form, whose payload preserves `mohe`, `sumti`, and `tehu`.'
        return cast(RecoveredField[SumtiMeksoOperandSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleMeksoOperandSyntaxSumtiMeksoOperand is final')

@final
class SimpleMeksoOperandSyntaxSelbriMeksoOperand(_SyntaxNode):
    'Uses the `selbri_mekso_operand` product form, whose payload preserves `nihe`, `selbri`, and `tehu`.'
    __slots__ = ()
    _schema_id = 289
    __match_args__ = ('selbri_mekso_operand',)
    def __new__(cls, selbri_mekso_operand: RecoveredField[SelbriMeksoOperandSyntax]) -> SimpleMeksoOperandSyntaxSelbriMeksoOperand:
        return cls._from_fields((selbri_mekso_operand,))
    def __init__(self, selbri_mekso_operand: RecoveredField[SelbriMeksoOperandSyntax]) -> None:
        pass
    @property
    def selbri_mekso_operand(self) -> RecoveredField[SelbriMeksoOperandSyntax]:
        'Uses the `selbri_mekso_operand` product form, whose payload preserves `nihe`, `selbri`, and `tehu`.'
        return cast(RecoveredField[SelbriMeksoOperandSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleMeksoOperandSyntaxSelbriMeksoOperand is final')

@final
class SimpleMeksoOperandSyntaxArrayMeksoOperand(_SyntaxNode):
    'Uses the `array_mekso_operand` product form, whose payload preserves `johi`, `expressions`, and `tehu`.'
    __slots__ = ()
    _schema_id = 290
    __match_args__ = ('array_mekso_operand',)
    def __new__(cls, array_mekso_operand: RecoveredField[ArrayMeksoOperandSyntax]) -> SimpleMeksoOperandSyntaxArrayMeksoOperand:
        return cls._from_fields((array_mekso_operand,))
    def __init__(self, array_mekso_operand: RecoveredField[ArrayMeksoOperandSyntax]) -> None:
        pass
    @property
    def array_mekso_operand(self) -> RecoveredField[ArrayMeksoOperandSyntax]:
        'Uses the `array_mekso_operand` product form, whose payload preserves `johi`, `expressions`, and `tehu`.'
        return cast(RecoveredField[ArrayMeksoOperandSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleMeksoOperandSyntaxArrayMeksoOperand is final')

@final
class SimpleMeksoOperandSyntaxNumberMekso(_SyntaxNode):
    'Uses the `number_mekso` product form, whose payload preserves `quantifier`.'
    __slots__ = ()
    _schema_id = 291
    __match_args__ = ('number_mekso',)
    def __new__(cls, number_mekso: RecoveredField[NumberMeksoSyntax]) -> SimpleMeksoOperandSyntaxNumberMekso:
        return cls._from_fields((number_mekso,))
    def __init__(self, number_mekso: RecoveredField[NumberMeksoSyntax]) -> None:
        pass
    @property
    def number_mekso(self) -> RecoveredField[NumberMeksoSyntax]:
        'Uses the `number_mekso` product form, whose payload preserves `quantifier`.'
        return cast(RecoveredField[NumberMeksoSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleMeksoOperandSyntaxNumberMekso is final')

@final
class SimpleMeksoOperandSyntaxLerfuStringMekso(_SyntaxNode):
    'Uses the `lerfu_string_mekso` product form, whose payload preserves `letters`, `boi`, and `free_modifiers`.'
    __slots__ = ()
    _schema_id = 292
    __match_args__ = ('lerfu_string_mekso',)
    def __new__(cls, lerfu_string_mekso: RecoveredField[LerfuStringMeksoSyntax]) -> SimpleMeksoOperandSyntaxLerfuStringMekso:
        return cls._from_fields((lerfu_string_mekso,))
    def __init__(self, lerfu_string_mekso: RecoveredField[LerfuStringMeksoSyntax]) -> None:
        pass
    @property
    def lerfu_string_mekso(self) -> RecoveredField[LerfuStringMeksoSyntax]:
        'Uses the `lerfu_string_mekso` product form, whose payload preserves `letters`, `boi`, and `free_modifiers`.'
        return cast(RecoveredField[LerfuStringMeksoSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleMeksoOperandSyntaxLerfuStringMekso is final')

@final
class SimpleMeksoOperandSyntaxZantufaScalarNegatedMeksoOperand(_SyntaxNode):
    'Uses the `zantufa_scalar_negated_mekso_operand` product form, whose payload preserves `nahe` and `inner_expression`.'
    __slots__ = ()
    _schema_id = 293
    __match_args__ = ('zantufa_scalar_negated_mekso_operand',)
    def __new__(cls, zantufa_scalar_negated_mekso_operand: RecoveredField[ZantufaScalarNegatedMeksoOperandSyntax]) -> SimpleMeksoOperandSyntaxZantufaScalarNegatedMeksoOperand:
        return cls._from_fields((zantufa_scalar_negated_mekso_operand,))
    def __init__(self, zantufa_scalar_negated_mekso_operand: RecoveredField[ZantufaScalarNegatedMeksoOperandSyntax]) -> None:
        pass
    @property
    def zantufa_scalar_negated_mekso_operand(self) -> RecoveredField[ZantufaScalarNegatedMeksoOperandSyntax]:
        'Uses the `zantufa_scalar_negated_mekso_operand` product form, whose payload preserves `nahe` and `inner_expression`.'
        return cast(RecoveredField[ZantufaScalarNegatedMeksoOperandSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleMeksoOperandSyntaxZantufaScalarNegatedMeksoOperand is final')

@final
class SimpleMeksoOperandSyntaxZantufaSelbriMoheMeksoOperand(_SyntaxNode):
    'Uses the `zantufa_selbri_mohe_mekso_operand` product form, whose payload preserves `mohe`, `selbri`, and `tehu`.'
    __slots__ = ()
    _schema_id = 294
    __match_args__ = ('zantufa_selbri_mohe_mekso_operand',)
    def __new__(cls, zantufa_selbri_mohe_mekso_operand: RecoveredField[ZantufaSelbriMoheMeksoOperandSyntax]) -> SimpleMeksoOperandSyntaxZantufaSelbriMoheMeksoOperand:
        return cls._from_fields((zantufa_selbri_mohe_mekso_operand,))
    def __init__(self, zantufa_selbri_mohe_mekso_operand: RecoveredField[ZantufaSelbriMoheMeksoOperandSyntax]) -> None:
        pass
    @property
    def zantufa_selbri_mohe_mekso_operand(self) -> RecoveredField[ZantufaSelbriMoheMeksoOperandSyntax]:
        'Uses the `zantufa_selbri_mohe_mekso_operand` product form, whose payload preserves `mohe`, `selbri`, and `tehu`.'
        return cast(RecoveredField[ZantufaSelbriMoheMeksoOperandSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleMeksoOperandSyntaxZantufaSelbriMoheMeksoOperand is final')

SimpleMeksoOperandSyntax: TypeAlias = SimpleMeksoOperandSyntaxForethoughtMeksoOperand | SimpleMeksoOperandSyntaxQualifiedMeksoOperand | SimpleMeksoOperandSyntaxParenthesizedMeksoOperand | SimpleMeksoOperandSyntaxSumtiMeksoOperand | SimpleMeksoOperandSyntaxSelbriMeksoOperand | SimpleMeksoOperandSyntaxArrayMeksoOperand | SimpleMeksoOperandSyntaxNumberMekso | SimpleMeksoOperandSyntaxLerfuStringMekso | SimpleMeksoOperandSyntaxZantufaScalarNegatedMeksoOperand | SimpleMeksoOperandSyntaxZantufaSelbriMoheMeksoOperand

@final
class ZantufaScalarNegatedMeksoOperandSyntax(_SyntaxNode):
    'Product node for scalar-negated operand; preserves `nahe` and `inner_expression` in source order.'
    __slots__ = ()
    _schema_id = 295
    __match_args__ = ('nahe', 'inner_expression')
    def __new__(cls, nahe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_expression: RecoveredField[MeksoOperandSyntax]) -> ZantufaScalarNegatedMeksoOperandSyntax:
        return cls._from_fields((nahe, inner_expression))
    def __init__(self, nahe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_expression: RecoveredField[MeksoOperandSyntax]) -> None:
        pass
    @property
    def nahe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Nahe`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def inner_expression(self) -> RecoveredField[MeksoOperandSyntax]:
        'The shared inner expression child syntax node.'
        return cast(RecoveredField[MeksoOperandSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaScalarNegatedMeksoOperandSyntax is final')

@final
class QualifiedMeksoOperandSyntax(_SyntaxNode):
    'Product node for qualified operand; preserves `nahe`, `bo`, `inner_expression`, and `luhu` in source order.'
    __slots__ = ()
    _schema_id = 296
    __match_args__ = ('nahe', 'bo', 'inner_expression', 'luhu')
    def __new__(cls, nahe: RecoveredField[Token], bo: RecoveredField[Token], inner_expression: RecoveredField[MeksoOperandSyntax], luhu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> QualifiedMeksoOperandSyntax:
        return cls._from_fields((nahe, bo, inner_expression, luhu))
    def __init__(self, nahe: RecoveredField[Token], bo: RecoveredField[Token], inner_expression: RecoveredField[MeksoOperandSyntax], luhu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def nahe(self) -> RecoveredField[Token]:
        'A word from selmaho `Nahe`.'
        return cast(RecoveredField[Token], self._field(0))
    @property
    def bo(self) -> RecoveredField[Token]:
        'The `Bo` cmavo marker.'
        return cast(RecoveredField[Token], self._field(1))
    @property
    def inner_expression(self) -> RecoveredField[MeksoOperandSyntax]:
        'The shared inner expression child syntax node.'
        return cast(RecoveredField[MeksoOperandSyntax], self._field(2))
    @property
    def luhu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Luhu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('QualifiedMeksoOperandSyntax is final')

@final
class ForethoughtMeksoOperandSyntax(_SyntaxNode):
    'Product node for forethought mex; preserves `gek`, `left_expression`, `gik`, and `right_expression` in source order.'
    __slots__ = ()
    _schema_id = 297
    __match_args__ = ('gek', 'left_expression', 'gik', 'right_expression')
    def __new__(cls, gek: RecoveredField[ModalForethoughtConnectiveSyntax], left_expression: RecoveredField[MeksoOperandSyntax], gik: RecoveredField[GikConnectiveSyntax], right_expression: RecoveredField[MeksoOperandSyntax]) -> ForethoughtMeksoOperandSyntax:
        return cls._from_fields((gek, left_expression, gik, right_expression))
    def __init__(self, gek: RecoveredField[ModalForethoughtConnectiveSyntax], left_expression: RecoveredField[MeksoOperandSyntax], gik: RecoveredField[GikConnectiveSyntax], right_expression: RecoveredField[MeksoOperandSyntax]) -> None:
        pass
    @property
    def gek(self) -> RecoveredField[ModalForethoughtConnectiveSyntax]:
        'The `modal_forethought_connective` forethought connective opening the paired branches of the `forethought_mekso_operand` production.'
        return cast(RecoveredField[ModalForethoughtConnectiveSyntax], self._field(0))
    @property
    def left_expression(self) -> RecoveredField[MeksoOperandSyntax]:
        'The shared left expression child syntax node.'
        return cast(RecoveredField[MeksoOperandSyntax], self._field(1))
    @property
    def gik(self) -> RecoveredField[GikConnectiveSyntax]:
        'The GI-family `gik_connective` connective separating the forethought branches of the `forethought_mekso_operand` production.'
        return cast(RecoveredField[GikConnectiveSyntax], self._field(2))
    @property
    def right_expression(self) -> RecoveredField[MeksoOperandSyntax]:
        'The shared right expression child syntax node.'
        return cast(RecoveredField[MeksoOperandSyntax], self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('ForethoughtMeksoOperandSyntax is final')

@final
class SumtiMeksoOperandSyntax(_SyntaxNode):
    'Product node for sumti operand; preserves `mohe`, `sumti`, and `tehu` in source order.'
    __slots__ = ()
    _schema_id = 298
    __match_args__ = ('mohe', 'sumti', 'tehu')
    def __new__(cls, mohe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], sumti: RecoveredField[SumtiSyntax], tehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> SumtiMeksoOperandSyntax:
        return cls._from_fields((mohe, sumti, tehu))
    def __init__(self, mohe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], sumti: RecoveredField[SumtiSyntax], tehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def mohe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Mohe` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def sumti(self) -> RecoveredField[SumtiSyntax]:
        'The shared sumti child syntax node.'
        return cast(RecoveredField[SumtiSyntax], self._field(1))
    @property
    def tehu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Tehu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('SumtiMeksoOperandSyntax is final')

@final
class ZantufaSelbriMoheMeksoOperandSyntax(_SyntaxNode):
    'Product node for selbri operand; preserves `mohe`, `selbri`, and `tehu` in source order.'
    __slots__ = ()
    _schema_id = 299
    __match_args__ = ('mohe', 'selbri', 'tehu')
    def __new__(cls, mohe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], selbri: RecoveredField[SelbriSyntax], tehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> ZantufaSelbriMoheMeksoOperandSyntax:
        return cls._from_fields((mohe, selbri, tehu))
    def __init__(self, mohe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], selbri: RecoveredField[SelbriSyntax], tehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def mohe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Mohe` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def selbri(self) -> RecoveredField[SelbriSyntax]:
        'The shared selbri child syntax node.'
        return cast(RecoveredField[SelbriSyntax], self._field(1))
    @property
    def tehu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Tehu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaSelbriMoheMeksoOperandSyntax is final')

@final
class SelbriMeksoOperandSyntax(_SyntaxNode):
    'Product node for selbri operand; preserves `nihe`, `selbri`, and `tehu` in source order.'
    __slots__ = ()
    _schema_id = 300
    __match_args__ = ('nihe', 'selbri', 'tehu')
    def __new__(cls, nihe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], selbri: RecoveredField[SelbriSyntax], tehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> SelbriMeksoOperandSyntax:
        return cls._from_fields((nihe, selbri, tehu))
    def __init__(self, nihe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], selbri: RecoveredField[SelbriSyntax], tehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def nihe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Nihe` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def selbri(self) -> RecoveredField[SelbriSyntax]:
        'The shared selbri child syntax node.'
        return cast(RecoveredField[SelbriSyntax], self._field(1))
    @property
    def tehu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Tehu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('SelbriMeksoOperandSyntax is final')

@final
class ParenthesizedMeksoOperandSyntax(_SyntaxNode):
    'Product node for parenthesized mex; preserves `vei`, `inner_expression`, and `veho` in source order.'
    __slots__ = ()
    _schema_id = 301
    __match_args__ = ('vei', 'inner_expression', 'veho')
    def __new__(cls, vei: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_expression: RecoveredField[MeksoSyntax], veho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> ParenthesizedMeksoOperandSyntax:
        return cls._from_fields((vei, inner_expression, veho))
    def __init__(self, vei: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_expression: RecoveredField[MeksoSyntax], veho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def vei(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Vei` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def inner_expression(self) -> RecoveredField[MeksoSyntax]:
        'The shared inner expression child syntax node.'
        return cast(RecoveredField[MeksoSyntax], self._field(1))
    @property
    def veho(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Veho` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('ParenthesizedMeksoOperandSyntax is final')

@final
class ArrayMeksoOperandSyntax(_SyntaxNode):
    'Product node for mekso array; preserves `johi`, `expressions`, and `tehu` in source order.'
    __slots__ = ()
    _schema_id = 302
    __match_args__ = ('johi', 'expressions', 'tehu')
    def __new__(cls, johi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], expressions: Sequence[RecoveredField[MeksoSyntax]], tehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> ArrayMeksoOperandSyntax:
        return cls._from_fields((johi, expressions, tehu))
    def __init__(self, johi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], expressions: Sequence[RecoveredField[MeksoSyntax]], tehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def johi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Johi` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def expressions(self) -> tuple[RecoveredField[MeksoSyntax], ...]:
        'Non-empty ordered sequence of expressions components.'
        return cast(tuple[RecoveredField[MeksoSyntax], ...], self._field(1))
    @property
    def tehu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Tehu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('ArrayMeksoOperandSyntax is final')

@final
class LetterStringSyntax(_SyntaxNode):
    'Product node for lerfu string; preserves `first_letter` and `continuations` in source order.'
    __slots__ = ()
    _schema_id = 303
    __match_args__ = ('first_letter', 'continuations')
    def __new__(cls, first_letter: RecoveredField[LetterTokensSyntax], continuations: Sequence[RecoveredField[LetterStringContinuationSyntax]]) -> LetterStringSyntax:
        return cls._from_fields((first_letter, continuations))
    def __init__(self, first_letter: RecoveredField[LetterTokensSyntax], continuations: Sequence[RecoveredField[LetterStringContinuationSyntax]]) -> None:
        pass
    @property
    def first_letter(self) -> RecoveredField[LetterTokensSyntax]:
        'The shared first letter child syntax node.'
        return cast(RecoveredField[LetterTokensSyntax], self._field(0))
    @property
    def continuations(self) -> tuple[RecoveredField[LetterStringContinuationSyntax], ...]:
        'Ordered sequence of zero or more continuations components.'
        return cast(tuple[RecoveredField[LetterStringContinuationSyntax], ...], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('LetterStringSyntax is final')

@final
class LetterStringContinuationSyntaxLetterStringPaContinuation(_SyntaxNode):
    'Uses the `letter_string_pa_continuation` product form, whose payload preserves `pa`.'
    __slots__ = ()
    _schema_id = 304
    __match_args__ = ('letter_string_pa_continuation',)
    def __new__(cls, letter_string_pa_continuation: RecoveredField[LetterStringPaContinuationSyntax]) -> LetterStringContinuationSyntaxLetterStringPaContinuation:
        return cls._from_fields((letter_string_pa_continuation,))
    def __init__(self, letter_string_pa_continuation: RecoveredField[LetterStringPaContinuationSyntax]) -> None:
        pass
    @property
    def letter_string_pa_continuation(self) -> RecoveredField[LetterStringPaContinuationSyntax]:
        'Uses the `letter_string_pa_continuation` product form, whose payload preserves `pa`.'
        return cast(RecoveredField[LetterStringPaContinuationSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('LetterStringContinuationSyntaxLetterStringPaContinuation is final')

@final
class LetterStringContinuationSyntaxLetterStringLerfuContinuation(_SyntaxNode):
    'Uses the `letter_string_lerfu_continuation` product form, whose payload preserves `letter`.'
    __slots__ = ()
    _schema_id = 305
    __match_args__ = ('letter_string_lerfu_continuation',)
    def __new__(cls, letter_string_lerfu_continuation: RecoveredField[LetterStringLerfuContinuationSyntax]) -> LetterStringContinuationSyntaxLetterStringLerfuContinuation:
        return cls._from_fields((letter_string_lerfu_continuation,))
    def __init__(self, letter_string_lerfu_continuation: RecoveredField[LetterStringLerfuContinuationSyntax]) -> None:
        pass
    @property
    def letter_string_lerfu_continuation(self) -> RecoveredField[LetterStringLerfuContinuationSyntax]:
        'Uses the `letter_string_lerfu_continuation` product form, whose payload preserves `letter`.'
        return cast(RecoveredField[LetterStringLerfuContinuationSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('LetterStringContinuationSyntaxLetterStringLerfuContinuation is final')

LetterStringContinuationSyntax: TypeAlias = LetterStringContinuationSyntaxLetterStringPaContinuation | LetterStringContinuationSyntaxLetterStringLerfuContinuation

@final
class LetterStringPaContinuationSyntax(_SyntaxNode):
    'Transparent product node for lerfu string continuation; preserves the `pa` component.'
    __slots__ = ()
    _schema_id = 306
    __match_args__ = ('pa',)
    def __new__(cls, pa: RecoveredField[Token]) -> LetterStringPaContinuationSyntax:
        return cls._from_fields((pa,))
    def __init__(self, pa: RecoveredField[Token]) -> None:
        pass
    @property
    def pa(self) -> RecoveredField[Token]:
        'The `pa_word` grammar result in the `pa` structural role of the `letter_string_pa_continuation` production.'
        return cast(RecoveredField[Token], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('LetterStringPaContinuationSyntax is final')

@final
class LetterStringLerfuContinuationSyntax(_SyntaxNode):
    'Transparent product node for lerfu string continuation; preserves the `letter` component.'
    __slots__ = ()
    _schema_id = 307
    __match_args__ = ('letter',)
    def __new__(cls, letter: RecoveredField[LetterTokensSyntax]) -> LetterStringLerfuContinuationSyntax:
        return cls._from_fields((letter,))
    def __init__(self, letter: RecoveredField[LetterTokensSyntax]) -> None:
        pass
    @property
    def letter(self) -> RecoveredField[LetterTokensSyntax]:
        'The shared letter child syntax node.'
        return cast(RecoveredField[LetterTokensSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('LetterStringLerfuContinuationSyntax is final')

@final
class NumberWordsSyntax(_SyntaxNode):
    'Product node for number; preserves `first_number` and `continuations` in source order.'
    __slots__ = ()
    _schema_id = 308
    __match_args__ = ('first_number', 'continuations')
    def __new__(cls, first_number: RecoveredField[Token], continuations: Sequence[RecoveredField[NumberWordContinuationSyntax]]) -> NumberWordsSyntax:
        return cls._from_fields((first_number, continuations))
    def __init__(self, first_number: RecoveredField[Token], continuations: Sequence[RecoveredField[NumberWordContinuationSyntax]]) -> None:
        pass
    @property
    def first_number(self) -> RecoveredField[Token]:
        'The initial `pa_word` constituent before the continuations of the `number_words` production.'
        return cast(RecoveredField[Token], self._field(0))
    @property
    def continuations(self) -> tuple[RecoveredField[NumberWordContinuationSyntax], ...]:
        'Ordered sequence of zero or more continuations components.'
        return cast(tuple[RecoveredField[NumberWordContinuationSyntax], ...], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('NumberWordsSyntax is final')

@final
class NumberWordContinuationSyntaxNumberWordPaContinuation(_SyntaxNode):
    'Uses the `number_word_pa_continuation` product form, whose payload preserves `pa`.'
    __slots__ = ()
    _schema_id = 309
    __match_args__ = ('number_word_pa_continuation',)
    def __new__(cls, number_word_pa_continuation: RecoveredField[NumberWordPaContinuationSyntax]) -> NumberWordContinuationSyntaxNumberWordPaContinuation:
        return cls._from_fields((number_word_pa_continuation,))
    def __init__(self, number_word_pa_continuation: RecoveredField[NumberWordPaContinuationSyntax]) -> None:
        pass
    @property
    def number_word_pa_continuation(self) -> RecoveredField[NumberWordPaContinuationSyntax]:
        'Uses the `number_word_pa_continuation` product form, whose payload preserves `pa`.'
        return cast(RecoveredField[NumberWordPaContinuationSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('NumberWordContinuationSyntaxNumberWordPaContinuation is final')

@final
class NumberWordContinuationSyntaxNumberWordLerfuContinuation(_SyntaxNode):
    'Uses the `number_word_lerfu_continuation` product form, whose payload preserves `letter`.'
    __slots__ = ()
    _schema_id = 310
    __match_args__ = ('number_word_lerfu_continuation',)
    def __new__(cls, number_word_lerfu_continuation: RecoveredField[NumberWordLerfuContinuationSyntax]) -> NumberWordContinuationSyntaxNumberWordLerfuContinuation:
        return cls._from_fields((number_word_lerfu_continuation,))
    def __init__(self, number_word_lerfu_continuation: RecoveredField[NumberWordLerfuContinuationSyntax]) -> None:
        pass
    @property
    def number_word_lerfu_continuation(self) -> RecoveredField[NumberWordLerfuContinuationSyntax]:
        'Uses the `number_word_lerfu_continuation` product form, whose payload preserves `letter`.'
        return cast(RecoveredField[NumberWordLerfuContinuationSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('NumberWordContinuationSyntaxNumberWordLerfuContinuation is final')

NumberWordContinuationSyntax: TypeAlias = NumberWordContinuationSyntaxNumberWordPaContinuation | NumberWordContinuationSyntaxNumberWordLerfuContinuation

@final
class NumberWordPaContinuationSyntax(_SyntaxNode):
    'Transparent product node for number continuation; preserves the `pa` component.'
    __slots__ = ()
    _schema_id = 311
    __match_args__ = ('pa',)
    def __new__(cls, pa: RecoveredField[Token]) -> NumberWordPaContinuationSyntax:
        return cls._from_fields((pa,))
    def __init__(self, pa: RecoveredField[Token]) -> None:
        pass
    @property
    def pa(self) -> RecoveredField[Token]:
        'The `pa_word` grammar result in the `pa` structural role of the `number_word_pa_continuation` production.'
        return cast(RecoveredField[Token], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('NumberWordPaContinuationSyntax is final')

@final
class NumberWordLerfuContinuationSyntax(_SyntaxNode):
    'Transparent product node for number continuation; preserves the `letter` component.'
    __slots__ = ()
    _schema_id = 312
    __match_args__ = ('letter',)
    def __new__(cls, letter: RecoveredField[LetterTokensSyntax]) -> NumberWordLerfuContinuationSyntax:
        return cls._from_fields((letter,))
    def __init__(self, letter: RecoveredField[LetterTokensSyntax]) -> None:
        pass
    @property
    def letter(self) -> RecoveredField[LetterTokensSyntax]:
        'The shared letter child syntax node.'
        return cast(RecoveredField[LetterTokensSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('NumberWordLerfuContinuationSyntax is final')

@final
class NumberOrLetterWordsSyntaxNumberWords(_SyntaxNode):
    'Uses the `number_words` product form, whose payload preserves `first_number` and `continuations`.'
    __slots__ = ()
    _schema_id = 313
    __match_args__ = ('number_words',)
    def __new__(cls, number_words: RecoveredField[NumberWordsSyntax]) -> NumberOrLetterWordsSyntaxNumberWords:
        return cls._from_fields((number_words,))
    def __init__(self, number_words: RecoveredField[NumberWordsSyntax]) -> None:
        pass
    @property
    def number_words(self) -> RecoveredField[NumberWordsSyntax]:
        'Uses the `number_words` product form, whose payload preserves `first_number` and `continuations`.'
        return cast(RecoveredField[NumberWordsSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('NumberOrLetterWordsSyntaxNumberWords is final')

@final
class NumberOrLetterWordsSyntaxLetterString(_SyntaxNode):
    'Uses the `letter_string` product form, whose payload preserves `first_letter` and `continuations`.'
    __slots__ = ()
    _schema_id = 314
    __match_args__ = ('letter_string',)
    def __new__(cls, letter_string: RecoveredField[LetterStringSyntax]) -> NumberOrLetterWordsSyntaxLetterString:
        return cls._from_fields((letter_string,))
    def __init__(self, letter_string: RecoveredField[LetterStringSyntax]) -> None:
        pass
    @property
    def letter_string(self) -> RecoveredField[LetterStringSyntax]:
        'Uses the `letter_string` product form, whose payload preserves `first_letter` and `continuations`.'
        return cast(RecoveredField[LetterStringSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('NumberOrLetterWordsSyntaxLetterString is final')

NumberOrLetterWordsSyntax: TypeAlias = NumberOrLetterWordsSyntaxNumberWords | NumberOrLetterWordsSyntaxLetterString

@final
class LetterTokensSyntaxSimpleLerfuWord(_SyntaxNode):
    'Uses the `simple_lerfu_word` product form, whose payload preserves `word`.'
    __slots__ = ()
    _schema_id = 315
    __match_args__ = ('simple_lerfu_word',)
    def __new__(cls, simple_lerfu_word: RecoveredField[SimpleLerfuWordSyntax]) -> LetterTokensSyntaxSimpleLerfuWord:
        return cls._from_fields((simple_lerfu_word,))
    def __init__(self, simple_lerfu_word: RecoveredField[SimpleLerfuWordSyntax]) -> None:
        pass
    @property
    def simple_lerfu_word(self) -> RecoveredField[SimpleLerfuWordSyntax]:
        'Uses the `simple_lerfu_word` product form, whose payload preserves `word`.'
        return cast(RecoveredField[SimpleLerfuWordSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('LetterTokensSyntaxSimpleLerfuWord is final')

@final
class LetterTokensSyntaxLauLerfuWord(_SyntaxNode):
    'Uses the `lau_lerfu_word` product form, whose payload preserves `lau` and `letter`.'
    __slots__ = ()
    _schema_id = 316
    __match_args__ = ('lau_lerfu_word',)
    def __new__(cls, lau_lerfu_word: RecoveredField[LauLerfuWordSyntax]) -> LetterTokensSyntaxLauLerfuWord:
        return cls._from_fields((lau_lerfu_word,))
    def __init__(self, lau_lerfu_word: RecoveredField[LauLerfuWordSyntax]) -> None:
        pass
    @property
    def lau_lerfu_word(self) -> RecoveredField[LauLerfuWordSyntax]:
        'Uses the `lau_lerfu_word` product form, whose payload preserves `lau` and `letter`.'
        return cast(RecoveredField[LauLerfuWordSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('LetterTokensSyntaxLauLerfuWord is final')

@final
class LetterTokensSyntaxTeiLerfuWord(_SyntaxNode):
    'Uses the `tei_lerfu_word` product form, whose payload preserves `tei`, `letters`, and `foi`.'
    __slots__ = ()
    _schema_id = 317
    __match_args__ = ('tei_lerfu_word',)
    def __new__(cls, tei_lerfu_word: RecoveredField[TeiLerfuWordSyntax]) -> LetterTokensSyntaxTeiLerfuWord:
        return cls._from_fields((tei_lerfu_word,))
    def __init__(self, tei_lerfu_word: RecoveredField[TeiLerfuWordSyntax]) -> None:
        pass
    @property
    def tei_lerfu_word(self) -> RecoveredField[TeiLerfuWordSyntax]:
        'Uses the `tei_lerfu_word` product form, whose payload preserves `tei`, `letters`, and `foi`.'
        return cast(RecoveredField[TeiLerfuWordSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('LetterTokensSyntaxTeiLerfuWord is final')

LetterTokensSyntax: TypeAlias = LetterTokensSyntaxSimpleLerfuWord | LetterTokensSyntaxLauLerfuWord | LetterTokensSyntaxTeiLerfuWord

@final
class SimpleLerfuWordSyntax(_SyntaxNode):
    'Transparent product node for lerfu word; preserves the `word` component.'
    __slots__ = ()
    _schema_id = 318
    __match_args__ = ('word',)
    def __new__(cls, word: RecoveredField[Token]) -> SimpleLerfuWordSyntax:
        return cls._from_fields((word,))
    def __init__(self, word: RecoveredField[Token]) -> None:
        pass
    @property
    def word(self) -> RecoveredField[Token]:
        'The `word_category` grammar result in the `word` structural role of the `simple_lerfu_word` production.'
        return cast(RecoveredField[Token], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleLerfuWordSyntax is final')

@final
class LauLerfuWordSyntax(_SyntaxNode):
    'Product node for lerfu word; preserves `lau` and `letter` in source order.'
    __slots__ = ()
    _schema_id = 319
    __match_args__ = ('lau', 'letter')
    def __new__(cls, lau: RecoveredField[Token], letter: RecoveredField[LetterTokensSyntax]) -> LauLerfuWordSyntax:
        return cls._from_fields((lau, letter))
    def __init__(self, lau: RecoveredField[Token], letter: RecoveredField[LetterTokensSyntax]) -> None:
        pass
    @property
    def lau(self) -> RecoveredField[Token]:
        'A word from selmaho `Lau`.'
        return cast(RecoveredField[Token], self._field(0))
    @property
    def letter(self) -> RecoveredField[LetterTokensSyntax]:
        'The shared letter child syntax node.'
        return cast(RecoveredField[LetterTokensSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('LauLerfuWordSyntax is final')

@final
class TeiLerfuWordSyntax(_SyntaxNode):
    'Product node for lerfu word; preserves `tei`, `letters`, and `foi` in source order.'
    __slots__ = ()
    _schema_id = 320
    __match_args__ = ('tei', 'letters', 'foi')
    def __new__(cls, tei: RecoveredField[Token], letters: RecoveredField[LetterStringSyntax], foi: RecoveredField[Token]) -> TeiLerfuWordSyntax:
        return cls._from_fields((tei, letters, foi))
    def __init__(self, tei: RecoveredField[Token], letters: RecoveredField[LetterStringSyntax], foi: RecoveredField[Token]) -> None:
        pass
    @property
    def tei(self) -> RecoveredField[Token]:
        'The `Tei` cmavo marker.'
        return cast(RecoveredField[Token], self._field(0))
    @property
    def letters(self) -> RecoveredField[LetterStringSyntax]:
        'The shared letters child syntax node.'
        return cast(RecoveredField[LetterStringSyntax], self._field(1))
    @property
    def foi(self) -> RecoveredField[Token]:
        'The `Foi` cmavo marker.'
        return cast(RecoveredField[Token], self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('TeiLerfuWordSyntax is final')

@final
class LerfuStringMeksoSyntax(_SyntaxNode):
    'Product node for lerfu string; preserves `letters`, `boi`, and `free_modifiers` in source order.'
    __slots__ = ()
    _schema_id = 321
    __match_args__ = ('letters', 'boi', 'free_modifiers')
    def __new__(cls, letters: RecoveredField[LetterStringSyntax], boi: RecoveredField[Token] | None, free_modifiers: Sequence[RecoveredField[FreeModifierSyntax]]) -> LerfuStringMeksoSyntax:
        return cls._from_fields((letters, boi, free_modifiers))
    def __init__(self, letters: RecoveredField[LetterStringSyntax], boi: RecoveredField[Token] | None, free_modifiers: Sequence[RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def letters(self) -> RecoveredField[LetterStringSyntax]:
        'The `letter_string` grammar result in the `letters` structural role of the `lerfu_string_mekso` production.'
        return cast(RecoveredField[LetterStringSyntax], self._field(0))
    @property
    def boi(self) -> RecoveredField[Token] | None:
        'The optional `Boi` cmavo marker.'
        return cast(RecoveredField[Token] | None, self._field(1))
    @property
    def free_modifiers(self) -> tuple[RecoveredField[FreeModifierSyntax], ...]:
        'Ordered sequence of zero or more free modifiers components.'
        return cast(tuple[RecoveredField[FreeModifierSyntax], ...], self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('LerfuStringMeksoSyntax is final')

@final
class MeksoBaseSyntaxZantufaBoGroupedMeksoBase(_SyntaxNode):
    'Uses the `zantufa_bo_grouped_mekso_base` product form, whose payload preserves `first` and `continuations`.'
    __slots__ = ()
    _schema_id = 322
    __match_args__ = ('zantufa_bo_grouped_mekso_base',)
    def __new__(cls, zantufa_bo_grouped_mekso_base: RecoveredField[ZantufaBoGroupedMeksoBaseSyntax]) -> MeksoBaseSyntaxZantufaBoGroupedMeksoBase:
        return cls._from_fields((zantufa_bo_grouped_mekso_base,))
    def __init__(self, zantufa_bo_grouped_mekso_base: RecoveredField[ZantufaBoGroupedMeksoBaseSyntax]) -> None:
        pass
    @property
    def zantufa_bo_grouped_mekso_base(self) -> RecoveredField[ZantufaBoGroupedMeksoBaseSyntax]:
        'Uses the `zantufa_bo_grouped_mekso_base` product form, whose payload preserves `first` and `continuations`.'
        return cast(RecoveredField[ZantufaBoGroupedMeksoBaseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('MeksoBaseSyntaxZantufaBoGroupedMeksoBase is final')

@final
class MeksoBaseSyntaxMeksoOperand(_SyntaxNode):
    'Uses the nested `mekso_operand` sum form and preserves its selected alternative.'
    __slots__ = ()
    _schema_id = 323
    __match_args__ = ('mekso_operand',)
    def __new__(cls, mekso_operand: RecoveredField[MeksoOperandSyntax]) -> MeksoBaseSyntaxMeksoOperand:
        return cls._from_fields((mekso_operand,))
    def __init__(self, mekso_operand: RecoveredField[MeksoOperandSyntax]) -> None:
        pass
    @property
    def mekso_operand(self) -> RecoveredField[MeksoOperandSyntax]:
        'Uses the nested `mekso_operand` sum form and preserves its selected alternative.'
        return cast(RecoveredField[MeksoOperandSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('MeksoBaseSyntaxMeksoOperand is final')

@final
class MeksoBaseSyntaxForethoughtCallMekso(_SyntaxNode):
    'Uses the `forethought_call_mekso` product form, whose payload preserves `peho`, `operator`, `operands`, and `kuhe`.'
    __slots__ = ()
    _schema_id = 324
    __match_args__ = ('forethought_call_mekso',)
    def __new__(cls, forethought_call_mekso: RecoveredField[ForethoughtCallMeksoSyntax]) -> MeksoBaseSyntaxForethoughtCallMekso:
        return cls._from_fields((forethought_call_mekso,))
    def __init__(self, forethought_call_mekso: RecoveredField[ForethoughtCallMeksoSyntax]) -> None:
        pass
    @property
    def forethought_call_mekso(self) -> RecoveredField[ForethoughtCallMeksoSyntax]:
        'Uses the `forethought_call_mekso` product form, whose payload preserves `peho`, `operator`, `operands`, and `kuhe`.'
        return cast(RecoveredField[ForethoughtCallMeksoSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('MeksoBaseSyntaxForethoughtCallMekso is final')

@final
class MeksoBaseSyntaxZantufaGroupedMeksoOperandSequence(_SyntaxNode):
    'Uses the `zantufa_grouped_mekso_operand_sequence` product form, whose payload preserves `ke`, `operands`, and `kehe`.'
    __slots__ = ()
    _schema_id = 325
    __match_args__ = ('zantufa_grouped_mekso_operand_sequence',)
    def __new__(cls, zantufa_grouped_mekso_operand_sequence: RecoveredField[ZantufaGroupedMeksoOperandSequenceSyntax]) -> MeksoBaseSyntaxZantufaGroupedMeksoOperandSequence:
        return cls._from_fields((zantufa_grouped_mekso_operand_sequence,))
    def __init__(self, zantufa_grouped_mekso_operand_sequence: RecoveredField[ZantufaGroupedMeksoOperandSequenceSyntax]) -> None:
        pass
    @property
    def zantufa_grouped_mekso_operand_sequence(self) -> RecoveredField[ZantufaGroupedMeksoOperandSequenceSyntax]:
        'Uses the `zantufa_grouped_mekso_operand_sequence` product form, whose payload preserves `ke`, `operands`, and `kehe`.'
        return cast(RecoveredField[ZantufaGroupedMeksoOperandSequenceSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('MeksoBaseSyntaxZantufaGroupedMeksoOperandSequence is final')

MeksoBaseSyntax: TypeAlias = MeksoBaseSyntaxZantufaBoGroupedMeksoBase | MeksoBaseSyntaxMeksoOperand | MeksoBaseSyntaxForethoughtCallMekso | MeksoBaseSyntaxZantufaGroupedMeksoOperandSequence

@final
class ZantufaBoGroupedMeksoBaseSyntax(_SyntaxNode):
    'Product node for grouped mex; preserves `first` and `continuations` in source order.'
    __slots__ = ()
    _schema_id = 326
    __match_args__ = ('first', 'continuations')
    def __new__(cls, first: RecoveredField[MeksoOperandSyntax], continuations: Sequence[RecoveredField[ZantufaBoGroupedMeksoContinuationSyntax]]) -> ZantufaBoGroupedMeksoBaseSyntax:
        return cls._from_fields((first, continuations))
    def __init__(self, first: RecoveredField[MeksoOperandSyntax], continuations: Sequence[RecoveredField[ZantufaBoGroupedMeksoContinuationSyntax]]) -> None:
        pass
    @property
    def first(self) -> RecoveredField[MeksoOperandSyntax]:
        'The shared first child syntax node.'
        return cast(RecoveredField[MeksoOperandSyntax], self._field(0))
    @property
    def continuations(self) -> tuple[RecoveredField[ZantufaBoGroupedMeksoContinuationSyntax], ...]:
        'Non-empty ordered sequence of continuations components.'
        return cast(tuple[RecoveredField[ZantufaBoGroupedMeksoContinuationSyntax], ...], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaBoGroupedMeksoBaseSyntax is final')

@final
class ZantufaBoGroupedMeksoContinuationSyntax(_SyntaxNode):
    'Product node for grouped mex; preserves `bo` and `expression` in source order.'
    __slots__ = ()
    _schema_id = 327
    __match_args__ = ('bo', 'expression')
    def __new__(cls, bo: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], expression: RecoveredField[MeksoOperandSyntax]) -> ZantufaBoGroupedMeksoContinuationSyntax:
        return cls._from_fields((bo, expression))
    def __init__(self, bo: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], expression: RecoveredField[MeksoOperandSyntax]) -> None:
        pass
    @property
    def bo(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Bo` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def expression(self) -> RecoveredField[MeksoOperandSyntax]:
        'The shared expression child syntax node.'
        return cast(RecoveredField[MeksoOperandSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaBoGroupedMeksoContinuationSyntax is final')

@final
class ZantufaGroupedMeksoOperandSequenceSyntax(_SyntaxNode):
    'Product node for grouped mex; preserves `ke`, `operands`, and `kehe` in source order.'
    __slots__ = ()
    _schema_id = 328
    __match_args__ = ('ke', 'operands', 'kehe')
    def __new__(cls, ke: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], operands: Sequence[RecoveredField[MeksoOperandSyntax]], kehe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> ZantufaGroupedMeksoOperandSequenceSyntax:
        return cls._from_fields((ke, operands, kehe))
    def __init__(self, ke: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], operands: Sequence[RecoveredField[MeksoOperandSyntax]], kehe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def ke(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Ke` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def operands(self) -> tuple[RecoveredField[MeksoOperandSyntax], ...]:
        'Non-empty ordered sequence of operands components.'
        return cast(tuple[RecoveredField[MeksoOperandSyntax], ...], self._field(1))
    @property
    def kehe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Kehe` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaGroupedMeksoOperandSequenceSyntax is final')

@final
class MeksoPrecedenceSyntax(_SyntaxNode):
    'Product node for mex; preserves `left_expression` and `tail` in source order.'
    __slots__ = ()
    _schema_id = 329
    __match_args__ = ('left_expression', 'tail')
    def __new__(cls, left_expression: RecoveredField[MeksoBaseSyntax], tail: RecoveredField[MeksoPrecedenceTailSyntax] | None) -> MeksoPrecedenceSyntax:
        return cls._from_fields((left_expression, tail))
    def __init__(self, left_expression: RecoveredField[MeksoBaseSyntax], tail: RecoveredField[MeksoPrecedenceTailSyntax] | None) -> None:
        pass
    @property
    def left_expression(self) -> RecoveredField[MeksoBaseSyntax]:
        'The shared left expression child syntax node.'
        return cast(RecoveredField[MeksoBaseSyntax], self._field(0))
    @property
    def tail(self) -> RecoveredField[MeksoPrecedenceTailSyntax] | None:
        'The optional tail component.'
        return cast(RecoveredField[MeksoPrecedenceTailSyntax] | None, self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('MeksoPrecedenceSyntax is final')

@final
class MeksoPrecedenceTailSyntax(_SyntaxNode):
    'Product node for mex precedence tail; preserves `bihe`, `operator`, and `right_expression` in source order.'
    __slots__ = ()
    _schema_id = 330
    __match_args__ = ('bihe', 'operator', 'right_expression')
    def __new__(cls, bihe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], operator: RecoveredField[MeksoOperatorSyntax], right_expression: RecoveredField[MeksoPrecedenceSyntax]) -> MeksoPrecedenceTailSyntax:
        return cls._from_fields((bihe, operator, right_expression))
    def __init__(self, bihe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], operator: RecoveredField[MeksoOperatorSyntax], right_expression: RecoveredField[MeksoPrecedenceSyntax]) -> None:
        pass
    @property
    def bihe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Bihe` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def operator(self) -> RecoveredField[MeksoOperatorSyntax]:
        'The shared operator child syntax node.'
        return cast(RecoveredField[MeksoOperatorSyntax], self._field(1))
    @property
    def right_expression(self) -> RecoveredField[MeksoPrecedenceSyntax]:
        'The shared right expression child syntax node.'
        return cast(RecoveredField[MeksoPrecedenceSyntax], self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('MeksoPrecedenceTailSyntax is final')

@final
class InfixMeksoSyntax(_SyntaxNode):
    'Product node for mex; preserves `first_expression` and `continuations` in source order.'
    __slots__ = ()
    _schema_id = 331
    __match_args__ = ('first_expression', 'continuations')
    def __new__(cls, first_expression: RecoveredField[MeksoPrecedenceSyntax], continuations: Sequence[RecoveredField[InfixMeksoContinuationSyntax]]) -> InfixMeksoSyntax:
        return cls._from_fields((first_expression, continuations))
    def __init__(self, first_expression: RecoveredField[MeksoPrecedenceSyntax], continuations: Sequence[RecoveredField[InfixMeksoContinuationSyntax]]) -> None:
        pass
    @property
    def first_expression(self) -> RecoveredField[MeksoPrecedenceSyntax]:
        'The shared first expression child syntax node.'
        return cast(RecoveredField[MeksoPrecedenceSyntax], self._field(0))
    @property
    def continuations(self) -> tuple[RecoveredField[InfixMeksoContinuationSyntax], ...]:
        'Ordered sequence of zero or more continuations components.'
        return cast(tuple[RecoveredField[InfixMeksoContinuationSyntax], ...], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('InfixMeksoSyntax is final')

@final
class InfixMeksoContinuationSyntax(_SyntaxNode):
    'Product node for mex continuation; preserves `operator` and `right_expression` in source order.'
    __slots__ = ()
    _schema_id = 332
    __match_args__ = ('operator', 'right_expression')
    def __new__(cls, operator: RecoveredField[MeksoOperatorSyntax], right_expression: RecoveredField[MeksoPrecedenceSyntax]) -> InfixMeksoContinuationSyntax:
        return cls._from_fields((operator, right_expression))
    def __init__(self, operator: RecoveredField[MeksoOperatorSyntax], right_expression: RecoveredField[MeksoPrecedenceSyntax]) -> None:
        pass
    @property
    def operator(self) -> RecoveredField[MeksoOperatorSyntax]:
        'The shared operator child syntax node.'
        return cast(RecoveredField[MeksoOperatorSyntax], self._field(0))
    @property
    def right_expression(self) -> RecoveredField[MeksoPrecedenceSyntax]:
        'The shared right expression child syntax node.'
        return cast(RecoveredField[MeksoPrecedenceSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('InfixMeksoContinuationSyntax is final')

@final
class ZantufaInfixMeksoSyntax(_SyntaxNode):
    'Product node for mex; preserves `first_expression` and `continuations` in source order.'
    __slots__ = ()
    _schema_id = 333
    __match_args__ = ('first_expression', 'continuations')
    def __new__(cls, first_expression: RecoveredField[MeksoPrecedenceSyntax], continuations: Sequence[RecoveredField[ZantufaInfixMeksoContinuationSyntax]]) -> ZantufaInfixMeksoSyntax:
        return cls._from_fields((first_expression, continuations))
    def __init__(self, first_expression: RecoveredField[MeksoPrecedenceSyntax], continuations: Sequence[RecoveredField[ZantufaInfixMeksoContinuationSyntax]]) -> None:
        pass
    @property
    def first_expression(self) -> RecoveredField[MeksoPrecedenceSyntax]:
        'The shared first expression child syntax node.'
        return cast(RecoveredField[MeksoPrecedenceSyntax], self._field(0))
    @property
    def continuations(self) -> tuple[RecoveredField[ZantufaInfixMeksoContinuationSyntax], ...]:
        'Ordered sequence of zero or more continuations components.'
        return cast(tuple[RecoveredField[ZantufaInfixMeksoContinuationSyntax], ...], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaInfixMeksoSyntax is final')

@final
class ZantufaInfixMeksoContinuationSyntax(_SyntaxNode):
    'Product node for mex continuation; preserves `operators` and `right_expression` in source order.'
    __slots__ = ()
    _schema_id = 334
    __match_args__ = ('operators', 'right_expression')
    def __new__(cls, operators: Sequence[RecoveredField[MeksoOperatorSyntax]], right_expression: RecoveredField[MeksoPrecedenceSyntax] | None) -> ZantufaInfixMeksoContinuationSyntax:
        return cls._from_fields((operators, right_expression))
    def __init__(self, operators: Sequence[RecoveredField[MeksoOperatorSyntax]], right_expression: RecoveredField[MeksoPrecedenceSyntax] | None) -> None:
        pass
    @property
    def operators(self) -> tuple[RecoveredField[MeksoOperatorSyntax], ...]:
        'Non-empty ordered sequence of operators components.'
        return cast(tuple[RecoveredField[MeksoOperatorSyntax], ...], self._field(0))
    @property
    def right_expression(self) -> RecoveredField[MeksoPrecedenceSyntax] | None:
        'The optional right expression component.'
        return cast(RecoveredField[MeksoPrecedenceSyntax] | None, self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaInfixMeksoContinuationSyntax is final')

@final
class ForethoughtCallMeksoSyntax(_SyntaxNode):
    'Product node for forethought mex; preserves `peho`, `operator`, `operands`, and `kuhe` in source order.'
    __slots__ = ()
    _schema_id = 335
    __match_args__ = ('peho', 'operator', 'operands', 'kuhe')
    def __new__(cls, peho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, operator: RecoveredField[MeksoOperatorSyntax], operands: Sequence[RecoveredField[MeksoBaseSyntax]], kuhe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> ForethoughtCallMeksoSyntax:
        return cls._from_fields((peho, operator, operands, kuhe))
    def __init__(self, peho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, operator: RecoveredField[MeksoOperatorSyntax], operands: Sequence[RecoveredField[MeksoBaseSyntax]], kuhe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def peho(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Peho` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(0))
    @property
    def operator(self) -> RecoveredField[MeksoOperatorSyntax]:
        'The shared operator child syntax node.'
        return cast(RecoveredField[MeksoOperatorSyntax], self._field(1))
    @property
    def operands(self) -> tuple[RecoveredField[MeksoBaseSyntax], ...]:
        'Non-empty ordered sequence of operands components.'
        return cast(tuple[RecoveredField[MeksoBaseSyntax], ...], self._field(2))
    @property
    def kuhe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Kuhe` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('ForethoughtCallMeksoSyntax is final')

@final
class MeksoSyntaxZantufaReversePolishMekso(_SyntaxNode):
    'Uses the `zantufa_reverse_polish_mekso` product form, whose payload preserves `fuha`, `operands`, `operator`, `tails`, and `kuhe`.'
    __slots__ = ()
    _schema_id = 336
    __match_args__ = ('zantufa_reverse_polish_mekso',)
    def __new__(cls, zantufa_reverse_polish_mekso: RecoveredField[ZantufaReversePolishMeksoSyntax]) -> MeksoSyntaxZantufaReversePolishMekso:
        return cls._from_fields((zantufa_reverse_polish_mekso,))
    def __init__(self, zantufa_reverse_polish_mekso: RecoveredField[ZantufaReversePolishMeksoSyntax]) -> None:
        pass
    @property
    def zantufa_reverse_polish_mekso(self) -> RecoveredField[ZantufaReversePolishMeksoSyntax]:
        'Uses the `zantufa_reverse_polish_mekso` product form, whose payload preserves `fuha`, `operands`, `operator`, `tails`, and `kuhe`.'
        return cast(RecoveredField[ZantufaReversePolishMeksoSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('MeksoSyntaxZantufaReversePolishMekso is final')

@final
class MeksoSyntaxZantufaInfixMekso(_SyntaxNode):
    'Uses the `zantufa_infix_mekso` product form, whose payload preserves `first_expression` and `continuations`.'
    __slots__ = ()
    _schema_id = 337
    __match_args__ = ('zantufa_infix_mekso',)
    def __new__(cls, zantufa_infix_mekso: RecoveredField[ZantufaInfixMeksoSyntax]) -> MeksoSyntaxZantufaInfixMekso:
        return cls._from_fields((zantufa_infix_mekso,))
    def __init__(self, zantufa_infix_mekso: RecoveredField[ZantufaInfixMeksoSyntax]) -> None:
        pass
    @property
    def zantufa_infix_mekso(self) -> RecoveredField[ZantufaInfixMeksoSyntax]:
        'Uses the `zantufa_infix_mekso` product form, whose payload preserves `first_expression` and `continuations`.'
        return cast(RecoveredField[ZantufaInfixMeksoSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('MeksoSyntaxZantufaInfixMekso is final')

@final
class MeksoSyntaxInfixMekso(_SyntaxNode):
    'Uses the `infix_mekso` product form, whose payload preserves `first_expression` and `continuations`.'
    __slots__ = ()
    _schema_id = 338
    __match_args__ = ('infix_mekso',)
    def __new__(cls, infix_mekso: RecoveredField[InfixMeksoSyntax]) -> MeksoSyntaxInfixMekso:
        return cls._from_fields((infix_mekso,))
    def __init__(self, infix_mekso: RecoveredField[InfixMeksoSyntax]) -> None:
        pass
    @property
    def infix_mekso(self) -> RecoveredField[InfixMeksoSyntax]:
        'Uses the `infix_mekso` product form, whose payload preserves `first_expression` and `continuations`.'
        return cast(RecoveredField[InfixMeksoSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('MeksoSyntaxInfixMekso is final')

@final
class MeksoSyntaxReversePolishMekso(_SyntaxNode):
    'Uses the `reverse_polish_mekso` product form, whose payload preserves `fuha` and `parts`.'
    __slots__ = ()
    _schema_id = 339
    __match_args__ = ('reverse_polish_mekso',)
    def __new__(cls, reverse_polish_mekso: RecoveredField[ReversePolishMeksoSyntax]) -> MeksoSyntaxReversePolishMekso:
        return cls._from_fields((reverse_polish_mekso,))
    def __init__(self, reverse_polish_mekso: RecoveredField[ReversePolishMeksoSyntax]) -> None:
        pass
    @property
    def reverse_polish_mekso(self) -> RecoveredField[ReversePolishMeksoSyntax]:
        'Uses the `reverse_polish_mekso` product form, whose payload preserves `fuha` and `parts`.'
        return cast(RecoveredField[ReversePolishMeksoSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('MeksoSyntaxReversePolishMekso is final')

MeksoSyntax: TypeAlias = MeksoSyntaxZantufaReversePolishMekso | MeksoSyntaxZantufaInfixMekso | MeksoSyntaxInfixMekso | MeksoSyntaxReversePolishMekso

@final
class ZantufaReversePolishMeksoSyntax(_SyntaxNode):
    'Product node for reverse Polish mex; preserves `fuha`, `operands`, `operator`, `tails`, and `kuhe` in source order.'
    __slots__ = ()
    _schema_id = 340
    __match_args__ = ('fuha', 'operands', 'operator', 'tails', 'kuhe')
    def __new__(cls, fuha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], operands: Sequence[RecoveredField[MeksoBaseSyntax]], operator: RecoveredField[MeksoOperatorSyntax], tails: Sequence[RecoveredField[ZantufaReversePolishTailSyntax]], kuhe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> ZantufaReversePolishMeksoSyntax:
        return cls._from_fields((fuha, operands, operator, tails, kuhe))
    def __init__(self, fuha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], operands: Sequence[RecoveredField[MeksoBaseSyntax]], operator: RecoveredField[MeksoOperatorSyntax], tails: Sequence[RecoveredField[ZantufaReversePolishTailSyntax]], kuhe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def fuha(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Fuha` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def operands(self) -> tuple[RecoveredField[MeksoBaseSyntax], ...]:
        'Non-empty ordered sequence of operands components.'
        return cast(tuple[RecoveredField[MeksoBaseSyntax], ...], self._field(1))
    @property
    def operator(self) -> RecoveredField[MeksoOperatorSyntax]:
        'The shared operator child syntax node.'
        return cast(RecoveredField[MeksoOperatorSyntax], self._field(2))
    @property
    def tails(self) -> tuple[RecoveredField[ZantufaReversePolishTailSyntax], ...]:
        'Ordered sequence of zero or more tails components.'
        return cast(tuple[RecoveredField[ZantufaReversePolishTailSyntax], ...], self._field(3))
    @property
    def kuhe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Kuhe` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(4))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaReversePolishMeksoSyntax is final')

@final
class ZantufaReversePolishTailSyntax(_SyntaxNode):
    'Product node for reverse Polish mex tail; preserves `operands` and `operator` in source order.'
    __slots__ = ()
    _schema_id = 341
    __match_args__ = ('operands', 'operator')
    def __new__(cls, operands: Sequence[RecoveredField[MeksoBaseSyntax]], operator: RecoveredField[MeksoOperatorSyntax]) -> ZantufaReversePolishTailSyntax:
        return cls._from_fields((operands, operator))
    def __init__(self, operands: Sequence[RecoveredField[MeksoBaseSyntax]], operator: RecoveredField[MeksoOperatorSyntax]) -> None:
        pass
    @property
    def operands(self) -> tuple[RecoveredField[MeksoBaseSyntax], ...]:
        'Ordered sequence of zero or more operands components.'
        return cast(tuple[RecoveredField[MeksoBaseSyntax], ...], self._field(0))
    @property
    def operator(self) -> RecoveredField[MeksoOperatorSyntax]:
        'The shared operator child syntax node.'
        return cast(RecoveredField[MeksoOperatorSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaReversePolishTailSyntax is final')

@final
class ReversePolishPartsSyntax(_SyntaxNode):
    'Product node for reverse Polish mex; preserves `first_operand` and `tails` in source order.'
    __slots__ = ()
    _schema_id = 342
    __match_args__ = ('first_operand', 'tails')
    def __new__(cls, first_operand: RecoveredField[MeksoOperandSyntax], tails: Sequence[RecoveredField[ReversePolishPartsTailSyntax]]) -> ReversePolishPartsSyntax:
        return cls._from_fields((first_operand, tails))
    def __init__(self, first_operand: RecoveredField[MeksoOperandSyntax], tails: Sequence[RecoveredField[ReversePolishPartsTailSyntax]]) -> None:
        pass
    @property
    def first_operand(self) -> RecoveredField[MeksoOperandSyntax]:
        'The shared first operand child syntax node.'
        return cast(RecoveredField[MeksoOperandSyntax], self._field(0))
    @property
    def tails(self) -> tuple[RecoveredField[ReversePolishPartsTailSyntax], ...]:
        'Ordered sequence of zero or more tails components.'
        return cast(tuple[RecoveredField[ReversePolishPartsTailSyntax], ...], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ReversePolishPartsSyntax is final')

@final
class ReversePolishPartsTailSyntax(_SyntaxNode):
    'Product node for reverse Polish mex tail; preserves `right_parts` and `operator` in source order.'
    __slots__ = ()
    _schema_id = 343
    __match_args__ = ('right_parts', 'operator')
    def __new__(cls, right_parts: RecoveredField[ReversePolishPartsSyntax], operator: RecoveredField[MeksoOperatorSyntax]) -> ReversePolishPartsTailSyntax:
        return cls._from_fields((right_parts, operator))
    def __init__(self, right_parts: RecoveredField[ReversePolishPartsSyntax], operator: RecoveredField[MeksoOperatorSyntax]) -> None:
        pass
    @property
    def right_parts(self) -> RecoveredField[ReversePolishPartsSyntax]:
        'The shared right parts child syntax node.'
        return cast(RecoveredField[ReversePolishPartsSyntax], self._field(0))
    @property
    def operator(self) -> RecoveredField[MeksoOperatorSyntax]:
        'The `mekso_operator` grammar result in the `operator` structural role of the `reverse_polish_parts_tail` production.'
        return cast(RecoveredField[MeksoOperatorSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ReversePolishPartsTailSyntax is final')

@final
class ReversePolishMeksoSyntax(_SyntaxNode):
    'Product node for reverse Polish mex; preserves `fuha` and `parts` in source order.'
    __slots__ = ()
    _schema_id = 344
    __match_args__ = ('fuha', 'parts')
    def __new__(cls, fuha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], parts: RecoveredField[ReversePolishPartsSyntax]) -> ReversePolishMeksoSyntax:
        return cls._from_fields((fuha, parts))
    def __init__(self, fuha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], parts: RecoveredField[ReversePolishPartsSyntax]) -> None:
        pass
    @property
    def fuha(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Fuha` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def parts(self) -> RecoveredField[ReversePolishPartsSyntax]:
        'The shared parts child syntax node.'
        return cast(RecoveredField[ReversePolishPartsSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ReversePolishMeksoSyntax is final')

@final
class NumberSumtiSyntax(_SyntaxNode):
    'Product node for number sumti; preserves `li`, `expression`, and `loho` in source order.'
    __slots__ = ()
    _schema_id = 345
    __match_args__ = ('li', 'expression', 'loho')
    def __new__(cls, li: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], expression: RecoveredField[MeksoSyntax], loho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> NumberSumtiSyntax:
        return cls._from_fields((li, expression, loho))
    def __init__(self, li: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], expression: RecoveredField[MeksoSyntax], loho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def li(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Li`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def expression(self) -> RecoveredField[MeksoSyntax]:
        'The shared expression child syntax node.'
        return cast(RecoveredField[MeksoSyntax], self._field(1))
    @property
    def loho(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Loho` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('NumberSumtiSyntax is final')

@final
class LerfuStringSumtiSyntax(_SyntaxNode):
    'Product node for lerfu string; preserves `words`, `boi`, and `free_modifiers` in source order.'
    __slots__ = ()
    _schema_id = 346
    __match_args__ = ('words', 'boi', 'free_modifiers')
    def __new__(cls, words: RecoveredField[LetterStringSyntax], boi: RecoveredField[Token] | None, free_modifiers: Sequence[RecoveredField[FreeModifierSyntax]]) -> LerfuStringSumtiSyntax:
        return cls._from_fields((words, boi, free_modifiers))
    def __init__(self, words: RecoveredField[LetterStringSyntax], boi: RecoveredField[Token] | None, free_modifiers: Sequence[RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def words(self) -> RecoveredField[LetterStringSyntax]:
        'The `letter_string` grammar result in the `words` structural role of the `lerfu_string_sumti` production.'
        return cast(RecoveredField[LetterStringSyntax], self._field(0))
    @property
    def boi(self) -> RecoveredField[Token] | None:
        'The optional `Boi` cmavo marker.'
        return cast(RecoveredField[Token] | None, self._field(1))
    @property
    def free_modifiers(self) -> tuple[RecoveredField[FreeModifierSyntax], ...]:
        'Ordered sequence of zero or more free modifiers components.'
        return cast(tuple[RecoveredField[FreeModifierSyntax], ...], self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('LerfuStringSumtiSyntax is final')

@final
class LaheSumtiSyntax(_SyntaxNode):
    'Product node for converted sumti; preserves `lahe`, `relative_clauses`, `inner_sumti`, and `luhu` in source order.'
    __slots__ = ()
    _schema_id = 347
    __match_args__ = ('lahe', 'relative_clauses', 'inner_sumti', 'luhu')
    def __new__(cls, lahe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], relative_clauses: RecoveredField[RelativeClauseListSyntax] | None, inner_sumti: RecoveredField[SumtiSyntax], luhu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> LaheSumtiSyntax:
        return cls._from_fields((lahe, relative_clauses, inner_sumti, luhu))
    def __init__(self, lahe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], relative_clauses: RecoveredField[RelativeClauseListSyntax] | None, inner_sumti: RecoveredField[SumtiSyntax], luhu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def lahe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Lahe`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def relative_clauses(self) -> RecoveredField[RelativeClauseListSyntax] | None:
        'The optional relative clauses component.'
        return cast(RecoveredField[RelativeClauseListSyntax] | None, self._field(1))
    @property
    def inner_sumti(self) -> RecoveredField[SumtiSyntax]:
        'The shared inner sumti child syntax node.'
        return cast(RecoveredField[SumtiSyntax], self._field(2))
    @property
    def luhu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Luhu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('LaheSumtiSyntax is final')

@final
class LaheTermWrapperSyntax(_SyntaxNode):
    'Product node for converted term; preserves `lahe`, `inner_term`, and `luhu` in source order.'
    __slots__ = ()
    _schema_id = 348
    __match_args__ = ('lahe', 'inner_term', 'luhu')
    def __new__(cls, lahe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_term: RecoveredField[TermSyntax], luhu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> LaheTermWrapperSyntax:
        return cls._from_fields((lahe, inner_term, luhu))
    def __init__(self, lahe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_term: RecoveredField[TermSyntax], luhu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def lahe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Lahe`.\n\nWrapping a bare term (rather than a sumti) in `LAhE` is a non-CLL extension:\nstandard grammar only allows `LAhE` over a sumti, so the term-wrapper form warns.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def inner_term(self) -> RecoveredField[TermSyntax]:
        'The shared inner term child syntax node.'
        return cast(RecoveredField[TermSyntax], self._field(1))
    @property
    def luhu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Luhu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('LaheTermWrapperSyntax is final')

@final
class ScalarNegatedTermWrapperWithBoSyntax(_SyntaxNode):
    'Product node for scalar-negated term; preserves `nahe`, `bo`, `inner_term`, and `luhu` in source order.'
    __slots__ = ()
    _schema_id = 349
    __match_args__ = ('nahe', 'bo', 'inner_term', 'luhu')
    def __new__(cls, nahe: RecoveredField[Token], bo: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_term: RecoveredField[TermSyntax], luhu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> ScalarNegatedTermWrapperWithBoSyntax:
        return cls._from_fields((nahe, bo, inner_term, luhu))
    def __init__(self, nahe: RecoveredField[Token], bo: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_term: RecoveredField[TermSyntax], luhu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def nahe(self) -> RecoveredField[Token]:
        'A word from selmaho `Nahe`.\n\n`NAhE BO` wrapping a bare term (rather than a sumti) is a non-CLL extension:\neven with `bo`, the standard grammar only allows `NAhE BO` over a sumti, so the\nterm-wrapper form warns. The warning anchors on `na\'e` to match the v0 behavior.'
        return cast(RecoveredField[Token], self._field(0))
    @property
    def bo(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Bo` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(1))
    @property
    def inner_term(self) -> RecoveredField[TermSyntax]:
        'The shared inner term child syntax node.'
        return cast(RecoveredField[TermSyntax], self._field(2))
    @property
    def luhu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Luhu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('ScalarNegatedTermWrapperWithBoSyntax is final')

@final
class ScalarNegatedTermWrapperSyntax(_SyntaxNode):
    'Product node for scalar-negated term; preserves `nahe`, `inner_term`, and `luhu` in source order.'
    __slots__ = ()
    _schema_id = 350
    __match_args__ = ('nahe', 'inner_term', 'luhu')
    def __new__(cls, nahe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_term: RecoveredField[TermSyntax], luhu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> ScalarNegatedTermWrapperSyntax:
        return cls._from_fields((nahe, inner_term, luhu))
    def __init__(self, nahe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_term: RecoveredField[TermSyntax], luhu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def nahe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Nahe`.\n\nBare `na\'e` wrapping a term (rather than a sumti) without `bo` is a non-CLL\nextension. Following v0, this carries only the term-wrapper warning\n(`ExperimentalLaheNaheTermWrapper`), not the sumti-oriented without-`bo`\nwarning: the distinguishing property here is the term payload, not the missing `bo`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def inner_term(self) -> RecoveredField[TermSyntax]:
        'The shared inner term child syntax node.'
        return cast(RecoveredField[TermSyntax], self._field(1))
    @property
    def luhu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Luhu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('ScalarNegatedTermWrapperSyntax is final')

@final
class ScalarNegatedSumtiWithBoSyntax(_SyntaxNode):
    'Product node for scalar-negated sumti; preserves `nahe`, `bo`, `inner_sumti`, and `luhu` in source order.'
    __slots__ = ()
    _schema_id = 351
    __match_args__ = ('nahe', 'bo', 'inner_sumti', 'luhu')
    def __new__(cls, nahe: RecoveredField[Token], bo: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_sumti: RecoveredField[SumtiSyntax], luhu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> ScalarNegatedSumtiWithBoSyntax:
        return cls._from_fields((nahe, bo, inner_sumti, luhu))
    def __init__(self, nahe: RecoveredField[Token], bo: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_sumti: RecoveredField[SumtiSyntax], luhu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def nahe(self) -> RecoveredField[Token]:
        'A word from selmaho `Nahe`.'
        return cast(RecoveredField[Token], self._field(0))
    @property
    def bo(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Bo` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(1))
    @property
    def inner_sumti(self) -> RecoveredField[SumtiSyntax]:
        'The shared inner sumti child syntax node.'
        return cast(RecoveredField[SumtiSyntax], self._field(2))
    @property
    def luhu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Luhu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('ScalarNegatedSumtiWithBoSyntax is final')

@final
class ScalarNegatedSumtiSyntax(_SyntaxNode):
    'Product node for scalar-negated sumti; preserves `nahe`, `inner_sumti`, and `luhu` in source order.'
    __slots__ = ()
    _schema_id = 352
    __match_args__ = ('nahe', 'inner_sumti', 'luhu')
    def __new__(cls, nahe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_sumti: RecoveredField[SumtiSyntax], luhu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> ScalarNegatedSumtiSyntax:
        return cls._from_fields((nahe, inner_sumti, luhu))
    def __init__(self, nahe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_sumti: RecoveredField[SumtiSyntax], luhu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def nahe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Nahe`.\n\nBare `na\'e` before a sumti without `bo` is a non-CLL extension (standard\n`sumti-6` permits only `NAhE BO` before a sumti), so it warns; the `bo`-ful\nsibling `scalar_negated_sumti_with_bo` is standard grammar and does not warn.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def inner_sumti(self) -> RecoveredField[SumtiSyntax]:
        'The shared inner sumti child syntax node.'
        return cast(RecoveredField[SumtiSyntax], self._field(1))
    @property
    def luhu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Luhu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('ScalarNegatedSumtiSyntax is final')

@final
class BridiDescriptionSumtiSyntax(_SyntaxNode):
    'Product node for bridi description; preserves `lohoi`, `additional_heads`, `statement`, and `kuhau` in source order.'
    __slots__ = ()
    _schema_id = 353
    __match_args__ = ('lohoi', 'additional_heads', 'statement', 'kuhau')
    def __new__(cls, lohoi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], additional_heads: Sequence[RecoveredField[LohoiDescriptionHeadContinuationSyntax]], statement: RecoveredField[StatementSyntax], kuhau: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> BridiDescriptionSumtiSyntax:
        return cls._from_fields((lohoi, additional_heads, statement, kuhau))
    def __init__(self, lohoi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], additional_heads: Sequence[RecoveredField[LohoiDescriptionHeadContinuationSyntax]], statement: RecoveredField[StatementSyntax], kuhau: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def lohoi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Lohoi`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def additional_heads(self) -> tuple[RecoveredField[LohoiDescriptionHeadContinuationSyntax], ...]:
        'Ordered sequence of zero or more additional heads components.'
        return cast(tuple[RecoveredField[LohoiDescriptionHeadContinuationSyntax], ...], self._field(1))
    @property
    def statement(self) -> RecoveredField[StatementSyntax]:
        'The shared statement child syntax node.'
        return cast(RecoveredField[StatementSyntax], self._field(2))
    @property
    def kuhau(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Kuhau` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('BridiDescriptionSumtiSyntax is final')

@final
class LohoiDescriptionHeadContinuationSyntax(_SyntaxNode):
    'Product node for bridi description; preserves `connective` and `lohoi` in source order.'
    __slots__ = ()
    _schema_id = 354
    __match_args__ = ('connective', 'lohoi')
    def __new__(cls, connective: RecoveredField[JoikConnectiveSyntax], lohoi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> LohoiDescriptionHeadContinuationSyntax:
        return cls._from_fields((connective, lohoi))
    def __init__(self, connective: RecoveredField[JoikConnectiveSyntax], lohoi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def connective(self) -> RecoveredField[JoikConnectiveSyntax]:
        'The `joik_connective` connective joining the adjacent constituents of the `lohoi_description_head_continuation` production.'
        return cast(RecoveredField[JoikConnectiveSyntax], self._field(0))
    @property
    def lohoi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Lohoi`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('LohoiDescriptionHeadContinuationSyntax is final')

@final
class ProSumtiSyntax(_SyntaxNode):
    'Transparent product node for sumti; preserves the `koha` component.'
    __slots__ = ()
    _schema_id = 355
    __match_args__ = ('koha',)
    def __new__(cls, koha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> ProSumtiSyntax:
        return cls._from_fields((koha,))
    def __init__(self, koha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def koha(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `word_category` grammar result in the `koha` structural role of the `pro_sumti` production.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ProSumtiSyntax is final')

@final
class NameSumtiSyntax(_SyntaxNode):
    'Product node for name; preserves `la`, `relative_clauses`, and `names` in source order.'
    __slots__ = ()
    _schema_id = 356
    __match_args__ = ('la', 'relative_clauses', 'names')
    def __new__(cls, la: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], relative_clauses: RecoveredField[RelativeClauseListSyntax] | None, names: WithFreeModifiers[Sequence[RecoveredField[Token]], RecoveredField[FreeModifierSyntax]]) -> NameSumtiSyntax:
        return cls._from_fields((la, relative_clauses, names))
    def __init__(self, la: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], relative_clauses: RecoveredField[RelativeClauseListSyntax] | None, names: WithFreeModifiers[Sequence[RecoveredField[Token]], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def la(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `La`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def relative_clauses(self) -> RecoveredField[RelativeClauseListSyntax] | None:
        'The optional relative clauses component.'
        return cast(RecoveredField[RelativeClauseListSyntax] | None, self._field(1))
    @property
    def names(self) -> WithFreeModifiers[tuple[RecoveredField[Token], ...], RecoveredField[FreeModifierSyntax]]:
        'Non-empty ordered sequence of names components.'
        return cast(WithFreeModifiers[tuple[RecoveredField[Token], ...], RecoveredField[FreeModifierSyntax]], self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('NameSumtiSyntax is final')

@final
class DescriptionHeadSyntax(_SyntaxNode):
    'Transparent product node for descriptor; preserves the `description` component.'
    __slots__ = ()
    _schema_id = 357
    __match_args__ = ('description',)
    def __new__(cls, description: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> DescriptionHeadSyntax:
        return cls._from_fields((description,))
    def __init__(self, description: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def description(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The required description-head word from either selmaho `Le` or selmaho `La`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('DescriptionHeadSyntax is final')

@final
class DescriptionHeadConnectiveSyntax(_SyntaxNode):
    'Transparent product node for descriptor connective; preserves the `connective` component.'
    __slots__ = ()
    _schema_id = 358
    __match_args__ = ('connective',)
    def __new__(cls, connective: RecoveredField[JekConnectiveSyntax]) -> DescriptionHeadConnectiveSyntax:
        return cls._from_fields((connective,))
    def __init__(self, connective: RecoveredField[JekConnectiveSyntax]) -> None:
        pass
    @property
    def connective(self) -> RecoveredField[JekConnectiveSyntax]:
        'The shared connective child syntax node.'
        return cast(RecoveredField[JekConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('DescriptionHeadConnectiveSyntax is final')

@final
class DescriptionConnectionSumtiSyntax(_SyntaxNode):
    'Product node for description; preserves `leading_description_head`, `connective`, `trailing_description_head`, `tail`, and `ku` in source order.'
    __slots__ = ()
    _schema_id = 359
    __match_args__ = ('leading_description_head', 'connective', 'trailing_description_head', 'tail', 'ku')
    def __new__(cls, leading_description_head: RecoveredField[DescriptionHeadSyntax], connective: RecoveredField[DescriptionHeadConnectiveSyntax], trailing_description_head: RecoveredField[DescriptionHeadSyntax], tail: RecoveredField[DescriptionTailSyntax], ku: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> DescriptionConnectionSumtiSyntax:
        return cls._from_fields((leading_description_head, connective, trailing_description_head, tail, ku))
    def __init__(self, leading_description_head: RecoveredField[DescriptionHeadSyntax], connective: RecoveredField[DescriptionHeadConnectiveSyntax], trailing_description_head: RecoveredField[DescriptionHeadSyntax], tail: RecoveredField[DescriptionTailSyntax], ku: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def leading_description_head(self) -> RecoveredField[DescriptionHeadSyntax]:
        'The shared leading description head child syntax node.'
        return cast(RecoveredField[DescriptionHeadSyntax], self._field(0))
    @property
    def connective(self) -> RecoveredField[DescriptionHeadConnectiveSyntax]:
        'The `description_head_connective` connective joining the adjacent constituents of the `description_connection_sumti` production.'
        return cast(RecoveredField[DescriptionHeadConnectiveSyntax], self._field(1))
    @property
    def trailing_description_head(self) -> RecoveredField[DescriptionHeadSyntax]:
        'The shared trailing description head child syntax node.'
        return cast(RecoveredField[DescriptionHeadSyntax], self._field(2))
    @property
    def tail(self) -> RecoveredField[DescriptionTailSyntax]:
        'The `description_tail` grammar result in the `tail` structural role of the `description_connection_sumti` production.'
        return cast(RecoveredField[DescriptionTailSyntax], self._field(3))
    @property
    def ku(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Ku` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(4))
    def __init_subclass__(cls) -> None:
        raise TypeError('DescriptionConnectionSumtiSyntax is final')

@final
class DescriptorWithGadriSumtiSyntax(_SyntaxNode):
    'Product node for description; preserves `description`, `tail`, and `ku` in source order.'
    __slots__ = ()
    _schema_id = 360
    __match_args__ = ('description', 'tail', 'ku')
    def __new__(cls, description: RecoveredField[DescriptionHeadSyntax], tail: RecoveredField[DescriptionTailSyntax], ku: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> DescriptorWithGadriSumtiSyntax:
        return cls._from_fields((description, tail, ku))
    def __init__(self, description: RecoveredField[DescriptionHeadSyntax], tail: RecoveredField[DescriptionTailSyntax], ku: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def description(self) -> RecoveredField[DescriptionHeadSyntax]:
        'The `description_head` grammar result in the `description` structural role of the `descriptor_with_gadri_sumti` production.'
        return cast(RecoveredField[DescriptionHeadSyntax], self._field(0))
    @property
    def tail(self) -> RecoveredField[DescriptionTailSyntax]:
        'The `description_tail` grammar result in the `tail` structural role of the `descriptor_with_gadri_sumti` production.'
        return cast(RecoveredField[DescriptionTailSyntax], self._field(1))
    @property
    def ku(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Ku` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('DescriptorWithGadriSumtiSyntax is final')

@final
class DescriptorWithOuterQuantifierSumtiSyntax(_SyntaxNode):
    'Product node for description; preserves `outer_quantifier`, `description`, `tail`, and `ku` in source order.'
    __slots__ = ()
    _schema_id = 361
    __match_args__ = ('outer_quantifier', 'description', 'tail', 'ku')
    def __new__(cls, outer_quantifier: RecoveredField[QuantifierSyntax], description: RecoveredField[DescriptionHeadSyntax], tail: RecoveredField[DescriptionTailSyntax], ku: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> DescriptorWithOuterQuantifierSumtiSyntax:
        return cls._from_fields((outer_quantifier, description, tail, ku))
    def __init__(self, outer_quantifier: RecoveredField[QuantifierSyntax], description: RecoveredField[DescriptionHeadSyntax], tail: RecoveredField[DescriptionTailSyntax], ku: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def outer_quantifier(self) -> RecoveredField[QuantifierSyntax]:
        'The `quantifier` grammar result in the `outer_quantifier` structural role of the `descriptor_with_outer_quantifier_sumti` production.'
        return cast(RecoveredField[QuantifierSyntax], self._field(0))
    @property
    def description(self) -> RecoveredField[DescriptionHeadSyntax]:
        'The `description_head` grammar result in the `description` structural role of the `descriptor_with_outer_quantifier_sumti` production.'
        return cast(RecoveredField[DescriptionHeadSyntax], self._field(1))
    @property
    def tail(self) -> RecoveredField[DescriptionTailSyntax]:
        'The `description_tail` grammar result in the `tail` structural role of the `descriptor_with_outer_quantifier_sumti` production.'
        return cast(RecoveredField[DescriptionTailSyntax], self._field(2))
    @property
    def ku(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Ku` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('DescriptorWithOuterQuantifierSumtiSyntax is final')

@final
class DescriptorWithoutGadriSumtiSyntax(_SyntaxNode):
    'Product node for description; preserves `quantifier`, `selbri`, `ku`, and `relative_clauses` in source order.'
    __slots__ = ()
    _schema_id = 362
    __match_args__ = ('quantifier', 'selbri', 'ku', 'relative_clauses')
    def __new__(cls, quantifier: RecoveredField[QuantifierSyntax], selbri: RecoveredField[SelbriSyntax], ku: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, relative_clauses: RecoveredField[RelativeClauseListSyntax] | None) -> DescriptorWithoutGadriSumtiSyntax:
        return cls._from_fields((quantifier, selbri, ku, relative_clauses))
    def __init__(self, quantifier: RecoveredField[QuantifierSyntax], selbri: RecoveredField[SelbriSyntax], ku: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, relative_clauses: RecoveredField[RelativeClauseListSyntax] | None) -> None:
        pass
    @property
    def quantifier(self) -> RecoveredField[QuantifierSyntax]:
        'The `quantifier` grammar result in the `quantifier` structural role of the `descriptor_without_gadri_sumti` production.'
        return cast(RecoveredField[QuantifierSyntax], self._field(0))
    @property
    def selbri(self) -> RecoveredField[SelbriSyntax]:
        'The shared selbri child syntax node.'
        return cast(RecoveredField[SelbriSyntax], self._field(1))
    @property
    def ku(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Ku` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    @property
    def relative_clauses(self) -> RecoveredField[RelativeClauseListSyntax] | None:
        'The optional relative clauses component.'
        return cast(RecoveredField[RelativeClauseListSyntax] | None, self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('DescriptorWithoutGadriSumtiSyntax is final')

@final
class DescriptionTailSyntax(_SyntaxNode):
    'Product node for description tail; preserves `leading_tail_elements` and `tail` in source order.'
    __slots__ = ()
    _schema_id = 363
    __match_args__ = ('leading_tail_elements', 'tail')
    def __new__(cls, leading_tail_elements: RecoveredField[LeadingDescriptionTailElementsSyntax], tail: RecoveredField[DescriptionTailBodySyntax]) -> DescriptionTailSyntax:
        return cls._from_fields((leading_tail_elements, tail))
    def __init__(self, leading_tail_elements: RecoveredField[LeadingDescriptionTailElementsSyntax], tail: RecoveredField[DescriptionTailBodySyntax]) -> None:
        pass
    @property
    def leading_tail_elements(self) -> RecoveredField[LeadingDescriptionTailElementsSyntax]:
        'The `leading_description_tail_elements` grammar result in the `leading_tail_elements` structural role of the `description_tail` production.'
        return cast(RecoveredField[LeadingDescriptionTailElementsSyntax], self._field(0))
    @property
    def tail(self) -> RecoveredField[DescriptionTailBodySyntax]:
        'The shared tail child syntax node.'
        return cast(RecoveredField[DescriptionTailBodySyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('DescriptionTailSyntax is final')

@final
class DescriptionTailBodySyntaxQuantifierRelationDescriptionTail(_SyntaxNode):
    'Uses the `quantifier_relation_description_tail` product form, whose payload preserves `quantifier`, `selbri`, and `relative_clauses`.'
    __slots__ = ()
    _schema_id = 364
    __match_args__ = ('quantifier_relation_description_tail',)
    def __new__(cls, quantifier_relation_description_tail: RecoveredField[QuantifierRelationDescriptionTailSyntax]) -> DescriptionTailBodySyntaxQuantifierRelationDescriptionTail:
        return cls._from_fields((quantifier_relation_description_tail,))
    def __init__(self, quantifier_relation_description_tail: RecoveredField[QuantifierRelationDescriptionTailSyntax]) -> None:
        pass
    @property
    def quantifier_relation_description_tail(self) -> RecoveredField[QuantifierRelationDescriptionTailSyntax]:
        'Uses the `quantifier_relation_description_tail` product form, whose payload preserves `quantifier`, `selbri`, and `relative_clauses`.'
        return cast(RecoveredField[QuantifierRelationDescriptionTailSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('DescriptionTailBodySyntaxQuantifierRelationDescriptionTail is final')

@final
class DescriptionTailBodySyntaxQuantifierSumtiDescriptionTail(_SyntaxNode):
    'Uses the `quantifier_sumti_description_tail` product form, whose payload preserves `quantifier` and `sumti`.'
    __slots__ = ()
    _schema_id = 365
    __match_args__ = ('quantifier_sumti_description_tail',)
    def __new__(cls, quantifier_sumti_description_tail: RecoveredField[QuantifierSumtiDescriptionTailSyntax]) -> DescriptionTailBodySyntaxQuantifierSumtiDescriptionTail:
        return cls._from_fields((quantifier_sumti_description_tail,))
    def __init__(self, quantifier_sumti_description_tail: RecoveredField[QuantifierSumtiDescriptionTailSyntax]) -> None:
        pass
    @property
    def quantifier_sumti_description_tail(self) -> RecoveredField[QuantifierSumtiDescriptionTailSyntax]:
        'Uses the `quantifier_sumti_description_tail` product form, whose payload preserves `quantifier` and `sumti`.'
        return cast(RecoveredField[QuantifierSumtiDescriptionTailSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('DescriptionTailBodySyntaxQuantifierSumtiDescriptionTail is final')

@final
class DescriptionTailBodySyntaxRelationDescriptionTail(_SyntaxNode):
    'Uses the `relation_description_tail` product form, whose payload preserves `selbri` and `relative_clauses`.'
    __slots__ = ()
    _schema_id = 366
    __match_args__ = ('relation_description_tail',)
    def __new__(cls, relation_description_tail: RecoveredField[RelationDescriptionTailSyntax]) -> DescriptionTailBodySyntaxRelationDescriptionTail:
        return cls._from_fields((relation_description_tail,))
    def __init__(self, relation_description_tail: RecoveredField[RelationDescriptionTailSyntax]) -> None:
        pass
    @property
    def relation_description_tail(self) -> RecoveredField[RelationDescriptionTailSyntax]:
        'Uses the `relation_description_tail` product form, whose payload preserves `selbri` and `relative_clauses`.'
        return cast(RecoveredField[RelationDescriptionTailSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('DescriptionTailBodySyntaxRelationDescriptionTail is final')

DescriptionTailBodySyntax: TypeAlias = DescriptionTailBodySyntaxQuantifierRelationDescriptionTail | DescriptionTailBodySyntaxQuantifierSumtiDescriptionTail | DescriptionTailBodySyntaxRelationDescriptionTail

@final
class LeadingDescriptionTailElementsSyntax(_SyntaxNode):
    'Product node for description tail; preserves `tail_sumti` and `relative_clauses` in source order.'
    __slots__ = ()
    _schema_id = 367
    __match_args__ = ('tail_sumti', 'relative_clauses')
    def __new__(cls, tail_sumti: RecoveredField[DescriptionTailSumtiSyntax] | None, relative_clauses: RecoveredField[RelativeClauseListSyntax] | None) -> LeadingDescriptionTailElementsSyntax:
        return cls._from_fields((tail_sumti, relative_clauses))
    def __init__(self, tail_sumti: RecoveredField[DescriptionTailSumtiSyntax] | None, relative_clauses: RecoveredField[RelativeClauseListSyntax] | None) -> None:
        pass
    @property
    def tail_sumti(self) -> RecoveredField[DescriptionTailSumtiSyntax] | None:
        'The optional tail sumti component.'
        return cast(RecoveredField[DescriptionTailSumtiSyntax] | None, self._field(0))
    @property
    def relative_clauses(self) -> RecoveredField[RelativeClauseListSyntax] | None:
        'The optional relative clauses component.'
        return cast(RecoveredField[RelativeClauseListSyntax] | None, self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('LeadingDescriptionTailElementsSyntax is final')

@final
class DescriptionTailSumtiSyntax(_SyntaxNode):
    'Transparent product node for description tail; preserves the `sumti` component.'
    __slots__ = ()
    _schema_id = 368
    __match_args__ = ('sumti',)
    def __new__(cls, sumti: RecoveredField[SumtiBaseSyntax]) -> DescriptionTailSumtiSyntax:
        return cls._from_fields((sumti,))
    def __init__(self, sumti: RecoveredField[SumtiBaseSyntax]) -> None:
        pass
    @property
    def sumti(self) -> RecoveredField[SumtiBaseSyntax]:
        'The shared sumti child syntax node.'
        return cast(RecoveredField[SumtiBaseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('DescriptionTailSumtiSyntax is final')

@final
class RelationDescriptionTailSyntax(_SyntaxNode):
    'Product node for description tail; preserves `selbri` and `relative_clauses` in source order.'
    __slots__ = ()
    _schema_id = 369
    __match_args__ = ('selbri', 'relative_clauses')
    def __new__(cls, selbri: RecoveredField[SelbriSyntax], relative_clauses: RecoveredField[RelativeClauseListSyntax] | None) -> RelationDescriptionTailSyntax:
        return cls._from_fields((selbri, relative_clauses))
    def __init__(self, selbri: RecoveredField[SelbriSyntax], relative_clauses: RecoveredField[RelativeClauseListSyntax] | None) -> None:
        pass
    @property
    def selbri(self) -> RecoveredField[SelbriSyntax]:
        'The shared selbri child syntax node.'
        return cast(RecoveredField[SelbriSyntax], self._field(0))
    @property
    def relative_clauses(self) -> RecoveredField[RelativeClauseListSyntax] | None:
        'The optional relative clauses component.'
        return cast(RecoveredField[RelativeClauseListSyntax] | None, self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('RelationDescriptionTailSyntax is final')

@final
class QuantifierRelationDescriptionTailSyntax(_SyntaxNode):
    'Product node for description tail; preserves `quantifier`, `selbri`, and `relative_clauses` in source order.'
    __slots__ = ()
    _schema_id = 370
    __match_args__ = ('quantifier', 'selbri', 'relative_clauses')
    def __new__(cls, quantifier: RecoveredField[QuantifierSyntax], selbri: RecoveredField[SelbriSyntax], relative_clauses: RecoveredField[RelativeClauseListSyntax] | None) -> QuantifierRelationDescriptionTailSyntax:
        return cls._from_fields((quantifier, selbri, relative_clauses))
    def __init__(self, quantifier: RecoveredField[QuantifierSyntax], selbri: RecoveredField[SelbriSyntax], relative_clauses: RecoveredField[RelativeClauseListSyntax] | None) -> None:
        pass
    @property
    def quantifier(self) -> RecoveredField[QuantifierSyntax]:
        'The `quantifier` grammar result in the `quantifier` structural role of the `quantifier_relation_description_tail` production.'
        return cast(RecoveredField[QuantifierSyntax], self._field(0))
    @property
    def selbri(self) -> RecoveredField[SelbriSyntax]:
        'The shared selbri child syntax node.'
        return cast(RecoveredField[SelbriSyntax], self._field(1))
    @property
    def relative_clauses(self) -> RecoveredField[RelativeClauseListSyntax] | None:
        'The optional relative clauses component.'
        return cast(RecoveredField[RelativeClauseListSyntax] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('QuantifierRelationDescriptionTailSyntax is final')

@final
class QuantifierSumtiDescriptionTailSyntax(_SyntaxNode):
    'Product node for description tail; preserves `quantifier` and `sumti` in source order.'
    __slots__ = ()
    _schema_id = 371
    __match_args__ = ('quantifier', 'sumti')
    def __new__(cls, quantifier: RecoveredField[QuantifierSyntax], sumti: RecoveredField[SumtiSyntax]) -> QuantifierSumtiDescriptionTailSyntax:
        return cls._from_fields((quantifier, sumti))
    def __init__(self, quantifier: RecoveredField[QuantifierSyntax], sumti: RecoveredField[SumtiSyntax]) -> None:
        pass
    @property
    def quantifier(self) -> RecoveredField[QuantifierSyntax]:
        'The `quantifier` grammar result in the `quantifier` structural role of the `quantifier_sumti_description_tail` production.'
        return cast(RecoveredField[QuantifierSyntax], self._field(0))
    @property
    def sumti(self) -> RecoveredField[SumtiSyntax]:
        'The shared sumti child syntax node.'
        return cast(RecoveredField[SumtiSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('QuantifierSumtiDescriptionTailSyntax is final')

@final
class QuoteSyntaxExperimentalMehoiCompoundQuote(_SyntaxNode):
    'Uses the `experimental_mehoi_compound_quote` product form, whose payload preserves `quote`.'
    __slots__ = ()
    _schema_id = 372
    __match_args__ = ('experimental_mehoi_compound_quote',)
    def __new__(cls, experimental_mehoi_compound_quote: RecoveredField[ExperimentalMehoiCompoundQuoteSyntax]) -> QuoteSyntaxExperimentalMehoiCompoundQuote:
        return cls._from_fields((experimental_mehoi_compound_quote,))
    def __init__(self, experimental_mehoi_compound_quote: RecoveredField[ExperimentalMehoiCompoundQuoteSyntax]) -> None:
        pass
    @property
    def experimental_mehoi_compound_quote(self) -> RecoveredField[ExperimentalMehoiCompoundQuoteSyntax]:
        'Uses the `experimental_mehoi_compound_quote` product form, whose payload preserves `quote`.'
        return cast(RecoveredField[ExperimentalMehoiCompoundQuoteSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('QuoteSyntaxExperimentalMehoiCompoundQuote is final')

@final
class QuoteSyntaxExperimentalZohoiCompoundQuote(_SyntaxNode):
    'Uses the `experimental_zohoi_compound_quote` product form, whose payload preserves `quote`.'
    __slots__ = ()
    _schema_id = 373
    __match_args__ = ('experimental_zohoi_compound_quote',)
    def __new__(cls, experimental_zohoi_compound_quote: RecoveredField[ExperimentalZohoiCompoundQuoteSyntax]) -> QuoteSyntaxExperimentalZohoiCompoundQuote:
        return cls._from_fields((experimental_zohoi_compound_quote,))
    def __init__(self, experimental_zohoi_compound_quote: RecoveredField[ExperimentalZohoiCompoundQuoteSyntax]) -> None:
        pass
    @property
    def experimental_zohoi_compound_quote(self) -> RecoveredField[ExperimentalZohoiCompoundQuoteSyntax]:
        'Uses the `experimental_zohoi_compound_quote` product form, whose payload preserves `quote`.'
        return cast(RecoveredField[ExperimentalZohoiCompoundQuoteSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('QuoteSyntaxExperimentalZohoiCompoundQuote is final')

@final
class QuoteSyntaxExperimentalRahoiCompoundQuote(_SyntaxNode):
    'Uses the `experimental_rahoi_compound_quote` product form, whose payload preserves `quote`.'
    __slots__ = ()
    _schema_id = 374
    __match_args__ = ('experimental_rahoi_compound_quote',)
    def __new__(cls, experimental_rahoi_compound_quote: RecoveredField[ExperimentalRahoiCompoundQuoteSyntax]) -> QuoteSyntaxExperimentalRahoiCompoundQuote:
        return cls._from_fields((experimental_rahoi_compound_quote,))
    def __init__(self, experimental_rahoi_compound_quote: RecoveredField[ExperimentalRahoiCompoundQuoteSyntax]) -> None:
        pass
    @property
    def experimental_rahoi_compound_quote(self) -> RecoveredField[ExperimentalRahoiCompoundQuoteSyntax]:
        'Uses the `experimental_rahoi_compound_quote` product form, whose payload preserves `quote`.'
        return cast(RecoveredField[ExperimentalRahoiCompoundQuoteSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('QuoteSyntaxExperimentalRahoiCompoundQuote is final')

@final
class QuoteSyntaxExperimentalGohoiCompoundQuote(_SyntaxNode):
    'Uses the `experimental_gohoi_compound_quote` product form, whose payload preserves `quote`.'
    __slots__ = ()
    _schema_id = 375
    __match_args__ = ('experimental_gohoi_compound_quote',)
    def __new__(cls, experimental_gohoi_compound_quote: RecoveredField[ExperimentalGohoiCompoundQuoteSyntax]) -> QuoteSyntaxExperimentalGohoiCompoundQuote:
        return cls._from_fields((experimental_gohoi_compound_quote,))
    def __init__(self, experimental_gohoi_compound_quote: RecoveredField[ExperimentalGohoiCompoundQuoteSyntax]) -> None:
        pass
    @property
    def experimental_gohoi_compound_quote(self) -> RecoveredField[ExperimentalGohoiCompoundQuoteSyntax]:
        'Uses the `experimental_gohoi_compound_quote` product form, whose payload preserves `quote`.'
        return cast(RecoveredField[ExperimentalGohoiCompoundQuoteSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('QuoteSyntaxExperimentalGohoiCompoundQuote is final')

@final
class QuoteSyntaxGenericCompoundQuote(_SyntaxNode):
    'Uses the `generic_compound_quote` product form, whose payload preserves `quote`.'
    __slots__ = ()
    _schema_id = 376
    __match_args__ = ('generic_compound_quote',)
    def __new__(cls, generic_compound_quote: RecoveredField[GenericCompoundQuoteSyntax]) -> QuoteSyntaxGenericCompoundQuote:
        return cls._from_fields((generic_compound_quote,))
    def __init__(self, generic_compound_quote: RecoveredField[GenericCompoundQuoteSyntax]) -> None:
        pass
    @property
    def generic_compound_quote(self) -> RecoveredField[GenericCompoundQuoteSyntax]:
        'Uses the `generic_compound_quote` product form, whose payload preserves `quote`.'
        return cast(RecoveredField[GenericCompoundQuoteSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('QuoteSyntaxGenericCompoundQuote is final')

@final
class QuoteSyntaxTextQuote(_SyntaxNode):
    'Uses the `text_quote` product form, whose payload preserves `lu`, `text`, and `lihu`.'
    __slots__ = ()
    _schema_id = 377
    __match_args__ = ('text_quote',)
    def __new__(cls, text_quote: RecoveredField[TextQuoteSyntax]) -> QuoteSyntaxTextQuote:
        return cls._from_fields((text_quote,))
    def __init__(self, text_quote: RecoveredField[TextQuoteSyntax]) -> None:
        pass
    @property
    def text_quote(self) -> RecoveredField[TextQuoteSyntax]:
        'Uses the `text_quote` product form, whose payload preserves `lu`, `text`, and `lihu`.'
        return cast(RecoveredField[TextQuoteSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('QuoteSyntaxTextQuote is final')

QuoteSyntax: TypeAlias = QuoteSyntaxExperimentalMehoiCompoundQuote | QuoteSyntaxExperimentalZohoiCompoundQuote | QuoteSyntaxExperimentalRahoiCompoundQuote | QuoteSyntaxExperimentalGohoiCompoundQuote | QuoteSyntaxGenericCompoundQuote | QuoteSyntaxTextQuote

@final
class TextQuoteSyntax(_SyntaxNode):
    'Product node for text quote; preserves `lu`, `text`, and `lihu` in source order.'
    __slots__ = ()
    _schema_id = 378
    __match_args__ = ('lu', 'text', 'lihu')
    def __new__(cls, lu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], text: RecoveredField[TextSyntax], lihu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> TextQuoteSyntax:
        return cls._from_fields((lu, text, lihu))
    def __init__(self, lu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], text: RecoveredField[TextSyntax], lihu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def lu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Lu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def text(self) -> RecoveredField[TextSyntax]:
        'The shared text child syntax node.'
        return cast(RecoveredField[TextSyntax], self._field(1))
    @property
    def lihu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Lihu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('TextQuoteSyntax is final')

@final
class ExperimentalMehoiCompoundQuoteSyntax(_SyntaxNode):
    'Transparent product node for quote; preserves the `quote` component.'
    __slots__ = ()
    _schema_id = 379
    __match_args__ = ('quote',)
    def __new__(cls, quote: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> ExperimentalMehoiCompoundQuoteSyntax:
        return cls._from_fields((quote,))
    def __init__(self, quote: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def quote(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `quote_marker` grammar result in the `quote` structural role of the `experimental_mehoi_compound_quote` production.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ExperimentalMehoiCompoundQuoteSyntax is final')

@final
class ExperimentalZohoiCompoundQuoteSyntax(_SyntaxNode):
    'Transparent product node for quote; preserves the `quote` component.'
    __slots__ = ()
    _schema_id = 380
    __match_args__ = ('quote',)
    def __new__(cls, quote: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> ExperimentalZohoiCompoundQuoteSyntax:
        return cls._from_fields((quote,))
    def __init__(self, quote: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def quote(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The selected grammar alternative in the `quote` structural role of the `experimental_zohoi_compound_quote` production.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ExperimentalZohoiCompoundQuoteSyntax is final')

@final
class ExperimentalRahoiCompoundQuoteSyntax(_SyntaxNode):
    'Transparent product node for quote; preserves the `quote` component.'
    __slots__ = ()
    _schema_id = 381
    __match_args__ = ('quote',)
    def __new__(cls, quote: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> ExperimentalRahoiCompoundQuoteSyntax:
        return cls._from_fields((quote,))
    def __init__(self, quote: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def quote(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `quote_marker` grammar result in the `quote` structural role of the `experimental_rahoi_compound_quote` production.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ExperimentalRahoiCompoundQuoteSyntax is final')

@final
class ExperimentalGohoiCompoundQuoteSyntax(_SyntaxNode):
    'Transparent product node for quote; preserves the `quote` component.'
    __slots__ = ()
    _schema_id = 382
    __match_args__ = ('quote',)
    def __new__(cls, quote: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> ExperimentalGohoiCompoundQuoteSyntax:
        return cls._from_fields((quote,))
    def __init__(self, quote: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def quote(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The selected grammar alternative in the `quote` structural role of the `experimental_gohoi_compound_quote` production.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ExperimentalGohoiCompoundQuoteSyntax is final')

@final
class GenericCompoundQuoteSyntax(_SyntaxNode):
    'Transparent product node for quote; preserves the `quote` component.'
    __slots__ = ()
    _schema_id = 383
    __match_args__ = ('quote',)
    def __new__(cls, quote: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> GenericCompoundQuoteSyntax:
        return cls._from_fields((quote,))
    def __init__(self, quote: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def quote(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `word_category` grammar result in the `quote` structural role of the `generic_compound_quote` production.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('GenericCompoundQuoteSyntax is final')

@final
class QuotedSumtiSyntax(_SyntaxNode):
    'Transparent product node for quote; preserves the `quote` component.'
    __slots__ = ()
    _schema_id = 384
    __match_args__ = ('quote',)
    def __new__(cls, quote: RecoveredField[QuoteSyntax]) -> QuotedSumtiSyntax:
        return cls._from_fields((quote,))
    def __init__(self, quote: RecoveredField[QuoteSyntax]) -> None:
        pass
    @property
    def quote(self) -> RecoveredField[QuoteSyntax]:
        'The shared quote child syntax node.'
        return cast(RecoveredField[QuoteSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('QuotedSumtiSyntax is final')

@final
class SelbriVocativeSumtiSyntax(_SyntaxNode):
    'Product node for vocative phrase; preserves `leading_relative_clauses`, `selbri`, and `trailing_relative_clauses` in source order.'
    __slots__ = ()
    _schema_id = 385
    __match_args__ = ('leading_relative_clauses', 'selbri', 'trailing_relative_clauses')
    def __new__(cls, leading_relative_clauses: RecoveredField[RelativeClauseListSyntax] | None, selbri: RecoveredField[SelbriSyntax], trailing_relative_clauses: RecoveredField[RelativeClauseListSyntax] | None) -> SelbriVocativeSumtiSyntax:
        return cls._from_fields((leading_relative_clauses, selbri, trailing_relative_clauses))
    def __init__(self, leading_relative_clauses: RecoveredField[RelativeClauseListSyntax] | None, selbri: RecoveredField[SelbriSyntax], trailing_relative_clauses: RecoveredField[RelativeClauseListSyntax] | None) -> None:
        pass
    @property
    def leading_relative_clauses(self) -> RecoveredField[RelativeClauseListSyntax] | None:
        'The optional leading relative clauses component.'
        return cast(RecoveredField[RelativeClauseListSyntax] | None, self._field(0))
    @property
    def selbri(self) -> RecoveredField[SelbriSyntax]:
        'The shared selbri child syntax node.'
        return cast(RecoveredField[SelbriSyntax], self._field(1))
    @property
    def trailing_relative_clauses(self) -> RecoveredField[RelativeClauseListSyntax] | None:
        'The optional trailing relative clauses component.'
        return cast(RecoveredField[RelativeClauseListSyntax] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('SelbriVocativeSumtiSyntax is final')

@final
class CmevlaVocativeSumtiSyntax(_SyntaxNode):
    'Product node for vocative phrase; preserves `leading_relative_clauses`, `names`, and `trailing_relative_clauses` in source order.'
    __slots__ = ()
    _schema_id = 386
    __match_args__ = ('leading_relative_clauses', 'names', 'trailing_relative_clauses')
    def __new__(cls, leading_relative_clauses: RecoveredField[RelativeClauseListSyntax] | None, names: WithFreeModifiers[Sequence[RecoveredField[Token]], RecoveredField[FreeModifierSyntax]], trailing_relative_clauses: RecoveredField[RelativeClauseListSyntax] | None) -> CmevlaVocativeSumtiSyntax:
        return cls._from_fields((leading_relative_clauses, names, trailing_relative_clauses))
    def __init__(self, leading_relative_clauses: RecoveredField[RelativeClauseListSyntax] | None, names: WithFreeModifiers[Sequence[RecoveredField[Token]], RecoveredField[FreeModifierSyntax]], trailing_relative_clauses: RecoveredField[RelativeClauseListSyntax] | None) -> None:
        pass
    @property
    def leading_relative_clauses(self) -> RecoveredField[RelativeClauseListSyntax] | None:
        'The optional leading relative clauses component.'
        return cast(RecoveredField[RelativeClauseListSyntax] | None, self._field(0))
    @property
    def names(self) -> WithFreeModifiers[tuple[RecoveredField[Token], ...], RecoveredField[FreeModifierSyntax]]:
        'Non-empty ordered sequence of names components.'
        return cast(WithFreeModifiers[tuple[RecoveredField[Token], ...], RecoveredField[FreeModifierSyntax]], self._field(1))
    @property
    def trailing_relative_clauses(self) -> RecoveredField[RelativeClauseListSyntax] | None:
        'The optional trailing relative clauses component.'
        return cast(RecoveredField[RelativeClauseListSyntax] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('CmevlaVocativeSumtiSyntax is final')

@final
class VocativeSumtiSyntaxSelbriVocativeSumti(_SyntaxNode):
    'Uses the `selbri_vocative_sumti` product form, whose payload preserves `leading_relative_clauses`, `selbri`, and `trailing_relative_clauses`.'
    __slots__ = ()
    _schema_id = 387
    __match_args__ = ('selbri_vocative_sumti',)
    def __new__(cls, selbri_vocative_sumti: RecoveredField[SelbriVocativeSumtiSyntax]) -> VocativeSumtiSyntaxSelbriVocativeSumti:
        return cls._from_fields((selbri_vocative_sumti,))
    def __init__(self, selbri_vocative_sumti: RecoveredField[SelbriVocativeSumtiSyntax]) -> None:
        pass
    @property
    def selbri_vocative_sumti(self) -> RecoveredField[SelbriVocativeSumtiSyntax]:
        'Uses the `selbri_vocative_sumti` product form, whose payload preserves `leading_relative_clauses`, `selbri`, and `trailing_relative_clauses`.'
        return cast(RecoveredField[SelbriVocativeSumtiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('VocativeSumtiSyntaxSelbriVocativeSumti is final')

@final
class VocativeSumtiSyntaxCmevlaVocativeSumti(_SyntaxNode):
    'Uses the `cmevla_vocative_sumti` product form, whose payload preserves `leading_relative_clauses`, `names`, and `trailing_relative_clauses`.'
    __slots__ = ()
    _schema_id = 388
    __match_args__ = ('cmevla_vocative_sumti',)
    def __new__(cls, cmevla_vocative_sumti: RecoveredField[CmevlaVocativeSumtiSyntax]) -> VocativeSumtiSyntaxCmevlaVocativeSumti:
        return cls._from_fields((cmevla_vocative_sumti,))
    def __init__(self, cmevla_vocative_sumti: RecoveredField[CmevlaVocativeSumtiSyntax]) -> None:
        pass
    @property
    def cmevla_vocative_sumti(self) -> RecoveredField[CmevlaVocativeSumtiSyntax]:
        'Uses the `cmevla_vocative_sumti` product form, whose payload preserves `leading_relative_clauses`, `names`, and `trailing_relative_clauses`.'
        return cast(RecoveredField[CmevlaVocativeSumtiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('VocativeSumtiSyntaxCmevlaVocativeSumti is final')

@final
class VocativeSumtiSyntaxSumti(_SyntaxNode):
    'Uses the `sumti` product form, whose payload preserves `base_sumti` and `vuho_attachment`.'
    __slots__ = ()
    _schema_id = 389
    __match_args__ = ('sumti',)
    def __new__(cls, sumti: RecoveredField[SumtiSyntax]) -> VocativeSumtiSyntaxSumti:
        return cls._from_fields((sumti,))
    def __init__(self, sumti: RecoveredField[SumtiSyntax]) -> None:
        pass
    @property
    def sumti(self) -> RecoveredField[SumtiSyntax]:
        'Uses the `sumti` product form, whose payload preserves `base_sumti` and `vuho_attachment`.'
        return cast(RecoveredField[SumtiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('VocativeSumtiSyntaxSumti is final')

VocativeSumtiSyntax: TypeAlias = VocativeSumtiSyntaxSelbriVocativeSumti | VocativeSumtiSyntaxCmevlaVocativeSumti | VocativeSumtiSyntaxSumti

@final
class VocativeMarkerWordsSyntaxCoiVocativeMarkerWords(_SyntaxNode):
    'Uses the `coi_vocative_marker_words` product form, whose payload preserves `first_coi`, `first_nai`, `additional_coi`, and `doi`.'
    __slots__ = ()
    _schema_id = 390
    __match_args__ = ('coi_vocative_marker_words',)
    def __new__(cls, coi_vocative_marker_words: RecoveredField[CoiVocativeMarkerWordsSyntax]) -> VocativeMarkerWordsSyntaxCoiVocativeMarkerWords:
        return cls._from_fields((coi_vocative_marker_words,))
    def __init__(self, coi_vocative_marker_words: RecoveredField[CoiVocativeMarkerWordsSyntax]) -> None:
        pass
    @property
    def coi_vocative_marker_words(self) -> RecoveredField[CoiVocativeMarkerWordsSyntax]:
        'Uses the `coi_vocative_marker_words` product form, whose payload preserves `first_coi`, `first_nai`, `additional_coi`, and `doi`.'
        return cast(RecoveredField[CoiVocativeMarkerWordsSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('VocativeMarkerWordsSyntaxCoiVocativeMarkerWords is final')

@final
class VocativeMarkerWordsSyntaxDoiVocativeMarkerWords(_SyntaxNode):
    'Uses the `doi_vocative_marker_words` product form, whose payload preserves `doi`.'
    __slots__ = ()
    _schema_id = 391
    __match_args__ = ('doi_vocative_marker_words',)
    def __new__(cls, doi_vocative_marker_words: RecoveredField[DoiVocativeMarkerWordsSyntax]) -> VocativeMarkerWordsSyntaxDoiVocativeMarkerWords:
        return cls._from_fields((doi_vocative_marker_words,))
    def __init__(self, doi_vocative_marker_words: RecoveredField[DoiVocativeMarkerWordsSyntax]) -> None:
        pass
    @property
    def doi_vocative_marker_words(self) -> RecoveredField[DoiVocativeMarkerWordsSyntax]:
        'Uses the `doi_vocative_marker_words` product form, whose payload preserves `doi`.'
        return cast(RecoveredField[DoiVocativeMarkerWordsSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('VocativeMarkerWordsSyntaxDoiVocativeMarkerWords is final')

VocativeMarkerWordsSyntax: TypeAlias = VocativeMarkerWordsSyntaxCoiVocativeMarkerWords | VocativeMarkerWordsSyntaxDoiVocativeMarkerWords

@final
class CoiVocativeMarkerWordsSyntax(_SyntaxNode):
    'Product node for vocative marker; preserves `first_coi`, `first_nai`, `additional_coi`, and `doi` in source order.'
    __slots__ = ()
    _schema_id = 392
    __match_args__ = ('first_coi', 'first_nai', 'additional_coi', 'doi')
    def __new__(cls, first_coi: RecoveredField[Token], first_nai: RecoveredField[Token] | None, additional_coi: Sequence[RecoveredField[AdditionalCoiVocativeMarkerSyntax]], doi: RecoveredField[Token] | None) -> CoiVocativeMarkerWordsSyntax:
        return cls._from_fields((first_coi, first_nai, additional_coi, doi))
    def __init__(self, first_coi: RecoveredField[Token], first_nai: RecoveredField[Token] | None, additional_coi: Sequence[RecoveredField[AdditionalCoiVocativeMarkerSyntax]], doi: RecoveredField[Token] | None) -> None:
        pass
    @property
    def first_coi(self) -> RecoveredField[Token]:
        'A word from selmaho `Coi`.'
        return cast(RecoveredField[Token], self._field(0))
    @property
    def first_nai(self) -> RecoveredField[Token] | None:
        'The optional `Nai` cmavo marker.'
        return cast(RecoveredField[Token] | None, self._field(1))
    @property
    def additional_coi(self) -> tuple[RecoveredField[AdditionalCoiVocativeMarkerSyntax], ...]:
        'Ordered sequence of zero or more additional coi components.'
        return cast(tuple[RecoveredField[AdditionalCoiVocativeMarkerSyntax], ...], self._field(2))
    @property
    def doi(self) -> RecoveredField[Token] | None:
        'The optional `Doi` cmavo marker.'
        return cast(RecoveredField[Token] | None, self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('CoiVocativeMarkerWordsSyntax is final')

@final
class AdditionalCoiVocativeMarkerSyntax(_SyntaxNode):
    'Product node for vocative marker; preserves `coi` and `nai` in source order.'
    __slots__ = ()
    _schema_id = 393
    __match_args__ = ('coi', 'nai')
    def __new__(cls, coi: RecoveredField[Token], nai: RecoveredField[Token] | None) -> AdditionalCoiVocativeMarkerSyntax:
        return cls._from_fields((coi, nai))
    def __init__(self, coi: RecoveredField[Token], nai: RecoveredField[Token] | None) -> None:
        pass
    @property
    def coi(self) -> RecoveredField[Token]:
        'A word from selmaho `Coi`.'
        return cast(RecoveredField[Token], self._field(0))
    @property
    def nai(self) -> RecoveredField[Token] | None:
        'The optional `Nai` cmavo marker.'
        return cast(RecoveredField[Token] | None, self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('AdditionalCoiVocativeMarkerSyntax is final')

@final
class DoiVocativeMarkerWordsSyntax(_SyntaxNode):
    'Transparent product node for vocative marker; preserves the `doi` component.'
    __slots__ = ()
    _schema_id = 394
    __match_args__ = ('doi',)
    def __new__(cls, doi: RecoveredField[Token]) -> DoiVocativeMarkerWordsSyntax:
        return cls._from_fields((doi,))
    def __init__(self, doi: RecoveredField[Token]) -> None:
        pass
    @property
    def doi(self) -> RecoveredField[Token]:
        'The `Doi` cmavo marker.'
        return cast(RecoveredField[Token], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('DoiVocativeMarkerWordsSyntax is final')

@final
class FreeModifierSyntaxTextReplacementFreeModifier(_SyntaxNode):
    'Uses the nested `text_replacement_free_modifier` sum form and preserves its selected alternative.'
    __slots__ = ()
    _schema_id = 395
    __match_args__ = ('text_replacement_free_modifier',)
    def __new__(cls, text_replacement_free_modifier: RecoveredField[TextReplacementFreeModifierSyntax]) -> FreeModifierSyntaxTextReplacementFreeModifier:
        return cls._from_fields((text_replacement_free_modifier,))
    def __init__(self, text_replacement_free_modifier: RecoveredField[TextReplacementFreeModifierSyntax]) -> None:
        pass
    @property
    def text_replacement_free_modifier(self) -> RecoveredField[TextReplacementFreeModifierSyntax]:
        'Uses the nested `text_replacement_free_modifier` sum form and preserves its selected alternative.'
        return cast(RecoveredField[TextReplacementFreeModifierSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('FreeModifierSyntaxTextReplacementFreeModifier is final')

@final
class FreeModifierSyntaxZantufaSeiStatementFreeModifier(_SyntaxNode):
    'Uses the `zantufa_sei_statement_free_modifier` product form, whose payload preserves `sei`, `statement`, and `sehu`.'
    __slots__ = ()
    _schema_id = 396
    __match_args__ = ('zantufa_sei_statement_free_modifier',)
    def __new__(cls, zantufa_sei_statement_free_modifier: RecoveredField[ZantufaSeiStatementFreeModifierSyntax]) -> FreeModifierSyntaxZantufaSeiStatementFreeModifier:
        return cls._from_fields((zantufa_sei_statement_free_modifier,))
    def __init__(self, zantufa_sei_statement_free_modifier: RecoveredField[ZantufaSeiStatementFreeModifierSyntax]) -> None:
        pass
    @property
    def zantufa_sei_statement_free_modifier(self) -> RecoveredField[ZantufaSeiStatementFreeModifierSyntax]:
        'Uses the `zantufa_sei_statement_free_modifier` product form, whose payload preserves `sei`, `statement`, and `sehu`.'
        return cast(RecoveredField[ZantufaSeiStatementFreeModifierSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('FreeModifierSyntaxZantufaSeiStatementFreeModifier is final')

@final
class FreeModifierSyntaxSeiFreeModifier(_SyntaxNode):
    'Uses the `sei_free_modifier` product form, whose payload preserves `sei`, `terms`, `cu`, `selbri`, and `sehu`.'
    __slots__ = ()
    _schema_id = 397
    __match_args__ = ('sei_free_modifier',)
    def __new__(cls, sei_free_modifier: RecoveredField[SeiFreeModifierSyntax]) -> FreeModifierSyntaxSeiFreeModifier:
        return cls._from_fields((sei_free_modifier,))
    def __init__(self, sei_free_modifier: RecoveredField[SeiFreeModifierSyntax]) -> None:
        pass
    @property
    def sei_free_modifier(self) -> RecoveredField[SeiFreeModifierSyntax]:
        'Uses the `sei_free_modifier` product form, whose payload preserves `sei`, `terms`, `cu`, `selbri`, and `sehu`.'
        return cast(RecoveredField[SeiFreeModifierSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('FreeModifierSyntaxSeiFreeModifier is final')

@final
class FreeModifierSyntaxXiFreeModifier(_SyntaxNode):
    'Uses the nested `xi_free_modifier` sum form and preserves its selected alternative.'
    __slots__ = ()
    _schema_id = 398
    __match_args__ = ('xi_free_modifier',)
    def __new__(cls, xi_free_modifier: RecoveredField[XiFreeModifierSyntax]) -> FreeModifierSyntaxXiFreeModifier:
        return cls._from_fields((xi_free_modifier,))
    def __init__(self, xi_free_modifier: RecoveredField[XiFreeModifierSyntax]) -> None:
        pass
    @property
    def xi_free_modifier(self) -> RecoveredField[XiFreeModifierSyntax]:
        'Uses the nested `xi_free_modifier` sum form and preserves its selected alternative.'
        return cast(RecoveredField[XiFreeModifierSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('FreeModifierSyntaxXiFreeModifier is final')

@final
class FreeModifierSyntaxMaiFreeModifier(_SyntaxNode):
    'Uses the `mai_free_modifier` product form, whose payload preserves `number` and `mai`.'
    __slots__ = ()
    _schema_id = 399
    __match_args__ = ('mai_free_modifier',)
    def __new__(cls, mai_free_modifier: RecoveredField[MaiFreeModifierSyntax]) -> FreeModifierSyntaxMaiFreeModifier:
        return cls._from_fields((mai_free_modifier,))
    def __init__(self, mai_free_modifier: RecoveredField[MaiFreeModifierSyntax]) -> None:
        pass
    @property
    def mai_free_modifier(self) -> RecoveredField[MaiFreeModifierSyntax]:
        'Uses the `mai_free_modifier` product form, whose payload preserves `number` and `mai`.'
        return cast(RecoveredField[MaiFreeModifierSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('FreeModifierSyntaxMaiFreeModifier is final')

@final
class FreeModifierSyntaxZantufaMeksoMaiFreeModifier(_SyntaxNode):
    'Uses the `zantufa_mekso_mai_free_modifier` product form, whose payload preserves `expression` and `mai`.'
    __slots__ = ()
    _schema_id = 400
    __match_args__ = ('zantufa_mekso_mai_free_modifier',)
    def __new__(cls, zantufa_mekso_mai_free_modifier: RecoveredField[ZantufaMeksoMaiFreeModifierSyntax]) -> FreeModifierSyntaxZantufaMeksoMaiFreeModifier:
        return cls._from_fields((zantufa_mekso_mai_free_modifier,))
    def __init__(self, zantufa_mekso_mai_free_modifier: RecoveredField[ZantufaMeksoMaiFreeModifierSyntax]) -> None:
        pass
    @property
    def zantufa_mekso_mai_free_modifier(self) -> RecoveredField[ZantufaMeksoMaiFreeModifierSyntax]:
        'Uses the `zantufa_mekso_mai_free_modifier` product form, whose payload preserves `expression` and `mai`.'
        return cast(RecoveredField[ZantufaMeksoMaiFreeModifierSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('FreeModifierSyntaxZantufaMeksoMaiFreeModifier is final')

@final
class FreeModifierSyntaxSoiFreeModifier(_SyntaxNode):
    'Uses the `soi_free_modifier` product form, whose payload preserves `soi`, `leading_sumti`, `trailing_sumti`, and `sehu`.'
    __slots__ = ()
    _schema_id = 401
    __match_args__ = ('soi_free_modifier',)
    def __new__(cls, soi_free_modifier: RecoveredField[SoiFreeModifierSyntax]) -> FreeModifierSyntaxSoiFreeModifier:
        return cls._from_fields((soi_free_modifier,))
    def __init__(self, soi_free_modifier: RecoveredField[SoiFreeModifierSyntax]) -> None:
        pass
    @property
    def soi_free_modifier(self) -> RecoveredField[SoiFreeModifierSyntax]:
        'Uses the `soi_free_modifier` product form, whose payload preserves `soi`, `leading_sumti`, `trailing_sumti`, and `sehu`.'
        return cast(RecoveredField[SoiFreeModifierSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('FreeModifierSyntaxSoiFreeModifier is final')

@final
class FreeModifierSyntaxParentheticalText(_SyntaxNode):
    'Uses the `parenthetical_text` product form, whose payload preserves `to`, `text`, and `toi`.'
    __slots__ = ()
    _schema_id = 402
    __match_args__ = ('parenthetical_text',)
    def __new__(cls, parenthetical_text: RecoveredField[ParentheticalTextSyntax]) -> FreeModifierSyntaxParentheticalText:
        return cls._from_fields((parenthetical_text,))
    def __init__(self, parenthetical_text: RecoveredField[ParentheticalTextSyntax]) -> None:
        pass
    @property
    def parenthetical_text(self) -> RecoveredField[ParentheticalTextSyntax]:
        'Uses the `parenthetical_text` product form, whose payload preserves `to`, `text`, and `toi`.'
        return cast(RecoveredField[ParentheticalTextSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('FreeModifierSyntaxParentheticalText is final')

@final
class FreeModifierSyntaxVocativeFreeModifier(_SyntaxNode):
    'Uses the `vocative_free_modifier` product form, whose payload preserves `vocative_markers`, `sumti`, and `dohu`.'
    __slots__ = ()
    _schema_id = 403
    __match_args__ = ('vocative_free_modifier',)
    def __new__(cls, vocative_free_modifier: RecoveredField[VocativeFreeModifierSyntax]) -> FreeModifierSyntaxVocativeFreeModifier:
        return cls._from_fields((vocative_free_modifier,))
    def __init__(self, vocative_free_modifier: RecoveredField[VocativeFreeModifierSyntax]) -> None:
        pass
    @property
    def vocative_free_modifier(self) -> RecoveredField[VocativeFreeModifierSyntax]:
        'Uses the `vocative_free_modifier` product form, whose payload preserves `vocative_markers`, `sumti`, and `dohu`.'
        return cast(RecoveredField[VocativeFreeModifierSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('FreeModifierSyntaxVocativeFreeModifier is final')

FreeModifierSyntax: TypeAlias = FreeModifierSyntaxTextReplacementFreeModifier | FreeModifierSyntaxZantufaSeiStatementFreeModifier | FreeModifierSyntaxSeiFreeModifier | FreeModifierSyntaxXiFreeModifier | FreeModifierSyntaxMaiFreeModifier | FreeModifierSyntaxZantufaMeksoMaiFreeModifier | FreeModifierSyntaxSoiFreeModifier | FreeModifierSyntaxParentheticalText | FreeModifierSyntaxVocativeFreeModifier

@final
class VocativeFreeModifierSyntax(_SyntaxNode):
    'Product node for vocative phrase; preserves `vocative_markers`, `sumti`, and `dohu` in source order.'
    __slots__ = ()
    _schema_id = 404
    __match_args__ = ('vocative_markers', 'sumti', 'dohu')
    def __new__(cls, vocative_markers: WithFreeModifiers[RecoveredField[VocativeMarkerWordsSyntax], RecoveredField[FreeModifierSyntax]], sumti: RecoveredField[VocativeSumtiSyntax] | None, dohu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> VocativeFreeModifierSyntax:
        return cls._from_fields((vocative_markers, sumti, dohu))
    def __init__(self, vocative_markers: WithFreeModifiers[RecoveredField[VocativeMarkerWordsSyntax], RecoveredField[FreeModifierSyntax]], sumti: RecoveredField[VocativeSumtiSyntax] | None, dohu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def vocative_markers(self) -> WithFreeModifiers[RecoveredField[VocativeMarkerWordsSyntax], RecoveredField[FreeModifierSyntax]]:
        'The `vocative_marker_words` grammar result in the `vocative_markers` structural role of the `vocative_free_modifier` production.'
        return cast(WithFreeModifiers[RecoveredField[VocativeMarkerWordsSyntax], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def sumti(self) -> RecoveredField[VocativeSumtiSyntax] | None:
        'The optional sumti component.'
        return cast(RecoveredField[VocativeSumtiSyntax] | None, self._field(1))
    @property
    def dohu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Dohu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('VocativeFreeModifierSyntax is final')

@final
class ParentheticalTextSyntax(_SyntaxNode):
    'Product node for parenthetical text; preserves `to`, `text`, and `toi` in source order.'
    __slots__ = ()
    _schema_id = 405
    __match_args__ = ('to', 'text', 'toi')
    def __new__(cls, to: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], text: RecoveredField[TextSyntax], toi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> ParentheticalTextSyntax:
        return cls._from_fields((to, text, toi))
    def __init__(self, to: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], text: RecoveredField[TextSyntax], toi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def to(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `To`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def text(self) -> RecoveredField[TextSyntax]:
        'The shared text child syntax node.'
        return cast(RecoveredField[TextSyntax], self._field(1))
    @property
    def toi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Toi` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('ParentheticalTextSyntax is final')

@final
class SeiFreeModifierSyntax(_SyntaxNode):
    'Product node for metalinguistic comment; preserves `sei`, `terms`, `cu`, `selbri`, and `sehu` in source order.'
    __slots__ = ()
    _schema_id = 406
    __match_args__ = ('sei', 'terms', 'cu', 'selbri', 'sehu')
    def __new__(cls, sei: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], terms: Sequence[RecoveredField[TermSyntax]], cu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, selbri: RecoveredField[SelbriSyntax], sehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> SeiFreeModifierSyntax:
        return cls._from_fields((sei, terms, cu, selbri, sehu))
    def __init__(self, sei: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], terms: Sequence[RecoveredField[TermSyntax]], cu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, selbri: RecoveredField[SelbriSyntax], sehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def sei(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Sei`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def terms(self) -> tuple[RecoveredField[TermSyntax], ...]:
        'Ordered sequence of zero or more terms components.'
        return cast(tuple[RecoveredField[TermSyntax], ...], self._field(1))
    @property
    def cu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Cu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    @property
    def selbri(self) -> RecoveredField[SelbriSyntax]:
        'The shared selbri child syntax node.'
        return cast(RecoveredField[SelbriSyntax], self._field(3))
    @property
    def sehu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Sehu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(4))
    def __init_subclass__(cls) -> None:
        raise TypeError('SeiFreeModifierSyntax is final')

@final
class ZantufaSeiStatementFreeModifierSyntax(_SyntaxNode):
    'Product node for metalinguistic comment; preserves `sei`, `statement`, and `sehu` in source order.'
    __slots__ = ()
    _schema_id = 407
    __match_args__ = ('sei', 'statement', 'sehu')
    def __new__(cls, sei: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], statement: RecoveredField[StatementSyntax], sehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> ZantufaSeiStatementFreeModifierSyntax:
        return cls._from_fields((sei, statement, sehu))
    def __init__(self, sei: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], statement: RecoveredField[StatementSyntax], sehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def sei(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Sei`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def statement(self) -> RecoveredField[StatementSyntax]:
        'The shared statement child syntax node.'
        return cast(RecoveredField[StatementSyntax], self._field(1))
    @property
    def sehu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Sehu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaSeiStatementFreeModifierSyntax is final')

@final
class XiFreeModifierSyntaxXiNumberFreeModifier(_SyntaxNode):
    'Uses the `xi_number_free_modifier` product form, whose payload preserves `xi` and `expression`.'
    __slots__ = ()
    _schema_id = 408
    __match_args__ = ('xi_number_free_modifier',)
    def __new__(cls, xi_number_free_modifier: RecoveredField[XiNumberFreeModifierSyntax]) -> XiFreeModifierSyntaxXiNumberFreeModifier:
        return cls._from_fields((xi_number_free_modifier,))
    def __init__(self, xi_number_free_modifier: RecoveredField[XiNumberFreeModifierSyntax]) -> None:
        pass
    @property
    def xi_number_free_modifier(self) -> RecoveredField[XiNumberFreeModifierSyntax]:
        'Uses the `xi_number_free_modifier` product form, whose payload preserves `xi` and `expression`.'
        return cast(RecoveredField[XiNumberFreeModifierSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('XiFreeModifierSyntaxXiNumberFreeModifier is final')

@final
class XiFreeModifierSyntaxXiLerfuStringFreeModifier(_SyntaxNode):
    'Uses the `xi_lerfu_string_free_modifier` product form, whose payload preserves `xi` and `expression`.'
    __slots__ = ()
    _schema_id = 409
    __match_args__ = ('xi_lerfu_string_free_modifier',)
    def __new__(cls, xi_lerfu_string_free_modifier: RecoveredField[XiLerfuStringFreeModifierSyntax]) -> XiFreeModifierSyntaxXiLerfuStringFreeModifier:
        return cls._from_fields((xi_lerfu_string_free_modifier,))
    def __init__(self, xi_lerfu_string_free_modifier: RecoveredField[XiLerfuStringFreeModifierSyntax]) -> None:
        pass
    @property
    def xi_lerfu_string_free_modifier(self) -> RecoveredField[XiLerfuStringFreeModifierSyntax]:
        'Uses the `xi_lerfu_string_free_modifier` product form, whose payload preserves `xi` and `expression`.'
        return cast(RecoveredField[XiLerfuStringFreeModifierSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('XiFreeModifierSyntaxXiLerfuStringFreeModifier is final')

@final
class XiFreeModifierSyntaxXiParenthesizedFreeModifier(_SyntaxNode):
    'Uses the `xi_parenthesized_free_modifier` product form, whose payload preserves `xi` and `expression`.'
    __slots__ = ()
    _schema_id = 410
    __match_args__ = ('xi_parenthesized_free_modifier',)
    def __new__(cls, xi_parenthesized_free_modifier: RecoveredField[XiParenthesizedFreeModifierSyntax]) -> XiFreeModifierSyntaxXiParenthesizedFreeModifier:
        return cls._from_fields((xi_parenthesized_free_modifier,))
    def __init__(self, xi_parenthesized_free_modifier: RecoveredField[XiParenthesizedFreeModifierSyntax]) -> None:
        pass
    @property
    def xi_parenthesized_free_modifier(self) -> RecoveredField[XiParenthesizedFreeModifierSyntax]:
        'Uses the `xi_parenthesized_free_modifier` product form, whose payload preserves `xi` and `expression`.'
        return cast(RecoveredField[XiParenthesizedFreeModifierSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('XiFreeModifierSyntaxXiParenthesizedFreeModifier is final')

XiFreeModifierSyntax: TypeAlias = XiFreeModifierSyntaxXiNumberFreeModifier | XiFreeModifierSyntaxXiLerfuStringFreeModifier | XiFreeModifierSyntaxXiParenthesizedFreeModifier

@final
class XiNumberFreeModifierSyntax(_SyntaxNode):
    'Product node for subscript; preserves `xi` and `expression` in source order.'
    __slots__ = ()
    _schema_id = 411
    __match_args__ = ('xi', 'expression')
    def __new__(cls, xi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], expression: RecoveredField[NumberMeksoSyntax]) -> XiNumberFreeModifierSyntax:
        return cls._from_fields((xi, expression))
    def __init__(self, xi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], expression: RecoveredField[NumberMeksoSyntax]) -> None:
        pass
    @property
    def xi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Xi`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def expression(self) -> RecoveredField[NumberMeksoSyntax]:
        'The shared expression child syntax node.'
        return cast(RecoveredField[NumberMeksoSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('XiNumberFreeModifierSyntax is final')

@final
class XiLerfuStringFreeModifierSyntax(_SyntaxNode):
    'Product node for subscript; preserves `xi` and `expression` in source order.'
    __slots__ = ()
    _schema_id = 412
    __match_args__ = ('xi', 'expression')
    def __new__(cls, xi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], expression: RecoveredField[LerfuStringMeksoSyntax]) -> XiLerfuStringFreeModifierSyntax:
        return cls._from_fields((xi, expression))
    def __init__(self, xi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], expression: RecoveredField[LerfuStringMeksoSyntax]) -> None:
        pass
    @property
    def xi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Xi`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def expression(self) -> RecoveredField[LerfuStringMeksoSyntax]:
        'The shared expression child syntax node.'
        return cast(RecoveredField[LerfuStringMeksoSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('XiLerfuStringFreeModifierSyntax is final')

@final
class XiParenthesizedFreeModifierSyntax(_SyntaxNode):
    'Product node for subscript; preserves `xi` and `expression` in source order.'
    __slots__ = ()
    _schema_id = 413
    __match_args__ = ('xi', 'expression')
    def __new__(cls, xi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], expression: RecoveredField[ParenthesizedMeksoOperandSyntax]) -> XiParenthesizedFreeModifierSyntax:
        return cls._from_fields((xi, expression))
    def __init__(self, xi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], expression: RecoveredField[ParenthesizedMeksoOperandSyntax]) -> None:
        pass
    @property
    def xi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Xi`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def expression(self) -> RecoveredField[ParenthesizedMeksoOperandSyntax]:
        'The shared expression child syntax node.'
        return cast(RecoveredField[ParenthesizedMeksoOperandSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('XiParenthesizedFreeModifierSyntax is final')

@final
class MaiFreeModifierSyntax(_SyntaxNode):
    'Product node for utterance ordinal; preserves `number` and `mai` in source order.'
    __slots__ = ()
    _schema_id = 414
    __match_args__ = ('number', 'mai')
    def __new__(cls, number: RecoveredField[NumberOrLetterWordsSyntax], mai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> MaiFreeModifierSyntax:
        return cls._from_fields((number, mai))
    def __init__(self, number: RecoveredField[NumberOrLetterWordsSyntax], mai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def number(self) -> RecoveredField[NumberOrLetterWordsSyntax]:
        'The `number_or_letter_words` grammar result in the `number` structural role of the `mai_free_modifier` production.'
        return cast(RecoveredField[NumberOrLetterWordsSyntax], self._field(0))
    @property
    def mai(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Mai`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('MaiFreeModifierSyntax is final')

@final
class ZantufaMeksoMaiFreeModifierSyntax(_SyntaxNode):
    'Product node for utterance ordinal; preserves `expression` and `mai` in source order.'
    __slots__ = ()
    _schema_id = 415
    __match_args__ = ('expression', 'mai')
    def __new__(cls, expression: RecoveredField[MeksoSyntax], mai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> ZantufaMeksoMaiFreeModifierSyntax:
        return cls._from_fields((expression, mai))
    def __init__(self, expression: RecoveredField[MeksoSyntax], mai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def expression(self) -> RecoveredField[MeksoSyntax]:
        'The required shared mekso expression parsed by `mekso`, accepted only when immediately followed by a MAI-family word.'
        return cast(RecoveredField[MeksoSyntax], self._field(0))
    @property
    def mai(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Mai`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaMeksoMaiFreeModifierSyntax is final')

@final
class SoiFreeModifierSyntax(_SyntaxNode):
    'Product node for reciprocal; preserves `soi`, `leading_sumti`, `trailing_sumti`, and `sehu` in source order.'
    __slots__ = ()
    _schema_id = 416
    __match_args__ = ('soi', 'leading_sumti', 'trailing_sumti', 'sehu')
    def __new__(cls, soi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], leading_sumti: RecoveredField[SumtiSyntax], trailing_sumti: RecoveredField[SumtiSyntax] | None, sehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> SoiFreeModifierSyntax:
        return cls._from_fields((soi, leading_sumti, trailing_sumti, sehu))
    def __init__(self, soi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], leading_sumti: RecoveredField[SumtiSyntax], trailing_sumti: RecoveredField[SumtiSyntax] | None, sehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def soi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Soi` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def leading_sumti(self) -> RecoveredField[SumtiSyntax]:
        'The shared leading sumti child syntax node.'
        return cast(RecoveredField[SumtiSyntax], self._field(1))
    @property
    def trailing_sumti(self) -> RecoveredField[SumtiSyntax] | None:
        'The optional trailing sumti component.'
        return cast(RecoveredField[SumtiSyntax] | None, self._field(2))
    @property
    def sehu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Sehu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('SoiFreeModifierSyntax is final')

@final
class TextReplacementFreeModifierSyntaxFullTextReplacementFreeModifier(_SyntaxNode):
    'Uses the `full_text_replacement_free_modifier` product form, whose payload preserves `lohai`, `old_words`, `sahai`, `new_words`, and `lehai`.'
    __slots__ = ()
    _schema_id = 417
    __match_args__ = ('full_text_replacement_free_modifier',)
    def __new__(cls, full_text_replacement_free_modifier: RecoveredField[FullTextReplacementFreeModifierSyntax]) -> TextReplacementFreeModifierSyntaxFullTextReplacementFreeModifier:
        return cls._from_fields((full_text_replacement_free_modifier,))
    def __init__(self, full_text_replacement_free_modifier: RecoveredField[FullTextReplacementFreeModifierSyntax]) -> None:
        pass
    @property
    def full_text_replacement_free_modifier(self) -> RecoveredField[FullTextReplacementFreeModifierSyntax]:
        'Uses the `full_text_replacement_free_modifier` product form, whose payload preserves `lohai`, `old_words`, `sahai`, `new_words`, and `lehai`.'
        return cast(RecoveredField[FullTextReplacementFreeModifierSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TextReplacementFreeModifierSyntaxFullTextReplacementFreeModifier is final')

@final
class TextReplacementFreeModifierSyntaxNewOnlyTextReplacementFreeModifier(_SyntaxNode):
    'Uses the `new_only_text_replacement_free_modifier` product form, whose payload preserves `sahai`, `new_words`, and `lehai`.'
    __slots__ = ()
    _schema_id = 418
    __match_args__ = ('new_only_text_replacement_free_modifier',)
    def __new__(cls, new_only_text_replacement_free_modifier: RecoveredField[NewOnlyTextReplacementFreeModifierSyntax]) -> TextReplacementFreeModifierSyntaxNewOnlyTextReplacementFreeModifier:
        return cls._from_fields((new_only_text_replacement_free_modifier,))
    def __init__(self, new_only_text_replacement_free_modifier: RecoveredField[NewOnlyTextReplacementFreeModifierSyntax]) -> None:
        pass
    @property
    def new_only_text_replacement_free_modifier(self) -> RecoveredField[NewOnlyTextReplacementFreeModifierSyntax]:
        'Uses the `new_only_text_replacement_free_modifier` product form, whose payload preserves `sahai`, `new_words`, and `lehai`.'
        return cast(RecoveredField[NewOnlyTextReplacementFreeModifierSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TextReplacementFreeModifierSyntaxNewOnlyTextReplacementFreeModifier is final')

@final
class TextReplacementFreeModifierSyntaxCloseOnlyTextReplacementFreeModifier(_SyntaxNode):
    'Uses the `close_only_text_replacement_free_modifier` product form, whose payload preserves `lehai`.'
    __slots__ = ()
    _schema_id = 419
    __match_args__ = ('close_only_text_replacement_free_modifier',)
    def __new__(cls, close_only_text_replacement_free_modifier: RecoveredField[CloseOnlyTextReplacementFreeModifierSyntax]) -> TextReplacementFreeModifierSyntaxCloseOnlyTextReplacementFreeModifier:
        return cls._from_fields((close_only_text_replacement_free_modifier,))
    def __init__(self, close_only_text_replacement_free_modifier: RecoveredField[CloseOnlyTextReplacementFreeModifierSyntax]) -> None:
        pass
    @property
    def close_only_text_replacement_free_modifier(self) -> RecoveredField[CloseOnlyTextReplacementFreeModifierSyntax]:
        'Uses the `close_only_text_replacement_free_modifier` product form, whose payload preserves `lehai`.'
        return cast(RecoveredField[CloseOnlyTextReplacementFreeModifierSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TextReplacementFreeModifierSyntaxCloseOnlyTextReplacementFreeModifier is final')

TextReplacementFreeModifierSyntax: TypeAlias = TextReplacementFreeModifierSyntaxFullTextReplacementFreeModifier | TextReplacementFreeModifierSyntaxNewOnlyTextReplacementFreeModifier | TextReplacementFreeModifierSyntaxCloseOnlyTextReplacementFreeModifier

@final
class FullTextReplacementFreeModifierSyntax(_SyntaxNode):
    'Product node for replacement phrase; preserves `lohai`, `old_words`, `sahai`, `new_words`, and `lehai` in source order.'
    __slots__ = ()
    _schema_id = 420
    __match_args__ = ('lohai', 'old_words', 'sahai', 'new_words', 'lehai')
    def __new__(cls, lohai: RecoveredField[Token], old_words: Sequence[RecoveredField[Token]], sahai: RecoveredField[Token] | None, new_words: Sequence[RecoveredField[Token]], lehai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> FullTextReplacementFreeModifierSyntax:
        return cls._from_fields((lohai, old_words, sahai, new_words, lehai))
    def __init__(self, lohai: RecoveredField[Token], old_words: Sequence[RecoveredField[Token]], sahai: RecoveredField[Token] | None, new_words: Sequence[RecoveredField[Token]], lehai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def lohai(self) -> RecoveredField[Token]:
        'The `Lohai` cmavo marker.'
        return cast(RecoveredField[Token], self._field(0))
    @property
    def old_words(self) -> tuple[RecoveredField[Token], ...]:
        'Ordered sequence of zero or more old words components.'
        return cast(tuple[RecoveredField[Token], ...], self._field(1))
    @property
    def sahai(self) -> RecoveredField[Token] | None:
        'The optional `Sahai` cmavo marker.'
        return cast(RecoveredField[Token] | None, self._field(2))
    @property
    def new_words(self) -> tuple[RecoveredField[Token], ...]:
        'Ordered sequence of zero or more new words components.'
        return cast(tuple[RecoveredField[Token], ...], self._field(3))
    @property
    def lehai(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Lehai` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(4))
    def __init_subclass__(cls) -> None:
        raise TypeError('FullTextReplacementFreeModifierSyntax is final')

@final
class NewOnlyTextReplacementFreeModifierSyntax(_SyntaxNode):
    'Product node for replacement phrase; preserves `sahai`, `new_words`, and `lehai` in source order.'
    __slots__ = ()
    _schema_id = 421
    __match_args__ = ('sahai', 'new_words', 'lehai')
    def __new__(cls, sahai: RecoveredField[Token], new_words: Sequence[RecoveredField[Token]], lehai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> NewOnlyTextReplacementFreeModifierSyntax:
        return cls._from_fields((sahai, new_words, lehai))
    def __init__(self, sahai: RecoveredField[Token], new_words: Sequence[RecoveredField[Token]], lehai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def sahai(self) -> RecoveredField[Token]:
        'The `Sahai` cmavo marker.'
        return cast(RecoveredField[Token], self._field(0))
    @property
    def new_words(self) -> tuple[RecoveredField[Token], ...]:
        'Ordered sequence of zero or more new words components.'
        return cast(tuple[RecoveredField[Token], ...], self._field(1))
    @property
    def lehai(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Lehai` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('NewOnlyTextReplacementFreeModifierSyntax is final')

@final
class CloseOnlyTextReplacementFreeModifierSyntax(_SyntaxNode):
    'Transparent product node for replacement phrase; preserves the `lehai` component.'
    __slots__ = ()
    _schema_id = 422
    __match_args__ = ('lehai',)
    def __new__(cls, lehai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> CloseOnlyTextReplacementFreeModifierSyntax:
        return cls._from_fields((lehai,))
    def __init__(self, lehai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def lehai(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Lehai` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('CloseOnlyTextReplacementFreeModifierSyntax is final')

@final
class RelativeClauseTailSyntaxJoinedRelativeClauseTail(_SyntaxNode):
    'Uses the `joined_relative_clause_tail` product form, whose payload preserves `zihe` and `inner`.'
    __slots__ = ()
    _schema_id = 423
    __match_args__ = ('joined_relative_clause_tail',)
    def __new__(cls, joined_relative_clause_tail: RecoveredField[JoinedRelativeClauseTailSyntax]) -> RelativeClauseTailSyntaxJoinedRelativeClauseTail:
        return cls._from_fields((joined_relative_clause_tail,))
    def __init__(self, joined_relative_clause_tail: RecoveredField[JoinedRelativeClauseTailSyntax]) -> None:
        pass
    @property
    def joined_relative_clause_tail(self) -> RecoveredField[JoinedRelativeClauseTailSyntax]:
        'Uses the `joined_relative_clause_tail` product form, whose payload preserves `zihe` and `inner`.'
        return cast(RecoveredField[JoinedRelativeClauseTailSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('RelativeClauseTailSyntaxJoinedRelativeClauseTail is final')

@final
class RelativeClauseTailSyntaxConnectedRelativeClauseTail(_SyntaxNode):
    'Uses the `connected_relative_clause_tail` product form, whose payload preserves `connective` and `inner`.'
    __slots__ = ()
    _schema_id = 424
    __match_args__ = ('connected_relative_clause_tail',)
    def __new__(cls, connected_relative_clause_tail: RecoveredField[ConnectedRelativeClauseTailSyntax]) -> RelativeClauseTailSyntaxConnectedRelativeClauseTail:
        return cls._from_fields((connected_relative_clause_tail,))
    def __init__(self, connected_relative_clause_tail: RecoveredField[ConnectedRelativeClauseTailSyntax]) -> None:
        pass
    @property
    def connected_relative_clause_tail(self) -> RecoveredField[ConnectedRelativeClauseTailSyntax]:
        'Uses the `connected_relative_clause_tail` product form, whose payload preserves `connective` and `inner`.'
        return cast(RecoveredField[ConnectedRelativeClauseTailSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('RelativeClauseTailSyntaxConnectedRelativeClauseTail is final')

RelativeClauseTailSyntax: TypeAlias = RelativeClauseTailSyntaxJoinedRelativeClauseTail | RelativeClauseTailSyntaxConnectedRelativeClauseTail

@final
class JoinedRelativeClauseTailSyntax(_SyntaxNode):
    'Product node for relative clause; preserves `zihe` and `inner` in source order.'
    __slots__ = ()
    _schema_id = 425
    __match_args__ = ('zihe', 'inner')
    def __new__(cls, zihe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner: RecoveredField[RelativeClauseAtomSyntax]) -> JoinedRelativeClauseTailSyntax:
        return cls._from_fields((zihe, inner))
    def __init__(self, zihe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner: RecoveredField[RelativeClauseAtomSyntax]) -> None:
        pass
    @property
    def zihe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Zihe` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def inner(self) -> RecoveredField[RelativeClauseAtomSyntax]:
        'The shared inner child syntax node.'
        return cast(RecoveredField[RelativeClauseAtomSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('JoinedRelativeClauseTailSyntax is final')

@final
class ConnectedRelativeClauseTailSyntax(_SyntaxNode):
    'Product node for relative clause; preserves `connective` and `inner` in source order.'
    __slots__ = ()
    _schema_id = 426
    __match_args__ = ('connective', 'inner')
    def __new__(cls, connective: RecoveredField[RelativeClauseConnectiveSyntax], inner: RecoveredField[RelativeClauseAtomSyntax]) -> ConnectedRelativeClauseTailSyntax:
        return cls._from_fields((connective, inner))
    def __init__(self, connective: RecoveredField[RelativeClauseConnectiveSyntax], inner: RecoveredField[RelativeClauseAtomSyntax]) -> None:
        pass
    @property
    def connective(self) -> RecoveredField[RelativeClauseConnectiveSyntax]:
        'The `relative_clause_connective` connective joining the adjacent constituents of the `connected_relative_clause_tail` production.'
        return cast(RecoveredField[RelativeClauseConnectiveSyntax], self._field(0))
    @property
    def inner(self) -> RecoveredField[RelativeClauseAtomSyntax]:
        'The shared inner child syntax node.'
        return cast(RecoveredField[RelativeClauseAtomSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ConnectedRelativeClauseTailSyntax is final')

@final
class RelativeClauseConnectiveSyntaxJoikConnective(_SyntaxNode):
    'Uses the nested `joik_connective` sum form and preserves its selected alternative.'
    __slots__ = ()
    _schema_id = 427
    __match_args__ = ('joik_connective',)
    def __new__(cls, joik_connective: RecoveredField[JoikConnectiveSyntax]) -> RelativeClauseConnectiveSyntaxJoikConnective:
        return cls._from_fields((joik_connective,))
    def __init__(self, joik_connective: RecoveredField[JoikConnectiveSyntax]) -> None:
        pass
    @property
    def joik_connective(self) -> RecoveredField[JoikConnectiveSyntax]:
        'Uses the nested `joik_connective` sum form and preserves its selected alternative.'
        return cast(RecoveredField[JoikConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('RelativeClauseConnectiveSyntaxJoikConnective is final')

@final
class RelativeClauseConnectiveSyntaxJekConnective(_SyntaxNode):
    'Uses the `jek_connective` product form, whose payload preserves `na`, `se`, `ja`, and `nai`.'
    __slots__ = ()
    _schema_id = 428
    __match_args__ = ('jek_connective',)
    def __new__(cls, jek_connective: RecoveredField[JekConnectiveSyntax]) -> RelativeClauseConnectiveSyntaxJekConnective:
        return cls._from_fields((jek_connective,))
    def __init__(self, jek_connective: RecoveredField[JekConnectiveSyntax]) -> None:
        pass
    @property
    def jek_connective(self) -> RecoveredField[JekConnectiveSyntax]:
        'Uses the `jek_connective` product form, whose payload preserves `na`, `se`, `ja`, and `nai`.'
        return cast(RecoveredField[JekConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('RelativeClauseConnectiveSyntaxJekConnective is final')

RelativeClauseConnectiveSyntax: TypeAlias = RelativeClauseConnectiveSyntaxJoikConnective | RelativeClauseConnectiveSyntaxJekConnective

@final
class RelativeClauseAtomSyntaxSumtiAssociationRelativeClause(_SyntaxNode):
    'Uses the `sumti_association_relative_clause` product form, whose payload preserves `association_marker`, `sumti`, and `gehu`.'
    __slots__ = ()
    _schema_id = 429
    __match_args__ = ('sumti_association_relative_clause',)
    def __new__(cls, sumti_association_relative_clause: RecoveredField[SumtiAssociationRelativeClauseSyntax]) -> RelativeClauseAtomSyntaxSumtiAssociationRelativeClause:
        return cls._from_fields((sumti_association_relative_clause,))
    def __init__(self, sumti_association_relative_clause: RecoveredField[SumtiAssociationRelativeClauseSyntax]) -> None:
        pass
    @property
    def sumti_association_relative_clause(self) -> RecoveredField[SumtiAssociationRelativeClauseSyntax]:
        'Uses the `sumti_association_relative_clause` product form, whose payload preserves `association_marker`, `sumti`, and `gehu`.'
        return cast(RecoveredField[SumtiAssociationRelativeClauseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('RelativeClauseAtomSyntaxSumtiAssociationRelativeClause is final')

@final
class RelativeClauseAtomSyntaxBridiRelativeClause(_SyntaxNode):
    'Uses the nested `bridi_relative_clause` sum form and preserves its selected alternative.'
    __slots__ = ()
    _schema_id = 430
    __match_args__ = ('bridi_relative_clause',)
    def __new__(cls, bridi_relative_clause: RecoveredField[BridiRelativeClauseSyntax]) -> RelativeClauseAtomSyntaxBridiRelativeClause:
        return cls._from_fields((bridi_relative_clause,))
    def __init__(self, bridi_relative_clause: RecoveredField[BridiRelativeClauseSyntax]) -> None:
        pass
    @property
    def bridi_relative_clause(self) -> RecoveredField[BridiRelativeClauseSyntax]:
        'Uses the nested `bridi_relative_clause` sum form and preserves its selected alternative.'
        return cast(RecoveredField[BridiRelativeClauseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('RelativeClauseAtomSyntaxBridiRelativeClause is final')

RelativeClauseAtomSyntax: TypeAlias = RelativeClauseAtomSyntaxSumtiAssociationRelativeClause | RelativeClauseAtomSyntaxBridiRelativeClause

@final
class SumtiAssociationRelativeClauseSyntax(_SyntaxNode):
    'Product node for sumti association phrase; preserves `association_marker`, `sumti`, and `gehu` in source order.'
    __slots__ = ()
    _schema_id = 431
    __match_args__ = ('association_marker', 'sumti', 'gehu')
    def __new__(cls, association_marker: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], sumti: RecoveredField[RelativeSumtiSyntax], gehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> SumtiAssociationRelativeClauseSyntax:
        return cls._from_fields((association_marker, sumti, gehu))
    def __init__(self, association_marker: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], sumti: RecoveredField[RelativeSumtiSyntax], gehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def association_marker(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Goi`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def sumti(self) -> RecoveredField[RelativeSumtiSyntax]:
        'The shared sumti child syntax node.'
        return cast(RecoveredField[RelativeSumtiSyntax], self._field(1))
    @property
    def gehu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Gehu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('SumtiAssociationRelativeClauseSyntax is final')

@final
class RelativeSumtiSyntaxTenseTaggedRelativeSumti(_SyntaxNode):
    'Uses the `tense_tagged_relative_sumti` product form, whose payload preserves `tense_modal` and `sumti`.'
    __slots__ = ()
    _schema_id = 432
    __match_args__ = ('tense_tagged_relative_sumti',)
    def __new__(cls, tense_tagged_relative_sumti: RecoveredField[TenseTaggedRelativeSumtiSyntax]) -> RelativeSumtiSyntaxTenseTaggedRelativeSumti:
        return cls._from_fields((tense_tagged_relative_sumti,))
    def __init__(self, tense_tagged_relative_sumti: RecoveredField[TenseTaggedRelativeSumtiSyntax]) -> None:
        pass
    @property
    def tense_tagged_relative_sumti(self) -> RecoveredField[TenseTaggedRelativeSumtiSyntax]:
        'Uses the `tense_tagged_relative_sumti` product form, whose payload preserves `tense_modal` and `sumti`.'
        return cast(RecoveredField[TenseTaggedRelativeSumtiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('RelativeSumtiSyntaxTenseTaggedRelativeSumti is final')

@final
class RelativeSumtiSyntaxNaKuRelativeSumti(_SyntaxNode):
    'Uses the `na_ku_relative_sumti` product form, whose payload preserves `na` and `ku`.'
    __slots__ = ()
    _schema_id = 433
    __match_args__ = ('na_ku_relative_sumti',)
    def __new__(cls, na_ku_relative_sumti: RecoveredField[NaKuRelativeSumtiSyntax]) -> RelativeSumtiSyntaxNaKuRelativeSumti:
        return cls._from_fields((na_ku_relative_sumti,))
    def __init__(self, na_ku_relative_sumti: RecoveredField[NaKuRelativeSumtiSyntax]) -> None:
        pass
    @property
    def na_ku_relative_sumti(self) -> RecoveredField[NaKuRelativeSumtiSyntax]:
        'Uses the `na_ku_relative_sumti` product form, whose payload preserves `na` and `ku`.'
        return cast(RecoveredField[NaKuRelativeSumtiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('RelativeSumtiSyntaxNaKuRelativeSumti is final')

@final
class RelativeSumtiSyntaxPlainRelativeSumti(_SyntaxNode):
    'Uses the `plain_relative_sumti` product form, whose payload preserves `sumti`.'
    __slots__ = ()
    _schema_id = 434
    __match_args__ = ('plain_relative_sumti',)
    def __new__(cls, plain_relative_sumti: RecoveredField[PlainRelativeSumtiSyntax]) -> RelativeSumtiSyntaxPlainRelativeSumti:
        return cls._from_fields((plain_relative_sumti,))
    def __init__(self, plain_relative_sumti: RecoveredField[PlainRelativeSumtiSyntax]) -> None:
        pass
    @property
    def plain_relative_sumti(self) -> RecoveredField[PlainRelativeSumtiSyntax]:
        'Uses the `plain_relative_sumti` product form, whose payload preserves `sumti`.'
        return cast(RecoveredField[PlainRelativeSumtiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('RelativeSumtiSyntaxPlainRelativeSumti is final')

RelativeSumtiSyntax: TypeAlias = RelativeSumtiSyntaxTenseTaggedRelativeSumti | RelativeSumtiSyntaxNaKuRelativeSumti | RelativeSumtiSyntaxPlainRelativeSumti

@final
class NaKuRelativeSumtiSyntax(_SyntaxNode):
    'Product node for sumti association phrase; preserves `na` and `ku` in source order.'
    __slots__ = ()
    _schema_id = 435
    __match_args__ = ('na', 'ku')
    def __new__(cls, na: RecoveredField[Token], ku: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> NaKuRelativeSumtiSyntax:
        return cls._from_fields((na, ku))
    def __init__(self, na: RecoveredField[Token], ku: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def na(self) -> RecoveredField[Token]:
        'A word from selmaho `Na`.'
        return cast(RecoveredField[Token], self._field(0))
    @property
    def ku(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Ku` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('NaKuRelativeSumtiSyntax is final')

@final
class TenseTaggedRelativeSumtiSyntax(_SyntaxNode):
    'Product node for tagged sumti; preserves `tense_modal` and `sumti` in source order.'
    __slots__ = ()
    _schema_id = 436
    __match_args__ = ('tense_modal', 'sumti')
    def __new__(cls, tense_modal: RecoveredField[TenseModalSyntax], sumti: RecoveredField[TaggedOrElidedSumtiSyntax]) -> TenseTaggedRelativeSumtiSyntax:
        return cls._from_fields((tense_modal, sumti))
    def __init__(self, tense_modal: RecoveredField[TenseModalSyntax], sumti: RecoveredField[TaggedOrElidedSumtiSyntax]) -> None:
        pass
    @property
    def tense_modal(self) -> RecoveredField[TenseModalSyntax]:
        'The shared tense modal child syntax node.'
        return cast(RecoveredField[TenseModalSyntax], self._field(0))
    @property
    def sumti(self) -> RecoveredField[TaggedOrElidedSumtiSyntax]:
        'The shared sumti child syntax node.'
        return cast(RecoveredField[TaggedOrElidedSumtiSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('TenseTaggedRelativeSumtiSyntax is final')

@final
class PlainRelativeSumtiSyntax(_SyntaxNode):
    'Transparent product node for sumti association phrase; preserves the `sumti` component.'
    __slots__ = ()
    _schema_id = 437
    __match_args__ = ('sumti',)
    def __new__(cls, sumti: RecoveredField[SumtiSyntax]) -> PlainRelativeSumtiSyntax:
        return cls._from_fields((sumti,))
    def __init__(self, sumti: RecoveredField[SumtiSyntax]) -> None:
        pass
    @property
    def sumti(self) -> RecoveredField[SumtiSyntax]:
        'The shared sumti child syntax node.'
        return cast(RecoveredField[SumtiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('PlainRelativeSumtiSyntax is final')

@final
class BridiRelativeClauseSyntaxZantufaRestrictiveStatementRelativeClause(_SyntaxNode):
    'Uses the `zantufa_restrictive_statement_relative_clause` product form, whose payload preserves `poi`, `statement`, and `kuho`.'
    __slots__ = ()
    _schema_id = 438
    __match_args__ = ('zantufa_restrictive_statement_relative_clause',)
    def __new__(cls, zantufa_restrictive_statement_relative_clause: RecoveredField[ZantufaRestrictiveStatementRelativeClauseSyntax]) -> BridiRelativeClauseSyntaxZantufaRestrictiveStatementRelativeClause:
        return cls._from_fields((zantufa_restrictive_statement_relative_clause,))
    def __init__(self, zantufa_restrictive_statement_relative_clause: RecoveredField[ZantufaRestrictiveStatementRelativeClauseSyntax]) -> None:
        pass
    @property
    def zantufa_restrictive_statement_relative_clause(self) -> RecoveredField[ZantufaRestrictiveStatementRelativeClauseSyntax]:
        'Uses the `zantufa_restrictive_statement_relative_clause` product form, whose payload preserves `poi`, `statement`, and `kuho`.'
        return cast(RecoveredField[ZantufaRestrictiveStatementRelativeClauseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('BridiRelativeClauseSyntaxZantufaRestrictiveStatementRelativeClause is final')

@final
class BridiRelativeClauseSyntaxZantufaIncidentalStatementRelativeClause(_SyntaxNode):
    'Uses the `zantufa_incidental_statement_relative_clause` product form, whose payload preserves `noi`, `statement`, and `kuho`.'
    __slots__ = ()
    _schema_id = 439
    __match_args__ = ('zantufa_incidental_statement_relative_clause',)
    def __new__(cls, zantufa_incidental_statement_relative_clause: RecoveredField[ZantufaIncidentalStatementRelativeClauseSyntax]) -> BridiRelativeClauseSyntaxZantufaIncidentalStatementRelativeClause:
        return cls._from_fields((zantufa_incidental_statement_relative_clause,))
    def __init__(self, zantufa_incidental_statement_relative_clause: RecoveredField[ZantufaIncidentalStatementRelativeClauseSyntax]) -> None:
        pass
    @property
    def zantufa_incidental_statement_relative_clause(self) -> RecoveredField[ZantufaIncidentalStatementRelativeClauseSyntax]:
        'Uses the `zantufa_incidental_statement_relative_clause` product form, whose payload preserves `noi`, `statement`, and `kuho`.'
        return cast(RecoveredField[ZantufaIncidentalStatementRelativeClauseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('BridiRelativeClauseSyntaxZantufaIncidentalStatementRelativeClause is final')

@final
class BridiRelativeClauseSyntaxRestrictiveBridiRelativeClause(_SyntaxNode):
    'Uses the `restrictive_bridi_relative_clause` product form, whose payload preserves `poi`, `subbridi`, and `kuho`.'
    __slots__ = ()
    _schema_id = 440
    __match_args__ = ('restrictive_bridi_relative_clause',)
    def __new__(cls, restrictive_bridi_relative_clause: RecoveredField[RestrictiveBridiRelativeClauseSyntax]) -> BridiRelativeClauseSyntaxRestrictiveBridiRelativeClause:
        return cls._from_fields((restrictive_bridi_relative_clause,))
    def __init__(self, restrictive_bridi_relative_clause: RecoveredField[RestrictiveBridiRelativeClauseSyntax]) -> None:
        pass
    @property
    def restrictive_bridi_relative_clause(self) -> RecoveredField[RestrictiveBridiRelativeClauseSyntax]:
        'Uses the `restrictive_bridi_relative_clause` product form, whose payload preserves `poi`, `subbridi`, and `kuho`.'
        return cast(RecoveredField[RestrictiveBridiRelativeClauseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('BridiRelativeClauseSyntaxRestrictiveBridiRelativeClause is final')

@final
class BridiRelativeClauseSyntaxIncidentalBridiRelativeClause(_SyntaxNode):
    'Uses the `incidental_bridi_relative_clause` product form, whose payload preserves `noi`, `subbridi`, and `kuho`.'
    __slots__ = ()
    _schema_id = 441
    __match_args__ = ('incidental_bridi_relative_clause',)
    def __new__(cls, incidental_bridi_relative_clause: RecoveredField[IncidentalBridiRelativeClauseSyntax]) -> BridiRelativeClauseSyntaxIncidentalBridiRelativeClause:
        return cls._from_fields((incidental_bridi_relative_clause,))
    def __init__(self, incidental_bridi_relative_clause: RecoveredField[IncidentalBridiRelativeClauseSyntax]) -> None:
        pass
    @property
    def incidental_bridi_relative_clause(self) -> RecoveredField[IncidentalBridiRelativeClauseSyntax]:
        'Uses the `incidental_bridi_relative_clause` product form, whose payload preserves `noi`, `subbridi`, and `kuho`.'
        return cast(RecoveredField[IncidentalBridiRelativeClauseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('BridiRelativeClauseSyntaxIncidentalBridiRelativeClause is final')

BridiRelativeClauseSyntax: TypeAlias = BridiRelativeClauseSyntaxZantufaRestrictiveStatementRelativeClause | BridiRelativeClauseSyntaxZantufaIncidentalStatementRelativeClause | BridiRelativeClauseSyntaxRestrictiveBridiRelativeClause | BridiRelativeClauseSyntaxIncidentalBridiRelativeClause

@final
class ZantufaRestrictiveStatementRelativeClauseSyntax(_SyntaxNode):
    'Product node for relative clause; preserves `poi`, `statement`, and `kuho` in source order.'
    __slots__ = ()
    _schema_id = 442
    __match_args__ = ('poi', 'statement', 'kuho')
    def __new__(cls, poi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], statement: RecoveredField[StatementSyntax], kuho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> ZantufaRestrictiveStatementRelativeClauseSyntax:
        return cls._from_fields((poi, statement, kuho))
    def __init__(self, poi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], statement: RecoveredField[StatementSyntax], kuho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def poi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The selected grammar alternative in the `poi` structural role of the `zantufa_restrictive_statement_relative_clause` production.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def statement(self) -> RecoveredField[StatementSyntax]:
        'The shared statement child syntax node.'
        return cast(RecoveredField[StatementSyntax], self._field(1))
    @property
    def kuho(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Kuho` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaRestrictiveStatementRelativeClauseSyntax is final')

@final
class ZantufaIncidentalStatementRelativeClauseSyntax(_SyntaxNode):
    'Product node for relative clause; preserves `noi`, `statement`, and `kuho` in source order.'
    __slots__ = ()
    _schema_id = 443
    __match_args__ = ('noi', 'statement', 'kuho')
    def __new__(cls, noi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], statement: RecoveredField[StatementSyntax], kuho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> ZantufaIncidentalStatementRelativeClauseSyntax:
        return cls._from_fields((noi, statement, kuho))
    def __init__(self, noi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], statement: RecoveredField[StatementSyntax], kuho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def noi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The selected grammar alternative in the `noi` structural role of the `zantufa_incidental_statement_relative_clause` production.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def statement(self) -> RecoveredField[StatementSyntax]:
        'The shared statement child syntax node.'
        return cast(RecoveredField[StatementSyntax], self._field(1))
    @property
    def kuho(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Kuho` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaIncidentalStatementRelativeClauseSyntax is final')

@final
class RestrictiveBridiRelativeClauseSyntax(_SyntaxNode):
    'Product node for relative clause; preserves `poi`, `subbridi`, and `kuho` in source order.'
    __slots__ = ()
    _schema_id = 444
    __match_args__ = ('poi', 'subbridi', 'kuho')
    def __new__(cls, poi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], subbridi: RecoveredField[SubbridiSyntax], kuho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> RestrictiveBridiRelativeClauseSyntax:
        return cls._from_fields((poi, subbridi, kuho))
    def __init__(self, poi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], subbridi: RecoveredField[SubbridiSyntax], kuho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def poi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The selected grammar alternative in the `poi` structural role of the `restrictive_bridi_relative_clause` production.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def subbridi(self) -> RecoveredField[SubbridiSyntax]:
        'The shared subbridi child syntax node.'
        return cast(RecoveredField[SubbridiSyntax], self._field(1))
    @property
    def kuho(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Kuho` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('RestrictiveBridiRelativeClauseSyntax is final')

@final
class IncidentalBridiRelativeClauseSyntax(_SyntaxNode):
    'Product node for relative clause; preserves `noi`, `subbridi`, and `kuho` in source order.'
    __slots__ = ()
    _schema_id = 445
    __match_args__ = ('noi', 'subbridi', 'kuho')
    def __new__(cls, noi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], subbridi: RecoveredField[SubbridiSyntax], kuho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> IncidentalBridiRelativeClauseSyntax:
        return cls._from_fields((noi, subbridi, kuho))
    def __init__(self, noi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], subbridi: RecoveredField[SubbridiSyntax], kuho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def noi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The selected grammar alternative in the `noi` structural role of the `incidental_bridi_relative_clause` production.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def subbridi(self) -> RecoveredField[SubbridiSyntax]:
        'The shared subbridi child syntax node.'
        return cast(RecoveredField[SubbridiSyntax], self._field(1))
    @property
    def kuho(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Kuho` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('IncidentalBridiRelativeClauseSyntax is final')

@final
class EkConnectiveSyntax(_SyntaxNode):
    'Product node for ek; preserves `na`, `se`, `a`, and `nai` in source order.'
    __slots__ = ()
    _schema_id = 446
    __match_args__ = ('na', 'se', 'a', 'nai')
    def __new__(cls, na: RecoveredField[Token] | None, se: RecoveredField[Token] | None, a: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> EkConnectiveSyntax:
        return cls._from_fields((na, se, a, nai))
    def __init__(self, na: RecoveredField[Token] | None, se: RecoveredField[Token] | None, a: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def na(self) -> RecoveredField[Token] | None:
        'The optional na component.'
        return cast(RecoveredField[Token] | None, self._field(0))
    @property
    def se(self) -> RecoveredField[Token] | None:
        'The optional se component.'
        return cast(RecoveredField[Token] | None, self._field(1))
    @property
    def a(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `A`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(2))
    @property
    def nai(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Nai` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('EkConnectiveSyntax is final')

@final
class JehiConnectiveSyntax(_SyntaxNode):
    'Product node for ek; preserves `na`, `se`, `jehi`, and `nai` in source order.'
    __slots__ = ()
    _schema_id = 447
    __match_args__ = ('na', 'se', 'jehi', 'nai')
    def __new__(cls, na: RecoveredField[Token] | None, se: RecoveredField[Token] | None, jehi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> JehiConnectiveSyntax:
        return cls._from_fields((na, se, jehi, nai))
    def __init__(self, na: RecoveredField[Token] | None, se: RecoveredField[Token] | None, jehi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def na(self) -> RecoveredField[Token] | None:
        'The optional na component.'
        return cast(RecoveredField[Token] | None, self._field(0))
    @property
    def se(self) -> RecoveredField[Token] | None:
        'The optional se component.'
        return cast(RecoveredField[Token] | None, self._field(1))
    @property
    def jehi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Jehi`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(2))
    @property
    def nai(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Nai` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('JehiConnectiveSyntax is final')

@final
class JekConnectiveSyntax(_SyntaxNode):
    'Product node for jek; preserves `na`, `se`, `ja`, and `nai` in source order.'
    __slots__ = ()
    _schema_id = 448
    __match_args__ = ('na', 'se', 'ja', 'nai')
    def __new__(cls, na: RecoveredField[Token] | None, se: RecoveredField[Token] | None, ja: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> JekConnectiveSyntax:
        return cls._from_fields((na, se, ja, nai))
    def __init__(self, na: RecoveredField[Token] | None, se: RecoveredField[Token] | None, ja: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def na(self) -> RecoveredField[Token] | None:
        'The optional na component.'
        return cast(RecoveredField[Token] | None, self._field(0))
    @property
    def se(self) -> RecoveredField[Token] | None:
        'The optional se component.'
        return cast(RecoveredField[Token] | None, self._field(1))
    @property
    def ja(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Ja`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(2))
    @property
    def nai(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Nai` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('JekConnectiveSyntax is final')

@final
class JoikConnectiveSyntaxJoiConnective(_SyntaxNode):
    'Uses the `joi_connective` product form, whose payload preserves `se`, `joi`, and `nai`.'
    __slots__ = ()
    _schema_id = 449
    __match_args__ = ('joi_connective',)
    def __new__(cls, joi_connective: RecoveredField[JoiConnectiveSyntax]) -> JoikConnectiveSyntaxJoiConnective:
        return cls._from_fields((joi_connective,))
    def __init__(self, joi_connective: RecoveredField[JoiConnectiveSyntax]) -> None:
        pass
    @property
    def joi_connective(self) -> RecoveredField[JoiConnectiveSyntax]:
        'Uses the `joi_connective` product form, whose payload preserves `se`, `joi`, and `nai`.'
        return cast(RecoveredField[JoiConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('JoikConnectiveSyntaxJoiConnective is final')

@final
class JoikConnectiveSyntaxSimpleIntervalConnective(_SyntaxNode):
    'Uses the `simple_interval_connective` product form, whose payload preserves `se`, `bihi`, and `nai`.'
    __slots__ = ()
    _schema_id = 450
    __match_args__ = ('simple_interval_connective',)
    def __new__(cls, simple_interval_connective: RecoveredField[SimpleIntervalConnectiveSyntax]) -> JoikConnectiveSyntaxSimpleIntervalConnective:
        return cls._from_fields((simple_interval_connective,))
    def __init__(self, simple_interval_connective: RecoveredField[SimpleIntervalConnectiveSyntax]) -> None:
        pass
    @property
    def simple_interval_connective(self) -> RecoveredField[SimpleIntervalConnectiveSyntax]:
        'Uses the `simple_interval_connective` product form, whose payload preserves `se`, `bihi`, and `nai`.'
        return cast(RecoveredField[SimpleIntervalConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('JoikConnectiveSyntaxSimpleIntervalConnective is final')

@final
class JoikConnectiveSyntaxClosedIntervalConnective(_SyntaxNode):
    'Uses the `closed_interval_connective` product form, whose payload preserves `left_interval`, `se`, `bihi`, `nai`, and `right_interval`.'
    __slots__ = ()
    _schema_id = 451
    __match_args__ = ('closed_interval_connective',)
    def __new__(cls, closed_interval_connective: RecoveredField[ClosedIntervalConnectiveSyntax]) -> JoikConnectiveSyntaxClosedIntervalConnective:
        return cls._from_fields((closed_interval_connective,))
    def __init__(self, closed_interval_connective: RecoveredField[ClosedIntervalConnectiveSyntax]) -> None:
        pass
    @property
    def closed_interval_connective(self) -> RecoveredField[ClosedIntervalConnectiveSyntax]:
        'Uses the `closed_interval_connective` product form, whose payload preserves `left_interval`, `se`, `bihi`, `nai`, and `right_interval`.'
        return cast(RecoveredField[ClosedIntervalConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('JoikConnectiveSyntaxClosedIntervalConnective is final')

JoikConnectiveSyntax: TypeAlias = JoikConnectiveSyntaxJoiConnective | JoikConnectiveSyntaxSimpleIntervalConnective | JoikConnectiveSyntaxClosedIntervalConnective

@final
class JoiConnectiveSyntax(_SyntaxNode):
    'Product node for joik; preserves `se`, `joi`, and `nai` in source order.'
    __slots__ = ()
    _schema_id = 452
    __match_args__ = ('se', 'joi', 'nai')
    def __new__(cls, se: RecoveredField[Token] | None, joi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> JoiConnectiveSyntax:
        return cls._from_fields((se, joi, nai))
    def __init__(self, se: RecoveredField[Token] | None, joi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def se(self) -> RecoveredField[Token] | None:
        'The optional se component.'
        return cast(RecoveredField[Token] | None, self._field(0))
    @property
    def joi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Joi`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(1))
    @property
    def nai(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Nai` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('JoiConnectiveSyntax is final')

@final
class SimpleIntervalConnectiveSyntax(_SyntaxNode):
    'Product node for interval; preserves `se`, `bihi`, and `nai` in source order.'
    __slots__ = ()
    _schema_id = 453
    __match_args__ = ('se', 'bihi', 'nai')
    def __new__(cls, se: RecoveredField[Token] | None, bihi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> SimpleIntervalConnectiveSyntax:
        return cls._from_fields((se, bihi, nai))
    def __init__(self, se: RecoveredField[Token] | None, bihi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def se(self) -> RecoveredField[Token] | None:
        'The optional se component.'
        return cast(RecoveredField[Token] | None, self._field(0))
    @property
    def bihi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Bihi`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(1))
    @property
    def nai(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Nai` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('SimpleIntervalConnectiveSyntax is final')

@final
class ClosedIntervalConnectiveSyntax(_SyntaxNode):
    'Product node for interval; preserves `left_interval`, `se`, `bihi`, `nai`, and `right_interval` in source order.'
    __slots__ = ()
    _schema_id = 454
    __match_args__ = ('left_interval', 'se', 'bihi', 'nai', 'right_interval')
    def __new__(cls, left_interval: RecoveredField[Token], se: RecoveredField[Token] | None, bihi: RecoveredField[Token], nai: RecoveredField[Token] | None, right_interval: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> ClosedIntervalConnectiveSyntax:
        return cls._from_fields((left_interval, se, bihi, nai, right_interval))
    def __init__(self, left_interval: RecoveredField[Token], se: RecoveredField[Token] | None, bihi: RecoveredField[Token], nai: RecoveredField[Token] | None, right_interval: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def left_interval(self) -> RecoveredField[Token]:
        'A word from selmaho `Gaho`.'
        return cast(RecoveredField[Token], self._field(0))
    @property
    def se(self) -> RecoveredField[Token] | None:
        'The optional se component.'
        return cast(RecoveredField[Token] | None, self._field(1))
    @property
    def bihi(self) -> RecoveredField[Token]:
        'A word from selmaho `Bihi`.'
        return cast(RecoveredField[Token], self._field(2))
    @property
    def nai(self) -> RecoveredField[Token] | None:
        'The optional `Nai` cmavo marker.'
        return cast(RecoveredField[Token] | None, self._field(3))
    @property
    def right_interval(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Gaho`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(4))
    def __init_subclass__(cls) -> None:
        raise TypeError('ClosedIntervalConnectiveSyntax is final')

@final
class VuhuNonlogicalConnectiveSyntax(_SyntaxNode):
    'Transparent product node for non-logical connective; preserves the `vuhu` component.'
    __slots__ = ()
    _schema_id = 455
    __match_args__ = ('vuhu',)
    def __new__(cls, vuhu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> VuhuNonlogicalConnectiveSyntax:
        return cls._from_fields((vuhu,))
    def __init__(self, vuhu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def vuhu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Vuhu`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('VuhuNonlogicalConnectiveSyntax is final')

@final
class ArgumentConnectiveSyntaxCeheConnective(_SyntaxNode):
    'Uses the `cehe_connective` product form, whose payload preserves `cehe` and `nai`.'
    __slots__ = ()
    _schema_id = 456
    __match_args__ = ('cehe_connective',)
    def __new__(cls, cehe_connective: RecoveredField[CeheConnectiveSyntax]) -> ArgumentConnectiveSyntaxCeheConnective:
        return cls._from_fields((cehe_connective,))
    def __init__(self, cehe_connective: RecoveredField[CeheConnectiveSyntax]) -> None:
        pass
    @property
    def cehe_connective(self) -> RecoveredField[CeheConnectiveSyntax]:
        'Uses the `cehe_connective` product form, whose payload preserves `cehe` and `nai`.'
        return cast(RecoveredField[CeheConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ArgumentConnectiveSyntaxCeheConnective is final')

@final
class ArgumentConnectiveSyntaxEkConnective(_SyntaxNode):
    'Uses the `ek_connective` product form, whose payload preserves `na`, `se`, `a`, and `nai`.'
    __slots__ = ()
    _schema_id = 457
    __match_args__ = ('ek_connective',)
    def __new__(cls, ek_connective: RecoveredField[EkConnectiveSyntax]) -> ArgumentConnectiveSyntaxEkConnective:
        return cls._from_fields((ek_connective,))
    def __init__(self, ek_connective: RecoveredField[EkConnectiveSyntax]) -> None:
        pass
    @property
    def ek_connective(self) -> RecoveredField[EkConnectiveSyntax]:
        'Uses the `ek_connective` product form, whose payload preserves `na`, `se`, `a`, and `nai`.'
        return cast(RecoveredField[EkConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ArgumentConnectiveSyntaxEkConnective is final')

@final
class ArgumentConnectiveSyntaxJehiConnective(_SyntaxNode):
    'Uses the `jehi_connective` product form, whose payload preserves `na`, `se`, `jehi`, and `nai`.'
    __slots__ = ()
    _schema_id = 458
    __match_args__ = ('jehi_connective',)
    def __new__(cls, jehi_connective: RecoveredField[JehiConnectiveSyntax]) -> ArgumentConnectiveSyntaxJehiConnective:
        return cls._from_fields((jehi_connective,))
    def __init__(self, jehi_connective: RecoveredField[JehiConnectiveSyntax]) -> None:
        pass
    @property
    def jehi_connective(self) -> RecoveredField[JehiConnectiveSyntax]:
        'Uses the `jehi_connective` product form, whose payload preserves `na`, `se`, `jehi`, and `nai`.'
        return cast(RecoveredField[JehiConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ArgumentConnectiveSyntaxJehiConnective is final')

@final
class ArgumentConnectiveSyntaxJoikConnective(_SyntaxNode):
    'Uses the nested `joik_connective` sum form and preserves its selected alternative.'
    __slots__ = ()
    _schema_id = 459
    __match_args__ = ('joik_connective',)
    def __new__(cls, joik_connective: RecoveredField[JoikConnectiveSyntax]) -> ArgumentConnectiveSyntaxJoikConnective:
        return cls._from_fields((joik_connective,))
    def __init__(self, joik_connective: RecoveredField[JoikConnectiveSyntax]) -> None:
        pass
    @property
    def joik_connective(self) -> RecoveredField[JoikConnectiveSyntax]:
        'Uses the nested `joik_connective` sum form and preserves its selected alternative.'
        return cast(RecoveredField[JoikConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ArgumentConnectiveSyntaxJoikConnective is final')

@final
class ArgumentConnectiveSyntaxVuhuNonlogicalConnective(_SyntaxNode):
    'Uses the `vuhu_nonlogical_connective` product form, whose payload preserves `vuhu`.'
    __slots__ = ()
    _schema_id = 460
    __match_args__ = ('vuhu_nonlogical_connective',)
    def __new__(cls, vuhu_nonlogical_connective: RecoveredField[VuhuNonlogicalConnectiveSyntax]) -> ArgumentConnectiveSyntaxVuhuNonlogicalConnective:
        return cls._from_fields((vuhu_nonlogical_connective,))
    def __init__(self, vuhu_nonlogical_connective: RecoveredField[VuhuNonlogicalConnectiveSyntax]) -> None:
        pass
    @property
    def vuhu_nonlogical_connective(self) -> RecoveredField[VuhuNonlogicalConnectiveSyntax]:
        'Uses the `vuhu_nonlogical_connective` product form, whose payload preserves `vuhu`.'
        return cast(RecoveredField[VuhuNonlogicalConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ArgumentConnectiveSyntaxVuhuNonlogicalConnective is final')

ArgumentConnectiveSyntax: TypeAlias = ArgumentConnectiveSyntaxCeheConnective | ArgumentConnectiveSyntaxEkConnective | ArgumentConnectiveSyntaxJehiConnective | ArgumentConnectiveSyntaxJoikConnective | ArgumentConnectiveSyntaxVuhuNonlogicalConnective

@final
class OperandConnectiveSyntaxJoikConnective(_SyntaxNode):
    'Uses the nested `joik_connective` sum form and preserves its selected alternative.'
    __slots__ = ()
    _schema_id = 461
    __match_args__ = ('joik_connective',)
    def __new__(cls, joik_connective: RecoveredField[JoikConnectiveSyntax]) -> OperandConnectiveSyntaxJoikConnective:
        return cls._from_fields((joik_connective,))
    def __init__(self, joik_connective: RecoveredField[JoikConnectiveSyntax]) -> None:
        pass
    @property
    def joik_connective(self) -> RecoveredField[JoikConnectiveSyntax]:
        'Uses the nested `joik_connective` sum form and preserves its selected alternative.'
        return cast(RecoveredField[JoikConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('OperandConnectiveSyntaxJoikConnective is final')

@final
class OperandConnectiveSyntaxEkConnective(_SyntaxNode):
    'Uses the `ek_connective` product form, whose payload preserves `na`, `se`, `a`, and `nai`.'
    __slots__ = ()
    _schema_id = 462
    __match_args__ = ('ek_connective',)
    def __new__(cls, ek_connective: RecoveredField[EkConnectiveSyntax]) -> OperandConnectiveSyntaxEkConnective:
        return cls._from_fields((ek_connective,))
    def __init__(self, ek_connective: RecoveredField[EkConnectiveSyntax]) -> None:
        pass
    @property
    def ek_connective(self) -> RecoveredField[EkConnectiveSyntax]:
        'Uses the `ek_connective` product form, whose payload preserves `na`, `se`, `a`, and `nai`.'
        return cast(RecoveredField[EkConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('OperandConnectiveSyntaxEkConnective is final')

@final
class OperandConnectiveSyntaxJekConnective(_SyntaxNode):
    'Uses the `jek_connective` product form, whose payload preserves `na`, `se`, `ja`, and `nai`.'
    __slots__ = ()
    _schema_id = 463
    __match_args__ = ('jek_connective',)
    def __new__(cls, jek_connective: RecoveredField[JekConnectiveSyntax]) -> OperandConnectiveSyntaxJekConnective:
        return cls._from_fields((jek_connective,))
    def __init__(self, jek_connective: RecoveredField[JekConnectiveSyntax]) -> None:
        pass
    @property
    def jek_connective(self) -> RecoveredField[JekConnectiveSyntax]:
        'Uses the `jek_connective` product form, whose payload preserves `na`, `se`, `ja`, and `nai`.'
        return cast(RecoveredField[JekConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('OperandConnectiveSyntaxJekConnective is final')

OperandConnectiveSyntax: TypeAlias = OperandConnectiveSyntaxJoikConnective | OperandConnectiveSyntaxEkConnective | OperandConnectiveSyntaxJekConnective

@final
class RelationAfterthoughtConnectiveSyntaxJoikConnective(_SyntaxNode):
    'Uses the nested `joik_connective` sum form and preserves its selected alternative.'
    __slots__ = ()
    _schema_id = 464
    __match_args__ = ('joik_connective',)
    def __new__(cls, joik_connective: RecoveredField[JoikConnectiveSyntax]) -> RelationAfterthoughtConnectiveSyntaxJoikConnective:
        return cls._from_fields((joik_connective,))
    def __init__(self, joik_connective: RecoveredField[JoikConnectiveSyntax]) -> None:
        pass
    @property
    def joik_connective(self) -> RecoveredField[JoikConnectiveSyntax]:
        'Uses the nested `joik_connective` sum form and preserves its selected alternative.'
        return cast(RecoveredField[JoikConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('RelationAfterthoughtConnectiveSyntaxJoikConnective is final')

@final
class RelationAfterthoughtConnectiveSyntaxJekConnective(_SyntaxNode):
    'Uses the `jek_connective` product form, whose payload preserves `na`, `se`, `ja`, and `nai`.'
    __slots__ = ()
    _schema_id = 465
    __match_args__ = ('jek_connective',)
    def __new__(cls, jek_connective: RecoveredField[JekConnectiveSyntax]) -> RelationAfterthoughtConnectiveSyntaxJekConnective:
        return cls._from_fields((jek_connective,))
    def __init__(self, jek_connective: RecoveredField[JekConnectiveSyntax]) -> None:
        pass
    @property
    def jek_connective(self) -> RecoveredField[JekConnectiveSyntax]:
        'Uses the `jek_connective` product form, whose payload preserves `na`, `se`, `ja`, and `nai`.'
        return cast(RecoveredField[JekConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('RelationAfterthoughtConnectiveSyntaxJekConnective is final')

@final
class RelationAfterthoughtConnectiveSyntaxEkConnective(_SyntaxNode):
    'Uses the `ek_connective` product form, whose payload preserves `na`, `se`, `a`, and `nai`.'
    __slots__ = ()
    _schema_id = 466
    __match_args__ = ('ek_connective',)
    def __new__(cls, ek_connective: RecoveredField[EkConnectiveSyntax]) -> RelationAfterthoughtConnectiveSyntaxEkConnective:
        return cls._from_fields((ek_connective,))
    def __init__(self, ek_connective: RecoveredField[EkConnectiveSyntax]) -> None:
        pass
    @property
    def ek_connective(self) -> RecoveredField[EkConnectiveSyntax]:
        'Uses the `ek_connective` product form, whose payload preserves `na`, `se`, `a`, and `nai`.'
        return cast(RecoveredField[EkConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('RelationAfterthoughtConnectiveSyntaxEkConnective is final')

@final
class RelationAfterthoughtConnectiveSyntaxVuhuNonlogicalConnective(_SyntaxNode):
    'Uses the `vuhu_nonlogical_connective` product form, whose payload preserves `vuhu`.'
    __slots__ = ()
    _schema_id = 467
    __match_args__ = ('vuhu_nonlogical_connective',)
    def __new__(cls, vuhu_nonlogical_connective: RecoveredField[VuhuNonlogicalConnectiveSyntax]) -> RelationAfterthoughtConnectiveSyntaxVuhuNonlogicalConnective:
        return cls._from_fields((vuhu_nonlogical_connective,))
    def __init__(self, vuhu_nonlogical_connective: RecoveredField[VuhuNonlogicalConnectiveSyntax]) -> None:
        pass
    @property
    def vuhu_nonlogical_connective(self) -> RecoveredField[VuhuNonlogicalConnectiveSyntax]:
        'Uses the `vuhu_nonlogical_connective` product form, whose payload preserves `vuhu`.'
        return cast(RecoveredField[VuhuNonlogicalConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('RelationAfterthoughtConnectiveSyntaxVuhuNonlogicalConnective is final')

RelationAfterthoughtConnectiveSyntax: TypeAlias = RelationAfterthoughtConnectiveSyntaxJoikConnective | RelationAfterthoughtConnectiveSyntaxJekConnective | RelationAfterthoughtConnectiveSyntaxEkConnective | RelationAfterthoughtConnectiveSyntaxVuhuNonlogicalConnective

@final
class StandardStatementConnectiveSyntaxJoikConnective(_SyntaxNode):
    'Uses the nested `joik_connective` sum form and preserves its selected alternative.'
    __slots__ = ()
    _schema_id = 468
    __match_args__ = ('joik_connective',)
    def __new__(cls, joik_connective: RecoveredField[JoikConnectiveSyntax]) -> StandardStatementConnectiveSyntaxJoikConnective:
        return cls._from_fields((joik_connective,))
    def __init__(self, joik_connective: RecoveredField[JoikConnectiveSyntax]) -> None:
        pass
    @property
    def joik_connective(self) -> RecoveredField[JoikConnectiveSyntax]:
        'Uses the nested `joik_connective` sum form and preserves its selected alternative.'
        return cast(RecoveredField[JoikConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('StandardStatementConnectiveSyntaxJoikConnective is final')

@final
class StandardStatementConnectiveSyntaxJekConnective(_SyntaxNode):
    'Uses the `jek_connective` product form, whose payload preserves `na`, `se`, `ja`, and `nai`.'
    __slots__ = ()
    _schema_id = 469
    __match_args__ = ('jek_connective',)
    def __new__(cls, jek_connective: RecoveredField[JekConnectiveSyntax]) -> StandardStatementConnectiveSyntaxJekConnective:
        return cls._from_fields((jek_connective,))
    def __init__(self, jek_connective: RecoveredField[JekConnectiveSyntax]) -> None:
        pass
    @property
    def jek_connective(self) -> RecoveredField[JekConnectiveSyntax]:
        'Uses the `jek_connective` product form, whose payload preserves `na`, `se`, `ja`, and `nai`.'
        return cast(RecoveredField[JekConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('StandardStatementConnectiveSyntaxJekConnective is final')

StandardStatementConnectiveSyntax: TypeAlias = StandardStatementConnectiveSyntaxJoikConnective | StandardStatementConnectiveSyntaxJekConnective

@final
class StatementConnectiveSyntaxJoikConnective(_SyntaxNode):
    'Uses the nested `joik_connective` sum form and preserves its selected alternative.'
    __slots__ = ()
    _schema_id = 470
    __match_args__ = ('joik_connective',)
    def __new__(cls, joik_connective: RecoveredField[JoikConnectiveSyntax]) -> StatementConnectiveSyntaxJoikConnective:
        return cls._from_fields((joik_connective,))
    def __init__(self, joik_connective: RecoveredField[JoikConnectiveSyntax]) -> None:
        pass
    @property
    def joik_connective(self) -> RecoveredField[JoikConnectiveSyntax]:
        'Uses the nested `joik_connective` sum form and preserves its selected alternative.'
        return cast(RecoveredField[JoikConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('StatementConnectiveSyntaxJoikConnective is final')

@final
class StatementConnectiveSyntaxJekConnective(_SyntaxNode):
    'Uses the `jek_connective` product form, whose payload preserves `na`, `se`, `ja`, and `nai`.'
    __slots__ = ()
    _schema_id = 471
    __match_args__ = ('jek_connective',)
    def __new__(cls, jek_connective: RecoveredField[JekConnectiveSyntax]) -> StatementConnectiveSyntaxJekConnective:
        return cls._from_fields((jek_connective,))
    def __init__(self, jek_connective: RecoveredField[JekConnectiveSyntax]) -> None:
        pass
    @property
    def jek_connective(self) -> RecoveredField[JekConnectiveSyntax]:
        'Uses the `jek_connective` product form, whose payload preserves `na`, `se`, `ja`, and `nai`.'
        return cast(RecoveredField[JekConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('StatementConnectiveSyntaxJekConnective is final')

@final
class StatementConnectiveSyntaxEkConnective(_SyntaxNode):
    'Uses the `ek_connective` product form, whose payload preserves `na`, `se`, `a`, and `nai`.'
    __slots__ = ()
    _schema_id = 472
    __match_args__ = ('ek_connective',)
    def __new__(cls, ek_connective: RecoveredField[EkConnectiveSyntax]) -> StatementConnectiveSyntaxEkConnective:
        return cls._from_fields((ek_connective,))
    def __init__(self, ek_connective: RecoveredField[EkConnectiveSyntax]) -> None:
        pass
    @property
    def ek_connective(self) -> RecoveredField[EkConnectiveSyntax]:
        'Uses the `ek_connective` product form, whose payload preserves `na`, `se`, `a`, and `nai`.'
        return cast(RecoveredField[EkConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('StatementConnectiveSyntaxEkConnective is final')

@final
class StatementConnectiveSyntaxVuhuNonlogicalConnective(_SyntaxNode):
    'Uses the `vuhu_nonlogical_connective` product form, whose payload preserves `vuhu`.'
    __slots__ = ()
    _schema_id = 473
    __match_args__ = ('vuhu_nonlogical_connective',)
    def __new__(cls, vuhu_nonlogical_connective: RecoveredField[VuhuNonlogicalConnectiveSyntax]) -> StatementConnectiveSyntaxVuhuNonlogicalConnective:
        return cls._from_fields((vuhu_nonlogical_connective,))
    def __init__(self, vuhu_nonlogical_connective: RecoveredField[VuhuNonlogicalConnectiveSyntax]) -> None:
        pass
    @property
    def vuhu_nonlogical_connective(self) -> RecoveredField[VuhuNonlogicalConnectiveSyntax]:
        'Uses the `vuhu_nonlogical_connective` product form, whose payload preserves `vuhu`.'
        return cast(RecoveredField[VuhuNonlogicalConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('StatementConnectiveSyntaxVuhuNonlogicalConnective is final')

StatementConnectiveSyntax: TypeAlias = StatementConnectiveSyntaxJoikConnective | StatementConnectiveSyntaxJekConnective | StatementConnectiveSyntaxEkConnective | StatementConnectiveSyntaxVuhuNonlogicalConnective

@final
class TextLeadingConnectiveSyntaxStandardStatementConnective(_SyntaxNode):
    'Uses the nested `standard_statement_connective` sum form and preserves its selected alternative.'
    __slots__ = ()
    _schema_id = 474
    __match_args__ = ('standard_statement_connective',)
    def __new__(cls, standard_statement_connective: RecoveredField[StandardStatementConnectiveSyntax]) -> TextLeadingConnectiveSyntaxStandardStatementConnective:
        return cls._from_fields((standard_statement_connective,))
    def __init__(self, standard_statement_connective: RecoveredField[StandardStatementConnectiveSyntax]) -> None:
        pass
    @property
    def standard_statement_connective(self) -> RecoveredField[StandardStatementConnectiveSyntax]:
        'Uses the nested `standard_statement_connective` sum form and preserves its selected alternative.'
        return cast(RecoveredField[StandardStatementConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TextLeadingConnectiveSyntaxStandardStatementConnective is final')

@final
class TextLeadingConnectiveSyntaxCeheConnective(_SyntaxNode):
    'Uses the `cehe_connective` product form, whose payload preserves `cehe` and `nai`.'
    __slots__ = ()
    _schema_id = 475
    __match_args__ = ('cehe_connective',)
    def __new__(cls, cehe_connective: RecoveredField[CeheConnectiveSyntax]) -> TextLeadingConnectiveSyntaxCeheConnective:
        return cls._from_fields((cehe_connective,))
    def __init__(self, cehe_connective: RecoveredField[CeheConnectiveSyntax]) -> None:
        pass
    @property
    def cehe_connective(self) -> RecoveredField[CeheConnectiveSyntax]:
        'Uses the `cehe_connective` product form, whose payload preserves `cehe` and `nai`.'
        return cast(RecoveredField[CeheConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TextLeadingConnectiveSyntaxCeheConnective is final')

TextLeadingConnectiveSyntax: TypeAlias = TextLeadingConnectiveSyntaxStandardStatementConnective | TextLeadingConnectiveSyntaxCeheConnective

@final
class IStatementConnectiveSyntaxIStandardStatementConnective(_SyntaxNode):
    'Uses the `i_standard_statement_connective` product form, whose payload preserves `connective` and `tag_bo`.'
    __slots__ = ()
    _schema_id = 476
    __match_args__ = ('i_standard_statement_connective',)
    def __new__(cls, i_standard_statement_connective: RecoveredField[IStandardStatementConnectiveSyntax]) -> IStatementConnectiveSyntaxIStandardStatementConnective:
        return cls._from_fields((i_standard_statement_connective,))
    def __init__(self, i_standard_statement_connective: RecoveredField[IStandardStatementConnectiveSyntax]) -> None:
        pass
    @property
    def i_standard_statement_connective(self) -> RecoveredField[IStandardStatementConnectiveSyntax]:
        'Uses the `i_standard_statement_connective` product form, whose payload preserves `connective` and `tag_bo`.'
        return cast(RecoveredField[IStandardStatementConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('IStatementConnectiveSyntaxIStandardStatementConnective is final')

@final
class IStatementConnectiveSyntaxITagBoStatementConnective(_SyntaxNode):
    'Uses the `i_tag_bo_statement_connective` product form, whose payload preserves `tense_modal` and `bo`.'
    __slots__ = ()
    _schema_id = 477
    __match_args__ = ('i_tag_bo_statement_connective',)
    def __new__(cls, i_tag_bo_statement_connective: RecoveredField[ITagBoStatementConnectiveSyntax]) -> IStatementConnectiveSyntaxITagBoStatementConnective:
        return cls._from_fields((i_tag_bo_statement_connective,))
    def __init__(self, i_tag_bo_statement_connective: RecoveredField[ITagBoStatementConnectiveSyntax]) -> None:
        pass
    @property
    def i_tag_bo_statement_connective(self) -> RecoveredField[ITagBoStatementConnectiveSyntax]:
        'Uses the `i_tag_bo_statement_connective` product form, whose payload preserves `tense_modal` and `bo`.'
        return cast(RecoveredField[ITagBoStatementConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('IStatementConnectiveSyntaxITagBoStatementConnective is final')

IStatementConnectiveSyntax: TypeAlias = IStatementConnectiveSyntaxIStandardStatementConnective | IStatementConnectiveSyntaxITagBoStatementConnective

@final
class IStandardStatementConnectiveSyntax(_SyntaxNode):
    'Product node for statement connective; preserves `connective` and `tag_bo` in source order.'
    __slots__ = ()
    _schema_id = 478
    __match_args__ = ('connective', 'tag_bo')
    def __new__(cls, connective: RecoveredField[StatementConnectiveSyntax], tag_bo: tuple[RecoveredField[TenseModalSyntax] | None, WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]] | None) -> IStandardStatementConnectiveSyntax:
        return cls._from_fields((connective, tag_bo))
    def __init__(self, connective: RecoveredField[StatementConnectiveSyntax], tag_bo: tuple[RecoveredField[TenseModalSyntax] | None, WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]] | None) -> None:
        pass
    @property
    def connective(self) -> RecoveredField[StatementConnectiveSyntax]:
        'The shared connective child syntax node.'
        return cast(RecoveredField[StatementConnectiveSyntax], self._field(0))
    @property
    def tag_bo(self) -> tuple[RecoveredField[TenseModalSyntax] | None, WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]] | None:
        'The optional pair containing an optional shared tense-modal child followed by a required `Bo` cmavo marker.'
        return cast(tuple[RecoveredField[TenseModalSyntax] | None, WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]] | None, self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('IStandardStatementConnectiveSyntax is final')

@final
class IParagraphStatementConnectiveSyntaxIStandardParagraphStatementConnective(_SyntaxNode):
    'Uses the `i_standard_paragraph_statement_connective` product form, whose payload preserves `connective` and `tag_bo`.'
    __slots__ = ()
    _schema_id = 479
    __match_args__ = ('i_standard_paragraph_statement_connective',)
    def __new__(cls, i_standard_paragraph_statement_connective: RecoveredField[IStandardParagraphStatementConnectiveSyntax]) -> IParagraphStatementConnectiveSyntaxIStandardParagraphStatementConnective:
        return cls._from_fields((i_standard_paragraph_statement_connective,))
    def __init__(self, i_standard_paragraph_statement_connective: RecoveredField[IStandardParagraphStatementConnectiveSyntax]) -> None:
        pass
    @property
    def i_standard_paragraph_statement_connective(self) -> RecoveredField[IStandardParagraphStatementConnectiveSyntax]:
        'Uses the `i_standard_paragraph_statement_connective` product form, whose payload preserves `connective` and `tag_bo`.'
        return cast(RecoveredField[IStandardParagraphStatementConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('IParagraphStatementConnectiveSyntaxIStandardParagraphStatementConnective is final')

@final
class IParagraphStatementConnectiveSyntaxITagBoParagraphStatementConnective(_SyntaxNode):
    'Uses the `i_tag_bo_paragraph_statement_connective` product form, whose payload preserves `tense_modal` and `bo`.'
    __slots__ = ()
    _schema_id = 480
    __match_args__ = ('i_tag_bo_paragraph_statement_connective',)
    def __new__(cls, i_tag_bo_paragraph_statement_connective: RecoveredField[ITagBoParagraphStatementConnectiveSyntax]) -> IParagraphStatementConnectiveSyntaxITagBoParagraphStatementConnective:
        return cls._from_fields((i_tag_bo_paragraph_statement_connective,))
    def __init__(self, i_tag_bo_paragraph_statement_connective: RecoveredField[ITagBoParagraphStatementConnectiveSyntax]) -> None:
        pass
    @property
    def i_tag_bo_paragraph_statement_connective(self) -> RecoveredField[ITagBoParagraphStatementConnectiveSyntax]:
        'Uses the `i_tag_bo_paragraph_statement_connective` product form, whose payload preserves `tense_modal` and `bo`.'
        return cast(RecoveredField[ITagBoParagraphStatementConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('IParagraphStatementConnectiveSyntaxITagBoParagraphStatementConnective is final')

IParagraphStatementConnectiveSyntax: TypeAlias = IParagraphStatementConnectiveSyntaxIStandardParagraphStatementConnective | IParagraphStatementConnectiveSyntaxITagBoParagraphStatementConnective

@final
class IStandardParagraphStatementConnectiveSyntax(_SyntaxNode):
    'Product node for statement connective; preserves `connective` and `tag_bo` in source order.'
    __slots__ = ()
    _schema_id = 481
    __match_args__ = ('connective', 'tag_bo')
    def __new__(cls, connective: RecoveredField[ParagraphStandardStatementConnectiveSyntax], tag_bo: tuple[RecoveredField[TenseModalSyntax] | None, RecoveredField[Token]] | None) -> IStandardParagraphStatementConnectiveSyntax:
        return cls._from_fields((connective, tag_bo))
    def __init__(self, connective: RecoveredField[ParagraphStandardStatementConnectiveSyntax], tag_bo: tuple[RecoveredField[TenseModalSyntax] | None, RecoveredField[Token]] | None) -> None:
        pass
    @property
    def connective(self) -> RecoveredField[ParagraphStandardStatementConnectiveSyntax]:
        'The shared connective child syntax node.'
        return cast(RecoveredField[ParagraphStandardStatementConnectiveSyntax], self._field(0))
    @property
    def tag_bo(self) -> tuple[RecoveredField[TenseModalSyntax] | None, RecoveredField[Token]] | None:
        'The optional pair containing an optional shared tense-modal child followed by a required `Bo` cmavo marker.'
        return cast(tuple[RecoveredField[TenseModalSyntax] | None, RecoveredField[Token]] | None, self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('IStandardParagraphStatementConnectiveSyntax is final')

@final
class ParagraphStandardStatementConnectiveSyntaxParagraphJoiConnective(_SyntaxNode):
    'Uses the `paragraph_joi_connective` product form, whose payload preserves `se`, `joi`, and `nai`.'
    __slots__ = ()
    _schema_id = 482
    __match_args__ = ('paragraph_joi_connective',)
    def __new__(cls, paragraph_joi_connective: RecoveredField[ParagraphJoiConnectiveSyntax]) -> ParagraphStandardStatementConnectiveSyntaxParagraphJoiConnective:
        return cls._from_fields((paragraph_joi_connective,))
    def __init__(self, paragraph_joi_connective: RecoveredField[ParagraphJoiConnectiveSyntax]) -> None:
        pass
    @property
    def paragraph_joi_connective(self) -> RecoveredField[ParagraphJoiConnectiveSyntax]:
        'Uses the `paragraph_joi_connective` product form, whose payload preserves `se`, `joi`, and `nai`.'
        return cast(RecoveredField[ParagraphJoiConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ParagraphStandardStatementConnectiveSyntaxParagraphJoiConnective is final')

@final
class ParagraphStandardStatementConnectiveSyntaxParagraphSimpleIntervalConnective(_SyntaxNode):
    'Uses the `paragraph_simple_interval_connective` product form, whose payload preserves `se`, `bihi`, and `nai`.'
    __slots__ = ()
    _schema_id = 483
    __match_args__ = ('paragraph_simple_interval_connective',)
    def __new__(cls, paragraph_simple_interval_connective: RecoveredField[ParagraphSimpleIntervalConnectiveSyntax]) -> ParagraphStandardStatementConnectiveSyntaxParagraphSimpleIntervalConnective:
        return cls._from_fields((paragraph_simple_interval_connective,))
    def __init__(self, paragraph_simple_interval_connective: RecoveredField[ParagraphSimpleIntervalConnectiveSyntax]) -> None:
        pass
    @property
    def paragraph_simple_interval_connective(self) -> RecoveredField[ParagraphSimpleIntervalConnectiveSyntax]:
        'Uses the `paragraph_simple_interval_connective` product form, whose payload preserves `se`, `bihi`, and `nai`.'
        return cast(RecoveredField[ParagraphSimpleIntervalConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ParagraphStandardStatementConnectiveSyntaxParagraphSimpleIntervalConnective is final')

@final
class ParagraphStandardStatementConnectiveSyntaxParagraphClosedIntervalConnective(_SyntaxNode):
    'Uses the `paragraph_closed_interval_connective` product form, whose payload preserves `left_interval`, `se`, `bihi`, `nai`, and `right_interval`.'
    __slots__ = ()
    _schema_id = 484
    __match_args__ = ('paragraph_closed_interval_connective',)
    def __new__(cls, paragraph_closed_interval_connective: RecoveredField[ParagraphClosedIntervalConnectiveSyntax]) -> ParagraphStandardStatementConnectiveSyntaxParagraphClosedIntervalConnective:
        return cls._from_fields((paragraph_closed_interval_connective,))
    def __init__(self, paragraph_closed_interval_connective: RecoveredField[ParagraphClosedIntervalConnectiveSyntax]) -> None:
        pass
    @property
    def paragraph_closed_interval_connective(self) -> RecoveredField[ParagraphClosedIntervalConnectiveSyntax]:
        'Uses the `paragraph_closed_interval_connective` product form, whose payload preserves `left_interval`, `se`, `bihi`, `nai`, and `right_interval`.'
        return cast(RecoveredField[ParagraphClosedIntervalConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ParagraphStandardStatementConnectiveSyntaxParagraphClosedIntervalConnective is final')

@final
class ParagraphStandardStatementConnectiveSyntaxParagraphJekConnective(_SyntaxNode):
    'Uses the `paragraph_jek_connective` product form, whose payload preserves `na`, `se`, `ja`, and `nai`.'
    __slots__ = ()
    _schema_id = 485
    __match_args__ = ('paragraph_jek_connective',)
    def __new__(cls, paragraph_jek_connective: RecoveredField[ParagraphJekConnectiveSyntax]) -> ParagraphStandardStatementConnectiveSyntaxParagraphJekConnective:
        return cls._from_fields((paragraph_jek_connective,))
    def __init__(self, paragraph_jek_connective: RecoveredField[ParagraphJekConnectiveSyntax]) -> None:
        pass
    @property
    def paragraph_jek_connective(self) -> RecoveredField[ParagraphJekConnectiveSyntax]:
        'Uses the `paragraph_jek_connective` product form, whose payload preserves `na`, `se`, `ja`, and `nai`.'
        return cast(RecoveredField[ParagraphJekConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ParagraphStandardStatementConnectiveSyntaxParagraphJekConnective is final')

ParagraphStandardStatementConnectiveSyntax: TypeAlias = ParagraphStandardStatementConnectiveSyntaxParagraphJoiConnective | ParagraphStandardStatementConnectiveSyntaxParagraphSimpleIntervalConnective | ParagraphStandardStatementConnectiveSyntaxParagraphClosedIntervalConnective | ParagraphStandardStatementConnectiveSyntaxParagraphJekConnective

@final
class ParagraphJekConnectiveSyntax(_SyntaxNode):
    'Product node for jek; preserves `na`, `se`, `ja`, and `nai` in source order.'
    __slots__ = ()
    _schema_id = 486
    __match_args__ = ('na', 'se', 'ja', 'nai')
    def __new__(cls, na: RecoveredField[Token] | None, se: RecoveredField[Token] | None, ja: RecoveredField[Token], nai: RecoveredField[Token] | None) -> ParagraphJekConnectiveSyntax:
        return cls._from_fields((na, se, ja, nai))
    def __init__(self, na: RecoveredField[Token] | None, se: RecoveredField[Token] | None, ja: RecoveredField[Token], nai: RecoveredField[Token] | None) -> None:
        pass
    @property
    def na(self) -> RecoveredField[Token] | None:
        'The optional na component.'
        return cast(RecoveredField[Token] | None, self._field(0))
    @property
    def se(self) -> RecoveredField[Token] | None:
        'The optional se component.'
        return cast(RecoveredField[Token] | None, self._field(1))
    @property
    def ja(self) -> RecoveredField[Token]:
        'A word from selmaho `Ja`.'
        return cast(RecoveredField[Token], self._field(2))
    @property
    def nai(self) -> RecoveredField[Token] | None:
        'The optional `Nai` cmavo marker.'
        return cast(RecoveredField[Token] | None, self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('ParagraphJekConnectiveSyntax is final')

@final
class ParagraphJoiConnectiveSyntax(_SyntaxNode):
    'Product node for joik; preserves `se`, `joi`, and `nai` in source order.'
    __slots__ = ()
    _schema_id = 487
    __match_args__ = ('se', 'joi', 'nai')
    def __new__(cls, se: RecoveredField[Token] | None, joi: RecoveredField[Token], nai: RecoveredField[Token] | None) -> ParagraphJoiConnectiveSyntax:
        return cls._from_fields((se, joi, nai))
    def __init__(self, se: RecoveredField[Token] | None, joi: RecoveredField[Token], nai: RecoveredField[Token] | None) -> None:
        pass
    @property
    def se(self) -> RecoveredField[Token] | None:
        'The optional se component.'
        return cast(RecoveredField[Token] | None, self._field(0))
    @property
    def joi(self) -> RecoveredField[Token]:
        'A word from selmaho `Joi`.'
        return cast(RecoveredField[Token], self._field(1))
    @property
    def nai(self) -> RecoveredField[Token] | None:
        'The optional `Nai` cmavo marker.'
        return cast(RecoveredField[Token] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('ParagraphJoiConnectiveSyntax is final')

@final
class ParagraphSimpleIntervalConnectiveSyntax(_SyntaxNode):
    'Product node for interval; preserves `se`, `bihi`, and `nai` in source order.'
    __slots__ = ()
    _schema_id = 488
    __match_args__ = ('se', 'bihi', 'nai')
    def __new__(cls, se: RecoveredField[Token] | None, bihi: RecoveredField[Token], nai: RecoveredField[Token] | None) -> ParagraphSimpleIntervalConnectiveSyntax:
        return cls._from_fields((se, bihi, nai))
    def __init__(self, se: RecoveredField[Token] | None, bihi: RecoveredField[Token], nai: RecoveredField[Token] | None) -> None:
        pass
    @property
    def se(self) -> RecoveredField[Token] | None:
        'The optional se component.'
        return cast(RecoveredField[Token] | None, self._field(0))
    @property
    def bihi(self) -> RecoveredField[Token]:
        'A word from selmaho `Bihi`.'
        return cast(RecoveredField[Token], self._field(1))
    @property
    def nai(self) -> RecoveredField[Token] | None:
        'The optional `Nai` cmavo marker.'
        return cast(RecoveredField[Token] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('ParagraphSimpleIntervalConnectiveSyntax is final')

@final
class ParagraphClosedIntervalConnectiveSyntax(_SyntaxNode):
    'Product node for interval; preserves `left_interval`, `se`, `bihi`, `nai`, and `right_interval` in source order.'
    __slots__ = ()
    _schema_id = 489
    __match_args__ = ('left_interval', 'se', 'bihi', 'nai', 'right_interval')
    def __new__(cls, left_interval: RecoveredField[Token], se: RecoveredField[Token] | None, bihi: RecoveredField[Token], nai: RecoveredField[Token] | None, right_interval: RecoveredField[Token]) -> ParagraphClosedIntervalConnectiveSyntax:
        return cls._from_fields((left_interval, se, bihi, nai, right_interval))
    def __init__(self, left_interval: RecoveredField[Token], se: RecoveredField[Token] | None, bihi: RecoveredField[Token], nai: RecoveredField[Token] | None, right_interval: RecoveredField[Token]) -> None:
        pass
    @property
    def left_interval(self) -> RecoveredField[Token]:
        'A word from selmaho `Gaho`.'
        return cast(RecoveredField[Token], self._field(0))
    @property
    def se(self) -> RecoveredField[Token] | None:
        'The optional se component.'
        return cast(RecoveredField[Token] | None, self._field(1))
    @property
    def bihi(self) -> RecoveredField[Token]:
        'A word from selmaho `Bihi`.'
        return cast(RecoveredField[Token], self._field(2))
    @property
    def nai(self) -> RecoveredField[Token] | None:
        'The optional `Nai` cmavo marker.'
        return cast(RecoveredField[Token] | None, self._field(3))
    @property
    def right_interval(self) -> RecoveredField[Token]:
        'A word from selmaho `Gaho`.'
        return cast(RecoveredField[Token], self._field(4))
    def __init_subclass__(cls) -> None:
        raise TypeError('ParagraphClosedIntervalConnectiveSyntax is final')

@final
class ITagBoParagraphStatementConnectiveSyntax(_SyntaxNode):
    'Product node for statement connective; preserves `tense_modal` and `bo` in source order.'
    __slots__ = ()
    _schema_id = 490
    __match_args__ = ('tense_modal', 'bo')
    def __new__(cls, tense_modal: RecoveredField[TenseModalSyntax] | None, bo: RecoveredField[Token]) -> ITagBoParagraphStatementConnectiveSyntax:
        return cls._from_fields((tense_modal, bo))
    def __init__(self, tense_modal: RecoveredField[TenseModalSyntax] | None, bo: RecoveredField[Token]) -> None:
        pass
    @property
    def tense_modal(self) -> RecoveredField[TenseModalSyntax] | None:
        'The optional tense modal component.'
        return cast(RecoveredField[TenseModalSyntax] | None, self._field(0))
    @property
    def bo(self) -> RecoveredField[Token]:
        'The `Bo` cmavo marker.'
        return cast(RecoveredField[Token], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ITagBoParagraphStatementConnectiveSyntax is final')

@final
class ITagBoStatementConnectiveSyntax(_SyntaxNode):
    'Product node for statement connective; preserves `tense_modal` and `bo` in source order.'
    __slots__ = ()
    _schema_id = 491
    __match_args__ = ('tense_modal', 'bo')
    def __new__(cls, tense_modal: RecoveredField[TenseModalSyntax] | None, bo: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> ITagBoStatementConnectiveSyntax:
        return cls._from_fields((tense_modal, bo))
    def __init__(self, tense_modal: RecoveredField[TenseModalSyntax] | None, bo: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def tense_modal(self) -> RecoveredField[TenseModalSyntax] | None:
        'The optional tense modal component.'
        return cast(RecoveredField[TenseModalSyntax] | None, self._field(0))
    @property
    def bo(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Bo` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ITagBoStatementConnectiveSyntax is final')

@final
class CeheConnectiveSyntax(_SyntaxNode):
    'Product node for termset connective; preserves `cehe` and `nai` in source order.'
    __slots__ = ()
    _schema_id = 492
    __match_args__ = ('cehe', 'nai')
    def __new__(cls, cehe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> CeheConnectiveSyntax:
        return cls._from_fields((cehe, nai))
    def __init__(self, cehe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def cehe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Cehe` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def nai(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Nai` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('CeheConnectiveSyntax is final')

@final
class GihekConnectiveSyntax(_SyntaxNode):
    'Product node for gihek; preserves `na`, `se`, `giha`, and `nai` in source order.'
    __slots__ = ()
    _schema_id = 493
    __match_args__ = ('na', 'se', 'giha', 'nai')
    def __new__(cls, na: RecoveredField[Token] | None, se: RecoveredField[Token] | None, giha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> GihekConnectiveSyntax:
        return cls._from_fields((na, se, giha, nai))
    def __init__(self, na: RecoveredField[Token] | None, se: RecoveredField[Token] | None, giha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def na(self) -> RecoveredField[Token] | None:
        'The optional na component.'
        return cast(RecoveredField[Token] | None, self._field(0))
    @property
    def se(self) -> RecoveredField[Token] | None:
        'The optional se component.'
        return cast(RecoveredField[Token] | None, self._field(1))
    @property
    def giha(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Giha`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(2))
    @property
    def nai(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Nai` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('GihekConnectiveSyntax is final')

@final
class GuhekConnectiveSyntax(_SyntaxNode):
    'Product node for forethought selbri connective; preserves `nahe`, `se`, `guha`, and `nai` in source order.'
    __slots__ = ()
    _schema_id = 494
    __match_args__ = ('nahe', 'se', 'guha', 'nai')
    def __new__(cls, nahe: RecoveredField[Token] | None, se: RecoveredField[Token] | None, guha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> GuhekConnectiveSyntax:
        return cls._from_fields((nahe, se, guha, nai))
    def __init__(self, nahe: RecoveredField[Token] | None, se: RecoveredField[Token] | None, guha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def nahe(self) -> RecoveredField[Token] | None:
        'The optional nahe component.'
        return cast(RecoveredField[Token] | None, self._field(0))
    @property
    def se(self) -> RecoveredField[Token] | None:
        'The optional se component.'
        return cast(RecoveredField[Token] | None, self._field(1))
    @property
    def guha(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Guha`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(2))
    @property
    def nai(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Nai` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('GuhekConnectiveSyntax is final')

@final
class BridiTailConnectiveSyntaxGihekConnective(_SyntaxNode):
    'Uses the `gihek_connective` product form, whose payload preserves `na`, `se`, `giha`, and `nai`.'
    __slots__ = ()
    _schema_id = 495
    __match_args__ = ('gihek_connective',)
    def __new__(cls, gihek_connective: RecoveredField[GihekConnectiveSyntax]) -> BridiTailConnectiveSyntaxGihekConnective:
        return cls._from_fields((gihek_connective,))
    def __init__(self, gihek_connective: RecoveredField[GihekConnectiveSyntax]) -> None:
        pass
    @property
    def gihek_connective(self) -> RecoveredField[GihekConnectiveSyntax]:
        'Uses the `gihek_connective` product form, whose payload preserves `na`, `se`, `giha`, and `nai`.'
        return cast(RecoveredField[GihekConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('BridiTailConnectiveSyntaxGihekConnective is final')

@final
class BridiTailConnectiveSyntaxRelationConnectiveAsBridiTail(_SyntaxNode):
    'Uses the `relation_connective_as_bridi_tail` product form, whose payload preserves `connective`.'
    __slots__ = ()
    _schema_id = 496
    __match_args__ = ('relation_connective_as_bridi_tail',)
    def __new__(cls, relation_connective_as_bridi_tail: RecoveredField[RelationConnectiveAsBridiTailSyntax]) -> BridiTailConnectiveSyntaxRelationConnectiveAsBridiTail:
        return cls._from_fields((relation_connective_as_bridi_tail,))
    def __init__(self, relation_connective_as_bridi_tail: RecoveredField[RelationConnectiveAsBridiTailSyntax]) -> None:
        pass
    @property
    def relation_connective_as_bridi_tail(self) -> RecoveredField[RelationConnectiveAsBridiTailSyntax]:
        'Uses the `relation_connective_as_bridi_tail` product form, whose payload preserves `connective`.'
        return cast(RecoveredField[RelationConnectiveAsBridiTailSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('BridiTailConnectiveSyntaxRelationConnectiveAsBridiTail is final')

BridiTailConnectiveSyntax: TypeAlias = BridiTailConnectiveSyntaxGihekConnective | BridiTailConnectiveSyntaxRelationConnectiveAsBridiTail

@final
class RelationConnectiveAsBridiTailSyntax(_SyntaxNode):
    'Transparent product node for bridi tail connective; preserves the `connective` component.'
    __slots__ = ()
    _schema_id = 497
    __match_args__ = ('connective',)
    def __new__(cls, connective: RecoveredField[RelationAfterthoughtConnectiveSyntax]) -> RelationConnectiveAsBridiTailSyntax:
        return cls._from_fields((connective,))
    def __init__(self, connective: RecoveredField[RelationAfterthoughtConnectiveSyntax]) -> None:
        pass
    @property
    def connective(self) -> RecoveredField[RelationAfterthoughtConnectiveSyntax]:
        'The shared connective child syntax node.'
        return cast(RecoveredField[RelationAfterthoughtConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('RelationConnectiveAsBridiTailSyntax is final')

@final
class ModalForethoughtConnectiveSyntaxGaForethoughtConnective(_SyntaxNode):
    'Uses the `ga_forethought_connective` product form, whose payload preserves `se`, `ga`, and `nai`.'
    __slots__ = ()
    _schema_id = 498
    __match_args__ = ('ga_forethought_connective',)
    def __new__(cls, ga_forethought_connective: RecoveredField[GaForethoughtConnectiveSyntax]) -> ModalForethoughtConnectiveSyntaxGaForethoughtConnective:
        return cls._from_fields((ga_forethought_connective,))
    def __init__(self, ga_forethought_connective: RecoveredField[GaForethoughtConnectiveSyntax]) -> None:
        pass
    @property
    def ga_forethought_connective(self) -> RecoveredField[GaForethoughtConnectiveSyntax]:
        'Uses the `ga_forethought_connective` product form, whose payload preserves `se`, `ga`, and `nai`.'
        return cast(RecoveredField[GaForethoughtConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ModalForethoughtConnectiveSyntaxGaForethoughtConnective is final')

@final
class ModalForethoughtConnectiveSyntaxJoikJekGiForethoughtConnective(_SyntaxNode):
    'Uses the `joik_jek_gi_forethought_connective` product form, whose payload preserves `connective`, `gi`, and `bo`.'
    __slots__ = ()
    _schema_id = 499
    __match_args__ = ('joik_jek_gi_forethought_connective',)
    def __new__(cls, joik_jek_gi_forethought_connective: RecoveredField[JoikJekGiForethoughtConnectiveSyntax]) -> ModalForethoughtConnectiveSyntaxJoikJekGiForethoughtConnective:
        return cls._from_fields((joik_jek_gi_forethought_connective,))
    def __init__(self, joik_jek_gi_forethought_connective: RecoveredField[JoikJekGiForethoughtConnectiveSyntax]) -> None:
        pass
    @property
    def joik_jek_gi_forethought_connective(self) -> RecoveredField[JoikJekGiForethoughtConnectiveSyntax]:
        'Uses the `joik_jek_gi_forethought_connective` product form, whose payload preserves `connective`, `gi`, and `bo`.'
        return cast(RecoveredField[JoikJekGiForethoughtConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ModalForethoughtConnectiveSyntaxJoikJekGiForethoughtConnective is final')

@final
class ModalForethoughtConnectiveSyntaxJekGiForethoughtConnective(_SyntaxNode):
    'Uses the `jek_gi_forethought_connective` product form, whose payload preserves `na`, `se`, `ja`, and 3 other fields.'
    __slots__ = ()
    _schema_id = 500
    __match_args__ = ('jek_gi_forethought_connective',)
    def __new__(cls, jek_gi_forethought_connective: RecoveredField[JekGiForethoughtConnectiveSyntax]) -> ModalForethoughtConnectiveSyntaxJekGiForethoughtConnective:
        return cls._from_fields((jek_gi_forethought_connective,))
    def __init__(self, jek_gi_forethought_connective: RecoveredField[JekGiForethoughtConnectiveSyntax]) -> None:
        pass
    @property
    def jek_gi_forethought_connective(self) -> RecoveredField[JekGiForethoughtConnectiveSyntax]:
        'Uses the `jek_gi_forethought_connective` product form, whose payload preserves `na`, `se`, `ja`, and 3 other fields.'
        return cast(RecoveredField[JekGiForethoughtConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ModalForethoughtConnectiveSyntaxJekGiForethoughtConnective is final')

@final
class ModalForethoughtConnectiveSyntaxModalGiForethoughtConnective(_SyntaxNode):
    'Uses the `modal_gi_forethought_connective` product form, whose payload preserves `tense_modal`, `gi`, and `bo`.'
    __slots__ = ()
    _schema_id = 501
    __match_args__ = ('modal_gi_forethought_connective',)
    def __new__(cls, modal_gi_forethought_connective: RecoveredField[ModalGiForethoughtConnectiveSyntax]) -> ModalForethoughtConnectiveSyntaxModalGiForethoughtConnective:
        return cls._from_fields((modal_gi_forethought_connective,))
    def __init__(self, modal_gi_forethought_connective: RecoveredField[ModalGiForethoughtConnectiveSyntax]) -> None:
        pass
    @property
    def modal_gi_forethought_connective(self) -> RecoveredField[ModalGiForethoughtConnectiveSyntax]:
        'Uses the `modal_gi_forethought_connective` product form, whose payload preserves `tense_modal`, `gi`, and `bo`.'
        return cast(RecoveredField[ModalGiForethoughtConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ModalForethoughtConnectiveSyntaxModalGiForethoughtConnective is final')

@final
class ModalForethoughtConnectiveSyntaxZantufaInitialGiForethoughtConnective(_SyntaxNode):
    'Uses the `zantufa_initial_gi_forethought_connective` product form, whose payload preserves `gi`, `tail`, and `bo`.'
    __slots__ = ()
    _schema_id = 502
    __match_args__ = ('zantufa_initial_gi_forethought_connective',)
    def __new__(cls, zantufa_initial_gi_forethought_connective: RecoveredField[ZantufaInitialGiForethoughtConnectiveSyntax]) -> ModalForethoughtConnectiveSyntaxZantufaInitialGiForethoughtConnective:
        return cls._from_fields((zantufa_initial_gi_forethought_connective,))
    def __init__(self, zantufa_initial_gi_forethought_connective: RecoveredField[ZantufaInitialGiForethoughtConnectiveSyntax]) -> None:
        pass
    @property
    def zantufa_initial_gi_forethought_connective(self) -> RecoveredField[ZantufaInitialGiForethoughtConnectiveSyntax]:
        'Uses the `zantufa_initial_gi_forethought_connective` product form, whose payload preserves `gi`, `tail`, and `bo`.'
        return cast(RecoveredField[ZantufaInitialGiForethoughtConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ModalForethoughtConnectiveSyntaxZantufaInitialGiForethoughtConnective is final')

ModalForethoughtConnectiveSyntax: TypeAlias = ModalForethoughtConnectiveSyntaxGaForethoughtConnective | ModalForethoughtConnectiveSyntaxJoikJekGiForethoughtConnective | ModalForethoughtConnectiveSyntaxJekGiForethoughtConnective | ModalForethoughtConnectiveSyntaxModalGiForethoughtConnective | ModalForethoughtConnectiveSyntaxZantufaInitialGiForethoughtConnective

@final
class GaForethoughtConnectiveSyntax(_SyntaxNode):
    'Product node for forethought connective; preserves `se`, `ga`, and `nai` in source order.'
    __slots__ = ()
    _schema_id = 503
    __match_args__ = ('se', 'ga', 'nai')
    def __new__(cls, se: RecoveredField[Token] | None, ga: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> GaForethoughtConnectiveSyntax:
        return cls._from_fields((se, ga, nai))
    def __init__(self, se: RecoveredField[Token] | None, ga: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def se(self) -> RecoveredField[Token] | None:
        'The optional se component.'
        return cast(RecoveredField[Token] | None, self._field(0))
    @property
    def ga(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Ga`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(1))
    @property
    def nai(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Nai` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('GaForethoughtConnectiveSyntax is final')

@final
class ZantufaInitialGiForethoughtConnectiveSyntax(_SyntaxNode):
    'Product node for forethought connective; preserves `gi`, `tail`, and `bo` in source order.'
    __slots__ = ()
    _schema_id = 504
    __match_args__ = ('gi', 'tail', 'bo')
    def __new__(cls, gi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], tail: RecoveredField[StandardStatementConnectiveSyntax], bo: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> ZantufaInitialGiForethoughtConnectiveSyntax:
        return cls._from_fields((gi, tail, bo))
    def __init__(self, gi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], tail: RecoveredField[StandardStatementConnectiveSyntax], bo: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def gi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Gi` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def tail(self) -> RecoveredField[StandardStatementConnectiveSyntax]:
        'The shared tail child syntax node.'
        return cast(RecoveredField[StandardStatementConnectiveSyntax], self._field(1))
    @property
    def bo(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Bo` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaInitialGiForethoughtConnectiveSyntax is final')

@final
class JoikJekGiForethoughtConnectiveSyntax(_SyntaxNode):
    'Product node for forethought connective; preserves `connective`, `gi`, and `bo` in source order.'
    __slots__ = ()
    _schema_id = 505
    __match_args__ = ('connective', 'gi', 'bo')
    def __new__(cls, connective: RecoveredField[JoikConnectiveSyntax], gi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], bo: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> JoikJekGiForethoughtConnectiveSyntax:
        return cls._from_fields((connective, gi, bo))
    def __init__(self, connective: RecoveredField[JoikConnectiveSyntax], gi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], bo: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def connective(self) -> RecoveredField[JoikConnectiveSyntax]:
        'The shared connective child syntax node.'
        return cast(RecoveredField[JoikConnectiveSyntax], self._field(0))
    @property
    def gi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Gi` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(1))
    @property
    def bo(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Bo` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('JoikJekGiForethoughtConnectiveSyntax is final')

@final
class JekGiForethoughtConnectiveSyntax(_SyntaxNode):
    'Product node for forethought connective; preserves `na`, `se`, `ja`, and 3 other fields in source order.'
    __slots__ = ()
    _schema_id = 506
    __match_args__ = ('na', 'se', 'ja', 'nai', 'gi', 'bo')
    def __new__(cls, na: RecoveredField[Token] | None, se: RecoveredField[Token] | None, ja: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, gi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], bo: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> JekGiForethoughtConnectiveSyntax:
        return cls._from_fields((na, se, ja, nai, gi, bo))
    def __init__(self, na: RecoveredField[Token] | None, se: RecoveredField[Token] | None, ja: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, gi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], bo: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def na(self) -> RecoveredField[Token] | None:
        'The optional na component.'
        return cast(RecoveredField[Token] | None, self._field(0))
    @property
    def se(self) -> RecoveredField[Token] | None:
        'The optional se component.'
        return cast(RecoveredField[Token] | None, self._field(1))
    @property
    def ja(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Ja`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(2))
    @property
    def nai(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Nai` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(3))
    @property
    def gi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Gi` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(4))
    @property
    def bo(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Bo` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(5))
    def __init_subclass__(cls) -> None:
        raise TypeError('JekGiForethoughtConnectiveSyntax is final')

@final
class ModalGiForethoughtConnectiveSyntax(_SyntaxNode):
    'Product node for forethought connective; preserves `tense_modal`, `gi`, and `bo` in source order.'
    __slots__ = ()
    _schema_id = 507
    __match_args__ = ('tense_modal', 'gi', 'bo')
    def __new__(cls, tense_modal: RecoveredField[TenseModalSyntax], gi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], bo: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> ModalGiForethoughtConnectiveSyntax:
        return cls._from_fields((tense_modal, gi, bo))
    def __init__(self, tense_modal: RecoveredField[TenseModalSyntax], gi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], bo: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def tense_modal(self) -> RecoveredField[TenseModalSyntax]:
        'The shared tense modal child syntax node.'
        return cast(RecoveredField[TenseModalSyntax], self._field(0))
    @property
    def gi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Gi` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(1))
    @property
    def bo(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Bo` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('ModalGiForethoughtConnectiveSyntax is final')

@final
class GikConnectiveSyntax(_SyntaxNode):
    'Product node for forethought connective; preserves `gi` and `nai` in source order.'
    __slots__ = ()
    _schema_id = 508
    __match_args__ = ('gi', 'nai')
    def __new__(cls, gi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> GikConnectiveSyntax:
        return cls._from_fields((gi, nai))
    def __init__(self, gi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def gi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Gi` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def nai(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Nai` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('GikConnectiveSyntax is final')

@final
class ZantufaExtraGikConnectiveSyntax(_SyntaxNode):
    'Transparent product node for forethought connective; preserves the `gi` component.'
    __slots__ = ()
    _schema_id = 509
    __match_args__ = ('gi',)
    def __new__(cls, gi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> ZantufaExtraGikConnectiveSyntax:
        return cls._from_fields((gi,))
    def __init__(self, gi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def gi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Gi` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaExtraGikConnectiveSyntax is final')

@final
class TenseModalSyntax(_SyntaxNode):
    'Transparent product node for tag; preserves the `body` component.'
    __slots__ = ()
    _schema_id = 510
    __match_args__ = ('body',)
    def __new__(cls, body: RecoveredField[TenseModalBodySyntax]) -> TenseModalSyntax:
        return cls._from_fields((body,))
    def __init__(self, body: RecoveredField[TenseModalBodySyntax]) -> None:
        pass
    @property
    def body(self) -> RecoveredField[TenseModalBodySyntax]:
        'The `tense_modal_body` grammar result in the `body` structural role of the `tense_modal` production.'
        return cast(RecoveredField[TenseModalBodySyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TenseModalSyntax is final')

@final
class TenseModalBodySyntaxConnectedTenseModal(_SyntaxNode):
    'Uses the `connected_tense_modal` product form, whose payload preserves `first` and `continuations`.'
    __slots__ = ()
    _schema_id = 511
    __match_args__ = ('connected_tense_modal',)
    def __new__(cls, connected_tense_modal: RecoveredField[ConnectedTenseModalSyntax]) -> TenseModalBodySyntaxConnectedTenseModal:
        return cls._from_fields((connected_tense_modal,))
    def __init__(self, connected_tense_modal: RecoveredField[ConnectedTenseModalSyntax]) -> None:
        pass
    @property
    def connected_tense_modal(self) -> RecoveredField[ConnectedTenseModalSyntax]:
        'Uses the `connected_tense_modal` product form, whose payload preserves `first` and `continuations`.'
        return cast(RecoveredField[ConnectedTenseModalSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TenseModalBodySyntaxConnectedTenseModal is final')

@final
class TenseModalBodySyntaxTenseModalAtom(_SyntaxNode):
    'Uses the nested `tense_modal_atom` sum form and preserves its selected alternative.'
    __slots__ = ()
    _schema_id = 512
    __match_args__ = ('tense_modal_atom',)
    def __new__(cls, tense_modal_atom: RecoveredField[TenseModalAtomSyntax]) -> TenseModalBodySyntaxTenseModalAtom:
        return cls._from_fields((tense_modal_atom,))
    def __init__(self, tense_modal_atom: RecoveredField[TenseModalAtomSyntax]) -> None:
        pass
    @property
    def tense_modal_atom(self) -> RecoveredField[TenseModalAtomSyntax]:
        'Uses the nested `tense_modal_atom` sum form and preserves its selected alternative.'
        return cast(RecoveredField[TenseModalAtomSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TenseModalBodySyntaxTenseModalAtom is final')

TenseModalBodySyntax: TypeAlias = TenseModalBodySyntaxConnectedTenseModal | TenseModalBodySyntaxTenseModalAtom

@final
class ConnectedTenseModalSyntax(_SyntaxNode):
    'Product node for connected tag; preserves `first` and `continuations` in source order.'
    __slots__ = ()
    _schema_id = 513
    __match_args__ = ('first', 'continuations')
    def __new__(cls, first: RecoveredField[TenseModalAtomSyntax], continuations: Sequence[RecoveredField[ConnectedTenseModalContinuationSyntax]]) -> ConnectedTenseModalSyntax:
        return cls._from_fields((first, continuations))
    def __init__(self, first: RecoveredField[TenseModalAtomSyntax], continuations: Sequence[RecoveredField[ConnectedTenseModalContinuationSyntax]]) -> None:
        pass
    @property
    def first(self) -> RecoveredField[TenseModalAtomSyntax]:
        'The shared first child syntax node.'
        return cast(RecoveredField[TenseModalAtomSyntax], self._field(0))
    @property
    def continuations(self) -> tuple[RecoveredField[ConnectedTenseModalContinuationSyntax], ...]:
        'Non-empty ordered sequence of continuations components.'
        return cast(tuple[RecoveredField[ConnectedTenseModalContinuationSyntax], ...], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ConnectedTenseModalSyntax is final')

@final
class ConnectedTenseModalContinuationSyntax(_SyntaxNode):
    'Product node for connected tag continuation; preserves `connective` and `tense_modal` in source order.'
    __slots__ = ()
    _schema_id = 514
    __match_args__ = ('connective', 'tense_modal')
    def __new__(cls, connective: RecoveredField[TenseModalConnectiveSyntax], tense_modal: RecoveredField[TenseModalAtomSyntax]) -> ConnectedTenseModalContinuationSyntax:
        return cls._from_fields((connective, tense_modal))
    def __init__(self, connective: RecoveredField[TenseModalConnectiveSyntax], tense_modal: RecoveredField[TenseModalAtomSyntax]) -> None:
        pass
    @property
    def connective(self) -> RecoveredField[TenseModalConnectiveSyntax]:
        'The `tense_modal_connective` connective joining the adjacent constituents of the `connected_tense_modal_continuation` production.'
        return cast(RecoveredField[TenseModalConnectiveSyntax], self._field(0))
    @property
    def tense_modal(self) -> RecoveredField[TenseModalAtomSyntax]:
        'The shared tense modal child syntax node.'
        return cast(RecoveredField[TenseModalAtomSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ConnectedTenseModalContinuationSyntax is final')

@final
class TenseModalConnectiveSyntaxJoikConnective(_SyntaxNode):
    'Uses the nested `joik_connective` sum form and preserves its selected alternative.'
    __slots__ = ()
    _schema_id = 515
    __match_args__ = ('joik_connective',)
    def __new__(cls, joik_connective: RecoveredField[JoikConnectiveSyntax]) -> TenseModalConnectiveSyntaxJoikConnective:
        return cls._from_fields((joik_connective,))
    def __init__(self, joik_connective: RecoveredField[JoikConnectiveSyntax]) -> None:
        pass
    @property
    def joik_connective(self) -> RecoveredField[JoikConnectiveSyntax]:
        'Uses the nested `joik_connective` sum form and preserves its selected alternative.'
        return cast(RecoveredField[JoikConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TenseModalConnectiveSyntaxJoikConnective is final')

@final
class TenseModalConnectiveSyntaxJekConnective(_SyntaxNode):
    'Uses the `jek_connective` product form, whose payload preserves `na`, `se`, `ja`, and `nai`.'
    __slots__ = ()
    _schema_id = 516
    __match_args__ = ('jek_connective',)
    def __new__(cls, jek_connective: RecoveredField[JekConnectiveSyntax]) -> TenseModalConnectiveSyntaxJekConnective:
        return cls._from_fields((jek_connective,))
    def __init__(self, jek_connective: RecoveredField[JekConnectiveSyntax]) -> None:
        pass
    @property
    def jek_connective(self) -> RecoveredField[JekConnectiveSyntax]:
        'Uses the `jek_connective` product form, whose payload preserves `na`, `se`, `ja`, and `nai`.'
        return cast(RecoveredField[JekConnectiveSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TenseModalConnectiveSyntaxJekConnective is final')

TenseModalConnectiveSyntax: TypeAlias = TenseModalConnectiveSyntaxJoikConnective | TenseModalConnectiveSyntaxJekConnective

@final
class TenseModalAtomSyntaxCompositeTense(_SyntaxNode):
    'Uses the nested `composite_tense` sum form and preserves its selected alternative.'
    __slots__ = ()
    _schema_id = 517
    __match_args__ = ('composite_tense',)
    def __new__(cls, composite_tense: RecoveredField[CompositeTenseSyntax]) -> TenseModalAtomSyntaxCompositeTense:
        return cls._from_fields((composite_tense,))
    def __init__(self, composite_tense: RecoveredField[CompositeTenseSyntax]) -> None:
        pass
    @property
    def composite_tense(self) -> RecoveredField[CompositeTenseSyntax]:
        'Uses the nested `composite_tense` sum form and preserves its selected alternative.'
        return cast(RecoveredField[CompositeTenseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TenseModalAtomSyntaxCompositeTense is final')

@final
class TenseModalAtomSyntaxFihoTense(_SyntaxNode):
    'Uses the `fiho_tense` product form, whose payload preserves `fiho`, `selbri`, and `fehu`.'
    __slots__ = ()
    _schema_id = 518
    __match_args__ = ('fiho_tense',)
    def __new__(cls, fiho_tense: RecoveredField[FihoTenseSyntax]) -> TenseModalAtomSyntaxFihoTense:
        return cls._from_fields((fiho_tense,))
    def __init__(self, fiho_tense: RecoveredField[FihoTenseSyntax]) -> None:
        pass
    @property
    def fiho_tense(self) -> RecoveredField[FihoTenseSyntax]:
        'Uses the `fiho_tense` product form, whose payload preserves `fiho`, `selbri`, and `fehu`.'
        return cast(RecoveredField[FihoTenseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TenseModalAtomSyntaxFihoTense is final')

@final
class TenseModalAtomSyntaxModalTense(_SyntaxNode):
    'Uses the `modal_tense` product form, whose payload preserves `nahe`, `se`, `bai`, `nai`, and `ki`.'
    __slots__ = ()
    _schema_id = 519
    __match_args__ = ('modal_tense',)
    def __new__(cls, modal_tense: RecoveredField[ModalTenseSyntax]) -> TenseModalAtomSyntaxModalTense:
        return cls._from_fields((modal_tense,))
    def __init__(self, modal_tense: RecoveredField[ModalTenseSyntax]) -> None:
        pass
    @property
    def modal_tense(self) -> RecoveredField[ModalTenseSyntax]:
        'Uses the `modal_tense` product form, whose payload preserves `nahe`, `se`, `bai`, `nai`, and `ki`.'
        return cast(RecoveredField[ModalTenseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TenseModalAtomSyntaxModalTense is final')

@final
class TenseModalAtomSyntaxNaheSeFlatPrefixedTense(_SyntaxNode):
    'Uses the `nahe_se_flat_prefixed_tense` product form, whose payload preserves `nahe`, `se`, and `atom`.'
    __slots__ = ()
    _schema_id = 520
    __match_args__ = ('nahe_se_flat_prefixed_tense',)
    def __new__(cls, nahe_se_flat_prefixed_tense: RecoveredField[NaheSeFlatPrefixedTenseSyntax]) -> TenseModalAtomSyntaxNaheSeFlatPrefixedTense:
        return cls._from_fields((nahe_se_flat_prefixed_tense,))
    def __init__(self, nahe_se_flat_prefixed_tense: RecoveredField[NaheSeFlatPrefixedTenseSyntax]) -> None:
        pass
    @property
    def nahe_se_flat_prefixed_tense(self) -> RecoveredField[NaheSeFlatPrefixedTenseSyntax]:
        'Uses the `nahe_se_flat_prefixed_tense` product form, whose payload preserves `nahe`, `se`, and `atom`.'
        return cast(RecoveredField[NaheSeFlatPrefixedTenseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TenseModalAtomSyntaxNaheSeFlatPrefixedTense is final')

@final
class TenseModalAtomSyntaxSeFlatPrefixedTense(_SyntaxNode):
    'Uses the `se_flat_prefixed_tense` product form, whose payload preserves `se` and `atom`.'
    __slots__ = ()
    _schema_id = 521
    __match_args__ = ('se_flat_prefixed_tense',)
    def __new__(cls, se_flat_prefixed_tense: RecoveredField[SeFlatPrefixedTenseSyntax]) -> TenseModalAtomSyntaxSeFlatPrefixedTense:
        return cls._from_fields((se_flat_prefixed_tense,))
    def __init__(self, se_flat_prefixed_tense: RecoveredField[SeFlatPrefixedTenseSyntax]) -> None:
        pass
    @property
    def se_flat_prefixed_tense(self) -> RecoveredField[SeFlatPrefixedTenseSyntax]:
        'Uses the `se_flat_prefixed_tense` product form, whose payload preserves `se` and `atom`.'
        return cast(RecoveredField[SeFlatPrefixedTenseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TenseModalAtomSyntaxSeFlatPrefixedTense is final')

@final
class TenseModalAtomSyntaxFaFlatTagTense(_SyntaxNode):
    'Uses the `fa_flat_tag_tense` product form, whose payload preserves `fa`.'
    __slots__ = ()
    _schema_id = 522
    __match_args__ = ('fa_flat_tag_tense',)
    def __new__(cls, fa_flat_tag_tense: RecoveredField[FaFlatTagTenseSyntax]) -> TenseModalAtomSyntaxFaFlatTagTense:
        return cls._from_fields((fa_flat_tag_tense,))
    def __init__(self, fa_flat_tag_tense: RecoveredField[FaFlatTagTenseSyntax]) -> None:
        pass
    @property
    def fa_flat_tag_tense(self) -> RecoveredField[FaFlatTagTenseSyntax]:
        'Uses the `fa_flat_tag_tense` product form, whose payload preserves `fa`.'
        return cast(RecoveredField[FaFlatTagTenseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TenseModalAtomSyntaxFaFlatTagTense is final')

@final
class TenseModalAtomSyntaxZantufaRecursiveTagTense(_SyntaxNode):
    'Uses the `zantufa_recursive_tag_tense` product form, whose payload preserves `first_prefix`, `additional_prefixes`, and `atom`.'
    __slots__ = ()
    _schema_id = 523
    __match_args__ = ('zantufa_recursive_tag_tense',)
    def __new__(cls, zantufa_recursive_tag_tense: RecoveredField[ZantufaRecursiveTagTenseSyntax]) -> TenseModalAtomSyntaxZantufaRecursiveTagTense:
        return cls._from_fields((zantufa_recursive_tag_tense,))
    def __init__(self, zantufa_recursive_tag_tense: RecoveredField[ZantufaRecursiveTagTenseSyntax]) -> None:
        pass
    @property
    def zantufa_recursive_tag_tense(self) -> RecoveredField[ZantufaRecursiveTagTenseSyntax]:
        'Uses the `zantufa_recursive_tag_tense` product form, whose payload preserves `first_prefix`, `additional_prefixes`, and `atom`.'
        return cast(RecoveredField[ZantufaRecursiveTagTenseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TenseModalAtomSyntaxZantufaRecursiveTagTense is final')

@final
class TenseModalAtomSyntaxStickyTense(_SyntaxNode):
    'Uses the `sticky_tense` product form, whose payload preserves `ki`.'
    __slots__ = ()
    _schema_id = 524
    __match_args__ = ('sticky_tense',)
    def __new__(cls, sticky_tense: RecoveredField[StickyTenseSyntax]) -> TenseModalAtomSyntaxStickyTense:
        return cls._from_fields((sticky_tense,))
    def __init__(self, sticky_tense: RecoveredField[StickyTenseSyntax]) -> None:
        pass
    @property
    def sticky_tense(self) -> RecoveredField[StickyTenseSyntax]:
        'Uses the `sticky_tense` product form, whose payload preserves `ki`.'
        return cast(RecoveredField[StickyTenseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TenseModalAtomSyntaxStickyTense is final')

TenseModalAtomSyntax: TypeAlias = TenseModalAtomSyntaxCompositeTense | TenseModalAtomSyntaxFihoTense | TenseModalAtomSyntaxModalTense | TenseModalAtomSyntaxNaheSeFlatPrefixedTense | TenseModalAtomSyntaxSeFlatPrefixedTense | TenseModalAtomSyntaxFaFlatTagTense | TenseModalAtomSyntaxZantufaRecursiveTagTense | TenseModalAtomSyntaxStickyTense

@final
class FihoTenseSyntax(_SyntaxNode):
    'Product node for FIhO modal; preserves `fiho`, `selbri`, and `fehu` in source order.'
    __slots__ = ()
    _schema_id = 525
    __match_args__ = ('fiho', 'selbri', 'fehu')
    def __new__(cls, fiho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], selbri: RecoveredField[SelbriSyntax], fehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> FihoTenseSyntax:
        return cls._from_fields((fiho, selbri, fehu))
    def __init__(self, fiho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], selbri: RecoveredField[SelbriSyntax], fehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def fiho(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Fiho` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def selbri(self) -> RecoveredField[SelbriSyntax]:
        'The shared selbri child syntax node.'
        return cast(RecoveredField[SelbriSyntax], self._field(1))
    @property
    def fehu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Fehu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('FihoTenseSyntax is final')

@final
class FaFlatTagTenseSyntax(_SyntaxNode):
    'Transparent product node for tag; preserves the `fa` component.'
    __slots__ = ()
    _schema_id = 526
    __match_args__ = ('fa',)
    def __new__(cls, fa: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> FaFlatTagTenseSyntax:
        return cls._from_fields((fa,))
    def __init__(self, fa: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def fa(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Fa`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('FaFlatTagTenseSyntax is final')

@final
class FlatTagAtomSyntaxFaFlatTagAtom(_SyntaxNode):
    'Uses the `fa_flat_tag_atom` product form, whose payload preserves `fa`.'
    __slots__ = ()
    _schema_id = 527
    __match_args__ = ('fa_flat_tag_atom',)
    def __new__(cls, fa_flat_tag_atom: RecoveredField[FaFlatTagAtomSyntax]) -> FlatTagAtomSyntaxFaFlatTagAtom:
        return cls._from_fields((fa_flat_tag_atom,))
    def __init__(self, fa_flat_tag_atom: RecoveredField[FaFlatTagAtomSyntax]) -> None:
        pass
    @property
    def fa_flat_tag_atom(self) -> RecoveredField[FaFlatTagAtomSyntax]:
        'Uses the `fa_flat_tag_atom` product form, whose payload preserves `fa`.'
        return cast(RecoveredField[FaFlatTagAtomSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('FlatTagAtomSyntaxFaFlatTagAtom is final')

@final
class FlatTagAtomSyntaxModalFlatTagAtom(_SyntaxNode):
    'Uses the `modal_flat_tag_atom` product form, whose payload preserves `modal`.'
    __slots__ = ()
    _schema_id = 528
    __match_args__ = ('modal_flat_tag_atom',)
    def __new__(cls, modal_flat_tag_atom: RecoveredField[ModalFlatTagAtomSyntax]) -> FlatTagAtomSyntaxModalFlatTagAtom:
        return cls._from_fields((modal_flat_tag_atom,))
    def __init__(self, modal_flat_tag_atom: RecoveredField[ModalFlatTagAtomSyntax]) -> None:
        pass
    @property
    def modal_flat_tag_atom(self) -> RecoveredField[ModalFlatTagAtomSyntax]:
        'Uses the `modal_flat_tag_atom` product form, whose payload preserves `modal`.'
        return cast(RecoveredField[ModalFlatTagAtomSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('FlatTagAtomSyntaxModalFlatTagAtom is final')

@final
class FlatTagAtomSyntaxCompositeFlatTagAtom(_SyntaxNode):
    'Uses the `composite_flat_tag_atom` product form, whose payload preserves `composite`.'
    __slots__ = ()
    _schema_id = 529
    __match_args__ = ('composite_flat_tag_atom',)
    def __new__(cls, composite_flat_tag_atom: RecoveredField[CompositeFlatTagAtomSyntax]) -> FlatTagAtomSyntaxCompositeFlatTagAtom:
        return cls._from_fields((composite_flat_tag_atom,))
    def __init__(self, composite_flat_tag_atom: RecoveredField[CompositeFlatTagAtomSyntax]) -> None:
        pass
    @property
    def composite_flat_tag_atom(self) -> RecoveredField[CompositeFlatTagAtomSyntax]:
        'Uses the `composite_flat_tag_atom` product form, whose payload preserves `composite`.'
        return cast(RecoveredField[CompositeFlatTagAtomSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('FlatTagAtomSyntaxCompositeFlatTagAtom is final')

FlatTagAtomSyntax: TypeAlias = FlatTagAtomSyntaxFaFlatTagAtom | FlatTagAtomSyntaxModalFlatTagAtom | FlatTagAtomSyntaxCompositeFlatTagAtom

@final
class FaFlatTagAtomSyntax(_SyntaxNode):
    'Transparent product node for tag; preserves the `fa` component.'
    __slots__ = ()
    _schema_id = 530
    __match_args__ = ('fa',)
    def __new__(cls, fa: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> FaFlatTagAtomSyntax:
        return cls._from_fields((fa,))
    def __init__(self, fa: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def fa(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Fa`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('FaFlatTagAtomSyntax is final')

@final
class ModalFlatTagAtomSyntax(_SyntaxNode):
    'Transparent product node for modal tag; preserves the `modal` component.'
    __slots__ = ()
    _schema_id = 531
    __match_args__ = ('modal',)
    def __new__(cls, modal: RecoveredField[ModalTenseSyntax]) -> ModalFlatTagAtomSyntax:
        return cls._from_fields((modal,))
    def __init__(self, modal: RecoveredField[ModalTenseSyntax]) -> None:
        pass
    @property
    def modal(self) -> RecoveredField[ModalTenseSyntax]:
        'The shared modal child syntax node.'
        return cast(RecoveredField[ModalTenseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ModalFlatTagAtomSyntax is final')

@final
class CompositeFlatTagAtomSyntax(_SyntaxNode):
    'Transparent product node for tag; preserves the `composite` component.'
    __slots__ = ()
    _schema_id = 532
    __match_args__ = ('composite',)
    def __new__(cls, composite: RecoveredField[CompositeTenseSyntax]) -> CompositeFlatTagAtomSyntax:
        return cls._from_fields((composite,))
    def __init__(self, composite: RecoveredField[CompositeTenseSyntax]) -> None:
        pass
    @property
    def composite(self) -> RecoveredField[CompositeTenseSyntax]:
        'The shared composite child syntax node.'
        return cast(RecoveredField[CompositeTenseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('CompositeFlatTagAtomSyntax is final')

@final
class NaheSeFlatPrefixedTenseSyntax(_SyntaxNode):
    'Product node for tag; preserves `nahe`, `se`, and `atom` in source order.'
    __slots__ = ()
    _schema_id = 533
    __match_args__ = ('nahe', 'se', 'atom')
    def __new__(cls, nahe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], se: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, atom: RecoveredField[FlatTagAtomSyntax]) -> NaheSeFlatPrefixedTenseSyntax:
        return cls._from_fields((nahe, se, atom))
    def __init__(self, nahe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], se: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, atom: RecoveredField[FlatTagAtomSyntax]) -> None:
        pass
    @property
    def nahe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Nahe`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def se(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional se component.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(1))
    @property
    def atom(self) -> RecoveredField[FlatTagAtomSyntax]:
        'The `flat_tag_atom` grammar result in the `atom` structural role of the `nahe_se_flat_prefixed_tense` production.'
        return cast(RecoveredField[FlatTagAtomSyntax], self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('NaheSeFlatPrefixedTenseSyntax is final')

@final
class SeFlatPrefixedTenseSyntax(_SyntaxNode):
    'Product node for tag; preserves `se` and `atom` in source order.'
    __slots__ = ()
    _schema_id = 534
    __match_args__ = ('se', 'atom')
    def __new__(cls, se: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], atom: RecoveredField[FlatTagAtomSyntax]) -> SeFlatPrefixedTenseSyntax:
        return cls._from_fields((se, atom))
    def __init__(self, se: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], atom: RecoveredField[FlatTagAtomSyntax]) -> None:
        pass
    @property
    def se(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Se`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def atom(self) -> RecoveredField[FlatTagAtomSyntax]:
        'The `flat_tag_atom` grammar result in the `atom` structural role of the `se_flat_prefixed_tense` production.'
        return cast(RecoveredField[FlatTagAtomSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('SeFlatPrefixedTenseSyntax is final')

@final
class ZantufaRecursiveTagTenseSyntax(_SyntaxNode):
    'Product node for tag; preserves `first_prefix`, `additional_prefixes`, and `atom` in source order.'
    __slots__ = ()
    _schema_id = 535
    __match_args__ = ('first_prefix', 'additional_prefixes', 'atom')
    def __new__(cls, first_prefix: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], additional_prefixes: Sequence[WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]], atom: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> ZantufaRecursiveTagTenseSyntax:
        return cls._from_fields((first_prefix, additional_prefixes, atom))
    def __init__(self, first_prefix: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], additional_prefixes: Sequence[WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]], atom: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def first_prefix(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The first selected prefix alternative before the recursively nested tag tense.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def additional_prefixes(self) -> tuple[WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], ...]:
        'Ordered sequence of zero or more additional prefixes components.'
        return cast(tuple[WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], ...], self._field(1))
    @property
    def atom(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The selected grammar alternative in the `atom` structural role of the `zantufa_recursive_tag_tense` production.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaRecursiveTagTenseSyntax is final')

@final
class CompositeTenseSyntaxPrefixedTimeSpaceCahaTense(_SyntaxNode):
    'Uses the `prefixed_time_space_caha_tense` product form, whose payload preserves `nahe`, `tense`, and `ki`.'
    __slots__ = ()
    _schema_id = 536
    __match_args__ = ('prefixed_time_space_caha_tense',)
    def __new__(cls, prefixed_time_space_caha_tense: RecoveredField[PrefixedTimeSpaceCahaTenseSyntax]) -> CompositeTenseSyntaxPrefixedTimeSpaceCahaTense:
        return cls._from_fields((prefixed_time_space_caha_tense,))
    def __init__(self, prefixed_time_space_caha_tense: RecoveredField[PrefixedTimeSpaceCahaTenseSyntax]) -> None:
        pass
    @property
    def prefixed_time_space_caha_tense(self) -> RecoveredField[PrefixedTimeSpaceCahaTenseSyntax]:
        'Uses the `prefixed_time_space_caha_tense` product form, whose payload preserves `nahe`, `tense`, and `ki`.'
        return cast(RecoveredField[PrefixedTimeSpaceCahaTenseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('CompositeTenseSyntaxPrefixedTimeSpaceCahaTense is final')

@final
class CompositeTenseSyntaxTimeSpaceCahaKiTense(_SyntaxNode):
    'Uses the `time_space_caha_ki_tense` product form, whose payload preserves `tense` and `ki`.'
    __slots__ = ()
    _schema_id = 537
    __match_args__ = ('time_space_caha_ki_tense',)
    def __new__(cls, time_space_caha_ki_tense: RecoveredField[TimeSpaceCahaKiTenseSyntax]) -> CompositeTenseSyntaxTimeSpaceCahaKiTense:
        return cls._from_fields((time_space_caha_ki_tense,))
    def __init__(self, time_space_caha_ki_tense: RecoveredField[TimeSpaceCahaKiTenseSyntax]) -> None:
        pass
    @property
    def time_space_caha_ki_tense(self) -> RecoveredField[TimeSpaceCahaKiTenseSyntax]:
        'Uses the `time_space_caha_ki_tense` product form, whose payload preserves `tense` and `ki`.'
        return cast(RecoveredField[TimeSpaceCahaKiTenseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('CompositeTenseSyntaxTimeSpaceCahaKiTense is final')

@final
class CompositeTenseSyntaxCuheTense(_SyntaxNode):
    'Uses the `cuhe_tense` product form, whose payload preserves `cuhe`.'
    __slots__ = ()
    _schema_id = 538
    __match_args__ = ('cuhe_tense',)
    def __new__(cls, cuhe_tense: RecoveredField[CuheTenseSyntax]) -> CompositeTenseSyntaxCuheTense:
        return cls._from_fields((cuhe_tense,))
    def __init__(self, cuhe_tense: RecoveredField[CuheTenseSyntax]) -> None:
        pass
    @property
    def cuhe_tense(self) -> RecoveredField[CuheTenseSyntax]:
        'Uses the `cuhe_tense` product form, whose payload preserves `cuhe`.'
        return cast(RecoveredField[CuheTenseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('CompositeTenseSyntaxCuheTense is final')

CompositeTenseSyntax: TypeAlias = CompositeTenseSyntaxPrefixedTimeSpaceCahaTense | CompositeTenseSyntaxTimeSpaceCahaKiTense | CompositeTenseSyntaxCuheTense

@final
class PrefixedTimeSpaceCahaTenseSyntax(_SyntaxNode):
    'Product node for tag; preserves `nahe`, `tense`, and `ki` in source order.'
    __slots__ = ()
    _schema_id = 539
    __match_args__ = ('nahe', 'tense', 'ki')
    def __new__(cls, nahe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], tense: RecoveredField[TimeSpaceCahaTenseSyntax], ki: RecoveredField[KiCompositeTenseSyntax] | None) -> PrefixedTimeSpaceCahaTenseSyntax:
        return cls._from_fields((nahe, tense, ki))
    def __init__(self, nahe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], tense: RecoveredField[TimeSpaceCahaTenseSyntax], ki: RecoveredField[KiCompositeTenseSyntax] | None) -> None:
        pass
    @property
    def nahe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Nahe`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def tense(self) -> RecoveredField[TimeSpaceCahaTenseSyntax]:
        'The shared tense child syntax node.'
        return cast(RecoveredField[TimeSpaceCahaTenseSyntax], self._field(1))
    @property
    def ki(self) -> RecoveredField[KiCompositeTenseSyntax] | None:
        'The optional ki component.'
        return cast(RecoveredField[KiCompositeTenseSyntax] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('PrefixedTimeSpaceCahaTenseSyntax is final')

@final
class TimeSpaceCahaKiTenseSyntax(_SyntaxNode):
    'Product node for tag; preserves `tense` and `ki` in source order.'
    __slots__ = ()
    _schema_id = 540
    __match_args__ = ('tense', 'ki')
    def __new__(cls, tense: RecoveredField[TimeSpaceCahaTenseSyntax], ki: RecoveredField[KiCompositeTenseSyntax] | None) -> TimeSpaceCahaKiTenseSyntax:
        return cls._from_fields((tense, ki))
    def __init__(self, tense: RecoveredField[TimeSpaceCahaTenseSyntax], ki: RecoveredField[KiCompositeTenseSyntax] | None) -> None:
        pass
    @property
    def tense(self) -> RecoveredField[TimeSpaceCahaTenseSyntax]:
        'The shared tense child syntax node.'
        return cast(RecoveredField[TimeSpaceCahaTenseSyntax], self._field(0))
    @property
    def ki(self) -> RecoveredField[KiCompositeTenseSyntax] | None:
        'The optional ki component.'
        return cast(RecoveredField[KiCompositeTenseSyntax] | None, self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('TimeSpaceCahaKiTenseSyntax is final')

@final
class TimeSpaceCahaTenseSyntaxTimeThenSpaceCahaTense(_SyntaxNode):
    'Uses the `time_then_space_caha_tense` product form, whose payload preserves `time`, `space`, and `caha`.'
    __slots__ = ()
    _schema_id = 541
    __match_args__ = ('time_then_space_caha_tense',)
    def __new__(cls, time_then_space_caha_tense: RecoveredField[TimeThenSpaceCahaTenseSyntax]) -> TimeSpaceCahaTenseSyntaxTimeThenSpaceCahaTense:
        return cls._from_fields((time_then_space_caha_tense,))
    def __init__(self, time_then_space_caha_tense: RecoveredField[TimeThenSpaceCahaTenseSyntax]) -> None:
        pass
    @property
    def time_then_space_caha_tense(self) -> RecoveredField[TimeThenSpaceCahaTenseSyntax]:
        'Uses the `time_then_space_caha_tense` product form, whose payload preserves `time`, `space`, and `caha`.'
        return cast(RecoveredField[TimeThenSpaceCahaTenseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TimeSpaceCahaTenseSyntaxTimeThenSpaceCahaTense is final')

@final
class TimeSpaceCahaTenseSyntaxSpaceThenTimeCahaTense(_SyntaxNode):
    'Uses the `space_then_time_caha_tense` product form, whose payload preserves `space`, `time`, and `caha`.'
    __slots__ = ()
    _schema_id = 542
    __match_args__ = ('space_then_time_caha_tense',)
    def __new__(cls, space_then_time_caha_tense: RecoveredField[SpaceThenTimeCahaTenseSyntax]) -> TimeSpaceCahaTenseSyntaxSpaceThenTimeCahaTense:
        return cls._from_fields((space_then_time_caha_tense,))
    def __init__(self, space_then_time_caha_tense: RecoveredField[SpaceThenTimeCahaTenseSyntax]) -> None:
        pass
    @property
    def space_then_time_caha_tense(self) -> RecoveredField[SpaceThenTimeCahaTenseSyntax]:
        'Uses the `space_then_time_caha_tense` product form, whose payload preserves `space`, `time`, and `caha`.'
        return cast(RecoveredField[SpaceThenTimeCahaTenseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TimeSpaceCahaTenseSyntaxSpaceThenTimeCahaTense is final')

@final
class TimeSpaceCahaTenseSyntaxCahaTense(_SyntaxNode):
    'Uses the `caha_tense` product form, whose payload preserves `caha`.'
    __slots__ = ()
    _schema_id = 543
    __match_args__ = ('caha_tense',)
    def __new__(cls, caha_tense: RecoveredField[CahaTenseSyntax]) -> TimeSpaceCahaTenseSyntaxCahaTense:
        return cls._from_fields((caha_tense,))
    def __init__(self, caha_tense: RecoveredField[CahaTenseSyntax]) -> None:
        pass
    @property
    def caha_tense(self) -> RecoveredField[CahaTenseSyntax]:
        'Uses the `caha_tense` product form, whose payload preserves `caha`.'
        return cast(RecoveredField[CahaTenseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TimeSpaceCahaTenseSyntaxCahaTense is final')

TimeSpaceCahaTenseSyntax: TypeAlias = TimeSpaceCahaTenseSyntaxTimeThenSpaceCahaTense | TimeSpaceCahaTenseSyntaxSpaceThenTimeCahaTense | TimeSpaceCahaTenseSyntaxCahaTense

@final
class TimeThenSpaceCahaTenseSyntax(_SyntaxNode):
    'Product node for time tense; preserves `time`, `space`, and `caha` in source order.'
    __slots__ = ()
    _schema_id = 544
    __match_args__ = ('time', 'space', 'caha')
    def __new__(cls, time: RecoveredField[TimeTenseSyntax], space: RecoveredField[SpaceTenseSyntax] | None, caha: RecoveredField[CahaTenseSyntax] | None) -> TimeThenSpaceCahaTenseSyntax:
        return cls._from_fields((time, space, caha))
    def __init__(self, time: RecoveredField[TimeTenseSyntax], space: RecoveredField[SpaceTenseSyntax] | None, caha: RecoveredField[CahaTenseSyntax] | None) -> None:
        pass
    @property
    def time(self) -> RecoveredField[TimeTenseSyntax]:
        'The shared time child syntax node.'
        return cast(RecoveredField[TimeTenseSyntax], self._field(0))
    @property
    def space(self) -> RecoveredField[SpaceTenseSyntax] | None:
        'The optional space component.'
        return cast(RecoveredField[SpaceTenseSyntax] | None, self._field(1))
    @property
    def caha(self) -> RecoveredField[CahaTenseSyntax] | None:
        'The optional caha component.'
        return cast(RecoveredField[CahaTenseSyntax] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('TimeThenSpaceCahaTenseSyntax is final')

@final
class SpaceThenTimeCahaTenseSyntax(_SyntaxNode):
    'Product node for space tense; preserves `space`, `time`, and `caha` in source order.'
    __slots__ = ()
    _schema_id = 545
    __match_args__ = ('space', 'time', 'caha')
    def __new__(cls, space: RecoveredField[SpaceTenseSyntax], time: RecoveredField[TimeTenseSyntax] | None, caha: RecoveredField[CahaTenseSyntax] | None) -> SpaceThenTimeCahaTenseSyntax:
        return cls._from_fields((space, time, caha))
    def __init__(self, space: RecoveredField[SpaceTenseSyntax], time: RecoveredField[TimeTenseSyntax] | None, caha: RecoveredField[CahaTenseSyntax] | None) -> None:
        pass
    @property
    def space(self) -> RecoveredField[SpaceTenseSyntax]:
        'The shared space child syntax node.'
        return cast(RecoveredField[SpaceTenseSyntax], self._field(0))
    @property
    def time(self) -> RecoveredField[TimeTenseSyntax] | None:
        'The optional time component.'
        return cast(RecoveredField[TimeTenseSyntax] | None, self._field(1))
    @property
    def caha(self) -> RecoveredField[CahaTenseSyntax] | None:
        'The optional caha component.'
        return cast(RecoveredField[CahaTenseSyntax] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('SpaceThenTimeCahaTenseSyntax is final')

@final
class TimeTenseSyntaxTimeTenseWithZi(_SyntaxNode):
    'Uses the `time_tense_with_zi` product form, whose payload preserves `zi`, `offsets`, `zeha`, and `properties`.'
    __slots__ = ()
    _schema_id = 546
    __match_args__ = ('time_tense_with_zi',)
    def __new__(cls, time_tense_with_zi: RecoveredField[TimeTenseWithZiSyntax]) -> TimeTenseSyntaxTimeTenseWithZi:
        return cls._from_fields((time_tense_with_zi,))
    def __init__(self, time_tense_with_zi: RecoveredField[TimeTenseWithZiSyntax]) -> None:
        pass
    @property
    def time_tense_with_zi(self) -> RecoveredField[TimeTenseWithZiSyntax]:
        'Uses the `time_tense_with_zi` product form, whose payload preserves `zi`, `offsets`, `zeha`, and `properties`.'
        return cast(RecoveredField[TimeTenseWithZiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TimeTenseSyntaxTimeTenseWithZi is final')

@final
class TimeTenseSyntaxTimeTenseWithOffset(_SyntaxNode):
    'Uses the `time_tense_with_offset` product form, whose payload preserves `zi`, `offsets`, `zeha`, and `properties`.'
    __slots__ = ()
    _schema_id = 547
    __match_args__ = ('time_tense_with_offset',)
    def __new__(cls, time_tense_with_offset: RecoveredField[TimeTenseWithOffsetSyntax]) -> TimeTenseSyntaxTimeTenseWithOffset:
        return cls._from_fields((time_tense_with_offset,))
    def __init__(self, time_tense_with_offset: RecoveredField[TimeTenseWithOffsetSyntax]) -> None:
        pass
    @property
    def time_tense_with_offset(self) -> RecoveredField[TimeTenseWithOffsetSyntax]:
        'Uses the `time_tense_with_offset` product form, whose payload preserves `zi`, `offsets`, `zeha`, and `properties`.'
        return cast(RecoveredField[TimeTenseWithOffsetSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TimeTenseSyntaxTimeTenseWithOffset is final')

@final
class TimeTenseSyntaxTimeTenseWithInterval(_SyntaxNode):
    'Uses the `time_tense_with_interval` product form, whose payload preserves `zi`, `offsets`, `zeha`, and `properties`.'
    __slots__ = ()
    _schema_id = 548
    __match_args__ = ('time_tense_with_interval',)
    def __new__(cls, time_tense_with_interval: RecoveredField[TimeTenseWithIntervalSyntax]) -> TimeTenseSyntaxTimeTenseWithInterval:
        return cls._from_fields((time_tense_with_interval,))
    def __init__(self, time_tense_with_interval: RecoveredField[TimeTenseWithIntervalSyntax]) -> None:
        pass
    @property
    def time_tense_with_interval(self) -> RecoveredField[TimeTenseWithIntervalSyntax]:
        'Uses the `time_tense_with_interval` product form, whose payload preserves `zi`, `offsets`, `zeha`, and `properties`.'
        return cast(RecoveredField[TimeTenseWithIntervalSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TimeTenseSyntaxTimeTenseWithInterval is final')

@final
class TimeTenseSyntaxTimeTenseWithProperties(_SyntaxNode):
    'Uses the `time_tense_with_properties` product form, whose payload preserves `zi`, `offsets`, `zeha`, and `properties`.'
    __slots__ = ()
    _schema_id = 549
    __match_args__ = ('time_tense_with_properties',)
    def __new__(cls, time_tense_with_properties: RecoveredField[TimeTenseWithPropertiesSyntax]) -> TimeTenseSyntaxTimeTenseWithProperties:
        return cls._from_fields((time_tense_with_properties,))
    def __init__(self, time_tense_with_properties: RecoveredField[TimeTenseWithPropertiesSyntax]) -> None:
        pass
    @property
    def time_tense_with_properties(self) -> RecoveredField[TimeTenseWithPropertiesSyntax]:
        'Uses the `time_tense_with_properties` product form, whose payload preserves `zi`, `offsets`, `zeha`, and `properties`.'
        return cast(RecoveredField[TimeTenseWithPropertiesSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TimeTenseSyntaxTimeTenseWithProperties is final')

TimeTenseSyntax: TypeAlias = TimeTenseSyntaxTimeTenseWithZi | TimeTenseSyntaxTimeTenseWithOffset | TimeTenseSyntaxTimeTenseWithInterval | TimeTenseSyntaxTimeTenseWithProperties

@final
class TimeTenseWithZiSyntax(_SyntaxNode):
    'Product node for time tense; preserves `zi`, `offsets`, `zeha`, and `properties` in source order.'
    __slots__ = ()
    _schema_id = 550
    __match_args__ = ('zi', 'offsets', 'zeha', 'properties')
    def __new__(cls, zi: RecoveredField[ZiTimeDistanceTenseSyntax], offsets: Sequence[RecoveredField[PuTimeOffsetTenseSyntax]], zeha: RecoveredField[ZehaTimeIntervalTenseSyntax] | None, properties: Sequence[RecoveredField[IntervalPropertyTenseSyntax]]) -> TimeTenseWithZiSyntax:
        return cls._from_fields((zi, offsets, zeha, properties))
    def __init__(self, zi: RecoveredField[ZiTimeDistanceTenseSyntax], offsets: Sequence[RecoveredField[PuTimeOffsetTenseSyntax]], zeha: RecoveredField[ZehaTimeIntervalTenseSyntax] | None, properties: Sequence[RecoveredField[IntervalPropertyTenseSyntax]]) -> None:
        pass
    @property
    def zi(self) -> RecoveredField[ZiTimeDistanceTenseSyntax]:
        'The shared zi child syntax node.'
        return cast(RecoveredField[ZiTimeDistanceTenseSyntax], self._field(0))
    @property
    def offsets(self) -> tuple[RecoveredField[PuTimeOffsetTenseSyntax], ...]:
        'Ordered sequence of zero or more offsets components.'
        return cast(tuple[RecoveredField[PuTimeOffsetTenseSyntax], ...], self._field(1))
    @property
    def zeha(self) -> RecoveredField[ZehaTimeIntervalTenseSyntax] | None:
        'The optional zeha component.'
        return cast(RecoveredField[ZehaTimeIntervalTenseSyntax] | None, self._field(2))
    @property
    def properties(self) -> tuple[RecoveredField[IntervalPropertyTenseSyntax], ...]:
        'Ordered sequence of zero or more properties components.'
        return cast(tuple[RecoveredField[IntervalPropertyTenseSyntax], ...], self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('TimeTenseWithZiSyntax is final')

@final
class TimeTenseWithOffsetSyntax(_SyntaxNode):
    'Product node for time tense; preserves `zi`, `offsets`, `zeha`, and `properties` in source order.'
    __slots__ = ()
    _schema_id = 551
    __match_args__ = ('zi', 'offsets', 'zeha', 'properties')
    def __new__(cls, zi: RecoveredField[ZiTimeDistanceTenseSyntax] | None, offsets: Sequence[RecoveredField[PuTimeOffsetTenseSyntax]], zeha: RecoveredField[ZehaTimeIntervalTenseSyntax] | None, properties: Sequence[RecoveredField[IntervalPropertyTenseSyntax]]) -> TimeTenseWithOffsetSyntax:
        return cls._from_fields((zi, offsets, zeha, properties))
    def __init__(self, zi: RecoveredField[ZiTimeDistanceTenseSyntax] | None, offsets: Sequence[RecoveredField[PuTimeOffsetTenseSyntax]], zeha: RecoveredField[ZehaTimeIntervalTenseSyntax] | None, properties: Sequence[RecoveredField[IntervalPropertyTenseSyntax]]) -> None:
        pass
    @property
    def zi(self) -> RecoveredField[ZiTimeDistanceTenseSyntax] | None:
        'The optional zi component.'
        return cast(RecoveredField[ZiTimeDistanceTenseSyntax] | None, self._field(0))
    @property
    def offsets(self) -> tuple[RecoveredField[PuTimeOffsetTenseSyntax], ...]:
        'Non-empty ordered sequence of offsets components.'
        return cast(tuple[RecoveredField[PuTimeOffsetTenseSyntax], ...], self._field(1))
    @property
    def zeha(self) -> RecoveredField[ZehaTimeIntervalTenseSyntax] | None:
        'The optional zeha component.'
        return cast(RecoveredField[ZehaTimeIntervalTenseSyntax] | None, self._field(2))
    @property
    def properties(self) -> tuple[RecoveredField[IntervalPropertyTenseSyntax], ...]:
        'Ordered sequence of zero or more properties components.'
        return cast(tuple[RecoveredField[IntervalPropertyTenseSyntax], ...], self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('TimeTenseWithOffsetSyntax is final')

@final
class TimeTenseWithIntervalSyntax(_SyntaxNode):
    'Product node for time tense; preserves `zi`, `offsets`, `zeha`, and `properties` in source order.'
    __slots__ = ()
    _schema_id = 552
    __match_args__ = ('zi', 'offsets', 'zeha', 'properties')
    def __new__(cls, zi: RecoveredField[ZiTimeDistanceTenseSyntax] | None, offsets: Sequence[RecoveredField[PuTimeOffsetTenseSyntax]], zeha: RecoveredField[ZehaTimeIntervalTenseSyntax], properties: Sequence[RecoveredField[IntervalPropertyTenseSyntax]]) -> TimeTenseWithIntervalSyntax:
        return cls._from_fields((zi, offsets, zeha, properties))
    def __init__(self, zi: RecoveredField[ZiTimeDistanceTenseSyntax] | None, offsets: Sequence[RecoveredField[PuTimeOffsetTenseSyntax]], zeha: RecoveredField[ZehaTimeIntervalTenseSyntax], properties: Sequence[RecoveredField[IntervalPropertyTenseSyntax]]) -> None:
        pass
    @property
    def zi(self) -> RecoveredField[ZiTimeDistanceTenseSyntax] | None:
        'The optional zi component.'
        return cast(RecoveredField[ZiTimeDistanceTenseSyntax] | None, self._field(0))
    @property
    def offsets(self) -> tuple[RecoveredField[PuTimeOffsetTenseSyntax], ...]:
        'Ordered sequence of zero or more offsets components.'
        return cast(tuple[RecoveredField[PuTimeOffsetTenseSyntax], ...], self._field(1))
    @property
    def zeha(self) -> RecoveredField[ZehaTimeIntervalTenseSyntax]:
        'The shared zeha child syntax node.'
        return cast(RecoveredField[ZehaTimeIntervalTenseSyntax], self._field(2))
    @property
    def properties(self) -> tuple[RecoveredField[IntervalPropertyTenseSyntax], ...]:
        'Ordered sequence of zero or more properties components.'
        return cast(tuple[RecoveredField[IntervalPropertyTenseSyntax], ...], self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('TimeTenseWithIntervalSyntax is final')

@final
class TimeTenseWithPropertiesSyntax(_SyntaxNode):
    'Product node for time tense; preserves `zi`, `offsets`, `zeha`, and `properties` in source order.'
    __slots__ = ()
    _schema_id = 553
    __match_args__ = ('zi', 'offsets', 'zeha', 'properties')
    def __new__(cls, zi: RecoveredField[ZiTimeDistanceTenseSyntax] | None, offsets: Sequence[RecoveredField[PuTimeOffsetTenseSyntax]], zeha: RecoveredField[ZehaTimeIntervalTenseSyntax] | None, properties: Sequence[RecoveredField[IntervalPropertyTenseSyntax]]) -> TimeTenseWithPropertiesSyntax:
        return cls._from_fields((zi, offsets, zeha, properties))
    def __init__(self, zi: RecoveredField[ZiTimeDistanceTenseSyntax] | None, offsets: Sequence[RecoveredField[PuTimeOffsetTenseSyntax]], zeha: RecoveredField[ZehaTimeIntervalTenseSyntax] | None, properties: Sequence[RecoveredField[IntervalPropertyTenseSyntax]]) -> None:
        pass
    @property
    def zi(self) -> RecoveredField[ZiTimeDistanceTenseSyntax] | None:
        'The optional zi component.'
        return cast(RecoveredField[ZiTimeDistanceTenseSyntax] | None, self._field(0))
    @property
    def offsets(self) -> tuple[RecoveredField[PuTimeOffsetTenseSyntax], ...]:
        'Ordered sequence of zero or more offsets components.'
        return cast(tuple[RecoveredField[PuTimeOffsetTenseSyntax], ...], self._field(1))
    @property
    def zeha(self) -> RecoveredField[ZehaTimeIntervalTenseSyntax] | None:
        'The optional zeha component.'
        return cast(RecoveredField[ZehaTimeIntervalTenseSyntax] | None, self._field(2))
    @property
    def properties(self) -> tuple[RecoveredField[IntervalPropertyTenseSyntax], ...]:
        'Non-empty ordered sequence of properties components.'
        return cast(tuple[RecoveredField[IntervalPropertyTenseSyntax], ...], self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('TimeTenseWithPropertiesSyntax is final')

@final
class IntervalPropertyTenseSyntaxNumberedIntervalPropertyTense(_SyntaxNode):
    'Uses the `numbered_interval_property_tense` product form, whose payload preserves `number`, `roi`, and `nai`.'
    __slots__ = ()
    _schema_id = 554
    __match_args__ = ('numbered_interval_property_tense',)
    def __new__(cls, numbered_interval_property_tense: RecoveredField[NumberedIntervalPropertyTenseSyntax]) -> IntervalPropertyTenseSyntaxNumberedIntervalPropertyTense:
        return cls._from_fields((numbered_interval_property_tense,))
    def __init__(self, numbered_interval_property_tense: RecoveredField[NumberedIntervalPropertyTenseSyntax]) -> None:
        pass
    @property
    def numbered_interval_property_tense(self) -> RecoveredField[NumberedIntervalPropertyTenseSyntax]:
        'Uses the `numbered_interval_property_tense` product form, whose payload preserves `number`, `roi`, and `nai`.'
        return cast(RecoveredField[NumberedIntervalPropertyTenseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('IntervalPropertyTenseSyntaxNumberedIntervalPropertyTense is final')

@final
class IntervalPropertyTenseSyntaxTaheIntervalPropertyTense(_SyntaxNode):
    'Uses the `tahe_interval_property_tense` product form, whose payload preserves `tahe` and `nai`.'
    __slots__ = ()
    _schema_id = 555
    __match_args__ = ('tahe_interval_property_tense',)
    def __new__(cls, tahe_interval_property_tense: RecoveredField[TaheIntervalPropertyTenseSyntax]) -> IntervalPropertyTenseSyntaxTaheIntervalPropertyTense:
        return cls._from_fields((tahe_interval_property_tense,))
    def __init__(self, tahe_interval_property_tense: RecoveredField[TaheIntervalPropertyTenseSyntax]) -> None:
        pass
    @property
    def tahe_interval_property_tense(self) -> RecoveredField[TaheIntervalPropertyTenseSyntax]:
        'Uses the `tahe_interval_property_tense` product form, whose payload preserves `tahe` and `nai`.'
        return cast(RecoveredField[TaheIntervalPropertyTenseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('IntervalPropertyTenseSyntaxTaheIntervalPropertyTense is final')

@final
class IntervalPropertyTenseSyntaxZahoIntervalPropertyTense(_SyntaxNode):
    'Uses the `zaho_interval_property_tense` product form, whose payload preserves `zaho` and `nai`.'
    __slots__ = ()
    _schema_id = 556
    __match_args__ = ('zaho_interval_property_tense',)
    def __new__(cls, zaho_interval_property_tense: RecoveredField[ZahoIntervalPropertyTenseSyntax]) -> IntervalPropertyTenseSyntaxZahoIntervalPropertyTense:
        return cls._from_fields((zaho_interval_property_tense,))
    def __init__(self, zaho_interval_property_tense: RecoveredField[ZahoIntervalPropertyTenseSyntax]) -> None:
        pass
    @property
    def zaho_interval_property_tense(self) -> RecoveredField[ZahoIntervalPropertyTenseSyntax]:
        'Uses the `zaho_interval_property_tense` product form, whose payload preserves `zaho` and `nai`.'
        return cast(RecoveredField[ZahoIntervalPropertyTenseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('IntervalPropertyTenseSyntaxZahoIntervalPropertyTense is final')

IntervalPropertyTenseSyntax: TypeAlias = IntervalPropertyTenseSyntaxNumberedIntervalPropertyTense | IntervalPropertyTenseSyntaxTaheIntervalPropertyTense | IntervalPropertyTenseSyntaxZahoIntervalPropertyTense

@final
class NumberedIntervalPropertyTenseSyntax(_SyntaxNode):
    'Product node for interval property; preserves `number`, `roi`, and `nai` in source order.'
    __slots__ = ()
    _schema_id = 557
    __match_args__ = ('number', 'roi', 'nai')
    def __new__(cls, number: WithFreeModifiers[RecoveredField[IntervalPropertyNumberWordsSyntax], RecoveredField[FreeModifierSyntax]], roi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> NumberedIntervalPropertyTenseSyntax:
        return cls._from_fields((number, roi, nai))
    def __init__(self, number: WithFreeModifiers[RecoveredField[IntervalPropertyNumberWordsSyntax], RecoveredField[FreeModifierSyntax]], roi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def number(self) -> WithFreeModifiers[RecoveredField[IntervalPropertyNumberWordsSyntax], RecoveredField[FreeModifierSyntax]]:
        'The `interval_property_number_words` grammar result in the `number` structural role of the `numbered_interval_property_tense` production.'
        return cast(WithFreeModifiers[RecoveredField[IntervalPropertyNumberWordsSyntax], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def roi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Roi`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(1))
    @property
    def nai(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Nai` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('NumberedIntervalPropertyTenseSyntax is final')

@final
class IntervalPropertyNumberWordsSyntax(_SyntaxNode):
    'Product node for number; preserves `first_number` and `continuations` in source order.'
    __slots__ = ()
    _schema_id = 558
    __match_args__ = ('first_number', 'continuations')
    def __new__(cls, first_number: RecoveredField[Token], continuations: Sequence[RecoveredField[IntervalPropertyNumberWordContinuationSyntax]]) -> IntervalPropertyNumberWordsSyntax:
        return cls._from_fields((first_number, continuations))
    def __init__(self, first_number: RecoveredField[Token], continuations: Sequence[RecoveredField[IntervalPropertyNumberWordContinuationSyntax]]) -> None:
        pass
    @property
    def first_number(self) -> RecoveredField[Token]:
        'The initial `pa_word` constituent before the continuations of the `interval_property_number_words` production.'
        return cast(RecoveredField[Token], self._field(0))
    @property
    def continuations(self) -> tuple[RecoveredField[IntervalPropertyNumberWordContinuationSyntax], ...]:
        'Ordered sequence of zero or more continuations components.'
        return cast(tuple[RecoveredField[IntervalPropertyNumberWordContinuationSyntax], ...], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('IntervalPropertyNumberWordsSyntax is final')

@final
class IntervalPropertyNumberWordContinuationSyntaxIntervalPropertyNumberPaContinuation(_SyntaxNode):
    'Uses the `interval_property_number_pa_continuation` product form, whose payload preserves `pa`.'
    __slots__ = ()
    _schema_id = 559
    __match_args__ = ('interval_property_number_pa_continuation',)
    def __new__(cls, interval_property_number_pa_continuation: RecoveredField[IntervalPropertyNumberPaContinuationSyntax]) -> IntervalPropertyNumberWordContinuationSyntaxIntervalPropertyNumberPaContinuation:
        return cls._from_fields((interval_property_number_pa_continuation,))
    def __init__(self, interval_property_number_pa_continuation: RecoveredField[IntervalPropertyNumberPaContinuationSyntax]) -> None:
        pass
    @property
    def interval_property_number_pa_continuation(self) -> RecoveredField[IntervalPropertyNumberPaContinuationSyntax]:
        'Uses the `interval_property_number_pa_continuation` product form, whose payload preserves `pa`.'
        return cast(RecoveredField[IntervalPropertyNumberPaContinuationSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('IntervalPropertyNumberWordContinuationSyntaxIntervalPropertyNumberPaContinuation is final')

@final
class IntervalPropertyNumberWordContinuationSyntaxIntervalPropertyNumberLetterContinuation(_SyntaxNode):
    'Uses the `interval_property_number_letter_continuation` product form, whose payload preserves `letter`.'
    __slots__ = ()
    _schema_id = 560
    __match_args__ = ('interval_property_number_letter_continuation',)
    def __new__(cls, interval_property_number_letter_continuation: RecoveredField[IntervalPropertyNumberLetterContinuationSyntax]) -> IntervalPropertyNumberWordContinuationSyntaxIntervalPropertyNumberLetterContinuation:
        return cls._from_fields((interval_property_number_letter_continuation,))
    def __init__(self, interval_property_number_letter_continuation: RecoveredField[IntervalPropertyNumberLetterContinuationSyntax]) -> None:
        pass
    @property
    def interval_property_number_letter_continuation(self) -> RecoveredField[IntervalPropertyNumberLetterContinuationSyntax]:
        'Uses the `interval_property_number_letter_continuation` product form, whose payload preserves `letter`.'
        return cast(RecoveredField[IntervalPropertyNumberLetterContinuationSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('IntervalPropertyNumberWordContinuationSyntaxIntervalPropertyNumberLetterContinuation is final')

IntervalPropertyNumberWordContinuationSyntax: TypeAlias = IntervalPropertyNumberWordContinuationSyntaxIntervalPropertyNumberPaContinuation | IntervalPropertyNumberWordContinuationSyntaxIntervalPropertyNumberLetterContinuation

@final
class IntervalPropertyNumberPaContinuationSyntax(_SyntaxNode):
    'Transparent product node for number continuation; preserves the `pa` component.'
    __slots__ = ()
    _schema_id = 561
    __match_args__ = ('pa',)
    def __new__(cls, pa: RecoveredField[Token]) -> IntervalPropertyNumberPaContinuationSyntax:
        return cls._from_fields((pa,))
    def __init__(self, pa: RecoveredField[Token]) -> None:
        pass
    @property
    def pa(self) -> RecoveredField[Token]:
        'The `pa_word` grammar result in the `pa` structural role of the `interval_property_number_pa_continuation` production.'
        return cast(RecoveredField[Token], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('IntervalPropertyNumberPaContinuationSyntax is final')

@final
class IntervalPropertyNumberLetterContinuationSyntax(_SyntaxNode):
    'Transparent product node for number continuation; preserves the `letter` component.'
    __slots__ = ()
    _schema_id = 562
    __match_args__ = ('letter',)
    def __new__(cls, letter: RecoveredField[Token]) -> IntervalPropertyNumberLetterContinuationSyntax:
        return cls._from_fields((letter,))
    def __init__(self, letter: RecoveredField[Token]) -> None:
        pass
    @property
    def letter(self) -> RecoveredField[Token]:
        'The `word_category` grammar result in the `letter` structural role of the `interval_property_number_letter_continuation` production.'
        return cast(RecoveredField[Token], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('IntervalPropertyNumberLetterContinuationSyntax is final')

@final
class TaheIntervalPropertyTenseSyntax(_SyntaxNode):
    'Product node for interval property; preserves `tahe` and `nai` in source order.'
    __slots__ = ()
    _schema_id = 563
    __match_args__ = ('tahe', 'nai')
    def __new__(cls, tahe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> TaheIntervalPropertyTenseSyntax:
        return cls._from_fields((tahe, nai))
    def __init__(self, tahe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def tahe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Tahe`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def nai(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Nai` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('TaheIntervalPropertyTenseSyntax is final')

@final
class ZahoIntervalPropertyTenseSyntax(_SyntaxNode):
    'Product node for interval property; preserves `zaho` and `nai` in source order.'
    __slots__ = ()
    _schema_id = 564
    __match_args__ = ('zaho', 'nai')
    def __new__(cls, zaho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> ZahoIntervalPropertyTenseSyntax:
        return cls._from_fields((zaho, nai))
    def __init__(self, zaho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def zaho(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Zaho`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def nai(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Nai` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZahoIntervalPropertyTenseSyntax is final')

@final
class PuTimeOffsetTenseSyntax(_SyntaxNode):
    'Product node for time tense; preserves `pu`, `nai`, and `distance` in source order.'
    __slots__ = ()
    _schema_id = 565
    __match_args__ = ('pu', 'nai', 'distance')
    def __new__(cls, pu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, distance: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> PuTimeOffsetTenseSyntax:
        return cls._from_fields((pu, nai, distance))
    def __init__(self, pu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, distance: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def pu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Pu`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def nai(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Nai` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(1))
    @property
    def distance(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional distance component.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('PuTimeOffsetTenseSyntax is final')

@final
class ZiTimeDistanceTenseSyntax(_SyntaxNode):
    'Transparent product node for time tense; preserves the `zi` component.'
    __slots__ = ()
    _schema_id = 566
    __match_args__ = ('zi',)
    def __new__(cls, zi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> ZiTimeDistanceTenseSyntax:
        return cls._from_fields((zi,))
    def __init__(self, zi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def zi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Zi`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZiTimeDistanceTenseSyntax is final')

@final
class ZehaTimeIntervalTenseSyntax(_SyntaxNode):
    'Product node for time interval; preserves `zeha` and `direction` in source order.'
    __slots__ = ()
    _schema_id = 567
    __match_args__ = ('zeha', 'direction')
    def __new__(cls, zeha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], direction: tuple[WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None] | None) -> ZehaTimeIntervalTenseSyntax:
        return cls._from_fields((zeha, direction))
    def __init__(self, zeha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], direction: tuple[WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None] | None) -> None:
        pass
    @property
    def zeha(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Zeha`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def direction(self) -> tuple[WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None] | None:
        'The optional pair containing a required PU-family direction word followed by an optional `Nai` cmavo marker.'
        return cast(tuple[WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None] | None, self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZehaTimeIntervalTenseSyntax is final')

@final
class SpaceTenseSyntaxSpaceTenseWithVa(_SyntaxNode):
    'Uses the `space_tense_with_va` product form, whose payload preserves `va`, `offsets`, `interval`, and `mohi`.'
    __slots__ = ()
    _schema_id = 568
    __match_args__ = ('space_tense_with_va',)
    def __new__(cls, space_tense_with_va: RecoveredField[SpaceTenseWithVaSyntax]) -> SpaceTenseSyntaxSpaceTenseWithVa:
        return cls._from_fields((space_tense_with_va,))
    def __init__(self, space_tense_with_va: RecoveredField[SpaceTenseWithVaSyntax]) -> None:
        pass
    @property
    def space_tense_with_va(self) -> RecoveredField[SpaceTenseWithVaSyntax]:
        'Uses the `space_tense_with_va` product form, whose payload preserves `va`, `offsets`, `interval`, and `mohi`.'
        return cast(RecoveredField[SpaceTenseWithVaSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SpaceTenseSyntaxSpaceTenseWithVa is final')

@final
class SpaceTenseSyntaxSpaceTenseWithOffset(_SyntaxNode):
    'Uses the `space_tense_with_offset` product form, whose payload preserves `va`, `offsets`, `interval`, and `mohi`.'
    __slots__ = ()
    _schema_id = 569
    __match_args__ = ('space_tense_with_offset',)
    def __new__(cls, space_tense_with_offset: RecoveredField[SpaceTenseWithOffsetSyntax]) -> SpaceTenseSyntaxSpaceTenseWithOffset:
        return cls._from_fields((space_tense_with_offset,))
    def __init__(self, space_tense_with_offset: RecoveredField[SpaceTenseWithOffsetSyntax]) -> None:
        pass
    @property
    def space_tense_with_offset(self) -> RecoveredField[SpaceTenseWithOffsetSyntax]:
        'Uses the `space_tense_with_offset` product form, whose payload preserves `va`, `offsets`, `interval`, and `mohi`.'
        return cast(RecoveredField[SpaceTenseWithOffsetSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SpaceTenseSyntaxSpaceTenseWithOffset is final')

@final
class SpaceTenseSyntaxSpaceTenseWithInterval(_SyntaxNode):
    'Uses the `space_tense_with_interval` product form, whose payload preserves `va`, `offsets`, `interval`, and `mohi`.'
    __slots__ = ()
    _schema_id = 570
    __match_args__ = ('space_tense_with_interval',)
    def __new__(cls, space_tense_with_interval: RecoveredField[SpaceTenseWithIntervalSyntax]) -> SpaceTenseSyntaxSpaceTenseWithInterval:
        return cls._from_fields((space_tense_with_interval,))
    def __init__(self, space_tense_with_interval: RecoveredField[SpaceTenseWithIntervalSyntax]) -> None:
        pass
    @property
    def space_tense_with_interval(self) -> RecoveredField[SpaceTenseWithIntervalSyntax]:
        'Uses the `space_tense_with_interval` product form, whose payload preserves `va`, `offsets`, `interval`, and `mohi`.'
        return cast(RecoveredField[SpaceTenseWithIntervalSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SpaceTenseSyntaxSpaceTenseWithInterval is final')

@final
class SpaceTenseSyntaxSpaceTenseWithMohi(_SyntaxNode):
    'Uses the `space_tense_with_mohi` product form, whose payload preserves `va`, `offsets`, `interval`, and `mohi`.'
    __slots__ = ()
    _schema_id = 571
    __match_args__ = ('space_tense_with_mohi',)
    def __new__(cls, space_tense_with_mohi: RecoveredField[SpaceTenseWithMohiSyntax]) -> SpaceTenseSyntaxSpaceTenseWithMohi:
        return cls._from_fields((space_tense_with_mohi,))
    def __init__(self, space_tense_with_mohi: RecoveredField[SpaceTenseWithMohiSyntax]) -> None:
        pass
    @property
    def space_tense_with_mohi(self) -> RecoveredField[SpaceTenseWithMohiSyntax]:
        'Uses the `space_tense_with_mohi` product form, whose payload preserves `va`, `offsets`, `interval`, and `mohi`.'
        return cast(RecoveredField[SpaceTenseWithMohiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SpaceTenseSyntaxSpaceTenseWithMohi is final')

SpaceTenseSyntax: TypeAlias = SpaceTenseSyntaxSpaceTenseWithVa | SpaceTenseSyntaxSpaceTenseWithOffset | SpaceTenseSyntaxSpaceTenseWithInterval | SpaceTenseSyntaxSpaceTenseWithMohi

@final
class SpaceTenseWithVaSyntax(_SyntaxNode):
    'Product node for space tense; preserves `va`, `offsets`, `interval`, and `mohi` in source order.'
    __slots__ = ()
    _schema_id = 572
    __match_args__ = ('va', 'offsets', 'interval', 'mohi')
    def __new__(cls, va: RecoveredField[VaSpaceDistanceTenseSyntax], offsets: Sequence[RecoveredField[FahaSpaceOffsetTenseSyntax]], interval: RecoveredField[SpaceIntervalTenseSyntax] | None, mohi: RecoveredField[MohiSpaceOffsetTenseSyntax] | None) -> SpaceTenseWithVaSyntax:
        return cls._from_fields((va, offsets, interval, mohi))
    def __init__(self, va: RecoveredField[VaSpaceDistanceTenseSyntax], offsets: Sequence[RecoveredField[FahaSpaceOffsetTenseSyntax]], interval: RecoveredField[SpaceIntervalTenseSyntax] | None, mohi: RecoveredField[MohiSpaceOffsetTenseSyntax] | None) -> None:
        pass
    @property
    def va(self) -> RecoveredField[VaSpaceDistanceTenseSyntax]:
        'The shared va child syntax node.'
        return cast(RecoveredField[VaSpaceDistanceTenseSyntax], self._field(0))
    @property
    def offsets(self) -> tuple[RecoveredField[FahaSpaceOffsetTenseSyntax], ...]:
        'Ordered sequence of zero or more offsets components.'
        return cast(tuple[RecoveredField[FahaSpaceOffsetTenseSyntax], ...], self._field(1))
    @property
    def interval(self) -> RecoveredField[SpaceIntervalTenseSyntax] | None:
        'The optional interval component.'
        return cast(RecoveredField[SpaceIntervalTenseSyntax] | None, self._field(2))
    @property
    def mohi(self) -> RecoveredField[MohiSpaceOffsetTenseSyntax] | None:
        'The optional mohi component.'
        return cast(RecoveredField[MohiSpaceOffsetTenseSyntax] | None, self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('SpaceTenseWithVaSyntax is final')

@final
class SpaceTenseWithOffsetSyntax(_SyntaxNode):
    'Product node for space tense; preserves `va`, `offsets`, `interval`, and `mohi` in source order.'
    __slots__ = ()
    _schema_id = 573
    __match_args__ = ('va', 'offsets', 'interval', 'mohi')
    def __new__(cls, va: RecoveredField[VaSpaceDistanceTenseSyntax] | None, offsets: Sequence[RecoveredField[FahaSpaceOffsetTenseSyntax]], interval: RecoveredField[SpaceIntervalTenseSyntax] | None, mohi: RecoveredField[MohiSpaceOffsetTenseSyntax] | None) -> SpaceTenseWithOffsetSyntax:
        return cls._from_fields((va, offsets, interval, mohi))
    def __init__(self, va: RecoveredField[VaSpaceDistanceTenseSyntax] | None, offsets: Sequence[RecoveredField[FahaSpaceOffsetTenseSyntax]], interval: RecoveredField[SpaceIntervalTenseSyntax] | None, mohi: RecoveredField[MohiSpaceOffsetTenseSyntax] | None) -> None:
        pass
    @property
    def va(self) -> RecoveredField[VaSpaceDistanceTenseSyntax] | None:
        'The optional va component.'
        return cast(RecoveredField[VaSpaceDistanceTenseSyntax] | None, self._field(0))
    @property
    def offsets(self) -> tuple[RecoveredField[FahaSpaceOffsetTenseSyntax], ...]:
        'Non-empty ordered sequence of offsets components.'
        return cast(tuple[RecoveredField[FahaSpaceOffsetTenseSyntax], ...], self._field(1))
    @property
    def interval(self) -> RecoveredField[SpaceIntervalTenseSyntax] | None:
        'The optional interval component.'
        return cast(RecoveredField[SpaceIntervalTenseSyntax] | None, self._field(2))
    @property
    def mohi(self) -> RecoveredField[MohiSpaceOffsetTenseSyntax] | None:
        'The optional mohi component.'
        return cast(RecoveredField[MohiSpaceOffsetTenseSyntax] | None, self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('SpaceTenseWithOffsetSyntax is final')

@final
class SpaceTenseWithIntervalSyntax(_SyntaxNode):
    'Product node for space tense; preserves `va`, `offsets`, `interval`, and `mohi` in source order.'
    __slots__ = ()
    _schema_id = 574
    __match_args__ = ('va', 'offsets', 'interval', 'mohi')
    def __new__(cls, va: RecoveredField[VaSpaceDistanceTenseSyntax] | None, offsets: Sequence[RecoveredField[FahaSpaceOffsetTenseSyntax]], interval: RecoveredField[SpaceIntervalTenseSyntax], mohi: RecoveredField[MohiSpaceOffsetTenseSyntax] | None) -> SpaceTenseWithIntervalSyntax:
        return cls._from_fields((va, offsets, interval, mohi))
    def __init__(self, va: RecoveredField[VaSpaceDistanceTenseSyntax] | None, offsets: Sequence[RecoveredField[FahaSpaceOffsetTenseSyntax]], interval: RecoveredField[SpaceIntervalTenseSyntax], mohi: RecoveredField[MohiSpaceOffsetTenseSyntax] | None) -> None:
        pass
    @property
    def va(self) -> RecoveredField[VaSpaceDistanceTenseSyntax] | None:
        'The optional va component.'
        return cast(RecoveredField[VaSpaceDistanceTenseSyntax] | None, self._field(0))
    @property
    def offsets(self) -> tuple[RecoveredField[FahaSpaceOffsetTenseSyntax], ...]:
        'Ordered sequence of zero or more offsets components.'
        return cast(tuple[RecoveredField[FahaSpaceOffsetTenseSyntax], ...], self._field(1))
    @property
    def interval(self) -> RecoveredField[SpaceIntervalTenseSyntax]:
        'The shared interval child syntax node.'
        return cast(RecoveredField[SpaceIntervalTenseSyntax], self._field(2))
    @property
    def mohi(self) -> RecoveredField[MohiSpaceOffsetTenseSyntax] | None:
        'The optional mohi component.'
        return cast(RecoveredField[MohiSpaceOffsetTenseSyntax] | None, self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('SpaceTenseWithIntervalSyntax is final')

@final
class SpaceTenseWithMohiSyntax(_SyntaxNode):
    'Product node for space tense; preserves `va`, `offsets`, `interval`, and `mohi` in source order.'
    __slots__ = ()
    _schema_id = 575
    __match_args__ = ('va', 'offsets', 'interval', 'mohi')
    def __new__(cls, va: RecoveredField[VaSpaceDistanceTenseSyntax] | None, offsets: Sequence[RecoveredField[FahaSpaceOffsetTenseSyntax]], interval: RecoveredField[SpaceIntervalTenseSyntax] | None, mohi: RecoveredField[MohiSpaceOffsetTenseSyntax]) -> SpaceTenseWithMohiSyntax:
        return cls._from_fields((va, offsets, interval, mohi))
    def __init__(self, va: RecoveredField[VaSpaceDistanceTenseSyntax] | None, offsets: Sequence[RecoveredField[FahaSpaceOffsetTenseSyntax]], interval: RecoveredField[SpaceIntervalTenseSyntax] | None, mohi: RecoveredField[MohiSpaceOffsetTenseSyntax]) -> None:
        pass
    @property
    def va(self) -> RecoveredField[VaSpaceDistanceTenseSyntax] | None:
        'The optional va component.'
        return cast(RecoveredField[VaSpaceDistanceTenseSyntax] | None, self._field(0))
    @property
    def offsets(self) -> tuple[RecoveredField[FahaSpaceOffsetTenseSyntax], ...]:
        'Ordered sequence of zero or more offsets components.'
        return cast(tuple[RecoveredField[FahaSpaceOffsetTenseSyntax], ...], self._field(1))
    @property
    def interval(self) -> RecoveredField[SpaceIntervalTenseSyntax] | None:
        'The optional interval component.'
        return cast(RecoveredField[SpaceIntervalTenseSyntax] | None, self._field(2))
    @property
    def mohi(self) -> RecoveredField[MohiSpaceOffsetTenseSyntax]:
        'The shared mohi child syntax node.'
        return cast(RecoveredField[MohiSpaceOffsetTenseSyntax], self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('SpaceTenseWithMohiSyntax is final')

@final
class VaSpaceDistanceTenseSyntax(_SyntaxNode):
    'Transparent product node for space tense; preserves the `va` component.'
    __slots__ = ()
    _schema_id = 576
    __match_args__ = ('va',)
    def __new__(cls, va: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> VaSpaceDistanceTenseSyntax:
        return cls._from_fields((va,))
    def __init__(self, va: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def va(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Va`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('VaSpaceDistanceTenseSyntax is final')

@final
class FahaSpaceOffsetTenseSyntax(_SyntaxNode):
    'Product node for space tense; preserves `faha`, `nai`, and `distance` in source order.'
    __slots__ = ()
    _schema_id = 577
    __match_args__ = ('faha', 'nai', 'distance')
    def __new__(cls, faha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, distance: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> FahaSpaceOffsetTenseSyntax:
        return cls._from_fields((faha, nai, distance))
    def __init__(self, faha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, distance: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def faha(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Faha`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def nai(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Nai` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(1))
    @property
    def distance(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional distance component.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('FahaSpaceOffsetTenseSyntax is final')

@final
class FahaIntervalDirectionTenseSyntax(_SyntaxNode):
    'Product node for space interval; preserves `faha` and `nai` in source order.'
    __slots__ = ()
    _schema_id = 578
    __match_args__ = ('faha', 'nai')
    def __new__(cls, faha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> FahaIntervalDirectionTenseSyntax:
        return cls._from_fields((faha, nai))
    def __init__(self, faha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def faha(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Faha`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def nai(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Nai` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('FahaIntervalDirectionTenseSyntax is final')

@final
class SpaceIntervalTenseSyntaxSpaceIntervalWithExtentTense(_SyntaxNode):
    'Uses the `space_interval_with_extent_tense` product form, whose payload preserves `extent`, `direction`, and `properties`.'
    __slots__ = ()
    _schema_id = 579
    __match_args__ = ('space_interval_with_extent_tense',)
    def __new__(cls, space_interval_with_extent_tense: RecoveredField[SpaceIntervalWithExtentTenseSyntax]) -> SpaceIntervalTenseSyntaxSpaceIntervalWithExtentTense:
        return cls._from_fields((space_interval_with_extent_tense,))
    def __init__(self, space_interval_with_extent_tense: RecoveredField[SpaceIntervalWithExtentTenseSyntax]) -> None:
        pass
    @property
    def space_interval_with_extent_tense(self) -> RecoveredField[SpaceIntervalWithExtentTenseSyntax]:
        'Uses the `space_interval_with_extent_tense` product form, whose payload preserves `extent`, `direction`, and `properties`.'
        return cast(RecoveredField[SpaceIntervalWithExtentTenseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SpaceIntervalTenseSyntaxSpaceIntervalWithExtentTense is final')

@final
class SpaceIntervalTenseSyntaxSpaceIntervalPropertiesTense(_SyntaxNode):
    'Uses the `space_interval_properties_tense` product form, whose payload preserves `first` and `additional`.'
    __slots__ = ()
    _schema_id = 580
    __match_args__ = ('space_interval_properties_tense',)
    def __new__(cls, space_interval_properties_tense: RecoveredField[SpaceIntervalPropertiesTenseSyntax]) -> SpaceIntervalTenseSyntaxSpaceIntervalPropertiesTense:
        return cls._from_fields((space_interval_properties_tense,))
    def __init__(self, space_interval_properties_tense: RecoveredField[SpaceIntervalPropertiesTenseSyntax]) -> None:
        pass
    @property
    def space_interval_properties_tense(self) -> RecoveredField[SpaceIntervalPropertiesTenseSyntax]:
        'Uses the `space_interval_properties_tense` product form, whose payload preserves `first` and `additional`.'
        return cast(RecoveredField[SpaceIntervalPropertiesTenseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SpaceIntervalTenseSyntaxSpaceIntervalPropertiesTense is final')

SpaceIntervalTenseSyntax: TypeAlias = SpaceIntervalTenseSyntaxSpaceIntervalWithExtentTense | SpaceIntervalTenseSyntaxSpaceIntervalPropertiesTense

@final
class SpaceIntervalWithExtentTenseSyntax(_SyntaxNode):
    'Product node for space interval; preserves `extent`, `direction`, and `properties` in source order.'
    __slots__ = ()
    _schema_id = 581
    __match_args__ = ('extent', 'direction', 'properties')
    def __new__(cls, extent: RecoveredField[SpaceIntervalExtentTenseSyntax], direction: RecoveredField[FahaIntervalDirectionTenseSyntax] | None, properties: RecoveredField[SpaceIntervalPropertiesTenseSyntax] | None) -> SpaceIntervalWithExtentTenseSyntax:
        return cls._from_fields((extent, direction, properties))
    def __init__(self, extent: RecoveredField[SpaceIntervalExtentTenseSyntax], direction: RecoveredField[FahaIntervalDirectionTenseSyntax] | None, properties: RecoveredField[SpaceIntervalPropertiesTenseSyntax] | None) -> None:
        pass
    @property
    def extent(self) -> RecoveredField[SpaceIntervalExtentTenseSyntax]:
        'The shared extent child syntax node.'
        return cast(RecoveredField[SpaceIntervalExtentTenseSyntax], self._field(0))
    @property
    def direction(self) -> RecoveredField[FahaIntervalDirectionTenseSyntax] | None:
        'The optional direction component.'
        return cast(RecoveredField[FahaIntervalDirectionTenseSyntax] | None, self._field(1))
    @property
    def properties(self) -> RecoveredField[SpaceIntervalPropertiesTenseSyntax] | None:
        'The optional properties component.'
        return cast(RecoveredField[SpaceIntervalPropertiesTenseSyntax] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('SpaceIntervalWithExtentTenseSyntax is final')

@final
class SpaceIntervalExtentTenseSyntaxVehaSpaceIntervalTense(_SyntaxNode):
    'Uses the `veha_space_interval_tense` product form, whose payload preserves `veha` and `viha`.'
    __slots__ = ()
    _schema_id = 582
    __match_args__ = ('veha_space_interval_tense',)
    def __new__(cls, veha_space_interval_tense: RecoveredField[VehaSpaceIntervalTenseSyntax]) -> SpaceIntervalExtentTenseSyntaxVehaSpaceIntervalTense:
        return cls._from_fields((veha_space_interval_tense,))
    def __init__(self, veha_space_interval_tense: RecoveredField[VehaSpaceIntervalTenseSyntax]) -> None:
        pass
    @property
    def veha_space_interval_tense(self) -> RecoveredField[VehaSpaceIntervalTenseSyntax]:
        'Uses the `veha_space_interval_tense` product form, whose payload preserves `veha` and `viha`.'
        return cast(RecoveredField[VehaSpaceIntervalTenseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SpaceIntervalExtentTenseSyntaxVehaSpaceIntervalTense is final')

@final
class SpaceIntervalExtentTenseSyntaxVihaSpaceIntervalTense(_SyntaxNode):
    'Uses the `viha_space_interval_tense` product form, whose payload preserves `viha`.'
    __slots__ = ()
    _schema_id = 583
    __match_args__ = ('viha_space_interval_tense',)
    def __new__(cls, viha_space_interval_tense: RecoveredField[VihaSpaceIntervalTenseSyntax]) -> SpaceIntervalExtentTenseSyntaxVihaSpaceIntervalTense:
        return cls._from_fields((viha_space_interval_tense,))
    def __init__(self, viha_space_interval_tense: RecoveredField[VihaSpaceIntervalTenseSyntax]) -> None:
        pass
    @property
    def viha_space_interval_tense(self) -> RecoveredField[VihaSpaceIntervalTenseSyntax]:
        'Uses the `viha_space_interval_tense` product form, whose payload preserves `viha`.'
        return cast(RecoveredField[VihaSpaceIntervalTenseSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SpaceIntervalExtentTenseSyntaxVihaSpaceIntervalTense is final')

SpaceIntervalExtentTenseSyntax: TypeAlias = SpaceIntervalExtentTenseSyntaxVehaSpaceIntervalTense | SpaceIntervalExtentTenseSyntaxVihaSpaceIntervalTense

@final
class SpaceIntervalPropertiesTenseSyntax(_SyntaxNode):
    'Product node for space interval; preserves `first` and `additional` in source order.'
    __slots__ = ()
    _schema_id = 584
    __match_args__ = ('first', 'additional')
    def __new__(cls, first: RecoveredField[FeheIntervalPropertyTenseSyntax], additional: Sequence[RecoveredField[FeheIntervalPropertyTenseSyntax]]) -> SpaceIntervalPropertiesTenseSyntax:
        return cls._from_fields((first, additional))
    def __init__(self, first: RecoveredField[FeheIntervalPropertyTenseSyntax], additional: Sequence[RecoveredField[FeheIntervalPropertyTenseSyntax]]) -> None:
        pass
    @property
    def first(self) -> RecoveredField[FeheIntervalPropertyTenseSyntax]:
        'The shared first child syntax node.'
        return cast(RecoveredField[FeheIntervalPropertyTenseSyntax], self._field(0))
    @property
    def additional(self) -> tuple[RecoveredField[FeheIntervalPropertyTenseSyntax], ...]:
        'Ordered sequence of zero or more additional components.'
        return cast(tuple[RecoveredField[FeheIntervalPropertyTenseSyntax], ...], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('SpaceIntervalPropertiesTenseSyntax is final')

@final
class VehaSpaceIntervalTenseSyntax(_SyntaxNode):
    'Product node for space interval; preserves `veha` and `viha` in source order.'
    __slots__ = ()
    _schema_id = 585
    __match_args__ = ('veha', 'viha')
    def __new__(cls, veha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], viha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> VehaSpaceIntervalTenseSyntax:
        return cls._from_fields((veha, viha))
    def __init__(self, veha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], viha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def veha(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Veha`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def viha(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional viha component.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('VehaSpaceIntervalTenseSyntax is final')

@final
class VihaSpaceIntervalTenseSyntax(_SyntaxNode):
    'Transparent product node for space interval; preserves the `viha` component.'
    __slots__ = ()
    _schema_id = 586
    __match_args__ = ('viha',)
    def __new__(cls, viha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> VihaSpaceIntervalTenseSyntax:
        return cls._from_fields((viha,))
    def __init__(self, viha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def viha(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Viha`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('VihaSpaceIntervalTenseSyntax is final')

@final
class FeheIntervalPropertyTenseSyntax(_SyntaxNode):
    'Product node for space interval property; preserves `fehe` and `property` in source order.'
    __slots__ = ()
    _schema_id = 587
    __match_args__ = ('fehe', 'property')
    def __new__(cls, fehe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], property: RecoveredField[IntervalPropertyTenseSyntax]) -> FeheIntervalPropertyTenseSyntax:
        return cls._from_fields((fehe, property))
    def __init__(self, fehe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], property: RecoveredField[IntervalPropertyTenseSyntax]) -> None:
        pass
    @property
    def fehe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Fehe` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def property(self) -> RecoveredField[IntervalPropertyTenseSyntax]:
        'The shared property child syntax node.'
        return cast(RecoveredField[IntervalPropertyTenseSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('FeheIntervalPropertyTenseSyntax is final')

@final
class MohiSpaceOffsetTenseSyntax(_SyntaxNode):
    'Product node for space tense; preserves `mohi` and `offset` in source order.'
    __slots__ = ()
    _schema_id = 588
    __match_args__ = ('mohi', 'offset')
    def __new__(cls, mohi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], offset: RecoveredField[FahaSpaceOffsetTenseSyntax]) -> MohiSpaceOffsetTenseSyntax:
        return cls._from_fields((mohi, offset))
    def __init__(self, mohi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], offset: RecoveredField[FahaSpaceOffsetTenseSyntax]) -> None:
        pass
    @property
    def mohi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Mohi`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def offset(self) -> RecoveredField[FahaSpaceOffsetTenseSyntax]:
        'The shared offset child syntax node.'
        return cast(RecoveredField[FahaSpaceOffsetTenseSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('MohiSpaceOffsetTenseSyntax is final')

@final
class CahaTenseSyntax(_SyntaxNode):
    'Transparent product node for tag; preserves the `caha` component.'
    __slots__ = ()
    _schema_id = 589
    __match_args__ = ('caha',)
    def __new__(cls, caha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> CahaTenseSyntax:
        return cls._from_fields((caha,))
    def __init__(self, caha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def caha(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Caha`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('CahaTenseSyntax is final')

@final
class KiCompositeTenseSyntax(_SyntaxNode):
    'Transparent product node for tag; preserves the `ki` component.'
    __slots__ = ()
    _schema_id = 590
    __match_args__ = ('ki',)
    def __new__(cls, ki: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> KiCompositeTenseSyntax:
        return cls._from_fields((ki,))
    def __init__(self, ki: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def ki(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Ki` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('KiCompositeTenseSyntax is final')

@final
class CuheTenseSyntax(_SyntaxNode):
    'Transparent product node for tag; preserves the `cuhe` component.'
    __slots__ = ()
    _schema_id = 591
    __match_args__ = ('cuhe',)
    def __new__(cls, cuhe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> CuheTenseSyntax:
        return cls._from_fields((cuhe,))
    def __init__(self, cuhe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def cuhe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Cuhe`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('CuheTenseSyntax is final')

@final
class ModalTenseSyntax(_SyntaxNode):
    'Product node for modal tag; preserves `nahe`, `se`, `bai`, `nai`, and `ki` in source order.'
    __slots__ = ()
    _schema_id = 592
    __match_args__ = ('nahe', 'se', 'bai', 'nai', 'ki')
    def __new__(cls, nahe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, se: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, bai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, ki: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> ModalTenseSyntax:
        return cls._from_fields((nahe, se, bai, nai, ki))
    def __init__(self, nahe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, se: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, bai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, ki: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def nahe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional nahe component.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(0))
    @property
    def se(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional se component.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(1))
    @property
    def bai(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Bai`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(2))
    @property
    def nai(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Nai` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(3))
    @property
    def ki(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Ki` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(4))
    def __init_subclass__(cls) -> None:
        raise TypeError('ModalTenseSyntax is final')

@final
class StickyTenseSyntax(_SyntaxNode):
    'Transparent product node for tag; preserves the `ki` component.'
    __slots__ = ()
    _schema_id = 593
    __match_args__ = ('ki',)
    def __new__(cls, ki: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> StickyTenseSyntax:
        return cls._from_fields((ki,))
    def __init__(self, ki: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def ki(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Ki` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('StickyTenseSyntax is final')

@final
class SelbriSyntaxTaggedSelbri(_SyntaxNode):
    'Uses the `tagged_selbri` product form, whose payload preserves `tense_modal` and `inner_selbri`.'
    __slots__ = ()
    _schema_id = 594
    __match_args__ = ('tagged_selbri',)
    def __new__(cls, tagged_selbri: RecoveredField[TaggedSelbriSyntax]) -> SelbriSyntaxTaggedSelbri:
        return cls._from_fields((tagged_selbri,))
    def __init__(self, tagged_selbri: RecoveredField[TaggedSelbriSyntax]) -> None:
        pass
    @property
    def tagged_selbri(self) -> RecoveredField[TaggedSelbriSyntax]:
        'Uses the `tagged_selbri` product form, whose payload preserves `tense_modal` and `inner_selbri`.'
        return cast(RecoveredField[TaggedSelbriSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SelbriSyntaxTaggedSelbri is final')

@final
class SelbriSyntaxUntaggedSelbri(_SyntaxNode):
    'Uses the nested `untagged_selbri` sum form and preserves its selected alternative.'
    __slots__ = ()
    _schema_id = 595
    __match_args__ = ('untagged_selbri',)
    def __new__(cls, untagged_selbri: RecoveredField[UntaggedSelbriSyntax]) -> SelbriSyntaxUntaggedSelbri:
        return cls._from_fields((untagged_selbri,))
    def __init__(self, untagged_selbri: RecoveredField[UntaggedSelbriSyntax]) -> None:
        pass
    @property
    def untagged_selbri(self) -> RecoveredField[UntaggedSelbriSyntax]:
        'Uses the nested `untagged_selbri` sum form and preserves its selected alternative.'
        return cast(RecoveredField[UntaggedSelbriSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SelbriSyntaxUntaggedSelbri is final')

SelbriSyntax: TypeAlias = SelbriSyntaxTaggedSelbri | SelbriSyntaxUntaggedSelbri

@final
class UntaggedSelbriSyntaxNegatedSelbri(_SyntaxNode):
    'Uses the `negated_selbri` product form, whose payload preserves `na` and `inner_selbri`.'
    __slots__ = ()
    _schema_id = 596
    __match_args__ = ('negated_selbri',)
    def __new__(cls, negated_selbri: RecoveredField[NegatedSelbriSyntax]) -> UntaggedSelbriSyntaxNegatedSelbri:
        return cls._from_fields((negated_selbri,))
    def __init__(self, negated_selbri: RecoveredField[NegatedSelbriSyntax]) -> None:
        pass
    @property
    def negated_selbri(self) -> RecoveredField[NegatedSelbriSyntax]:
        'Uses the `negated_selbri` product form, whose payload preserves `na` and `inner_selbri`.'
        return cast(RecoveredField[NegatedSelbriSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('UntaggedSelbriSyntaxNegatedSelbri is final')

@final
class UntaggedSelbriSyntaxCoSelbri(_SyntaxNode):
    'Uses the `co_selbri` product form, whose payload preserves `leading_selbri` and `co_tail`.'
    __slots__ = ()
    _schema_id = 597
    __match_args__ = ('co_selbri',)
    def __new__(cls, co_selbri: RecoveredField[CoSelbriSyntax]) -> UntaggedSelbriSyntaxCoSelbri:
        return cls._from_fields((co_selbri,))
    def __init__(self, co_selbri: RecoveredField[CoSelbriSyntax]) -> None:
        pass
    @property
    def co_selbri(self) -> RecoveredField[CoSelbriSyntax]:
        'Uses the `co_selbri` product form, whose payload preserves `leading_selbri` and `co_tail`.'
        return cast(RecoveredField[CoSelbriSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('UntaggedSelbriSyntaxCoSelbri is final')

@final
class UntaggedSelbriSyntaxForethoughtSelbriConnection(_SyntaxNode):
    'Uses the `forethought_selbri_connection` product form, whose payload preserves `guhek`, `leading_selbri`, `first_branch`, `additional_branches`, and `gihi`.'
    __slots__ = ()
    _schema_id = 598
    __match_args__ = ('forethought_selbri_connection',)
    def __new__(cls, forethought_selbri_connection: RecoveredField[ForethoughtSelbriConnectionSyntax]) -> UntaggedSelbriSyntaxForethoughtSelbriConnection:
        return cls._from_fields((forethought_selbri_connection,))
    def __init__(self, forethought_selbri_connection: RecoveredField[ForethoughtSelbriConnectionSyntax]) -> None:
        pass
    @property
    def forethought_selbri_connection(self) -> RecoveredField[ForethoughtSelbriConnectionSyntax]:
        'Uses the `forethought_selbri_connection` product form, whose payload preserves `guhek`, `leading_selbri`, `first_branch`, `additional_branches`, and `gihi`.'
        return cast(RecoveredField[ForethoughtSelbriConnectionSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('UntaggedSelbriSyntaxForethoughtSelbriConnection is final')

UntaggedSelbriSyntax: TypeAlias = UntaggedSelbriSyntaxNegatedSelbri | UntaggedSelbriSyntaxCoSelbri | UntaggedSelbriSyntaxForethoughtSelbriConnection

@final
class TaggedSelbriSyntax(_SyntaxNode):
    'Product node for tagged selbri; preserves `tense_modal` and `inner_selbri` in source order.'
    __slots__ = ()
    _schema_id = 599
    __match_args__ = ('tense_modal', 'inner_selbri')
    def __new__(cls, tense_modal: RecoveredField[TenseModalSyntax], inner_selbri: RecoveredField[UntaggedSelbriSyntax]) -> TaggedSelbriSyntax:
        return cls._from_fields((tense_modal, inner_selbri))
    def __init__(self, tense_modal: RecoveredField[TenseModalSyntax], inner_selbri: RecoveredField[UntaggedSelbriSyntax]) -> None:
        pass
    @property
    def tense_modal(self) -> RecoveredField[TenseModalSyntax]:
        'The shared tense modal child syntax node.'
        return cast(RecoveredField[TenseModalSyntax], self._field(0))
    @property
    def inner_selbri(self) -> RecoveredField[UntaggedSelbriSyntax]:
        'The shared inner selbri child syntax node.'
        return cast(RecoveredField[UntaggedSelbriSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('TaggedSelbriSyntax is final')

@final
class NegatedSelbriSyntax(_SyntaxNode):
    'Product node for negated selbri; preserves `na` and `inner_selbri` in source order.'
    __slots__ = ()
    _schema_id = 600
    __match_args__ = ('na', 'inner_selbri')
    def __new__(cls, na: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_selbri: RecoveredField[SelbriSyntax]) -> NegatedSelbriSyntax:
        return cls._from_fields((na, inner_selbri))
    def __init__(self, na: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_selbri: RecoveredField[SelbriSyntax]) -> None:
        pass
    @property
    def na(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Na`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def inner_selbri(self) -> RecoveredField[SelbriSyntax]:
        'The shared inner selbri child syntax node.'
        return cast(RecoveredField[SelbriSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('NegatedSelbriSyntax is final')

@final
class CoSelbriSyntax(_SyntaxNode):
    'Product node for selbri; preserves `leading_selbri` and `co_tail` in source order.'
    __slots__ = ()
    _schema_id = 601
    __match_args__ = ('leading_selbri', 'co_tail')
    def __new__(cls, leading_selbri: RecoveredField[ConnectedSelbriSyntax], co_tail: RecoveredField[CoSelbriTailSyntax] | None) -> CoSelbriSyntax:
        return cls._from_fields((leading_selbri, co_tail))
    def __init__(self, leading_selbri: RecoveredField[ConnectedSelbriSyntax], co_tail: RecoveredField[CoSelbriTailSyntax] | None) -> None:
        pass
    @property
    def leading_selbri(self) -> RecoveredField[ConnectedSelbriSyntax]:
        'The shared leading selbri child syntax node.'
        return cast(RecoveredField[ConnectedSelbriSyntax], self._field(0))
    @property
    def co_tail(self) -> RecoveredField[CoSelbriTailSyntax] | None:
        'The optional co tail component.'
        return cast(RecoveredField[CoSelbriTailSyntax] | None, self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('CoSelbriSyntax is final')

@final
class CoSelbriTailSyntax(_SyntaxNode):
    'Product node for selbri; preserves `co` and `trailing_selbri` in source order.'
    __slots__ = ()
    _schema_id = 602
    __match_args__ = ('co', 'trailing_selbri')
    def __new__(cls, co: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], trailing_selbri: RecoveredField[CoSelbriSyntax]) -> CoSelbriTailSyntax:
        return cls._from_fields((co, trailing_selbri))
    def __init__(self, co: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], trailing_selbri: RecoveredField[CoSelbriSyntax]) -> None:
        pass
    @property
    def co(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Co` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def trailing_selbri(self) -> RecoveredField[CoSelbriSyntax]:
        'The shared trailing selbri child syntax node.'
        return cast(RecoveredField[CoSelbriSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('CoSelbriTailSyntax is final')

@final
class ForethoughtSelbriConnectionSyntax(_SyntaxNode):
    'Product node for forethought selbri connection; preserves `guhek`, `leading_selbri`, `first_branch`, `additional_branches`, and `gihi` in source order.'
    __slots__ = ()
    _schema_id = 603
    __match_args__ = ('guhek', 'leading_selbri', 'first_branch', 'additional_branches', 'gihi')
    def __new__(cls, guhek: RecoveredField[GuhekConnectiveSyntax], leading_selbri: RecoveredField[SelbriSyntax], first_branch: RecoveredField[ForethoughtSelbriBranchSyntax], additional_branches: Sequence[RecoveredField[ZantufaForethoughtSelbriBranchSyntax]], gihi: RecoveredField[Token] | None) -> ForethoughtSelbriConnectionSyntax:
        return cls._from_fields((guhek, leading_selbri, first_branch, additional_branches, gihi))
    def __init__(self, guhek: RecoveredField[GuhekConnectiveSyntax], leading_selbri: RecoveredField[SelbriSyntax], first_branch: RecoveredField[ForethoughtSelbriBranchSyntax], additional_branches: Sequence[RecoveredField[ZantufaForethoughtSelbriBranchSyntax]], gihi: RecoveredField[Token] | None) -> None:
        pass
    @property
    def guhek(self) -> RecoveredField[GuhekConnectiveSyntax]:
        'The `guhek_connective` forethought connective opening the paired branches of the `forethought_selbri_connection` production.'
        return cast(RecoveredField[GuhekConnectiveSyntax], self._field(0))
    @property
    def leading_selbri(self) -> RecoveredField[SelbriSyntax]:
        'The shared leading selbri child syntax node.'
        return cast(RecoveredField[SelbriSyntax], self._field(1))
    @property
    def first_branch(self) -> RecoveredField[ForethoughtSelbriBranchSyntax]:
        'The initial `forethought_selbri_branch` constituent before the continuations of the `forethought_selbri_connection` production.'
        return cast(RecoveredField[ForethoughtSelbriBranchSyntax], self._field(2))
    @property
    def additional_branches(self) -> tuple[RecoveredField[ZantufaForethoughtSelbriBranchSyntax], ...]:
        'Ordered sequence of zero or more additional branches components.'
        return cast(tuple[RecoveredField[ZantufaForethoughtSelbriBranchSyntax], ...], self._field(3))
    @property
    def gihi(self) -> RecoveredField[Token] | None:
        'The optional gihi component.'
        return cast(RecoveredField[Token] | None, self._field(4))
    def __init_subclass__(cls) -> None:
        raise TypeError('ForethoughtSelbriConnectionSyntax is final')

@final
class ForethoughtSelbriBranchSyntax(_SyntaxNode):
    'Product node for forethought selbri connection; preserves `gik` and `selbri` in source order.'
    __slots__ = ()
    _schema_id = 604
    __match_args__ = ('gik', 'selbri')
    def __new__(cls, gik: RecoveredField[GikConnectiveSyntax], selbri: RecoveredField[SelbriSyntax]) -> ForethoughtSelbriBranchSyntax:
        return cls._from_fields((gik, selbri))
    def __init__(self, gik: RecoveredField[GikConnectiveSyntax], selbri: RecoveredField[SelbriSyntax]) -> None:
        pass
    @property
    def gik(self) -> RecoveredField[GikConnectiveSyntax]:
        'The GI-family `gik_connective` connective separating the forethought branches of the `forethought_selbri_branch` production.'
        return cast(RecoveredField[GikConnectiveSyntax], self._field(0))
    @property
    def selbri(self) -> RecoveredField[SelbriSyntax]:
        'The shared selbri child syntax node.'
        return cast(RecoveredField[SelbriSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ForethoughtSelbriBranchSyntax is final')

@final
class ZantufaForethoughtSelbriBranchSyntax(_SyntaxNode):
    'Product node for forethought selbri connection; preserves `gik` and `selbri` in source order.'
    __slots__ = ()
    _schema_id = 605
    __match_args__ = ('gik', 'selbri')
    def __new__(cls, gik: RecoveredField[ZantufaExtraGikConnectiveSyntax], selbri: RecoveredField[SelbriSyntax]) -> ZantufaForethoughtSelbriBranchSyntax:
        return cls._from_fields((gik, selbri))
    def __init__(self, gik: RecoveredField[ZantufaExtraGikConnectiveSyntax], selbri: RecoveredField[SelbriSyntax]) -> None:
        pass
    @property
    def gik(self) -> RecoveredField[ZantufaExtraGikConnectiveSyntax]:
        'The GI-family `zantufa_extra_gik_connective` connective separating the forethought branches of the `zantufa_forethought_selbri_branch` production.'
        return cast(RecoveredField[ZantufaExtraGikConnectiveSyntax], self._field(0))
    @property
    def selbri(self) -> RecoveredField[SelbriSyntax]:
        'The shared selbri child syntax node.'
        return cast(RecoveredField[SelbriSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaForethoughtSelbriBranchSyntax is final')

@final
class ConnectedSelbriSyntax(_SyntaxNode):
    'Product node for selbri connection; preserves `leading_selbri` and `continuations` in source order.'
    __slots__ = ()
    _schema_id = 606
    __match_args__ = ('leading_selbri', 'continuations')
    def __new__(cls, leading_selbri: RecoveredField[TanruSelbriSyntax], continuations: Sequence[RecoveredField[ConnectedSelbriContinuationSyntax]]) -> ConnectedSelbriSyntax:
        return cls._from_fields((leading_selbri, continuations))
    def __init__(self, leading_selbri: RecoveredField[TanruSelbriSyntax], continuations: Sequence[RecoveredField[ConnectedSelbriContinuationSyntax]]) -> None:
        pass
    @property
    def leading_selbri(self) -> RecoveredField[TanruSelbriSyntax]:
        'The shared leading selbri child syntax node.'
        return cast(RecoveredField[TanruSelbriSyntax], self._field(0))
    @property
    def continuations(self) -> tuple[RecoveredField[ConnectedSelbriContinuationSyntax], ...]:
        'Ordered sequence of zero or more continuations components.'
        return cast(tuple[RecoveredField[ConnectedSelbriContinuationSyntax], ...], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ConnectedSelbriSyntax is final')

@final
class ConnectedSelbriContinuationSyntax(_SyntaxNode):
    'Product node for selbri connection continuation; preserves `connective` and `trailing_selbri` in source order.'
    __slots__ = ()
    _schema_id = 607
    __match_args__ = ('connective', 'trailing_selbri')
    def __new__(cls, connective: RecoveredField[RelationAfterthoughtConnectiveSyntax], trailing_selbri: RecoveredField[TanruSelbriSyntax]) -> ConnectedSelbriContinuationSyntax:
        return cls._from_fields((connective, trailing_selbri))
    def __init__(self, connective: RecoveredField[RelationAfterthoughtConnectiveSyntax], trailing_selbri: RecoveredField[TanruSelbriSyntax]) -> None:
        pass
    @property
    def connective(self) -> RecoveredField[RelationAfterthoughtConnectiveSyntax]:
        'The `relation_afterthought_connective` connective joining the adjacent constituents of the `connected_selbri_continuation` production.'
        return cast(RecoveredField[RelationAfterthoughtConnectiveSyntax], self._field(0))
    @property
    def trailing_selbri(self) -> RecoveredField[TanruSelbriSyntax]:
        'The shared trailing selbri child syntax node.'
        return cast(RecoveredField[TanruSelbriSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ConnectedSelbriContinuationSyntax is final')

@final
class TanruSelbriSyntax(_SyntaxNode):
    'Product node for tanru; preserves `first_unit` and `additional_units` in source order.'
    __slots__ = ()
    _schema_id = 608
    __match_args__ = ('first_unit', 'additional_units')
    def __new__(cls, first_unit: RecoveredField[TanruUnitSyntax], additional_units: Sequence[RecoveredField[TanruUnitSyntax]]) -> TanruSelbriSyntax:
        return cls._from_fields((first_unit, additional_units))
    def __init__(self, first_unit: RecoveredField[TanruUnitSyntax], additional_units: Sequence[RecoveredField[TanruUnitSyntax]]) -> None:
        pass
    @property
    def first_unit(self) -> RecoveredField[TanruUnitSyntax]:
        'The initial `tanru_unit` constituent before the continuations of the `tanru_selbri` production.'
        return cast(RecoveredField[TanruUnitSyntax], self._field(0))
    @property
    def additional_units(self) -> tuple[RecoveredField[TanruUnitSyntax], ...]:
        'Ordered sequence of zero or more additional units components.'
        return cast(tuple[RecoveredField[TanruUnitSyntax], ...], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruSelbriSyntax is final')

@final
class TanruUnitSyntax(_SyntaxNode):
    'Transparent product node for tanru unit; preserves the `units` component.'
    __slots__ = ()
    _schema_id = 609
    __match_args__ = ('units',)
    def __new__(cls, units: Chain[RecoveredField[BoOrLinkedTanruUnitSyntax], RecoveredField[TanruUnitContinuationSyntax]]) -> TanruUnitSyntax:
        return cls._from_fields((units,))
    def __init__(self, units: Chain[RecoveredField[BoOrLinkedTanruUnitSyntax], RecoveredField[TanruUnitContinuationSyntax]]) -> None:
        pass
    @property
    def units(self) -> Chain[RecoveredField[BoOrLinkedTanruUnitSyntax], RecoveredField[TanruUnitContinuationSyntax]]:
        'The source-ordered `units` chain assembled by the `tanru_unit` production.'
        return cast(Chain[RecoveredField[BoOrLinkedTanruUnitSyntax], RecoveredField[TanruUnitContinuationSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitSyntax is final')

@final
class TanruUnitContinuationSyntax(_SyntaxNode):
    'Product node for tanru unit continuation; preserves `connective` and `trailing_unit` in source order.'
    __slots__ = ()
    _schema_id = 610
    __match_args__ = ('connective', 'trailing_unit')
    def __new__(cls, connective: RecoveredField[RelationAfterthoughtConnectiveSyntax], trailing_unit: RecoveredField[BoOrLinkedTanruUnitSyntax]) -> TanruUnitContinuationSyntax:
        return cls._from_fields((connective, trailing_unit))
    def __init__(self, connective: RecoveredField[RelationAfterthoughtConnectiveSyntax], trailing_unit: RecoveredField[BoOrLinkedTanruUnitSyntax]) -> None:
        pass
    @property
    def connective(self) -> RecoveredField[RelationAfterthoughtConnectiveSyntax]:
        'The `relation_afterthought_connective` connective joining the adjacent constituents of the `tanru_unit_continuation` production.'
        return cast(RecoveredField[RelationAfterthoughtConnectiveSyntax], self._field(0))
    @property
    def trailing_unit(self) -> RecoveredField[BoOrLinkedTanruUnitSyntax]:
        'The shared trailing unit child syntax node.'
        return cast(RecoveredField[BoOrLinkedTanruUnitSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitContinuationSyntax is final')

@final
class BoOrLinkedTanruUnitSyntaxForethoughtSelbriGroupTanruUnit(_SyntaxNode):
    'Uses the `forethought_selbri_group_tanru_unit` product form, whose payload preserves `guhek`, `leading_selbri`, `first_branch`, `additional_branches`, and `gihi`.'
    __slots__ = ()
    _schema_id = 611
    __match_args__ = ('forethought_selbri_group_tanru_unit',)
    def __new__(cls, forethought_selbri_group_tanru_unit: RecoveredField[ForethoughtSelbriGroupTanruUnitSyntax]) -> BoOrLinkedTanruUnitSyntaxForethoughtSelbriGroupTanruUnit:
        return cls._from_fields((forethought_selbri_group_tanru_unit,))
    def __init__(self, forethought_selbri_group_tanru_unit: RecoveredField[ForethoughtSelbriGroupTanruUnitSyntax]) -> None:
        pass
    @property
    def forethought_selbri_group_tanru_unit(self) -> RecoveredField[ForethoughtSelbriGroupTanruUnitSyntax]:
        'Uses the `forethought_selbri_group_tanru_unit` product form, whose payload preserves `guhek`, `leading_selbri`, `first_branch`, `additional_branches`, and `gihi`.'
        return cast(RecoveredField[ForethoughtSelbriGroupTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('BoOrLinkedTanruUnitSyntaxForethoughtSelbriGroupTanruUnit is final')

@final
class BoOrLinkedTanruUnitSyntaxBoundTanruUnit(_SyntaxNode):
    'Uses the `bound_tanru_unit` product form, whose payload preserves `leading_unit`, `bo_connective`, `bo_tense_modal`, `bo`, and `trailing_unit`.'
    __slots__ = ()
    _schema_id = 612
    __match_args__ = ('bound_tanru_unit',)
    def __new__(cls, bound_tanru_unit: RecoveredField[BoundTanruUnitSyntax]) -> BoOrLinkedTanruUnitSyntaxBoundTanruUnit:
        return cls._from_fields((bound_tanru_unit,))
    def __init__(self, bound_tanru_unit: RecoveredField[BoundTanruUnitSyntax]) -> None:
        pass
    @property
    def bound_tanru_unit(self) -> RecoveredField[BoundTanruUnitSyntax]:
        'Uses the `bound_tanru_unit` product form, whose payload preserves `leading_unit`, `bo_connective`, `bo_tense_modal`, `bo`, and `trailing_unit`.'
        return cast(RecoveredField[BoundTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('BoOrLinkedTanruUnitSyntaxBoundTanruUnit is final')

@final
class BoOrLinkedTanruUnitSyntaxAssignedProBridiTanruUnit(_SyntaxNode):
    'Uses the `assigned_pro_bridi_tanru_unit` product form, whose payload preserves `base` and `assignments`.'
    __slots__ = ()
    _schema_id = 613
    __match_args__ = ('assigned_pro_bridi_tanru_unit',)
    def __new__(cls, assigned_pro_bridi_tanru_unit: RecoveredField[AssignedProBridiTanruUnitSyntax]) -> BoOrLinkedTanruUnitSyntaxAssignedProBridiTanruUnit:
        return cls._from_fields((assigned_pro_bridi_tanru_unit,))
    def __init__(self, assigned_pro_bridi_tanru_unit: RecoveredField[AssignedProBridiTanruUnitSyntax]) -> None:
        pass
    @property
    def assigned_pro_bridi_tanru_unit(self) -> RecoveredField[AssignedProBridiTanruUnitSyntax]:
        'Uses the `assigned_pro_bridi_tanru_unit` product form, whose payload preserves `base` and `assignments`.'
        return cast(RecoveredField[AssignedProBridiTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('BoOrLinkedTanruUnitSyntaxAssignedProBridiTanruUnit is final')

@final
class BoOrLinkedTanruUnitSyntaxLinkedTanruUnit(_SyntaxNode):
    'Uses the `linked_tanru_unit` product form, whose payload preserves `base` and `linkargs`.'
    __slots__ = ()
    _schema_id = 614
    __match_args__ = ('linked_tanru_unit',)
    def __new__(cls, linked_tanru_unit: RecoveredField[LinkedTanruUnitSyntax]) -> BoOrLinkedTanruUnitSyntaxLinkedTanruUnit:
        return cls._from_fields((linked_tanru_unit,))
    def __init__(self, linked_tanru_unit: RecoveredField[LinkedTanruUnitSyntax]) -> None:
        pass
    @property
    def linked_tanru_unit(self) -> RecoveredField[LinkedTanruUnitSyntax]:
        'Uses the `linked_tanru_unit` product form, whose payload preserves `base` and `linkargs`.'
        return cast(RecoveredField[LinkedTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('BoOrLinkedTanruUnitSyntaxLinkedTanruUnit is final')

BoOrLinkedTanruUnitSyntax: TypeAlias = BoOrLinkedTanruUnitSyntaxForethoughtSelbriGroupTanruUnit | BoOrLinkedTanruUnitSyntaxBoundTanruUnit | BoOrLinkedTanruUnitSyntaxAssignedProBridiTanruUnit | BoOrLinkedTanruUnitSyntaxLinkedTanruUnit

@final
class ForethoughtSelbriGroupTanruUnitSyntax(_SyntaxNode):
    'Product node for forethought selbri connection; preserves `guhek`, `leading_selbri`, `first_branch`, `additional_branches`, and `gihi` in source order.'
    __slots__ = ()
    _schema_id = 615
    __match_args__ = ('guhek', 'leading_selbri', 'first_branch', 'additional_branches', 'gihi')
    def __new__(cls, guhek: RecoveredField[GuhekConnectiveSyntax], leading_selbri: RecoveredField[SelbriSyntax], first_branch: RecoveredField[ForethoughtSelbriGroupBranchSyntax], additional_branches: Sequence[RecoveredField[ZantufaForethoughtSelbriGroupBranchSyntax]], gihi: RecoveredField[Token] | None) -> ForethoughtSelbriGroupTanruUnitSyntax:
        return cls._from_fields((guhek, leading_selbri, first_branch, additional_branches, gihi))
    def __init__(self, guhek: RecoveredField[GuhekConnectiveSyntax], leading_selbri: RecoveredField[SelbriSyntax], first_branch: RecoveredField[ForethoughtSelbriGroupBranchSyntax], additional_branches: Sequence[RecoveredField[ZantufaForethoughtSelbriGroupBranchSyntax]], gihi: RecoveredField[Token] | None) -> None:
        pass
    @property
    def guhek(self) -> RecoveredField[GuhekConnectiveSyntax]:
        'The `guhek_connective` forethought connective opening the paired branches of the `forethought_selbri_group_tanru_unit` production.'
        return cast(RecoveredField[GuhekConnectiveSyntax], self._field(0))
    @property
    def leading_selbri(self) -> RecoveredField[SelbriSyntax]:
        'The shared leading selbri child syntax node.'
        return cast(RecoveredField[SelbriSyntax], self._field(1))
    @property
    def first_branch(self) -> RecoveredField[ForethoughtSelbriGroupBranchSyntax]:
        'The initial `forethought_selbri_group_branch` constituent before the continuations of the `forethought_selbri_group_tanru_unit` production.'
        return cast(RecoveredField[ForethoughtSelbriGroupBranchSyntax], self._field(2))
    @property
    def additional_branches(self) -> tuple[RecoveredField[ZantufaForethoughtSelbriGroupBranchSyntax], ...]:
        'Ordered sequence of zero or more additional branches components.'
        return cast(tuple[RecoveredField[ZantufaForethoughtSelbriGroupBranchSyntax], ...], self._field(3))
    @property
    def gihi(self) -> RecoveredField[Token] | None:
        'The optional gihi component.'
        return cast(RecoveredField[Token] | None, self._field(4))
    def __init_subclass__(cls) -> None:
        raise TypeError('ForethoughtSelbriGroupTanruUnitSyntax is final')

@final
class ForethoughtSelbriGroupBranchSyntax(_SyntaxNode):
    'Product node for forethought selbri connection; preserves `gik` and `unit` in source order.'
    __slots__ = ()
    _schema_id = 616
    __match_args__ = ('gik', 'unit')
    def __new__(cls, gik: RecoveredField[GikConnectiveSyntax], unit: RecoveredField[BoOrLinkedTanruUnitSyntax]) -> ForethoughtSelbriGroupBranchSyntax:
        return cls._from_fields((gik, unit))
    def __init__(self, gik: RecoveredField[GikConnectiveSyntax], unit: RecoveredField[BoOrLinkedTanruUnitSyntax]) -> None:
        pass
    @property
    def gik(self) -> RecoveredField[GikConnectiveSyntax]:
        'The GI-family `gik_connective` connective separating the forethought branches of the `forethought_selbri_group_branch` production.'
        return cast(RecoveredField[GikConnectiveSyntax], self._field(0))
    @property
    def unit(self) -> RecoveredField[BoOrLinkedTanruUnitSyntax]:
        'The shared unit child syntax node.'
        return cast(RecoveredField[BoOrLinkedTanruUnitSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ForethoughtSelbriGroupBranchSyntax is final')

@final
class ZantufaForethoughtSelbriGroupBranchSyntax(_SyntaxNode):
    'Product node for forethought selbri connection; preserves `gik` and `unit` in source order.'
    __slots__ = ()
    _schema_id = 617
    __match_args__ = ('gik', 'unit')
    def __new__(cls, gik: RecoveredField[ZantufaExtraGikConnectiveSyntax], unit: RecoveredField[BoOrLinkedTanruUnitSyntax]) -> ZantufaForethoughtSelbriGroupBranchSyntax:
        return cls._from_fields((gik, unit))
    def __init__(self, gik: RecoveredField[ZantufaExtraGikConnectiveSyntax], unit: RecoveredField[BoOrLinkedTanruUnitSyntax]) -> None:
        pass
    @property
    def gik(self) -> RecoveredField[ZantufaExtraGikConnectiveSyntax]:
        'The GI-family `zantufa_extra_gik_connective` connective separating the forethought branches of the `zantufa_forethought_selbri_group_branch` production.'
        return cast(RecoveredField[ZantufaExtraGikConnectiveSyntax], self._field(0))
    @property
    def unit(self) -> RecoveredField[BoOrLinkedTanruUnitSyntax]:
        'The shared unit child syntax node.'
        return cast(RecoveredField[BoOrLinkedTanruUnitSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaForethoughtSelbriGroupBranchSyntax is final')

@final
class BoundTanruUnitSyntax(_SyntaxNode):
    'Product node for BO-grouped tanru unit; preserves `leading_unit`, `bo_connective`, `bo_tense_modal`, `bo`, and `trailing_unit` in source order.'
    __slots__ = ()
    _schema_id = 618
    __match_args__ = ('leading_unit', 'bo_connective', 'bo_tense_modal', 'bo', 'trailing_unit')
    def __new__(cls, leading_unit: RecoveredField[LinkedTanruUnitSyntax], bo_connective: RecoveredField[RelationAfterthoughtConnectiveSyntax] | None, bo_tense_modal: RecoveredField[TenseModalSyntax] | None, bo: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], trailing_unit: RecoveredField[BoOrLinkedTanruUnitSyntax]) -> BoundTanruUnitSyntax:
        return cls._from_fields((leading_unit, bo_connective, bo_tense_modal, bo, trailing_unit))
    def __init__(self, leading_unit: RecoveredField[LinkedTanruUnitSyntax], bo_connective: RecoveredField[RelationAfterthoughtConnectiveSyntax] | None, bo_tense_modal: RecoveredField[TenseModalSyntax] | None, bo: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], trailing_unit: RecoveredField[BoOrLinkedTanruUnitSyntax]) -> None:
        pass
    @property
    def leading_unit(self) -> RecoveredField[LinkedTanruUnitSyntax]:
        'The shared leading unit child syntax node.'
        return cast(RecoveredField[LinkedTanruUnitSyntax], self._field(0))
    @property
    def bo_connective(self) -> RecoveredField[RelationAfterthoughtConnectiveSyntax] | None:
        'The optional bo connective component.'
        return cast(RecoveredField[RelationAfterthoughtConnectiveSyntax] | None, self._field(1))
    @property
    def bo_tense_modal(self) -> RecoveredField[TenseModalSyntax] | None:
        'The optional bo tense modal component.'
        return cast(RecoveredField[TenseModalSyntax] | None, self._field(2))
    @property
    def bo(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Bo` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(3))
    @property
    def trailing_unit(self) -> RecoveredField[BoOrLinkedTanruUnitSyntax]:
        'The shared trailing unit child syntax node.'
        return cast(RecoveredField[BoOrLinkedTanruUnitSyntax], self._field(4))
    def __init_subclass__(cls) -> None:
        raise TypeError('BoundTanruUnitSyntax is final')

@final
class AssignedProBridiTanruUnitSyntax(_SyntaxNode):
    'Product node for pro-bridi assignment; preserves `base` and `assignments` in source order.'
    __slots__ = ()
    _schema_id = 619
    __match_args__ = ('base', 'assignments')
    def __new__(cls, base: RecoveredField[LinkedTanruUnitForCeiSyntax], assignments: Sequence[RecoveredField[ProBridiTanruUnitAssignmentSyntax]]) -> AssignedProBridiTanruUnitSyntax:
        return cls._from_fields((base, assignments))
    def __init__(self, base: RecoveredField[LinkedTanruUnitForCeiSyntax], assignments: Sequence[RecoveredField[ProBridiTanruUnitAssignmentSyntax]]) -> None:
        pass
    @property
    def base(self) -> RecoveredField[LinkedTanruUnitForCeiSyntax]:
        'The shared base child syntax node.'
        return cast(RecoveredField[LinkedTanruUnitForCeiSyntax], self._field(0))
    @property
    def assignments(self) -> tuple[RecoveredField[ProBridiTanruUnitAssignmentSyntax], ...]:
        'Non-empty ordered sequence of assignments components.'
        return cast(tuple[RecoveredField[ProBridiTanruUnitAssignmentSyntax], ...], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('AssignedProBridiTanruUnitSyntax is final')

@final
class ProBridiTanruUnitAssignmentSyntax(_SyntaxNode):
    'Product node for pro-bridi assignment; preserves `cei` and `tanru_unit` in source order.'
    __slots__ = ()
    _schema_id = 620
    __match_args__ = ('cei', 'tanru_unit')
    def __new__(cls, cei: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], tanru_unit: RecoveredField[LinkedTanruUnitForCeiSyntax]) -> ProBridiTanruUnitAssignmentSyntax:
        return cls._from_fields((cei, tanru_unit))
    def __init__(self, cei: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], tanru_unit: RecoveredField[LinkedTanruUnitForCeiSyntax]) -> None:
        pass
    @property
    def cei(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Cei` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def tanru_unit(self) -> RecoveredField[LinkedTanruUnitForCeiSyntax]:
        'The shared tanru unit child syntax node.'
        return cast(RecoveredField[LinkedTanruUnitForCeiSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ProBridiTanruUnitAssignmentSyntax is final')

@final
class LinkedTanruUnitSyntax(_SyntaxNode):
    'Product node for tanru unit; preserves `base` and `linkargs` in source order.'
    __slots__ = ()
    _schema_id = 621
    __match_args__ = ('base', 'linkargs')
    def __new__(cls, base: RecoveredField[TanruUnitAtomSyntax], linkargs: RecoveredField[LinkargsSyntax] | None) -> LinkedTanruUnitSyntax:
        return cls._from_fields((base, linkargs))
    def __init__(self, base: RecoveredField[TanruUnitAtomSyntax], linkargs: RecoveredField[LinkargsSyntax] | None) -> None:
        pass
    @property
    def base(self) -> RecoveredField[TanruUnitAtomSyntax]:
        'The shared base child syntax node.'
        return cast(RecoveredField[TanruUnitAtomSyntax], self._field(0))
    @property
    def linkargs(self) -> RecoveredField[LinkargsSyntax] | None:
        'The optional linkargs component.'
        return cast(RecoveredField[LinkargsSyntax] | None, self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('LinkedTanruUnitSyntax is final')

@final
class LinkedTanruUnitForCeiSyntax(_SyntaxNode):
    'Product node for tanru unit; preserves `base` and `linkargs` in source order.'
    __slots__ = ()
    _schema_id = 622
    __match_args__ = ('base', 'linkargs')
    def __new__(cls, base: RecoveredField[TanruUnitAtomForCeiSyntax], linkargs: RecoveredField[LinkargsSyntax] | None) -> LinkedTanruUnitForCeiSyntax:
        return cls._from_fields((base, linkargs))
    def __init__(self, base: RecoveredField[TanruUnitAtomForCeiSyntax], linkargs: RecoveredField[LinkargsSyntax] | None) -> None:
        pass
    @property
    def base(self) -> RecoveredField[TanruUnitAtomForCeiSyntax]:
        'The shared base child syntax node.'
        return cast(RecoveredField[TanruUnitAtomForCeiSyntax], self._field(0))
    @property
    def linkargs(self) -> RecoveredField[LinkargsSyntax] | None:
        'The optional linkargs component.'
        return cast(RecoveredField[LinkargsSyntax] | None, self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('LinkedTanruUnitForCeiSyntax is final')

@final
class TanruUnitAtomForCeiSyntax(_SyntaxNode):
    'Product node for tanru unit; preserves `conversions` and `base` in source order.'
    __slots__ = ()
    _schema_id = 623
    __match_args__ = ('conversions', 'base')
    def __new__(cls, conversions: Sequence[WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]], base: RecoveredField[TanruUnitAtomBaseForCeiSyntax]) -> TanruUnitAtomForCeiSyntax:
        return cls._from_fields((conversions, base))
    def __init__(self, conversions: Sequence[WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]], base: RecoveredField[TanruUnitAtomBaseForCeiSyntax]) -> None:
        pass
    @property
    def conversions(self) -> tuple[WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], ...]:
        'Ordered sequence of zero or more conversions components.'
        return cast(tuple[WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], ...], self._field(0))
    @property
    def base(self) -> RecoveredField[TanruUnitAtomBaseForCeiSyntax]:
        'The shared base child syntax node.'
        return cast(RecoveredField[TanruUnitAtomBaseForCeiSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomForCeiSyntax is final')

@final
class TanruUnitAtomBaseForCeiSyntaxProBridiTanruUnit(_SyntaxNode):
    'Uses the `pro_bridi_tanru_unit` product form, whose payload preserves `goha` and `raho`.'
    __slots__ = ()
    _schema_id = 624
    __match_args__ = ('pro_bridi_tanru_unit',)
    def __new__(cls, pro_bridi_tanru_unit: RecoveredField[ProBridiTanruUnitSyntax]) -> TanruUnitAtomBaseForCeiSyntaxProBridiTanruUnit:
        return cls._from_fields((pro_bridi_tanru_unit,))
    def __init__(self, pro_bridi_tanru_unit: RecoveredField[ProBridiTanruUnitSyntax]) -> None:
        pass
    @property
    def pro_bridi_tanru_unit(self) -> RecoveredField[ProBridiTanruUnitSyntax]:
        'Uses the `pro_bridi_tanru_unit` product form, whose payload preserves `goha` and `raho`.'
        return cast(RecoveredField[ProBridiTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseForCeiSyntaxProBridiTanruUnit is final')

@final
class TanruUnitAtomBaseForCeiSyntaxOrdinalTanruUnit(_SyntaxNode):
    'Uses the `ordinal_tanru_unit` product form, whose payload preserves `number` and `moi`.'
    __slots__ = ()
    _schema_id = 625
    __match_args__ = ('ordinal_tanru_unit',)
    def __new__(cls, ordinal_tanru_unit: RecoveredField[OrdinalTanruUnitSyntax]) -> TanruUnitAtomBaseForCeiSyntaxOrdinalTanruUnit:
        return cls._from_fields((ordinal_tanru_unit,))
    def __init__(self, ordinal_tanru_unit: RecoveredField[OrdinalTanruUnitSyntax]) -> None:
        pass
    @property
    def ordinal_tanru_unit(self) -> RecoveredField[OrdinalTanruUnitSyntax]:
        'Uses the `ordinal_tanru_unit` product form, whose payload preserves `number` and `moi`.'
        return cast(RecoveredField[OrdinalTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseForCeiSyntaxOrdinalTanruUnit is final')

@final
class TanruUnitAtomBaseForCeiSyntaxWordTanruUnit(_SyntaxNode):
    'Uses the `word_tanru_unit` product form, whose payload preserves `word`.'
    __slots__ = ()
    _schema_id = 626
    __match_args__ = ('word_tanru_unit',)
    def __new__(cls, word_tanru_unit: RecoveredField[WordTanruUnitSyntax]) -> TanruUnitAtomBaseForCeiSyntaxWordTanruUnit:
        return cls._from_fields((word_tanru_unit,))
    def __init__(self, word_tanru_unit: RecoveredField[WordTanruUnitSyntax]) -> None:
        pass
    @property
    def word_tanru_unit(self) -> RecoveredField[WordTanruUnitSyntax]:
        'Uses the `word_tanru_unit` product form, whose payload preserves `word`.'
        return cast(RecoveredField[WordTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseForCeiSyntaxWordTanruUnit is final')

@final
class TanruUnitAtomBaseForCeiSyntaxPreposedLinkargsTanruUnit(_SyntaxNode):
    'Uses the `preposed_linkargs_tanru_unit` product form, whose payload preserves `linkargs` and `base`.'
    __slots__ = ()
    _schema_id = 627
    __match_args__ = ('preposed_linkargs_tanru_unit',)
    def __new__(cls, preposed_linkargs_tanru_unit: RecoveredField[PreposedLinkargsTanruUnitSyntax]) -> TanruUnitAtomBaseForCeiSyntaxPreposedLinkargsTanruUnit:
        return cls._from_fields((preposed_linkargs_tanru_unit,))
    def __init__(self, preposed_linkargs_tanru_unit: RecoveredField[PreposedLinkargsTanruUnitSyntax]) -> None:
        pass
    @property
    def preposed_linkargs_tanru_unit(self) -> RecoveredField[PreposedLinkargsTanruUnitSyntax]:
        'Uses the `preposed_linkargs_tanru_unit` product form, whose payload preserves `linkargs` and `base`.'
        return cast(RecoveredField[PreposedLinkargsTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseForCeiSyntaxPreposedLinkargsTanruUnit is final')

@final
class TanruUnitAtomBaseForCeiSyntaxJaiModalTanruUnit(_SyntaxNode):
    'Uses the `jai_modal_tanru_unit` product form, whose payload preserves `jai`, `tense_modal`, and `inner_unit`.'
    __slots__ = ()
    _schema_id = 628
    __match_args__ = ('jai_modal_tanru_unit',)
    def __new__(cls, jai_modal_tanru_unit: RecoveredField[JaiModalTanruUnitSyntax]) -> TanruUnitAtomBaseForCeiSyntaxJaiModalTanruUnit:
        return cls._from_fields((jai_modal_tanru_unit,))
    def __init__(self, jai_modal_tanru_unit: RecoveredField[JaiModalTanruUnitSyntax]) -> None:
        pass
    @property
    def jai_modal_tanru_unit(self) -> RecoveredField[JaiModalTanruUnitSyntax]:
        'Uses the `jai_modal_tanru_unit` product form, whose payload preserves `jai`, `tense_modal`, and `inner_unit`.'
        return cast(RecoveredField[JaiModalTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseForCeiSyntaxJaiModalTanruUnit is final')

@final
class TanruUnitAtomBaseForCeiSyntaxScalarNegatedTanruUnit(_SyntaxNode):
    'Uses the `scalar_negated_tanru_unit` product form, whose payload preserves `nahe` and `inner_unit`.'
    __slots__ = ()
    _schema_id = 629
    __match_args__ = ('scalar_negated_tanru_unit',)
    def __new__(cls, scalar_negated_tanru_unit: RecoveredField[ScalarNegatedTanruUnitSyntax]) -> TanruUnitAtomBaseForCeiSyntaxScalarNegatedTanruUnit:
        return cls._from_fields((scalar_negated_tanru_unit,))
    def __init__(self, scalar_negated_tanru_unit: RecoveredField[ScalarNegatedTanruUnitSyntax]) -> None:
        pass
    @property
    def scalar_negated_tanru_unit(self) -> RecoveredField[ScalarNegatedTanruUnitSyntax]:
        'Uses the `scalar_negated_tanru_unit` product form, whose payload preserves `nahe` and `inner_unit`.'
        return cast(RecoveredField[ScalarNegatedTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseForCeiSyntaxScalarNegatedTanruUnit is final')

@final
class TanruUnitAtomBaseForCeiSyntaxZantufaStatementAbstractionTanruUnit(_SyntaxNode):
    'Uses the `zantufa_statement_abstraction_tanru_unit` product form, whose payload preserves `nu`, `nai`, `abstractor_connections`, `statement`, and `kei`.'
    __slots__ = ()
    _schema_id = 630
    __match_args__ = ('zantufa_statement_abstraction_tanru_unit',)
    def __new__(cls, zantufa_statement_abstraction_tanru_unit: RecoveredField[ZantufaStatementAbstractionTanruUnitSyntax]) -> TanruUnitAtomBaseForCeiSyntaxZantufaStatementAbstractionTanruUnit:
        return cls._from_fields((zantufa_statement_abstraction_tanru_unit,))
    def __init__(self, zantufa_statement_abstraction_tanru_unit: RecoveredField[ZantufaStatementAbstractionTanruUnitSyntax]) -> None:
        pass
    @property
    def zantufa_statement_abstraction_tanru_unit(self) -> RecoveredField[ZantufaStatementAbstractionTanruUnitSyntax]:
        'Uses the `zantufa_statement_abstraction_tanru_unit` product form, whose payload preserves `nu`, `nai`, `abstractor_connections`, `statement`, and `kei`.'
        return cast(RecoveredField[ZantufaStatementAbstractionTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseForCeiSyntaxZantufaStatementAbstractionTanruUnit is final')

@final
class TanruUnitAtomBaseForCeiSyntaxAbstractionTanruUnit(_SyntaxNode):
    'Uses the `abstraction_tanru_unit` product form, whose payload preserves `nu`, `nai`, `abstractor_connections`, `subbridi`, and `kei`.'
    __slots__ = ()
    _schema_id = 631
    __match_args__ = ('abstraction_tanru_unit',)
    def __new__(cls, abstraction_tanru_unit: RecoveredField[AbstractionTanruUnitSyntax]) -> TanruUnitAtomBaseForCeiSyntaxAbstractionTanruUnit:
        return cls._from_fields((abstraction_tanru_unit,))
    def __init__(self, abstraction_tanru_unit: RecoveredField[AbstractionTanruUnitSyntax]) -> None:
        pass
    @property
    def abstraction_tanru_unit(self) -> RecoveredField[AbstractionTanruUnitSyntax]:
        'Uses the `abstraction_tanru_unit` product form, whose payload preserves `nu`, `nai`, `abstractor_connections`, `subbridi`, and `kei`.'
        return cast(RecoveredField[AbstractionTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseForCeiSyntaxAbstractionTanruUnit is final')

@final
class TanruUnitAtomBaseForCeiSyntaxSumtiSelbriTanruUnit(_SyntaxNode):
    'Uses the `sumti_selbri_tanru_unit` product form, whose payload preserves `me`, `sumti`, `mehu`, and `moi_marker`.'
    __slots__ = ()
    _schema_id = 632
    __match_args__ = ('sumti_selbri_tanru_unit',)
    def __new__(cls, sumti_selbri_tanru_unit: RecoveredField[SumtiSelbriTanruUnitSyntax]) -> TanruUnitAtomBaseForCeiSyntaxSumtiSelbriTanruUnit:
        return cls._from_fields((sumti_selbri_tanru_unit,))
    def __init__(self, sumti_selbri_tanru_unit: RecoveredField[SumtiSelbriTanruUnitSyntax]) -> None:
        pass
    @property
    def sumti_selbri_tanru_unit(self) -> RecoveredField[SumtiSelbriTanruUnitSyntax]:
        'Uses the `sumti_selbri_tanru_unit` product form, whose payload preserves `me`, `sumti`, `mehu`, and `moi_marker`.'
        return cast(RecoveredField[SumtiSelbriTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseForCeiSyntaxSumtiSelbriTanruUnit is final')

@final
class TanruUnitAtomBaseForCeiSyntaxZantufaMeTanruUnit(_SyntaxNode):
    'Uses the `zantufa_me_tanru_unit` product form, whose payload preserves `me`, `body`, `mehu`, and `moi_marker`.'
    __slots__ = ()
    _schema_id = 633
    __match_args__ = ('zantufa_me_tanru_unit',)
    def __new__(cls, zantufa_me_tanru_unit: RecoveredField[ZantufaMeTanruUnitSyntax]) -> TanruUnitAtomBaseForCeiSyntaxZantufaMeTanruUnit:
        return cls._from_fields((zantufa_me_tanru_unit,))
    def __init__(self, zantufa_me_tanru_unit: RecoveredField[ZantufaMeTanruUnitSyntax]) -> None:
        pass
    @property
    def zantufa_me_tanru_unit(self) -> RecoveredField[ZantufaMeTanruUnitSyntax]:
        'Uses the `zantufa_me_tanru_unit` product form, whose payload preserves `me`, `body`, `mehu`, and `moi_marker`.'
        return cast(RecoveredField[ZantufaMeTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseForCeiSyntaxZantufaMeTanruUnit is final')

@final
class TanruUnitAtomBaseForCeiSyntaxZantufaMexMoiTanruUnit(_SyntaxNode):
    'Uses the `zantufa_mex_moi_tanru_unit` product form, whose payload preserves `expression` and `moi`.'
    __slots__ = ()
    _schema_id = 634
    __match_args__ = ('zantufa_mex_moi_tanru_unit',)
    def __new__(cls, zantufa_mex_moi_tanru_unit: RecoveredField[ZantufaMexMoiTanruUnitSyntax]) -> TanruUnitAtomBaseForCeiSyntaxZantufaMexMoiTanruUnit:
        return cls._from_fields((zantufa_mex_moi_tanru_unit,))
    def __init__(self, zantufa_mex_moi_tanru_unit: RecoveredField[ZantufaMexMoiTanruUnitSyntax]) -> None:
        pass
    @property
    def zantufa_mex_moi_tanru_unit(self) -> RecoveredField[ZantufaMexMoiTanruUnitSyntax]:
        'Uses the `zantufa_mex_moi_tanru_unit` product form, whose payload preserves `expression` and `moi`.'
        return cast(RecoveredField[ZantufaMexMoiTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseForCeiSyntaxZantufaMexMoiTanruUnit is final')

@final
class TanruUnitAtomBaseForCeiSyntaxOperatorSelbriTanruUnit(_SyntaxNode):
    'Uses the `operator_selbri_tanru_unit` product form, whose payload preserves `nuha` and `mekso_operator`.'
    __slots__ = ()
    _schema_id = 635
    __match_args__ = ('operator_selbri_tanru_unit',)
    def __new__(cls, operator_selbri_tanru_unit: RecoveredField[OperatorSelbriTanruUnitSyntax]) -> TanruUnitAtomBaseForCeiSyntaxOperatorSelbriTanruUnit:
        return cls._from_fields((operator_selbri_tanru_unit,))
    def __init__(self, operator_selbri_tanru_unit: RecoveredField[OperatorSelbriTanruUnitSyntax]) -> None:
        pass
    @property
    def operator_selbri_tanru_unit(self) -> RecoveredField[OperatorSelbriTanruUnitSyntax]:
        'Uses the `operator_selbri_tanru_unit` product form, whose payload preserves `nuha` and `mekso_operator`.'
        return cast(RecoveredField[OperatorSelbriTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseForCeiSyntaxOperatorSelbriTanruUnit is final')

@final
class TanruUnitAtomBaseForCeiSyntaxQuotedBridiSelbriTanruUnit(_SyntaxNode):
    'Uses the `quoted_bridi_selbri_tanru_unit` product form, whose payload preserves `quote`.'
    __slots__ = ()
    _schema_id = 636
    __match_args__ = ('quoted_bridi_selbri_tanru_unit',)
    def __new__(cls, quoted_bridi_selbri_tanru_unit: RecoveredField[QuotedBridiSelbriTanruUnitSyntax]) -> TanruUnitAtomBaseForCeiSyntaxQuotedBridiSelbriTanruUnit:
        return cls._from_fields((quoted_bridi_selbri_tanru_unit,))
    def __init__(self, quoted_bridi_selbri_tanru_unit: RecoveredField[QuotedBridiSelbriTanruUnitSyntax]) -> None:
        pass
    @property
    def quoted_bridi_selbri_tanru_unit(self) -> RecoveredField[QuotedBridiSelbriTanruUnitSyntax]:
        'Uses the `quoted_bridi_selbri_tanru_unit` product form, whose payload preserves `quote`.'
        return cast(RecoveredField[QuotedBridiSelbriTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseForCeiSyntaxQuotedBridiSelbriTanruUnit is final')

@final
class TanruUnitAtomBaseForCeiSyntaxQuotedTextSelbriTanruUnit(_SyntaxNode):
    'Uses the `quoted_text_selbri_tanru_unit` product form, whose payload preserves `muhoi`.'
    __slots__ = ()
    _schema_id = 637
    __match_args__ = ('quoted_text_selbri_tanru_unit',)
    def __new__(cls, quoted_text_selbri_tanru_unit: RecoveredField[QuotedTextSelbriTanruUnitSyntax]) -> TanruUnitAtomBaseForCeiSyntaxQuotedTextSelbriTanruUnit:
        return cls._from_fields((quoted_text_selbri_tanru_unit,))
    def __init__(self, quoted_text_selbri_tanru_unit: RecoveredField[QuotedTextSelbriTanruUnitSyntax]) -> None:
        pass
    @property
    def quoted_text_selbri_tanru_unit(self) -> RecoveredField[QuotedTextSelbriTanruUnitSyntax]:
        'Uses the `quoted_text_selbri_tanru_unit` product form, whose payload preserves `muhoi`.'
        return cast(RecoveredField[QuotedTextSelbriTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseForCeiSyntaxQuotedTextSelbriTanruUnit is final')

@final
class TanruUnitAtomBaseForCeiSyntaxTextSelbriTanruUnit(_SyntaxNode):
    'Uses the `text_selbri_tanru_unit` product form, whose payload preserves `luhei`, `text`, and `lihau`.'
    __slots__ = ()
    _schema_id = 638
    __match_args__ = ('text_selbri_tanru_unit',)
    def __new__(cls, text_selbri_tanru_unit: RecoveredField[TextSelbriTanruUnitSyntax]) -> TanruUnitAtomBaseForCeiSyntaxTextSelbriTanruUnit:
        return cls._from_fields((text_selbri_tanru_unit,))
    def __init__(self, text_selbri_tanru_unit: RecoveredField[TextSelbriTanruUnitSyntax]) -> None:
        pass
    @property
    def text_selbri_tanru_unit(self) -> RecoveredField[TextSelbriTanruUnitSyntax]:
        'Uses the `text_selbri_tanru_unit` product form, whose payload preserves `luhei`, `text`, and `lihau`.'
        return cast(RecoveredField[TextSelbriTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseForCeiSyntaxTextSelbriTanruUnit is final')

@final
class TanruUnitAtomBaseForCeiSyntaxTagSelbriTanruUnit(_SyntaxNode):
    'Uses the `tag_selbri_tanru_unit` product form, whose payload preserves `xohi` and `tag`.'
    __slots__ = ()
    _schema_id = 639
    __match_args__ = ('tag_selbri_tanru_unit',)
    def __new__(cls, tag_selbri_tanru_unit: RecoveredField[TagSelbriTanruUnitSyntax]) -> TanruUnitAtomBaseForCeiSyntaxTagSelbriTanruUnit:
        return cls._from_fields((tag_selbri_tanru_unit,))
    def __init__(self, tag_selbri_tanru_unit: RecoveredField[TagSelbriTanruUnitSyntax]) -> None:
        pass
    @property
    def tag_selbri_tanru_unit(self) -> RecoveredField[TagSelbriTanruUnitSyntax]:
        'Uses the `tag_selbri_tanru_unit` product form, whose payload preserves `xohi` and `tag`.'
        return cast(RecoveredField[TagSelbriTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseForCeiSyntaxTagSelbriTanruUnit is final')

@final
class TanruUnitAtomBaseForCeiSyntaxGohaWordTanruUnit(_SyntaxNode):
    'Uses the `goha_word_tanru_unit` product form, whose payload preserves `word`.'
    __slots__ = ()
    _schema_id = 640
    __match_args__ = ('goha_word_tanru_unit',)
    def __new__(cls, goha_word_tanru_unit: RecoveredField[GohaWordTanruUnitSyntax]) -> TanruUnitAtomBaseForCeiSyntaxGohaWordTanruUnit:
        return cls._from_fields((goha_word_tanru_unit,))
    def __init__(self, goha_word_tanru_unit: RecoveredField[GohaWordTanruUnitSyntax]) -> None:
        pass
    @property
    def goha_word_tanru_unit(self) -> RecoveredField[GohaWordTanruUnitSyntax]:
        'Uses the `goha_word_tanru_unit` product form, whose payload preserves `word`.'
        return cast(RecoveredField[GohaWordTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseForCeiSyntaxGohaWordTanruUnit is final')

@final
class TanruUnitAtomBaseForCeiSyntaxGroupedTanruUnit(_SyntaxNode):
    'Uses the `grouped_tanru_unit` product form, whose payload preserves `ke`, `selbri`, and `kehe`.'
    __slots__ = ()
    _schema_id = 641
    __match_args__ = ('grouped_tanru_unit',)
    def __new__(cls, grouped_tanru_unit: RecoveredField[GroupedTanruUnitSyntax]) -> TanruUnitAtomBaseForCeiSyntaxGroupedTanruUnit:
        return cls._from_fields((grouped_tanru_unit,))
    def __init__(self, grouped_tanru_unit: RecoveredField[GroupedTanruUnitSyntax]) -> None:
        pass
    @property
    def grouped_tanru_unit(self) -> RecoveredField[GroupedTanruUnitSyntax]:
        'Uses the `grouped_tanru_unit` product form, whose payload preserves `ke`, `selbri`, and `kehe`.'
        return cast(RecoveredField[GroupedTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseForCeiSyntaxGroupedTanruUnit is final')

TanruUnitAtomBaseForCeiSyntax: TypeAlias = TanruUnitAtomBaseForCeiSyntaxProBridiTanruUnit | TanruUnitAtomBaseForCeiSyntaxOrdinalTanruUnit | TanruUnitAtomBaseForCeiSyntaxWordTanruUnit | TanruUnitAtomBaseForCeiSyntaxPreposedLinkargsTanruUnit | TanruUnitAtomBaseForCeiSyntaxJaiModalTanruUnit | TanruUnitAtomBaseForCeiSyntaxScalarNegatedTanruUnit | TanruUnitAtomBaseForCeiSyntaxZantufaStatementAbstractionTanruUnit | TanruUnitAtomBaseForCeiSyntaxAbstractionTanruUnit | TanruUnitAtomBaseForCeiSyntaxSumtiSelbriTanruUnit | TanruUnitAtomBaseForCeiSyntaxZantufaMeTanruUnit | TanruUnitAtomBaseForCeiSyntaxZantufaMexMoiTanruUnit | TanruUnitAtomBaseForCeiSyntaxOperatorSelbriTanruUnit | TanruUnitAtomBaseForCeiSyntaxQuotedBridiSelbriTanruUnit | TanruUnitAtomBaseForCeiSyntaxQuotedTextSelbriTanruUnit | TanruUnitAtomBaseForCeiSyntaxTextSelbriTanruUnit | TanruUnitAtomBaseForCeiSyntaxTagSelbriTanruUnit | TanruUnitAtomBaseForCeiSyntaxGohaWordTanruUnit | TanruUnitAtomBaseForCeiSyntaxGroupedTanruUnit

@final
class TanruUnitAtomSyntax(_SyntaxNode):
    'Product node for tanru unit; preserves `conversions` and `base` in source order.'
    __slots__ = ()
    _schema_id = 642
    __match_args__ = ('conversions', 'base')
    def __new__(cls, conversions: Sequence[WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]], base: RecoveredField[TanruUnitAtomBaseSyntax]) -> TanruUnitAtomSyntax:
        return cls._from_fields((conversions, base))
    def __init__(self, conversions: Sequence[WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]], base: RecoveredField[TanruUnitAtomBaseSyntax]) -> None:
        pass
    @property
    def conversions(self) -> tuple[WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], ...]:
        'Ordered sequence of zero or more conversions components.'
        return cast(tuple[WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], ...], self._field(0))
    @property
    def base(self) -> RecoveredField[TanruUnitAtomBaseSyntax]:
        'The shared base child syntax node.'
        return cast(RecoveredField[TanruUnitAtomBaseSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomSyntax is final')

@final
class TanruUnitAtomBaseSyntaxOrdinalTanruUnit(_SyntaxNode):
    'Uses the `ordinal_tanru_unit` product form, whose payload preserves `number` and `moi`.'
    __slots__ = ()
    _schema_id = 643
    __match_args__ = ('ordinal_tanru_unit',)
    def __new__(cls, ordinal_tanru_unit: RecoveredField[OrdinalTanruUnitSyntax]) -> TanruUnitAtomBaseSyntaxOrdinalTanruUnit:
        return cls._from_fields((ordinal_tanru_unit,))
    def __init__(self, ordinal_tanru_unit: RecoveredField[OrdinalTanruUnitSyntax]) -> None:
        pass
    @property
    def ordinal_tanru_unit(self) -> RecoveredField[OrdinalTanruUnitSyntax]:
        'Uses the `ordinal_tanru_unit` product form, whose payload preserves `number` and `moi`.'
        return cast(RecoveredField[OrdinalTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseSyntaxOrdinalTanruUnit is final')

@final
class TanruUnitAtomBaseSyntaxWordTanruUnit(_SyntaxNode):
    'Uses the `word_tanru_unit` product form, whose payload preserves `word`.'
    __slots__ = ()
    _schema_id = 644
    __match_args__ = ('word_tanru_unit',)
    def __new__(cls, word_tanru_unit: RecoveredField[WordTanruUnitSyntax]) -> TanruUnitAtomBaseSyntaxWordTanruUnit:
        return cls._from_fields((word_tanru_unit,))
    def __init__(self, word_tanru_unit: RecoveredField[WordTanruUnitSyntax]) -> None:
        pass
    @property
    def word_tanru_unit(self) -> RecoveredField[WordTanruUnitSyntax]:
        'Uses the `word_tanru_unit` product form, whose payload preserves `word`.'
        return cast(RecoveredField[WordTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseSyntaxWordTanruUnit is final')

@final
class TanruUnitAtomBaseSyntaxPreposedLinkargsTanruUnit(_SyntaxNode):
    'Uses the `preposed_linkargs_tanru_unit` product form, whose payload preserves `linkargs` and `base`.'
    __slots__ = ()
    _schema_id = 645
    __match_args__ = ('preposed_linkargs_tanru_unit',)
    def __new__(cls, preposed_linkargs_tanru_unit: RecoveredField[PreposedLinkargsTanruUnitSyntax]) -> TanruUnitAtomBaseSyntaxPreposedLinkargsTanruUnit:
        return cls._from_fields((preposed_linkargs_tanru_unit,))
    def __init__(self, preposed_linkargs_tanru_unit: RecoveredField[PreposedLinkargsTanruUnitSyntax]) -> None:
        pass
    @property
    def preposed_linkargs_tanru_unit(self) -> RecoveredField[PreposedLinkargsTanruUnitSyntax]:
        'Uses the `preposed_linkargs_tanru_unit` product form, whose payload preserves `linkargs` and `base`.'
        return cast(RecoveredField[PreposedLinkargsTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseSyntaxPreposedLinkargsTanruUnit is final')

@final
class TanruUnitAtomBaseSyntaxJaiModalTanruUnit(_SyntaxNode):
    'Uses the `jai_modal_tanru_unit` product form, whose payload preserves `jai`, `tense_modal`, and `inner_unit`.'
    __slots__ = ()
    _schema_id = 646
    __match_args__ = ('jai_modal_tanru_unit',)
    def __new__(cls, jai_modal_tanru_unit: RecoveredField[JaiModalTanruUnitSyntax]) -> TanruUnitAtomBaseSyntaxJaiModalTanruUnit:
        return cls._from_fields((jai_modal_tanru_unit,))
    def __init__(self, jai_modal_tanru_unit: RecoveredField[JaiModalTanruUnitSyntax]) -> None:
        pass
    @property
    def jai_modal_tanru_unit(self) -> RecoveredField[JaiModalTanruUnitSyntax]:
        'Uses the `jai_modal_tanru_unit` product form, whose payload preserves `jai`, `tense_modal`, and `inner_unit`.'
        return cast(RecoveredField[JaiModalTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseSyntaxJaiModalTanruUnit is final')

@final
class TanruUnitAtomBaseSyntaxScalarNegatedTanruUnit(_SyntaxNode):
    'Uses the `scalar_negated_tanru_unit` product form, whose payload preserves `nahe` and `inner_unit`.'
    __slots__ = ()
    _schema_id = 647
    __match_args__ = ('scalar_negated_tanru_unit',)
    def __new__(cls, scalar_negated_tanru_unit: RecoveredField[ScalarNegatedTanruUnitSyntax]) -> TanruUnitAtomBaseSyntaxScalarNegatedTanruUnit:
        return cls._from_fields((scalar_negated_tanru_unit,))
    def __init__(self, scalar_negated_tanru_unit: RecoveredField[ScalarNegatedTanruUnitSyntax]) -> None:
        pass
    @property
    def scalar_negated_tanru_unit(self) -> RecoveredField[ScalarNegatedTanruUnitSyntax]:
        'Uses the `scalar_negated_tanru_unit` product form, whose payload preserves `nahe` and `inner_unit`.'
        return cast(RecoveredField[ScalarNegatedTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseSyntaxScalarNegatedTanruUnit is final')

@final
class TanruUnitAtomBaseSyntaxZantufaStatementAbstractionTanruUnit(_SyntaxNode):
    'Uses the `zantufa_statement_abstraction_tanru_unit` product form, whose payload preserves `nu`, `nai`, `abstractor_connections`, `statement`, and `kei`.'
    __slots__ = ()
    _schema_id = 648
    __match_args__ = ('zantufa_statement_abstraction_tanru_unit',)
    def __new__(cls, zantufa_statement_abstraction_tanru_unit: RecoveredField[ZantufaStatementAbstractionTanruUnitSyntax]) -> TanruUnitAtomBaseSyntaxZantufaStatementAbstractionTanruUnit:
        return cls._from_fields((zantufa_statement_abstraction_tanru_unit,))
    def __init__(self, zantufa_statement_abstraction_tanru_unit: RecoveredField[ZantufaStatementAbstractionTanruUnitSyntax]) -> None:
        pass
    @property
    def zantufa_statement_abstraction_tanru_unit(self) -> RecoveredField[ZantufaStatementAbstractionTanruUnitSyntax]:
        'Uses the `zantufa_statement_abstraction_tanru_unit` product form, whose payload preserves `nu`, `nai`, `abstractor_connections`, `statement`, and `kei`.'
        return cast(RecoveredField[ZantufaStatementAbstractionTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseSyntaxZantufaStatementAbstractionTanruUnit is final')

@final
class TanruUnitAtomBaseSyntaxAbstractionTanruUnit(_SyntaxNode):
    'Uses the `abstraction_tanru_unit` product form, whose payload preserves `nu`, `nai`, `abstractor_connections`, `subbridi`, and `kei`.'
    __slots__ = ()
    _schema_id = 649
    __match_args__ = ('abstraction_tanru_unit',)
    def __new__(cls, abstraction_tanru_unit: RecoveredField[AbstractionTanruUnitSyntax]) -> TanruUnitAtomBaseSyntaxAbstractionTanruUnit:
        return cls._from_fields((abstraction_tanru_unit,))
    def __init__(self, abstraction_tanru_unit: RecoveredField[AbstractionTanruUnitSyntax]) -> None:
        pass
    @property
    def abstraction_tanru_unit(self) -> RecoveredField[AbstractionTanruUnitSyntax]:
        'Uses the `abstraction_tanru_unit` product form, whose payload preserves `nu`, `nai`, `abstractor_connections`, `subbridi`, and `kei`.'
        return cast(RecoveredField[AbstractionTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseSyntaxAbstractionTanruUnit is final')

@final
class TanruUnitAtomBaseSyntaxSumtiSelbriTanruUnit(_SyntaxNode):
    'Uses the `sumti_selbri_tanru_unit` product form, whose payload preserves `me`, `sumti`, `mehu`, and `moi_marker`.'
    __slots__ = ()
    _schema_id = 650
    __match_args__ = ('sumti_selbri_tanru_unit',)
    def __new__(cls, sumti_selbri_tanru_unit: RecoveredField[SumtiSelbriTanruUnitSyntax]) -> TanruUnitAtomBaseSyntaxSumtiSelbriTanruUnit:
        return cls._from_fields((sumti_selbri_tanru_unit,))
    def __init__(self, sumti_selbri_tanru_unit: RecoveredField[SumtiSelbriTanruUnitSyntax]) -> None:
        pass
    @property
    def sumti_selbri_tanru_unit(self) -> RecoveredField[SumtiSelbriTanruUnitSyntax]:
        'Uses the `sumti_selbri_tanru_unit` product form, whose payload preserves `me`, `sumti`, `mehu`, and `moi_marker`.'
        return cast(RecoveredField[SumtiSelbriTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseSyntaxSumtiSelbriTanruUnit is final')

@final
class TanruUnitAtomBaseSyntaxZantufaMeTanruUnit(_SyntaxNode):
    'Uses the `zantufa_me_tanru_unit` product form, whose payload preserves `me`, `body`, `mehu`, and `moi_marker`.'
    __slots__ = ()
    _schema_id = 651
    __match_args__ = ('zantufa_me_tanru_unit',)
    def __new__(cls, zantufa_me_tanru_unit: RecoveredField[ZantufaMeTanruUnitSyntax]) -> TanruUnitAtomBaseSyntaxZantufaMeTanruUnit:
        return cls._from_fields((zantufa_me_tanru_unit,))
    def __init__(self, zantufa_me_tanru_unit: RecoveredField[ZantufaMeTanruUnitSyntax]) -> None:
        pass
    @property
    def zantufa_me_tanru_unit(self) -> RecoveredField[ZantufaMeTanruUnitSyntax]:
        'Uses the `zantufa_me_tanru_unit` product form, whose payload preserves `me`, `body`, `mehu`, and `moi_marker`.'
        return cast(RecoveredField[ZantufaMeTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseSyntaxZantufaMeTanruUnit is final')

@final
class TanruUnitAtomBaseSyntaxZantufaMexMoiTanruUnit(_SyntaxNode):
    'Uses the `zantufa_mex_moi_tanru_unit` product form, whose payload preserves `expression` and `moi`.'
    __slots__ = ()
    _schema_id = 652
    __match_args__ = ('zantufa_mex_moi_tanru_unit',)
    def __new__(cls, zantufa_mex_moi_tanru_unit: RecoveredField[ZantufaMexMoiTanruUnitSyntax]) -> TanruUnitAtomBaseSyntaxZantufaMexMoiTanruUnit:
        return cls._from_fields((zantufa_mex_moi_tanru_unit,))
    def __init__(self, zantufa_mex_moi_tanru_unit: RecoveredField[ZantufaMexMoiTanruUnitSyntax]) -> None:
        pass
    @property
    def zantufa_mex_moi_tanru_unit(self) -> RecoveredField[ZantufaMexMoiTanruUnitSyntax]:
        'Uses the `zantufa_mex_moi_tanru_unit` product form, whose payload preserves `expression` and `moi`.'
        return cast(RecoveredField[ZantufaMexMoiTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseSyntaxZantufaMexMoiTanruUnit is final')

@final
class TanruUnitAtomBaseSyntaxOperatorSelbriTanruUnit(_SyntaxNode):
    'Uses the `operator_selbri_tanru_unit` product form, whose payload preserves `nuha` and `mekso_operator`.'
    __slots__ = ()
    _schema_id = 653
    __match_args__ = ('operator_selbri_tanru_unit',)
    def __new__(cls, operator_selbri_tanru_unit: RecoveredField[OperatorSelbriTanruUnitSyntax]) -> TanruUnitAtomBaseSyntaxOperatorSelbriTanruUnit:
        return cls._from_fields((operator_selbri_tanru_unit,))
    def __init__(self, operator_selbri_tanru_unit: RecoveredField[OperatorSelbriTanruUnitSyntax]) -> None:
        pass
    @property
    def operator_selbri_tanru_unit(self) -> RecoveredField[OperatorSelbriTanruUnitSyntax]:
        'Uses the `operator_selbri_tanru_unit` product form, whose payload preserves `nuha` and `mekso_operator`.'
        return cast(RecoveredField[OperatorSelbriTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseSyntaxOperatorSelbriTanruUnit is final')

@final
class TanruUnitAtomBaseSyntaxQuotedBridiSelbriTanruUnit(_SyntaxNode):
    'Uses the `quoted_bridi_selbri_tanru_unit` product form, whose payload preserves `quote`.'
    __slots__ = ()
    _schema_id = 654
    __match_args__ = ('quoted_bridi_selbri_tanru_unit',)
    def __new__(cls, quoted_bridi_selbri_tanru_unit: RecoveredField[QuotedBridiSelbriTanruUnitSyntax]) -> TanruUnitAtomBaseSyntaxQuotedBridiSelbriTanruUnit:
        return cls._from_fields((quoted_bridi_selbri_tanru_unit,))
    def __init__(self, quoted_bridi_selbri_tanru_unit: RecoveredField[QuotedBridiSelbriTanruUnitSyntax]) -> None:
        pass
    @property
    def quoted_bridi_selbri_tanru_unit(self) -> RecoveredField[QuotedBridiSelbriTanruUnitSyntax]:
        'Uses the `quoted_bridi_selbri_tanru_unit` product form, whose payload preserves `quote`.'
        return cast(RecoveredField[QuotedBridiSelbriTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseSyntaxQuotedBridiSelbriTanruUnit is final')

@final
class TanruUnitAtomBaseSyntaxQuotedTextSelbriTanruUnit(_SyntaxNode):
    'Uses the `quoted_text_selbri_tanru_unit` product form, whose payload preserves `muhoi`.'
    __slots__ = ()
    _schema_id = 655
    __match_args__ = ('quoted_text_selbri_tanru_unit',)
    def __new__(cls, quoted_text_selbri_tanru_unit: RecoveredField[QuotedTextSelbriTanruUnitSyntax]) -> TanruUnitAtomBaseSyntaxQuotedTextSelbriTanruUnit:
        return cls._from_fields((quoted_text_selbri_tanru_unit,))
    def __init__(self, quoted_text_selbri_tanru_unit: RecoveredField[QuotedTextSelbriTanruUnitSyntax]) -> None:
        pass
    @property
    def quoted_text_selbri_tanru_unit(self) -> RecoveredField[QuotedTextSelbriTanruUnitSyntax]:
        'Uses the `quoted_text_selbri_tanru_unit` product form, whose payload preserves `muhoi`.'
        return cast(RecoveredField[QuotedTextSelbriTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseSyntaxQuotedTextSelbriTanruUnit is final')

@final
class TanruUnitAtomBaseSyntaxTextSelbriTanruUnit(_SyntaxNode):
    'Uses the `text_selbri_tanru_unit` product form, whose payload preserves `luhei`, `text`, and `lihau`.'
    __slots__ = ()
    _schema_id = 656
    __match_args__ = ('text_selbri_tanru_unit',)
    def __new__(cls, text_selbri_tanru_unit: RecoveredField[TextSelbriTanruUnitSyntax]) -> TanruUnitAtomBaseSyntaxTextSelbriTanruUnit:
        return cls._from_fields((text_selbri_tanru_unit,))
    def __init__(self, text_selbri_tanru_unit: RecoveredField[TextSelbriTanruUnitSyntax]) -> None:
        pass
    @property
    def text_selbri_tanru_unit(self) -> RecoveredField[TextSelbriTanruUnitSyntax]:
        'Uses the `text_selbri_tanru_unit` product form, whose payload preserves `luhei`, `text`, and `lihau`.'
        return cast(RecoveredField[TextSelbriTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseSyntaxTextSelbriTanruUnit is final')

@final
class TanruUnitAtomBaseSyntaxTagSelbriTanruUnit(_SyntaxNode):
    'Uses the `tag_selbri_tanru_unit` product form, whose payload preserves `xohi` and `tag`.'
    __slots__ = ()
    _schema_id = 657
    __match_args__ = ('tag_selbri_tanru_unit',)
    def __new__(cls, tag_selbri_tanru_unit: RecoveredField[TagSelbriTanruUnitSyntax]) -> TanruUnitAtomBaseSyntaxTagSelbriTanruUnit:
        return cls._from_fields((tag_selbri_tanru_unit,))
    def __init__(self, tag_selbri_tanru_unit: RecoveredField[TagSelbriTanruUnitSyntax]) -> None:
        pass
    @property
    def tag_selbri_tanru_unit(self) -> RecoveredField[TagSelbriTanruUnitSyntax]:
        'Uses the `tag_selbri_tanru_unit` product form, whose payload preserves `xohi` and `tag`.'
        return cast(RecoveredField[TagSelbriTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseSyntaxTagSelbriTanruUnit is final')

@final
class TanruUnitAtomBaseSyntaxGohaWordTanruUnit(_SyntaxNode):
    'Uses the `goha_word_tanru_unit` product form, whose payload preserves `word`.'
    __slots__ = ()
    _schema_id = 658
    __match_args__ = ('goha_word_tanru_unit',)
    def __new__(cls, goha_word_tanru_unit: RecoveredField[GohaWordTanruUnitSyntax]) -> TanruUnitAtomBaseSyntaxGohaWordTanruUnit:
        return cls._from_fields((goha_word_tanru_unit,))
    def __init__(self, goha_word_tanru_unit: RecoveredField[GohaWordTanruUnitSyntax]) -> None:
        pass
    @property
    def goha_word_tanru_unit(self) -> RecoveredField[GohaWordTanruUnitSyntax]:
        'Uses the `goha_word_tanru_unit` product form, whose payload preserves `word`.'
        return cast(RecoveredField[GohaWordTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseSyntaxGohaWordTanruUnit is final')

@final
class TanruUnitAtomBaseSyntaxProBridiTanruUnit(_SyntaxNode):
    'Uses the `pro_bridi_tanru_unit` product form, whose payload preserves `goha` and `raho`.'
    __slots__ = ()
    _schema_id = 659
    __match_args__ = ('pro_bridi_tanru_unit',)
    def __new__(cls, pro_bridi_tanru_unit: RecoveredField[ProBridiTanruUnitSyntax]) -> TanruUnitAtomBaseSyntaxProBridiTanruUnit:
        return cls._from_fields((pro_bridi_tanru_unit,))
    def __init__(self, pro_bridi_tanru_unit: RecoveredField[ProBridiTanruUnitSyntax]) -> None:
        pass
    @property
    def pro_bridi_tanru_unit(self) -> RecoveredField[ProBridiTanruUnitSyntax]:
        'Uses the `pro_bridi_tanru_unit` product form, whose payload preserves `goha` and `raho`.'
        return cast(RecoveredField[ProBridiTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseSyntaxProBridiTanruUnit is final')

@final
class TanruUnitAtomBaseSyntaxGroupedTanruUnit(_SyntaxNode):
    'Uses the `grouped_tanru_unit` product form, whose payload preserves `ke`, `selbri`, and `kehe`.'
    __slots__ = ()
    _schema_id = 660
    __match_args__ = ('grouped_tanru_unit',)
    def __new__(cls, grouped_tanru_unit: RecoveredField[GroupedTanruUnitSyntax]) -> TanruUnitAtomBaseSyntaxGroupedTanruUnit:
        return cls._from_fields((grouped_tanru_unit,))
    def __init__(self, grouped_tanru_unit: RecoveredField[GroupedTanruUnitSyntax]) -> None:
        pass
    @property
    def grouped_tanru_unit(self) -> RecoveredField[GroupedTanruUnitSyntax]:
        'Uses the `grouped_tanru_unit` product form, whose payload preserves `ke`, `selbri`, and `kehe`.'
        return cast(RecoveredField[GroupedTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruUnitAtomBaseSyntaxGroupedTanruUnit is final')

TanruUnitAtomBaseSyntax: TypeAlias = TanruUnitAtomBaseSyntaxOrdinalTanruUnit | TanruUnitAtomBaseSyntaxWordTanruUnit | TanruUnitAtomBaseSyntaxPreposedLinkargsTanruUnit | TanruUnitAtomBaseSyntaxJaiModalTanruUnit | TanruUnitAtomBaseSyntaxScalarNegatedTanruUnit | TanruUnitAtomBaseSyntaxZantufaStatementAbstractionTanruUnit | TanruUnitAtomBaseSyntaxAbstractionTanruUnit | TanruUnitAtomBaseSyntaxSumtiSelbriTanruUnit | TanruUnitAtomBaseSyntaxZantufaMeTanruUnit | TanruUnitAtomBaseSyntaxZantufaMexMoiTanruUnit | TanruUnitAtomBaseSyntaxOperatorSelbriTanruUnit | TanruUnitAtomBaseSyntaxQuotedBridiSelbriTanruUnit | TanruUnitAtomBaseSyntaxQuotedTextSelbriTanruUnit | TanruUnitAtomBaseSyntaxTextSelbriTanruUnit | TanruUnitAtomBaseSyntaxTagSelbriTanruUnit | TanruUnitAtomBaseSyntaxGohaWordTanruUnit | TanruUnitAtomBaseSyntaxProBridiTanruUnit | TanruUnitAtomBaseSyntaxGroupedTanruUnit

@final
class TaggedSelbriGroupTanruUnitSyntax(_SyntaxNode):
    'Product node for tagged selbri; preserves `tense_modal` and `inner_selbri` in source order.'
    __slots__ = ()
    _schema_id = 661
    __match_args__ = ('tense_modal', 'inner_selbri')
    def __new__(cls, tense_modal: RecoveredField[TenseModalSyntax], inner_selbri: RecoveredField[ConnectedSelbriSyntax]) -> TaggedSelbriGroupTanruUnitSyntax:
        return cls._from_fields((tense_modal, inner_selbri))
    def __init__(self, tense_modal: RecoveredField[TenseModalSyntax], inner_selbri: RecoveredField[ConnectedSelbriSyntax]) -> None:
        pass
    @property
    def tense_modal(self) -> RecoveredField[TenseModalSyntax]:
        'The shared tense modal child syntax node.'
        return cast(RecoveredField[TenseModalSyntax], self._field(0))
    @property
    def inner_selbri(self) -> RecoveredField[ConnectedSelbriSyntax]:
        'The shared inner selbri child syntax node.'
        return cast(RecoveredField[ConnectedSelbriSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('TaggedSelbriGroupTanruUnitSyntax is final')

@final
class PreposedLinkargsTanruUnitSyntax(_SyntaxNode):
    'Product node for linked arguments; preserves `linkargs` and `base` in source order.'
    __slots__ = ()
    _schema_id = 662
    __match_args__ = ('linkargs', 'base')
    def __new__(cls, linkargs: RecoveredField[LinkargsSyntax], base: RecoveredField[TanruUnitSyntax]) -> PreposedLinkargsTanruUnitSyntax:
        return cls._from_fields((linkargs, base))
    def __init__(self, linkargs: RecoveredField[LinkargsSyntax], base: RecoveredField[TanruUnitSyntax]) -> None:
        pass
    @property
    def linkargs(self) -> RecoveredField[LinkargsSyntax]:
        'The `linkargs` grammar result in the `linkargs` structural role of the `preposed_linkargs_tanru_unit` production.'
        return cast(RecoveredField[LinkargsSyntax], self._field(0))
    @property
    def base(self) -> RecoveredField[TanruUnitSyntax]:
        'The shared base child syntax node.'
        return cast(RecoveredField[TanruUnitSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('PreposedLinkargsTanruUnitSyntax is final')

@final
class ScalarNegatedTanruUnitSyntax(_SyntaxNode):
    'Product node for scalar-negated tanru unit; preserves `nahe` and `inner_unit` in source order.'
    __slots__ = ()
    _schema_id = 663
    __match_args__ = ('nahe', 'inner_unit')
    def __new__(cls, nahe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_unit: RecoveredField[ScalarNegatedTanruInnerUnitSyntax]) -> ScalarNegatedTanruUnitSyntax:
        return cls._from_fields((nahe, inner_unit))
    def __init__(self, nahe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_unit: RecoveredField[ScalarNegatedTanruInnerUnitSyntax]) -> None:
        pass
    @property
    def nahe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Nahe`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def inner_unit(self) -> RecoveredField[ScalarNegatedTanruInnerUnitSyntax]:
        'The shared inner unit child syntax node.'
        return cast(RecoveredField[ScalarNegatedTanruInnerUnitSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ScalarNegatedTanruUnitSyntax is final')

@final
class ScalarNegatedTanruInnerUnitSyntaxTaggedSelbriGroupTanruUnit(_SyntaxNode):
    'Uses the `tagged_selbri_group_tanru_unit` product form, whose payload preserves `tense_modal` and `inner_selbri`.'
    __slots__ = ()
    _schema_id = 664
    __match_args__ = ('tagged_selbri_group_tanru_unit',)
    def __new__(cls, tagged_selbri_group_tanru_unit: RecoveredField[TaggedSelbriGroupTanruUnitSyntax]) -> ScalarNegatedTanruInnerUnitSyntaxTaggedSelbriGroupTanruUnit:
        return cls._from_fields((tagged_selbri_group_tanru_unit,))
    def __init__(self, tagged_selbri_group_tanru_unit: RecoveredField[TaggedSelbriGroupTanruUnitSyntax]) -> None:
        pass
    @property
    def tagged_selbri_group_tanru_unit(self) -> RecoveredField[TaggedSelbriGroupTanruUnitSyntax]:
        'Uses the `tagged_selbri_group_tanru_unit` product form, whose payload preserves `tense_modal` and `inner_selbri`.'
        return cast(RecoveredField[TaggedSelbriGroupTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ScalarNegatedTanruInnerUnitSyntaxTaggedSelbriGroupTanruUnit is final')

@final
class ScalarNegatedTanruInnerUnitSyntaxProBridiTanruUnit(_SyntaxNode):
    'Uses the `pro_bridi_tanru_unit` product form, whose payload preserves `goha` and `raho`.'
    __slots__ = ()
    _schema_id = 665
    __match_args__ = ('pro_bridi_tanru_unit',)
    def __new__(cls, pro_bridi_tanru_unit: RecoveredField[ProBridiTanruUnitSyntax]) -> ScalarNegatedTanruInnerUnitSyntaxProBridiTanruUnit:
        return cls._from_fields((pro_bridi_tanru_unit,))
    def __init__(self, pro_bridi_tanru_unit: RecoveredField[ProBridiTanruUnitSyntax]) -> None:
        pass
    @property
    def pro_bridi_tanru_unit(self) -> RecoveredField[ProBridiTanruUnitSyntax]:
        'Uses the `pro_bridi_tanru_unit` product form, whose payload preserves `goha` and `raho`.'
        return cast(RecoveredField[ProBridiTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ScalarNegatedTanruInnerUnitSyntaxProBridiTanruUnit is final')

@final
class ScalarNegatedTanruInnerUnitSyntaxTanruUnitAtom(_SyntaxNode):
    'Uses the `tanru_unit_atom` product form, whose payload preserves `conversions` and `base`.'
    __slots__ = ()
    _schema_id = 666
    __match_args__ = ('tanru_unit_atom',)
    def __new__(cls, tanru_unit_atom: RecoveredField[TanruUnitAtomSyntax]) -> ScalarNegatedTanruInnerUnitSyntaxTanruUnitAtom:
        return cls._from_fields((tanru_unit_atom,))
    def __init__(self, tanru_unit_atom: RecoveredField[TanruUnitAtomSyntax]) -> None:
        pass
    @property
    def tanru_unit_atom(self) -> RecoveredField[TanruUnitAtomSyntax]:
        'Uses the `tanru_unit_atom` product form, whose payload preserves `conversions` and `base`.'
        return cast(RecoveredField[TanruUnitAtomSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ScalarNegatedTanruInnerUnitSyntaxTanruUnitAtom is final')

ScalarNegatedTanruInnerUnitSyntax: TypeAlias = ScalarNegatedTanruInnerUnitSyntaxTaggedSelbriGroupTanruUnit | ScalarNegatedTanruInnerUnitSyntaxProBridiTanruUnit | ScalarNegatedTanruInnerUnitSyntaxTanruUnitAtom

@final
class JaiModalTanruUnitSyntax(_SyntaxNode):
    'Product node for modal conversion; preserves `jai`, `tense_modal`, and `inner_unit` in source order.'
    __slots__ = ()
    _schema_id = 667
    __match_args__ = ('jai', 'tense_modal', 'inner_unit')
    def __new__(cls, jai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], tense_modal: RecoveredField[TenseModalSyntax] | None, inner_unit: RecoveredField[JaiInnerTanruUnitSyntax]) -> JaiModalTanruUnitSyntax:
        return cls._from_fields((jai, tense_modal, inner_unit))
    def __init__(self, jai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], tense_modal: RecoveredField[TenseModalSyntax] | None, inner_unit: RecoveredField[JaiInnerTanruUnitSyntax]) -> None:
        pass
    @property
    def jai(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Jai` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def tense_modal(self) -> RecoveredField[TenseModalSyntax] | None:
        'The optional tense modal component.'
        return cast(RecoveredField[TenseModalSyntax] | None, self._field(1))
    @property
    def inner_unit(self) -> RecoveredField[JaiInnerTanruUnitSyntax]:
        'The shared inner unit child syntax node.'
        return cast(RecoveredField[JaiInnerTanruUnitSyntax], self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('JaiModalTanruUnitSyntax is final')

@final
class JaiInnerTanruUnitSyntaxConvertedJaiInnerTanruUnit(_SyntaxNode):
    'Uses the `converted_jai_inner_tanru_unit` product form, whose payload preserves `se` and `inner_unit`.'
    __slots__ = ()
    _schema_id = 668
    __match_args__ = ('converted_jai_inner_tanru_unit',)
    def __new__(cls, converted_jai_inner_tanru_unit: RecoveredField[ConvertedJaiInnerTanruUnitSyntax]) -> JaiInnerTanruUnitSyntaxConvertedJaiInnerTanruUnit:
        return cls._from_fields((converted_jai_inner_tanru_unit,))
    def __init__(self, converted_jai_inner_tanru_unit: RecoveredField[ConvertedJaiInnerTanruUnitSyntax]) -> None:
        pass
    @property
    def converted_jai_inner_tanru_unit(self) -> RecoveredField[ConvertedJaiInnerTanruUnitSyntax]:
        'Uses the `converted_jai_inner_tanru_unit` product form, whose payload preserves `se` and `inner_unit`.'
        return cast(RecoveredField[ConvertedJaiInnerTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('JaiInnerTanruUnitSyntaxConvertedJaiInnerTanruUnit is final')

@final
class JaiInnerTanruUnitSyntaxScalarNegatedJaiInnerTanruUnit(_SyntaxNode):
    'Uses the `scalar_negated_jai_inner_tanru_unit` product form, whose payload preserves `nahe` and `inner_unit`.'
    __slots__ = ()
    _schema_id = 669
    __match_args__ = ('scalar_negated_jai_inner_tanru_unit',)
    def __new__(cls, scalar_negated_jai_inner_tanru_unit: RecoveredField[ScalarNegatedJaiInnerTanruUnitSyntax]) -> JaiInnerTanruUnitSyntaxScalarNegatedJaiInnerTanruUnit:
        return cls._from_fields((scalar_negated_jai_inner_tanru_unit,))
    def __init__(self, scalar_negated_jai_inner_tanru_unit: RecoveredField[ScalarNegatedJaiInnerTanruUnitSyntax]) -> None:
        pass
    @property
    def scalar_negated_jai_inner_tanru_unit(self) -> RecoveredField[ScalarNegatedJaiInnerTanruUnitSyntax]:
        'Uses the `scalar_negated_jai_inner_tanru_unit` product form, whose payload preserves `nahe` and `inner_unit`.'
        return cast(RecoveredField[ScalarNegatedJaiInnerTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('JaiInnerTanruUnitSyntaxScalarNegatedJaiInnerTanruUnit is final')

@final
class JaiInnerTanruUnitSyntaxSumtiSelbriTanruUnit(_SyntaxNode):
    'Uses the `sumti_selbri_tanru_unit` product form, whose payload preserves `me`, `sumti`, `mehu`, and `moi_marker`.'
    __slots__ = ()
    _schema_id = 670
    __match_args__ = ('sumti_selbri_tanru_unit',)
    def __new__(cls, sumti_selbri_tanru_unit: RecoveredField[SumtiSelbriTanruUnitSyntax]) -> JaiInnerTanruUnitSyntaxSumtiSelbriTanruUnit:
        return cls._from_fields((sumti_selbri_tanru_unit,))
    def __init__(self, sumti_selbri_tanru_unit: RecoveredField[SumtiSelbriTanruUnitSyntax]) -> None:
        pass
    @property
    def sumti_selbri_tanru_unit(self) -> RecoveredField[SumtiSelbriTanruUnitSyntax]:
        'Uses the `sumti_selbri_tanru_unit` product form, whose payload preserves `me`, `sumti`, `mehu`, and `moi_marker`.'
        return cast(RecoveredField[SumtiSelbriTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('JaiInnerTanruUnitSyntaxSumtiSelbriTanruUnit is final')

@final
class JaiInnerTanruUnitSyntaxQuotedBridiSelbriTanruUnit(_SyntaxNode):
    'Uses the `quoted_bridi_selbri_tanru_unit` product form, whose payload preserves `quote`.'
    __slots__ = ()
    _schema_id = 671
    __match_args__ = ('quoted_bridi_selbri_tanru_unit',)
    def __new__(cls, quoted_bridi_selbri_tanru_unit: RecoveredField[QuotedBridiSelbriTanruUnitSyntax]) -> JaiInnerTanruUnitSyntaxQuotedBridiSelbriTanruUnit:
        return cls._from_fields((quoted_bridi_selbri_tanru_unit,))
    def __init__(self, quoted_bridi_selbri_tanru_unit: RecoveredField[QuotedBridiSelbriTanruUnitSyntax]) -> None:
        pass
    @property
    def quoted_bridi_selbri_tanru_unit(self) -> RecoveredField[QuotedBridiSelbriTanruUnitSyntax]:
        'Uses the `quoted_bridi_selbri_tanru_unit` product form, whose payload preserves `quote`.'
        return cast(RecoveredField[QuotedBridiSelbriTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('JaiInnerTanruUnitSyntaxQuotedBridiSelbriTanruUnit is final')

@final
class JaiInnerTanruUnitSyntaxQuotedTextSelbriTanruUnit(_SyntaxNode):
    'Uses the `quoted_text_selbri_tanru_unit` product form, whose payload preserves `muhoi`.'
    __slots__ = ()
    _schema_id = 672
    __match_args__ = ('quoted_text_selbri_tanru_unit',)
    def __new__(cls, quoted_text_selbri_tanru_unit: RecoveredField[QuotedTextSelbriTanruUnitSyntax]) -> JaiInnerTanruUnitSyntaxQuotedTextSelbriTanruUnit:
        return cls._from_fields((quoted_text_selbri_tanru_unit,))
    def __init__(self, quoted_text_selbri_tanru_unit: RecoveredField[QuotedTextSelbriTanruUnitSyntax]) -> None:
        pass
    @property
    def quoted_text_selbri_tanru_unit(self) -> RecoveredField[QuotedTextSelbriTanruUnitSyntax]:
        'Uses the `quoted_text_selbri_tanru_unit` product form, whose payload preserves `muhoi`.'
        return cast(RecoveredField[QuotedTextSelbriTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('JaiInnerTanruUnitSyntaxQuotedTextSelbriTanruUnit is final')

@final
class JaiInnerTanruUnitSyntaxTextSelbriTanruUnit(_SyntaxNode):
    'Uses the `text_selbri_tanru_unit` product form, whose payload preserves `luhei`, `text`, and `lihau`.'
    __slots__ = ()
    _schema_id = 673
    __match_args__ = ('text_selbri_tanru_unit',)
    def __new__(cls, text_selbri_tanru_unit: RecoveredField[TextSelbriTanruUnitSyntax]) -> JaiInnerTanruUnitSyntaxTextSelbriTanruUnit:
        return cls._from_fields((text_selbri_tanru_unit,))
    def __init__(self, text_selbri_tanru_unit: RecoveredField[TextSelbriTanruUnitSyntax]) -> None:
        pass
    @property
    def text_selbri_tanru_unit(self) -> RecoveredField[TextSelbriTanruUnitSyntax]:
        'Uses the `text_selbri_tanru_unit` product form, whose payload preserves `luhei`, `text`, and `lihau`.'
        return cast(RecoveredField[TextSelbriTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('JaiInnerTanruUnitSyntaxTextSelbriTanruUnit is final')

@final
class JaiInnerTanruUnitSyntaxGroupedJaiInnerTanruUnit(_SyntaxNode):
    'Uses the `grouped_jai_inner_tanru_unit` product form, whose payload preserves `ke`, `selbri`, and `kehe`.'
    __slots__ = ()
    _schema_id = 674
    __match_args__ = ('grouped_jai_inner_tanru_unit',)
    def __new__(cls, grouped_jai_inner_tanru_unit: RecoveredField[GroupedJaiInnerTanruUnitSyntax]) -> JaiInnerTanruUnitSyntaxGroupedJaiInnerTanruUnit:
        return cls._from_fields((grouped_jai_inner_tanru_unit,))
    def __init__(self, grouped_jai_inner_tanru_unit: RecoveredField[GroupedJaiInnerTanruUnitSyntax]) -> None:
        pass
    @property
    def grouped_jai_inner_tanru_unit(self) -> RecoveredField[GroupedJaiInnerTanruUnitSyntax]:
        'Uses the `grouped_jai_inner_tanru_unit` product form, whose payload preserves `ke`, `selbri`, and `kehe`.'
        return cast(RecoveredField[GroupedJaiInnerTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('JaiInnerTanruUnitSyntaxGroupedJaiInnerTanruUnit is final')

@final
class JaiInnerTanruUnitSyntaxOrdinalTanruUnit(_SyntaxNode):
    'Uses the `ordinal_tanru_unit` product form, whose payload preserves `number` and `moi`.'
    __slots__ = ()
    _schema_id = 675
    __match_args__ = ('ordinal_tanru_unit',)
    def __new__(cls, ordinal_tanru_unit: RecoveredField[OrdinalTanruUnitSyntax]) -> JaiInnerTanruUnitSyntaxOrdinalTanruUnit:
        return cls._from_fields((ordinal_tanru_unit,))
    def __init__(self, ordinal_tanru_unit: RecoveredField[OrdinalTanruUnitSyntax]) -> None:
        pass
    @property
    def ordinal_tanru_unit(self) -> RecoveredField[OrdinalTanruUnitSyntax]:
        'Uses the `ordinal_tanru_unit` product form, whose payload preserves `number` and `moi`.'
        return cast(RecoveredField[OrdinalTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('JaiInnerTanruUnitSyntaxOrdinalTanruUnit is final')

@final
class JaiInnerTanruUnitSyntaxOperatorSelbriTanruUnit(_SyntaxNode):
    'Uses the `operator_selbri_tanru_unit` product form, whose payload preserves `nuha` and `mekso_operator`.'
    __slots__ = ()
    _schema_id = 676
    __match_args__ = ('operator_selbri_tanru_unit',)
    def __new__(cls, operator_selbri_tanru_unit: RecoveredField[OperatorSelbriTanruUnitSyntax]) -> JaiInnerTanruUnitSyntaxOperatorSelbriTanruUnit:
        return cls._from_fields((operator_selbri_tanru_unit,))
    def __init__(self, operator_selbri_tanru_unit: RecoveredField[OperatorSelbriTanruUnitSyntax]) -> None:
        pass
    @property
    def operator_selbri_tanru_unit(self) -> RecoveredField[OperatorSelbriTanruUnitSyntax]:
        'Uses the `operator_selbri_tanru_unit` product form, whose payload preserves `nuha` and `mekso_operator`.'
        return cast(RecoveredField[OperatorSelbriTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('JaiInnerTanruUnitSyntaxOperatorSelbriTanruUnit is final')

@final
class JaiInnerTanruUnitSyntaxProBridiTanruUnit(_SyntaxNode):
    'Uses the `pro_bridi_tanru_unit` product form, whose payload preserves `goha` and `raho`.'
    __slots__ = ()
    _schema_id = 677
    __match_args__ = ('pro_bridi_tanru_unit',)
    def __new__(cls, pro_bridi_tanru_unit: RecoveredField[ProBridiTanruUnitSyntax]) -> JaiInnerTanruUnitSyntaxProBridiTanruUnit:
        return cls._from_fields((pro_bridi_tanru_unit,))
    def __init__(self, pro_bridi_tanru_unit: RecoveredField[ProBridiTanruUnitSyntax]) -> None:
        pass
    @property
    def pro_bridi_tanru_unit(self) -> RecoveredField[ProBridiTanruUnitSyntax]:
        'Uses the `pro_bridi_tanru_unit` product form, whose payload preserves `goha` and `raho`.'
        return cast(RecoveredField[ProBridiTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('JaiInnerTanruUnitSyntaxProBridiTanruUnit is final')

@final
class JaiInnerTanruUnitSyntaxWordTanruUnit(_SyntaxNode):
    'Uses the `word_tanru_unit` product form, whose payload preserves `word`.'
    __slots__ = ()
    _schema_id = 678
    __match_args__ = ('word_tanru_unit',)
    def __new__(cls, word_tanru_unit: RecoveredField[WordTanruUnitSyntax]) -> JaiInnerTanruUnitSyntaxWordTanruUnit:
        return cls._from_fields((word_tanru_unit,))
    def __init__(self, word_tanru_unit: RecoveredField[WordTanruUnitSyntax]) -> None:
        pass
    @property
    def word_tanru_unit(self) -> RecoveredField[WordTanruUnitSyntax]:
        'Uses the `word_tanru_unit` product form, whose payload preserves `word`.'
        return cast(RecoveredField[WordTanruUnitSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('JaiInnerTanruUnitSyntaxWordTanruUnit is final')

JaiInnerTanruUnitSyntax: TypeAlias = JaiInnerTanruUnitSyntaxConvertedJaiInnerTanruUnit | JaiInnerTanruUnitSyntaxScalarNegatedJaiInnerTanruUnit | JaiInnerTanruUnitSyntaxSumtiSelbriTanruUnit | JaiInnerTanruUnitSyntaxQuotedBridiSelbriTanruUnit | JaiInnerTanruUnitSyntaxQuotedTextSelbriTanruUnit | JaiInnerTanruUnitSyntaxTextSelbriTanruUnit | JaiInnerTanruUnitSyntaxGroupedJaiInnerTanruUnit | JaiInnerTanruUnitSyntaxOrdinalTanruUnit | JaiInnerTanruUnitSyntaxOperatorSelbriTanruUnit | JaiInnerTanruUnitSyntaxProBridiTanruUnit | JaiInnerTanruUnitSyntaxWordTanruUnit

@final
class ConvertedJaiInnerTanruUnitSyntax(_SyntaxNode):
    'Product node for converted tanru unit; preserves `se` and `inner_unit` in source order.'
    __slots__ = ()
    _schema_id = 679
    __match_args__ = ('se', 'inner_unit')
    def __new__(cls, se: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_unit: RecoveredField[JaiInnerTanruUnitSyntax]) -> ConvertedJaiInnerTanruUnitSyntax:
        return cls._from_fields((se, inner_unit))
    def __init__(self, se: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_unit: RecoveredField[JaiInnerTanruUnitSyntax]) -> None:
        pass
    @property
    def se(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Se`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def inner_unit(self) -> RecoveredField[JaiInnerTanruUnitSyntax]:
        'The shared inner unit child syntax node.'
        return cast(RecoveredField[JaiInnerTanruUnitSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ConvertedJaiInnerTanruUnitSyntax is final')

@final
class ScalarNegatedJaiInnerTanruUnitSyntax(_SyntaxNode):
    'Product node for scalar-negated tanru unit; preserves `nahe` and `inner_unit` in source order.'
    __slots__ = ()
    _schema_id = 680
    __match_args__ = ('nahe', 'inner_unit')
    def __new__(cls, nahe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_unit: RecoveredField[JaiInnerTanruUnitSyntax]) -> ScalarNegatedJaiInnerTanruUnitSyntax:
        return cls._from_fields((nahe, inner_unit))
    def __init__(self, nahe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], inner_unit: RecoveredField[JaiInnerTanruUnitSyntax]) -> None:
        pass
    @property
    def nahe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Nahe`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def inner_unit(self) -> RecoveredField[JaiInnerTanruUnitSyntax]:
        'The shared inner unit child syntax node.'
        return cast(RecoveredField[JaiInnerTanruUnitSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ScalarNegatedJaiInnerTanruUnitSyntax is final')

@final
class QuotedBridiSelbriTanruUnitSyntax(_SyntaxNode):
    'Transparent product node for quoted bridi selbri; preserves the `quote` component.'
    __slots__ = ()
    _schema_id = 681
    __match_args__ = ('quote',)
    def __new__(cls, quote: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> QuotedBridiSelbriTanruUnitSyntax:
        return cls._from_fields((quote,))
    def __init__(self, quote: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def quote(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The selected grammar alternative in the `quote` structural role of the `quoted_bridi_selbri_tanru_unit` production.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('QuotedBridiSelbriTanruUnitSyntax is final')

@final
class TextSelbriTanruUnitSyntax(_SyntaxNode):
    'Product node for text selbri; preserves `luhei`, `text`, and `lihau` in source order.'
    __slots__ = ()
    _schema_id = 682
    __match_args__ = ('luhei', 'text', 'lihau')
    def __new__(cls, luhei: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], text: RecoveredField[TextSyntax], lihau: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> TextSelbriTanruUnitSyntax:
        return cls._from_fields((luhei, text, lihau))
    def __init__(self, luhei: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], text: RecoveredField[TextSyntax], lihau: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def luhei(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Luhei` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def text(self) -> RecoveredField[TextSyntax]:
        'The shared text child syntax node.'
        return cast(RecoveredField[TextSyntax], self._field(1))
    @property
    def lihau(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Lihau` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('TextSelbriTanruUnitSyntax is final')

@final
class QuotedTextSelbriTanruUnitSyntax(_SyntaxNode):
    'Transparent product node for quoted text selbri; preserves the `muhoi` component.'
    __slots__ = ()
    _schema_id = 683
    __match_args__ = ('muhoi',)
    def __new__(cls, muhoi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> QuotedTextSelbriTanruUnitSyntax:
        return cls._from_fields((muhoi,))
    def __init__(self, muhoi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def muhoi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `delimited_quote_marker` grammar result in the `muhoi` structural role of the `quoted_text_selbri_tanru_unit` production.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('QuotedTextSelbriTanruUnitSyntax is final')

@final
class TagSelbriTanruUnitSyntax(_SyntaxNode):
    'Product node for tag selbri; preserves `xohi` and `tag` in source order.'
    __slots__ = ()
    _schema_id = 684
    __match_args__ = ('xohi', 'tag')
    def __new__(cls, xohi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], tag: RecoveredField[TenseModalSyntax]) -> TagSelbriTanruUnitSyntax:
        return cls._from_fields((xohi, tag))
    def __init__(self, xohi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], tag: RecoveredField[TenseModalSyntax]) -> None:
        pass
    @property
    def xohi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Xohi` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def tag(self) -> RecoveredField[TenseModalSyntax]:
        'The shared tag child syntax node.'
        return cast(RecoveredField[TenseModalSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('TagSelbriTanruUnitSyntax is final')

@final
class OrdinalTanruUnitSyntax(_SyntaxNode):
    'Product node for ordinal selbri; preserves `number` and `moi` in source order.'
    __slots__ = ()
    _schema_id = 685
    __match_args__ = ('number', 'moi')
    def __new__(cls, number: RecoveredField[NumberOrLetterWordsSyntax], moi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> OrdinalTanruUnitSyntax:
        return cls._from_fields((number, moi))
    def __init__(self, number: RecoveredField[NumberOrLetterWordsSyntax], moi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def number(self) -> RecoveredField[NumberOrLetterWordsSyntax]:
        'The `number_or_letter_words` grammar result in the `number` structural role of the `ordinal_tanru_unit` production.'
        return cast(RecoveredField[NumberOrLetterWordsSyntax], self._field(0))
    @property
    def moi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Moi`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('OrdinalTanruUnitSyntax is final')

@final
class WordTanruUnitSyntax(_SyntaxNode):
    'Transparent product node for tanru unit; preserves the `word` component.'
    __slots__ = ()
    _schema_id = 686
    __match_args__ = ('word',)
    def __new__(cls, word: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> WordTanruUnitSyntax:
        return cls._from_fields((word,))
    def __init__(self, word: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def word(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `tanru_unit_relation_word` grammar result in the `word` structural role of the `word_tanru_unit` production.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('WordTanruUnitSyntax is final')

@final
class GohaWordTanruUnitSyntax(_SyntaxNode):
    'Transparent product node for tanru unit; preserves the `word` component.'
    __slots__ = ()
    _schema_id = 687
    __match_args__ = ('word',)
    def __new__(cls, word: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> GohaWordTanruUnitSyntax:
        return cls._from_fields((word,))
    def __init__(self, word: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def word(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Goha`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('GohaWordTanruUnitSyntax is final')

@final
class ProBridiTanruUnitSyntax(_SyntaxNode):
    'Product node for pro-bridi; preserves `goha` and `raho` in source order.'
    __slots__ = ()
    _schema_id = 688
    __match_args__ = ('goha', 'raho')
    def __new__(cls, goha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], raho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> ProBridiTanruUnitSyntax:
        return cls._from_fields((goha, raho))
    def __init__(self, goha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], raho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def goha(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Goha`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def raho(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Raho` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ProBridiTanruUnitSyntax is final')

@final
class SumtiSelbriTanruUnitSyntax(_SyntaxNode):
    'Product node for sumti-to-selbri; preserves `me`, `sumti`, `mehu`, and `moi_marker` in source order.'
    __slots__ = ()
    _schema_id = 689
    __match_args__ = ('me', 'sumti', 'mehu', 'moi_marker')
    def __new__(cls, me: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], sumti: RecoveredField[SumtiSelbriSumtiSyntax], mehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, moi_marker: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> SumtiSelbriTanruUnitSyntax:
        return cls._from_fields((me, sumti, mehu, moi_marker))
    def __init__(self, me: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], sumti: RecoveredField[SumtiSelbriSumtiSyntax], mehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, moi_marker: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def me(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Me` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def sumti(self) -> RecoveredField[SumtiSelbriSumtiSyntax]:
        'The shared sumti child syntax node.'
        return cast(RecoveredField[SumtiSelbriSumtiSyntax], self._field(1))
    @property
    def mehu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Mehu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    @property
    def moi_marker(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional moi marker component.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('SumtiSelbriTanruUnitSyntax is final')

@final
class ZantufaMeTanruUnitSyntax(_SyntaxNode):
    'Product node for sumti-to-selbri; preserves `me`, `body`, `mehu`, and `moi_marker` in source order.'
    __slots__ = ()
    _schema_id = 690
    __match_args__ = ('me', 'body', 'mehu', 'moi_marker')
    def __new__(cls, me: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], body: RecoveredField[ZantufaMeSelbriBodySyntax], mehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, moi_marker: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> ZantufaMeTanruUnitSyntax:
        return cls._from_fields((me, body, mehu, moi_marker))
    def __init__(self, me: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], body: RecoveredField[ZantufaMeSelbriBodySyntax], mehu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, moi_marker: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def me(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Me` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def body(self) -> RecoveredField[ZantufaMeSelbriBodySyntax]:
        'The shared body child syntax node.'
        return cast(RecoveredField[ZantufaMeSelbriBodySyntax], self._field(1))
    @property
    def mehu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Mehu` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    @property
    def moi_marker(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional moi marker component.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaMeTanruUnitSyntax is final')

@final
class ZantufaMeSelbriBodySyntaxZantufaMeOperatorSelbriBody(_SyntaxNode):
    'Uses the `zantufa_me_operator_selbri_body` product form, whose payload preserves `operators`.'
    __slots__ = ()
    _schema_id = 691
    __match_args__ = ('zantufa_me_operator_selbri_body',)
    def __new__(cls, zantufa_me_operator_selbri_body: RecoveredField[ZantufaMeOperatorSelbriBodySyntax]) -> ZantufaMeSelbriBodySyntaxZantufaMeOperatorSelbriBody:
        return cls._from_fields((zantufa_me_operator_selbri_body,))
    def __init__(self, zantufa_me_operator_selbri_body: RecoveredField[ZantufaMeOperatorSelbriBodySyntax]) -> None:
        pass
    @property
    def zantufa_me_operator_selbri_body(self) -> RecoveredField[ZantufaMeOperatorSelbriBodySyntax]:
        'Uses the `zantufa_me_operator_selbri_body` product form, whose payload preserves `operators`.'
        return cast(RecoveredField[ZantufaMeOperatorSelbriBodySyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaMeSelbriBodySyntaxZantufaMeOperatorSelbriBody is final')

@final
class ZantufaMeSelbriBodySyntaxZantufaMeMeksoSelbriBody(_SyntaxNode):
    'Uses the `zantufa_me_mekso_selbri_body` product form, whose payload preserves `expression`.'
    __slots__ = ()
    _schema_id = 692
    __match_args__ = ('zantufa_me_mekso_selbri_body',)
    def __new__(cls, zantufa_me_mekso_selbri_body: RecoveredField[ZantufaMeMeksoSelbriBodySyntax]) -> ZantufaMeSelbriBodySyntaxZantufaMeMeksoSelbriBody:
        return cls._from_fields((zantufa_me_mekso_selbri_body,))
    def __init__(self, zantufa_me_mekso_selbri_body: RecoveredField[ZantufaMeMeksoSelbriBodySyntax]) -> None:
        pass
    @property
    def zantufa_me_mekso_selbri_body(self) -> RecoveredField[ZantufaMeMeksoSelbriBodySyntax]:
        'Uses the `zantufa_me_mekso_selbri_body` product form, whose payload preserves `expression`.'
        return cast(RecoveredField[ZantufaMeMeksoSelbriBodySyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaMeSelbriBodySyntaxZantufaMeMeksoSelbriBody is final')

@final
class ZantufaMeSelbriBodySyntaxZantufaMeTagSelbriBody(_SyntaxNode):
    'Uses the `zantufa_me_tag_selbri_body` product form, whose payload preserves `tag`.'
    __slots__ = ()
    _schema_id = 693
    __match_args__ = ('zantufa_me_tag_selbri_body',)
    def __new__(cls, zantufa_me_tag_selbri_body: RecoveredField[ZantufaMeTagSelbriBodySyntax]) -> ZantufaMeSelbriBodySyntaxZantufaMeTagSelbriBody:
        return cls._from_fields((zantufa_me_tag_selbri_body,))
    def __init__(self, zantufa_me_tag_selbri_body: RecoveredField[ZantufaMeTagSelbriBodySyntax]) -> None:
        pass
    @property
    def zantufa_me_tag_selbri_body(self) -> RecoveredField[ZantufaMeTagSelbriBodySyntax]:
        'Uses the `zantufa_me_tag_selbri_body` product form, whose payload preserves `tag`.'
        return cast(RecoveredField[ZantufaMeTagSelbriBodySyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaMeSelbriBodySyntaxZantufaMeTagSelbriBody is final')

ZantufaMeSelbriBodySyntax: TypeAlias = ZantufaMeSelbriBodySyntaxZantufaMeOperatorSelbriBody | ZantufaMeSelbriBodySyntaxZantufaMeMeksoSelbriBody | ZantufaMeSelbriBodySyntaxZantufaMeTagSelbriBody

@final
class ZantufaMeOperatorSelbriBodySyntax(_SyntaxNode):
    'Transparent product node for sumti-to-selbri; preserves the `operators` component.'
    __slots__ = ()
    _schema_id = 694
    __match_args__ = ('operators',)
    def __new__(cls, operators: Sequence[RecoveredField[MeksoOperatorSyntax]]) -> ZantufaMeOperatorSelbriBodySyntax:
        return cls._from_fields((operators,))
    def __init__(self, operators: Sequence[RecoveredField[MeksoOperatorSyntax]]) -> None:
        pass
    @property
    def operators(self) -> tuple[RecoveredField[MeksoOperatorSyntax], ...]:
        'Non-empty ordered sequence of operators components.'
        return cast(tuple[RecoveredField[MeksoOperatorSyntax], ...], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaMeOperatorSelbriBodySyntax is final')

@final
class ZantufaMeMeksoSelbriBodySyntax(_SyntaxNode):
    'Transparent product node for sumti-to-selbri; preserves the `expression` component.'
    __slots__ = ()
    _schema_id = 695
    __match_args__ = ('expression',)
    def __new__(cls, expression: RecoveredField[MeksoSyntax]) -> ZantufaMeMeksoSelbriBodySyntax:
        return cls._from_fields((expression,))
    def __init__(self, expression: RecoveredField[MeksoSyntax]) -> None:
        pass
    @property
    def expression(self) -> RecoveredField[MeksoSyntax]:
        'The shared expression child syntax node.'
        return cast(RecoveredField[MeksoSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaMeMeksoSelbriBodySyntax is final')

@final
class ZantufaMeTagSelbriBodySyntax(_SyntaxNode):
    'Transparent product node for sumti-to-selbri; preserves the `tag` component.'
    __slots__ = ()
    _schema_id = 696
    __match_args__ = ('tag',)
    def __new__(cls, tag: RecoveredField[TenseModalSyntax]) -> ZantufaMeTagSelbriBodySyntax:
        return cls._from_fields((tag,))
    def __init__(self, tag: RecoveredField[TenseModalSyntax]) -> None:
        pass
    @property
    def tag(self) -> RecoveredField[TenseModalSyntax]:
        'The shared tag child syntax node.'
        return cast(RecoveredField[TenseModalSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaMeTagSelbriBodySyntax is final')

@final
class ZantufaMexMoiTanruUnitSyntax(_SyntaxNode):
    'Product node for mex selbri; preserves `expression` and `moi` in source order.'
    __slots__ = ()
    _schema_id = 697
    __match_args__ = ('expression', 'moi')
    def __new__(cls, expression: RecoveredField[MeksoSyntax], moi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> ZantufaMexMoiTanruUnitSyntax:
        return cls._from_fields((expression, moi))
    def __init__(self, expression: RecoveredField[MeksoSyntax], moi: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]) -> None:
        pass
    @property
    def expression(self) -> RecoveredField[MeksoSyntax]:
        'The required shared mekso expression parsed by `mekso`, completed immediately before the following MOI-family word.'
        return cast(RecoveredField[MeksoSyntax], self._field(0))
    @property
    def moi(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Moi`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaMexMoiTanruUnitSyntax is final')

@final
class SumtiSelbriSumtiSyntaxSumti(_SyntaxNode):
    'Uses the `sumti` product form, whose payload preserves `base_sumti` and `vuho_attachment`.'
    __slots__ = ()
    _schema_id = 698
    __match_args__ = ('sumti',)
    def __new__(cls, sumti: RecoveredField[SumtiSyntax]) -> SumtiSelbriSumtiSyntaxSumti:
        return cls._from_fields((sumti,))
    def __init__(self, sumti: RecoveredField[SumtiSyntax]) -> None:
        pass
    @property
    def sumti(self) -> RecoveredField[SumtiSyntax]:
        'Uses the `sumti` product form, whose payload preserves `base_sumti` and `vuho_attachment`.'
        return cast(RecoveredField[SumtiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SumtiSelbriSumtiSyntaxSumti is final')

@final
class SumtiSelbriSumtiSyntaxMeLerfuSumti(_SyntaxNode):
    'Uses the `me_lerfu_sumti` product form, whose payload preserves `words`.'
    __slots__ = ()
    _schema_id = 699
    __match_args__ = ('me_lerfu_sumti',)
    def __new__(cls, me_lerfu_sumti: RecoveredField[MeLerfuSumtiSyntax]) -> SumtiSelbriSumtiSyntaxMeLerfuSumti:
        return cls._from_fields((me_lerfu_sumti,))
    def __init__(self, me_lerfu_sumti: RecoveredField[MeLerfuSumtiSyntax]) -> None:
        pass
    @property
    def me_lerfu_sumti(self) -> RecoveredField[MeLerfuSumtiSyntax]:
        'Uses the `me_lerfu_sumti` product form, whose payload preserves `words`.'
        return cast(RecoveredField[MeLerfuSumtiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('SumtiSelbriSumtiSyntaxMeLerfuSumti is final')

SumtiSelbriSumtiSyntax: TypeAlias = SumtiSelbriSumtiSyntaxSumti | SumtiSelbriSumtiSyntaxMeLerfuSumti

@final
class MeLerfuSumtiSyntax(_SyntaxNode):
    'Transparent product node for lerfu string; preserves the `words` component.'
    __slots__ = ()
    _schema_id = 700
    __match_args__ = ('words',)
    def __new__(cls, words: RecoveredField[LetterStringSyntax]) -> MeLerfuSumtiSyntax:
        return cls._from_fields((words,))
    def __init__(self, words: RecoveredField[LetterStringSyntax]) -> None:
        pass
    @property
    def words(self) -> RecoveredField[LetterStringSyntax]:
        'The `letter_string` grammar result in the `words` structural role of the `me_lerfu_sumti` production.'
        return cast(RecoveredField[LetterStringSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('MeLerfuSumtiSyntax is final')

@final
class OperatorSelbriTanruUnitSyntax(_SyntaxNode):
    'Product node for operator-to-selbri; preserves `nuha` and `mekso_operator` in source order.'
    __slots__ = ()
    _schema_id = 701
    __match_args__ = ('nuha', 'mekso_operator')
    def __new__(cls, nuha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], mekso_operator: RecoveredField[MeksoOperatorSyntax]) -> OperatorSelbriTanruUnitSyntax:
        return cls._from_fields((nuha, mekso_operator))
    def __init__(self, nuha: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], mekso_operator: RecoveredField[MeksoOperatorSyntax]) -> None:
        pass
    @property
    def nuha(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Nuha` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def mekso_operator(self) -> RecoveredField[MeksoOperatorSyntax]:
        'The shared mekso operator child syntax node.'
        return cast(RecoveredField[MeksoOperatorSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('OperatorSelbriTanruUnitSyntax is final')

@final
class GroupedTanruUnitSyntax(_SyntaxNode):
    'Product node for grouped tanru; preserves `ke`, `selbri`, and `kehe` in source order.'
    __slots__ = ()
    _schema_id = 702
    __match_args__ = ('ke', 'selbri', 'kehe')
    def __new__(cls, ke: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], selbri: RecoveredField[ConnectedSelbriSyntax], kehe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> GroupedTanruUnitSyntax:
        return cls._from_fields((ke, selbri, kehe))
    def __init__(self, ke: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], selbri: RecoveredField[ConnectedSelbriSyntax], kehe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def ke(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Ke` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def selbri(self) -> RecoveredField[ConnectedSelbriSyntax]:
        'The shared selbri child syntax node.'
        return cast(RecoveredField[ConnectedSelbriSyntax], self._field(1))
    @property
    def kehe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Kehe` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('GroupedTanruUnitSyntax is final')

@final
class GroupedJaiInnerTanruUnitSyntax(_SyntaxNode):
    'Product node for grouped tanru; preserves `ke`, `selbri`, and `kehe` in source order.'
    __slots__ = ()
    _schema_id = 703
    __match_args__ = ('ke', 'selbri', 'kehe')
    def __new__(cls, ke: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], selbri: RecoveredField[ConnectedJaiInnerSelbriSyntax], kehe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> GroupedJaiInnerTanruUnitSyntax:
        return cls._from_fields((ke, selbri, kehe))
    def __init__(self, ke: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], selbri: RecoveredField[ConnectedJaiInnerSelbriSyntax], kehe: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def ke(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Ke` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def selbri(self) -> RecoveredField[ConnectedJaiInnerSelbriSyntax]:
        'The shared selbri child syntax node.'
        return cast(RecoveredField[ConnectedJaiInnerSelbriSyntax], self._field(1))
    @property
    def kehe(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Kehe` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('GroupedJaiInnerTanruUnitSyntax is final')

@final
class ConnectedJaiInnerSelbriSyntax(_SyntaxNode):
    'Product node for selbri connection; preserves `leading_selbri` and `continuations` in source order.'
    __slots__ = ()
    _schema_id = 704
    __match_args__ = ('leading_selbri', 'continuations')
    def __new__(cls, leading_selbri: RecoveredField[TanruJaiInnerSelbriSyntax], continuations: Sequence[RecoveredField[ConnectedJaiInnerSelbriContinuationSyntax]]) -> ConnectedJaiInnerSelbriSyntax:
        return cls._from_fields((leading_selbri, continuations))
    def __init__(self, leading_selbri: RecoveredField[TanruJaiInnerSelbriSyntax], continuations: Sequence[RecoveredField[ConnectedJaiInnerSelbriContinuationSyntax]]) -> None:
        pass
    @property
    def leading_selbri(self) -> RecoveredField[TanruJaiInnerSelbriSyntax]:
        'The shared leading selbri child syntax node.'
        return cast(RecoveredField[TanruJaiInnerSelbriSyntax], self._field(0))
    @property
    def continuations(self) -> tuple[RecoveredField[ConnectedJaiInnerSelbriContinuationSyntax], ...]:
        'Ordered sequence of zero or more continuations components.'
        return cast(tuple[RecoveredField[ConnectedJaiInnerSelbriContinuationSyntax], ...], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ConnectedJaiInnerSelbriSyntax is final')

@final
class ConnectedJaiInnerSelbriContinuationSyntax(_SyntaxNode):
    'Product node for selbri connection continuation; preserves `connective` and `trailing_selbri` in source order.'
    __slots__ = ()
    _schema_id = 705
    __match_args__ = ('connective', 'trailing_selbri')
    def __new__(cls, connective: RecoveredField[RelationAfterthoughtConnectiveSyntax], trailing_selbri: RecoveredField[TanruJaiInnerSelbriSyntax]) -> ConnectedJaiInnerSelbriContinuationSyntax:
        return cls._from_fields((connective, trailing_selbri))
    def __init__(self, connective: RecoveredField[RelationAfterthoughtConnectiveSyntax], trailing_selbri: RecoveredField[TanruJaiInnerSelbriSyntax]) -> None:
        pass
    @property
    def connective(self) -> RecoveredField[RelationAfterthoughtConnectiveSyntax]:
        'The `relation_afterthought_connective` connective joining the adjacent constituents of the `connected_jai_inner_selbri_continuation` production.'
        return cast(RecoveredField[RelationAfterthoughtConnectiveSyntax], self._field(0))
    @property
    def trailing_selbri(self) -> RecoveredField[TanruJaiInnerSelbriSyntax]:
        'The shared trailing selbri child syntax node.'
        return cast(RecoveredField[TanruJaiInnerSelbriSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('ConnectedJaiInnerSelbriContinuationSyntax is final')

@final
class TanruJaiInnerSelbriSyntax(_SyntaxNode):
    'Product node for selbri; preserves `first_unit` and `additional_units` in source order.'
    __slots__ = ()
    _schema_id = 706
    __match_args__ = ('first_unit', 'additional_units')
    def __new__(cls, first_unit: RecoveredField[JaiInnerTanruUnitSyntax], additional_units: Sequence[RecoveredField[JaiInnerTanruUnitSyntax]]) -> TanruJaiInnerSelbriSyntax:
        return cls._from_fields((first_unit, additional_units))
    def __init__(self, first_unit: RecoveredField[JaiInnerTanruUnitSyntax], additional_units: Sequence[RecoveredField[JaiInnerTanruUnitSyntax]]) -> None:
        pass
    @property
    def first_unit(self) -> RecoveredField[JaiInnerTanruUnitSyntax]:
        'The initial `jai_inner_tanru_unit` constituent before the continuations of the `tanru_jai_inner_selbri` production.'
        return cast(RecoveredField[JaiInnerTanruUnitSyntax], self._field(0))
    @property
    def additional_units(self) -> tuple[RecoveredField[JaiInnerTanruUnitSyntax], ...]:
        'Ordered sequence of zero or more additional units components.'
        return cast(tuple[RecoveredField[JaiInnerTanruUnitSyntax], ...], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('TanruJaiInnerSelbriSyntax is final')

@final
class LinkedSumtiSyntaxPlaceTaggedLinkedSumti(_SyntaxNode):
    'Uses the `place_tagged_linked_sumti` product form, whose payload preserves `fa` and `sumti`.'
    __slots__ = ()
    _schema_id = 707
    __match_args__ = ('place_tagged_linked_sumti',)
    def __new__(cls, place_tagged_linked_sumti: RecoveredField[PlaceTaggedLinkedSumtiSyntax]) -> LinkedSumtiSyntaxPlaceTaggedLinkedSumti:
        return cls._from_fields((place_tagged_linked_sumti,))
    def __init__(self, place_tagged_linked_sumti: RecoveredField[PlaceTaggedLinkedSumtiSyntax]) -> None:
        pass
    @property
    def place_tagged_linked_sumti(self) -> RecoveredField[PlaceTaggedLinkedSumtiSyntax]:
        'Uses the `place_tagged_linked_sumti` product form, whose payload preserves `fa` and `sumti`.'
        return cast(RecoveredField[PlaceTaggedLinkedSumtiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('LinkedSumtiSyntaxPlaceTaggedLinkedSumti is final')

@final
class LinkedSumtiSyntaxTenseTaggedLinkedSumti(_SyntaxNode):
    'Uses the `tense_tagged_linked_sumti` product form, whose payload preserves `tense_modal` and `sumti`.'
    __slots__ = ()
    _schema_id = 708
    __match_args__ = ('tense_tagged_linked_sumti',)
    def __new__(cls, tense_tagged_linked_sumti: RecoveredField[TenseTaggedLinkedSumtiSyntax]) -> LinkedSumtiSyntaxTenseTaggedLinkedSumti:
        return cls._from_fields((tense_tagged_linked_sumti,))
    def __init__(self, tense_tagged_linked_sumti: RecoveredField[TenseTaggedLinkedSumtiSyntax]) -> None:
        pass
    @property
    def tense_tagged_linked_sumti(self) -> RecoveredField[TenseTaggedLinkedSumtiSyntax]:
        'Uses the `tense_tagged_linked_sumti` product form, whose payload preserves `tense_modal` and `sumti`.'
        return cast(RecoveredField[TenseTaggedLinkedSumtiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('LinkedSumtiSyntaxTenseTaggedLinkedSumti is final')

@final
class LinkedSumtiSyntaxPlainLinkedSumti(_SyntaxNode):
    'Uses the `plain_linked_sumti` product form, whose payload preserves `sumti`.'
    __slots__ = ()
    _schema_id = 709
    __match_args__ = ('plain_linked_sumti',)
    def __new__(cls, plain_linked_sumti: RecoveredField[PlainLinkedSumtiSyntax]) -> LinkedSumtiSyntaxPlainLinkedSumti:
        return cls._from_fields((plain_linked_sumti,))
    def __init__(self, plain_linked_sumti: RecoveredField[PlainLinkedSumtiSyntax]) -> None:
        pass
    @property
    def plain_linked_sumti(self) -> RecoveredField[PlainLinkedSumtiSyntax]:
        'Uses the `plain_linked_sumti` product form, whose payload preserves `sumti`.'
        return cast(RecoveredField[PlainLinkedSumtiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('LinkedSumtiSyntaxPlainLinkedSumti is final')

@final
class LinkedSumtiSyntaxEmptyLinkedSumti(_SyntaxNode):
    'Uses the marker-only `empty_linked_sumti` product form.'
    __slots__ = ()
    _schema_id = 710
    __match_args__ = ('empty_linked_sumti',)
    def __new__(cls, empty_linked_sumti: RecoveredField[EmptyLinkedSumtiSyntax]) -> LinkedSumtiSyntaxEmptyLinkedSumti:
        return cls._from_fields((empty_linked_sumti,))
    def __init__(self, empty_linked_sumti: RecoveredField[EmptyLinkedSumtiSyntax]) -> None:
        pass
    @property
    def empty_linked_sumti(self) -> RecoveredField[EmptyLinkedSumtiSyntax]:
        'Uses the marker-only `empty_linked_sumti` product form.'
        return cast(RecoveredField[EmptyLinkedSumtiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('LinkedSumtiSyntaxEmptyLinkedSumti is final')

LinkedSumtiSyntax: TypeAlias = LinkedSumtiSyntaxPlaceTaggedLinkedSumti | LinkedSumtiSyntaxTenseTaggedLinkedSumti | LinkedSumtiSyntaxPlainLinkedSumti | LinkedSumtiSyntaxEmptyLinkedSumti

@final
class PlaceTaggedLinkedSumtiSyntax(_SyntaxNode):
    'Product node for linked arguments; preserves `fa` and `sumti` in source order.'
    __slots__ = ()
    _schema_id = 711
    __match_args__ = ('fa', 'sumti')
    def __new__(cls, fa: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], sumti: RecoveredField[TaggedOrElidedSumtiSyntax]) -> PlaceTaggedLinkedSumtiSyntax:
        return cls._from_fields((fa, sumti))
    def __init__(self, fa: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], sumti: RecoveredField[TaggedOrElidedSumtiSyntax]) -> None:
        pass
    @property
    def fa(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Fa`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def sumti(self) -> RecoveredField[TaggedOrElidedSumtiSyntax]:
        'The shared sumti child syntax node.'
        return cast(RecoveredField[TaggedOrElidedSumtiSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('PlaceTaggedLinkedSumtiSyntax is final')

@final
class TenseTaggedLinkedSumtiSyntax(_SyntaxNode):
    'Product node for linked arguments; preserves `tense_modal` and `sumti` in source order.'
    __slots__ = ()
    _schema_id = 712
    __match_args__ = ('tense_modal', 'sumti')
    def __new__(cls, tense_modal: RecoveredField[TenseModalSyntax], sumti: RecoveredField[TaggedOrElidedSumtiSyntax]) -> TenseTaggedLinkedSumtiSyntax:
        return cls._from_fields((tense_modal, sumti))
    def __init__(self, tense_modal: RecoveredField[TenseModalSyntax], sumti: RecoveredField[TaggedOrElidedSumtiSyntax]) -> None:
        pass
    @property
    def tense_modal(self) -> RecoveredField[TenseModalSyntax]:
        'The shared tense modal child syntax node.'
        return cast(RecoveredField[TenseModalSyntax], self._field(0))
    @property
    def sumti(self) -> RecoveredField[TaggedOrElidedSumtiSyntax]:
        'The shared sumti child syntax node.'
        return cast(RecoveredField[TaggedOrElidedSumtiSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('TenseTaggedLinkedSumtiSyntax is final')

@final
class PlainLinkedSumtiSyntax(_SyntaxNode):
    'Transparent product node for linked arguments; preserves the `sumti` component.'
    __slots__ = ()
    _schema_id = 713
    __match_args__ = ('sumti',)
    def __new__(cls, sumti: RecoveredField[SumtiSyntax]) -> PlainLinkedSumtiSyntax:
        return cls._from_fields((sumti,))
    def __init__(self, sumti: RecoveredField[SumtiSyntax]) -> None:
        pass
    @property
    def sumti(self) -> RecoveredField[SumtiSyntax]:
        'The shared sumti child syntax node.'
        return cast(RecoveredField[SumtiSyntax], self._field(0))
    def __init_subclass__(cls) -> None:
        raise TypeError('PlainLinkedSumtiSyntax is final')

@final
class EmptyLinkedSumtiSyntax(_SyntaxNode):
    'Marker-only product node for linked arguments; the parser retains no public fields.'
    __slots__ = ()
    _schema_id = 714
    __match_args__ = ()
    def __new__(cls) -> EmptyLinkedSumtiSyntax:
        return cls._from_fields(())
    def __init__(self) -> None:
        pass
    def __init_subclass__(cls) -> None:
        raise TypeError('EmptyLinkedSumtiSyntax is final')

@final
class BeiLinkSyntax(_SyntaxNode):
    'Product node for linked arguments; preserves `bei` and `link` in source order.'
    __slots__ = ()
    _schema_id = 715
    __match_args__ = ('bei', 'link')
    def __new__(cls, bei: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], link: RecoveredField[LinkedSumtiSyntax]) -> BeiLinkSyntax:
        return cls._from_fields((bei, link))
    def __init__(self, bei: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], link: RecoveredField[LinkedSumtiSyntax]) -> None:
        pass
    @property
    def bei(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Bei` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def link(self) -> RecoveredField[LinkedSumtiSyntax]:
        'The `linked_sumti` grammar result in the `link` structural role of the `bei_link` production.'
        return cast(RecoveredField[LinkedSumtiSyntax], self._field(1))
    def __init_subclass__(cls) -> None:
        raise TypeError('BeiLinkSyntax is final')

@final
class LinkargsSyntax(_SyntaxNode):
    'Product node for linked arguments; preserves `be`, `first_link`, `bei_links`, and `beho` in source order.'
    __slots__ = ()
    _schema_id = 716
    __match_args__ = ('be', 'first_link', 'bei_links', 'beho')
    def __new__(cls, be: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], first_link: RecoveredField[LinkedSumtiSyntax], bei_links: Sequence[RecoveredField[BeiLinkSyntax]], beho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> LinkargsSyntax:
        return cls._from_fields((be, first_link, bei_links, beho))
    def __init__(self, be: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], first_link: RecoveredField[LinkedSumtiSyntax], bei_links: Sequence[RecoveredField[BeiLinkSyntax]], beho: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def be(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'The `Be` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def first_link(self) -> RecoveredField[LinkedSumtiSyntax]:
        'The initial `linked_sumti` constituent before the continuations of the `linkargs` production.'
        return cast(RecoveredField[LinkedSumtiSyntax], self._field(1))
    @property
    def bei_links(self) -> tuple[RecoveredField[BeiLinkSyntax], ...]:
        'Ordered sequence of zero or more bei links components.'
        return cast(tuple[RecoveredField[BeiLinkSyntax], ...], self._field(2))
    @property
    def beho(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Beho` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(3))
    def __init_subclass__(cls) -> None:
        raise TypeError('LinkargsSyntax is final')

@final
class AbstractionTanruUnitSyntax(_SyntaxNode):
    'Product node for abstraction; preserves `nu`, `nai`, `abstractor_connections`, `subbridi`, and `kei` in source order.'
    __slots__ = ()
    _schema_id = 717
    __match_args__ = ('nu', 'nai', 'abstractor_connections', 'subbridi', 'kei')
    def __new__(cls, nu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, abstractor_connections: Sequence[RecoveredField[AbstractorConnectionSyntax]], subbridi: RecoveredField[SubbridiSyntax], kei: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> AbstractionTanruUnitSyntax:
        return cls._from_fields((nu, nai, abstractor_connections, subbridi, kei))
    def __init__(self, nu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, abstractor_connections: Sequence[RecoveredField[AbstractorConnectionSyntax]], subbridi: RecoveredField[SubbridiSyntax], kei: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def nu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Nu`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def nai(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Nai` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(1))
    @property
    def abstractor_connections(self) -> tuple[RecoveredField[AbstractorConnectionSyntax], ...]:
        'Ordered sequence of zero or more abstractor connections components.'
        return cast(tuple[RecoveredField[AbstractorConnectionSyntax], ...], self._field(2))
    @property
    def subbridi(self) -> RecoveredField[SubbridiSyntax]:
        'The shared subbridi child syntax node.'
        return cast(RecoveredField[SubbridiSyntax], self._field(3))
    @property
    def kei(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Kei` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(4))
    def __init_subclass__(cls) -> None:
        raise TypeError('AbstractionTanruUnitSyntax is final')

@final
class AbstractorConnectionSyntax(_SyntaxNode):
    'Product node for abstractor connection; preserves `connective`, `nu`, and `nai` in source order.'
    __slots__ = ()
    _schema_id = 718
    __match_args__ = ('connective', 'nu', 'nai')
    def __new__(cls, connective: RecoveredField[StandardStatementConnectiveSyntax], nu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> AbstractorConnectionSyntax:
        return cls._from_fields((connective, nu, nai))
    def __init__(self, connective: RecoveredField[StandardStatementConnectiveSyntax], nu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def connective(self) -> RecoveredField[StandardStatementConnectiveSyntax]:
        'The `standard_statement_connective` connective joining the adjacent constituents of the `abstractor_connection` production.'
        return cast(RecoveredField[StandardStatementConnectiveSyntax], self._field(0))
    @property
    def nu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Nu`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(1))
    @property
    def nai(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Nai` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('AbstractorConnectionSyntax is final')

@final
class ZantufaStatementAbstractionTanruUnitSyntax(_SyntaxNode):
    'Product node for abstraction; preserves `nu`, `nai`, `abstractor_connections`, `statement`, and `kei` in source order.'
    __slots__ = ()
    _schema_id = 719
    __match_args__ = ('nu', 'nai', 'abstractor_connections', 'statement', 'kei')
    def __new__(cls, nu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, abstractor_connections: Sequence[RecoveredField[ZantufaAbstractorConnectionSyntax]], statement: RecoveredField[StatementSyntax], kei: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> ZantufaStatementAbstractionTanruUnitSyntax:
        return cls._from_fields((nu, nai, abstractor_connections, statement, kei))
    def __init__(self, nu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, abstractor_connections: Sequence[RecoveredField[ZantufaAbstractorConnectionSyntax]], statement: RecoveredField[StatementSyntax], kei: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def nu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Nu`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(0))
    @property
    def nai(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Nai` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(1))
    @property
    def abstractor_connections(self) -> tuple[RecoveredField[ZantufaAbstractorConnectionSyntax], ...]:
        'Ordered sequence of zero or more abstractor connections components.'
        return cast(tuple[RecoveredField[ZantufaAbstractorConnectionSyntax], ...], self._field(2))
    @property
    def statement(self) -> RecoveredField[StatementSyntax]:
        'The shared statement child syntax node.'
        return cast(RecoveredField[StatementSyntax], self._field(3))
    @property
    def kei(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Kei` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(4))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaStatementAbstractionTanruUnitSyntax is final')

@final
class ZantufaAbstractorConnectionSyntax(_SyntaxNode):
    'Product node for abstractor connection; preserves `connective`, `nu`, and `nai` in source order.'
    __slots__ = ()
    _schema_id = 720
    __match_args__ = ('connective', 'nu', 'nai')
    def __new__(cls, connective: RecoveredField[JoikConnectiveSyntax], nu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> ZantufaAbstractorConnectionSyntax:
        return cls._from_fields((connective, nu, nai))
    def __init__(self, connective: RecoveredField[JoikConnectiveSyntax], nu: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], nai: WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None) -> None:
        pass
    @property
    def connective(self) -> RecoveredField[JoikConnectiveSyntax]:
        'The `joik_connective` connective joining the adjacent constituents of the `zantufa_abstractor_connection` production.'
        return cast(RecoveredField[JoikConnectiveSyntax], self._field(0))
    @property
    def nu(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]]:
        'A word from selmaho `Nu`.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]], self._field(1))
    @property
    def nai(self) -> WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None:
        'The optional `Nai` cmavo marker.'
        return cast(WithFreeModifiers[RecoveredField[Token], RecoveredField[FreeModifierSyntax]] | None, self._field(2))
    def __init_subclass__(cls) -> None:
        raise TypeError('ZantufaAbstractorConnectionSyntax is final')

__all__ = (
    'LeadingIndicatorSyntax',
    'TextSyntaxExplicitXauhaLohoiText',
    'TextSyntaxRegularText',
    'TextSyntax',
    'ExplicitXauhaLohoiTextSyntax',
    'RegularTextSyntax',
    'TextParagraphsSyntaxTextParagraphWithAdditionalNiho',
    'TextParagraphsSyntaxTextNihoParagraphs',
    'TextParagraphsSyntax',
    'TextParagraphWithAdditionalNihoSyntax',
    'TextNihoParagraphsSyntax',
    'LeadingIStatementSyntax',
    'ParagraphSyntaxINihoParagraph',
    'ParagraphSyntaxSimpleParagraph',
    'ParagraphSyntax',
    'SimpleParagraphSyntax',
    'ParagraphStatementSequenceSyntax',
    'INihoParagraphSyntax',
    'NihoParagraphSyntax',
    'InitialParagraphStatementSyntax',
    'FollowingParagraphStatementSyntax',
    'TrailingIjekParagraphStatementSyntax',
    'StatementSyntaxIStatementConnection',
    'StatementSyntaxPreposedIStatementConnection',
    'StatementSyntaxStatementBase',
    'StatementSyntax',
    'StatementBaseSyntaxPrenexStatement',
    'StatementBaseSyntaxForethoughtStatement',
    'StatementBaseSyntaxBridiStatement',
    'StatementBaseSyntaxTextGroupStatement',
    'StatementBaseSyntax',
    'StatementOrFragmentSyntaxZantufaStatementTermsStatement',
    'StatementOrFragmentSyntaxStatementOrFragmentStatement',
    'StatementOrFragmentSyntaxFragmentStatement',
    'StatementOrFragmentSyntax',
    'ZantufaStatementTermsStatementSyntax',
    'ZantufaStatementTermsTailSyntaxZantufaIauStatementTermsTail',
    'ZantufaStatementTermsTailSyntaxZantufaBareStatementTermsTail',
    'ZantufaStatementTermsTailSyntax',
    'ZantufaIauStatementTermsTailSyntax',
    'ZantufaBareStatementTermsTailSyntax',
    'StatementOrFragmentStatementSyntax',
    'FragmentStatementSyntaxPrenexFragment',
    'FragmentStatementSyntaxSelbriFragment',
    'FragmentStatementSyntaxEkFragment',
    'FragmentStatementSyntaxGihekFragment',
    'FragmentStatementSyntaxMultipleNaFragment',
    'FragmentStatementSyntaxSingleNaFragment',
    'FragmentStatementSyntaxTermsFragment',
    'FragmentStatementSyntaxMeksoFragment',
    'FragmentStatementSyntaxRelativeClauseFragment',
    'FragmentStatementSyntaxLinkedSumtiContinuationFragment',
    'FragmentStatementSyntaxLinkedSumtiFragment',
    'FragmentStatementSyntaxZantufaMeksoFragment',
    'FragmentStatementSyntax',
    'StatementAfterIConnectiveSyntaxForethoughtStatement',
    'StatementAfterIConnectiveSyntaxBridiStatement',
    'StatementAfterIConnectiveSyntaxTextGroupStatement',
    'StatementAfterIConnectiveSyntax',
    'MultipleNaFragmentSyntax',
    'SingleNaFragmentSyntax',
    'EkFragmentSyntax',
    'GihekFragmentSyntax',
    'IStatementConnectionSyntax',
    'PendingIConnectiveSyntax',
    'IStatementConnectionTailSyntaxChainedIConnectiveStatementTail',
    'IStatementConnectionTailSyntaxSimpleIConnectiveStatementTail',
    'IStatementConnectionTailSyntax',
    'ChainedIConnectiveStatementTailSyntax',
    'SimpleIConnectiveStatementTailSyntax',
    'PreposedIStatementConnectionSyntax',
    'TextGroupStatementSyntax',
    'PrenexFragmentSyntax',
    'PrenexStatementSyntax',
    'ForethoughtStatementSyntax',
    'ForethoughtStatementBranchSyntax',
    'ZantufaForethoughtStatementBranchSyntax',
    'BridiStatementSyntax',
    'BridiStatementContinuationSyntaxBoBridiStatementContinuation',
    'BridiStatementContinuationSyntaxKeBridiStatementContinuation',
    'BridiStatementContinuationSyntax',
    'BoBridiStatementContinuationSyntax',
    'KeBridiStatementContinuationSyntax',
    'SelbriFragmentSyntax',
    'TermsFragmentSyntax',
    'MeksoFragmentSyntax',
    'ZantufaMeksoFragmentSyntax',
    'RelativeClauseListSyntax',
    'RelativeClauseFragmentSyntax',
    'LinkedSumtiContinuationFragmentSyntax',
    'LinkedSumtiFragmentSyntax',
    'BridiSyntaxBridiWithLeadingTerms',
    'BridiSyntaxBridiWithPostCuTerms',
    'BridiSyntaxBareCuBridi',
    'BridiSyntaxBareCuTermsBridi',
    'BridiSyntaxRelationOnlyBridi',
    'BridiSyntax',
    'BridiWithLeadingTermsSyntax',
    'BridiWithPostCuTermsSyntax',
    'BareCuBridiSyntax',
    'BareCuTermsBridiSyntax',
    'RelationOnlyBridiSyntax',
    'CuTermsBridiTailSyntax',
    'BridiTailSyntaxZantufaGroupedBridiTail',
    'BridiTailSyntaxBridiTailWithPossibleTailTerms',
    'BridiTailSyntaxBridiTailWithoutTailTerms',
    'BridiTailSyntax',
    'ZantufaGroupedBridiTailSyntax',
    'BridiTailWithoutTailTermsSyntax',
    'BridiTailWithPossibleTailTermsSyntax',
    'AfterthoughtBridiTailWithoutTailTermsSyntax',
    'AfterthoughtBridiTailSyntax',
    'BoGroupedBridiTailWithoutTailTermsSyntax',
    'BoGroupedBridiTailSyntax',
    'SimpleBridiTailWithoutTailTermsSyntaxForethoughtSimpleBridiTailWithoutTailTerms',
    'SimpleBridiTailWithoutTailTermsSyntaxSelbriSimpleBridiTailWithoutTailTerms',
    'SimpleBridiTailWithoutTailTermsSyntax',
    'SimpleBridiTailSyntaxForethoughtSimpleBridiTail',
    'SimpleBridiTailSyntaxSelbriSimpleBridiTail',
    'SimpleBridiTailSyntax',
    'ForethoughtSimpleBridiTailWithoutTailTermsSyntax',
    'ForethoughtSimpleBridiTailSyntax',
    'SelbriSimpleBridiTailWithoutTailTermsSyntax',
    'SelbriSimpleBridiTailSyntax',
    'ForethoughtBridiConnectionSyntaxDirectForethoughtBridiConnection',
    'ForethoughtBridiConnectionSyntaxGroupedForethoughtBridiConnection',
    'ForethoughtBridiConnectionSyntaxNegatedForethoughtBridiConnection',
    'ForethoughtBridiConnectionSyntax',
    'ForethoughtBridiConnectionWithoutTailTermsSyntaxDirectForethoughtBridiConnectionWithoutTailTerms',
    'ForethoughtBridiConnectionWithoutTailTermsSyntaxGroupedForethoughtBridiConnectionWithoutTailTerms',
    'ForethoughtBridiConnectionWithoutTailTermsSyntaxNegatedForethoughtBridiConnectionWithoutTailTerms',
    'ForethoughtBridiConnectionWithoutTailTermsSyntax',
    'DirectForethoughtBridiConnectionSyntax',
    'DirectForethoughtBridiConnectionWithoutTailTermsSyntax',
    'ForethoughtBridiBranchSyntax',
    'ZantufaForethoughtBridiBranchSyntax',
    'GroupedForethoughtBridiConnectionSyntax',
    'GroupedForethoughtBridiConnectionWithoutTailTermsSyntax',
    'NegatedForethoughtBridiConnectionSyntax',
    'NegatedForethoughtBridiConnectionWithoutTailTermsSyntax',
    'BridiTailKeContinuationSyntax',
    'GihekBridiTailKeContinuationSyntax',
    'BridiTailBoContinuationWithoutTailTermsSyntax',
    'BridiTailBoContinuationSyntax',
    'BridiTailContinuationWithoutTailTermsSyntax',
    'BridiTailContinuationSyntax',
    'SubbridiSyntaxPrenexSubbridi',
    'SubbridiSyntaxBridiSubbridi',
    'SubbridiSyntax',
    'BridiSubbridiSyntax',
    'PrenexSubbridiSyntax',
    'TermSyntaxPeheTermsetConnection',
    'TermSyntaxBoundTermConnection',
    'TermSyntaxTermsetGroup',
    'TermSyntaxConnectedTerm',
    'TermSyntaxSimpleTerm',
    'TermSyntax',
    'PeheTermsetConnectionSyntax',
    'PeheTermsetConnectionContinuationSyntax',
    'PeheTermsetOperandSyntaxBoundTermConnection',
    'PeheTermsetOperandSyntaxTermsetGroup',
    'PeheTermsetOperandSyntaxSimpleTerm',
    'PeheTermsetOperandSyntax',
    'SimpleTermSyntaxPlaceTaggedSumtiTerm',
    'SimpleTermSyntaxJaiTaggedSumtiTerm',
    'SimpleTermSyntaxTaggedSumtiBeforeTagTerm',
    'SimpleTermSyntaxTaggedSumtiTerm',
    'SimpleTermSyntaxNoihaAdverbialTerm',
    'SimpleTermSyntaxFihoiAdverbialTerm',
    'SimpleTermSyntaxSoiAdverbialTerm',
    'SimpleTermSyntaxNaKuTerm',
    'SimpleTermSyntaxSumtiTerm',
    'SimpleTermSyntaxBareNaTerm',
    'SimpleTermSyntaxForethoughtTermset',
    'SimpleTermSyntaxNuhiTermset',
    'SimpleTermSyntaxKeTermset',
    'SimpleTermSyntax',
    'BoundTermConnectionSyntax',
    'BoundTermConnectiveSyntaxJoikConnective',
    'BoundTermConnectiveSyntaxEkConnective',
    'BoundTermConnectiveSyntax',
    'ConnectedTermSyntax',
    'ConnectedTermContinuationSyntax',
    'ConnectedTermConnectiveSyntaxJoikConnective',
    'ConnectedTermConnectiveSyntaxJekConnective',
    'ConnectedTermConnectiveSyntaxEkConnective',
    'ConnectedTermConnectiveSyntaxVuhuNonlogicalConnective',
    'ConnectedTermConnectiveSyntax',
    'TermsetGroupSyntax',
    'TermsetGroupContinuationSyntax',
    'ForethoughtTermsetSyntax',
    'ForethoughtTermsetBranchSyntax',
    'ZantufaForethoughtTermsetBranchSyntax',
    'NuhiTermsetSyntax',
    'KeTermsetSyntax',
    'NoihaAdverbialTermSyntaxNoihaVariableAdverbialTerm',
    'NoihaAdverbialTermSyntaxNoihaRelativeAdverbialTerm',
    'NoihaAdverbialTermSyntax',
    'NoihaVariableAdverbialTermSyntax',
    'NoihaRelativeAdverbialTermSyntax',
    'FihoiAdverbialTermSyntax',
    'SoiAdverbialTermSyntax',
    'SumtiTermSyntax',
    'PlaceTaggedSumtiTermSyntax',
    'NaKuTermSyntax',
    'BareNaTermSyntax',
    'TaggedSumtiBeforeTagTermSyntax',
    'TaggedSumtiTermSyntax',
    'JaiTaggedSumtiTermSyntax',
    'LeadingTermTagTenseModalSyntaxPuBeforeNaheLeadingTermTagTense',
    'LeadingTermTagTenseModalSyntaxPuDistanceBeforeTagLeadingTermTagTense',
    'LeadingTermTagTenseModalSyntaxZiBeforeZiLeadingTermTagTense',
    'LeadingTermTagTenseModalSyntaxVaBeforeVaLeadingTermTagTense',
    'LeadingTermTagTenseModalSyntaxMohiBeforeMohiLeadingTermTagTense',
    'LeadingTermTagTenseModalSyntaxCahaBeforeTagLeadingTermTagTense',
    'LeadingTermTagTenseModalSyntaxIntervalPropertyLeadingTermTagTense',
    'LeadingTermTagTenseModalSyntaxTenseModal',
    'LeadingTermTagTenseModalSyntax',
    'PuBeforeNaheLeadingTermTagTenseSyntax',
    'PuDistanceBeforeTagLeadingTermTagTenseSyntax',
    'ZiBeforeZiLeadingTermTagTenseSyntax',
    'VaBeforeVaLeadingTermTagTenseSyntax',
    'MohiBeforeMohiLeadingTermTagTenseSyntax',
    'CahaBeforeTagLeadingTermTagTenseSyntax',
    'IntervalPropertyLeadingTermTagTenseSyntax',
    'TaggedOrElidedSumtiSyntaxSumti',
    'TaggedOrElidedSumtiSyntaxTaggedElidedSumti',
    'TaggedOrElidedSumtiSyntax',
    'TaggedElidedSumtiSyntax',
    'SumtiSyntax',
    'SumtiGroupedSyntax',
    'SumtiAfterthoughtSyntax',
    'SumtiBoundSyntax',
    'SumtiForethoughtSyntaxForethoughtSumti',
    'SumtiForethoughtSyntaxSimpleSumti',
    'SumtiForethoughtSyntax',
    'ForethoughtSumtiSyntax',
    'ForethoughtSumtiBranchSyntax',
    'ZantufaForethoughtSumtiBranchSyntax',
    'BoundSumtiTailSyntax',
    'SumtiAfterthoughtTailSyntax',
    'GroupedSumtiTailSyntax',
    'VuhoSumtiAttachmentTailSyntaxVuhoRelativeSumtiAttachmentTail',
    'VuhoSumtiAttachmentTailSyntaxVuhoConnectedSumtiAttachmentTail',
    'VuhoSumtiAttachmentTailSyntax',
    'VuhoRelativeSumtiAttachmentTailSyntax',
    'VuhoConnectedSumtiAttachmentTailSyntax',
    'SimpleSumtiSyntax',
    'SumtiAtomSyntaxSumtiBase',
    'SumtiAtomSyntaxQuantifiedSumti',
    'SumtiAtomSyntax',
    'SumtiBaseSyntaxScalarNegatedSumtiWithBo',
    'SumtiBaseSyntaxScalarNegatedSumti',
    'SumtiBaseSyntaxLaheSumti',
    'SumtiBaseSyntaxLaheTermWrapper',
    'SumtiBaseSyntaxScalarNegatedTermWrapperWithBo',
    'SumtiBaseSyntaxScalarNegatedTermWrapper',
    'SumtiBaseSyntaxBridiDescriptionSumti',
    'SumtiBaseSyntaxNameSumti',
    'SumtiBaseSyntaxDescriptionConnectionSumti',
    'SumtiBaseSyntaxDescriptorWithOuterQuantifierSumti',
    'SumtiBaseSyntaxDescriptorWithGadriSumti',
    'SumtiBaseSyntaxDescriptorWithoutGadriSumti',
    'SumtiBaseSyntaxNumberSumti',
    'SumtiBaseSyntaxLerfuStringSumti',
    'SumtiBaseSyntaxQuotedSumti',
    'SumtiBaseSyntaxProSumti',
    'SumtiBaseSyntax',
    'QuantifiedSumtiSyntax',
    'SumtiConnectionTailSyntax',
    'PaRunQuantifierSyntax',
    'MeksoQuantifierSyntax',
    'ZantufaRawMeksoQuantifierSyntax',
    'ZantufaPriorityRawMeksoQuantifierSyntax',
    'QuantifierSyntaxZantufaPriorityRawMeksoQuantifier',
    'QuantifierSyntaxMeksoQuantifier',
    'QuantifierSyntaxPaRunQuantifier',
    'QuantifierSyntaxZantufaRawMeksoQuantifier',
    'QuantifierSyntax',
    'NumberMeksoSyntax',
    'PrimitiveMeksoOperatorSyntax',
    'MeksoOperatorSyntaxAfterthoughtMeksoOperator',
    'MeksoOperatorSyntaxBoundMeksoOperator',
    'MeksoOperatorSyntaxSimpleMeksoOperator',
    'MeksoOperatorSyntax',
    'AfterthoughtMeksoOperatorSyntax',
    'AfterthoughtMeksoOperatorContinuationSyntax',
    'BoundOrAtomMeksoOperatorSyntaxBoundMeksoOperator',
    'BoundOrAtomMeksoOperatorSyntaxSimpleMeksoOperator',
    'BoundOrAtomMeksoOperatorSyntax',
    'BoundMeksoOperatorSyntax',
    'SimpleMeksoOperatorSyntaxConvertedMeksoOperator',
    'SimpleMeksoOperatorSyntaxScalarNegatedMeksoOperator',
    'SimpleMeksoOperatorSyntaxForethoughtMeksoOperator',
    'SimpleMeksoOperatorSyntaxGroupedMeksoOperator',
    'SimpleMeksoOperatorSyntaxSelbriMeksoOperator',
    'SimpleMeksoOperatorSyntaxOperandMeksoOperator',
    'SimpleMeksoOperatorSyntaxZantufaMahoSelbriMeksoOperator',
    'SimpleMeksoOperatorSyntaxZantufaMahoSumtiMeksoOperator',
    'SimpleMeksoOperatorSyntaxZantufaConnectiveMeksoOperator',
    'SimpleMeksoOperatorSyntaxPrimitiveMeksoOperator',
    'SimpleMeksoOperatorSyntax',
    'ConvertedMeksoOperatorSyntax',
    'ScalarNegatedMeksoOperatorSyntax',
    'ForethoughtMeksoOperatorSyntax',
    'GroupedMeksoOperatorSyntax',
    'SelbriMeksoOperatorSyntax',
    'OperandMeksoOperatorSyntax',
    'ZantufaMahoSelbriMeksoOperatorSyntax',
    'ZantufaMahoSumtiMeksoOperatorSyntax',
    'ZantufaConnectiveMeksoOperatorSyntax',
    'MeksoOperandSyntaxAfterthoughtMeksoOperand',
    'MeksoOperandSyntaxBoundMeksoOperand',
    'MeksoOperandSyntaxSimpleMeksoOperand',
    'MeksoOperandSyntax',
    'AfterthoughtMeksoOperandSyntax',
    'AfterthoughtMeksoOperandContinuationSyntax',
    'BoundOrSimpleMeksoOperandSyntaxBoundMeksoOperand',
    'BoundOrSimpleMeksoOperandSyntaxSimpleMeksoOperand',
    'BoundOrSimpleMeksoOperandSyntax',
    'BoundMeksoOperandSyntax',
    'SimpleMeksoOperandSyntaxForethoughtMeksoOperand',
    'SimpleMeksoOperandSyntaxQualifiedMeksoOperand',
    'SimpleMeksoOperandSyntaxParenthesizedMeksoOperand',
    'SimpleMeksoOperandSyntaxSumtiMeksoOperand',
    'SimpleMeksoOperandSyntaxSelbriMeksoOperand',
    'SimpleMeksoOperandSyntaxArrayMeksoOperand',
    'SimpleMeksoOperandSyntaxNumberMekso',
    'SimpleMeksoOperandSyntaxLerfuStringMekso',
    'SimpleMeksoOperandSyntaxZantufaScalarNegatedMeksoOperand',
    'SimpleMeksoOperandSyntaxZantufaSelbriMoheMeksoOperand',
    'SimpleMeksoOperandSyntax',
    'ZantufaScalarNegatedMeksoOperandSyntax',
    'QualifiedMeksoOperandSyntax',
    'ForethoughtMeksoOperandSyntax',
    'SumtiMeksoOperandSyntax',
    'ZantufaSelbriMoheMeksoOperandSyntax',
    'SelbriMeksoOperandSyntax',
    'ParenthesizedMeksoOperandSyntax',
    'ArrayMeksoOperandSyntax',
    'LetterStringSyntax',
    'LetterStringContinuationSyntaxLetterStringPaContinuation',
    'LetterStringContinuationSyntaxLetterStringLerfuContinuation',
    'LetterStringContinuationSyntax',
    'LetterStringPaContinuationSyntax',
    'LetterStringLerfuContinuationSyntax',
    'NumberWordsSyntax',
    'NumberWordContinuationSyntaxNumberWordPaContinuation',
    'NumberWordContinuationSyntaxNumberWordLerfuContinuation',
    'NumberWordContinuationSyntax',
    'NumberWordPaContinuationSyntax',
    'NumberWordLerfuContinuationSyntax',
    'NumberOrLetterWordsSyntaxNumberWords',
    'NumberOrLetterWordsSyntaxLetterString',
    'NumberOrLetterWordsSyntax',
    'LetterTokensSyntaxSimpleLerfuWord',
    'LetterTokensSyntaxLauLerfuWord',
    'LetterTokensSyntaxTeiLerfuWord',
    'LetterTokensSyntax',
    'SimpleLerfuWordSyntax',
    'LauLerfuWordSyntax',
    'TeiLerfuWordSyntax',
    'LerfuStringMeksoSyntax',
    'MeksoBaseSyntaxZantufaBoGroupedMeksoBase',
    'MeksoBaseSyntaxMeksoOperand',
    'MeksoBaseSyntaxForethoughtCallMekso',
    'MeksoBaseSyntaxZantufaGroupedMeksoOperandSequence',
    'MeksoBaseSyntax',
    'ZantufaBoGroupedMeksoBaseSyntax',
    'ZantufaBoGroupedMeksoContinuationSyntax',
    'ZantufaGroupedMeksoOperandSequenceSyntax',
    'MeksoPrecedenceSyntax',
    'MeksoPrecedenceTailSyntax',
    'InfixMeksoSyntax',
    'InfixMeksoContinuationSyntax',
    'ZantufaInfixMeksoSyntax',
    'ZantufaInfixMeksoContinuationSyntax',
    'ForethoughtCallMeksoSyntax',
    'MeksoSyntaxZantufaReversePolishMekso',
    'MeksoSyntaxZantufaInfixMekso',
    'MeksoSyntaxInfixMekso',
    'MeksoSyntaxReversePolishMekso',
    'MeksoSyntax',
    'ZantufaReversePolishMeksoSyntax',
    'ZantufaReversePolishTailSyntax',
    'ReversePolishPartsSyntax',
    'ReversePolishPartsTailSyntax',
    'ReversePolishMeksoSyntax',
    'NumberSumtiSyntax',
    'LerfuStringSumtiSyntax',
    'LaheSumtiSyntax',
    'LaheTermWrapperSyntax',
    'ScalarNegatedTermWrapperWithBoSyntax',
    'ScalarNegatedTermWrapperSyntax',
    'ScalarNegatedSumtiWithBoSyntax',
    'ScalarNegatedSumtiSyntax',
    'BridiDescriptionSumtiSyntax',
    'LohoiDescriptionHeadContinuationSyntax',
    'ProSumtiSyntax',
    'NameSumtiSyntax',
    'DescriptionHeadSyntax',
    'DescriptionHeadConnectiveSyntax',
    'DescriptionConnectionSumtiSyntax',
    'DescriptorWithGadriSumtiSyntax',
    'DescriptorWithOuterQuantifierSumtiSyntax',
    'DescriptorWithoutGadriSumtiSyntax',
    'DescriptionTailSyntax',
    'DescriptionTailBodySyntaxQuantifierRelationDescriptionTail',
    'DescriptionTailBodySyntaxQuantifierSumtiDescriptionTail',
    'DescriptionTailBodySyntaxRelationDescriptionTail',
    'DescriptionTailBodySyntax',
    'LeadingDescriptionTailElementsSyntax',
    'DescriptionTailSumtiSyntax',
    'RelationDescriptionTailSyntax',
    'QuantifierRelationDescriptionTailSyntax',
    'QuantifierSumtiDescriptionTailSyntax',
    'QuoteSyntaxExperimentalMehoiCompoundQuote',
    'QuoteSyntaxExperimentalZohoiCompoundQuote',
    'QuoteSyntaxExperimentalRahoiCompoundQuote',
    'QuoteSyntaxExperimentalGohoiCompoundQuote',
    'QuoteSyntaxGenericCompoundQuote',
    'QuoteSyntaxTextQuote',
    'QuoteSyntax',
    'TextQuoteSyntax',
    'ExperimentalMehoiCompoundQuoteSyntax',
    'ExperimentalZohoiCompoundQuoteSyntax',
    'ExperimentalRahoiCompoundQuoteSyntax',
    'ExperimentalGohoiCompoundQuoteSyntax',
    'GenericCompoundQuoteSyntax',
    'QuotedSumtiSyntax',
    'SelbriVocativeSumtiSyntax',
    'CmevlaVocativeSumtiSyntax',
    'VocativeSumtiSyntaxSelbriVocativeSumti',
    'VocativeSumtiSyntaxCmevlaVocativeSumti',
    'VocativeSumtiSyntaxSumti',
    'VocativeSumtiSyntax',
    'VocativeMarkerWordsSyntaxCoiVocativeMarkerWords',
    'VocativeMarkerWordsSyntaxDoiVocativeMarkerWords',
    'VocativeMarkerWordsSyntax',
    'CoiVocativeMarkerWordsSyntax',
    'AdditionalCoiVocativeMarkerSyntax',
    'DoiVocativeMarkerWordsSyntax',
    'FreeModifierSyntaxTextReplacementFreeModifier',
    'FreeModifierSyntaxZantufaSeiStatementFreeModifier',
    'FreeModifierSyntaxSeiFreeModifier',
    'FreeModifierSyntaxXiFreeModifier',
    'FreeModifierSyntaxMaiFreeModifier',
    'FreeModifierSyntaxZantufaMeksoMaiFreeModifier',
    'FreeModifierSyntaxSoiFreeModifier',
    'FreeModifierSyntaxParentheticalText',
    'FreeModifierSyntaxVocativeFreeModifier',
    'FreeModifierSyntax',
    'VocativeFreeModifierSyntax',
    'ParentheticalTextSyntax',
    'SeiFreeModifierSyntax',
    'ZantufaSeiStatementFreeModifierSyntax',
    'XiFreeModifierSyntaxXiNumberFreeModifier',
    'XiFreeModifierSyntaxXiLerfuStringFreeModifier',
    'XiFreeModifierSyntaxXiParenthesizedFreeModifier',
    'XiFreeModifierSyntax',
    'XiNumberFreeModifierSyntax',
    'XiLerfuStringFreeModifierSyntax',
    'XiParenthesizedFreeModifierSyntax',
    'MaiFreeModifierSyntax',
    'ZantufaMeksoMaiFreeModifierSyntax',
    'SoiFreeModifierSyntax',
    'TextReplacementFreeModifierSyntaxFullTextReplacementFreeModifier',
    'TextReplacementFreeModifierSyntaxNewOnlyTextReplacementFreeModifier',
    'TextReplacementFreeModifierSyntaxCloseOnlyTextReplacementFreeModifier',
    'TextReplacementFreeModifierSyntax',
    'FullTextReplacementFreeModifierSyntax',
    'NewOnlyTextReplacementFreeModifierSyntax',
    'CloseOnlyTextReplacementFreeModifierSyntax',
    'RelativeClauseTailSyntaxJoinedRelativeClauseTail',
    'RelativeClauseTailSyntaxConnectedRelativeClauseTail',
    'RelativeClauseTailSyntax',
    'JoinedRelativeClauseTailSyntax',
    'ConnectedRelativeClauseTailSyntax',
    'RelativeClauseConnectiveSyntaxJoikConnective',
    'RelativeClauseConnectiveSyntaxJekConnective',
    'RelativeClauseConnectiveSyntax',
    'RelativeClauseAtomSyntaxSumtiAssociationRelativeClause',
    'RelativeClauseAtomSyntaxBridiRelativeClause',
    'RelativeClauseAtomSyntax',
    'SumtiAssociationRelativeClauseSyntax',
    'RelativeSumtiSyntaxTenseTaggedRelativeSumti',
    'RelativeSumtiSyntaxNaKuRelativeSumti',
    'RelativeSumtiSyntaxPlainRelativeSumti',
    'RelativeSumtiSyntax',
    'NaKuRelativeSumtiSyntax',
    'TenseTaggedRelativeSumtiSyntax',
    'PlainRelativeSumtiSyntax',
    'BridiRelativeClauseSyntaxZantufaRestrictiveStatementRelativeClause',
    'BridiRelativeClauseSyntaxZantufaIncidentalStatementRelativeClause',
    'BridiRelativeClauseSyntaxRestrictiveBridiRelativeClause',
    'BridiRelativeClauseSyntaxIncidentalBridiRelativeClause',
    'BridiRelativeClauseSyntax',
    'ZantufaRestrictiveStatementRelativeClauseSyntax',
    'ZantufaIncidentalStatementRelativeClauseSyntax',
    'RestrictiveBridiRelativeClauseSyntax',
    'IncidentalBridiRelativeClauseSyntax',
    'EkConnectiveSyntax',
    'JehiConnectiveSyntax',
    'JekConnectiveSyntax',
    'JoikConnectiveSyntaxJoiConnective',
    'JoikConnectiveSyntaxSimpleIntervalConnective',
    'JoikConnectiveSyntaxClosedIntervalConnective',
    'JoikConnectiveSyntax',
    'JoiConnectiveSyntax',
    'SimpleIntervalConnectiveSyntax',
    'ClosedIntervalConnectiveSyntax',
    'VuhuNonlogicalConnectiveSyntax',
    'ArgumentConnectiveSyntaxCeheConnective',
    'ArgumentConnectiveSyntaxEkConnective',
    'ArgumentConnectiveSyntaxJehiConnective',
    'ArgumentConnectiveSyntaxJoikConnective',
    'ArgumentConnectiveSyntaxVuhuNonlogicalConnective',
    'ArgumentConnectiveSyntax',
    'OperandConnectiveSyntaxJoikConnective',
    'OperandConnectiveSyntaxEkConnective',
    'OperandConnectiveSyntaxJekConnective',
    'OperandConnectiveSyntax',
    'RelationAfterthoughtConnectiveSyntaxJoikConnective',
    'RelationAfterthoughtConnectiveSyntaxJekConnective',
    'RelationAfterthoughtConnectiveSyntaxEkConnective',
    'RelationAfterthoughtConnectiveSyntaxVuhuNonlogicalConnective',
    'RelationAfterthoughtConnectiveSyntax',
    'StandardStatementConnectiveSyntaxJoikConnective',
    'StandardStatementConnectiveSyntaxJekConnective',
    'StandardStatementConnectiveSyntax',
    'StatementConnectiveSyntaxJoikConnective',
    'StatementConnectiveSyntaxJekConnective',
    'StatementConnectiveSyntaxEkConnective',
    'StatementConnectiveSyntaxVuhuNonlogicalConnective',
    'StatementConnectiveSyntax',
    'TextLeadingConnectiveSyntaxStandardStatementConnective',
    'TextLeadingConnectiveSyntaxCeheConnective',
    'TextLeadingConnectiveSyntax',
    'IStatementConnectiveSyntaxIStandardStatementConnective',
    'IStatementConnectiveSyntaxITagBoStatementConnective',
    'IStatementConnectiveSyntax',
    'IStandardStatementConnectiveSyntax',
    'IParagraphStatementConnectiveSyntaxIStandardParagraphStatementConnective',
    'IParagraphStatementConnectiveSyntaxITagBoParagraphStatementConnective',
    'IParagraphStatementConnectiveSyntax',
    'IStandardParagraphStatementConnectiveSyntax',
    'ParagraphStandardStatementConnectiveSyntaxParagraphJoiConnective',
    'ParagraphStandardStatementConnectiveSyntaxParagraphSimpleIntervalConnective',
    'ParagraphStandardStatementConnectiveSyntaxParagraphClosedIntervalConnective',
    'ParagraphStandardStatementConnectiveSyntaxParagraphJekConnective',
    'ParagraphStandardStatementConnectiveSyntax',
    'ParagraphJekConnectiveSyntax',
    'ParagraphJoiConnectiveSyntax',
    'ParagraphSimpleIntervalConnectiveSyntax',
    'ParagraphClosedIntervalConnectiveSyntax',
    'ITagBoParagraphStatementConnectiveSyntax',
    'ITagBoStatementConnectiveSyntax',
    'CeheConnectiveSyntax',
    'GihekConnectiveSyntax',
    'GuhekConnectiveSyntax',
    'BridiTailConnectiveSyntaxGihekConnective',
    'BridiTailConnectiveSyntaxRelationConnectiveAsBridiTail',
    'BridiTailConnectiveSyntax',
    'RelationConnectiveAsBridiTailSyntax',
    'ModalForethoughtConnectiveSyntaxGaForethoughtConnective',
    'ModalForethoughtConnectiveSyntaxJoikJekGiForethoughtConnective',
    'ModalForethoughtConnectiveSyntaxJekGiForethoughtConnective',
    'ModalForethoughtConnectiveSyntaxModalGiForethoughtConnective',
    'ModalForethoughtConnectiveSyntaxZantufaInitialGiForethoughtConnective',
    'ModalForethoughtConnectiveSyntax',
    'GaForethoughtConnectiveSyntax',
    'ZantufaInitialGiForethoughtConnectiveSyntax',
    'JoikJekGiForethoughtConnectiveSyntax',
    'JekGiForethoughtConnectiveSyntax',
    'ModalGiForethoughtConnectiveSyntax',
    'GikConnectiveSyntax',
    'ZantufaExtraGikConnectiveSyntax',
    'TenseModalSyntax',
    'TenseModalBodySyntaxConnectedTenseModal',
    'TenseModalBodySyntaxTenseModalAtom',
    'TenseModalBodySyntax',
    'ConnectedTenseModalSyntax',
    'ConnectedTenseModalContinuationSyntax',
    'TenseModalConnectiveSyntaxJoikConnective',
    'TenseModalConnectiveSyntaxJekConnective',
    'TenseModalConnectiveSyntax',
    'TenseModalAtomSyntaxCompositeTense',
    'TenseModalAtomSyntaxFihoTense',
    'TenseModalAtomSyntaxModalTense',
    'TenseModalAtomSyntaxNaheSeFlatPrefixedTense',
    'TenseModalAtomSyntaxSeFlatPrefixedTense',
    'TenseModalAtomSyntaxFaFlatTagTense',
    'TenseModalAtomSyntaxZantufaRecursiveTagTense',
    'TenseModalAtomSyntaxStickyTense',
    'TenseModalAtomSyntax',
    'FihoTenseSyntax',
    'FaFlatTagTenseSyntax',
    'FlatTagAtomSyntaxFaFlatTagAtom',
    'FlatTagAtomSyntaxModalFlatTagAtom',
    'FlatTagAtomSyntaxCompositeFlatTagAtom',
    'FlatTagAtomSyntax',
    'FaFlatTagAtomSyntax',
    'ModalFlatTagAtomSyntax',
    'CompositeFlatTagAtomSyntax',
    'NaheSeFlatPrefixedTenseSyntax',
    'SeFlatPrefixedTenseSyntax',
    'ZantufaRecursiveTagTenseSyntax',
    'CompositeTenseSyntaxPrefixedTimeSpaceCahaTense',
    'CompositeTenseSyntaxTimeSpaceCahaKiTense',
    'CompositeTenseSyntaxCuheTense',
    'CompositeTenseSyntax',
    'PrefixedTimeSpaceCahaTenseSyntax',
    'TimeSpaceCahaKiTenseSyntax',
    'TimeSpaceCahaTenseSyntaxTimeThenSpaceCahaTense',
    'TimeSpaceCahaTenseSyntaxSpaceThenTimeCahaTense',
    'TimeSpaceCahaTenseSyntaxCahaTense',
    'TimeSpaceCahaTenseSyntax',
    'TimeThenSpaceCahaTenseSyntax',
    'SpaceThenTimeCahaTenseSyntax',
    'TimeTenseSyntaxTimeTenseWithZi',
    'TimeTenseSyntaxTimeTenseWithOffset',
    'TimeTenseSyntaxTimeTenseWithInterval',
    'TimeTenseSyntaxTimeTenseWithProperties',
    'TimeTenseSyntax',
    'TimeTenseWithZiSyntax',
    'TimeTenseWithOffsetSyntax',
    'TimeTenseWithIntervalSyntax',
    'TimeTenseWithPropertiesSyntax',
    'IntervalPropertyTenseSyntaxNumberedIntervalPropertyTense',
    'IntervalPropertyTenseSyntaxTaheIntervalPropertyTense',
    'IntervalPropertyTenseSyntaxZahoIntervalPropertyTense',
    'IntervalPropertyTenseSyntax',
    'NumberedIntervalPropertyTenseSyntax',
    'IntervalPropertyNumberWordsSyntax',
    'IntervalPropertyNumberWordContinuationSyntaxIntervalPropertyNumberPaContinuation',
    'IntervalPropertyNumberWordContinuationSyntaxIntervalPropertyNumberLetterContinuation',
    'IntervalPropertyNumberWordContinuationSyntax',
    'IntervalPropertyNumberPaContinuationSyntax',
    'IntervalPropertyNumberLetterContinuationSyntax',
    'TaheIntervalPropertyTenseSyntax',
    'ZahoIntervalPropertyTenseSyntax',
    'PuTimeOffsetTenseSyntax',
    'ZiTimeDistanceTenseSyntax',
    'ZehaTimeIntervalTenseSyntax',
    'SpaceTenseSyntaxSpaceTenseWithVa',
    'SpaceTenseSyntaxSpaceTenseWithOffset',
    'SpaceTenseSyntaxSpaceTenseWithInterval',
    'SpaceTenseSyntaxSpaceTenseWithMohi',
    'SpaceTenseSyntax',
    'SpaceTenseWithVaSyntax',
    'SpaceTenseWithOffsetSyntax',
    'SpaceTenseWithIntervalSyntax',
    'SpaceTenseWithMohiSyntax',
    'VaSpaceDistanceTenseSyntax',
    'FahaSpaceOffsetTenseSyntax',
    'FahaIntervalDirectionTenseSyntax',
    'SpaceIntervalTenseSyntaxSpaceIntervalWithExtentTense',
    'SpaceIntervalTenseSyntaxSpaceIntervalPropertiesTense',
    'SpaceIntervalTenseSyntax',
    'SpaceIntervalWithExtentTenseSyntax',
    'SpaceIntervalExtentTenseSyntaxVehaSpaceIntervalTense',
    'SpaceIntervalExtentTenseSyntaxVihaSpaceIntervalTense',
    'SpaceIntervalExtentTenseSyntax',
    'SpaceIntervalPropertiesTenseSyntax',
    'VehaSpaceIntervalTenseSyntax',
    'VihaSpaceIntervalTenseSyntax',
    'FeheIntervalPropertyTenseSyntax',
    'MohiSpaceOffsetTenseSyntax',
    'CahaTenseSyntax',
    'KiCompositeTenseSyntax',
    'CuheTenseSyntax',
    'ModalTenseSyntax',
    'StickyTenseSyntax',
    'SelbriSyntaxTaggedSelbri',
    'SelbriSyntaxUntaggedSelbri',
    'SelbriSyntax',
    'UntaggedSelbriSyntaxNegatedSelbri',
    'UntaggedSelbriSyntaxCoSelbri',
    'UntaggedSelbriSyntaxForethoughtSelbriConnection',
    'UntaggedSelbriSyntax',
    'TaggedSelbriSyntax',
    'NegatedSelbriSyntax',
    'CoSelbriSyntax',
    'CoSelbriTailSyntax',
    'ForethoughtSelbriConnectionSyntax',
    'ForethoughtSelbriBranchSyntax',
    'ZantufaForethoughtSelbriBranchSyntax',
    'ConnectedSelbriSyntax',
    'ConnectedSelbriContinuationSyntax',
    'TanruSelbriSyntax',
    'TanruUnitSyntax',
    'TanruUnitContinuationSyntax',
    'BoOrLinkedTanruUnitSyntaxForethoughtSelbriGroupTanruUnit',
    'BoOrLinkedTanruUnitSyntaxBoundTanruUnit',
    'BoOrLinkedTanruUnitSyntaxAssignedProBridiTanruUnit',
    'BoOrLinkedTanruUnitSyntaxLinkedTanruUnit',
    'BoOrLinkedTanruUnitSyntax',
    'ForethoughtSelbriGroupTanruUnitSyntax',
    'ForethoughtSelbriGroupBranchSyntax',
    'ZantufaForethoughtSelbriGroupBranchSyntax',
    'BoundTanruUnitSyntax',
    'AssignedProBridiTanruUnitSyntax',
    'ProBridiTanruUnitAssignmentSyntax',
    'LinkedTanruUnitSyntax',
    'LinkedTanruUnitForCeiSyntax',
    'TanruUnitAtomForCeiSyntax',
    'TanruUnitAtomBaseForCeiSyntaxProBridiTanruUnit',
    'TanruUnitAtomBaseForCeiSyntaxOrdinalTanruUnit',
    'TanruUnitAtomBaseForCeiSyntaxWordTanruUnit',
    'TanruUnitAtomBaseForCeiSyntaxPreposedLinkargsTanruUnit',
    'TanruUnitAtomBaseForCeiSyntaxJaiModalTanruUnit',
    'TanruUnitAtomBaseForCeiSyntaxScalarNegatedTanruUnit',
    'TanruUnitAtomBaseForCeiSyntaxZantufaStatementAbstractionTanruUnit',
    'TanruUnitAtomBaseForCeiSyntaxAbstractionTanruUnit',
    'TanruUnitAtomBaseForCeiSyntaxSumtiSelbriTanruUnit',
    'TanruUnitAtomBaseForCeiSyntaxZantufaMeTanruUnit',
    'TanruUnitAtomBaseForCeiSyntaxZantufaMexMoiTanruUnit',
    'TanruUnitAtomBaseForCeiSyntaxOperatorSelbriTanruUnit',
    'TanruUnitAtomBaseForCeiSyntaxQuotedBridiSelbriTanruUnit',
    'TanruUnitAtomBaseForCeiSyntaxQuotedTextSelbriTanruUnit',
    'TanruUnitAtomBaseForCeiSyntaxTextSelbriTanruUnit',
    'TanruUnitAtomBaseForCeiSyntaxTagSelbriTanruUnit',
    'TanruUnitAtomBaseForCeiSyntaxGohaWordTanruUnit',
    'TanruUnitAtomBaseForCeiSyntaxGroupedTanruUnit',
    'TanruUnitAtomBaseForCeiSyntax',
    'TanruUnitAtomSyntax',
    'TanruUnitAtomBaseSyntaxOrdinalTanruUnit',
    'TanruUnitAtomBaseSyntaxWordTanruUnit',
    'TanruUnitAtomBaseSyntaxPreposedLinkargsTanruUnit',
    'TanruUnitAtomBaseSyntaxJaiModalTanruUnit',
    'TanruUnitAtomBaseSyntaxScalarNegatedTanruUnit',
    'TanruUnitAtomBaseSyntaxZantufaStatementAbstractionTanruUnit',
    'TanruUnitAtomBaseSyntaxAbstractionTanruUnit',
    'TanruUnitAtomBaseSyntaxSumtiSelbriTanruUnit',
    'TanruUnitAtomBaseSyntaxZantufaMeTanruUnit',
    'TanruUnitAtomBaseSyntaxZantufaMexMoiTanruUnit',
    'TanruUnitAtomBaseSyntaxOperatorSelbriTanruUnit',
    'TanruUnitAtomBaseSyntaxQuotedBridiSelbriTanruUnit',
    'TanruUnitAtomBaseSyntaxQuotedTextSelbriTanruUnit',
    'TanruUnitAtomBaseSyntaxTextSelbriTanruUnit',
    'TanruUnitAtomBaseSyntaxTagSelbriTanruUnit',
    'TanruUnitAtomBaseSyntaxGohaWordTanruUnit',
    'TanruUnitAtomBaseSyntaxProBridiTanruUnit',
    'TanruUnitAtomBaseSyntaxGroupedTanruUnit',
    'TanruUnitAtomBaseSyntax',
    'TaggedSelbriGroupTanruUnitSyntax',
    'PreposedLinkargsTanruUnitSyntax',
    'ScalarNegatedTanruUnitSyntax',
    'ScalarNegatedTanruInnerUnitSyntaxTaggedSelbriGroupTanruUnit',
    'ScalarNegatedTanruInnerUnitSyntaxProBridiTanruUnit',
    'ScalarNegatedTanruInnerUnitSyntaxTanruUnitAtom',
    'ScalarNegatedTanruInnerUnitSyntax',
    'JaiModalTanruUnitSyntax',
    'JaiInnerTanruUnitSyntaxConvertedJaiInnerTanruUnit',
    'JaiInnerTanruUnitSyntaxScalarNegatedJaiInnerTanruUnit',
    'JaiInnerTanruUnitSyntaxSumtiSelbriTanruUnit',
    'JaiInnerTanruUnitSyntaxQuotedBridiSelbriTanruUnit',
    'JaiInnerTanruUnitSyntaxQuotedTextSelbriTanruUnit',
    'JaiInnerTanruUnitSyntaxTextSelbriTanruUnit',
    'JaiInnerTanruUnitSyntaxGroupedJaiInnerTanruUnit',
    'JaiInnerTanruUnitSyntaxOrdinalTanruUnit',
    'JaiInnerTanruUnitSyntaxOperatorSelbriTanruUnit',
    'JaiInnerTanruUnitSyntaxProBridiTanruUnit',
    'JaiInnerTanruUnitSyntaxWordTanruUnit',
    'JaiInnerTanruUnitSyntax',
    'ConvertedJaiInnerTanruUnitSyntax',
    'ScalarNegatedJaiInnerTanruUnitSyntax',
    'QuotedBridiSelbriTanruUnitSyntax',
    'TextSelbriTanruUnitSyntax',
    'QuotedTextSelbriTanruUnitSyntax',
    'TagSelbriTanruUnitSyntax',
    'OrdinalTanruUnitSyntax',
    'WordTanruUnitSyntax',
    'GohaWordTanruUnitSyntax',
    'ProBridiTanruUnitSyntax',
    'SumtiSelbriTanruUnitSyntax',
    'ZantufaMeTanruUnitSyntax',
    'ZantufaMeSelbriBodySyntaxZantufaMeOperatorSelbriBody',
    'ZantufaMeSelbriBodySyntaxZantufaMeMeksoSelbriBody',
    'ZantufaMeSelbriBodySyntaxZantufaMeTagSelbriBody',
    'ZantufaMeSelbriBodySyntax',
    'ZantufaMeOperatorSelbriBodySyntax',
    'ZantufaMeMeksoSelbriBodySyntax',
    'ZantufaMeTagSelbriBodySyntax',
    'ZantufaMexMoiTanruUnitSyntax',
    'SumtiSelbriSumtiSyntaxSumti',
    'SumtiSelbriSumtiSyntaxMeLerfuSumti',
    'SumtiSelbriSumtiSyntax',
    'MeLerfuSumtiSyntax',
    'OperatorSelbriTanruUnitSyntax',
    'GroupedTanruUnitSyntax',
    'GroupedJaiInnerTanruUnitSyntax',
    'ConnectedJaiInnerSelbriSyntax',
    'ConnectedJaiInnerSelbriContinuationSyntax',
    'TanruJaiInnerSelbriSyntax',
    'LinkedSumtiSyntaxPlaceTaggedLinkedSumti',
    'LinkedSumtiSyntaxTenseTaggedLinkedSumti',
    'LinkedSumtiSyntaxPlainLinkedSumti',
    'LinkedSumtiSyntaxEmptyLinkedSumti',
    'LinkedSumtiSyntax',
    'PlaceTaggedLinkedSumtiSyntax',
    'TenseTaggedLinkedSumtiSyntax',
    'PlainLinkedSumtiSyntax',
    'EmptyLinkedSumtiSyntax',
    'BeiLinkSyntax',
    'LinkargsSyntax',
    'AbstractionTanruUnitSyntax',
    'AbstractorConnectionSyntax',
    'ZantufaStatementAbstractionTanruUnitSyntax',
    'ZantufaAbstractorConnectionSyntax'
)

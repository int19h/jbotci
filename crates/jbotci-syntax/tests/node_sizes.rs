use std::{cmp::Reverse, mem::size_of};

#[allow(unused_imports)]
use bityzba::{ensures, requires};
use jbotci_morphology::{Jvopau, Verbatim, Word, WordLike};
use jbotci_source::SourceSpan;
use jbotci_syntax::{
    Token, WithIndicators,
    ast::{
        AbstractionSyntax, AdditionalNuSyntax, ArgumentConnectionSyntax, ArgumentSyntax,
        ArgumentTagSyntax, ArgumentTailElementSyntax, BeLinkSyntax, BeiLinkSyntax,
        BoPredicateTailSyntax, CeiAssignmentSyntax, CompositeTenseModalPartSyntax,
        ConnectedDescriptorSyntax, ConnectiveKind, ConnectiveSyntax, DescriptorHeadSyntax,
        DescriptorSyntax, FihoModalSyntax, FragmentSyntax, FreeModifierSyntax, GekSentenceSyntax,
        GoiRelativeClauseSyntax, Indicator, IntervalTenseSyntax, KePredicateTailSyntax,
        LinkArgumentSyntax, MathExpressionSyntax, MathOperatorSyntax, ParagraphStatementSyntax,
        ParagraphSyntax, PredicateStatementContinuationMarkerSyntax,
        PredicateStatementContinuationSyntax, PredicateSyntax, PredicateTail1Syntax,
        PredicateTail2Syntax, PredicateTail3Syntax, PredicateTailContinuationSyntax,
        PredicateTailSyntax, QuantifierSyntax, QuoteSyntax, RelationSyntax, RelationUnitSyntax,
        RelativeClauseSyntax, SelbriRelativeClauseSyntax, SimpleTenseModalSyntax, SpaceTenseSyntax,
        StatementSyntax, SubsentenceSyntax, TenseModalSyntax, TermSyntax, TermWrapperKindSyntax,
        TextSyntax, TimeTenseSyntax, WithFreeModifiers,
    },
};

const NODE_SIZE_LIMIT: usize = 1024;

#[test]
#[requires(true)]
#[ensures(true)]
fn ast_node_sizes_stay_within_stack_budget() {
    let mut sizes = vec![
        ("SourceSpan", size_of::<SourceSpan>()),
        ("Word", size_of::<Word>()),
        ("Jvopau", size_of::<Jvopau>()),
        ("Verbatim", size_of::<Verbatim>()),
        ("WordLike", size_of::<WordLike>()),
        (
            "WithIndicators<WordLike>",
            size_of::<WithIndicators<WordLike>>(),
        ),
        ("Token", size_of::<Token>()),
        (
            "WithFreeModifiers<Token>",
            size_of::<WithFreeModifiers<Token>>(),
        ),
        ("Indicator", size_of::<Indicator>()),
        ("PredicateSyntax", size_of::<PredicateSyntax>()),
        ("PredicateTailSyntax", size_of::<PredicateTailSyntax>()),
        ("KePredicateTailSyntax", size_of::<KePredicateTailSyntax>()),
        ("PredicateTail1Syntax", size_of::<PredicateTail1Syntax>()),
        (
            "PredicateTailContinuationSyntax",
            size_of::<PredicateTailContinuationSyntax>(),
        ),
        ("PredicateTail2Syntax", size_of::<PredicateTail2Syntax>()),
        ("BoPredicateTailSyntax", size_of::<BoPredicateTailSyntax>()),
        ("PredicateTail3Syntax", size_of::<PredicateTail3Syntax>()),
        ("GekSentenceSyntax", size_of::<GekSentenceSyntax>()),
        ("SubsentenceSyntax", size_of::<SubsentenceSyntax>()),
        ("TextSyntax", size_of::<TextSyntax>()),
        ("ParagraphSyntax", size_of::<ParagraphSyntax>()),
        (
            "ParagraphStatementSyntax",
            size_of::<ParagraphStatementSyntax>(),
        ),
        ("FreeModifierSyntax", size_of::<FreeModifierSyntax>()),
        ("StatementSyntax", size_of::<StatementSyntax>()),
        (
            "PredicateStatementContinuationSyntax",
            size_of::<PredicateStatementContinuationSyntax>(),
        ),
        (
            "PredicateStatementContinuationMarkerSyntax",
            size_of::<PredicateStatementContinuationMarkerSyntax>(),
        ),
        ("FragmentSyntax", size_of::<FragmentSyntax>()),
        ("TermSyntax", size_of::<TermSyntax>()),
        ("TermWrapperKindSyntax", size_of::<TermWrapperKindSyntax>()),
        ("ArgumentTagSyntax", size_of::<ArgumentTagSyntax>()),
        (
            "ArgumentConnectionSyntax",
            size_of::<ArgumentConnectionSyntax>(),
        ),
        ("ArgumentSyntax", size_of::<ArgumentSyntax>()),
        ("RelativeClauseSyntax", size_of::<RelativeClauseSyntax>()),
        (
            "GoiRelativeClauseSyntax",
            size_of::<GoiRelativeClauseSyntax>(),
        ),
        (
            "SelbriRelativeClauseSyntax",
            size_of::<SelbriRelativeClauseSyntax>(),
        ),
        ("QuoteSyntax", size_of::<QuoteSyntax>()),
        ("DescriptorSyntax", size_of::<DescriptorSyntax>()),
        ("DescriptorHeadSyntax", size_of::<DescriptorHeadSyntax>()),
        (
            "ConnectedDescriptorSyntax",
            size_of::<ConnectedDescriptorSyntax>(),
        ),
        ("ConnectiveSyntax", size_of::<ConnectiveSyntax>()),
        ("BeiLinkSyntax", size_of::<BeiLinkSyntax>()),
        ("LinkArgumentSyntax", size_of::<LinkArgumentSyntax>()),
        ("BeLinkSyntax", size_of::<BeLinkSyntax>()),
        ("ConnectiveKind", size_of::<ConnectiveKind>()),
        (
            "ArgumentTailElementSyntax",
            size_of::<ArgumentTailElementSyntax>(),
        ),
        ("QuantifierSyntax", size_of::<QuantifierSyntax>()),
        ("MathExpressionSyntax", size_of::<MathExpressionSyntax>()),
        ("MathOperatorSyntax", size_of::<MathOperatorSyntax>()),
        ("RelationSyntax", size_of::<RelationSyntax>()),
        ("TimeTenseSyntax", size_of::<TimeTenseSyntax>()),
        ("SpaceTenseSyntax", size_of::<SpaceTenseSyntax>()),
        ("IntervalTenseSyntax", size_of::<IntervalTenseSyntax>()),
        (
            "SimpleTenseModalSyntax",
            size_of::<SimpleTenseModalSyntax>(),
        ),
        ("FihoModalSyntax", size_of::<FihoModalSyntax>()),
        (
            "CompositeTenseModalPartSyntax",
            size_of::<CompositeTenseModalPartSyntax>(),
        ),
        ("TenseModalSyntax", size_of::<TenseModalSyntax>()),
        ("AbstractionSyntax", size_of::<AbstractionSyntax>()),
        ("AdditionalNuSyntax", size_of::<AdditionalNuSyntax>()),
        ("RelationUnitSyntax", size_of::<RelationUnitSyntax>()),
        ("CeiAssignmentSyntax", size_of::<CeiAssignmentSyntax>()),
    ];
    sizes.sort_by_key(|(_, size)| Reverse(*size));

    let report = sizes
        .iter()
        .map(|(name, size)| format!("{size:>5} {name}"))
        .collect::<Vec<_>>()
        .join("\n");
    println!("\nAST node sizes (bytes):\n{report}");

    if cfg!(target_pointer_width = "64") {
        let oversized = sizes
            .iter()
            .filter(|(_, size)| *size > NODE_SIZE_LIMIT)
            .map(|(name, size)| format!("{name}: {size}"))
            .collect::<Vec<_>>();
        assert!(
            oversized.is_empty(),
            "AST node sizes exceeded {NODE_SIZE_LIMIT} bytes: {}\n\n{report}",
            oversized.join(", ")
        );
    }
}

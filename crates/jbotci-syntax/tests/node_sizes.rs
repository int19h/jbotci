use std::{cmp::Reverse, mem::size_of};

#[allow(unused_imports)]
use bityzba::{ensures, requires};
use jbotci_morphology::{LujvoPart, Verbatim, Word, WordLike};
use jbotci_source::SourceSpan;
use jbotci_syntax::{
    Token, WithIndicators,
    ast::{
        AbstractionSyntax, AdHocModalSyntax, AdditionalLinkedSumtiSyntax,
        AfterthoughtBridiTailSyntax, BoundBridiTailConnectionSyntax, BoGroupedBridiTailSyntax,
        BridiStatementContinuationMarkerSyntax, BridiStatementContinuationSyntax, BridiSyntax,
        BridiTailConnectionSyntax, BridiTailSyntax, CompositeTenseModalPartSyntax,
        AbstractorConnectionSyntax, DescriptionConnectionSyntax, ConnectiveKind, ConnectiveSyntax,
        DescriptionHeadSyntax, DescriptionSyntax, DescriptionTailElementSyntax,
        ForethoughtBridiConnectionSyntax, FragmentSyntax, FreeModifierSyntax, Indicator,
        IntervalTenseSyntax, GroupedBridiTailConnectionSyntax, LinkedSumtiListSyntax, LinkedSumtiSyntax,
        MeksoOperatorSyntax, MeksoSyntax, ParagraphStatementSyntax, ParagraphSyntax,
        ProBridiAssignmentSyntax, QuantifierSyntax, QuoteSyntax, RelativeClauseSyntax,
        SelbriRelativePhraseSyntax, SelbriSyntax, SimpleBridiTailSyntax, SimpleTenseModalSyntax,
        SpaceTenseSyntax, StatementSyntax, SubbridiSyntax, SumtiAssociationPhraseSyntax,
        SumtiConnectionSyntax, SumtiSyntax, SumtiTagSyntax, SumtiWrapperKindSyntax,
        TanruUnitSyntax, TenseModalSyntax, TermSyntax, TextSyntax, TimeTenseSyntax,
        WithFreeModifiers,
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
        ("LujvoPart", size_of::<LujvoPart>()),
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
        ("BridiSyntax", size_of::<BridiSyntax>()),
        ("BridiTailSyntax", size_of::<BridiTailSyntax>()),
        (
            "GroupedBridiTailConnectionSyntax",
            size_of::<GroupedBridiTailConnectionSyntax>(),
        ),
        (
            "AfterthoughtBridiTailSyntax",
            size_of::<AfterthoughtBridiTailSyntax>(),
        ),
        (
            "BridiTailConnectionSyntax",
            size_of::<BridiTailConnectionSyntax>(),
        ),
        (
            "BoGroupedBridiTailSyntax",
            size_of::<BoGroupedBridiTailSyntax>(),
        ),
        (
            "BoundBridiTailConnectionSyntax",
            size_of::<BoundBridiTailConnectionSyntax>(),
        ),
        ("SimpleBridiTailSyntax", size_of::<SimpleBridiTailSyntax>()),
        (
            "ForethoughtBridiConnectionSyntax",
            size_of::<ForethoughtBridiConnectionSyntax>(),
        ),
        ("SubbridiSyntax", size_of::<SubbridiSyntax>()),
        ("TextSyntax", size_of::<TextSyntax>()),
        ("ParagraphSyntax", size_of::<ParagraphSyntax>()),
        (
            "ParagraphStatementSyntax",
            size_of::<ParagraphStatementSyntax>(),
        ),
        ("FreeModifierSyntax", size_of::<FreeModifierSyntax>()),
        ("StatementSyntax", size_of::<StatementSyntax>()),
        (
            "BridiStatementContinuationSyntax",
            size_of::<BridiStatementContinuationSyntax>(),
        ),
        (
            "BridiStatementContinuationMarkerSyntax",
            size_of::<BridiStatementContinuationMarkerSyntax>(),
        ),
        ("FragmentSyntax", size_of::<FragmentSyntax>()),
        ("TermSyntax", size_of::<TermSyntax>()),
        (
            "SumtiWrapperKindSyntax",
            size_of::<SumtiWrapperKindSyntax>(),
        ),
        ("SumtiTagSyntax", size_of::<SumtiTagSyntax>()),
        ("SumtiConnectionSyntax", size_of::<SumtiConnectionSyntax>()),
        ("SumtiSyntax", size_of::<SumtiSyntax>()),
        ("RelativeClauseSyntax", size_of::<RelativeClauseSyntax>()),
        (
            "SumtiAssociationPhraseSyntax",
            size_of::<SumtiAssociationPhraseSyntax>(),
        ),
        (
            "SelbriRelativePhraseSyntax",
            size_of::<SelbriRelativePhraseSyntax>(),
        ),
        ("QuoteSyntax", size_of::<QuoteSyntax>()),
        ("DescriptionSyntax", size_of::<DescriptionSyntax>()),
        ("DescriptionHeadSyntax", size_of::<DescriptionHeadSyntax>()),
        (
            "DescriptionConnectionSyntax",
            size_of::<DescriptionConnectionSyntax>(),
        ),
        ("ConnectiveSyntax", size_of::<ConnectiveSyntax>()),
        (
            "AdditionalLinkedSumtiSyntax",
            size_of::<AdditionalLinkedSumtiSyntax>(),
        ),
        ("LinkedSumtiSyntax", size_of::<LinkedSumtiSyntax>()),
        ("LinkedSumtiListSyntax", size_of::<LinkedSumtiListSyntax>()),
        ("ConnectiveKind", size_of::<ConnectiveKind>()),
        (
            "DescriptionTailElementSyntax",
            size_of::<DescriptionTailElementSyntax>(),
        ),
        ("QuantifierSyntax", size_of::<QuantifierSyntax>()),
        ("MeksoSyntax", size_of::<MeksoSyntax>()),
        ("MeksoOperatorSyntax", size_of::<MeksoOperatorSyntax>()),
        ("SelbriSyntax", size_of::<SelbriSyntax>()),
        ("TimeTenseSyntax", size_of::<TimeTenseSyntax>()),
        ("SpaceTenseSyntax", size_of::<SpaceTenseSyntax>()),
        ("IntervalTenseSyntax", size_of::<IntervalTenseSyntax>()),
        (
            "SimpleTenseModalSyntax",
            size_of::<SimpleTenseModalSyntax>(),
        ),
        ("AdHocModalSyntax", size_of::<AdHocModalSyntax>()),
        (
            "CompositeTenseModalPartSyntax",
            size_of::<CompositeTenseModalPartSyntax>(),
        ),
        ("TenseModalSyntax", size_of::<TenseModalSyntax>()),
        ("AbstractionSyntax", size_of::<AbstractionSyntax>()),
        (
            "AbstractorConnectionSyntax",
            size_of::<AbstractorConnectionSyntax>(),
        ),
        ("TanruUnitSyntax", size_of::<TanruUnitSyntax>()),
        (
            "ProBridiAssignmentSyntax",
            size_of::<ProBridiAssignmentSyntax>(),
        ),
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

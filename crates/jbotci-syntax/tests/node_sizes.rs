use std::{cmp::Reverse, mem::size_of};

#[allow(unused_imports)]
use bityzba::{ensures, requires};
use jbotci_morphology::{LujvoPart, Verbatim, Word, WordLike};
use jbotci_source::SourceSpan;
use jbotci_syntax::{
    Token, WithIndicators,
    generated_model::{
        AtomicMeksoOperatorSyntax, BoundOrSimpleMeksoOperandSyntax, BridiStatementSyntax,
        BridiSyntax, BridiTailSyntax, DescriptionHeadSyntax, FreeModifierSyntax,
        InitialParagraphStatementSyntax, InnerMeksoOperatorSyntax, LetterStringSyntax,
        MeksoOperandSyntax, MeksoOperatorSyntax, MeksoSyntax, NumberWordsSyntax, ParagraphSyntax,
        QuoteSyntax, SelbriSyntax, SimpleMeksoOperandSyntax, StatementSyntax, SumtiSyntax,
        TanruUnitSyntax, TenseModalSyntax, TermSyntax, TextSyntax, ZantufaForethoughtMeksoSyntax,
        ZantufaMex1Syntax, ZantufaMex2Syntax, ZantufaMexSyntax, ZantufaOperandSyntax,
        ZantufaOperatorSyntax, ZantufaPriorityMexSyntax,
    },
    tree::WithFreeModifiers,
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
            size_of::<WithFreeModifiers<Token, FreeModifierSyntax>>(),
        ),
        ("BridiSyntax", size_of::<BridiSyntax>()),
        ("BridiTailSyntax", size_of::<BridiTailSyntax>()),
        ("TextSyntax", size_of::<TextSyntax>()),
        ("ParagraphSyntax", size_of::<ParagraphSyntax>()),
        (
            "InitialParagraphStatementSyntax",
            size_of::<InitialParagraphStatementSyntax>(),
        ),
        ("FreeModifierSyntax", size_of::<FreeModifierSyntax>()),
        ("StatementSyntax", size_of::<StatementSyntax>()),
        ("BridiStatementSyntax", size_of::<BridiStatementSyntax>()),
        ("TermSyntax", size_of::<TermSyntax>()),
        ("SumtiSyntax", size_of::<SumtiSyntax>()),
        ("QuoteSyntax", size_of::<QuoteSyntax>()),
        ("DescriptionHeadSyntax", size_of::<DescriptionHeadSyntax>()),
        ("NumberWordsSyntax", size_of::<NumberWordsSyntax>()),
        ("LetterStringSyntax", size_of::<LetterStringSyntax>()),
        ("MeksoSyntax", size_of::<MeksoSyntax>()),
        ("MeksoOperandSyntax", size_of::<MeksoOperandSyntax>()),
        ("MeksoOperatorSyntax", size_of::<MeksoOperatorSyntax>()),
        (
            "BoundOrSimpleMeksoOperandSyntax",
            size_of::<BoundOrSimpleMeksoOperandSyntax>(),
        ),
        (
            "SimpleMeksoOperandSyntax",
            size_of::<SimpleMeksoOperandSyntax>(),
        ),
        (
            "InnerMeksoOperatorSyntax",
            size_of::<InnerMeksoOperatorSyntax>(),
        ),
        (
            "AtomicMeksoOperatorSyntax",
            size_of::<AtomicMeksoOperatorSyntax>(),
        ),
        ("ZantufaMexSyntax", size_of::<ZantufaMexSyntax>()),
        (
            "ZantufaPriorityMexSyntax",
            size_of::<ZantufaPriorityMexSyntax>(),
        ),
        ("ZantufaMex1Syntax", size_of::<ZantufaMex1Syntax>()),
        ("ZantufaMex2Syntax", size_of::<ZantufaMex2Syntax>()),
        ("ZantufaOperandSyntax", size_of::<ZantufaOperandSyntax>()),
        ("ZantufaOperatorSyntax", size_of::<ZantufaOperatorSyntax>()),
        (
            "ZantufaForethoughtMeksoSyntax",
            size_of::<ZantufaForethoughtMeksoSyntax>(),
        ),
        ("SelbriSyntax", size_of::<SelbriSyntax>()),
        ("TenseModalSyntax", size_of::<TenseModalSyntax>()),
        ("TanruUnitSyntax", size_of::<TanruUnitSyntax>()),
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

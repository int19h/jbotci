#[allow(dead_code)]
struct SyntaxGrammarEnv;
#[allow(dead_code)]
struct TextSyntax;
#[allow(dead_code)]
struct StatementSyntax;
#[allow(dead_code)]
struct LinkedSumtiListSyntax;
#[allow(dead_code)]
struct BoSumtiTail;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum Cmavo {
    Be,
    Bo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum Selmaho {
    Fa,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum SyntaxWordCategory {
    Quote,
}

jbotci_syntax_macros::syntax_grammar! {
    env SyntaxGrammarEnv;

    recursive {
        text: TextSyntax;
        statement: StatementSyntax;
    }

    alias passthrough_statement(statement) -> StatementSyntax {
        context "statement";
        require cmavo(Bo).not();
        statement;
    }

    node linkargs(sumti) -> LinkedSumtiListSyntax {
        context "linked arguments";
        fields {
            field be = cmavo(Be).wf();
            require cmavo(Bo).not();
            field fa = selmaho(Fa).wf();
            field first_sumti = opt(boxed(sumti));
            field recovered_sumti = recover_as(association_argument, boxed(sumti));
            when feature(ZantufaTags) field tagged = boxed(sumti);
            when policy(ZantufaQuotes) require word_category(Quote).not();
            when policy(ZantufaQuotes) let scratch = fold_chain(head, tail);
            scratch parsed_guard = cmavo(Bo).ignored();
            default trailing_sumti = Vec::new();
        }
    }

    product bo_sumti_tail -> BoSumtiTail {
        construct direct;
        fields {
            field connective = choice(joik(), jek());
            field bo = cmavo(Bo).wf();
            field maybe_bo = some(cmavo(Bo));
        }
    }
}

#[test]
fn grammar_macro_exports_declaration_metadata() {
    assert_eq!(SYNTAX_GRAMMAR_ENV, "SyntaxGrammarEnv");
    assert_eq!(SYNTAX_GRAMMAR_RECURSIVE_RULES.len(), 2);
    assert_eq!(SYNTAX_GRAMMAR_RECURSIVE_RULES[0].name, "text");
    assert_eq!(SYNTAX_GRAMMAR_RECURSIVE_RULES[1].output, "StatementSyntax");

    assert_eq!(SYNTAX_GRAMMAR_RULES.len(), 3);
    assert_eq!(SYNTAX_GRAMMAR_RULES[0].kind, "alias");
    assert_eq!(SYNTAX_GRAMMAR_RULES[0].name, "passthrough_statement");
    assert_eq!(SYNTAX_GRAMMAR_RULES[0].arguments, &["statement"]);
    assert_eq!(SYNTAX_GRAMMAR_RULES[0].context, Some("statement"));
    assert_eq!(SYNTAX_GRAMMAR_RULES[0].fields[0].kind, "require");
    assert_eq!(
        SYNTAX_GRAMMAR_RULES[0].fields[0].recovery,
        SyntaxGrammarRecoveryExpr::Not(&SyntaxGrammarRecoveryExpr::Cmavo(Cmavo::Bo))
    );
    assert_eq!(SYNTAX_GRAMMAR_RULES[0].fields[1].kind, "alias");
    assert_eq!(
        SYNTAX_GRAMMAR_RULES[0].fields[1].recovery,
        SyntaxGrammarRecoveryExpr::Rule("statement")
    );

    assert_eq!(SYNTAX_GRAMMAR_RULES[1].kind, "node");
    assert_eq!(SYNTAX_GRAMMAR_RULES[1].name, "linkargs");
    assert_eq!(SYNTAX_GRAMMAR_RULES[1].arguments, &["sumti"]);
    assert_eq!(SYNTAX_GRAMMAR_RULES[1].context, Some("linked arguments"));
    assert_eq!(SYNTAX_GRAMMAR_RULES[1].fields[0].name, "be");
    assert_eq!(SYNTAX_GRAMMAR_RULES[1].fields[0].parser, "cmavo(Be).wf()");
    assert_eq!(
        SYNTAX_GRAMMAR_RULES[1].fields[0].recovery,
        SyntaxGrammarRecoveryExpr::WithFreeModifiers(&SyntaxGrammarRecoveryExpr::Cmavo(Cmavo::Be))
    );
    assert_eq!(
        SYNTAX_GRAMMAR_RULES[1].fields[1].recovery,
        SyntaxGrammarRecoveryExpr::Not(&SyntaxGrammarRecoveryExpr::Cmavo(Cmavo::Bo))
    );
    assert_eq!(SYNTAX_GRAMMAR_RULES[1].fields[1].kind, "require");
    assert_eq!(SYNTAX_GRAMMAR_RULES[1].fields[1].name, "");
    assert_eq!(
        SYNTAX_GRAMMAR_RULES[1].fields[2].recovery,
        SyntaxGrammarRecoveryExpr::WithFreeModifiers(&SyntaxGrammarRecoveryExpr::Selmaho(
            Selmaho::Fa
        ))
    );
    assert_eq!(
        SYNTAX_GRAMMAR_RULES[1].fields[3].recovery,
        SyntaxGrammarRecoveryExpr::Opt(&SyntaxGrammarRecoveryExpr::Boxed(
            &SyntaxGrammarRecoveryExpr::Rule("sumti")
        ))
    );
    assert_eq!(
        SYNTAX_GRAMMAR_RULES[1].fields[4].recovery,
        SyntaxGrammarRecoveryExpr::Rule("association_argument")
    );
    assert_eq!(SYNTAX_GRAMMAR_RULES[1].fields[5].conditions.len(), 1);
    assert_eq!(
        SYNTAX_GRAMMAR_RULES[1].fields[5].conditions[0],
        SyntaxGrammarCondition {
            kind: SyntaxGrammarConditionKind::Feature,
            name: "ZantufaTags",
        }
    );
    assert_eq!(SYNTAX_GRAMMAR_RULES[1].fields[6].kind, "require");
    assert_eq!(
        SYNTAX_GRAMMAR_RULES[1].fields[6].conditions[0],
        SyntaxGrammarCondition {
            kind: SyntaxGrammarConditionKind::Policy,
            name: "ZantufaQuotes",
        }
    );
    assert_eq!(SYNTAX_GRAMMAR_RULES[1].fields[7].kind, "let");
    assert_eq!(
        SYNTAX_GRAMMAR_RULES[1].fields[7].conditions[0],
        SyntaxGrammarCondition {
            kind: SyntaxGrammarConditionKind::Policy,
            name: "ZantufaQuotes",
        }
    );
    assert_eq!(SYNTAX_GRAMMAR_RULES[1].fields[8].kind, "scratch");
    assert_eq!(SYNTAX_GRAMMAR_RULES[1].fields[8].name, "parsed_guard");
    assert_eq!(
        SYNTAX_GRAMMAR_RULES[1].fields[8].recovery,
        SyntaxGrammarRecoveryExpr::Ignored(&SyntaxGrammarRecoveryExpr::Cmavo(Cmavo::Bo))
    );
    assert_eq!(SYNTAX_GRAMMAR_RULES[1].fields[9].kind, "default");
    assert_eq!(SYNTAX_GRAMMAR_RULES[1].fields[9].name, "trailing_sumti");
    assert_eq!(SYNTAX_GRAMMAR_RULES[1].fields[9].parser, "Vec::new()");
    assert_eq!(
        SYNTAX_GRAMMAR_RULES[1].fields[7].recovery,
        SyntaxGrammarRecoveryExpr::Opaque("fold_chain(head,tail)")
    );

    assert_eq!(SYNTAX_GRAMMAR_RULES[2].kind, "product");
    assert_eq!(SYNTAX_GRAMMAR_RULES[2].fields[1].name, "bo");
    assert_eq!(
        SYNTAX_GRAMMAR_RULES[2].fields[1].recovery,
        SyntaxGrammarRecoveryExpr::WithFreeModifiers(&SyntaxGrammarRecoveryExpr::Cmavo(Cmavo::Bo))
    );
    assert_eq!(SYNTAX_GRAMMAR_RULES[2].fields[2].name, "maybe_bo");
    assert_eq!(
        SYNTAX_GRAMMAR_RULES[2].fields[2].recovery,
        SyntaxGrammarRecoveryExpr::Some(&SyntaxGrammarRecoveryExpr::Cmavo(Cmavo::Bo))
    );
}

#[test]
fn grammar_macro_exports_rule_lookup() {
    let rule = syntax_grammar_rule_by_name("linkargs").expect("linkargs rule exists");
    assert_eq!(rule.output, "LinkedSumtiListSyntax");
    assert!(syntax_grammar_rule_by_name("missing").is_none());
}

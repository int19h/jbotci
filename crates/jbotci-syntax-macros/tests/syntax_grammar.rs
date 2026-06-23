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

    node linkargs(sumti) -> LinkedSumtiListSyntax {
        context "linked arguments";
        fields {
            field be = cmavo(Be).wf();
            field fa = selmaho(Fa).wf();
            field first_sumti = opt(boxed(sumti));
            field recovered_sumti = recover_as(association_argument, boxed(sumti));
            when feature(ZantufaTags) field tagged = boxed(sumti);
            when policy(ZantufaQuotes) let scratch = fold_chain(head, tail);
        }
    }

    product bo_sumti_tail -> BoSumtiTail {
        fields {
            field connective = choice(joik(), jek());
            field bo = cmavo(Bo).wf();
        }
    }
}

#[test]
fn grammar_macro_exports_declaration_metadata() {
    assert_eq!(SYNTAX_GRAMMAR_ENV, "SyntaxGrammarEnv");
    assert_eq!(SYNTAX_GRAMMAR_RECURSIVE_RULES.len(), 2);
    assert_eq!(SYNTAX_GRAMMAR_RECURSIVE_RULES[0].name, "text");
    assert_eq!(SYNTAX_GRAMMAR_RECURSIVE_RULES[1].output, "StatementSyntax");

    assert_eq!(SYNTAX_GRAMMAR_RULES.len(), 2);
    assert_eq!(SYNTAX_GRAMMAR_RULES[0].kind, "node");
    assert_eq!(SYNTAX_GRAMMAR_RULES[0].name, "linkargs");
    assert_eq!(SYNTAX_GRAMMAR_RULES[0].arguments, &["sumti"]);
    assert_eq!(SYNTAX_GRAMMAR_RULES[0].context, Some("linked arguments"));
    assert_eq!(SYNTAX_GRAMMAR_RULES[0].fields[0].name, "be");
    assert_eq!(SYNTAX_GRAMMAR_RULES[0].fields[0].parser, "cmavo(Be).wf()");
    assert_eq!(
        SYNTAX_GRAMMAR_RULES[0].fields[0].recovery,
        SyntaxGrammarRecoveryExpr::WithFreeModifiers(&SyntaxGrammarRecoveryExpr::Cmavo(Cmavo::Be))
    );
    assert_eq!(
        SYNTAX_GRAMMAR_RULES[0].fields[1].recovery,
        SyntaxGrammarRecoveryExpr::WithFreeModifiers(&SyntaxGrammarRecoveryExpr::Selmaho(
            Selmaho::Fa
        ))
    );
    assert_eq!(
        SYNTAX_GRAMMAR_RULES[0].fields[2].recovery,
        SyntaxGrammarRecoveryExpr::Opt(&SyntaxGrammarRecoveryExpr::Boxed(
            &SyntaxGrammarRecoveryExpr::Rule("sumti")
        ))
    );
    assert_eq!(
        SYNTAX_GRAMMAR_RULES[0].fields[3].recovery,
        SyntaxGrammarRecoveryExpr::Rule("association_argument")
    );
    assert_eq!(SYNTAX_GRAMMAR_RULES[0].fields[4].conditions.len(), 1);
    assert_eq!(
        SYNTAX_GRAMMAR_RULES[0].fields[4].conditions[0],
        SyntaxGrammarCondition {
            kind: SyntaxGrammarConditionKind::Feature,
            name: "ZantufaTags",
        }
    );
    assert_eq!(SYNTAX_GRAMMAR_RULES[0].fields[5].kind, "let");
    assert_eq!(
        SYNTAX_GRAMMAR_RULES[0].fields[5].conditions[0],
        SyntaxGrammarCondition {
            kind: SyntaxGrammarConditionKind::Policy,
            name: "ZantufaQuotes",
        }
    );
    assert_eq!(
        SYNTAX_GRAMMAR_RULES[0].fields[5].recovery,
        SyntaxGrammarRecoveryExpr::Opaque("fold_chain(head,tail)")
    );

    assert_eq!(SYNTAX_GRAMMAR_RULES[1].kind, "product");
    assert_eq!(SYNTAX_GRAMMAR_RULES[1].fields[1].name, "bo");
    assert_eq!(
        SYNTAX_GRAMMAR_RULES[1].fields[1].recovery,
        SyntaxGrammarRecoveryExpr::WithFreeModifiers(&SyntaxGrammarRecoveryExpr::Cmavo(Cmavo::Bo))
    );
}

#[test]
fn grammar_macro_exports_rule_lookup() {
    let rule = syntax_grammar_rule_by_name("linkargs").expect("linkargs rule exists");
    assert_eq!(rule.output, "LinkedSumtiListSyntax");
    assert!(syntax_grammar_rule_by_name("missing").is_none());
}

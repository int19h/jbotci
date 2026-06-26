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

mod generated_model {
    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
    pub struct Token;

    jbotci_syntax_macros::syntax_grammar! {
        tree_model {
            #[bityzba::invariant(true)]
            #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
            pub struct ItemSyntax {
                pub token: Token,
            }
        }
        model;

        recursive {
            item: ItemSyntax;
        }

        alias item_alias(item) -> ItemSyntax {
            item;
        }

        node pair(item) -> PairSyntax {
            fields {
                field head = cmavo(Be);
                field nonempty = vec1(cmavo(Be));
                require cmavo(Bo).not();
                scratch parser_only = cmavo(Bo).ignored();
                default tail: Vec<Token> = Vec::new();
                let computed: usize = 0usize;
                #[tree_child(primary)]
                field child = boxed(item);
            }
        }

        node first_choice -> ChoiceSyntax {
            construct variant First;
            fields {
                field token = cmavo(Be);
            }
        }

        node second_choice(item) -> ChoiceSyntax {
            construct variant Second;
            fields {
                field item = boxed(item);
            }
        }

        node renamed_choice -> RenamedChoiceSyntax {
            construct variant RuntimeName;
            model_variant ModelName;
            fields {
                field token = cmavo(Be);
            }
        }

        product helper_product -> HelperSyntax {
            fields {
                field token = cmavo(Be);
            }
        }
    }

    #[test]
    fn grammar_macro_emits_model_items_from_type_bearing_rules() {
        let item = ItemSyntax { token: Token };
        let pair = PairSyntax {
            head: Token,
            nonempty: vec1::Vec1::new(Token),
            tail: Vec::new(),
            computed: 0,
            child: Box::new(item.clone()),
        };
        let first = ChoiceSyntax::First { token: Token };
        let second = ChoiceSyntax::Second {
            item: Box::new(item),
        };
        let renamed = RenamedChoiceSyntax::ModelName { token: Token };
        let helper = HelperSyntax { token: Token };

        assert!(matches!(first, ChoiceSyntax::First { .. }));
        assert!(matches!(second, ChoiceSyntax::Second { .. }));
        assert!(matches!(renamed, RenamedChoiceSyntax::ModelName { .. }));
        assert_eq!(pair.tail.len(), 0);
        assert_eq!(helper.token, Token);
    }
}

mod generated_model_filter {
    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
    pub struct Token;

    jbotci_syntax_macros::syntax_grammar! {
        tree_model {}
        model { KeptSyntax };

        node kept -> KeptSyntax {
            fields {
                field token = cmavo(Be);
            }
        }

        node skipped_first -> SkippedSyntax {
            fields {
                field token = cmavo(Be);
            }
        }

        node skipped_second -> SkippedSyntax {
            fields {
                field token = cmavo(Bo);
            }
        }
    }

    #[test]
    fn grammar_macro_filters_generated_model_outputs() {
        let kept = KeptSyntax { token: Token };
        assert_eq!(kept.token, Token);
    }
}

mod generated_model_with_env {
    use crate::{Cmavo, Selmaho};

    #[allow(dead_code)]
    struct SyntaxGrammarEnv;

    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
    pub struct Token;

    jbotci_syntax_macros::syntax_grammar! {
        tree_model {}
        model { EnvNodeSyntax };
        env SyntaxGrammarEnv;

        node env_node -> EnvNodeSyntax {
            fields {
                field token = cmavo(Be);
            }
        }
    }

    #[test]
    fn grammar_macro_emits_model_items_when_env_is_present() {
        let node = EnvNodeSyntax { token: Token };
        assert_eq!(node.token, Token);
        assert_eq!(SYNTAX_GRAMMAR_ENV, "SyntaxGrammarEnv");
    }
}

mod new_dsl {
    use crate::{Cmavo, Selmaho};

    #[allow(dead_code)]
    struct SyntaxGrammarEnv;

    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
    pub struct Token;

    jbotci_syntax_macros::syntax_grammar! {
        tree_model {}
        model;
        env SyntaxGrammarEnv;

        recursive {
            item: ItemSyntax;
        }

        rule "item" item -> struct {
            field token <- cmavo(Be);
            field computed: usize = 1usize;
            let temp = 2usize;
            assert !cmavo(Bo);
        }

        rule "other item" other_item -> struct {
            field token <- cmavo(Bo);
        }

        rule "item choice" item_choice -> enum {
            item,
            other_item,
        }

        alias "item alias" item_alias = item;
    }

    #[test]
    fn grammar_macro_accepts_explicit_struct_enum_and_alias_rules() {
        let item = ItemSyntax {
            token: Token,
            computed: 1,
        };
        let other_item = OtherItemSyntax { token: Token };
        let item_choice = ItemChoiceSyntax::Item { item: item.clone() };
        let other_choice = ItemChoiceSyntax::OtherItem { other_item };

        assert_eq!(item.token, Token);
        assert_eq!(item.computed, 1);
        assert!(matches!(item_choice, ItemChoiceSyntax::Item { .. }));
        assert!(matches!(other_choice, ItemChoiceSyntax::OtherItem { .. }));
    }

    #[test]
    fn grammar_macro_exports_new_dsl_metadata() {
        assert_eq!(SYNTAX_GRAMMAR_RULES.len(), 4);
        assert_eq!(SYNTAX_GRAMMAR_RULES[0].kind, "struct");
        assert_eq!(SYNTAX_GRAMMAR_RULES[0].name, "item");
        assert_eq!(SYNTAX_GRAMMAR_RULES[0].output, "ItemSyntax");
        assert_eq!(SYNTAX_GRAMMAR_RULES[0].context, Some("item"));
        assert_eq!(SYNTAX_GRAMMAR_RULES[0].fields[0].kind, "field");
        assert_eq!(SYNTAX_GRAMMAR_RULES[0].fields[1].kind, "field");
        assert_eq!(SYNTAX_GRAMMAR_RULES[0].fields[2].kind, "let");
        assert_eq!(SYNTAX_GRAMMAR_RULES[0].fields[3].kind, "require");
        assert_eq!(
            SYNTAX_GRAMMAR_RULES[0].fields[3].recovery,
            SyntaxGrammarRecoveryExpr::Not(&SyntaxGrammarRecoveryExpr::Cmavo(Cmavo::Bo))
        );

        assert_eq!(SYNTAX_GRAMMAR_RULES[2].kind, "enum");
        assert_eq!(SYNTAX_GRAMMAR_RULES[2].output, "ItemChoiceSyntax");
        assert_eq!(SYNTAX_GRAMMAR_RULES[2].fields[0].kind, "variant");
        assert_eq!(SYNTAX_GRAMMAR_RULES[2].fields[0].name, "item");

        assert_eq!(SYNTAX_GRAMMAR_RULES[3].kind, "alias");
        assert_eq!(SYNTAX_GRAMMAR_RULES[3].output, "ItemSyntax");
        assert_eq!(SYNTAX_GRAMMAR_RULES[3].context, Some("item alias"));
    }
}

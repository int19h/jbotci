#[bityzba::invariant(true)]
#[allow(dead_code)]
struct SyntaxGrammarEnv;
#[bityzba::invariant(true)]
#[allow(dead_code)]
struct TextSyntax;
#[bityzba::invariant(true)]
#[allow(dead_code)]
struct StatementSyntax;
#[bityzba::invariant(true)]
#[allow(dead_code)]
struct LinkedSumtiListSyntax;
#[bityzba::invariant(true)]
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
    Pa,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum SyntaxWordCategory {
    Cmevla,
    Quote,
    SelbriWord,
}

jbotci_syntax_macros::syntax_grammar! {
    env SyntaxGrammarEnv;

    recursive {
        text: TextSyntax;
        statement: StatementSyntax;
    }

    alias "statement" passthrough_statement(statement) =
        cmavo(Bo).not().ignore_then(statement);

    rule "linked arguments" linkargs(sumti) -> struct {
        field be <- cmavo(Be).wf();
        assert !cmavo(Bo);
        field fa <- selmaho(Fa).wf();
        field first_sumti <- opt(boxed(sumti));
        when feature(ZantufaTags) field tagged <- boxed(sumti);
        assert feature(ZantufaTags);
        assert !policy(ZantufaQuotes);
        when policy(ZantufaQuotes) assert !word_category(Quote);
        when policy(ZantufaQuotes) let folded = fold_chain(head, tail);
        field computed: usize = 0usize;
    }

    rule "bo sumti tail" bo_sumti_tail -> struct {
        field connective <- choice(joik(), jek());
        field bo <- cmavo(Bo).wf();
        field maybe_bo <- opt(cmavo(Bo));
    }
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
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
    assert_eq!(SYNTAX_GRAMMAR_RULES[0].fields[0].kind, "alias");
    assert_eq!(
        SYNTAX_GRAMMAR_RULES[0].fields[0].recovery,
        SyntaxGrammarRecoveryExpr::Sequence(&[
            SyntaxGrammarRecoveryExpr::Not(&SyntaxGrammarRecoveryExpr::Cmavo(Cmavo::Bo)),
            SyntaxGrammarRecoveryExpr::Rule("statement"),
        ])
    );

    assert_eq!(SYNTAX_GRAMMAR_RULES[1].kind, "struct");
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
    assert_eq!(SYNTAX_GRAMMAR_RULES[1].fields[4].conditions.len(), 1);
    assert_eq!(
        SYNTAX_GRAMMAR_RULES[1].fields[4].conditions[0],
        SyntaxGrammarCondition {
            kind: SyntaxGrammarConditionKind::Feature,
            name: "ZantufaTags",
        }
    );
    assert_eq!(SYNTAX_GRAMMAR_RULES[1].fields[5].kind, "require");
    assert_eq!(
        SYNTAX_GRAMMAR_RULES[1].fields[5].recovery,
        SyntaxGrammarRecoveryExpr::Ignored(&SyntaxGrammarRecoveryExpr::Lookahead(
            &SyntaxGrammarRecoveryExpr::Opaque("feature(ZantufaTags)")
        ))
    );
    assert_eq!(SYNTAX_GRAMMAR_RULES[1].fields[6].kind, "require");
    assert_eq!(
        SYNTAX_GRAMMAR_RULES[1].fields[6].recovery,
        SyntaxGrammarRecoveryExpr::Not(&SyntaxGrammarRecoveryExpr::Opaque("policy(ZantufaQuotes)"))
    );
    assert_eq!(SYNTAX_GRAMMAR_RULES[1].fields[7].kind, "require");
    assert_eq!(
        SYNTAX_GRAMMAR_RULES[1].fields[7].conditions[0],
        SyntaxGrammarCondition {
            kind: SyntaxGrammarConditionKind::Policy,
            name: "ZantufaQuotes",
        }
    );
    assert_eq!(SYNTAX_GRAMMAR_RULES[1].fields[8].kind, "let");
    assert_eq!(
        SYNTAX_GRAMMAR_RULES[1].fields[8].conditions[0],
        SyntaxGrammarCondition {
            kind: SyntaxGrammarConditionKind::Policy,
            name: "ZantufaQuotes",
        }
    );
    assert_eq!(SYNTAX_GRAMMAR_RULES[1].fields[9].kind, "field");
    assert_eq!(SYNTAX_GRAMMAR_RULES[1].fields[9].name, "computed");
    assert_eq!(
        SYNTAX_GRAMMAR_RULES[1].fields[9].recovery,
        SyntaxGrammarRecoveryExpr::Opaque("0usize")
    );
    assert_eq!(
        SYNTAX_GRAMMAR_RULES[1].fields[8].recovery,
        SyntaxGrammarRecoveryExpr::Opaque("fold_chain(head,tail)")
    );

    assert_eq!(SYNTAX_GRAMMAR_RULES[2].kind, "struct");
    assert_eq!(
        SYNTAX_GRAMMAR_RULES[2].fields[0].recovery,
        SyntaxGrammarRecoveryExpr::Choice(&[
            SyntaxGrammarRecoveryExpr::Opaque("joik()"),
            SyntaxGrammarRecoveryExpr::Opaque("jek()"),
        ])
    );
    assert_eq!(SYNTAX_GRAMMAR_RULES[2].fields[1].name, "bo");
    assert_eq!(
        SYNTAX_GRAMMAR_RULES[2].fields[1].recovery,
        SyntaxGrammarRecoveryExpr::WithFreeModifiers(&SyntaxGrammarRecoveryExpr::Cmavo(Cmavo::Bo))
    );
    assert_eq!(SYNTAX_GRAMMAR_RULES[2].fields[2].name, "maybe_bo");
    assert_eq!(
        SYNTAX_GRAMMAR_RULES[2].fields[2].recovery,
        SyntaxGrammarRecoveryExpr::Opt(&SyntaxGrammarRecoveryExpr::Cmavo(Cmavo::Bo))
    );
}

mod recovery_classification {
    use crate::{Cmavo, Selmaho, SyntaxWordCategory};

    #[bityzba::invariant(true)]
    #[allow(dead_code)]
    struct SyntaxGrammarEnv;
    #[bityzba::invariant(true)]
    #[allow(dead_code)]
    struct Token;

    jbotci_syntax_macros::syntax_grammar! {
        env SyntaxGrammarEnv;

        rule "token helpers" token_helpers -> struct {
            field pa <- pa_word();
            field cmevla <- cmevla_word();
            field leading_cmevla <- text_leading_cmevla_word();
            field relation <- relation_word();
            field external <- external_helper();
        }
    }

    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    #[test]
    fn grammar_macro_classifies_builtin_and_external_recovery_calls() {
        assert_eq!(
            SYNTAX_GRAMMAR_RULES[0].fields[0].recovery,
            SyntaxGrammarRecoveryExpr::Selmaho(Selmaho::Pa)
        );
        assert_eq!(
            SYNTAX_GRAMMAR_RULES[0].fields[1].recovery,
            SyntaxGrammarRecoveryExpr::WordCategory(SyntaxWordCategory::Cmevla)
        );
        assert_eq!(
            SYNTAX_GRAMMAR_RULES[0].fields[2].recovery,
            SyntaxGrammarRecoveryExpr::WordCategory(SyntaxWordCategory::Cmevla)
        );
        assert_eq!(
            SYNTAX_GRAMMAR_RULES[0].fields[3].recovery,
            SyntaxGrammarRecoveryExpr::RelationWord
        );
        assert_eq!(
            SYNTAX_GRAMMAR_RULES[0].fields[4].recovery,
            SyntaxGrammarRecoveryExpr::Opaque("external_helper()")
        );
    }
}

mod anchor_metadata {
    use crate::{Cmavo, Selmaho};

    #[bityzba::invariant(true)]
    #[allow(dead_code)]
    struct SyntaxGrammarEnv;
    #[bityzba::invariant(true)]
    #[allow(dead_code)]
    struct TextSyntax;
    #[bityzba::invariant(true)]
    #[allow(dead_code)]
    struct ItemSyntax;

    jbotci_syntax_macros::syntax_grammar! {
        env SyntaxGrammarEnv;

        recursive {
            text: TextSyntax;
            item: ItemSyntax;
        }

        rule "item" item(item) -> enum {
            literal_item,
            recursive_item,
            when feature(ZantufaTags) gated_item,
            nullable_item,
            explicit_argument_item,
        }

        rule "literal item" literal_item(item) -> struct {
            field be <- cmavo(Be).wf();
            field bo <- cmavo(Bo).wf();
            field maybe_fa <- opt(selmaho(Fa).wf());
            field tail <- opt(arc(item));
        }

        rule "recursive item" recursive_item(item) -> struct {
            field inner <- opt(arc(item));
            field pa <- selmaho(Pa).wf();
        }

        rule "gated item" gated_item -> struct {
            field fa <- selmaho(Fa).warn(ExperimentalAnchorMetadata).wf();
            when feature(ZantufaTags) field bo <- cmavo(Bo).wf();
        }

        rule "nullable item" nullable_item -> struct {
            field maybe_bo <- opt(cmavo(Bo));
        }

        rule "explicit argument item" explicit_argument_item(item) -> struct {
            field inner <- literal_item(item);
        }

        rule "text quote" text_quote(text) -> struct {
            field be <- cmavo(Be).wf();
            field text <- arc(text);
            field bo <- opt(cmavo(Bo).wf()).elidable_terminator(Bo);
        }
    }

    #[bityzba::requires(!rule.is_empty())]
    #[bityzba::ensures(ret.rule == rule)]
    fn anchors_for(rule: &str) -> &'static SyntaxGrammarRuleAnchorMetadata {
        syntax_grammar_anchor_metadata_by_rule_name(rule).expect("anchor metadata exists")
    }

    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    fn token_set_contains(
        tokens: &[SyntaxGrammarAnchorToken],
        token: SyntaxGrammarAnchorToken,
    ) -> bool {
        tokens.contains(&token)
    }

    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    #[test]
    fn grammar_macro_derives_anchor_metadata() {
        assert_eq!(
            SYNTAX_GRAMMAR_RECOVERY_ANCHORS.len(),
            SYNTAX_GRAMMAR_RULES.len()
        );

        let literal = anchors_for("literal_item");
        assert_eq!(literal.fields[0].anchors[0].resume_field, 0);
        assert!(token_set_contains(
            literal.fields[0].anchors[0].start_tokens,
            SyntaxGrammarAnchorToken::Cmavo(Cmavo::Be),
        ));
        assert!(
            !token_set_contains(
                literal.fields[0].anchors[0].start_tokens,
                SyntaxGrammarAnchorToken::Cmavo(Cmavo::Bo),
            ),
            "adjacent literal runs match only the run start token",
        );
        assert_eq!(literal.fields[1].anchors[0].resume_field, 1);
        assert!(token_set_contains(
            literal.fields[1].anchors[0].start_tokens,
            SyntaxGrammarAnchorToken::Cmavo(Cmavo::Bo),
        ));
        assert_eq!(literal.fields[2].anchors[0].resume_field, 2);
        assert!(token_set_contains(
            literal.fields[2].anchors[0].start_tokens,
            SyntaxGrammarAnchorToken::Selmaho(Selmaho::Fa),
        ));

        let item = anchors_for("item");
        assert!(item.fields.is_empty(), "enum rules carry no field anchors");
        assert!(item.first.iter().any(|entry| token_set_contains(
            entry.tokens,
            SyntaxGrammarAnchorToken::Cmavo(Cmavo::Be)
        )));
        assert!(item.first.iter().any(|entry| token_set_contains(
            entry.tokens,
            SyntaxGrammarAnchorToken::Selmaho(Selmaho::Pa)
        )));
        let explicit = anchors_for("explicit_argument_item");
        assert!(explicit.fields[0].anchors.iter().any(|anchor| {
            anchor.resume_field == 0
                && token_set_contains(
                    anchor.start_tokens,
                    SyntaxGrammarAnchorToken::Cmavo(Cmavo::Be),
                )
        }));
        let gated_first = item
            .first
            .iter()
            .find(|entry| {
                token_set_contains(entry.tokens, SyntaxGrammarAnchorToken::Selmaho(Selmaho::Fa))
            })
            .expect("gated first token");
        assert_eq!(
            gated_first.conditions,
            &[SyntaxGrammarCondition {
                kind: SyntaxGrammarConditionKind::Feature,
                name: "ZantufaTags",
            }]
        );
        let gated = anchors_for("gated_item");
        let gated_field_anchor = gated.fields[1]
            .anchors
            .iter()
            .find(|anchor| {
                token_set_contains(
                    anchor.start_tokens,
                    SyntaxGrammarAnchorToken::Cmavo(Cmavo::Bo),
                )
            })
            .expect("gated field anchor");
        assert_eq!(
            gated_field_anchor.conditions,
            &[SyntaxGrammarCondition {
                kind: SyntaxGrammarConditionKind::Feature,
                name: "ZantufaTags",
            }]
        );

        let tail_anchors = &literal.fields[3].anchors;
        assert!(tail_anchors.iter().any(|anchor| {
            anchor.resume_field == 3
                && token_set_contains(
                    anchor.start_tokens,
                    SyntaxGrammarAnchorToken::Cmavo(Cmavo::Be),
                )
        }));
    }

    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    #[test]
    fn grammar_macro_derives_subtext_containers() {
        assert_eq!(SYNTAX_GRAMMAR_SUBTEXT_CONTAINERS.len(), 1);
        let container = &SYNTAX_GRAMMAR_SUBTEXT_CONTAINERS[0];
        assert_eq!(container.rule, "text_quote");
        assert_eq!(container.opener_field, 0);
        assert_eq!(container.text_field, 1);
        assert_eq!(container.closer_field, 2);
        assert!(token_set_contains(
            container.opener_tokens,
            SyntaxGrammarAnchorToken::Cmavo(Cmavo::Be),
        ));
        assert!(token_set_contains(
            container.closer_tokens,
            SyntaxGrammarAnchorToken::Cmavo(Cmavo::Bo),
        ));
    }
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
#[test]
fn grammar_macro_exports_rule_lookup() {
    let rule = syntax_grammar_rule_by_name("linkargs").expect("linkargs rule exists");
    assert_eq!(rule.output, "LinkargsSyntax");
    assert!(syntax_grammar_rule_by_name("missing").is_none());
}

mod generated_model {
    use crate::Cmavo;

    #[bityzba::invariant(true)]
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

        alias "item" item_alias = item;

        rule "pair" pair(item) -> struct {
            field head <- cmavo(Be);
            field nonempty <- [one_or_more cmavo(Be)];
            assert !cmavo(Bo);
            field computed: usize = 0usize;
            let temporary = 1usize;
            #[tree_child(primary)]
            field child <- boxed(item);
        }

        rule "choice" choice -> enum {
            choice_first,
            choice_second,
        }

        rule "choice first" choice_first -> struct {
            field token <- cmavo(Be);
        }

        rule "choice second" choice_second(item) -> struct {
            field item <- boxed(item);
        }

        rule "helper product" helper_product -> struct {
            field token <- cmavo(Be);
        }
    }

    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    #[test]
    fn grammar_macro_emits_model_items_from_type_bearing_rules() {
        let item = ItemSyntax { token: Token };
        let pair = PairSyntax {
            head: Token,
            nonempty: vec1::Vec1::new(Token),
            computed: 0,
            child: Box::new(item.clone()),
        };
        let first = ChoiceSyntax::ChoiceFirst(ChoiceFirstSyntax(Token));
        let second = ChoiceSyntax::ChoiceSecond(ChoiceSecondSyntax(Box::new(item)));
        let helper = HelperProductSyntax(Token);

        assert!(matches!(first, ChoiceSyntax::ChoiceFirst(_)));
        assert!(matches!(second, ChoiceSyntax::ChoiceSecond(_)));
        assert_eq!(helper.0, Token);
        assert_eq!(pair.computed, 0);
    }
}

mod generated_model_filter {
    use crate::Cmavo;

    #[bityzba::invariant(true)]
    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
    pub struct Token;

    jbotci_syntax_macros::syntax_grammar! {
        tree_model {}
        model { KeptSyntax };

        rule "kept" kept -> struct {
            field token <- cmavo(Be);
        }

        rule "skipped first" skipped_first -> struct {
            field token <- cmavo(Be);
        }

        rule "skipped second" skipped_second -> struct {
            field token <- cmavo(Bo);
        }
    }

    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    #[test]
    fn grammar_macro_filters_generated_model_outputs() {
        let kept = KeptSyntax(Token);
        assert_eq!(kept.0, Token);
    }
}

mod generated_model_with_env {
    use crate::{Cmavo, Selmaho};

    #[bityzba::invariant(true)]
    #[allow(dead_code)]
    struct SyntaxGrammarEnv;

    #[bityzba::invariant(true)]
    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
    pub struct Token;

    jbotci_syntax_macros::syntax_grammar! {
        tree_model {}
        model { EnvNodeSyntax };
        env SyntaxGrammarEnv;

        rule "env node" env_node -> struct {
            field token <- cmavo(Be);
        }
    }

    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    #[test]
    fn grammar_macro_emits_model_items_when_env_is_present() {
        let node = EnvNodeSyntax(Token);
        assert_eq!(node.0, Token);
        assert_eq!(SYNTAX_GRAMMAR_ENV, "SyntaxGrammarEnv");
    }
}

mod new_dsl {
    use crate::{Cmavo, Selmaho};

    #[bityzba::invariant(true)]
    #[allow(dead_code)]
    struct SyntaxGrammarEnv;

    #[bityzba::invariant(true)]
    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
    pub struct Token;

    #[bityzba::invariant(true)]
    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
    pub struct ExternalSyntax {
        pub token: Token,
    }

    jbotci_syntax_macros::syntax_grammar! {
        tree_model {}
        model;
        env SyntaxGrammarEnv;

        recursive {
            item: ItemSyntax;
            external: ExternalSyntax;
        }

        rule "item" item -> struct {
            field token <- cmavo(Be);
            field computed: usize = 1usize;
            let temp = 2usize;
            assert feature(ZantufaTags);
            assert !policy(ZantufaQuotes);
            assert !cmavo(Bo);
        }

        rule "other item" other_item -> struct {
            field token <- cmavo(Bo);
        }

        rule "gated item" gated_item -> struct {
            field token <- cmavo(Be);
        }

        rule "token list" token_list -> struct {
            field tokens <- [
                cmavo(Be);
                zero_or_more cmavo(Bo);
                zero_or_more ..[cmavo(Bo)];
                one_or_more ..[cmavo(Be)];
                assert !cmavo(Be);
            ];
        }

        rule "nested token list" nested_token_list -> struct {
            field tokens <- choice((
                [cmavo(Be)],
                [cmavo(Bo)],
            ));
        }

        rule "item choice" item_choice -> enum {
            item,
            other_item,
            when feature(ZantufaTags) gated_item,
        }

        rule "item choice" external_item_choice(external) -> enum {
            external,
            item,
        }

        alias "item alias" item_alias = item;

        alias "guarded item alias" guarded_item_alias = cmavo(Bo).not().ignore_then(item);

        rule "chain link" chain_link -> struct {
            field connector <- cmavo(Bo);
            field item <- item;
        }

        rule "item chain" item_chain -> struct {
            field run <- chain(first: item, zero_or_more: chain_link, element: item);
        }
    }

    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    #[test]
    fn grammar_macro_accepts_explicit_struct_enum_and_alias_rules() {
        let item = ItemSyntax {
            token: Token,
            computed: 1,
        };
        let other_item = OtherItemSyntax(Token);
        let external = ExternalSyntax { token: Token };
        let token_list = TokenListSyntax(vec1::Vec1::new(Token));
        let nested_token_list = NestedTokenListSyntax(vec1::Vec1::new(Token));
        let item_choice = ItemChoiceSyntax::Item(item.clone());
        let other_choice = ItemChoiceSyntax::OtherItem(other_item);
        let external_choice = ExternalItemChoiceSyntax::External(external);
        let chain = ItemChainSyntax(jbotci_tree::Chain::new(
            item.clone(),
            vec![ChainLinkSyntax {
                connector: Token,
                item: item.clone(),
            }],
        ));

        assert_eq!(item.token, Token);
        assert_eq!(item.computed, 1);
        assert_eq!(token_list.0.len(), 1);
        assert_eq!(nested_token_list.0.len(), 1);
        assert_eq!(chain.0.links.len(), 1);
        assert!(matches!(item_choice, ItemChoiceSyntax::Item(_)));
        assert!(matches!(other_choice, ItemChoiceSyntax::OtherItem(_)));
        assert!(matches!(
            external_choice,
            ExternalItemChoiceSyntax::External(_)
        ));
    }

    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    #[test]
    fn grammar_macro_exports_new_dsl_metadata() {
        assert_eq!(SYNTAX_GRAMMAR_RULES.len(), 11);
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
            SyntaxGrammarRecoveryExpr::Ignored(&SyntaxGrammarRecoveryExpr::Lookahead(
                &SyntaxGrammarRecoveryExpr::Opaque("feature(ZantufaTags)")
            ))
        );
        assert_eq!(SYNTAX_GRAMMAR_RULES[0].fields[4].kind, "require");
        assert_eq!(
            SYNTAX_GRAMMAR_RULES[0].fields[4].recovery,
            SyntaxGrammarRecoveryExpr::Not(&SyntaxGrammarRecoveryExpr::Opaque(
                "policy(ZantufaQuotes)"
            ))
        );
        assert_eq!(SYNTAX_GRAMMAR_RULES[0].fields[5].kind, "require");
        assert_eq!(
            SYNTAX_GRAMMAR_RULES[0].fields[5].recovery,
            SyntaxGrammarRecoveryExpr::Not(&SyntaxGrammarRecoveryExpr::Cmavo(Cmavo::Bo))
        );

        assert_eq!(SYNTAX_GRAMMAR_RULES[3].kind, "struct");
        assert_eq!(SYNTAX_GRAMMAR_RULES[3].name, "token_list");
        assert_eq!(SYNTAX_GRAMMAR_RULES[3].fields[0].kind, "field");
        assert!(matches!(
            SYNTAX_GRAMMAR_RULES[3].fields[0].recovery,
            SyntaxGrammarRecoveryExpr::Sequence(_)
        ));

        assert_eq!(SYNTAX_GRAMMAR_RULES[4].kind, "struct");
        assert_eq!(SYNTAX_GRAMMAR_RULES[4].name, "nested_token_list");
        assert_eq!(SYNTAX_GRAMMAR_RULES[4].fields[0].kind, "field");
        assert_eq!(
            SYNTAX_GRAMMAR_RULES[4].fields[0].recovery,
            SyntaxGrammarRecoveryExpr::Choice(&[
                SyntaxGrammarRecoveryExpr::Sequence(&[SyntaxGrammarRecoveryExpr::Cmavo(Cmavo::Be)]),
                SyntaxGrammarRecoveryExpr::Sequence(&[SyntaxGrammarRecoveryExpr::Cmavo(Cmavo::Bo)]),
            ])
        );

        assert_eq!(SYNTAX_GRAMMAR_RULES[5].kind, "enum");
        assert_eq!(SYNTAX_GRAMMAR_RULES[5].output, "ItemChoiceSyntax");
        assert_eq!(SYNTAX_GRAMMAR_RULES[5].fields[0].kind, "variant");
        assert_eq!(SYNTAX_GRAMMAR_RULES[5].fields[0].name, "item");
        assert_eq!(SYNTAX_GRAMMAR_RULES[5].fields[2].name, "gated_item");
        assert_eq!(
            SYNTAX_GRAMMAR_RULES[5].fields[2].conditions,
            &[SyntaxGrammarCondition {
                kind: SyntaxGrammarConditionKind::Feature,
                name: "ZantufaTags",
            }]
        );

        assert_eq!(SYNTAX_GRAMMAR_RULES[6].kind, "enum");
        assert_eq!(SYNTAX_GRAMMAR_RULES[6].output, "ExternalItemChoiceSyntax");
        assert_eq!(SYNTAX_GRAMMAR_RULES[6].fields[0].name, "external");

        assert_eq!(SYNTAX_GRAMMAR_RULES[7].kind, "alias");
        assert_eq!(SYNTAX_GRAMMAR_RULES[7].output, "ItemSyntax");
        assert_eq!(SYNTAX_GRAMMAR_RULES[7].context, Some("item alias"));

        assert_eq!(SYNTAX_GRAMMAR_RULES[8].kind, "alias");
        assert_eq!(SYNTAX_GRAMMAR_RULES[8].output, "ItemSyntax");
        assert_eq!(SYNTAX_GRAMMAR_RULES[8].context, Some("guarded item alias"));
        assert_eq!(SYNTAX_GRAMMAR_RULES[8].fields[0].kind, "alias");
        assert_eq!(
            SYNTAX_GRAMMAR_RULES[8].fields[0].recovery,
            SyntaxGrammarRecoveryExpr::Sequence(&[
                SyntaxGrammarRecoveryExpr::Not(&SyntaxGrammarRecoveryExpr::Cmavo(Cmavo::Bo)),
                SyntaxGrammarRecoveryExpr::Rule("item"),
            ])
        );

        assert_eq!(SYNTAX_GRAMMAR_RULES[9].kind, "struct");
        assert_eq!(SYNTAX_GRAMMAR_RULES[9].name, "chain_link");
        assert_eq!(SYNTAX_GRAMMAR_RULES[10].kind, "struct");
        assert_eq!(SYNTAX_GRAMMAR_RULES[10].name, "item_chain");
        assert_eq!(
            SYNTAX_GRAMMAR_RULES[10].fields[0].recovery,
            SyntaxGrammarRecoveryExpr::Sequence(&[
                SyntaxGrammarRecoveryExpr::Rule("item"),
                SyntaxGrammarRecoveryExpr::Many(&SyntaxGrammarRecoveryExpr::Rule("chain_link")),
            ])
        );
        assert_eq!(
            GENERATED_MODEL_CHAIN_LINK_TREE_ELEMENT_FIELDS,
            &[("ChainLink", "item")]
        );
    }
}

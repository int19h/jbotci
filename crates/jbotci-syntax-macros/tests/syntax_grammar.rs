#[bityzba::requires(true)]
#[bityzba::ensures(!ret || documentation.starts_with("/// A word from selmaho `"))]
fn is_direct_selmaho_documentation(documentation: &str) -> bool {
    documentation
        .strip_prefix("/// A word from selmaho `")
        .and_then(|family| family.strip_suffix("`."))
        .is_some_and(|family| {
            !family.is_empty()
                && family
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric())
        })
}

#[bityzba::requires(true)]
#[bityzba::ensures(!ret || documentation.starts_with("/// The "))]
fn is_direct_cmavo_documentation(documentation: &str) -> bool {
    documentation
        .strip_prefix("/// The ")
        .and_then(|body| body.strip_suffix(" cmavo marker."))
        .map(|body| body.strip_prefix("optional ").unwrap_or(body))
        .and_then(|name| name.strip_prefix('`'))
        .and_then(|name| name.strip_suffix('`'))
        .is_some_and(|name| {
            !name.is_empty()
                && name
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric())
        })
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
fn has_misleading_single_token_field_documentation(source: &str) -> bool {
    for (offset, _) in source.match_indices("/// ") {
        let following = &source[offset..];
        let documentation = following
            .lines()
            .next()
            .expect("documentation match must contain its source line");
        let direct_selmaho_documentation = is_direct_selmaho_documentation(documentation);
        let direct_cmavo_documentation = is_direct_cmavo_documentation(documentation);
        if !direct_selmaho_documentation && !direct_cmavo_documentation {
            continue;
        }

        let Some(field_start) = following.find("field ") else {
            return true;
        };
        let declaration = &following[field_start..];
        let Some(declaration_end) = declaration.find(';') else {
            return true;
        };
        let declaration = &declaration[..declaration_end];
        let Some((_, parser)) = declaration.split_once("<-") else {
            return true;
        };
        let parser = parser.trim_start();
        let parser_inside_optional = parser.strip_prefix("opt(").unwrap_or(parser);
        if (direct_selmaho_documentation && parser.starts_with("arc("))
            || parser_inside_optional.starts_with("choice(")
            || parser_inside_optional.starts_with('(')
        {
            return true;
        }
    }
    false
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
#[test]
fn canonical_generated_grammar_has_no_placeholder_field_documentation() {
    let grammar = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../jbotci-syntax/src/grammar/generated.rs"
    ));
    for forbidden in [
        " component retained by the `",
        " component of this syntax node.",
    ] {
        assert!(
            !grammar.contains(forbidden),
            "canonical grammar documentation contains legacy placeholder `{forbidden}`"
        );
    }
    assert!(!has_misleading_single_token_field_documentation(grammar));
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
#[test]
fn single_token_documentation_audit_distinguishes_composite_fields() {
    for accepted in [
        "/// The optional `Bo` cmavo marker.\nfield bo <- opt(cmavo(Bo).wf());",
        "/// The optional pair containing an optional tag followed by a required `Bo` cmavo marker.\nfield tag_bo <- opt((opt(arc(tag)), cmavo(Bo).wf()));",
    ] {
        assert!(!has_misleading_single_token_field_documentation(accepted));
    }

    for rejected in [
        "/// The optional `Bo` cmavo marker.\nfield tag_bo <- opt((opt(arc(tag)), cmavo(Bo).wf()));",
        "/// A word from selmaho `Le`.\nfield description <- choice((selmaho(Le), selmaho(La))).wf();",
        "/// The `Bo` cmavo marker.\nfield pair <- (cmavo(Bo), cmavo(Be));",
    ] {
        assert!(has_misleading_single_token_field_documentation(rejected));
    }
}

#[bityzba::requires(true)]
#[bityzba::ensures(true)]
#[test]
fn external_schema_consumer_preserves_metadata_pair_ownership() {
    let consumer = include_str!("binding-schema-consumer/src/lib.rs");
    assert!(!consumer.contains("pair.first"));
    assert!(!consumer.contains("pair.second"));
    assert_eq!(
        consumer.matches("pair.into_data()").count(),
        3,
        "each metadata-pair consumer must move through the validated wrapper API"
    );
}

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
    A,
    Fa,
    Na,
    Pa,
    Se,
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

    /// Syntax model for linked arguments parsed by the `linkargs` grammar rule.
    rule "linked arguments" linkargs(sumti) -> struct {
        /// The source-ordered `be` component retained by the `linkargs` syntax node.
        field be <- cmavo(Be).wf();
        assert !cmavo(Bo);
        /// The source-ordered `fa` component retained by the `linkargs` syntax node.
        field fa <- selmaho(Fa).wf();
        /// The source-ordered `first_sumti` component retained by the `linkargs` syntax node.
        field first_sumti <- opt(boxed(sumti));
        /// The source-ordered `tagged` component retained by the `linkargs` syntax node.
        when feature(ZantufaTags) field tagged <- boxed(sumti);
        assert feature(ZantufaTags);
        assert !policy(ZantufaQuotes);
        when policy(ZantufaQuotes) assert !word_category(Quote);
        when policy(ZantufaQuotes) let folded = fold_chain(head, tail);
        /// The computed `computed` component retained by the `linkargs` syntax node.
        field computed: usize = 0usize;
    }

    /// Syntax model for bo sumti tail parsed by the `bo_sumti_tail` grammar rule.
    rule "bo sumti tail" bo_sumti_tail -> struct {
        /// The source-ordered `connective` component retained by the `bo_sumti_tail` syntax node.
        field connective <- choice(joik(), jek());
        /// The source-ordered `bo` component retained by the `bo_sumti_tail` syntax node.
        field bo <- cmavo(Bo).wf();
        /// The source-ordered `maybe_bo` component retained by the `bo_sumti_tail` syntax node.
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

        /// Syntax model for token helpers parsed by the `token_helpers` grammar rule.
        rule "token helpers" token_helpers -> struct {
            /// The source-ordered `pa` component retained by the `token_helpers` syntax node.
            field pa <- pa_word();
            /// The source-ordered `cmevla` component retained by the `token_helpers` syntax node.
            field cmevla <- cmevla_word();
            /// The source-ordered `leading_cmevla` component retained by the `token_helpers` syntax node.
            field leading_cmevla <- text_leading_cmevla_word();
            /// The source-ordered `relation` component retained by the `token_helpers` syntax node.
            field relation <- relation_word();
            /// The source-ordered `external` component retained by the `token_helpers` syntax node.
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

        /// Syntax model for item parsed by the `item` grammar rule.
        rule "item" item(item) -> enum {
            /// The `literal_item` alternative of item.
            literal_item,
            /// The `recursive_item` alternative of item.
            recursive_item,
            /// The `gated_item` alternative of item.
            when feature(ZantufaTags) gated_item,
            /// The `nullable_item` alternative of item.
            nullable_item,
            /// The `explicit_argument_item` alternative of item.
            explicit_argument_item,
        }

        /// Syntax model for literal item parsed by the `literal_item` grammar rule.
        rule "literal item" literal_item(item) -> struct {
            /// The source-ordered `be` component retained by the `literal_item` syntax node.
            field be <- cmavo(Be).wf();
            /// The source-ordered `bo` component retained by the `literal_item` syntax node.
            field bo <- cmavo(Bo).wf();
            /// The source-ordered `maybe_fa` component retained by the `literal_item` syntax node.
            field maybe_fa <- opt(selmaho(Fa).wf());
            /// The source-ordered `tail` component retained by the `literal_item` syntax node.
            field tail <- opt(arc(item));
        }

        /// Syntax model for optional run item parsed by the `optional_run_item` grammar rule.
        rule "optional run item" optional_run_item -> struct {
            /// The source-ordered `na` component retained by the `optional_run_item` syntax node.
            field na <- opt(selmaho(Na).wf());
            /// The source-ordered `se` component retained by the `optional_run_item` syntax node.
            field se <- opt(selmaho(Se).wf());
            /// The source-ordered `a` component retained by the `optional_run_item` syntax node.
            field a <- selmaho(A).wf();
        }

        /// Syntax model for recursive item parsed by the `recursive_item` grammar rule.
        rule "recursive item" recursive_item(item) -> struct {
            /// The source-ordered `inner` component retained by the `recursive_item` syntax node.
            field inner <- opt(arc(item));
            /// The source-ordered `pa` component retained by the `recursive_item` syntax node.
            field pa <- selmaho(Pa).wf();
        }

        /// Syntax model for repeated item parsed by the `repeated_item` grammar rule.
        rule "repeated item" repeated_item(item) -> struct {
            /// The source-ordered `items` component retained by the `repeated_item` syntax node.
            #[recovery_boundary]
            field items <- [zero_or_more item];
        }

        /// Syntax model for gated item parsed by the `gated_item` grammar rule.
        rule "gated item" gated_item -> struct {
            /// The source-ordered `fa` component retained by the `gated_item` syntax node.
            field fa <- selmaho(Fa).warn(ExperimentalAnchorMetadata).wf();
            /// The source-ordered `bo` component retained by the `gated_item` syntax node.
            when feature(ZantufaTags) field bo <- cmavo(Bo).wf();
        }

        /// Syntax model for nullable item parsed by the `nullable_item` grammar rule.
        rule "nullable item" nullable_item -> struct {
            /// The source-ordered `maybe_bo` component retained by the `nullable_item` syntax node.
            field maybe_bo <- opt(cmavo(Bo));
        }

        /// Syntax model for explicit argument item parsed by the `explicit_argument_item` grammar rule.
        rule "explicit argument item" explicit_argument_item(item) -> struct {
            /// The source-ordered `inner` component retained by the `explicit_argument_item` syntax node.
            field inner <- literal_item(item);
        }

        /// Syntax model for text quote parsed by the `text_quote` grammar rule.
        rule "text quote" text_quote(text) -> struct {
            /// The source-ordered `be` component retained by the `text_quote` syntax node.
            field be <- cmavo(Be).wf();
            /// The source-ordered `text` component retained by the `text_quote` syntax node.
            field text <- arc(text);
            /// The source-ordered `bo` component retained by the `text_quote` syntax node.
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
        let repeated_rule =
            syntax_grammar_rule_by_name("repeated_item").expect("repeated rule exists");
        assert!(repeated_rule.fields[0].recovery_boundary);
        assert!(
            anchors_for("repeated_item").fields[0]
                .anchors
                .iter()
                .all(|anchor| anchor.boundary_resync),
        );

        let literal = anchors_for("literal_item");
        assert_eq!(literal.fields[0].anchors[0].resume_field, 0);
        assert_eq!(
            literal.fields[0].anchors[0].origin,
            SyntaxGrammarAnchorOrigin::LiteralRun,
        );
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
        let optional_run = anchors_for("optional_run_item");
        let optional_run_start = &optional_run.fields[0].anchors[0];
        assert_eq!(optional_run_start.resume_field, 0);
        assert_eq!(
            optional_run_start.origin,
            SyntaxGrammarAnchorOrigin::LiteralRun,
        );
        for token in [Selmaho::Na, Selmaho::Se, Selmaho::A] {
            assert!(
                token_set_contains(
                    optional_run_start.start_tokens,
                    SyntaxGrammarAnchorToken::Selmaho(token),
                ),
                "optional literal run should include {token:?} in its start set",
            );
        }

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
                && anchor.origin == SyntaxGrammarAnchorOrigin::FieldFirst
                && token_set_contains(
                    anchor.start_tokens,
                    SyntaxGrammarAnchorToken::Cmavo(Cmavo::Be),
                )
        }));
        let repeated = anchors_for("repeated_item");
        assert!(repeated.fields[0].anchors.iter().any(|anchor| {
            anchor.resume_field == 0
                && anchor.origin == SyntaxGrammarAnchorOrigin::RepetitionElementFirst
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
    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

        /// Syntax model for pair parsed by the `pair` grammar rule.
        rule "pair" pair(item) -> struct {
            /// The source-ordered `head` component retained by the `pair` syntax node.
            field head <- cmavo(Be);
            /// The source-ordered `nonempty` component retained by the `pair` syntax node.
            field nonempty <- [one_or_more cmavo(Be)];
            assert !cmavo(Bo);
            /// The computed `computed` component retained by the `pair` syntax node.
            field computed: usize = 0usize;
            let temporary = 1usize;
            #[tree_child(primary)]
            /// The source-ordered `child` component retained by the `pair` syntax node.
            field child <- boxed(item);
        }

        /// Syntax model for choice parsed by the `choice` grammar rule.
        rule "choice" choice -> enum {
            /// The `choice_first` alternative of choice.
            choice_first,
            /// The `choice_second` alternative of choice.
            choice_second,
        }

        /// Syntax model for choice first parsed by the `choice_first` grammar rule.
        rule "choice first" choice_first -> struct {
            /// The source-ordered `token` component retained by the `choice_first` syntax node.
            field token <- cmavo(Be);
        }

        /// Syntax model for choice second parsed by the `choice_second` grammar rule.
        rule "choice second" choice_second(item) -> struct {
            /// The source-ordered `item` component retained by the `choice_second` syntax node.
            field item <- boxed(item);
        }

        /// Syntax model for helper product parsed by the `helper_product` grammar rule.
        rule "helper product" helper_product -> struct {
            /// The source-ordered `token` component retained by the `helper_product` syntax node.
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

mod binding_schema {
    #![allow(dead_code)]

    use crate::Cmavo;

    #[bityzba::invariant(true)]
    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub struct WordLike;

    #[bityzba::invariant(true)]
    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub struct Token;

    #[bityzba::invariant(true)]
    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub struct WithIndicators<T>(pub T);

    #[bityzba::invariant(true)]
    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
    pub struct WithFreeModifiers<T, F> {
        pub value: T,
        pub free_modifiers: Vec<F>,
    }

    type RecoveryTreeItem = ();
    type Recovered<T> = jbotci_tree::Recovered<T, RecoveryTreeItem>;

    pub mod jbotci_source {
        #[bityzba::invariant(true)]
        #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
        pub struct SourceSpan;

        #[bityzba::contract_trait]
        impl jbotci_tree::RecoveredFieldState for SourceSpan {
            fn recovery_error_slots(&self) -> usize {
                0
            }
        }
    }

    jbotci_syntax_macros::syntax_grammar! {
        tree_model {
            #![tree_recovered]
            #![tree_with_free_modifiers]
        }
        model;
        binding_schema __binding_schema_fixture;

        recursive {
            item: ItemSyntax;
        }

        /// Canonical item documentation shared by Rust and bindings.
        ///
        /// Its blank paragraph separator must survive schema normalization.
        #[doc(hidden)]
        #[doc(alias = "schema item")]
        #[deny(missing_docs)]
        rule "item" item(item) -> struct {
            /// The item token.
            field token <- cmavo(Be);
            /// An optional token.
            field optional <- opt(cmavo(Be));
            /// A possibly empty token sequence.
            field repeated <- [zero_or_more cmavo(Be)];
            /// A non-empty token sequence.
            field non_empty <- [one_or_more cmavo(Be)];
            /// A boxed recursive item.
            field boxed <- boxed(item);
            /// A shared recursive item.
            field shared <- arc(item);
            /// A token carrying following free modifiers.
            field with_free_modifiers <- cmavo(Be).wf();
            /// A morphology value carrying indicators.
            field with_indicators: WithIndicators<WordLike> = unreachable!();
            /// A shared source span.
            field source_span: std::sync::Arc<jbotci_source::SourceSpan> = unreachable!();
            /// A source span whose absolute type path must remain distinct in the schema.
            field absolute_source_span: ::jbotci_source::SourceSpan = unreachable!();
            /// A small repeated token sequence.
            field small: smallvec::SmallVec<[Token; 2]> = smallvec::SmallVec::new();
            /// A non-empty small token sequence.
            // This initialized-unreachable schema probe is outside this synthetic model's
            // traversal contract.
            #[tree_child(false)]
            field small_non_empty: vec1::smallvec_v1::SmallVec1<[Token; 2]> = unreachable!();
            /// Exactly two tokens.
            // This initialized-unreachable schema probe is outside this synthetic model's
            // traversal contract.
            #[tree_child(false)]
            field fixed: [Token; 2] = unreachable!();
            /// A pair of token values.
            field tuple: (Token, Token) = unreachable!();
            /// A token already represented as an explicit recovery field.
            // This initialized-unreachable schema probe is outside this synthetic model's
            // traversal contract.
            #[tree_child(false)]
            field explicit_recovered: Recovered<Token> = unreachable!();
            /// An optional BE terminator recorded in generated model metadata.
            field terminator <- opt(cmavo(Be)).elidable_terminator(Be);
        }

        /// Canonical free-modifier documentation.
        rule "free modifier" free_modifier -> struct {
            /// The free-modifier token.
            field token <- cmavo(Be);
        }

        /// Canonical transparent-wrapper documentation.
        rule "wrapper" wrapper -> struct {
            /// The wrapped token.
            field token <- cmavo(Be);
        }

        /// A source-ordered link in the representative item chain.
        rule "chain link" chain_link(item) -> struct {
            /// The BO connector introducing this link.
            field connector <- cmavo(Bo);
            /// The item contributed by this link.
            field item <- item;
        }

        /// A representative chain whose first item and links have distinct payload types.
        rule "item chain" item_chain(item) -> struct {
            /// The first item followed by zero or more BO-linked items.
            field run <- chain(first: item, zero_or_more: chain_link(item), element: item);
        }

        /// Canonical choice documentation.
        #[deny(missing_docs)]
        rule "choice" choice(item) -> enum {
            /// The item alternative.
            item,
            /// The wrapper alternative added directly from the grammar.
            wrapper,
        }
    }

    macro_rules! capture_binding_schema {
        ($($schema:tt)*) => {
            const CAPTURED_BINDING_SCHEMA: &str = stringify!($($schema)*);
        };
    }

    __binding_schema_fixture!(capture_binding_schema);

    #[bityzba::requires(!schema.is_empty() && !field.is_empty())]
    #[bityzba::ensures(ret.starts_with("field{"))]
    fn field_schema<'schema>(schema: &'schema str, field: &str) -> &'schema str {
        let marker = format!("field{{source_name(\"{field}\"),");
        let start = schema.find(&marker).expect("schema field is present");
        let tail = &schema[start..];
        let end = tail[marker.len()..]
            .find("field{")
            .map_or(tail.len(), |offset| marker.len() + offset);
        &tail[..end]
    }

    #[bityzba::requires(!schema.is_empty() && !field.is_empty())]
    #[bityzba::ensures(true)]
    fn assert_field_shapes(schema: &str, field: &str, strict: &str, recovered: &str) {
        let field_schema = field_schema(schema, field);
        assert!(
            field_schema.contains(&format!("strict({strict}),recovered({recovered})")),
            "field `{field}` has unexpected strict/recovered schema: {field_schema}"
        );
    }

    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    #[test]
    fn callback_exports_documented_normalized_strict_and_recovered_shapes() {
        let schema = CAPTURED_BINDING_SCHEMA;
        assert!(schema.contains("Canonical item documentation shared by Rust and bindings."));
        assert!(
            schema.contains("Its blank paragraph separator must survive schema normalization.")
        );
        assert!(
            schema.contains("\"\"") || schema.contains("\" \""),
            "blank Rustdoc fragments remain present in the schema"
        );
        assert!(schema.contains("The item token."));
        assert!(schema.contains("The item alternative."));
        assert!(schema.contains("The wrapper alternative added directly from the grammar."));
        assert!(
            !schema.contains("schema item"),
            "non-text rustdoc metadata is not duplicated into canonical schema text"
        );

        let compact = schema.split_whitespace().collect::<String>();
        assert!(compact.contains("names(strict(\"ItemSyntax\"),recovered(\"ItemSyntax\"))"));
        let token = "reference(leaf(kind(syntax_token),absolute(false),path(\"Token\")))";
        let recovered_token = format!("recovered_field({token})");
        let item = "reference(model(\"ItemSyntax\"))";
        let recovered_item = format!("recovered_field({item})");
        assert_field_shapes(&compact, "token", token, &recovered_token);
        assert_field_shapes(
            &compact,
            "optional",
            &format!("optional({token})"),
            &format!("optional({recovered_token})"),
        );
        assert_field_shapes(
            &compact,
            "repeated",
            &format!("repeated({token})"),
            &format!("repeated({recovered_token})"),
        );
        assert_field_shapes(
            &compact,
            "non_empty",
            &format!("non_empty_repeated({token})"),
            &format!("non_empty_repeated({recovered_token})"),
        );
        assert_field_shapes(
            &compact,
            "boxed",
            &format!("boxed({item})"),
            &format!("boxed({recovered_item})"),
        );
        assert_field_shapes(
            &compact,
            "shared",
            &format!("shared({item})"),
            &format!("shared({recovered_item})"),
        );
        let free_modifier = "reference(model(\"FreeModifierSyntax\"))";
        assert_field_shapes(
            &compact,
            "with_free_modifiers",
            &format!("with_free_modifiers(value({token}),free_modifier({free_modifier}))"),
            &format!(
                "with_free_modifiers(value({recovered_token}),free_modifiers(repeated(recovered_field({free_modifier}))))"
            ),
        );
        let word_like =
            "reference(leaf(kind(morphology_word_like),absolute(false),path(\"WordLike\")))";
        assert_field_shapes(
            &compact,
            "with_indicators",
            &format!("with_indicators({word_like})"),
            &format!("recovered_field(with_indicators({word_like}))"),
        );
        let source_span = "reference(leaf(kind(source_span),absolute(false),path(\"jbotci_source\",\"SourceSpan\")))";
        assert_field_shapes(
            &compact,
            "source_span",
            &format!("shared({source_span})"),
            &format!("shared(recovered_field({source_span}))"),
        );
        let absolute_source_span = "reference(leaf(kind(source_span),absolute(true),path(\"jbotci_source\",\"SourceSpan\")))";
        assert_field_shapes(
            &compact,
            "absolute_source_span",
            absolute_source_span,
            &format!("recovered_field({absolute_source_span})"),
        );
        assert_field_shapes(
            &compact,
            "small",
            &format!("repeated({token})"),
            &format!("repeated({recovered_token})"),
        );
        assert_field_shapes(
            &compact,
            "small_non_empty",
            &format!("non_empty_repeated({token})"),
            &format!("non_empty_repeated({recovered_token})"),
        );
        assert_field_shapes(
            &compact,
            "fixed",
            &format!("fixed(length(2usize),value({token}))"),
            &format!("fixed(length(2usize),value({recovered_token}))"),
        );
        assert_field_shapes(
            &compact,
            "tuple",
            &format!("tuple({token},{token})"),
            &format!("tuple({recovered_token},{recovered_token})"),
        );
        assert_field_shapes(
            &compact,
            "explicit_recovered",
            &format!("recovered_field({token})"),
            &format!("recovered_field({recovered_token})"),
        );
        assert_field_shapes(
            &compact,
            "terminator",
            &format!("optional({token})"),
            &format!("optional({recovered_token})"),
        );
        let chain_link = "reference(model(\"ChainLinkSyntax\"))";
        assert_field_shapes(
            &compact,
            "run",
            &format!("chain(first({item}),links(repeated({chain_link})))"),
            &format!(
                "chain(first({recovered_item}),links(repeated(recovered_field({chain_link}))))"
            ),
        );
        assert!(
            compact.contains("shape(tuple)"),
            "transparent products and variants are tuples"
        );
        assert!(compact.contains(
            "reference(leaf(kind(source_span),absolute(true),path(\"jbotci_source\",\"SourceSpan\")))"
        ));
        assert!(compact.contains("transparent_field(\"Wrapper\",\"token\")"));
        assert!(compact.contains("chain_link_element_field(\"ChainLink\",\"item\")"));
        assert!(compact.contains("elidable_terminator(\"terminator\",\"Be\")"));
        assert!(compact.contains("field_order(\"ChainLink\",[\"connector\",\"item\"])"));
        assert!(compact.contains("constructor_label(\"Item\",\"item\")"));

        let token = compact.find("source_name(\"token\")").expect("token field");
        let optional = compact
            .find("source_name(\"optional\")")
            .expect("optional field");
        let repeated = compact
            .find("source_name(\"repeated\")")
            .expect("repeated field");
        assert!(
            token < optional && optional < repeated,
            "source field order is preserved"
        );

        let item_variant = compact.find("source_rule(\"item\")").expect("item variant");
        let wrapper_variant = compact
            .find("source_rule(\"wrapper\")")
            .expect("wrapper variant");
        assert!(
            item_variant < wrapper_variant,
            "enum branch order is preserved"
        );
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

        /// Syntax model for kept parsed by the `kept` grammar rule.
        rule "kept" kept -> struct {
            /// The source-ordered `token` component retained by the `kept` syntax node.
            field token <- cmavo(Be);
        }

        /// Syntax model for skipped first parsed by the `skipped_first` grammar rule.
        rule "skipped first" skipped_first -> struct {
            /// The source-ordered `token` component retained by the `skipped_first` syntax node.
            field token <- cmavo(Be);
        }

        /// Syntax model for skipped second parsed by the `skipped_second` grammar rule.
        rule "skipped second" skipped_second -> struct {
            /// The source-ordered `token` component retained by the `skipped_second` syntax node.
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

        /// Syntax model for env node parsed by the `env_node` grammar rule.
        rule "env node" env_node -> struct {
            /// The source-ordered `token` component retained by the `env_node` syntax node.
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

        /// Syntax model for item parsed by the `item` grammar rule.
        rule "item" item -> struct {
            /// The source-ordered `token` component retained by the `item` syntax node.
            field token <- cmavo(Be);
            /// The computed `computed` component retained by the `item` syntax node.
            field computed: usize = 1usize;
            let temp = 2usize;
            assert feature(ZantufaTags);
            assert !policy(ZantufaQuotes);
            assert !cmavo(Bo);
        }

        /// Syntax model for other item parsed by the `other_item` grammar rule.
        rule "other item" other_item -> struct {
            /// The source-ordered `token` component retained by the `other_item` syntax node.
            field token <- cmavo(Bo);
        }

        /// Syntax model for gated item parsed by the `gated_item` grammar rule.
        rule "gated item" gated_item -> struct {
            /// The source-ordered `token` component retained by the `gated_item` syntax node.
            field token <- cmavo(Be);
        }

        /// Syntax model for token list parsed by the `token_list` grammar rule.
        rule "token list" token_list -> struct {
            /// The source-ordered `tokens` component retained by the `token_list` syntax node.
            field tokens <- [
                cmavo(Be);
                zero_or_more cmavo(Bo);
                zero_or_more ..[cmavo(Bo)];
                one_or_more ..[cmavo(Be)];
                assert !cmavo(Be);
            ];
        }

        /// Syntax model for nested token list parsed by the `nested_token_list` grammar rule.
        rule "nested token list" nested_token_list -> struct {
            /// The source-ordered `tokens` component retained by the `nested_token_list` syntax node.
            field tokens <- choice((
                [cmavo(Be)],
                [cmavo(Bo)],
            ));
        }

        /// Syntax model for item choice parsed by the `item_choice` grammar rule.
        rule "item choice" item_choice -> enum {
            /// The `item` alternative of item choice.
            item,
            /// The `other_item` alternative of item choice.
            other_item,
            /// The `gated_item` alternative of item choice.
            when feature(ZantufaTags) gated_item,
        }

        /// Syntax model for item choice parsed by the `external_item_choice` grammar rule.
        rule "item choice" external_item_choice(external) -> enum {
            /// The `external` alternative of item choice.
            external,
            /// The `item` alternative of item choice.
            item,
        }

        alias "item alias" item_alias = item;

        alias "guarded item alias" guarded_item_alias = cmavo(Bo).not().ignore_then(item);

        /// Syntax model for chain link parsed by the `chain_link` grammar rule.
        rule "chain link" chain_link -> struct {
            /// The source-ordered `connector` component retained by the `chain_link` syntax node.
            field connector <- cmavo(Bo);
            /// The source-ordered `item` component retained by the `chain_link` syntax node.
            field item <- item;
        }

        /// Syntax model for item chain parsed by the `item_chain` grammar rule.
        rule "item chain" item_chain -> struct {
            /// The source-ordered `run` component retained by the `item_chain` syntax node.
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

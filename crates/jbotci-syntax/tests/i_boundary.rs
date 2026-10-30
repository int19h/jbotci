#![recursion_limit = "1024"]

//! Statement ownership at I boundaries follows the declared connection tiers.

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use jbotci_dialect::parse_dialect_definition;
use jbotci_morphology::{Cmavo, segment_words_with_modifiers};
use jbotci_syntax::{
    ExperimentalConstruct, ParseOptions, SyntaxParse, generated_model as model,
    parse_syntax_tree_with_source_and_options,
};

/// Each collection borrows nodes from one immutable tree.
#[invariant(true)]
#[derive(Default)]
struct Boundaries<'tree> {
    joins: Vec<&'tree model::StatementJoinSyntax>,
    bo: Vec<&'tree model::BoStatementConnectionSyntax>,
    following: Vec<&'tree model::FollowingParagraphStatementSyntax>,
    trailers: Vec<&'tree model::ExpSentenceTrailerSyntax>,
    soi: Vec<&'tree model::ExpSoiSubsentenceAdverbialSyntax>,
    sei: Vec<&'tree model::SeiFreeModifierSyntax>,
}

impl<'tree> model::TreeWalker<'tree> for Boundaries<'tree> {
    #[requires(true)]
    #[ensures(self.sei.len() >= old(self.sei.len()) + 1)]
    fn walk_sei_free_modifier(&mut self, node: &'tree model::SeiFreeModifierSyntax) {
        self.sei.push(node);
        model::walk::sei_free_modifier(self, node);
    }

    #[requires(true)]
    #[ensures(self.joins.len() >= old(self.joins.len()) + 1)]
    fn walk_statement_join(&mut self, node: &'tree model::StatementJoinSyntax) {
        self.joins.push(node);
        model::walk::statement_join(self, node);
    }

    #[requires(true)]
    #[ensures(self.bo.len() >= old(self.bo.len()) + 1)]
    fn walk_bo_statement_connection(&mut self, node: &'tree model::BoStatementConnectionSyntax) {
        self.bo.push(node);
        model::walk::bo_statement_connection(self, node);
    }

    #[requires(true)]
    #[ensures(self.following.len() >= old(self.following.len()) + 1)]
    fn walk_following_paragraph_statement(
        &mut self,
        node: &'tree model::FollowingParagraphStatementSyntax,
    ) {
        self.following.push(node);
        model::walk::following_paragraph_statement(self, node);
    }

    #[requires(true)]
    #[ensures(self.trailers.len() >= old(self.trailers.len()) + 1)]
    fn walk_exp_sentence_trailer(&mut self, node: &'tree model::ExpSentenceTrailerSyntax) {
        self.trailers.push(node);
        model::walk::exp_sentence_trailer(self, node);
    }

    #[requires(true)]
    #[ensures(self.soi.len() >= old(self.soi.len()) + 1)]
    fn walk_exp_soi_subsentence_adverbial(
        &mut self,
        node: &'tree model::ExpSoiSubsentenceAdverbialSyntax,
    ) {
        self.soi.push(node);
        model::walk::exp_soi_subsentence_adverbial(self, node);
    }
}

#[requires(true)]
#[ensures(true)]
fn parse(source: &str, dialect: &str) -> SyntaxParse {
    let words = segment_words_with_modifiers(source).expect("valid morphology");
    let dialect = parse_dialect_definition(dialect).expect("valid dialect");
    parse_syntax_tree_with_source_and_options(
        &words,
        source,
        &ParseOptions::default().with_dialect_definition(&dialect),
    )
    .unwrap_or_else(|error| panic!("{source}: {error:?}"))
}

#[requires(true)]
#[ensures(true)]
fn boundaries(parsed: &SyntaxParse) -> Boundaries<'_> {
    let mut visitor = Boundaries::default();
    model::TreeWalkable::walk_with(parsed.parse_tree.as_ref(), &mut visitor);
    visitor
}

#[test]
#[requires(true)]
#[ensures(true)]
fn rightless_joins_resume_at_a_later_bare_i() {
    for source in [
        "mi klama .i je .i do cadzu",
        "ro da zo'u mi klama .i je .i mi",
        "lu mi klama .i je .i do cadzu li'u",
    ] {
        let parsed = parse(source, "()");
        let nodes = boundaries(&parsed);
        assert_eq!(nodes.joins.len(), 1, "{source}");
        assert!(nodes.joins[0].trailing_statement.is_none(), "{source}");
        assert_eq!(nodes.following.len(), 1, "{source}");
        assert!(nodes.following[0].statement.is_some(), "{source}");
    }
    let parsed = parse("mi klama .i je .i joi do cadzu", "()");
    let nodes = boundaries(&parsed);
    assert_eq!(nodes.joins.len(), 2);
    assert!(nodes.joins[0].trailing_statement.is_none());
    assert!(nodes.joins[1].trailing_statement.is_some());
    assert!(nodes.following.is_empty());
}

#[test]
#[requires(true)]
#[ensures(true)]
fn bo_owns_its_right_recursive_operand() {
    let parsed = parse("mi klama .i je bo do cadzu .i ja bo ti tavla", "()");
    let nodes = boundaries(&parsed);
    assert_eq!(nodes.bo.len(), 2);
    assert!(matches!(
        nodes.bo[0].trailing_statement.as_deref(),
        Some(model::BoStatementSyntax::BoStatementConnection(_))
    ));
    assert!(nodes.joins.is_empty());

    let parsed = parse("mi klama .i je do cadzu .i ja bo", "()");
    let nodes = boundaries(&parsed);
    assert_eq!(nodes.joins.len(), 1);
    assert!(matches!(
        nodes.joins[0].trailing_statement.as_deref(),
        Some(model::BoStatementSyntax::BoStatementConnection(_))
    ));
    assert_eq!(nodes.bo.len(), 1);
    assert!(nodes.bo[0].trailing_statement.is_none());
}

#[test]
#[requires(true)]
#[ensures(true)]
fn ek_feature_keeps_the_standard_fragment_reading_when_disabled() {
    let source = "mi klama .i .e";
    let standard = parse(source, "()");
    let nodes = boundaries(&standard);
    assert!(nodes.joins.is_empty());
    assert_eq!(nodes.following.len(), 1);
    assert!(standard.warnings.is_empty());

    let experimental = parse(source, "(+ek-statement-connective)");
    let nodes = boundaries(&experimental);
    assert_eq!(nodes.joins.len(), 1);
    assert!(nodes.following.is_empty());
    assert_eq!(experimental.warnings.len(), 1);
    assert_eq!(
        experimental.warnings[0].kind,
        ExperimentalConstruct::ExperimentalEkStatementConnective
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn completed_ek_joins_need_the_feature() {
    for source in [
        "y xu jibri galfi .iji jibri zvati galfi",
        " i lu e'upei mi'a troci lo drata morna be fi le xasyjukni vonseldansu sei la cpicinfo cu di'a cusku iji aupei la jitfa cakyrespa cu sanga da do li'u ",
        "mi klama .i .e do klama",
        "mi klama .i .e do cadzu",
        "mi klama .i .e bo do cadzu",
        "mi klama .i je .i .e do cadzu",
        "lu mi klama .i ji .au pei la jitfa cakyrespa cu sanga da do li'u",
    ] {
        let words = segment_words_with_modifiers(source).expect("valid morphology");
        assert!(
            parse_syntax_tree_with_source_and_options(&words, source, &ParseOptions::default())
                .is_err(),
            "{source}"
        );
        let enabled = parse(source, "(+ek-statement-connective)");
        assert_eq!(
            enabled
                .warnings
                .iter()
                .filter(|warning| {
                    warning.kind == ExperimentalConstruct::ExperimentalEkStatementConnective
                })
                .count(),
            1,
            "{source}"
        );
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn an_elided_quote_keeps_the_parent_sei_words_by_default() {
    for source in [
        "mi klama sei lu broda .i .e do tavla se'u",
        "mi klama sei lu broda .i .e do cu tavla se'u",
    ] {
        let standard = parse(source, "()");
        let nodes = boundaries(&standard);
        assert!(nodes.joins.is_empty());
        assert_eq!(nodes.following.len(), 1);
        assert_eq!(nodes.sei.len(), 1);
        assert_eq!(nodes.sei[0].terms.len(), 2);
        assert!(standard.warnings.is_empty());

        let words = segment_words_with_modifiers(source).expect("valid morphology");
        let dialect = parse_dialect_definition("(+ek-statement-connective)").unwrap();
        assert!(
            parse_syntax_tree_with_source_and_options(
                &words,
                source,
                &ParseOptions::default().with_dialect_definition(&dialect),
            )
            .is_err()
        );
    }
    let explicit = "mi klama sei lu broda .i .e li'u do cu tavla se'u";
    for dialect in ["()", "(+ek-statement-connective)"] {
        let parsed = parse(explicit, dialect);
        assert_eq!(boundaries(&parsed).sei[0].terms.len(), 2);
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn sentence_trailers_reach_nested_subsentence_consumers() {
    for source in [
        "mi klama je .i ro da zo'u do cadzu",
        "mi djica lo nu mi klama je .i do cadzu kei",
        "lo prenu poi mi klama je .i do cadzu ku'o cu tavla",
    ] {
        let parsed = parse(source, "()");
        assert_eq!(boundaries(&parsed).trailers.len(), 1, "{source}");
        assert!(parsed.warnings.iter().any(|warning| {
            warning.kind == ExperimentalConstruct::ExperimentalPreISentenceTrailer
        }));
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn fragments_and_pending_joins_cannot_supply_a_missing_bo_operand() {
    for source in [
        "mi .i je",
        "mi klama .i je .i bo do cadzu",
        "mi klama .i su'i bo do cadzu",
        "mi klama je .i tu'e do cadzu tu'u",
        "mi klama .i je ro da zo'u do cadzu",
        "mi klama .i bo ro da zo'u do cadzu",
        "ni'o .i mi klama",
        "ni'o .i ni'o mi klama",
    ] {
        let words = segment_words_with_modifiers(source).expect("valid morphology");
        assert!(
            parse_syntax_tree_with_source_and_options(&words, source, &ParseOptions::default())
                .is_err(),
            "{source}"
        );
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn stag_keeps_its_ordered_standard_route_and_warns_on_the_exp_route() {
    for source in [
        "mi klama .i fi'o broda fe'u bo do cadzu",
        "mi klama .i je pu je ba bo do cadzu",
    ] {
        let parsed = parse(source, "()");
        assert!(
            !parsed.warnings.iter().any(|warning| {
                warning.kind == ExperimentalConstruct::ExperimentalStatementStag
            })
        );
    }
    let parsed = parse("mi klama .i pu to broda toi bo do cadzu", "()");
    assert!(
        parsed
            .warnings
            .iter()
            .any(|warning| { warning.kind == ExperimentalConstruct::ExperimentalStatementStag })
    );
    let parsed = parse(".i je to broda toi bo mi klama", "()");
    assert!(parsed.warnings.iter().any(|warning| {
        warning.kind == ExperimentalConstruct::CllProhibitedFreeModifierPlacement
    }));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn ek_inside_stag_is_default_and_warns_on_its_connector() {
    for source in [
        "mi klama .i pu .e ba bo do cadzu",
        "mi klama .i fi'o broda fe'u .e pu bo do cadzu",
    ] {
        for dialect in ["()", "(+ek-statement-connective)"] {
            let parsed = parse(source, dialect);
            let nodes = boundaries(&parsed);
            assert_eq!(nodes.bo.len(), 1, "{source}: {dialect}");
            assert!(nodes.following.is_empty(), "{source}: {dialect}");
            let warning = parsed
                .warnings
                .iter()
                .find(|warning| warning.kind == ExperimentalConstruct::ExperimentalEkStagConnective)
                .expect("EK inside the stag has a warning");
            assert_eq!(warning.anchor.cmavo(), Some(Cmavo::E));
        }
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn inherited_prohibited_stag_placements_remain_warned() {
    for source in [
        "mi klama .i pa to broda toi roi bo do cadzu",
        "mi klama .i fe'e to broda toi ta'e bo do cadzu",
        "mi klama .i mo'i to broda toi zu'a bo do cadzu",
        "mi klama .i pa to broda toi roi nai bo do cadzu",
        "mi klama .i fe'e to broda toi ta'e nai bo do cadzu",
        "mi klama .i mo'i to broda toi zu'a nai va bo do cadzu",
    ] {
        let parsed = parse(source, "()");
        let warning = parsed
            .warnings
            .iter()
            .find(|warning| {
                warning.kind == ExperimentalConstruct::CllProhibitedFreeModifierPlacement
            })
            .expect("the actual modifier has a placement warning");
        assert_eq!(warning.anchor.cmavo(), Some(Cmavo::To));
    }
    for source in [
        "mi klama .i pa roi bo do cadzu",
        "mi klama .i fe'e ta'e bo do cadzu",
        "mi klama .i mo'i zu'a bo do cadzu",
        "mi klama .i pa roi nai bo do cadzu",
        "mi klama .i fe'e ta'e nai bo do cadzu",
        "mi klama .i mo'i zu'a nai va bo do cadzu",
    ] {
        let parsed = parse(source, "()");
        assert!(!parsed.warnings.iter().any(|warning| {
            warning.kind == ExperimentalConstruct::CllProhibitedFreeModifierPlacement
                || warning.kind == ExperimentalConstruct::ExperimentalStatementStag
        }));
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn a_sentence_trailer_stays_inside_an_elided_soi_adverbial() {
    let parsed = parse("mi klama soi do cadzu je .i ti tavla", "()");
    let nodes = boundaries(&parsed);
    assert_eq!(nodes.soi.len(), 1);
    let model::SubbridiSyntax::BridiSubbridi(inner) = nodes.soi[0].subsentence.as_ref() else {
        panic!("the SOI body is a sentence");
    };
    assert!(matches!(
        inner.0.as_ref(),
        model::BridiSyntax::ExpTrailedBridi(_)
    ));
    assert!(
        parsed
            .warnings
            .iter()
            .any(|warning| { warning.kind == ExperimentalConstruct::ExperimentalSoiAdverbial })
    );

    let baseline = parse("mi klama soi do cadzu", "()");
    assert!(boundaries(&baseline).soi.is_empty());
    assert!(baseline.warnings.is_empty());
}

#[test]
#[requires(true)]
#[ensures(true)]
fn a_trailer_warns_for_free_modifiers_between_bo_and_i() {
    for dialect in ["()", "(+unrestricted-free)"] {
        let parsed = parse("mi klama je bo to broda toi .i do cadzu", dialect);
        let warning = parsed
            .warnings
            .iter()
            .find(|warning| {
                warning.kind == ExperimentalConstruct::CllProhibitedFreeModifierPlacement
            })
            .expect("the free modifier before I has a placement warning");
        assert_eq!(warning.anchor.cmavo(), Some(Cmavo::To));
    }
    for source in [
        "mi klama je bo .i to broda toi do cadzu",
        "mi klama je bo .i do cadzu",
    ] {
        let parsed = parse(source, "()");
        assert!(parsed.warnings.iter().all(|warning| {
            warning.kind != ExperimentalConstruct::CllProhibitedFreeModifierPlacement
        }));
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn na_joi_in_the_new_source_slots_needs_no_dialect_feature() {
    for dialect in ["()", "(+na-joik)"] {
        for source in [
            "mi klama na joi .i do cadzu",
            "mi klama na joi pu bo .i do cadzu",
            "mi klama na se joi nai .i do cadzu",
        ] {
            let parsed = parse(source, dialect);
            let nodes = boundaries(&parsed);
            assert_eq!(nodes.trailers.len(), 1, "{source}");
            assert!(matches!(
                nodes.trailers[0].connective.value.as_ref(),
                model::SentenceTrailerConnectiveSyntax::TrailerExpParagraphNaJoiConnective(_)
            ));
            let warning = parsed
                .warnings
                .iter()
                .find(|warning| warning.kind == ExperimentalConstruct::ExperimentalNaJoiConnective)
                .expect("the connector has its NA JOI warning");
            assert_eq!(warning.anchor.cmavo(), Some(Cmavo::Joi));
        }
        for source in [
            "mi klama .i pu na joi ba bo do cadzu",
            "mi klama .i fi'o broda fe'u na joi pu bo do cadzu",
        ] {
            let parsed = parse(source, dialect);
            let nodes = boundaries(&parsed);
            assert_eq!(nodes.bo.len(), 1, "{source}");
            assert!(nodes.following.is_empty(), "{source}");
            assert!(parsed.warnings.iter().any(|warning| {
                warning.kind == ExperimentalConstruct::ExperimentalNaJoiConnective
            }));
        }
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn the_na_joi_prefix_refusal_preserves_the_standard_forethought_reading() {
    let parsed = parse("mi klama .i na joi gi do cadzu gi ti tavla", "()");
    let nodes = boundaries(&parsed);
    assert_eq!(nodes.following.len(), 1);
    assert!(nodes.joins.is_empty());
    assert!(
        parsed
            .warnings
            .iter()
            .all(|warning| { warning.kind != ExperimentalConstruct::ExperimentalNaJoiConnective })
    );
    let free = parse("mi klama na to broda toi joi .i do cadzu", "()");
    assert!(
        free.warnings
            .iter()
            .all(|warning| { warning.kind != ExperimentalConstruct::ExperimentalNaJoiConnective })
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn a_pre_i_negation_stays_with_its_converted_connector() {
    for dialect in ["()", "(+na-joik)"] {
        for source in [
            "mi klama na se je .i do cadzu",
            "mi klama na je .i do cadzu",
        ] {
            let parsed = parse(source, dialect);
            let nodes = boundaries(&parsed);
            let model::SentenceTrailerConnectiveSyntax::TrailerParagraphJekConnective(connector) =
                nodes.trailers[0].connective.value.as_ref()
            else {
                panic!("the trailer has a JEK connector");
            };
            assert!(connector.na.is_some());
        }
        let parsed = parse("mi klama na se .e .i do cadzu", dialect);
        let nodes = boundaries(&parsed);
        let model::SentenceTrailerConnectiveSyntax::TrailerEkConnective(connector) =
            nodes.trailers[0].connective.value.as_ref()
        else {
            panic!("the trailer has an EK connector");
        };
        assert!(connector.na.is_some());
        let parsed = parse("mi klama na to broda toi se je .i do cadzu", dialect);
        let nodes = boundaries(&parsed);
        let model::SentenceTrailerConnectiveSyntax::TrailerParagraphJekConnective(connector) =
            nodes.trailers[0].connective.value.as_ref()
        else {
            panic!("the trailer has a JEK connector after the free modifier");
        };
        assert!(connector.na.is_none());
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn unrestricted_free_keeps_the_legacy_pre_i_nai_attachment() {
    for source in [
        "mi klama je to broda toi nai .i do cadzu",
        "mi klama na joi to broda toi nai .i do cadzu",
        "mi klama bi'i to broda toi nai .i do cadzu",
        "mi klama .e to broda toi nai .i do cadzu",
    ] {
        let words = segment_words_with_modifiers(source).expect("valid morphology");
        assert!(
            parse_syntax_tree_with_source_and_options(&words, source, &ParseOptions::default(),)
                .is_err(),
            "{source}"
        );
        let parsed = parse(source, "(+unrestricted-free)");
        assert_eq!(boundaries(&parsed).trailers.len(), 1, "{source}");
        assert!(parsed.warnings.iter().any(|warning| {
            warning.kind == ExperimentalConstruct::ExperimentalPreISentenceTrailer
        }));
    }
}

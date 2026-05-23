//! Source-backed syntax AST model and generated tree traversal.

// The syntax AST mirrors the source grammar and v0 constructors.
// Boxing only for enum-size symmetry would obscure that shape during the port.
#![allow(clippy::large_enum_variant)]

use std::fmt;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use jbotci_morphology::{Word, WordKind, WordLike};
use serde::{Deserialize, Serialize};
use vec1::{Vec1, smallvec_v1::SmallVec1};

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WithFreeModifiers<T> {
    pub value: T,
    pub free_modifiers: Vec<FreeModifierSyntax>,
}

#[invariant(true)]
#[invariant(::Bare(_) => true)]
#[invariant(::Emphasized => true)]
#[invariant(::WithIndicator => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WithIndicators<T> {
    Bare(T),
    Emphasized {
        bahe: Word,
        word_like: T,
    },
    WithIndicator {
        base: Box<WithIndicators<T>>,
        indicator: Word,
        nai: Option<Word>,
    },
}

impl<T> WithIndicators<T> {
    #[requires(true)]
    #[ensures(true)]
    pub fn bare(word_like: T) -> Self {
        WithIndicators::Bare(word_like)
    }

    #[requires(bahe.selmaho() == Some("BAhE"))]
    #[ensures(true)]
    pub fn emphasized(bahe: Word, word_like: T) -> Self {
        WithIndicators::Emphasized {
            bahe: bahe,
            word_like: word_like,
        }
    }

    #[requires(crate::is_indicator_word(&indicator))]
    #[requires(nai.as_ref().is_none_or(|nai| nai.is_cmavo_text("nai")))]
    #[ensures(true)]
    pub fn with_indicator(base: WithIndicators<T>, indicator: Word, nai: Option<Word>) -> Self {
        WithIndicators::WithIndicator {
            base: Box::new(base),
            indicator: indicator,
            nai: nai,
        }
    }
}

impl WithIndicators<WordLike> {
    #[requires(true)]
    #[ensures(true)]
    pub fn word_like(&self) -> Option<&WordLike> {
        match self {
            WithIndicators::Bare(word_like) | WithIndicators::Emphasized { word_like, .. } => {
                Some(word_like)
            }
            WithIndicators::WithIndicator { base, .. } => base.word_like(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn visible_word(&self) -> Option<&Word> {
        self.word_like().and_then(WordLike::visible_base_word)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn source_spans(&self) -> Vec<&jbotci_source::SourceSpan> {
        let mut spans = Vec::new();
        self.source_spans_into(&mut spans);
        spans
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn source_spans_into<'a>(&'a self, out: &mut Vec<&'a jbotci_source::SourceSpan>) {
        match self {
            WithIndicators::Bare(word_like) => word_like.source_spans_into(out),
            WithIndicators::Emphasized { bahe, word_like } => {
                out.push(bahe.span());
                word_like.source_spans_into(out);
            }
            WithIndicators::WithIndicator {
                base,
                indicator,
                nai,
            } => {
                base.source_spans_into(out);
                out.push(indicator.span());
                if let Some(nai) = nai {
                    out.push(nai.span());
                }
            }
        }
    }
}

impl<T: fmt::Display> fmt::Display for WithIndicators<T> {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WithIndicators::Bare(word_like) => write!(f, "{word_like}"),
            WithIndicators::Emphasized { bahe, word_like } => {
                write!(f, "{bahe}-{word_like}")
            }
            WithIndicators::WithIndicator {
                base,
                indicator,
                nai,
            } => {
                write!(f, "{base}-{indicator}")?;
                if let Some(nai) = nai {
                    write!(f, "-{nai}")?;
                }
                Ok(())
            }
        }
    }
}

jbotci_tree::tree_model! {
pub type WordRun = SmallVec1<[WithIndicators<WordLike>; 2]>;
pub type MathExpressionVec = Vec1<MathExpressionSyntax>;

#[invariant(indicator.visible_word().is_some_and(crate::is_indicator_word))]
#[invariant(nai.as_ref().is_none_or(|nai| nai.is_cmavo_text("nai")))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Indicator {
    pub indicator: WithIndicators<WordLike>,
    pub nai: Option<Word>,
}

#[invariant(crate::tree::opt_free_cmavo_text(cu, "cu"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PredicateSyntax {
    pub leading_terms: Vec<TermSyntax>,
    pub cu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    #[tree_child(primary)]
    pub predicate_tail: PredicateTailSyntax,
    pub free_modifiers: Vec<FreeModifierSyntax>,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PredicateTailSyntax {
    #[tree_child(primary)]
    pub first: PredicateTail1Syntax,
    pub ke_continuation: Option<Box<KePredicateTailSyntax>>,
}

#[invariant(crate::tree::free_cmavo_text(ke, "ke"))]
#[invariant(crate::tree::opt_free_cmavo_text(kehe, "ke'e"))]
#[invariant(crate::tree::opt_free_cmavo_text(vau, "vau"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KePredicateTailSyntax {
    pub connective: ConnectiveSyntax,
    pub tense_modal: Option<Box<TenseModalSyntax>>,
    pub ke: WithFreeModifiers<WithIndicators<WordLike>>,
    #[tree_child(primary)]
    pub predicate_tail: PredicateTailSyntax,
    pub kehe: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    pub tail_terms: Vec<TermSyntax>,
    pub vau: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    pub free_modifiers: Vec<FreeModifierSyntax>,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PredicateTail1Syntax {
    #[tree_child(primary)]
    pub first: PredicateTail2Syntax,
    pub continuations: Vec<PredicateTailContinuationSyntax>,
}

#[invariant(crate::tree::opt_free_cmavo_text(cu, "cu"))]
#[invariant(crate::tree::opt_free_cmavo_text(vau, "vau"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PredicateTailContinuationSyntax {
    pub connective: ConnectiveSyntax,
    pub tense_modal: Option<Box<TenseModalSyntax>>,
    pub cu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    #[tree_child(primary)]
    pub predicate_tail: PredicateTail2Syntax,
    pub tail_terms: Vec<TermSyntax>,
    pub vau: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    pub free_modifiers: Vec<FreeModifierSyntax>,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PredicateTail2Syntax {
    #[tree_child(primary)]
    pub first: PredicateTail3Syntax,
    pub bo_continuation: Option<Box<BoPredicateTailSyntax>>,
}

#[invariant(crate::tree::free_cmavo_text(bo, "bo"))]
#[invariant(crate::tree::opt_free_cmavo_text(cu, "cu"))]
#[invariant(crate::tree::opt_free_cmavo_text(vau, "vau"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BoPredicateTailSyntax {
    pub connective: ConnectiveSyntax,
    pub tense_modal: Option<Box<TenseModalSyntax>>,
    pub bo: WithFreeModifiers<WithIndicators<WordLike>>,
    pub cu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    #[tree_child(primary)]
    pub predicate_tail: PredicateTail2Syntax,
    pub tail_terms: Vec<TermSyntax>,
    pub vau: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    pub free_modifiers: Vec<FreeModifierSyntax>,
}

#[invariant(true)]
#[invariant(::Relation => crate::tree::opt_free_cmavo_text(vau, "vau"))]
#[invariant(::GekSentence(..) => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum PredicateTail3Syntax {
    Relation {
        #[tree_child(primary)]
        relation: RelationSyntax,
        terms: Vec<TermSyntax>,
        vau: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    GekSentence(GekSentenceSyntax),
}

#[invariant(true)]
#[invariant(::Pair => crate::tree::opt_gihi(gihi) && crate::tree::opt_free_cmavo_text(vau, "vau"))]
#[invariant(::Ke => crate::tree::free_cmavo_text(ke, "ke") && crate::tree::opt_free_cmavo_text(kehe, "ke'e"))]
#[invariant(::Na => crate::tree::free_cmavo_label(na, "NA", &["na", "ja'a"]))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum GekSentenceSyntax {
    Pair {
        gek: ConnectiveSyntax,
        first: Box<SubsentenceSyntax>,
        gik: ConnectiveSyntax,
        second: Box<SubsentenceSyntax>,
        gihi: Option<WithIndicators<WordLike>>,
        tail_terms: Vec<TermSyntax>,
        vau: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Ke {
        tense_modal: Option<Box<TenseModalSyntax>>,
        ke: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner: Box<GekSentenceSyntax>,
        kehe: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Na {
        na: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner: Box<GekSentenceSyntax>,
    },
}

#[invariant(true)]
#[invariant(::Plain(..) => true)]
#[invariant(::Prenex => crate::tree::free_cmavo_text(zohu, "zo'u"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum SubsentenceSyntax {
    Plain(PredicateSyntax),
    Prenex {
        prenex_terms: Vec<TermSyntax>,
        zohu: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner_subsentence: Box<SubsentenceSyntax>,
    },
}

#[invariant(leading_nai.iter().all(|nai| crate::tree::wi_cmavo_text(nai, "nai")))]
#[invariant(leading_cmevla.iter().all(crate::grammar::tokens::is_cmevla_word))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TextSyntax {
    pub leading_nai: Vec<WithIndicators<WordLike>>,
    pub leading_cmevla: Vec<WithIndicators<WordLike>>,
    pub leading_indicators: Vec<Indicator>,
    pub leading_free_modifiers: Vec<FreeModifierSyntax>,
    pub leading_connective: Option<Box<ConnectiveSyntax>>,
    #[tree_child(primary)]
    pub paragraphs: Vec<ParagraphSyntax>,
}

#[invariant(crate::tree::opt_wi_cmavo_text(i, "i"))]
#[invariant(niho.iter().all(|niho| crate::tree::wi_cmavo_label(niho, "NIhO", &["ni'o", "no'i"])))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ParagraphSyntax {
    pub i: Option<WithIndicators<WordLike>>,
    pub niho: Vec<WithIndicators<WordLike>>,
    pub free_modifiers: Vec<FreeModifierSyntax>,
    #[tree_child(primary)]
    pub statements: Vec<ParagraphStatementSyntax>,
}

#[invariant(crate::tree::opt_wi_cmavo_text(i, "i"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ParagraphStatementSyntax {
    pub i: Option<WithIndicators<WordLike>>,
    pub connective: Option<Box<ConnectiveSyntax>>,
    pub free_modifiers: Vec<FreeModifierSyntax>,
    #[tree_child(primary)]
    pub statement: Option<Box<StatementSyntax>>,
}

#[invariant(true)]
#[invariant(::Sei => crate::tree::free_cmavo_label(sei, "SEI", &["sei", "ti'o", "xoi"]) && crate::tree::opt_free_cmavo_text(cu, "cu") && crate::tree::opt_free_cmavo_text(sehu, "se'u"))]
#[invariant(::To => crate::tree::free_cmavo_label(to, "TO", &["to'i", "to"]) && crate::tree::opt_free_cmavo_text(toi, "toi"))]
#[invariant(::Xi => crate::tree::free_cmavo_label(xi, "XI", &["xi", "te'ai"]))]
#[invariant(::Mai => crate::tree::word_run_number_or_letter(number) && crate::tree::free_cmavo_label(mai, "MAI", crate::grammar::tokens::MAI_WORDS))]
#[invariant(::Soi => crate::tree::free_cmavo_label(soi, "SOI", &["soi", "xoi"]) && crate::tree::opt_free_cmavo_text(sehu, "se'u"))]
#[invariant(::Vocative => crate::tree::free_words_vocative_markers(vocative_markers) && crate::tree::opt_free_cmavo_text(dohu, "do'u"))]
#[invariant(::Replacement => crate::tree::opt_wi_cmavo_text(lohai, "lo'ai") && crate::tree::opt_wi_cmavo_text(sahai, "sa'ai") && crate::tree::free_cmavo_text(lehai, "le'ai"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum FreeModifierSyntax {
    Sei {
        sei: WithFreeModifiers<WithIndicators<WordLike>>,
        terms: Vec<TermSyntax>,
        cu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        relation: RelationSyntax,
        sehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    To {
        to: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        text: Box<TextSyntax>,
        toi: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Xi {
        xi: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        expression: MathExpressionSyntax,
    },
    Mai {
        number: WordRun,
        mai: WithFreeModifiers<WithIndicators<WordLike>>,
    },
    Soi {
        soi: WithFreeModifiers<WithIndicators<WordLike>>,
        leading_argument: Box<ArgumentSyntax>,
        trailing_argument: Option<Box<ArgumentSyntax>>,
        sehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Vocative {
        vocative_markers: WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
        argument: Option<Box<ArgumentSyntax>>,
        dohu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Replacement {
        lohai: Option<WithIndicators<WordLike>>,
        old_words: Vec<WithIndicators<WordLike>>,
        sahai: Option<WithIndicators<WordLike>>,
        new_words: Vec<WithIndicators<WordLike>>,
        lehai: WithFreeModifiers<WithIndicators<WordLike>>,
    },
}

#[invariant(true)]
#[invariant(::Tuhe => crate::tree::free_cmavo_text(tuhe, "tu'e") && crate::tree::opt_free_cmavo_text(tuhu, "tu'u"))]
#[invariant(::Prenex => crate::tree::free_cmavo_text(zohu, "zo'u"))]
#[invariant(::Predicate(..) => true)]
#[invariant(::Connected => crate::tree::wi_cmavo_text(i, "i"))]
#[invariant(::PreIConnected => crate::tree::wi_cmavo_text(i, "i"))]
#[invariant(::Iau => crate::tree::free_cmavo_text(iau, "i'au"))]
#[invariant(::ExperimentalPredicateContinuation => true)]
#[invariant(::Fragment(..) => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum StatementSyntax {
    Tuhe {
        tense_modal: Option<Box<TenseModalSyntax>>,
        tuhe: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        text: Box<TextSyntax>,
        tuhu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Prenex {
        prenex_terms: Vec<TermSyntax>,
        zohu: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner_statement: Box<StatementSyntax>,
    },
    Predicate(PredicateSyntax),
    Connected {
        leading_statement: Box<StatementSyntax>,
        i: WithIndicators<WordLike>,
        connective: ConnectiveSyntax,
        trailing_statement: Box<StatementSyntax>,
    },
    PreIConnected {
        leading_statement: Box<StatementSyntax>,
        connective: ConnectiveSyntax,
        i: WithIndicators<WordLike>,
        trailing_statement: Box<StatementSyntax>,
    },
    Iau {
        #[tree_child(primary)]
        inner_statement: Box<StatementSyntax>,
        iau: WithFreeModifiers<WithIndicators<WordLike>>,
        reset_terms: Vec<TermSyntax>,
    },
    ExperimentalPredicateContinuation {
        leading_statement: Box<StatementSyntax>,
        continuation: PredicateStatementContinuationSyntax,
    },
    Fragment(FragmentSyntax),
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PredicateStatementContinuationSyntax {
    pub connective: ConnectiveSyntax,
    pub tense_modal: Option<Box<TenseModalSyntax>>,
    pub marker: PredicateStatementContinuationMarkerSyntax,
    pub trailing_subsentence: SubsentenceSyntax,
}

#[invariant(true)]
#[invariant(::Bo(bo) => crate::tree::free_cmavo_text(bo, "bo"))]
#[invariant(::Ke => crate::tree::free_cmavo_text(ke, "ke") && crate::tree::opt_free_cmavo_text(kehe, "ke'e"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum PredicateStatementContinuationMarkerSyntax {
    Bo(WithFreeModifiers<WithIndicators<WordLike>>),
    Ke {
        ke: WithFreeModifiers<WithIndicators<WordLike>>,
        kehe: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
}

#[invariant(true)]
#[invariant(::Ek(..) => true)]
#[invariant(::Gihek(..) => true)]
#[invariant(::Other(words) => !words.value.is_empty())]
#[invariant(::Ijek => crate::tree::wi_cmavo_text(i, "i"))]
#[invariant(::Prenex => crate::tree::free_cmavo_text(zohu, "zo'u"))]
#[invariant(::BeLink => crate::tree::free_cmavo_text(be, "be") && crate::tree::opt_free_cmavo_label(fa, "FA", crate::grammar::tokens::FA_WORDS) && (fa.is_none() || first_argument.is_some()) && crate::tree::opt_free_cmavo_text(beho, "be'o"))]
#[invariant(::BeiLink(bei_only_links) => !bei_only_links.is_empty())]
#[invariant(::RelativeClause(relative_clauses) => !relative_clauses.is_empty())]
#[invariant(::MathExpression(..) => true)]
#[invariant(::Term => crate::tree::opt_free_cmavo_text(vau, "vau"))]
#[invariant(::Relation(..) => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum FragmentSyntax {
    Ek(ConnectiveSyntax),
    Gihek(ConnectiveSyntax),
    Other(WithFreeModifiers<Vec<WithIndicators<WordLike>>>),
    Ijek {
        i: WithIndicators<WordLike>,
        connective: ConnectiveSyntax,
    },
    Prenex {
        terms: Vec<TermSyntax>,
        zohu: WithFreeModifiers<WithIndicators<WordLike>>,
    },
    BeLink {
        be: WithFreeModifiers<WithIndicators<WordLike>>,
        fa: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        first_argument: Option<Box<ArgumentSyntax>>,
        bei_links: Vec<BeiLinkSyntax>,
        beho: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    BeiLink(Vec<BeiLinkSyntax>),
    RelativeClause(Vec<RelativeClauseSyntax>),
    MathExpression(MathExpressionSyntax),
    Term {
        terms: Vec<TermSyntax>,
        vau: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Relation(RelationSyntax),
}

#[invariant(true)]
#[invariant(::NuhiTermset => crate::tree::free_cmavo_text(nuhi, "nu'i") && !termset.is_empty() && crate::tree::opt_free_cmavo_text(nuhu, "nu'u"))]
#[invariant(::GekNuhiTermset => m_nuhi.as_ref().is_none_or(|nuhi| crate::tree::free_cmavo_text(nuhi, "nu'i")) && !terms.is_empty() && crate::tree::opt_free_cmavo_text(nuhu, "nu'u") && !gik_terms.is_empty() && crate::tree::opt_gihi(gihi) && crate::tree::opt_free_cmavo_text(gik_nuhu, "nu'u"))]
#[invariant(::Cehe => !leading_terms.is_empty() && crate::tree::free_cmavo_text(cehe, "ce'e") && !trailing_terms.is_empty())]
#[invariant(::Pehe => !leading_terms.is_empty() && crate::tree::free_cmavo_text(pehe, "pe'e") && !trailing_terms.is_empty())]
#[invariant(::Argument(..) => true)]
#[invariant(::Fa => crate::tree::free_cmavo_label(fa, "FA", crate::grammar::tokens::FA_WORDS) && crate::tree::opt_free_cmavo_text(ku, "ku"))]
#[invariant(::NaKu => crate::tree::wi_cmavo_label(na, "NA", &["na", "ja'a"]) && crate::tree::free_cmavo_text(na_ku, "ku"))]
#[invariant(::BareNa(na) => crate::tree::free_cmavo_label(na, "NA", &["na", "ja'a"]))]
#[invariant(::NoihaAdverbial => crate::tree::free_cmavo_label(noiha, "NOIhA", &["noi'a", "poi'a", "poi'o'a", "soi'a", "noi'o'a"]) && crate::tree::opt_free_cmavo_text(fehu, "fe'u"))]
#[invariant(::PoihaBrigahi => crate::tree::free_cmavo_label(poiha, "NOIhA", &["noi'a", "poi'a", "poi'o'a", "soi'a", "noi'o'a"]) && crate::tree::free_cmavo_text(brigahi_ku, "ku"))]
#[invariant(::FihoiAdverbial => crate::tree::free_cmavo_text(fihoi, "fi'oi") && crate::tree::opt_free_cmavo_text(fihau, "fi'au"))]
#[invariant(::SoiAdverbial => crate::tree::free_cmavo_label(soi, "SOI", &["soi", "xoi"]) && crate::tree::opt_free_cmavo_text(sehu, "se'u"))]
#[invariant(::JaiTagged => crate::tree::free_cmavo_text(jai, "jai"))]
#[invariant(::Tagged => tense_modal.is_some())]
#[invariant(::Connected => !leading_terms.is_empty() && !trailing_terms.is_empty())]
#[invariant(::BoConnected => !leading_terms.is_empty() && crate::tree::free_cmavo_text(bo, "bo"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum TermSyntax {
    NuhiTermset {
        nuhi: WithFreeModifiers<WithIndicators<WordLike>>,
        termset: Vec<TermSyntax>,
        nuhu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    GekNuhiTermset {
        m_nuhi: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        gek: ConnectiveSyntax,
        terms: Vec<TermSyntax>,
        nuhu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        gik: ConnectiveSyntax,
        gik_terms: Vec<TermSyntax>,
        gihi: Option<WithIndicators<WordLike>>,
        gik_nuhu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Cehe {
        leading_terms: Vec<TermSyntax>,
        cehe: WithFreeModifiers<WithIndicators<WordLike>>,
        trailing_terms: Vec<TermSyntax>,
    },
    Pehe {
        leading_terms: Vec<TermSyntax>,
        pehe: WithFreeModifiers<WithIndicators<WordLike>>,
        connective: ConnectiveSyntax,
        trailing_terms: Vec<TermSyntax>,
    },
    Argument(ArgumentSyntax),
    Fa {
        fa: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        argument: ArgumentSyntax,
        ku: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    NaKu {
        na: WithIndicators<WordLike>,
        na_ku: WithFreeModifiers<WithIndicators<WordLike>>,
    },
    BareNa(WithFreeModifiers<WithIndicators<WordLike>>),
    NoihaAdverbial {
        noiha: WithFreeModifiers<WithIndicators<WordLike>>,
        tail_elements: Vec<ArgumentTailElementSyntax>,
        relation: Option<Box<RelationSyntax>>,
        relative_clauses: Vec<RelativeClauseSyntax>,
        fehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    PoihaBrigahi {
        poiha: WithFreeModifiers<WithIndicators<WordLike>>,
        tail_elements: Vec<ArgumentTailElementSyntax>,
        relation: Option<Box<RelationSyntax>>,
        relative_clauses: Vec<RelativeClauseSyntax>,
        brigahi_ku: WithFreeModifiers<WithIndicators<WordLike>>,
    },
    FihoiAdverbial {
        fihoi: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        subsentence: Box<SubsentenceSyntax>,
        fihau: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    SoiAdverbial {
        soi: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        subsentence: Box<SubsentenceSyntax>,
        sehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    JaiTagged {
        jai: WithFreeModifiers<WithIndicators<WordLike>>,
        tag: Option<Box<TenseModalSyntax>>,
        #[tree_child(primary)]
        argument: ArgumentSyntax,
    },
    Tagged {
        tense_modal: Option<Box<TenseModalSyntax>>,
        #[tree_child(primary)]
        argument: ArgumentSyntax,
    },
    Connected {
        leading_terms: Vec<TermSyntax>,
        connective: ConnectiveSyntax,
        trailing_terms: Vec<TermSyntax>,
    },
    BoConnected {
        leading_terms: Vec<TermSyntax>,
        bo_connective: Option<Box<ConnectiveSyntax>>,
        tense_modal: Option<Box<TenseModalSyntax>>,
        bo: WithFreeModifiers<WithIndicators<WordLike>>,
        trailing_term: Box<TermSyntax>,
    },
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum TermWrapperKindSyntax {
    Lahe,
    NaheBo,
    Nahe,
}

#[invariant(true)]
#[invariant(::TenseModal(..) => true)]
#[invariant(::Fa(fa) => crate::tree::free_cmavo_label(fa, "FA", crate::grammar::tokens::FA_WORDS))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ArgumentTagSyntax {
    TenseModal(TenseModalSyntax),
    Fa(WithFreeModifiers<WithIndicators<WordLike>>),
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ArgumentConnectionSyntax {
    pub connective: ConnectiveSyntax,
    #[tree_child(primary)]
    pub argument: Box<ArgumentSyntax>,
}

#[invariant(true)]
#[invariant(::Quote(..) => true)]
#[invariant(::MathExpression => crate::tree::free_cmavo_label(li, "LI", &["li", "me'o"]) && crate::tree::opt_free_cmavo_text(loho, "lo'o"))]
#[invariant(::Letter => crate::tree::free_word_run_number_or_letter(letter) && crate::tree::opt_free_cmavo_text(boi, "boi"))]
#[invariant(::Quantified => true)]
#[invariant(::RelativeClause => crate::tree::opt_free_cmavo_text(vuho, "vu'o") && !relative_clauses.is_empty())]
#[invariant(::Vuho => crate::tree::free_cmavo_text(vuho_marker, "vu'o") && (!relative_clauses.is_empty() || connected_argument.is_some()))]
#[invariant(::BridiDescription => crate::tree::free_cmavo_label(lohoi, "LOhOI", &["lo'oi", "mau'a", "xau'a"]) && crate::tree::opt_free_cmavo_text(kuhau, "ku'au"))]
#[invariant(::NaKu => crate::tree::wi_cmavo_label(na, "NA", &["na", "ja'a"]) && crate::tree::free_cmavo_text(ku, "ku"))]
#[invariant(::Tagged => true)]
#[invariant(::NaheBo => crate::tree::wi_cmavo_label(nahe, "NAhE", &["na'e", "to'e", "no'e", "je'a"]) && crate::tree::free_cmavo_text(bo, "bo") && crate::tree::opt_free_cmavo_text(luhu, "lu'u"))]
#[invariant(::Nahe => crate::tree::free_cmavo_label(nahe, "NAhE", &["na'e", "to'e", "no'e", "je'a"]) && crate::tree::opt_free_cmavo_text(luhu, "lu'u"))]
#[invariant(::TermWrapped => crate::tree::term_wrapper_is_valid(term_wrapper_kind, wrapper, wrapper_bo) && crate::tree::opt_free_cmavo_text(luhu, "lu'u"))]
#[invariant(::Koha(koha) => crate::grammar::tokens::is_koha_argument(&koha.value))]
#[invariant(::Zohe => crate::tree::opt_free_cmavo_text(maybe_ku, "ku"))]
#[invariant(::Lahe => crate::tree::free_cmavo_label(lahe, "LAhE", &["tu'a", "lu'a", "lu'o", "la'e", "vu'i", "lu'i", "lu'e"]) && crate::tree::opt_free_cmavo_text(luhu, "lu'u"))]
#[invariant(::Connected => true)]
#[invariant(::Ke => crate::tree::free_cmavo_text(ke, "ke") && crate::tree::opt_free_cmavo_text(kehe, "ke'e"))]
#[invariant(::Bo => crate::tree::free_cmavo_text(bo, "bo"))]
#[invariant(::Gek => crate::tree::opt_gihi(gihi))]
#[invariant(::Descriptor(descriptor) => crate::tree::descriptor_is_valid(descriptor))]
#[invariant(::ConnectedDescriptor(descriptor) => crate::tree::connected_descriptor_is_valid(descriptor))]
#[invariant(::Name => crate::tree::free_cmavo_label(la, "LA", &["lai", "la'i", "la"]) && crate::tree::free_word_run_cmevla(names))]
#[invariant(::Cmevla(names) => crate::tree::free_word_run_cmevla(names))]
#[invariant(::RelationVocative => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ArgumentSyntax {
    Quote(QuoteSyntax),
    MathExpression {
        li: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        expression: MathExpressionSyntax,
        loho: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Letter {
        letter: WithFreeModifiers<WordRun>,
        boi: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Quantified {
        quantifier: QuantifierSyntax,
        #[tree_child(primary)]
        inner_argument: Box<ArgumentSyntax>,
    },
    RelativeClause {
        base_argument: Box<ArgumentSyntax>,
        vuho: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        relative_clauses: Vec<RelativeClauseSyntax>,
    },
    Vuho {
        base_argument: Box<ArgumentSyntax>,
        vuho_marker: WithFreeModifiers<WithIndicators<WordLike>>,
        relative_clauses: Vec<RelativeClauseSyntax>,
        connected_argument: Option<Box<ArgumentConnectionSyntax>>,
    },
    BridiDescription {
        lohoi: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        subsentence: Box<SubsentenceSyntax>,
        kuhau: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    NaKu {
        na: WithIndicators<WordLike>,
        ku: WithFreeModifiers<WithIndicators<WordLike>>,
    },
    Tagged {
        tag: ArgumentTagSyntax,
        #[tree_child(primary)]
        inner_argument: Box<ArgumentSyntax>,
    },
    NaheBo {
        nahe: WithIndicators<WordLike>,
        bo: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner_argument: Box<ArgumentSyntax>,
        luhu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Nahe {
        nahe: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner_argument: Box<ArgumentSyntax>,
        luhu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    TermWrapped {
        term_wrapper_kind: TermWrapperKindSyntax,
        wrapper: WithFreeModifiers<WithIndicators<WordLike>>,
        wrapper_bo: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        #[tree_child(primary)]
        inner_term: Box<TermSyntax>,
        luhu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Koha(WithFreeModifiers<WithIndicators<WordLike>>),
    Zohe {
        tag: Option<Box<ArgumentTagSyntax>>,
        maybe_ku: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        free_modifiers: Vec<FreeModifierSyntax>,
    },
    Lahe {
        lahe: WithFreeModifiers<WithIndicators<WordLike>>,
        relative_clauses: Vec<RelativeClauseSyntax>,
        #[tree_child(primary)]
        inner_argument: Box<ArgumentSyntax>,
        luhu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Connected {
        leading_argument: Box<ArgumentSyntax>,
        connective: ConnectiveSyntax,
        trailing_argument: Box<ArgumentSyntax>,
    },
    Ke {
        ke: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner_argument: Box<ArgumentSyntax>,
        kehe: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Bo {
        leading_argument: Box<ArgumentSyntax>,
        bo_connective: Option<Box<ConnectiveSyntax>>,
        bo_tense_modal: Option<Box<TenseModalSyntax>>,
        bo: WithFreeModifiers<WithIndicators<WordLike>>,
        trailing_argument: Box<ArgumentSyntax>,
    },
    Gek {
        gek: ConnectiveSyntax,
        leading_argument: Box<ArgumentSyntax>,
        gik: ConnectiveSyntax,
        trailing_argument: Box<ArgumentSyntax>,
        gihi: Option<WithIndicators<WordLike>>,
    },
    Descriptor(DescriptorSyntax),
    ConnectedDescriptor(ConnectedDescriptorSyntax),
    Name {
        la: WithFreeModifiers<WithIndicators<WordLike>>,
        names: WithFreeModifiers<WordRun>,
    },
    Cmevla(WithFreeModifiers<WordRun>),
    RelationVocative {
        leading_relative_clauses: Vec<RelativeClauseSyntax>,
        relation: RelationSyntax,
        trailing_relative_clauses: Vec<RelativeClauseSyntax>,
    },
}

#[invariant(true)]
#[invariant(::Goi(goi) => crate::tree::goi_relative_clause_is_valid(goi))]
#[invariant(::Noi => crate::tree::free_cmavo_label(noi, "NOI", &["noi", "voi"]) && crate::tree::opt_free_cmavo_text(kuho, "ku'o"))]
#[invariant(::Poi => crate::tree::free_cmavo_text(poi, "poi") && crate::tree::opt_free_cmavo_text(kuho, "ku'o"))]
#[invariant(::Zihe => crate::tree::free_cmavo_text(zihe, "zi'e"))]
#[invariant(::Connected => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum RelativeClauseSyntax {
    Goi(GoiRelativeClauseSyntax),
    Noi {
        noi: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        subsentence: SubsentenceSyntax,
        kuho: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Poi {
        poi: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        subsentence: SubsentenceSyntax,
        kuho: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Zihe {
        zihe: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner: Box<RelativeClauseSyntax>,
    },
    Connected {
        connective: ConnectiveSyntax,
        #[tree_child(primary)]
        inner: Box<RelativeClauseSyntax>,
    },
}

#[invariant(crate::tree::free_cmavo_label(
    goi,
    "GOI",
    &["pe", "ne", "po", "po'e", "po'u", "no'u", "goi"],
))]
#[invariant(crate::tree::opt_free_cmavo_text(gehu, "ge'u"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GoiRelativeClauseSyntax {
    pub goi: WithFreeModifiers<WithIndicators<WordLike>>,
    #[tree_child(primary)]
    pub argument: ArgumentSyntax,
    pub gehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
}

#[invariant(crate::tree::free_cmavo_text(nohoi, "no'oi"))]
#[invariant(crate::tree::opt_free_cmavo_text(kuhoi, "ku'oi"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SelbriRelativeClauseSyntax {
    pub nohoi: WithFreeModifiers<WithIndicators<WordLike>>,
    #[tree_child(primary)]
    pub relation: RelationSyntax,
    pub kuhoi: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
}

#[invariant(true)]
#[invariant(::Lu => crate::tree::free_cmavo_text(lu, "lu") && crate::tree::opt_free_cmavo_text(lihu, "li'u"))]
#[invariant(::Zo(zo) => crate::tree::free_cmavo_text(zo, "zo"))]
#[invariant(::ZohOi(zohoi) => crate::tree::free_cmavo_any(zohoi, &["zo'oi", "la'oi", "ra'oi", "me'oi", "go'oi"]))]
#[invariant(::Zoi(zoi) => crate::tree::free_cmavo_label(zoi, "ZOI", &["zoi", "la'o", "mu'oi"]))]
#[invariant(::Lohu(lohu) => crate::tree::free_cmavo_text(lohu, "lo'u"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum QuoteSyntax {
    Lu {
        lu: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        text: TextSyntax,
        lihu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Zo(WithFreeModifiers<WithIndicators<WordLike>>),
    ZohOi(WithFreeModifiers<WithIndicators<WordLike>>),
    Zoi(WithFreeModifiers<WithIndicators<WordLike>>),
    Lohu(WithFreeModifiers<WithIndicators<WordLike>>),
}

#[invariant(descriptor.as_ref().is_none_or(crate::tree::descriptor_marker_is_valid))]
#[invariant(crate::tree::opt_free_cmavo_text(ku, "ku"))]
#[invariant(descriptor.is_some() || (!tail_elements.is_empty() && relation.is_some()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DescriptorSyntax {
    pub descriptor: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    pub outer_quantifier: Option<Box<QuantifierSyntax>>,
    pub tail_elements: Vec<ArgumentTailElementSyntax>,
    pub relation: Option<Box<RelationSyntax>>,
    pub relative_clauses: Vec<RelativeClauseSyntax>,
    pub ku: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
}

#[invariant(crate::tree::descriptor_marker_is_valid(descriptor))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DescriptorHeadSyntax {
    pub descriptor: WithFreeModifiers<WithIndicators<WordLike>>,
}

#[invariant(crate::tree::opt_free_cmavo_text(ku, "ku"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConnectedDescriptorSyntax {
    pub leading_descriptor_head: DescriptorHeadSyntax,
    pub connective: ConnectiveSyntax,
    pub trailing_descriptor_head: DescriptorHeadSyntax,
    pub tail_elements: Vec<ArgumentTailElementSyntax>,
    pub relation: Option<Box<RelationSyntax>>,
    pub relative_clauses: Vec<RelativeClauseSyntax>,
    pub ku: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
}

#[invariant(true)]
#[invariant(::Afterthought => crate::tree::connective_parts_are_valid(se, nahe, na, cmavo, nai, crate::tree::ConnectiveKind::Afterthought))]
#[invariant(::Relation => crate::tree::connective_parts_are_valid(se, nahe, na, cmavo, nai, crate::tree::ConnectiveKind::Relation))]
#[invariant(::PredicateTail => crate::tree::connective_parts_are_valid(se, nahe, na, cmavo, nai, crate::tree::ConnectiveKind::PredicateTail))]
#[invariant(::Forethought => crate::tree::connective_parts_are_valid(se, nahe, na, cmavo, nai, crate::tree::ConnectiveKind::Forethought))]
#[invariant(::NonLogical => crate::tree::connective_parts_are_valid(se, nahe, na, cmavo, nai, crate::tree::ConnectiveKind::NonLogical))]
#[invariant(::Interval => crate::tree::connective_parts_are_valid(se, nahe, na, cmavo, nai, crate::tree::ConnectiveKind::Interval))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ConnectiveSyntax {
    Afterthought {
        se: Option<WithIndicators<WordLike>>,
        nahe: Option<WithIndicators<WordLike>>,
        na: Option<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        cmavo: WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
        nai: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Relation {
        se: Option<WithIndicators<WordLike>>,
        nahe: Option<WithIndicators<WordLike>>,
        na: Option<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        cmavo: WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
        nai: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    PredicateTail {
        se: Option<WithIndicators<WordLike>>,
        nahe: Option<WithIndicators<WordLike>>,
        na: Option<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        cmavo: WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
        nai: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Forethought {
        se: Option<WithIndicators<WordLike>>,
        nahe: Option<WithIndicators<WordLike>>,
        na: Option<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        cmavo: WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
        nai: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    NonLogical {
        se: Option<WithIndicators<WordLike>>,
        nahe: Option<WithIndicators<WordLike>>,
        na: Option<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        cmavo: WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
        nai: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Interval {
        se: Option<WithIndicators<WordLike>>,
        nahe: Option<WithIndicators<WordLike>>,
        na: Option<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        cmavo: WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
        nai: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
}

#[invariant(crate::tree::free_cmavo_text(bei, "bei"))]
#[invariant(fa.is_none() || argument.is_some(), "lifted FA link tags must have an argument")]
#[invariant(crate::tree::opt_free_cmavo_label(fa, "FA", crate::grammar::tokens::FA_WORDS))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BeiLinkSyntax {
    pub bei: WithFreeModifiers<WithIndicators<WordLike>>,
    pub fa: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    pub argument: Option<Box<ArgumentSyntax>>,
}

#[invariant(fa.is_none() || argument.is_some(), "lifted FA link tags must have an argument")]
#[invariant(fa.as_ref().is_none_or(|fa| fa.value.visible_word().is_some_and(|word| word.selmaho() == Some("FA"))))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LinkArgumentSyntax {
    pub fa: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    pub argument: Option<Box<ArgumentSyntax>>,
}

#[invariant(be.value.visible_word().is_some_and(|word| word.is_cmavo_text("be")))]
#[invariant(fa.is_none() || first_argument.is_some(), "lifted FA link tags must have an argument")]
#[invariant(fa.as_ref().is_none_or(|fa| fa.value.visible_word().is_some_and(|word| word.selmaho() == Some("FA"))))]
#[invariant(beho.as_ref().is_none_or(|beho| beho.value.visible_word().is_some_and(|word| word.is_cmavo_text("be'o"))))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BeLinkSyntax {
    pub be: WithFreeModifiers<WithIndicators<WordLike>>,
    pub fa: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    pub first_argument: Option<Box<ArgumentSyntax>>,
    pub bei_links: Vec<BeiLinkSyntax>,
    pub beho: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ConnectiveKind {
    Afterthought,
    Relation,
    PredicateTail,
    Forethought,
    NonLogical,
    Interval,
}

#[invariant(true)]
#[invariant(::Argument(..) => true)]
#[invariant(::RelativeClauses(relative_clauses) => !relative_clauses.is_empty())]
#[invariant(::Quantifier(..) => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ArgumentTailElementSyntax {
    Argument(Box<ArgumentSyntax>),
    RelativeClauses(Vec<RelativeClauseSyntax>),
    Quantifier(QuantifierSyntax),
}

#[invariant(true)]
#[invariant(::Number => crate::tree::free_word_run_number_or_letter(number) && crate::tree::opt_free_cmavo_text(boi, "boi"))]
#[invariant(::Vei => crate::tree::free_cmavo_text(vei, "vei") && crate::tree::opt_free_cmavo_text(veho, "ve'o"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum QuantifierSyntax {
    Number {
        #[tree_child(primary)]
        number: WithFreeModifiers<WordRun>,
        boi: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Vei {
        vei: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        math_expression: Box<MathExpressionSyntax>,
        veho: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
}

#[invariant(true)]
#[invariant(::Number(..) => true)]
#[invariant(::Letter => crate::tree::free_word_run_number_or_letter(letter) && crate::tree::opt_free_cmavo_text(boi, "boi"))]
#[invariant(::Vei => crate::tree::free_cmavo_text(vei, "vei") && crate::tree::opt_free_cmavo_text(veho, "ve'o"))]
#[invariant(::Gek => true)]
#[invariant(::Forethought => peho.as_ref().is_none_or(|peho| crate::tree::free_cmavo_text(peho, "pe'o")) && !operands.is_empty() && crate::tree::opt_free_cmavo_text(kuhe, "ku'e"))]
#[invariant(::ReversePolish => crate::tree::free_cmavo_text(fuha, "fu'a") && !operands.is_empty())]
#[invariant(::Nihe => crate::tree::free_cmavo_text(nihe, "ni'e") && crate::tree::opt_free_cmavo_text(tehu, "te'u"))]
#[invariant(::Mohe => crate::tree::free_cmavo_text(mohe, "mo'e") && crate::tree::opt_free_cmavo_text(tehu, "te'u"))]
#[invariant(::Johi => crate::tree::free_cmavo_text(johi, "jo'i") && crate::tree::opt_free_cmavo_text(tehu, "te'u"))]
#[invariant(::Lahe => crate::tree::free_nahe_bo_markers(markers) && crate::tree::opt_free_cmavo_text(luhu, "lu'u"))]
#[invariant(::Connected => true)]
#[invariant(::Binary => true)]
#[invariant(::Bihe => crate::tree::free_cmavo_text(bihe, "bi'e"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum MathExpressionSyntax {
    Number(QuantifierSyntax),
    Letter {
        #[tree_child(primary)]
        letter: WithFreeModifiers<WordRun>,
        boi: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Vei {
        vei: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner_expression: Box<MathExpressionSyntax>,
        veho: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Gek {
        gek: ConnectiveSyntax,
        left_expression: Box<MathExpressionSyntax>,
        gik: ConnectiveSyntax,
        right_expression: Box<MathExpressionSyntax>,
    },
    Forethought {
        peho: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        operator: MathOperatorSyntax,
        operands: Vec<MathExpressionSyntax>,
        kuhe: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    ReversePolish {
        fuha: WithFreeModifiers<WithIndicators<WordLike>>,
        operands: Vec<MathExpressionSyntax>,
        operators: Vec<MathOperatorSyntax>,
    },
    Nihe {
        nihe: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        relation: RelationSyntax,
        tehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Mohe {
        mohe: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        argument: Box<ArgumentSyntax>,
        tehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Johi {
        johi: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        expressions: MathExpressionVec,
        tehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Lahe {
        markers: WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
        #[tree_child(primary)]
        inner_expression: Box<MathExpressionSyntax>,
        luhu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Connected {
        left_expression: Box<MathExpressionSyntax>,
        connective: ConnectiveSyntax,
        right_expression: Box<MathExpressionSyntax>,
    },
    Binary {
        left_expression: Box<MathExpressionSyntax>,
        operator: MathOperatorSyntax,
        right_expression: Box<MathExpressionSyntax>,
    },
    Bihe {
        left_expression: Box<MathExpressionSyntax>,
        bihe: WithFreeModifiers<WithIndicators<WordLike>>,
        operator: MathOperatorSyntax,
        right_expression: Box<MathExpressionSyntax>,
    },
}

#[invariant(true)]
#[invariant(::Vuhu(vuhu) => crate::tree::free_cmavo_label(vuhu, "VUhU", crate::grammar::tokens::VUHU_WORDS))]
#[invariant(::Maho => crate::tree::free_cmavo_text(maho, "ma'o") && crate::tree::opt_free_cmavo_text(tehu, "te'u"))]
#[invariant(::Se => crate::tree::free_cmavo_label(se, "SE", &["se", "te", "ve", "xe"]))]
#[invariant(::Nahe => crate::tree::free_cmavo_label(nahe, "NAhE", &["na'e", "to'e", "no'e", "je'a"]))]
#[invariant(::Nahu => crate::tree::free_cmavo_text(nahu, "na'u") && crate::tree::opt_free_cmavo_text(tehu, "te'u"))]
#[invariant(::Ke => crate::tree::free_cmavo_text(ke, "ke") && crate::tree::opt_free_cmavo_text(kehe, "ke'e"))]
#[invariant(::Bo => crate::tree::free_cmavo_text(bo, "bo"))]
#[invariant(::Connected => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum MathOperatorSyntax {
    Vuhu(WithFreeModifiers<WithIndicators<WordLike>>),
    Maho {
        maho: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        math_expression: Box<MathExpressionSyntax>,
        tehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Se {
        se: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner_operator: Box<MathOperatorSyntax>,
    },
    Nahe {
        nahe: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner_operator: Box<MathOperatorSyntax>,
    },
    Nahu {
        nahu: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        relation: RelationSyntax,
        tehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Ke {
        ke: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner_operator: Box<MathOperatorSyntax>,
        kehe: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Bo {
        left_operator: Box<MathOperatorSyntax>,
        connective: ConnectiveSyntax,
        bo: WithFreeModifiers<WithIndicators<WordLike>>,
        right_operator: Box<MathOperatorSyntax>,
    },
    Connected {
        left_operator: Box<MathOperatorSyntax>,
        connective: ConnectiveSyntax,
        right_operator: Box<MathOperatorSyntax>,
    },
}

#[invariant(true)]
#[invariant(::Connected => true)]
#[invariant(::Co => crate::tree::free_cmavo_text(co, "co"))]
#[invariant(::Bo => crate::tree::free_cmavo_text(bo, "bo"))]
#[invariant(::Na => crate::tree::free_cmavo_label(na, "NA", &["na", "ja'a"]))]
#[invariant(::Base(word) => crate::tree::wi_relation_word(word))]
#[invariant(::Se => crate::tree::free_cmavo_label(se, "SE", &["se", "te", "ve", "xe"]))]
#[invariant(::Ke => crate::tree::free_cmavo_text(ke, "ke") && crate::tree::opt_free_cmavo_text(kehe, "ke'e"))]
#[invariant(::TenseModal => true)]
#[invariant(::Guha => crate::tree::opt_gihi(gihi))]
#[invariant(::Abstraction(abstraction) => crate::tree::abstraction_is_valid(abstraction))]
#[invariant(::Compound(..) => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum RelationSyntax {
    Connected {
        leading_relation: Box<RelationSyntax>,
        connective: ConnectiveSyntax,
        trailing_relation: Box<RelationSyntax>,
    },
    Co {
        leading_relation: Box<RelationSyntax>,
        co: WithFreeModifiers<WithIndicators<WordLike>>,
        trailing_relation: Box<RelationSyntax>,
    },
    Bo {
        leading_relation: Box<RelationSyntax>,
        bo_connective: Option<Box<ConnectiveSyntax>>,
        bo_tense_modal: Option<Box<TenseModalSyntax>>,
        bo: WithFreeModifiers<WithIndicators<WordLike>>,
        trailing_relation: Box<RelationSyntax>,
    },
    Na {
        na: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner_relation: Box<RelationSyntax>,
    },
    Base(WithIndicators<WordLike>),
    Se {
        se: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner_relation: Box<RelationSyntax>,
    },
    Ke {
        ke_tense_modal: Option<Box<TenseModalSyntax>>,
        ke: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        relation: Box<RelationSyntax>,
        kehe: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    TenseModal {
        tense_modal: TenseModalSyntax,
        #[tree_child(primary)]
        inner_relation: Box<RelationSyntax>,
    },
    Guha {
        guhek: ConnectiveSyntax,
        leading_predicate: Box<PredicateSyntax>,
        gik: ConnectiveSyntax,
        trailing_predicate: Box<PredicateSyntax>,
        gihi: Option<WithIndicators<WordLike>>,
    },
    Abstraction(AbstractionSyntax),
    Compound(Box<RelationUnitVec>),
}

pub type RelationUnitVec = SmallVec1<[RelationUnitSyntax; 2]>;

#[invariant(direction.iter().all(|direction| crate::tree::wi_cmavo_label(direction, "PU", &["pu", "ca", "ba"])))]
#[invariant(distance.as_ref().is_none_or(|distance| crate::tree::wi_cmavo_label(distance, "ZI", &["zi", "za", "zu"])))]
#[invariant(interval.as_ref().is_none_or(|interval| crate::tree::wi_cmavo_label(interval, "ZEhA", &["ze'i", "ze'a", "ze'u", "ze'e"])))]
#[invariant(crate::tree::opt_wi_cmavo_text(nai, "nai"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TimeTenseSyntax {
    pub direction: Vec<WithIndicators<WordLike>>,
    pub distance: Option<WithIndicators<WordLike>>,
    pub interval: Option<WithIndicators<WordLike>>,
    pub nai: Option<WithIndicators<WordLike>>,
}

#[invariant(direction.iter().all(|direction| crate::tree::wi_cmavo_label(direction, "FAhA", crate::tree::FAHA_WORDS)))]
#[invariant(distance.iter().all(|distance| crate::tree::wi_cmavo_label(distance, "VA", &["vi", "va", "vu"])))]
#[invariant(interval.iter().all(|interval| crate::tree::wi_cmavo_label(interval, "VEhA", &["ve'i", "ve'a", "ve'u", "ve'e"])))]
#[invariant(dimensions.iter().all(|dimension| crate::tree::wi_cmavo_label(dimension, "VIhA", &["vi'i", "vi'a", "vi'u", "vi'e"])))]
#[invariant(crate::tree::opt_wi_cmavo_text(mohi, "mo'i"))]
#[invariant(crate::tree::opt_wi_cmavo_text(fehe, "fe'e"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SpaceTenseSyntax {
    pub direction: Vec<WithIndicators<WordLike>>,
    pub distance: Vec<WithIndicators<WordLike>>,
    pub interval: Vec<WithIndicators<WordLike>>,
    pub dimensions: Vec<WithIndicators<WordLike>>,
    pub mohi: Option<WithIndicators<WordLike>>,
    pub fehe: Option<WithIndicators<WordLike>>,
}

#[invariant(number.as_ref().is_none_or(crate::tree::word_run_number_or_letter))]
#[invariant(crate::tree::wi_cmavo_label(roi_or_tahe, "ROI", crate::grammar::tokens::ROI_WORDS)
    || crate::tree::wi_cmavo_label(roi_or_tahe, "TAhE", &["di'i", "na'o", "ru'i", "ta'e"]))]
#[invariant(crate::tree::opt_wi_cmavo_text(nai, "nai"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IntervalTenseSyntax {
    pub number: Option<WordRun>,
    pub roi_or_tahe: WithIndicators<WordLike>,
    pub nai: Option<WithIndicators<WordLike>>,
}

#[invariant(nahe.as_ref().is_none_or(|nahe| crate::tree::wi_cmavo_label(nahe, "NAhE", &["na'e", "to'e", "no'e", "je'a"])))]
#[invariant(se.as_ref().is_none_or(|se| crate::tree::wi_cmavo_label(se, "SE", &["se", "te", "ve", "xe"])))]
#[invariant(bai.as_ref().is_none_or(|bai| crate::tree::wi_cmavo_label(bai, "BAI", crate::grammar::tokens::BAI_WORDS)))]
#[invariant(crate::tree::opt_wi_cmavo_text(nai, "nai"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SimpleTenseModalSyntax {
    pub nahe: Option<WithIndicators<WordLike>>,
    pub se: Option<WithIndicators<WordLike>>,
    pub bai: Option<WithIndicators<WordLike>>,
    pub nai: Option<WithIndicators<WordLike>>,
}

#[invariant(nahe.as_ref().is_none_or(|nahe| crate::tree::wi_cmavo_label(nahe, "NAhE", &["na'e", "to'e", "no'e", "je'a"])))]
#[invariant(crate::tree::free_cmavo_text(fiho, "fi'o"))]
#[invariant(crate::tree::opt_free_cmavo_text(fehu, "fe'u"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FihoModalSyntax {
    pub nahe: Option<WithIndicators<WordLike>>,
    pub fiho: WithFreeModifiers<WithIndicators<WordLike>>,
    pub relation: RelationSyntax,
    pub fehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
}

#[invariant(true)]
#[invariant(::Word(word) => crate::tree::tense_modal_word_is_valid(word))]
#[invariant(::Fiho(fiho) => crate::tree::fiho_modal_is_valid(fiho))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum CompositeTenseModalPartSyntax {
    Word(WithIndicators<WordLike>),
    Fiho(FihoModalSyntax),
}

#[invariant(true)]
#[invariant(::Composite => !parts.value.is_empty())]
#[invariant(::Pu(pu) => crate::tree::free_cmavo_label(pu, "PU", &["pu", "ca", "ba"]))]
#[invariant(::PuDistance => crate::tree::wi_cmavo_label(pu, "PU", &["pu", "ca", "ba"]) && crate::tree::free_cmavo_label(distance, "ZI", &["zi", "za", "zu"]))]
#[invariant(::TimeInterval(interval) => crate::tree::free_cmavo_label(interval, "ZEhA", &["ze'i", "ze'a", "ze'u", "ze'e"]))]
#[invariant(::PuCaha => crate::tree::wi_cmavo_label(pu, "PU", &["pu", "ca", "ba"]) && crate::tree::free_cmavo_label(caha, "CAhA", crate::grammar::tokens::CAHA_WORDS))]
#[invariant(::SpaceDistance(distance) => crate::tree::free_cmavo_label(distance, "VA", &["vi", "va", "vu"]))]
#[invariant(::SpaceDirection(direction) => crate::tree::free_cmavo_label(direction, "FAhA", crate::tree::FAHA_WORDS))]
#[invariant(::SpaceMovement => crate::tree::wi_cmavo_text(mohi, "mo'i") && crate::tree::free_cmavo_label(direction, "FAhA", crate::tree::FAHA_WORDS) && crate::tree::opt_free_cmavo_label(distance, "VA", &["vi", "va", "vu"]))]
#[invariant(::Simple => nahe.as_ref().is_none_or(|nahe| crate::tree::free_cmavo_label(nahe, "NAhE", &["na'e", "to'e", "no'e", "je'a"])) && se.as_ref().is_none_or(|se| crate::tree::free_cmavo_label(se, "SE", &["se", "te", "ve", "xe"])) && crate::tree::free_cmavo_label(bai, "BAI", crate::grammar::tokens::BAI_WORDS) && crate::tree::opt_free_cmavo_text(nai, "nai") && crate::tree::opt_free_cmavo_text(ki, "ki"))]
#[invariant(::Ki(ki) => crate::tree::free_cmavo_text(ki, "ki"))]
#[invariant(::Fiho => crate::tree::free_cmavo_text(fiho, "fi'o") && crate::tree::opt_free_cmavo_text(fehu, "fe'u"))]
#[invariant(::Caha(caha) => crate::tree::free_cmavo_label(caha, "CAhA", crate::grammar::tokens::CAHA_WORDS))]
#[invariant(::Zaho(zaho) => crate::tree::free_words_cmavo_label(zaho, "ZAhO", crate::grammar::tokens::ZAHO_WORDS))]
#[invariant(::Interval => number.as_ref().is_none_or(crate::tree::word_run_number_or_letter) && (crate::tree::free_cmavo_label(roi_or_tahe, "ROI", crate::grammar::tokens::ROI_WORDS) || crate::tree::free_cmavo_label(roi_or_tahe, "TAhE", &["di'i", "na'o", "ru'i", "ta'e"])) && crate::tree::opt_free_cmavo_text(nai, "nai"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum TenseModalSyntax {
    Composite {
        #[tree_child(primary)]
        parts: WithFreeModifiers<Vec<CompositeTenseModalPartSyntax>>,
    },
    Pu(WithFreeModifiers<WithIndicators<WordLike>>),
    PuDistance {
        pu: WithIndicators<WordLike>,
        distance: WithFreeModifiers<WithIndicators<WordLike>>,
    },
    TimeInterval(WithFreeModifiers<WithIndicators<WordLike>>),
    PuCaha {
        pu: WithIndicators<WordLike>,
        caha: WithFreeModifiers<WithIndicators<WordLike>>,
    },
    SpaceDistance(WithFreeModifiers<WithIndicators<WordLike>>),
    SpaceDirection(WithFreeModifiers<WithIndicators<WordLike>>),
    SpaceMovement {
        mohi: WithIndicators<WordLike>,
        direction: WithFreeModifiers<WithIndicators<WordLike>>,
        distance: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Simple {
        nahe: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        se: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        bai: WithFreeModifiers<WithIndicators<WordLike>>,
        nai: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        ki: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Ki(WithFreeModifiers<WithIndicators<WordLike>>),
    Fiho {
        fiho: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        relation: Box<RelationSyntax>,
        fehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Caha(WithFreeModifiers<WithIndicators<WordLike>>),
    Zaho(WithFreeModifiers<Vec<WithIndicators<WordLike>>>),
    Interval {
        number: Option<WordRun>,
        roi_or_tahe: WithFreeModifiers<WithIndicators<WordLike>>,
        nai: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
}

#[invariant(crate::tree::free_cmavo_label(nu, "NU", crate::grammar::tokens::NU_WORDS))]
#[invariant(crate::tree::opt_free_cmavo_text(nai, "nai"))]
#[invariant(crate::tree::opt_free_cmavo_text(kei, "kei"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AbstractionSyntax {
    pub nu: WithFreeModifiers<WithIndicators<WordLike>>,
    pub nai: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    pub additional_nu: Vec<AdditionalNuSyntax>,
    #[tree_child(primary)]
    pub subsentence: Box<SubsentenceSyntax>,
    pub kei: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
}

#[invariant(crate::tree::free_cmavo_label(nu, "NU", crate::grammar::tokens::NU_WORDS))]
#[invariant(crate::tree::opt_free_cmavo_text(nai, "nai"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AdditionalNuSyntax {
    pub connective: ConnectiveSyntax,
    pub nu: WithFreeModifiers<WithIndicators<WordLike>>,
    pub nai: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
}

#[invariant(true)]
#[invariant(::Word(word) => crate::tree::free_relation_word(word))]
#[invariant(::Goha => crate::tree::free_cmavo_label(goha, "GOhA", crate::grammar::tokens::GOHA_WORDS) && crate::tree::opt_free_cmavo_text(raho, "ra'o"))]
#[invariant(::Se => crate::tree::free_cmavo_label(se, "SE", &["se", "te", "ve", "xe"]))]
#[invariant(::Ke => crate::tree::free_cmavo_text(ke, "ke") && crate::tree::opt_free_cmavo_text(kehe, "ke'e"))]
#[invariant(::Nahe => crate::tree::free_cmavo_label(nahe, "NAhE", &["na'e", "to'e", "no'e", "je'a"]))]
#[invariant(::Bo => crate::tree::free_cmavo_text(bo, "bo"))]
#[invariant(::Connected => true)]
#[invariant(::SelbriRelativeClause => !selbri_relative_clauses.is_empty())]
#[invariant(::Wrapped(..) => true)]
#[invariant(::Jai => crate::tree::free_cmavo_text(jai, "jai"))]
#[invariant(::Be => crate::tree::free_cmavo_text(be, "be") && crate::tree::opt_free_cmavo_label(fa, "FA", crate::grammar::tokens::FA_WORDS) && (fa.is_none() || first_argument.is_some()) && crate::tree::opt_free_cmavo_text(beho, "be'o"))]
#[invariant(::PreposedBe => crate::tree::free_cmavo_text(be, "be") && crate::tree::opt_free_cmavo_label(fa, "FA", crate::grammar::tokens::FA_WORDS) && (fa.is_none() || first_argument.is_some()) && crate::tree::opt_free_cmavo_text(beho, "be'o"))]
#[invariant(::Abstraction(abstraction) => crate::tree::abstraction_is_valid(abstraction))]
#[invariant(::Me => crate::tree::free_cmavo_text(me, "me") && crate::tree::opt_free_cmavo_text(mehu, "me'u") && crate::tree::opt_free_cmavo_label(moi_marker, "MOI", crate::grammar::tokens::MOI_WORDS))]
#[invariant(::Mehoi(mehoi) => crate::tree::free_cmavo_text(mehoi, "me'oi"))]
#[invariant(::Gohoi(gohoi) => crate::tree::free_cmavo_text(gohoi, "go'oi"))]
#[invariant(::Muhoi(muhoi) => crate::tree::free_cmavo_text(muhoi, "mu'oi"))]
#[invariant(::Luhei => crate::tree::free_cmavo_text(luhei, "lu'ei") && crate::tree::opt_free_cmavo_text(liau, "li'au"))]
#[invariant(::Moi => crate::tree::word_run_number_or_letter(number) && crate::tree::free_cmavo_label(moi, "MOI", crate::grammar::tokens::MOI_WORDS))]
#[invariant(::Nuha => crate::tree::free_cmavo_text(nuha, "nu'a"))]
#[invariant(::Xohi => crate::tree::free_cmavo_text(xohi, "xo'i"))]
#[invariant(::Cei => !assignments.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum RelationUnitSyntax {
    Word(WithFreeModifiers<WithIndicators<WordLike>>),
    Goha {
        goha: WithFreeModifiers<WithIndicators<WordLike>>,
        raho: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Se {
        se: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner_unit: Box<RelationUnitSyntax>,
    },
    Ke {
        ke_tense_modal: Option<Box<TenseModalSyntax>>,
        ke: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        relation: RelationSyntax,
        kehe: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Nahe {
        nahe: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        inner_unit: Box<RelationUnitSyntax>,
    },
    Bo {
        leading_unit: Box<RelationUnitSyntax>,
        bo_connective: Option<Box<ConnectiveSyntax>>,
        bo_tense_modal: Option<Box<TenseModalSyntax>>,
        bo: WithFreeModifiers<WithIndicators<WordLike>>,
        trailing_unit: Box<RelationUnitSyntax>,
    },
    Connected {
        leading_unit: Box<RelationUnitSyntax>,
        connective: ConnectiveSyntax,
        trailing_unit: Box<RelationUnitSyntax>,
    },
    SelbriRelativeClause {
        #[tree_child(primary)]
        base: Box<RelationUnitSyntax>,
        selbri_relative_clauses: Vec<SelbriRelativeClauseSyntax>,
    },
    Wrapped(RelationSyntax),
    Jai {
        jai: WithFreeModifiers<WithIndicators<WordLike>>,
        tense_modal: Option<Box<TenseModalSyntax>>,
        #[tree_child(primary)]
        inner_unit: Box<RelationUnitSyntax>,
    },
    Be {
        #[tree_child(primary)]
        base: Box<RelationUnitSyntax>,
        be: WithFreeModifiers<WithIndicators<WordLike>>,
        fa: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        first_argument: Option<Box<ArgumentSyntax>>,
        bei_links: Vec<BeiLinkSyntax>,
        beho: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    PreposedBe {
        be: WithFreeModifiers<WithIndicators<WordLike>>,
        fa: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        first_argument: Option<Box<ArgumentSyntax>>,
        bei_links: Vec<BeiLinkSyntax>,
        beho: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        #[tree_child(primary)]
        base: Box<RelationUnitSyntax>,
    },
    Abstraction(AbstractionSyntax),
    Me {
        me: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        argument: ArgumentSyntax,
        mehu: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
        moi_marker: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Mehoi(WithFreeModifiers<WithIndicators<WordLike>>),
    Gohoi(WithFreeModifiers<WithIndicators<WordLike>>),
    Muhoi(WithFreeModifiers<WithIndicators<WordLike>>),
    Luhei {
        luhei: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        text: TextSyntax,
        liau: Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    },
    Moi {
        number: WordRun,
        moi: WithFreeModifiers<WithIndicators<WordLike>>,
    },
    Nuha {
        nuha: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        math_operator: MathOperatorSyntax,
    },
    Xohi {
        xohi: WithFreeModifiers<WithIndicators<WordLike>>,
        #[tree_child(primary)]
        tag: TenseModalSyntax,
    },
    Cei {
        #[tree_child(primary)]
        base: Box<RelationUnitSyntax>,
        assignments: Vec<CeiAssignmentSyntax>,
    },
}

#[invariant(crate::tree::free_cmavo_text(cei, "cei"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CeiAssignmentSyntax {
    pub cei: WithFreeModifiers<WithIndicators<WordLike>>,
    #[tree_child(primary)]
    pub relation_unit: RelationUnitSyntax,
}

}

#[requires(!expected.is_empty())]
#[ensures(true)]
pub(crate) fn wi_cmavo_text(word: &WithIndicators<WordLike>, expected: &str) -> bool {
    word.visible_word()
        .is_some_and(|word| word.is_cmavo_text(expected))
}

#[requires(!expected.is_empty())]
#[ensures(true)]
pub(crate) fn free_cmavo_text(
    word: &WithFreeModifiers<WithIndicators<WordLike>>,
    expected: &str,
) -> bool {
    wi_cmavo_text(&word.value, expected)
}

#[requires(!texts.is_empty())]
#[ensures(true)]
pub(crate) fn wi_cmavo_any(word: &WithIndicators<WordLike>, texts: &[&str]) -> bool {
    texts.iter().any(|text| wi_cmavo_text(word, text))
}

#[requires(!texts.is_empty())]
#[ensures(true)]
pub(crate) fn free_cmavo_any(
    word: &WithFreeModifiers<WithIndicators<WordLike>>,
    texts: &[&str],
) -> bool {
    wi_cmavo_any(&word.value, texts)
}

#[requires(!expected.is_empty())]
#[ensures(true)]
pub(crate) fn opt_wi_cmavo_text(word: &Option<WithIndicators<WordLike>>, expected: &str) -> bool {
    word.as_ref()
        .is_none_or(|word| wi_cmavo_text(word, expected))
}

#[requires(!expected.is_empty())]
#[ensures(true)]
pub(crate) fn opt_free_cmavo_text(
    word: &Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    expected: &str,
) -> bool {
    word.as_ref()
        .is_none_or(|word| free_cmavo_text(word, expected))
}

#[requires(!label.is_empty())]
#[requires(!texts.is_empty())]
#[ensures(true)]
pub(crate) fn wi_cmavo_label(word: &WithIndicators<WordLike>, label: &str, texts: &[&str]) -> bool {
    let Some(word) = word.visible_word() else {
        return false;
    };
    texts.iter().any(|text| word.is_cmavo_text(text))
        || crate::grammar::tokens::zantufa_cmavo_words_for(label)
            .iter()
            .any(|text| word.is_cmavo_text(text))
}

#[requires(!label.is_empty())]
#[requires(!texts.is_empty())]
#[ensures(true)]
pub(crate) fn free_cmavo_label(
    word: &WithFreeModifiers<WithIndicators<WordLike>>,
    label: &str,
    texts: &[&str],
) -> bool {
    wi_cmavo_label(&word.value, label, texts)
}

#[requires(!label.is_empty())]
#[requires(!texts.is_empty())]
#[ensures(true)]
pub(crate) fn opt_wi_cmavo_label(
    word: &Option<WithIndicators<WordLike>>,
    label: &str,
    texts: &[&str],
) -> bool {
    word.as_ref()
        .is_none_or(|word| wi_cmavo_label(word, label, texts))
}

#[requires(!label.is_empty())]
#[requires(!texts.is_empty())]
#[ensures(true)]
pub(crate) fn opt_free_cmavo_label(
    word: &Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    label: &str,
    texts: &[&str],
) -> bool {
    word.as_ref()
        .is_none_or(|word| free_cmavo_label(word, label, texts))
}

#[requires(!label.is_empty())]
#[requires(!texts.is_empty())]
#[ensures(true)]
pub(crate) fn free_words_cmavo_label(
    words: &WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
    label: &str,
    texts: &[&str],
) -> bool {
    words
        .value
        .iter()
        .all(|word| wi_cmavo_label(word, label, texts))
}

#[requires(!texts.is_empty())]
#[ensures(true)]
pub(crate) fn free_words_cmavo_texts(
    words: &WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
    texts: &[&str],
) -> bool {
    words.value.iter().all(|word| wi_cmavo_any(word, texts))
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn free_nahe_bo_markers(
    markers: &WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
) -> bool {
    matches!(
        markers.value.as_slice(),
        [nahe, bo]
            if wi_cmavo_label(nahe, "NAhE", &["na'e", "to'e", "no'e", "je'a"])
                && wi_cmavo_text(bo, "bo")
    )
}

#[requires(!label.is_empty())]
#[requires(!texts.is_empty())]
#[ensures(true)]
pub(crate) fn word_run_cmavo_label(words: &WordRun, label: &str, texts: &[&str]) -> bool {
    words.iter().all(|word| wi_cmavo_label(word, label, texts))
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn word_run_number_or_letter(words: &WordRun) -> bool {
    words.iter().all(|word| {
        wi_cmavo_label(word, "PA", crate::grammar::tokens::PA_WORDS)
            || crate::grammar::tokens::is_letter_word(word)
    })
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn free_word_run_number_or_letter(words: &WithFreeModifiers<WordRun>) -> bool {
    word_run_number_or_letter(&words.value)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn word_run_cmevla(words: &WordRun) -> bool {
    words.iter().all(crate::grammar::tokens::is_cmevla_word)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn free_word_run_cmevla(words: &WithFreeModifiers<WordRun>) -> bool {
    word_run_cmevla(&words.value)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn wi_word_kind(word: &WithIndicators<WordLike>, expected: WordKind) -> bool {
    word.visible_word()
        .is_some_and(|word| word.kind() == expected)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn free_word_kind(
    word: &WithFreeModifiers<WithIndicators<WordLike>>,
    expected: WordKind,
) -> bool {
    wi_word_kind(&word.value, expected)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn wi_relation_word(word: &WithIndicators<WordLike>) -> bool {
    crate::grammar::tokens::is_relation_word(word)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn free_relation_word(word: &WithFreeModifiers<WithIndicators<WordLike>>) -> bool {
    wi_relation_word(&word.value)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn opt_gihi(word: &Option<WithIndicators<WordLike>>) -> bool {
    opt_wi_cmavo_label(word, "GIhI", &["gi'i"])
}

pub(crate) const FAHA_WORDS: &[&str] = &[
    "be'a", "du'a", "vu'a", "ne'u", "ca'u", "ri'u", "zu'a", "ga'u", "ni'a", "ti'a", "ru'u", "re'o",
    "te'e", "bu'u", "ne'a", "pa'o", "ne'i", "fa'a", "to'o", "zo'a", "zo'i", "ze'o",
];

#[requires(true)]
#[ensures(true)]
pub(crate) fn free_words_vocative_markers(
    markers: &WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
) -> bool {
    if markers.value.is_empty() {
        return false;
    }

    let mut may_take_nai = false;
    for (index, word) in markers.value.iter().enumerate() {
        if wi_cmavo_label(word, "COI", crate::grammar::tokens::COI_WORDS) {
            may_take_nai = true;
        } else if may_take_nai && wi_cmavo_text(word, "nai") {
            may_take_nai = false;
        } else if wi_cmavo_text(word, "doi") && index + 1 == markers.value.len() {
            may_take_nai = false;
        } else {
            return false;
        }
    }
    true
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn goi_relative_clause_is_valid(clause: &GoiRelativeClauseSyntax) -> bool {
    free_cmavo_label(
        &clause.goi,
        "GOI",
        &["pe", "ne", "po", "po'e", "po'u", "no'u", "goi"],
    ) && opt_free_cmavo_text(&clause.gehu, "ge'u")
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn descriptor_is_valid(descriptor: &DescriptorSyntax) -> bool {
    descriptor
        .descriptor
        .as_ref()
        .is_none_or(descriptor_marker_is_valid)
        && opt_free_cmavo_text(&descriptor.ku, "ku")
        && (descriptor.descriptor.is_some()
            || (!descriptor.tail_elements.is_empty() && descriptor.relation.is_some()))
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn connected_descriptor_is_valid(descriptor: &ConnectedDescriptorSyntax) -> bool {
    descriptor_head_is_valid(&descriptor.leading_descriptor_head)
        && descriptor_head_is_valid(&descriptor.trailing_descriptor_head)
        && opt_free_cmavo_text(&descriptor.ku, "ku")
}

#[requires(true)]
#[ensures(true)]
fn descriptor_head_is_valid(head: &DescriptorHeadSyntax) -> bool {
    descriptor_marker_is_valid(&head.descriptor)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn descriptor_marker_is_valid(
    marker: &WithFreeModifiers<WithIndicators<WordLike>>,
) -> bool {
    free_cmavo_label(
        marker,
        "LE",
        &["lei", "loi", "le'i", "lo'i", "le'e", "lo'e", "lo", "le"],
    ) || free_cmavo_label(marker, "LA", &["lai", "la'i", "la"])
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn abstraction_is_valid(abstraction: &AbstractionSyntax) -> bool {
    free_cmavo_label(&abstraction.nu, "NU", crate::grammar::tokens::NU_WORDS)
        && opt_free_cmavo_text(&abstraction.nai, "nai")
        && abstraction.additional_nu.iter().all(additional_nu_is_valid)
        && opt_free_cmavo_text(&abstraction.kei, "kei")
}

#[requires(true)]
#[ensures(true)]
fn additional_nu_is_valid(additional_nu: &AdditionalNuSyntax) -> bool {
    free_cmavo_label(&additional_nu.nu, "NU", crate::grammar::tokens::NU_WORDS)
        && opt_free_cmavo_text(&additional_nu.nai, "nai")
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn fiho_modal_is_valid(modal: &FihoModalSyntax) -> bool {
    opt_wi_cmavo_label(&modal.nahe, "NAhE", &["na'e", "to'e", "no'e", "je'a"])
        && free_cmavo_text(&modal.fiho, "fi'o")
        && opt_free_cmavo_text(&modal.fehu, "fe'u")
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn term_wrapper_is_valid(
    kind: &TermWrapperKindSyntax,
    wrapper: &WithFreeModifiers<WithIndicators<WordLike>>,
    wrapper_bo: &Option<WithFreeModifiers<WithIndicators<WordLike>>>,
) -> bool {
    match kind {
        TermWrapperKindSyntax::Lahe => {
            free_cmavo_label(
                wrapper,
                "LAhE",
                &["tu'a", "lu'a", "lu'o", "la'e", "vu'i", "lu'i", "lu'e"],
            ) && wrapper_bo.is_none()
        }
        TermWrapperKindSyntax::NaheBo => {
            free_cmavo_label(wrapper, "NAhE", &["na'e", "to'e", "no'e", "je'a"])
                && wrapper_bo
                    .as_ref()
                    .is_some_and(|bo| free_cmavo_text(bo, "bo"))
        }
        TermWrapperKindSyntax::Nahe => {
            free_cmavo_label(wrapper, "NAhE", &["na'e", "to'e", "no'e", "je'a"])
                && wrapper_bo.is_none()
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn connective_parts_are_valid(
    se: &Option<WithIndicators<WordLike>>,
    nahe: &Option<WithIndicators<WordLike>>,
    na: &Option<WithIndicators<WordLike>>,
    cmavo: &WithFreeModifiers<Vec<WithIndicators<WordLike>>>,
    nai: &Option<WithFreeModifiers<WithIndicators<WordLike>>>,
    _kind: ConnectiveKind,
) -> bool {
    opt_wi_cmavo_label(se, "SE", &["se", "te", "ve", "xe"])
        && opt_wi_cmavo_label(nahe, "NAhE", &["na'e", "to'e", "no'e", "je'a"])
        && opt_wi_cmavo_label(na, "NA", &["na", "ja'a"])
        && !cmavo.value.is_empty()
        && cmavo.value.iter().all(connective_word_is_valid)
        && opt_free_cmavo_text(nai, "nai")
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn connective_word_is_valid(word: &WithIndicators<WordLike>) -> bool {
    wi_cmavo_label(word, "A", &["a", "e", "o", "u", "ji"])
        || wi_cmavo_label(word, "JA", &["je'i", "ja", "je", "jo", "ju"])
        || wi_cmavo_label(
            word,
            "JOI",
            &[
                "ce", "ce'e", "ce'o", "fa'u", "jo'e", "jo'u", "joi", "ju'e", "ku'a", "pi'u",
            ],
        )
        || wi_cmavo_label(word, "BIhI", &["mi'i", "bi'o", "bi'i"])
        || wi_cmavo_label(word, "GAhO", &["ga'o", "ke'i"])
        || wi_cmavo_label(word, "GIhA", &["gi'e", "gi'i", "gi'o", "gi'a", "gi'u"])
        || wi_cmavo_label(word, "GA", &["ga", "ge", "ge'i", "go", "gu"])
        || wi_cmavo_label(word, "GUhA", &["gu'a", "gu'e", "gu'i", "gu'o", "gu'u"])
        || wi_cmavo_text(word, "gi")
        || wi_cmavo_text(word, "bo")
        || wi_cmavo_label(word, "VUhU", crate::grammar::tokens::VUHU_WORDS)
        || tense_modal_word_is_valid(word)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn tense_modal_word_is_valid(word: &WithIndicators<WordLike>) -> bool {
    wi_cmavo_label(word, "PU", &["pu", "ca", "ba"])
        || wi_cmavo_label(word, "ZI", &["zi", "za", "zu"])
        || wi_cmavo_label(word, "VA", &["vi", "va", "vu"])
        || wi_cmavo_label(word, "ZEhA", &["ze'i", "ze'a", "ze'u", "ze'e"])
        || wi_cmavo_label(word, "FAhA", FAHA_WORDS)
        || wi_cmavo_label(word, "VEhA", &["ve'i", "ve'a", "ve'u", "ve'e"])
        || wi_cmavo_label(word, "VIhA", &["vi'i", "vi'a", "vi'u", "vi'e"])
        || wi_cmavo_label(word, "CAhA", crate::grammar::tokens::CAHA_WORDS)
        || wi_cmavo_label(word, "ZAhO", crate::grammar::tokens::ZAHO_WORDS)
        || wi_cmavo_label(word, "ROI", crate::grammar::tokens::ROI_WORDS)
        || wi_cmavo_label(word, "TAhE", &["di'i", "na'o", "ru'i", "ta'e"])
        || wi_cmavo_label(word, "BAI", crate::grammar::tokens::BAI_WORDS)
        || wi_cmavo_label(word, "NAhE", &["na'e", "to'e", "no'e", "je'a"])
        || wi_cmavo_label(word, "SE", &["se", "te", "ve", "xe"])
        || wi_cmavo_label(word, "PA", crate::grammar::tokens::PA_WORDS)
        || wi_cmavo_label(word, "FA", crate::grammar::tokens::FA_WORDS)
        || crate::grammar::tokens::is_letter_word(word)
        || wi_cmavo_text(word, "ki")
        || wi_cmavo_text(word, "cu'e")
        || wi_cmavo_text(word, "nau")
        || wi_cmavo_text(word, "fe'e")
        || wi_cmavo_text(word, "mo'i")
        || wi_cmavo_text(word, "nai")
}

impl<T: TreeNode> TreeNode for WithFreeModifiers<T> {
    #[requires(true)]
    #[ensures(true)]
    fn visit_in_order<'tree, V>(&'tree self, visitor: &mut V)
    where
        V: jbotci_tree::TreeVisitor<'tree, Node = NodeRef<'tree>, Atom = AtomRef<'tree>>,
    {
        self.value.visit_in_order(visitor);
        if !self.free_modifiers.is_empty() {
            let field_ref = jbotci_tree::FieldRef::new(Some("free_modifiers"), false);
            visitor.enter_field(field_ref);
            self.free_modifiers.visit_in_order(visitor);
            visitor.exit_field(field_ref);
        }
    }
}

impl Indicator {
    #[requires(true)]
    #[ensures(true)]
    pub fn new(indicator: WithIndicators<WordLike>, nai: Option<Word>) -> Self {
        new!(Indicator {
            indicator: indicator,
            nai: nai,
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn words(&self) -> Vec<WithIndicators<WordLike>> {
        let mut words = vec![self.indicator.clone()];
        if let Some(nai) = &self.nai {
            words.push(WithIndicators::bare(WordLike::bare(nai.clone())));
        }
        words
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn visit_words(&self, visitor: &mut impl FnMut(&WithIndicators<WordLike>)) {
        visitor(&self.indicator);
        if let Some(nai) = &self.nai {
            let nai = WithIndicators::bare(WordLike::bare(nai.clone()));
            visitor(&nai);
        }
    }

    #[requires(true)]
    #[ensures(ret >= 1)]
    pub fn word_count(&self) -> usize {
        1 + usize::from(self.nai.is_some())
    }
}

#[allow(unused_imports)]
use bityzba::ensures;
use bityzba::{data, new};
use bityzba::{invariant, requires};
use jbotci_morphology::{Cmavo, LeadingPauseContext, Phonemes, Word, WordLike, WordLikeData};
use jbotci_orthography::render_latin_word_surface_for_script;
use jbotci_syntax::generated_model::{
    AtomRef as GeneratedSyntaxAtomRef, NodeRef as GeneratedSyntaxNodeRef,
    TextSyntax as GeneratedTextSyntax, TreeNode as GeneratedSyntaxTreeNode,
};
use jbotci_syntax::{
    Token, WithIndicators, WithIndicatorsData, elidable_terminator_for_absent_field_ref,
};
use jbotci_tree::{FieldRef, TreeVisitor};

use crate::{
    BracketRenderOptions, BracketSourceFragment, BracketSourceRange, OutputError, sexpr, surface,
};

#[derive(Debug, Clone, Copy)]
#[invariant(true)]
pub(crate) struct BracketContext {
    pub(crate) options: BracketRenderOptions,
}

#[requires(true)]
#[ensures(words.is_empty() || ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub(crate) fn pretty_morphology_brackets_with_options(
    words: &[WordLike],
    source: &str,
    options: BracketRenderOptions,
) -> Result<String, OutputError> {
    let _ = source;
    let context = BracketContext { options };
    let sexpr = sexpr::node(
        words
            .iter()
            .map(|word_like| word_like_brackets(word_like, &context))
            .collect(),
    );
    Ok(sexpr::render_bracketed_with_options(
        &sexpr::flatten(sexpr),
        options,
    ))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub(crate) fn pretty_generated_model_brackets_with_options(
    tree: &GeneratedTextSyntax,
    source: &str,
    options: BracketRenderOptions,
) -> Result<String, OutputError> {
    let sexpr = generated_model_bracket_sexpr(tree, source, options);
    Ok(sexpr::render_bracketed_with_options(
        &sexpr::flatten(sexpr),
        options,
    ))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|fragments| !fragments.is_empty()) || ret.is_err())]
pub(crate) fn pretty_generated_model_bracket_source_fragments_with_options(
    tree: &GeneratedTextSyntax,
    source: &str,
    options: BracketRenderOptions,
) -> Result<Vec<BracketSourceFragment>, OutputError> {
    let sexpr = generated_model_bracket_sexpr(tree, source, options);
    Ok(sexpr::render_bracketed_source_fragments_with_options(
        &sexpr::flatten(sexpr),
        options,
    ))
}

#[requires(true)]
#[ensures(true)]
fn generated_model_bracket_sexpr(
    tree: &GeneratedTextSyntax,
    source: &str,
    options: BracketRenderOptions,
) -> sexpr::SExpr {
    let _ = source;
    let context = BracketContext { options };
    let mut visitor = GeneratedBracketVisitor::new(&context);
    tree.visit_in_order(&mut visitor);
    visitor.finish()
}

#[derive(Debug)]
#[invariant(true)]
struct GeneratedBracketFrame {
    children: Vec<sexpr::SExpr>,
}

#[derive(Debug)]
#[invariant(true)]
struct GeneratedBracketVisitor<'source> {
    source: &'source BracketContext,
    stack: Vec<GeneratedBracketFrame>,
    root: Option<sexpr::SExpr>,
}

impl<'source> GeneratedBracketVisitor<'source> {
    #[requires(true)]
    #[ensures(ret.stack.is_empty())]
    fn new(source: &'source BracketContext) -> Self {
        Self {
            source,
            stack: Vec::new(),
            root: None,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn finish(self) -> sexpr::SExpr {
        self.root
            .expect("generated syntax bracket walk produced a root")
    }

    #[requires(true)]
    #[ensures(matches!(self.stack.last(), Some(frame) if frame.children.is_empty()))]
    fn push_frame(&mut self) {
        self.stack.push(GeneratedBracketFrame {
            children: Vec::new(),
        });
    }

    #[requires(matches!(self.stack.last(), Some(_)))]
    #[ensures(true)]
    fn pop_frame(&mut self) {
        let frame = self
            .stack
            .pop()
            .expect("generated syntax bracket walker popped an entered frame");
        self.push_expr(sexpr::node(frame.children));
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_expr(&mut self, expr: sexpr::SExpr) {
        if let Some(frame) = self.stack.last_mut() {
            frame.children.push(expr);
        } else {
            self.root = Some(expr);
        }
    }
}

impl<'tree> TreeVisitor<'tree> for GeneratedBracketVisitor<'_> {
    type Node = GeneratedSyntaxNodeRef<'tree>;
    type Atom = GeneratedSyntaxAtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn enter_node(&mut self, _node: Self::Node) {
        self.push_frame();
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_node(&mut self, _node: Self::Node) {
        self.pop_frame();
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_field(&mut self, _field: FieldRef) {
        self.push_frame();
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_field(&mut self, _field: FieldRef) {
        self.pop_frame();
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_sequence(&mut self) {
        self.push_frame();
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_sequence(&mut self) {
        self.pop_frame();
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_absent_optional_field(&mut self, field: FieldRef) {
        if !self.source.options.show_elided {
            return;
        }
        let Some(cmavo) = elidable_terminator_for_absent_field_ref(field) else {
            return;
        };
        let Some(frame) = self.stack.last_mut() else {
            return;
        };
        frame
            .children
            .push(elided_terminator_leaf(cmavo, &frame.children, self.source));
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        match atom {
            GeneratedSyntaxAtomRef::Token(token) => self.push_expr(word(token, self.source)),
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn word(word: &Token, source: &BracketContext) -> sexpr::SExpr {
    with_indicators_brackets(word.as_indicators(), source)
}

#[requires(true)]
#[ensures(true)]
fn with_indicators_brackets(
    word: &WithIndicators<WordLike>,
    source: &BracketContext,
) -> sexpr::SExpr {
    match word.as_data() {
        data!(WithIndicators::Plain(word_like)) => word_like_brackets(word_like, source),
        data!(WithIndicators::Emphasized {
            bahe,
            extra_bahe,
            word_like,
        }) => {
            let mut children = vec![word_leaf(bahe, source)];
            children.extend(extra_bahe.iter().map(|bahe| word_leaf(bahe, source)));
            children.push(word_like_brackets(word_like, source));
            sexpr::node(children)
        }
        data!(WithIndicators::WithIndicator {
            base,
            indicator_bahe,
            indicator,
            nai_bahe,
            nai,
        }) => {
            let mut children = vec![with_indicators_brackets(base, source)];
            children.extend(indicator_bahe.iter().map(|bahe| word_leaf(bahe, source)));
            children.push(word_leaf(indicator, source));
            if let Some(nai) = nai {
                children.extend(nai_bahe.iter().map(|bahe| word_leaf(bahe, source)));
                children.push(word_leaf(nai, source));
            }
            sexpr::node(children)
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn word_like_brackets(word_like: &WordLike, source: &BracketContext) -> sexpr::SExpr {
    word_like_brackets_in_context(word_like, source, LeadingPauseContext::IndependentWord)
}

#[requires(true)]
#[ensures(true)]
fn word_like_brackets_in_context(
    word_like: &WordLike,
    source: &BracketContext,
    context: LeadingPauseContext,
) -> sexpr::SExpr {
    match word_like.as_data() {
        data!(WordLike::PlainWord(word)) => word_leaf_in_context(word, source, context),
        data!(WordLike::QuotedWord { zo, word }) => {
            sexpr::node(vec![word_leaf(zo, source), word_leaf(word, source)])
        }
        data!(WordLike::SelmahoQuotedWord { mahoi, word }) => {
            sexpr::node(vec![word_leaf(mahoi, source), word_leaf(word, source)])
        }
        data!(WordLike::DelimitedNonLojbanQuote {
            zoi,
            opening_delimiter,
            quoted_text,
            closing_delimiter,
        }) => sexpr::node(vec![
            word_leaf(zoi, source),
            word_leaf(opening_delimiter, source),
            quoted_text_leaf(quoted_text),
            word_leaf(closing_delimiter, source),
        ]),
        data!(WordLike::QuotedWords {
            lohu,
            quoted_words,
            lehu,
        }) => {
            let mut children = vec![word_leaf(lohu, source)];
            children.extend(quoted_words.iter().map(|word| word_leaf(word, source)));
            children.push(word_leaf(lehu, source));
            sexpr::node(children)
        }
        data!(WordLike::DelimitedWordQuote {
            marker,
            quoted_text,
        }) => sexpr::node(vec![
            word_leaf(marker, source),
            quoted_text_leaf(quoted_text),
        ]),
        data!(WordLike::LerfuWord { base, bu }) => sexpr::node(vec![
            word_like_brackets_in_context(base, source, LeadingPauseContext::BuLetterBase),
            word_leaf(bu, source),
        ]),
        data!(WordLike::ZeiCompound { left, zei, right }) => sexpr::node(vec![
            word_like_brackets(left, source),
            word_leaf(zei, source),
            word_leaf(right, source),
        ]),
    }
}

#[requires(true)]
#[ensures(true)]
fn word_leaf(word: &Word, source: &BracketContext) -> sexpr::SExpr {
    word_leaf_in_context(word, source, LeadingPauseContext::IndependentWord)
}

#[requires(true)]
#[ensures(true)]
fn word_leaf_in_context(
    word: &Word,
    source: &BracketContext,
    context: LeadingPauseContext,
) -> sexpr::SExpr {
    let latin = if source.options.decompose_lujvo
        && let Some(parts) = word.lujvo_parts()
    {
        parts
            .iter()
            .map(|part| part.phonemes().render(source.options.phonemes))
            .collect::<Vec<_>>()
            .join(source.options.glyphs.lujvo_separator())
    } else {
        surface::format_word_with_options_in_context(word, source.options.phonemes, context)
    };
    sexpr::leaf_with_range(
        render_latin_word_surface_for_script(source.options.script, word.kind(), &latin),
        Some(word_bracket_source_range(word)),
    )
}

#[requires(true)]
#[ensures(true)]
fn quoted_text_leaf(verbatim: &jbotci_morphology::Verbatim) -> sexpr::SExpr {
    sexpr::leaf_with_range(
        verbatim.text.trim().to_owned(),
        Some(new!(BracketSourceRange {
            byte_start: verbatim.span.byte_start,
            byte_end: verbatim.span.byte_end,
        })),
    )
}

#[requires(word.span().byte_start <= word.span().byte_end)]
#[ensures(ret.byte_start == word.span().byte_start)]
fn word_bracket_source_range(word: &Word) -> BracketSourceRange {
    new!(BracketSourceRange {
        byte_start: word.span().byte_start,
        byte_end: word.span().byte_end,
    })
}

#[requires(true)]
#[ensures(true)]
fn elided_terminator_leaf(
    cmavo: Cmavo,
    previous_siblings: &[sexpr::SExpr],
    source: &BracketContext,
) -> sexpr::SExpr {
    sexpr::elided_leaf_with_range(
        render_latin_word_surface_for_script(
            source.options.script,
            jbotci_morphology::WordKind::Cmavo,
            &elided_cmavo_text(cmavo, source.options.phonemes),
        ),
        last_child_end_range(previous_siblings),
    )
}

#[requires(true)]
#[ensures(true)]
fn last_child_end_range(children: &[sexpr::SExpr]) -> Option<BracketSourceRange> {
    children.iter().rev().find_map(expr_end_range)
}

#[requires(true)]
#[ensures(ret.is_none_or(|range| range.byte_start == range.byte_end))]
fn expr_end_range(expr: &sexpr::SExpr) -> Option<BracketSourceRange> {
    let range = match expr {
        sexpr::SExpr::Leaf { range, .. } | sexpr::SExpr::Node { range, .. } => *range,
    }?;
    Some(new!(BracketSourceRange {
        byte_start: range.byte_end,
        byte_end: range.byte_end,
    }))
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn elided_cmavo_text(cmavo: Cmavo, options: jbotci_morphology::PhonemeRenderOptions) -> String {
    Phonemes::from_canonical(cmavo.canonical_text().to_owned())
        .expect("cmavo canonical text is valid phoneme text")
        .render(options)
}

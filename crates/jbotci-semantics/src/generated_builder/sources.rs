use super::*;

#[requires(true)]
#[ensures(true)]
pub(super) fn collect_generated_node_spans<N: TreeNode>(node: &N, spans: &mut Vec<SourceSpan>) {
    let mut visitor = GeneratedSpanCollector::default();
    node.visit_in_order(&mut visitor);
    spans.extend(visitor.spans);
}

#[requires(span.byte_start <= span.byte_end)]
#[ensures(true)]
pub(super) fn generated_node_contains_byte_span<N: TreeNode>(
    node: &N,
    span: &SourceByteSpan,
) -> bool {
    let mut spans = Vec::new();
    collect_generated_node_spans(node, &mut spans);
    generated_source_spans_contain_byte_span(&spans, span)
}

#[requires(span.byte_start <= span.byte_end)]
#[ensures(true)]
pub(super) fn generated_source_spans_contain_byte_span(
    spans: &[SourceSpan],
    span: &SourceByteSpan,
) -> bool {
    let Some(first) = spans.first() else {
        return false;
    };
    let byte_start = spans
        .iter()
        .map(|span| span.byte_start)
        .min()
        .unwrap_or(first.byte_start);
    let byte_end = spans
        .iter()
        .map(|span| span.byte_end)
        .max()
        .unwrap_or(first.byte_end);
    byte_start <= span.byte_start && span.byte_end <= byte_end
}

#[invariant(true)]
pub(super) struct GeneratedSpanCollector<'tree> {
    pub(super) spans: Vec<SourceSpan>,
    pub(super) tokens: Vec<&'tree Token>,
}

impl<'tree> Default for GeneratedSpanCollector<'tree> {
    #[requires(true)]
    #[ensures(ret.spans.is_empty())]
    fn default() -> Self {
        Self {
            spans: Vec::new(),
            tokens: Vec::new(),
        }
    }
}

impl<'tree> TreeVisitor<'tree> for GeneratedSpanCollector<'tree> {
    type Atom = GeneratedAtomRef<'tree>;
    type Node = jbotci_syntax::generated_model::NodeRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        let GeneratedAtomRef::Token(token) = atom;
        self.spans.extend(token.source_spans().into_iter().cloned());
        self.tokens.push(token);
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub(super) fn generated_node_surface_text<N: TreeNode>(node: &N) -> Result<String, SemanticsError> {
    let mut visitor = GeneratedSpanCollector::default();
    node.visit_in_order(&mut visitor);
    non_empty_token_list_text(visitor.tokens.iter().copied())
        .ok_or_else(|| unsupported("empty generated node surface text"))
}

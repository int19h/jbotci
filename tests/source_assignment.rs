#[allow(unused_imports)]
use bityzba::{ensures, requires};
use jbotci_dialect::parse_dialect_definition;
use jbotci_morphology::{MorphologyOptions, WordLike, segment_words_with_modifiers_with_options};
use jbotci_source::SourceSpan;
use jbotci_syntax::{ParseOptions, WithIndicators, parse_syntax_tree_with_source_and_options};

#[test]
#[requires(true)]
#[ensures(true)]
fn syntax_assigns_simple_sentence_tokens_once_in_order() {
    run_on_large_stack(|| assert_source_assignment("mi cu klama la zdani"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn syntax_assignment_includes_single_word_quote_text() {
    run_on_large_stack(|| assert_source_assignment("zo .ai"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn syntax_assignment_includes_zoi_raw_quoted_text() {
    run_on_large_stack(|| assert_source_assignment("zoi gy Steve gy"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn syntax_assignment_handles_non_ascii_spans() {
    run_on_large_stack(|| assert_source_assignment("zoi gy café gy"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn syntax_assignment_includes_muhoi_raw_quoted_text_once() {
    run_on_large_stack(|| {
        let dialect =
            parse_dialect_definition("(+ZANTUFA-QUOTES)").expect("valid dialect definition");
        let options = ParseOptions::default().with_dialect_definition(&dialect);

        assert_source_assignment_with_options("mi cu mu'oi gy foo gy", &options);
    });
}

#[test]
#[requires(true)]
#[ensures(true)]
fn syntax_assignment_handles_zantufa_jai_tag_term() {
    run_on_large_stack(|| {
        let dialect =
            parse_dialect_definition("(+ZANTUFA-TAGS)").expect("valid dialect definition");
        let options = ParseOptions::default().with_dialect_definition(&dialect);

        assert_source_assignment_with_options("jai pu mi cu klama", &options);
    });
}

#[test]
#[requires(true)]
#[ensures(true)]
fn syntax_assignment_handles_zantufa_poiha_brigahi() {
    run_on_large_stack(|| {
        let dialect =
            parse_dialect_definition("(+ZANTUFA-ADVERBIALS)").expect("valid dialect definition");
        let options = ParseOptions::default().with_dialect_definition(&dialect);

        assert_source_assignment_with_options("noi'a klama ku mi cu broda", &options);
    });
}

#[requires(true)]
#[ensures(true)]
fn run_on_large_stack(test: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(test)
        .expect("spawn large-stack source assignment test")
        .join()
        .expect("large-stack source assignment test thread");
}

#[requires(!source.is_empty())]
#[ensures(true)]
fn assert_source_assignment(source: &str) {
    assert_source_assignment_with_options(source, &ParseOptions::default());
}

#[requires(!source.is_empty())]
#[ensures(true)]
fn assert_source_assignment_with_options(source: &str, options: &ParseOptions) {
    let words = segment_words_with_options(source);
    let parse = parse_syntax_tree_with_source_and_options(&words, source, options)
        .expect("source should parse");

    let morphology = morphology_source_ranges(&words);
    let syntax = syntax_source_ranges(&parse.parse_tree);
    assert_eq!(syntax, morphology);
    assert!(ranges_are_strictly_ordered(&syntax));
}

#[requires(!source.is_empty())]
#[ensures(!ret.is_empty())]
fn segment_words_with_options(source: &str) -> Vec<WordLike> {
    segment_words_with_modifiers_with_options(source, &MorphologyOptions::default())
        .expect("source should segment")
}

#[requires(true)]
#[ensures(true)]
fn morphology_source_ranges(words: &[WordLike]) -> Vec<(usize, usize)> {
    words
        .iter()
        .flat_map(WordLike::source_spans)
        .map(span_range)
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn syntax_source_ranges(tree: &jbotci_syntax::ast::TextSyntax) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    tree.visit_words(&mut |word| append_word_ranges(word, &mut ranges));
    ranges
}

#[requires(true)]
#[ensures(true)]
fn append_word_ranges(word: &WithIndicators<WordLike>, ranges: &mut Vec<(usize, usize)>) {
    ranges.extend(word.source_spans().into_iter().map(span_range));
}

#[requires(true)]
#[ensures(true)]
fn span_range(span: &SourceSpan) -> (usize, usize) {
    (span.byte_start, span.byte_end)
}

#[requires(true)]
#[ensures(true)]
fn ranges_are_strictly_ordered(ranges: &[(usize, usize)]) -> bool {
    ranges
        .windows(2)
        .all(|pair| pair[0].0 <= pair[0].1 && pair[0].1 <= pair[1].0)
}

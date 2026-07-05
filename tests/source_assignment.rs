#[allow(unused_imports)]
use bityzba::{ensures, requires};
use jbotci_dialect::parse_dialect_definition;
use jbotci_morphology::{
    MorphologyOptions, WordLike, segment_words_with_modifiers_with_options_and_source_id,
};
use jbotci_source::SourceSpan;
use jbotci_syntax::{
    ParseOptions, generated_model_text_syntax_leaf_spans_match_words,
    parse_syntax_tree_generated_model_with_source_and_options,
    parse_syntax_tree_with_source_and_options,
};

#[test]
#[requires(true)]
#[ensures(true)]
fn syntax_assigns_simple_sentence_tokens_once_in_order() {
    assert_source_assignment("mi cu klama la zdani");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn syntax_assignment_includes_single_word_quote_text() {
    assert_source_assignment("zo .ai");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn syntax_assignment_includes_zoi_raw_quoted_text() {
    assert_source_assignment("zoi gy Steve gy");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn syntax_assignment_handles_non_ascii_spans() {
    assert_source_assignment("zoi gy café gy");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn syntax_assignment_includes_muhoi_raw_quoted_text_once() {
    let dialect = parse_dialect_definition("(+ZANTUFA-QUOTES)").expect("valid dialect definition");
    let options = ParseOptions::default().with_dialect_definition(&dialect);

    assert_source_assignment_with_options("mi cu mu'oi gy foo gy", &options);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn syntax_assignment_handles_zantufa_jai_tag_term() {
    let dialect = parse_dialect_definition("(+ZANTUFA-TAGS)").expect("valid dialect definition");
    let options = ParseOptions::default().with_dialect_definition(&dialect);

    assert_source_assignment_with_options("jai pu mi cu klama", &options);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn syntax_assignment_handles_zantufa_poiha_brigahi() {
    let dialect =
        parse_dialect_definition("(+ZANTUFA-ADVERBIALS)").expect("valid dialect definition");
    let options = ParseOptions::default().with_dialect_definition(&dialect);

    assert_source_assignment_with_options("noi'a klama ku mi cu broda", &options);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn syntax_assignment_handles_v0_experimental_linkargs() {
    for source in [
        "lo be mi broda cu melbi",
        "lo be broda cu melbi",
        "lo broda be cu melbi",
        "lo broda be mi bei cu melbi",
        "lo broda be bei mi cu melbi",
    ] {
        assert_source_assignment(source);
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn syntax_assignment_handles_v0_zantufa_output_order_cases() {
    let dialect = parse_dialect_definition("(zantufa)").expect("valid dialect definition");
    let options = ParseOptions::default().with_dialect_definition(&dialect);

    for source in [
        "mi klama noi'a broda ku",
        "mi mu'oi gy Alice gy",
        "mi lu'ei do klama li'au",
    ] {
        assert_source_assignment_with_options(source, &options);
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn generated_syntax_assignment_handles_folded_source_islands() {
    for source in [
        "li re gu'e su'i gi pi'i re du li vo",
        "to .ui ba'e cai toi",
        "li fu'a reboi ci pi'i voboi mu pi'i su'i du li rexa",
        "mi ba'e ba'e klama",
    ] {
        assert_generated_model_source_assignment(source);
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn range_order_check_rejects_inverted_final_range() {
    assert!(!ranges_are_strictly_ordered(&[(0, 1), (3, 2)]));
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
    let generated_model =
        parse_syntax_tree_generated_model_with_source_and_options(&words, source, options)
            .expect("source should parse with generated model");

    let morphology = morphology_source_ranges(&words);
    let syntax = syntax_source_ranges(&parse.parse_tree);
    assert_eq!(syntax, morphology);
    assert!(ranges_are_strictly_ordered(&syntax));
    assert!(generated_model_text_syntax_leaf_spans_match_words(
        &words,
        &generated_model
    ));
}

#[requires(!source.is_empty())]
#[ensures(true)]
fn assert_generated_model_source_assignment(source: &str) {
    let words = segment_words_with_options(source);
    let generated_model = parse_syntax_tree_generated_model_with_source_and_options(
        &words,
        source,
        &ParseOptions::default(),
    )
    .expect("source should parse with generated model");
    let morphology = morphology_source_ranges(&words);
    let syntax = generated_model_syntax_source_ranges(&generated_model);
    assert_eq!(syntax, morphology, "{source}");
    assert!(ranges_are_strictly_ordered(&syntax), "{source}");
}

#[requires(!source.is_empty())]
#[ensures(!ret.is_empty())]
fn segment_words_with_options(source: &str) -> Vec<WordLike> {
    segment_words_with_modifiers_with_options_and_source_id(
        source,
        &MorphologyOptions::default(),
        None,
    )
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
fn syntax_source_ranges(tree: &jbotci_syntax::TextSyntax) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    tree.visit_source_spans(&mut |span| ranges.push(span_range(span)));
    ranges
}

#[requires(true)]
#[ensures(true)]
fn generated_model_syntax_source_ranges(
    tree: &jbotci_syntax::generated_model::TextSyntax,
) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    tree.visit_source_spans(&mut |span| ranges.push(span_range(span)));
    ranges
}

#[requires(true)]
#[ensures(true)]
fn span_range(span: &SourceSpan) -> (usize, usize) {
    (span.byte_start, span.byte_end)
}

#[requires(true)]
#[ensures(true)]
fn ranges_are_strictly_ordered(ranges: &[(usize, usize)]) -> bool {
    ranges.iter().all(|(start, end)| start <= end)
        && ranges.windows(2).all(|pair| pair[0].1 <= pair[1].0)
}

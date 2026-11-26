//! `gentufa --format raw` prints the syntax tree with `{tree:#?}`, or with
//! `{tree:?}` when the indent is 0. The files next to this module are that
//! output on main. `WithIndicators` is inside the tree, so a hand-written
//! `Debug` that drops the alternate flag fails the pretty file.

use std::fmt;

use bityzba::{data, invariant, requires};
use jbotci_morphology::WordLike;
use jbotci_tree::debug_emit::Emit;

use crate::WithIndicatorsData;
use crate::tree::WithIndicators;

/// The test value writes no text. The struct holds no data.
#[invariant(true)]
struct EmptyLeaf;

impl fmt::Debug for EmptyLeaf {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, _formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

/// The test value writes one fixed line. The struct holds no data.
#[invariant(true)]
struct TrailingNewlineLeaf;

impl fmt::Debug for TrailingNewlineLeaf {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CustomLeaf\n")
    }
}

/// A standard `Debug` painter for one borrowed value.
///
/// The borrowed value already checks its own shape. This struct holds no
/// extra state, so it has no tighter invariant.
#[invariant(true)]
struct StdView<'a, T: ?Sized> {
    value: &'a T,
    paint: fn(&T, &mut fmt::Formatter<'_>) -> fmt::Result,
}

impl<T: ?Sized> fmt::Debug for StdView<'_, T> {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.paint)(self.value, formatter)
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn raw_debug_matches_the_derived_form() {
    let samples = [
        (
            "mi ui",
            include_str!("testdata/mi-ui.compact.txt"),
            include_str!("testdata/mi-ui.pretty.txt"),
        ),
        (
            "mi ui ui",
            include_str!("testdata/mi-ui-ui.compact.txt"),
            include_str!("testdata/mi-ui-ui.pretty.txt"),
        ),
        (
            "ba'e broda",
            include_str!("testdata/bahe-broda.compact.txt"),
            include_str!("testdata/bahe-broda.pretty.txt"),
        ),
    ];
    for (input, compact, pretty) in samples {
        let tree = syntax_tree(input);
        let got_compact = format!("{tree:?}\n");
        let got_pretty = format!("{tree:#?}\n");
        assert_eq!(got_compact, compact, "compact {input}");
        assert_eq!(got_pretty, pretty, "pretty {input}");
        assert_ne!(compact, pretty, "{input}");
    }
    assert_indicators_match_std();
    assert_emit_list_matches_std(&EmptyLeaf);
    assert_emit_list_matches_std(&TrailingNewlineLeaf);
}

#[requires(true)]
#[ensures(true)]
fn paint_indicators<T: fmt::Debug>(
    value: &WithIndicators<T>,
    formatter: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    match value.as_data() {
        data!(WithIndicators::Plain(inner)) => formatter.debug_tuple("Plain").field(inner).finish(),
        data!(WithIndicators::Emphasized {
            bahe,
            extra_bahe,
            word_like,
        }) => formatter
            .debug_struct("Emphasized")
            .field("bahe", bahe)
            .field("extra_bahe", extra_bahe)
            .field("word_like", word_like)
            .finish(),
        data!(WithIndicators::WithIndicator {
            base,
            indicator_bahe,
            indicator,
            nai_bahe,
            nai,
        }) => formatter
            .debug_struct("WithIndicator")
            .field(
                "base",
                &StdView {
                    value: base.as_ref(),
                    paint: paint_indicators::<T>,
                },
            )
            .field("indicator_bahe", indicator_bahe)
            .field("indicator", indicator)
            .field("nai_bahe", nai_bahe)
            .field("nai", nai)
            .finish(),
    }
}

#[requires(true)]
#[ensures(true)]
fn assert_one_indicators<T: fmt::Debug>(value: &WithIndicators<T>) {
    let view = StdView {
        value,
        paint: paint_indicators::<T>,
    };
    assert_eq!(format!("{value:?}"), format!("{view:?}"));
    assert_eq!(format!("{value:#?}"), format!("{view:#?}"));
}

#[requires(true)]
#[ensures(true)]
fn bare_word(input: &str) -> jbotci_morphology::Word {
    let words = jbotci_morphology::segment_words_with_modifiers(input).expect(input);
    let [word] = words.as_slice() else {
        panic!("{input} produced {words:?}");
    };
    word.bare_word()
        .unwrap_or_else(|| panic!("{input} is not one bare word"))
        .clone()
}

#[requires(!input.is_empty())]
#[ensures(true)]
fn one_word_like(input: &str) -> WordLike {
    let mut words = jbotci_morphology::segment_words_with_modifiers(input).expect(input);
    assert_eq!(words.len(), 1, "{input}");
    words.pop().expect(input)
}

#[requires(true)]
#[ensures(true)]
fn assert_indicators_match_std() {
    let broda = one_word_like("broda");
    let bahe = bare_word("ba'e");
    let extra = bare_word("ba'e");
    let indicator_bahe = bare_word("ba'e");
    let nai_bahe = bare_word("za'e");
    let indicator = bare_word("ui");
    let nai = bare_word("nai");
    let emphasized = WithIndicators::emphasized_with_extra_bahe(bahe, vec![extra], broda.clone());
    match emphasized.as_data() {
        data!(WithIndicators::Emphasized { extra_bahe, .. }) => {
            assert!(!extra_bahe.is_empty());
        }
        _ => panic!("expected emphasized indicators"),
    }
    let indicated = WithIndicators::with_indicator_with_modifiers(
        WithIndicators::bare(broda),
        vec![indicator_bahe],
        indicator,
        vec![nai_bahe],
        Some(nai),
    );
    match indicated.as_data() {
        data!(WithIndicators::WithIndicator {
            indicator_bahe,
            nai_bahe,
            nai,
            ..
        }) => {
            assert!(!indicator_bahe.is_empty());
            assert!(!nai_bahe.is_empty());
            assert!(nai.is_some());
        }
        _ => panic!("expected an indicator layer"),
    }
    assert_one_indicators(&emphasized);
    assert_one_indicators(&indicated);
    assert_one_indicators(&WithIndicators::bare(EmptyLeaf));
    assert_one_indicators(&WithIndicators::bare(TrailingNewlineLeaf));
    assert_one_indicators(&WithIndicators::emphasized_with_extra_bahe(
        bare_word("ba'e"),
        vec![bare_word("ba'e")],
        EmptyLeaf,
    ));
    assert_one_indicators(&WithIndicators::with_indicator_with_modifiers(
        WithIndicators::bare(TrailingNewlineLeaf),
        vec![bare_word("ba'e")],
        bare_word("ui"),
        vec![bare_word("za'e")],
        Some(bare_word("nai")),
    ));
}

#[requires(true)]
#[ensures(true)]
fn paint_debug_list<T: fmt::Debug>(value: &T, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.debug_list().entry(value).entry(value).finish()
}

#[requires(true)]
#[ensures(true)]
fn assert_emit_list_matches_std<T: fmt::Debug>(value: &T) {
    for pretty in [false, true] {
        let mut emit = Emit::new(pretty);
        emit.begin_list();
        emit.begin_item();
        emit.debug_value(value);
        emit.end_item();
        emit.begin_item();
        emit.debug_value(value);
        emit.end_item();
        emit.end_list();
        let view = StdView {
            value,
            paint: paint_debug_list::<T>,
        };
        let expect = if pretty {
            format!("{view:#?}")
        } else {
            format!("{view:?}")
        };
        assert_eq!(emit.finish(), expect, "pretty={pretty}");
    }
}

#[requires(true)]
#[ensures(true)]
fn syntax_tree(input: &str) -> Box<crate::TextSyntax> {
    let recovered =
        jbotci_morphology::segment_words_with_modifiers_recovered_with_options_and_source_id(
            input,
            &jbotci_morphology::MorphologyOptions::default(),
            Some(jbotci_source::SourceId("<input>".to_owned())),
        );
    assert!(
        recovered.errors.is_empty(),
        "{input} {:?}",
        recovered.errors
    );
    let attempt = crate::parse_syntax_tree_with_recovery_with_source_and_options_attempt(
        &recovered.words,
        input,
        &crate::ParseOptions::default(),
    );
    match attempt.result.into_data() {
        data!(crate::SyntaxRecoveryParse::Valid { parse }) => parse.into_data().parse_tree,
        other => panic!("expected a valid parse of {input}: {other:?}"),
    }
}

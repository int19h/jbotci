#[allow(unused_imports)]
use bityzba::{ensures, requires};
use jbotci_budget::{ByteLimit, TextScope};
use jbotci_morphology::{GlideMark, PhonemeRenderOptions, Phonemes, StressMark};

#[test]
#[requires(true)]
#[ensures(true)]
fn phoneme_writer_preserves_each_mark_configuration() {
    for (stress, glides, expected) in [
        (StressMark::Acute, GlideMark::Breve, "áĭŭ"),
        (StressMark::Caps, GlideMark::None, "Aiu"),
        (StressMark::None, GlideMark::None, "aiu"),
    ] {
        let mut output = String::from("prefix ");
        Phonemes::render_canonical_to(
            "áĭŭ",
            PhonemeRenderOptions {
                mark_stress: stress,
                mark_glides: glides,
            },
            &mut output,
        )
        .unwrap();
        assert_eq!(output.strip_prefix("prefix ").unwrap(), expected);
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn phoneme_writer_respects_the_utf8_bound_without_a_truncated_success() {
    for bytes in [5, 6] {
        let scope = TextScope::new(ByteLimit::new(bytes).unwrap());
        let mut output = scope.buffer();
        let result =
            Phonemes::render_canonical_to("áĭŭ", PhonemeRenderOptions::default(), &mut output);
        if bytes == 6 {
            result.unwrap();
            assert_eq!(scope.finish(output.into_piece()).unwrap(), "áĭŭ");
        } else {
            assert!(result.is_err());
            assert!(scope.finish(output.into_piece()).is_err());
        }
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn invalid_phonemes_leave_the_writer_unchanged() {
    for input in ["", "a?"] {
        let mut output = String::from("kept");
        assert!(
            Phonemes::render_canonical_to(input, PhonemeRenderOptions::default(), &mut output)
                .is_err()
        );
        assert_eq!(output, "kept");
    }
}

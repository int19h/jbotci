use bityzba::requires;
use jbotci_morphology::lex_single_word;

#[test]
#[requires(true)]
#[ensures(true)]
fn lexical_single_word_identity_preserves_written_boundaries() {
    for text in [
        "lo.jban.girzu",
        "KLAmami",
        "LOjbangirzu",
        "me.hoi",
        "ло.жбан.гирзу",
        "\u{ed84}\u{eda3}.\u{ed96}\u{ed90}\u{eda0}\u{ed97}.\u{ed92}\u{eda2}\u{ed94}\u{ed95}\u{eda4}",
    ] {
        assert!(lex_single_word(text).is_none(), "{text}");
    }
    for text in [
        ".klama.",
        " klama ",
        "LojbangIrzu",
        "lo,jban,girzu",
        "mehoi",
        "ложбангирзу",
        "\u{ed84}\u{eda3}\u{ed96}\u{ed90}\u{eda0}\u{ed97}\u{ed92}\u{eda2}\u{ed94}\u{ed95}\u{eda4}",
    ] {
        assert!(lex_single_word(text).is_some(), "{text}");
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn lexical_single_word_identity_precedes_magic_word_fault_recovery() {
    let marker = lex_single_word("me'oi").expect("the quote marker is one lexical word");
    assert_eq!(marker.selmaho(), Some("MEhOI"));
    for text in [
        "bu si",
        "zei si",
        "le'u si",
        "bu sa klama",
        "zei sa klama",
        "me'oi klama",
    ] {
        assert!(lex_single_word(text).is_none(), "{text}");
    }
}

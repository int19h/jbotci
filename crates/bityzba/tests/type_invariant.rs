/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use bityzba::{data, invariant, new, try_new};
use serde::{Deserialize, Serialize};

#[invariant(start <= end, "span bounds must be ordered")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Span {
    pub start: usize,
    pub end: usize,
    pub label: String,
}

#[invariant(::Named => !name.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
enum Choice {
    Unset,
    Named { name: String },
}

#[invariant(::Pair(label, _) => !label.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
enum TupleChoice {
    Unset,
    Pair(String, usize),
}

#[invariant(::Leaf => !label.is_empty())]
#[invariant(::Branch => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum Tree {
    Leaf { label: String },
    Branch { children: Vec<Tree> },
}

#[invariant(start <= end)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct CustomSpan {
    start: usize,
    end: usize,
}

#[invariant(::Named { name } => !name.is_empty())]
#[invariant(::Pair(label, count) => !label.is_empty() && *count > 0)]
#[bityzba::expensive_invariant(::Expensive => *value > 0)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum PatternChoice {
    Named { name: String },
    Pair(String, usize),
    Expensive { value: usize },
    Empty,
}

#[invariant(true)]
#[derive(Debug, PartialEq, Eq)]
struct PlainMarker {
    pub value: usize,
}

#[invariant(::Named => true)]
#[derive(Debug, PartialEq, Eq)]
enum PlainChoice {
    Empty,
    Named { name: String },
}

impl CustomSpan {
    fn new(start: usize, end: usize) -> Result<Self, &'static str> {
        if start > end {
            return Err("inverted span");
        }
        Ok(Self::from_data(data!(CustomSpan { start, end })))
    }
}

#[test]
fn constructs_valid_struct_from_data() {
    let span = new!(Span {
        start: 1,
        end: 3,
        label: String::from("sumti"),
    });

    assert_eq!(span.start, 1);
    assert_eq!(span.end, 3);

    let data!(Span { start, label, .. }) = span.as_data();
    assert_eq!(*start, 1);
    assert_eq!(label, "sumti");
}

#[test]
fn rejects_invalid_struct_data() {
    let error = try_new!(Span {
        start: 3,
        end: 1,
        label: String::from("bad"),
    })
    .expect_err("invalid bounds");

    assert!(error.to_string().contains("span bounds must be ordered"));
}

#[test]
fn updates_by_revalidating_whole_value() {
    let span = new!(Span {
        start: 1,
        end: 3,
        label: String::from("sumti"),
    });

    let updated = span.with_data(data! {
        end: 4,
    });

    assert_eq!(updated.end, 4);
    let panic = std::panic::catch_unwind(|| {
        let _ = updated.with_data(data! {
            end: 0,
        });
    });
    assert!(panic.is_err());
}

#[test]
fn serde_deserialization_validates_invariant() {
    let span = new!(Span {
        start: 1,
        end: 3,
        label: String::from("sumti"),
    });

    let json = serde_json::to_string(&span).expect("serialize");
    assert_eq!(
        serde_json::from_str::<Span>(&json).expect("deserialize"),
        span
    );

    let invalid = r#"{"start":3,"end":1,"label":"bad"}"#;
    assert!(serde_json::from_str::<Span>(invalid).is_err());
}

#[test]
fn enum_data_conversion_validates_invariant() {
    assert!(try_new!(Choice::Unset).is_ok());
    assert!(
        try_new!(Choice::Named {
            name: String::new()
        })
        .is_err()
    );

    let choice = new!(Choice::Named {
        name: String::from("cmavo"),
    });

    match choice.as_data() {
        data!(Choice::Named { name }) => assert_eq!(name, "cmavo"),
        data!(Choice::Unset) => panic!("wrong variant"),
    }
}

#[test]
fn tuple_enum_variants_construct_and_match_through_macros() {
    let unset = new!(TupleChoice::Unset);
    assert!(matches!(unset.as_data(), data!(TupleChoice::Unset)));

    let choice = new!(TupleChoice::Pair(String::from("cmavo"), 2));

    match choice.as_data() {
        data!(TupleChoice::Pair(label, count)) => {
            assert_eq!(label, "cmavo");
            assert_eq!(*count, 2);
        }
        data!(TupleChoice::Unset) => panic!("wrong variant"),
    }

    assert!(try_new!(TupleChoice::Pair(String::new(), 2)).is_err());
}

#[test]
fn enum_variant_invariants_accept_explicit_patterns_and_unit_variants() {
    assert!(
        try_new!(PatternChoice::Named {
            name: String::new()
        })
        .is_err()
    );
    assert!(try_new!(PatternChoice::Pair(String::from("cmavo"), 0)).is_err());
    assert!(try_new!(PatternChoice::Empty).is_ok());

    let value = new!(PatternChoice::Pair(String::from("cmavo"), 1));
    match value.as_data() {
        data!(PatternChoice::Pair(label, count)) => {
            assert_eq!(label, "cmavo");
            assert_eq!(*count, 1);
        }
        _ => panic!("wrong variant"),
    }

    #[cfg(not(feature = "expensive_contracts"))]
    {
        let expensive = new!(PatternChoice::Expensive { value: 0 });
        assert!(matches!(
            expensive.as_data(),
            data!(PatternChoice::Expensive { value }) if *value == 0
        ));
    }

    #[cfg(feature = "expensive_contracts")]
    {
        assert!(try_new!(PatternChoice::Expensive { value: 0 }).is_err());
        let expensive = new!(PatternChoice::Expensive { value: 1 });
        assert!(matches!(
            expensive.as_data(),
            data!(PatternChoice::Expensive { value }) if *value == 1
        ));
    }
}

#[test]
fn recursive_enum_invariants_validate_children() {
    let leaf = new!(Tree::Leaf {
        label: String::from("leaf"),
    });
    let tree = new!(Tree::Branch {
        children: vec![leaf],
    });

    match tree.as_data() {
        data!(Tree::Branch { children }) => {
            assert_eq!(children.len(), 1);
            match children[0].as_data() {
                data!(Tree::Leaf { label }) => assert_eq!(label, "leaf"),
                data!(Tree::Branch { .. }) => panic!("wrong child variant"),
            }
        }
        data!(Tree::Leaf { .. }) => panic!("wrong root variant"),
    }

    let invalid = r#"{"kind":"leaf","label":""}"#;
    assert!(serde_json::from_str::<Tree>(invalid).is_err());
}

#[test]
fn type_invariant_allows_user_defined_new_constructor() {
    let span = CustomSpan::new(1, 3).expect("valid custom span");
    assert_eq!(span.start, 1);
    assert_eq!(span.end, 3);
    assert_eq!(CustomSpan::new(3, 1), Err("inverted span"));
}

#[test]
fn true_type_invariant_is_only_a_marker() {
    let mut marker = PlainMarker { value: 1 };
    marker.value = 2;
    let PlainMarker { value } = marker;
    assert_eq!(value, 2);

    let choice = PlainChoice::Named {
        name: String::from("cmavo"),
    };
    assert!(matches!(choice, PlainChoice::Named { .. }));
    assert!(matches!(PlainChoice::Empty, PlainChoice::Empty));
}

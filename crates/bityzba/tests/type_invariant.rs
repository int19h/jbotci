/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use bityzba::{data, invariant, new, try_new};
use serde::{Deserialize, Serialize};

#[invariant(self.start <= self.end, "span bounds must be ordered")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Span {
    pub start: usize,
    pub end: usize,
    pub label: String,
}

#[invariant(matches!(self.as_data(), ChoiceData::Named { name } if !name.is_empty()) || matches!(self.as_data(), ChoiceData::Unset))]
#[derive(Debug, Clone, PartialEq, Eq)]
enum Choice {
    Unset,
    Named { name: String },
}

#[invariant(matches!(self.as_data(), TupleChoiceData::Pair(label, _) if !label.is_empty()) || matches!(self.as_data(), TupleChoiceData::Unset))]
#[derive(Debug, Clone, PartialEq, Eq)]
enum TupleChoice {
    Unset,
    Pair(String, usize),
}

#[invariant(tree_data_is_valid(self.as_data()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum Tree {
    Leaf { label: String },
    Branch { children: Vec<Tree> },
}

#[invariant(self.start <= self.end)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct CustomSpan {
    start: usize,
    end: usize,
}

impl CustomSpan {
    fn new(start: usize, end: usize) -> Result<Self, &'static str> {
        if start > end {
            return Err("inverted span");
        }
        Ok(Self::from_data(data!(CustomSpan {
            start: start,
            end: end,
        })))
    }
}

fn tree_data_is_valid(data: &TreeData) -> bool {
    match data {
        data!(Tree::Leaf { label }) => !label.is_empty(),
        data!(Tree::Branch { children }) => children
            .iter()
            .all(|child| tree_data_is_valid(child.as_data())),
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

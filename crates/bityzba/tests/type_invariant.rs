/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use bityzba::{fields, invariant};
use serde::{Deserialize, Serialize};

#[invariant(self.start <= self.end, "span bounds must be ordered")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Span {
    pub start: usize,
    pub end: usize,
    pub label: String,
}

#[invariant(matches!(self.as_raw(), ChoiceRaw::Named { name } if !name.is_empty()) || matches!(self.as_raw(), ChoiceRaw::Unset))]
#[derive(Debug, Clone, PartialEq, Eq)]
enum Choice {
    Unset,
    Named { name: String },
}

#[invariant(tree_raw_is_valid(self.as_raw()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum Tree {
    Leaf { label: String },
    Branch { children: Vec<Tree> },
}

#[invariant(self.start <= self.end)]
#[bityzba(no_new)]
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
        Ok(Self::from_raw(fields!(CustomSpan {
            start: start,
            end: end,
        })))
    }
}

fn tree_raw_is_valid(raw: &TreeRaw) -> bool {
    match raw {
        fields!(Tree::Leaf { label }) => !label.is_empty(),
        fields!(Tree::Branch { children }) => children
            .iter()
            .all(|child| tree_raw_is_valid(child.as_raw())),
    }
}

#[test]
fn constructs_valid_struct_from_fields() {
    let span = Span::new(fields! {
        start: 1,
        end: 3,
        label: String::from("sumti"),
    });

    assert_eq!(span.start, 1);
    assert_eq!(span.end, 3);

    let fields!(Span { start, label, .. }) = span.as_raw();
    assert_eq!(*start, 1);
    assert_eq!(label, "sumti");
}

#[test]
fn rejects_invalid_struct_fields() {
    let error = Span::try_from_raw(fields!(Span {
        start: 3,
        end: 1,
        label: String::from("bad"),
    }))
    .expect_err("invalid bounds");

    assert!(error.to_string().contains("span bounds must be ordered"));
}

#[test]
fn updates_by_revalidating_whole_value() {
    let span = Span::new(fields! {
        start: 1,
        end: 3,
        label: String::from("sumti"),
    });

    let updated = span.with_fields(fields! {
        end: 4,
    });

    assert_eq!(updated.end, 4);
    let panic = std::panic::catch_unwind(|| {
        let _ = updated.with_fields(fields! {
            end: 0,
        });
    });
    assert!(panic.is_err());
}

#[test]
fn serde_deserialization_validates_invariant() {
    let span = Span::new(fields! {
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
fn enum_raw_conversion_validates_invariant() {
    assert!(Choice::try_from_raw(fields!(Choice::Unset)).is_ok());
    assert!(
        Choice::try_from_raw(fields!(Choice::Named {
            name: String::new()
        }))
        .is_err()
    );

    let choice = Choice::from_raw(fields!(Choice::Named {
        name: String::from("cmavo"),
    }));

    match choice.as_raw() {
        fields!(Choice::Named { name }) => assert_eq!(name, "cmavo"),
        fields!(Choice::Unset) => panic!("wrong variant"),
    }
}

#[test]
fn recursive_enum_invariants_validate_children() {
    let leaf = Tree::from_raw(fields!(Tree::Leaf {
        label: String::from("leaf"),
    }));
    let tree = Tree::from_raw(fields!(Tree::Branch {
        children: vec![leaf],
    }));

    match tree.as_raw() {
        fields!(Tree::Branch { children }) => {
            assert_eq!(children.len(), 1);
            match children[0].as_raw() {
                fields!(Tree::Leaf { label }) => assert_eq!(label, "leaf"),
                fields!(Tree::Branch { .. }) => panic!("wrong child variant"),
            }
        }
        fields!(Tree::Leaf { .. }) => panic!("wrong root variant"),
    }

    let invalid = r#"{"kind":"leaf","label":""}"#;
    assert!(serde_json::from_str::<Tree>(invalid).is_err());
}

#[test]
fn type_invariant_can_skip_generated_new_for_custom_constructor() {
    let span = CustomSpan::new(1, 3).expect("valid custom span");
    assert_eq!(span.start, 1);
    assert_eq!(span.end, 3);
    assert_eq!(CustomSpan::new(3, 1), Err("inverted span"));
}

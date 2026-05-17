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

#[test]
fn constructs_valid_struct_from_fields() {
    let span = Span::try_from_fields(fields! {
        start: 1,
        end: 3,
        label: String::from("sumti"),
    })
    .expect("valid span");

    assert_eq!(span.start, 1);
    assert_eq!(span.end, 3);

    let fields!(Span { start, label, .. }) = span.as_raw();
    assert_eq!(*start, 1);
    assert_eq!(label, "sumti");
}

#[test]
fn rejects_invalid_struct_fields() {
    let error = Span::try_from_fields(fields! {
        start: 3,
        end: 1,
        label: String::from("bad"),
    })
    .expect_err("invalid bounds");

    assert!(error.to_string().contains("span bounds must be ordered"));
}

#[test]
fn updates_by_revalidating_whole_value() {
    let span = Span::try_from_fields(fields! {
        start: 1,
        end: 3,
        label: String::from("sumti"),
    })
    .expect("valid span");

    let updated = span
        .try_with_fields(fields! {
            end: 4,
        })
        .expect("valid update");

    assert_eq!(updated.end, 4);
    assert!(updated.try_with_fields(fields! { end: 0 }).is_err());
}

#[test]
fn serde_deserialization_validates_invariant() {
    let span = Span::try_from_fields(fields! {
        start: 1,
        end: 3,
        label: String::from("sumti"),
    })
    .expect("valid span");

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
    assert!(Choice::try_from_raw(ChoiceRaw::Unset).is_ok());
    assert!(
        Choice::try_from_raw(ChoiceRaw::Named {
            name: String::new()
        })
        .is_err()
    );

    let choice = Choice::try_from_raw(ChoiceRaw::Named {
        name: String::from("cmavo"),
    })
    .expect("valid choice");

    match choice.as_raw() {
        fields!(Choice::Named { name }) => assert_eq!(name, "cmavo"),
        fields!(Choice::Unset) => panic!("wrong variant"),
    }
}

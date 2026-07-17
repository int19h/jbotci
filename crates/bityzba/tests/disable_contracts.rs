/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use bityzba::{
    data, ensures, expensive_invariant, expensive_requires, invariant, new, requires, try_new,
};

#[requires(value > 0, "cheap precondition")]
fn require_positive(value: i32) -> i32 {
    value
}

#[ensures(ret > 0, "cheap postcondition")]
fn return_non_positive() -> i32 {
    0
}

#[invariant(*value > 0, "cheap function invariant")]
fn leave_unchanged(value: &mut i32) {
    let _ = value;
}

#[invariant(*value > 0, "cheap type invariant")]
#[derive(Debug, PartialEq, Eq)]
struct Positive {
    value: i32,
}

#[expensive_requires(value > 0, "expensive precondition")]
fn expensive_require_positive(value: i32) -> i32 {
    value
}

#[expensive_invariant(*value > 0, "expensive type invariant")]
#[derive(Debug, PartialEq, Eq)]
struct ExpensivePositive {
    value: i32,
}

#[test]
fn cheap_function_contracts_follow_disable_switch() {
    let postcondition = std::panic::catch_unwind(return_non_positive);
    let function_invariant = std::panic::catch_unwind(|| {
        let mut value = 0;
        leave_unchanged(&mut value);
    });

    let contracts_are_disabled = cfg!(feature = "disable_contracts");
    assert_eq!(postcondition.is_ok(), contracts_are_disabled);
    assert_eq!(function_invariant.is_ok(), contracts_are_disabled);
}

#[cfg(not(feature = "disable_contracts"))]
#[test]
#[should_panic(expected = "cheap precondition")]
fn violated_contract_panics_without_disable_switch() {
    let _ = require_positive(0);
}

#[cfg(feature = "disable_contracts")]
#[test]
fn violated_contract_passes_with_disable_switch() {
    assert_eq!(require_positive(0), 0);
}

#[test]
fn cheap_type_validation_paths_follow_disable_switch() {
    let contracts_are_disabled = cfg!(feature = "disable_contracts");

    let fallible = try_new!(Positive { value: 0 });
    assert_eq!(fallible.is_ok(), contracts_are_disabled);

    let panicking = std::panic::catch_unwind(|| new!(Positive { value: 0 }));
    assert_eq!(panicking.is_ok(), contracts_are_disabled);

    let direct_data = Positive::try_from_data(data!(Positive { value: 0 }));
    assert_eq!(direct_data.is_ok(), contracts_are_disabled);

    let value = new!(Positive { value: 1 });
    let update = std::panic::catch_unwind(|| {
        value.with_data(data! {
            value: 0,
        })
    });
    assert_eq!(update.is_ok(), contracts_are_disabled);
}

#[test]
fn expensive_contracts_remain_independent() {
    let precondition = std::panic::catch_unwind(|| expensive_require_positive(0));
    let type_invariant = try_new!(ExpensivePositive { value: 0 });
    let expensive_contracts_are_enabled = cfg!(feature = "expensive_contracts");

    assert_eq!(precondition.is_err(), expensive_contracts_are_enabled);
    assert_eq!(type_invariant.is_err(), expensive_contracts_are_enabled);
}

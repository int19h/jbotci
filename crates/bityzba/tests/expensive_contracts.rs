/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#![deny(unused_parens)]

use bityzba::*;
#[contract_trait]
trait ExpensiveMapper {
    #[expensive_requires(value > 0, "expensive trait precondition")]
    #[expensive_ensures(value > 0 -> ret > 0, "expensive trait postcondition")]
    fn map(&self, value: i32) -> i32;
}

#[expensive_invariant(self.value.is_multiple_of(2), "expensive type invariant")]
struct ExpensiveEven {
    value: usize,
}

struct BadMapper;

#[contract_trait]
impl ExpensiveMapper for BadMapper {
    fn map(&self, value: i32) -> i32 {
        -value
    }
}

#[cfg(not(feature = "expensive_contracts"))]
#[test]
fn expensive_trait_contracts_are_disabled_without_feature() {
    let mapper: &dyn ExpensiveMapper = &BadMapper;
    assert_eq!(mapper.map(1), -1);
    assert_eq!(mapper.map(0), 0);

    let _ = new!(ExpensiveEven { value: 3 });
}

#[cfg(not(feature = "expensive_contracts"))]
#[test]
fn disabled_expensive_old_capture_is_typechecked_but_not_evaluated() {
    fn panic_if_called() -> bool {
        panic!("disabled expensive old() capture was evaluated");
    }

    #[expensive_ensures(old(panic_if_called()))]
    fn no_op() {}

    #[expensive_ensures(old(value.clone()).len() == ret.len())]
    fn consume(value: String) -> String {
        value
    }

    no_op();
    assert_eq!(consume(String::from("abc")), "abc");
}

#[cfg(feature = "expensive_contracts")]
#[test]
#[should_panic(expected = "expensive trait precondition")]
fn expensive_trait_precondition_is_checked_with_feature() {
    let mapper = BadMapper;
    let _ = mapper.map(0);
}

#[cfg(feature = "expensive_contracts")]
#[test]
#[should_panic(expected = "expensive trait postcondition")]
fn expensive_trait_postcondition_is_checked_with_feature() {
    let mapper: &dyn ExpensiveMapper = &BadMapper;
    let _ = mapper.map(1);
}

#[cfg(feature = "expensive_contracts")]
#[test]
fn expensive_type_invariant_is_checked_with_feature() {
    assert!(try_new!(ExpensiveEven { value: 3 }).is_err());
}

#[test]
fn implication_in_expensive_call_argument() {
    #[expensive_ensures(std::convert::identity(value > 0 -> ret))]
    fn check_positive(value: i32) -> bool {
        value > 1
    }

    assert!(!check_positive(0));
    assert!(check_positive(2));
    assert_eq!(
        std::panic::catch_unwind(|| check_positive(1)).is_err(),
        cfg!(feature = "expensive_contracts")
    );
}

#[test]
fn implication_in_expensive_old_capture() {
    #[expensive_ensures(old(*premise -> conclusion) == ret)]
    fn set_premise(premise: &mut bool, conclusion: bool) -> bool {
        let result = !*premise || conclusion;
        *premise = true;
        result
    }

    let mut premise = false;
    assert!(set_premise(&mut premise, false));
    assert!(!set_premise(&mut premise, false));
}

#[test]
fn implication_in_expensive_variant_invariant() {
    #[expensive_invariant(::Pair(premise, conclusion) =>
        std::convert::identity(*premise -> *conclusion)
    )]
    enum Implication {
        Pair(bool, bool),
    }

    assert!(try_new!(Implication::Pair(false, false)).is_ok());
    assert!(try_new!(Implication::Pair(true, true)).is_ok());
    assert_eq!(
        try_new!(Implication::Pair(true, false)).is_err(),
        cfg!(feature = "expensive_contracts")
    );
}

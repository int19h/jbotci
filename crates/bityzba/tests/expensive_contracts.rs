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

#[test]
fn wrapped_implication_capture_keeps_its_binding() {
    macro_rules! wrapped {
        ($name:ident, $contract:ident, $capture:ident $arrow:tt) => {
            #[$contract(std::convert::identity($capture(a $arrow b)) == ret)]
            fn $name(a: bool, b: bool) -> bool {
                !a || b
            }
        };
    }

    wrapped!(ordinary, ensures, old ->);
    wrapped!(expensive, expensive_ensures, old ->);
    assert!(ordinary(false, false));
    assert!(!ordinary(true, false));
    assert!(expensive(false, false));
    assert!(!expensive(true, false));
}

#[test]
fn ordinary_capture_keeps_user_parameter_names() {
    #[requires(ret == 8)]
    #[ensures(old(ret == 8) && ret == 9 && __contract_old_0 == 7)]
    fn ordinary(ret: u32, __contract_old_0: u32) -> u32 {
        ret + 1
    }

    assert_eq!(ordinary(8, 7), 9);
}

#[test]
fn nested_expensive_captures_keep_user_parameter_names() {
    #[expensive_ensures(
        old(old(value) >= __contract_old_0_marker
            && __bityzba_expensive_contract_old_marker == marker)
            == ret
        && __contract_old_0 == 7
        && __contract_old_1 == 8
        && __contract_old_1_marker == 9
        && __bityzba_expensive_contract_old_value == 10
    )]
    #[allow(clippy::too_many_arguments)]
    fn expensive(
        value: u32,
        marker: u32,
        __contract_old_0_marker: u32,
        __contract_old_1_marker: u32,
        __contract_old_0: u32,
        __contract_old_1: u32,
        __bityzba_expensive_contract_old_marker: u32,
        __bityzba_expensive_contract_old_value: u32,
    ) -> bool {
        value >= __contract_old_0_marker && __bityzba_expensive_contract_old_marker == marker
    }

    assert!(expensive(4, 5, 3, 9, 7, 8, 5, 10));
    assert!(!expensive(2, 5, 3, 9, 7, 8, 5, 10));
}

#[test]
fn return_rewrite_uses_the_generated_label() {
    #[ensures(ret == if early { 1 } else { 2 })]
    fn labeled(early: bool) -> u32 {
        'run: {
            if early {
                return 1;
            }
            break 'run;
        }
        2
    }

    assert_eq!(labeled(true), 1);
    assert_eq!(labeled(false), 2);
}

#[test]
fn wildcard_trait_parameter_keeps_the_user_parameter() {
    #[contract_trait]
    trait Forward {
        #[ensures(ret == __bityzba_contract_arg_0)]
        fn forward(&self, _: bool, __bityzba_contract_arg_0: u32) -> u32;
    }

    struct Forwarder;

    #[contract_trait]
    impl Forward for Forwarder {
        fn forward(&self, flag: bool, value: u32) -> u32 {
            assert!(flag);
            value
        }
    }

    assert_eq!(Forwarder.forward(true, 7), 7);
}

#[test]
fn builder_closures_keep_the_user_parameter() {
    #[invariant(*value > 0)]
    struct Positive {
        value: u32,
    }

    fn build(__bityzba_data: u32) -> Positive {
        new!(Positive {
            value: __bityzba_data,
        })
        .with_data(data! { value: __bityzba_data + 1 })
    }

    assert_eq!(build(7).value, 8);
}

#[test]
fn enum_debug_keeps_fields_with_generated_names() {
    #[invariant(::Value => *formatter > 0 && *debug > 0)]
    #[derive(Debug)]
    enum Data {
        Value { formatter: u32, debug: u32 },
    }

    let data = new!(Data::Value {
        formatter: 3,
        debug: 4,
    });
    assert_eq!(format!("{data:?}"), "Value { formatter: 3, debug: 4 }");
}

// The user must write these parentheses. Without them, the closure body, the
// call argument and the `if` condition end before the implication. They must
// not get the `unused_parens` lint, in either feature mode.
#[test]
fn needed_parentheses_around_an_implication_are_not_unused() {
    fn both(first: bool, second: bool) -> bool {
        first && second
    }

    #[requires(true)]
    #[ensures(values.iter().all(|value| (*value > 0 -> ret)))]
    #[expensive_ensures(values.iter().all(|value| (*value > 0 -> ret)))]
    fn closure_body(values: &[u32]) -> bool {
        values.iter().all(|value| *value > 0)
    }

    #[requires(true)]
    #[ensures(both((flag -> ret), flag || !flag))]
    #[expensive_ensures(both((flag -> ret), flag || !flag))]
    fn call_argument(flag: bool) -> bool {
        flag
    }

    #[requires(true)]
    #[ensures(if (flag -> ret) { true } else { false })]
    #[expensive_ensures(if (flag -> ret) { true } else { false })]
    fn if_condition(flag: bool) -> bool {
        flag
    }

    #[requires(true)]
    #[ensures(Some(flag).is_some_and(|value| (value -> ret)))]
    #[expensive_ensures(Some(flag).is_some_and(|value| (value -> ret)))]
    fn method_closure(flag: bool) -> bool {
        flag
    }

    assert!(closure_body(&[1, 2]));
    assert!(!closure_body(&[0]));
    assert!(call_argument(true));
    assert!(!call_argument(false));
    assert!(if_condition(true));
    assert!(method_closure(true));
}

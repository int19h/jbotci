/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use bityzba::*;
#[contract_trait]
trait ExpensiveMapper {
    #[expensive_requires(value > 0, "expensive trait precondition")]
    #[expensive_ensures(ret > 0, "expensive trait postcondition")]
    fn map(&self, value: i32) -> i32;
}

#[expensive_invariant(self.value % 2 == 0, "expensive type invariant")]
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

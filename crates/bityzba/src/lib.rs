/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#![doc = include_str!("../README.md")]

#[cfg(feature = "contract_scanner")]
mod contract_scanner;

#[cfg(feature = "contract_scanner")]
pub use contract_scanner::{ContractScanError, ContractScanner, require_contracts};

pub use bityzba_macros::{
    contract_trait, data, debug_ensures, debug_invariant, debug_requires, ensures,
    expensive_ensures, expensive_invariant, expensive_requires, invariant, new, requires,
    test_ensures, test_invariant, test_requires, try_new,
};

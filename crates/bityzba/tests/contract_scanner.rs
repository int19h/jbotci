/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#![cfg(feature = "contract_scanner")]

use std::path::PathBuf;

use bityzba::ContractScanner;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("contract_scanner")
        .join(name)
}

#[test]
fn complete_contracts_scan_cleanly() {
    ContractScanner::new(fixture("complete"))
        .scan()
        .expect("complete fixture should satisfy scanner");
}

#[test]
fn missing_contracts_report_separate_diagnostics() {
    let error = ContractScanner::new(fixture("missing"))
        .scan()
        .expect_err("missing fixture should fail scanner");
    let output = error.to_string();

    assert!(
        output.contains("src/lib.rs:1: missing bityzba type invariant on struct `MissingType`")
    );
    assert!(output.contains("src/lib.rs:5: missing bityzba type invariant on enum `MissingEnum`"));
    assert!(
        output.contains("src/lib.rs:9: missing bityzba contract_trait on trait `MissingTrait`")
    );
    assert!(output.contains(
        "src/lib.rs:10: missing bityzba precondition on trait method `MissingTrait::parse_term`"
    ));
    assert!(output.contains(
        "src/lib.rs:10: missing bityzba postcondition on trait method `MissingTrait::parse_term`"
    ));
    assert!(
        output.contains("src/lib.rs:13: missing bityzba precondition on function `parse_term`")
    );
    assert!(
        output.contains("src/lib.rs:13: missing bityzba postcondition on function `parse_term`")
    );
    assert!(output.contains("only use `#[requires(true)]` as a last resort"));
    assert!(output.contains("only use `#[ensures(true)]` as a last resort"));
    assert!(output.contains(
        "only use `#[invariant(true)]` when the field types already express the invariant"
    ));
}

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
    assert!(output.contains(
        "src/lib.rs:12: missing bityzba invariant on data-carrying enum variant `MissingVariantInvariant::Present`"
    ));
    assert!(!output.contains("missing bityzba type invariant on enum `MissingEnum`"));
    assert!(
        output.contains("src/lib.rs:15: missing bityzba contract_trait on trait `MissingTrait`")
    );
    assert!(output.contains(
        "src/lib.rs:16: missing bityzba precondition on trait method `MissingTrait::parse_term`"
    ));
    assert!(output.contains(
        "src/lib.rs:16: missing bityzba postcondition on trait method `MissingTrait::parse_term`"
    ));
    assert!(
        output.contains("src/lib.rs:19: missing bityzba precondition on function `parse_term`")
    );
    assert!(
        output.contains("src/lib.rs:19: missing bityzba postcondition on function `parse_term`")
    );
    assert!(output.contains(
        "src/lib.rs:22: Result-returning function `unsound_result_contract` has an `is_ok_and` postcondition without a Result error escape"
    ));
    assert!(output.contains(
        "src/lib.rs:28: Result-returning function `unsound_result_contract_with_unrelated_error_probe` has an `is_ok_and` postcondition without a Result error escape"
    ));
    assert!(output.contains(
        "src/lib.rs:37: Result-returning function `unsound_result_contract_with_nested_ret_error_probe` has an `is_ok_and` postcondition without a Result error escape"
    ));
    assert!(output.contains("only use `#[requires(true)]` as a last resort"));
    assert!(output.contains("only use `#[ensures(true)]` as a last resort"));
    assert!(output.contains(
        "only use `#[invariant(true)]` when the field types already express the invariant"
    ));
}

#[test]
fn function_local_items_are_scanned_recursively() {
    let error = ContractScanner::new(fixture("fn_local"))
        .scan()
        .expect_err("fn-local item fixture should fail scanner");
    let output = error.to_string();

    assert!(output.contains("missing bityzba type invariant on struct `LocalStruct`"));
    assert!(
        output.contains(
            "missing bityzba invariant on data-carrying enum variant `LocalEnum::Present`"
        )
    );
    assert!(output.contains("missing bityzba contract_trait on trait `LocalTrait`"));
    assert!(output.contains("missing bityzba precondition on trait method `LocalTrait::run`"));
    assert!(output.contains("missing bityzba postcondition on trait method `LocalTrait::run`"));
    assert!(output.contains("missing bityzba precondition on function `inner`"));
    assert!(output.contains("missing bityzba postcondition on function `inner`"));
    assert!(output.contains("missing bityzba precondition on method `local_method`"));
    assert!(output.contains("missing bityzba postcondition on method `local_method`"));
    assert!(output.contains("missing bityzba precondition on function `inside_expression`"));
    assert!(output.contains("missing bityzba postcondition on function `inside_expression`"));
    assert!(output.contains("missing bityzba type invariant on struct `InsideTraitImpl`"));
}

#[test]
fn misordered_contracts_report_order_diagnostics() {
    let error = ContractScanner::new(fixture("misordered"))
        .scan()
        .expect_err("misordered fixture should fail scanner");
    let output = error.to_string();

    assert!(output.contains(
        "bityzba contract attribute `requires` appears after `ensures` on function `parse_term`"
    ));
    assert!(output.contains(
        "bityzba contract attribute `requires` appears after `invariant` on enum `MisorderedEnum`"
    ));
    assert!(output.contains(
        "order bityzba contract attributes as `requires`, then `ensures`, then `invariant`"
    ));
}

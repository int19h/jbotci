extern crate bityzba;

use std::env;

#[allow(unused_imports)]
use bityzba::{ensures, requires};

#[path = "src/windows_stack.rs"]
mod windows_stack;

#[requires(true)]
#[ensures(true)]
fn main() {
    bityzba::require_contracts().unwrap();

    let target_os = env::var("CARGO_CFG_TARGET_OS")
        .expect("Cargo must provide CARGO_CFG_TARGET_OS to build scripts");
    let target_env = env::var("CARGO_CFG_TARGET_ENV")
        .expect("Cargo must provide CARGO_CFG_TARGET_ENV to build scripts");
    match windows_stack::windows_stack_link_arg(&target_os, &target_env) {
        Ok(Some(link_arg)) => println!("cargo:rustc-link-arg-bin=jbotci={link_arg}"),
        Ok(None) => {}
        Err(message) => panic!("{message}"),
    }
}

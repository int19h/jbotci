use std::process::ExitCode;

#[allow(unused_imports)]
use bityzba::{ensures, requires};

#[requires(true)]
#[ensures(true)]
fn main() -> ExitCode {
    jbotci_cli::main_entry()
}

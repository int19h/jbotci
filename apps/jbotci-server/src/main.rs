use std::process::ExitCode;

#[allow(unused_imports)]
use bityzba::{ensures, requires};

#[tokio::main]
#[requires(true)]
#[ensures(true)]
async fn main() -> ExitCode {
    match jbotci_server::run_server(jbotci_server::config_from_env()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("jbotci-server: {error}");
            ExitCode::FAILURE
        }
    }
}

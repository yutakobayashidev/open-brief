use std::process::ExitCode;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "openbriefd", version, about = "Run the OpenBrief local daemon")]
struct Cli {}

fn main() -> ExitCode {
    let _cli = Cli::parse();
    match openbrief_app::run_daemon() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("openbriefd: {error}");
            ExitCode::from(1)
        }
    }
}

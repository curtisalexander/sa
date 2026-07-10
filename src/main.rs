use std::process::ExitCode;

use clap::Parser;
use colored::Colorize;

use sa::cli::Args;

fn main() -> ExitCode {
    match sa::run(Args::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{} {error:#}", "error:".bright_red().bold());
            ExitCode::FAILURE
        }
    }
}

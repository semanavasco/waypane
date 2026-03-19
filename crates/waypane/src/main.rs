mod cli;
mod dynamic;
mod lua;
mod modules;
mod shell;
mod widgets;

use crate::{cli::Command, shell::lifecycle};
use anyhow::Result;
use clap::Parser;
use cli::Cli;
use gtk4::glib::ExitCode;

fn main() -> Result<ExitCode> {
    let cli = Cli::parse();

    match cli.command {
        Command::Run {
            config,
            log_level,
            watch_css,
        } => {
            let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&log_level));

            tracing_subscriber::fmt().with_env_filter(env_filter).init();

            let shell = shell::load(&config)?;

            Ok(lifecycle::run_app(shell, watch_css))
        }
        Command::GenStubs => {
            let stubs = lua::gen_stubs()?;
            println!("{stubs}");
            Ok(ExitCode::new(0))
        }
    }
}

use admiral_harness::{Runtime, parse, run};
use clap::Parser;
use std::path::PathBuf;
use std::process::ExitCode;

/// Plays the agent role from a scenario file against a runtime, with no model.
#[derive(Parser)]
#[command(name = "admiral-harness", version)]
struct Args {
    /// The scenario file (scenario/1 JSON)
    scenario: PathBuf,
    /// The runtime program to drive; it must answer `next` and `advance`
    #[arg(long, default_value = "admiral")]
    admiral: PathBuf,
    /// The directory the runtime runs in; the current directory by default
    #[arg(long)]
    dir: Option<PathBuf>,
}

fn main() -> ExitCode {
    let args = Args::parse();
    let text = match std::fs::read_to_string(&args.scenario) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("cannot read {}: {e}", args.scenario.display());
            return ExitCode::from(2);
        }
    };
    let scenario = match parse(&text) {
        Ok(scenario) => scenario,
        Err(e) => {
            eprintln!("{}: {e}", args.scenario.display());
            return ExitCode::from(2);
        }
    };
    let dir = match args.dir.map_or_else(std::env::current_dir, Ok) {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("cannot find the working directory: {e}");
            return ExitCode::from(2);
        }
    };
    let runtime = Runtime {
        program: args.admiral,
        dir,
    };
    match run(&scenario, &runtime) {
        Ok(()) => {
            println!("PASS {}", scenario.name);
            ExitCode::SUCCESS
        }
        Err(e) => {
            println!("FAIL {}: {e}", scenario.name);
            ExitCode::from(1)
        }
    }
}

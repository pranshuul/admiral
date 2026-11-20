use std::path::PathBuf;
use std::process::ExitCode;

use admiral_store::{SourceKind, Sources, StoreError};
use clap::{Args, Parser, Subcommand};
use serde_json::json;

#[derive(Parser)]
#[command(name = "admiral", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Register, update, publish and inspect sources.
    Sources {
        #[command(subcommand)]
        command: SourcesCommand,
    },
}

#[derive(Subcommand)]
enum SourcesCommand {
    /// Clone a source into ~/.admiral/sources/<name> and discover its items.
    Add(AddArgs),
}

#[derive(Args)]
struct AddArgs {
    /// The source's alias; items are named `<name>/<item>`.
    name: String,
    /// The repository to clone.
    url: String,
    /// The source is the user's own: pull merges instead of hard-resetting.
    #[arg(long)]
    authored: bool,
    /// Print JSON instead of text.
    #[arg(long)]
    json: bool,
    /// Check the alias and report what would happen without cloning.
    #[arg(long)]
    dry_run: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        None => ExitCode::SUCCESS,
        Some(Command::Sources {
            command: SourcesCommand::Add(args),
        }) => {
            let as_json = args.json;
            match add(&args) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    if as_json {
                        let error = json!({"error": {
                            "code": e.code, "message": e.message, "remedy": e.remedy}});
                        println!("{error:#}");
                    } else {
                        eprintln!("{e}");
                    }
                    ExitCode::FAILURE
                }
            }
        }
    }
}

fn sources_dir() -> Result<PathBuf, StoreError> {
    match std::env::var_os("HOME") {
        Some(home) if !home.is_empty() => Ok(PathBuf::from(home).join(".admiral/sources")),
        _ => Err(StoreError::new(
            "E1005",
            "HOME is not set, so ~/.admiral cannot be located".to_string(),
            "Set HOME to your home directory and retry.",
        )),
    }
}

fn add(args: &AddArgs) -> Result<(), StoreError> {
    let sources = Sources::new(sources_dir()?);
    let kind = if args.authored {
        SourceKind::Authored
    } else {
        SourceKind::Consumed
    };
    let path = sources.path(&args.name);
    let items = if args.dry_run {
        sources.check_alias(&args.name)?;
        Vec::new()
    } else {
        sources.add(&args.name, &args.url, kind)?.items
    };
    if args.json {
        let items: Vec<_> = items
            .iter()
            .map(|i| {
                json!({"id": i.id(), "kind": i.kind.as_str(),
                    "path": i.path.to_string_lossy()})
            })
            .collect();
        let out = json!({
            "name": args.name, "url": args.url, "kind": kind.as_str(),
            "path": path, "dry_run": args.dry_run, "items": items,
        });
        println!("{out:#}");
    } else if args.dry_run {
        println!(
            "would clone {} into {} as a {} source",
            args.url,
            path.display(),
            kind.as_str()
        );
    } else {
        println!(
            "added {} source {} at {}",
            kind.as_str(),
            args.name,
            path.display()
        );
        for item in &items {
            println!("  {} {}", item.kind.as_str(), item.id());
        }
    }
    Ok(())
}

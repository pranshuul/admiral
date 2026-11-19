use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("schemas-check") if args.len() == 1 => {
            let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
            match xtask::check_schemas(&admiral_model::schemas(), &root.join("schemas")) {
                Ok(()) => {
                    println!("schemas: generated schemas equal committed schemas");
                    ExitCode::SUCCESS
                }
                Err(errors) => {
                    for error in errors {
                        eprintln!("schemas: {error}");
                    }
                    ExitCode::FAILURE
                }
            }
        }
        _ => {
            eprintln!("usage: cargo xtask schemas-check");
            ExitCode::from(2)
        }
    }
}

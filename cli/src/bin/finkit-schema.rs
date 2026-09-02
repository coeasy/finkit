use clap::Parser;
use finkit::formula::compat::FORMULA_TERMINAL_SCHEMA_VERSION;
use finkit::formula::FormulaTerminal;
use finkit::schema::FunctionApiSchema;
use serde::Serialize;
use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "finkit-schema")]
#[command(about = "Export Finkit's canonical function and compatibility metadata as JSON")]
#[command(version)]
struct Args {
    /// Export only one canonical function or alias.
    #[arg(long, conflicts_with = "terminals")]
    function: Option<String>,

    /// Export declared formula-terminal compatibility metadata.
    #[arg(long, conflicts_with = "function")]
    terminals: bool,

    /// Write JSON to a file instead of stdout.
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Emit compact JSON instead of human-readable pretty JSON.
    #[arg(long)]
    compact: bool,
}

#[derive(Serialize)]
struct FunctionEnvelope<'a, T> {
    schema_version: &'a str,
    function: &'a T,
}

#[derive(Serialize)]
struct TerminalSchema {
    schema_version: &'static str,
    terminals: Vec<TerminalSpec>,
}

#[derive(Serialize)]
struct TerminalSpec {
    terminal: &'static str,
    dialect: &'static str,
    compatibility: &'static str,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let json = if args.terminals {
        let schema = TerminalSchema {
            schema_version: FORMULA_TERMINAL_SCHEMA_VERSION,
            terminals: FormulaTerminal::all()
                .iter()
                .copied()
                .map(|terminal| TerminalSpec {
                    terminal: terminal.as_str(),
                    dialect: terminal.canonical_dialect().as_str(),
                    compatibility: terminal.compatibility_level().as_str(),
                })
                .collect(),
        };
        serialize_json(&schema, args.compact)?
    } else {
        let schema = FunctionApiSchema::builtin();
        if let Some(name) = args.function.as_deref() {
            let function = schema
                .get(name)
                .ok_or_else(|| format!("unknown canonical function or alias: {name}"))?;
            let envelope = FunctionEnvelope {
                schema_version: &schema.schema_version,
                function,
            };
            serialize_json(&envelope, args.compact)?
        } else {
            serialize_json(&schema, args.compact)?
        }
    };

    if let Some(path) = args.output {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(path, format!("{json}\n"))?;
    } else {
        let mut stdout = io::stdout().lock();
        stdout.write_all(json.as_bytes())?;
        stdout.write_all(b"\n")?;
    }

    Ok(())
}

fn serialize_json(value: &impl Serialize, compact: bool) -> Result<String, serde_json::Error> {
    if compact {
        serde_json::to_string(value)
    } else {
        serde_json::to_string_pretty(value)
    }
}

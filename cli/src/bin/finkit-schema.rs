use clap::Parser;
use finkit::schema::FunctionApiSchema;
use serde::Serialize;
use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "finkit-schema")]
#[command(about = "Export Finkit's canonical function metadata schema as JSON")]
#[command(version)]
struct Args {
    /// Export only one canonical function or alias.
    #[arg(long)]
    function: Option<String>,

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

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let schema = FunctionApiSchema::builtin();

    let json = if let Some(name) = args.function.as_deref() {
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

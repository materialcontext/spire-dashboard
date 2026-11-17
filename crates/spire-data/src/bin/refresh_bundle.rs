//! Regenerates `data/catalog.json`, the data compiled into the app.
//!
//! ```text
//! cargo run -p spire-data --bin refresh-bundle                    # fetch from Spire Codex
//! cargo run -p spire-data --bin refresh-bundle -- --from-dir DIR  # use saved <endpoint>.json files
//! ```
//!
//! Options: `--base-url URL`, `--out PATH` (default `data/catalog.json`).

use std::path::PathBuf;
use std::process::ExitCode;

use spire_data::codex::{self, Responses};
use spire_data::{DataError, fetch, now_rfc3339};

struct Args {
    from_dir: Option<PathBuf>,
    base_url: String,
    out: PathBuf,
}

fn parse_args() -> Result<Args, String> {
    let mut args =
        Args { from_dir: None, base_url: codex::DEFAULT_BASE_URL.into(), out: PathBuf::from("data/catalog.json") };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut value = || it.next().ok_or(format!("{flag} needs a value"));
        match flag.as_str() {
            "--from-dir" => args.from_dir = Some(PathBuf::from(value()?)),
            "--base-url" => args.base_url = value()?,
            "--out" => args.out = PathBuf::from(value()?),
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(args)
}

fn read_dir(dir: &std::path::Path) -> Result<(Responses, Option<String>), DataError> {
    let mut responses = Responses::new();
    for endpoint in codex::ENDPOINTS {
        responses.insert(endpoint, std::fs::read_to_string(dir.join(format!("{endpoint}.json")))?);
    }
    let version =
        std::fs::read_to_string(dir.join("versions.json")).ok().and_then(|b| codex::latest_stable_version(&b));
    Ok((responses, version))
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let (responses, version) = match &args.from_dir {
        Some(dir) => read_dir(dir),
        None => fetch::codex_responses(&args.base_url),
    }
    .map_err(|e| e.to_string())?;
    let pack = codex::to_pack(&responses, version, Some(now_rfc3339())).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(&pack).map_err(|e| e.to_string())?;
    std::fs::write(&args.out, json + "\n").map_err(|e| e.to_string())?;
    eprintln!("wrote {}: {}", args.out.display(), pack.summary());
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("refresh-bundle: {e}");
            ExitCode::FAILURE
        }
    }
}

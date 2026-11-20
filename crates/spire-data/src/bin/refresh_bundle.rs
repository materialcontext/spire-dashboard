//! Regenerates `data/catalog.json`, the data compiled into the app.
//!
//! ```text
//! cargo run -p spire-data --bin refresh-bundle                    # fetch from Spire Codex
//! cargo run -p spire-data --bin refresh-bundle -- --from-dir DIR  # use saved <endpoint>.json files
//! ```
//!
//! Options: `--base-url URL`, `--out PATH` (default `data/catalog.json`),
//! `--run-pages N` pages of community runs (default 12, 0 = none),
//! `--runs-since RFC3339` (default: 14 days ago), `--channel beta|stable`
//! (default beta). With `--from-dir`, run pages
//! are read from `DIR/runs/*` (gzip or plain JSONL).

use std::path::PathBuf;
use std::process::ExitCode;

use spire_data::codex::{self, Channel, Responses};
use spire_data::{DataError, fetch, now_rfc3339, rfc3339, unix_now};

struct Args {
    from_dir: Option<PathBuf>,
    base_url: String,
    out: PathBuf,
    run_pages: u32,
    runs_since: Option<String>,
    channel: Channel,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        from_dir: None,
        base_url: codex::DEFAULT_BASE_URL.into(),
        out: PathBuf::from("data/catalog.json"),
        run_pages: fetch::DEFAULT_RUN_PAGES,
        runs_since: None,
        channel: Channel::Beta,
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut value = || it.next().ok_or(format!("{flag} needs a value"));
        match flag.as_str() {
            "--from-dir" => args.from_dir = Some(PathBuf::from(value()?)),
            "--base-url" => args.base_url = value()?,
            "--out" => args.out = PathBuf::from(value()?),
            "--run-pages" => args.run_pages = value()?.parse().map_err(|e| format!("--run-pages: {e}"))?,
            "--runs-since" => args.runs_since = Some(value()?),
            "--channel" => {
                args.channel = match value()?.as_str() {
                    "beta" => Channel::Beta,
                    "stable" => Channel::Stable,
                    other => return Err(format!("--channel: unknown {other}")),
                }
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(args)
}

fn read_dir(dir: &std::path::Path) -> Result<(Responses, Option<String>), DataError> {
    let mut responses = Responses::new();
    for endpoint in codex::ENDPOINTS {
        // Snapshots taken before an endpoint was added may lack it.
        if let Ok(body) = std::fs::read_to_string(dir.join(format!("{endpoint}.json"))) {
            responses.insert(endpoint, body);
        }
    }
    let version =
        std::fs::read_to_string(dir.join("versions.json")).ok().and_then(|b| codex::latest_stable_version(&b));
    Ok((responses, version))
}

fn read_run_pages(dir: &std::path::Path) -> Result<Vec<String>, DataError> {
    let runs = dir.join("runs");
    if !runs.is_dir() {
        return Ok(Vec::new());
    }
    let mut paths: Vec<_> = std::fs::read_dir(runs)?.filter_map(|e| e.ok().map(|e| e.path())).collect();
    paths.sort();
    paths.iter().map(|p| fetch::decode_maybe_gzip(&std::fs::read(p)?)).collect()
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let (responses, version) = match &args.from_dir {
        Some(dir) => read_dir(dir),
        None => fetch::codex_responses(&args.base_url, args.channel),
    }
    .map_err(|e| e.to_string())?;
    let mut pack = codex::to_pack(&responses, version, Some(now_rfc3339())).map_err(|e| e.to_string())?;

    let pages = match &args.from_dir {
        Some(dir) => read_run_pages(dir),
        None if args.run_pages > 0 => {
            let since =
                args.runs_since.clone().unwrap_or_else(|| rfc3339(unix_now() - fetch::RUN_WINDOW_DAYS * 86_400));
            // The bundle is all or nothing: a page that fails fails it.
            match fetch::codex_run_pages(&args.base_url, &since, args.run_pages) {
                (pages, None) => Ok(pages),
                (_, Some(e)) => Err(e),
            }
        }
        None => Ok(Vec::new()),
    }
    .map_err(|e| e.to_string())?;
    let mut notes = Vec::new();
    if !pages.is_empty() {
        fetch::add_community_data(&mut pack, &pages, &mut notes);
    }

    let json = serde_json::to_string_pretty(&pack).map_err(|e| e.to_string())?;
    std::fs::write(&args.out, json + "\n").map_err(|e| e.to_string())?;
    for n in notes {
        eprintln!("{n}");
    }
    for n in spire_data::curated::report(&pack) {
        eprintln!("data/patterns.json: {n}");
    }
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

//! Network access. Only ever called from an explicit user action (the
//! "Update data" button or the `refresh-bundle` tool); nothing polls.

use std::io::Read;
use std::time::Duration;

use spire_core::DataPack;
use ureq::tls::{RootCerts, TlsConfig};

use crate::codex::{self, Channel, Responses};
use crate::{DataError, codex_runs, now_rfc3339, rfc3339, unix_now};

/// Default pages of community runs (1000 runs each) read per update.
pub const DEFAULT_RUN_PAGES: u32 = 3;
/// How far back community runs are read from, so lineups match the current game.
pub const RUN_WINDOW_DAYS: i64 = 14;

fn default_run_pages() -> u32 {
    DEFAULT_RUN_PAGES
}

/// Where to fetch from. New sources (a mirror, a different API) become new
/// variants with their own adapter module.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Source {
    /// Spire Codex REST API at `base_url`. `run_pages` pages of recent
    /// community runs are read for exact lineups and fight stats (0 = skip).
    SpireCodex {
        base_url: String,
        #[serde(default = "default_run_pages")]
        run_pages: u32,
        /// Game branch; beta unless set.
        #[serde(default)]
        channel: Channel,
    },
    /// A URL serving a complete `DataPack` JSON document.
    PackUrl { url: String },
}

impl Default for Source {
    fn default() -> Self {
        Source::SpireCodex {
            base_url: codex::DEFAULT_BASE_URL.into(),
            run_pages: DEFAULT_RUN_PAGES,
            channel: Channel::Beta,
        }
    }
}

impl Source {
    pub fn describe(&self) -> String {
        match self {
            Source::SpireCodex { base_url, channel, .. } => {
                format!("Spire Codex, {} branch ({base_url})", channel.label())
            }
            Source::PackUrl { url } => format!("Data pack ({url})"),
        }
    }
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(60)))
        .user_agent("spire-dashboard")
        // Use the OS trust store so corporate proxies and custom CAs work.
        .tls_config(TlsConfig::builder().root_certs(RootCerts::PlatformVerifier).build())
        .build()
        .into()
}

fn get(agent: &ureq::Agent, url: &str) -> Result<String, DataError> {
    agent
        .get(url)
        .call()
        .map_err(|e| DataError::Http(format!("{url}: {e}")))?
        .body_mut()
        .with_config()
        .limit(64 * 1024 * 1024)
        .read_to_string()
        .map_err(|e| DataError::Http(format!("{url}: {e}")))
}

/// Downloads every Codex endpoint the adapter needs.
pub fn codex_responses(base_url: &str, channel: Channel) -> Result<(Responses, Option<String>), DataError> {
    let agent = agent();
    let base = base_url.trim_end_matches('/');
    let mut responses = Responses::new();
    for endpoint in codex::ENDPOINTS {
        responses.insert(endpoint, get(&agent, &format!("{base}/api/{endpoint}{}", channel.query()))?);
    }
    // The version is informational; don't fail the update without it.
    let version = match channel {
        Channel::Beta => get(&agent, &format!("{base}/api/beta/version")).ok().and_then(|b| codex::beta_version(&b)),
        Channel::Stable => {
            get(&agent, &format!("{base}/api/versions")).ok().and_then(|b| codex::latest_stable_version(&b))
        }
    }
    .map(|v| format!("{v} ({})", channel.label()));
    Ok((responses, version))
}

/// Downloads up to `pages` pages of community runs submitted since `start`,
/// returning decompressed JSONL text per page.
pub fn codex_run_pages(base_url: &str, start: &str, pages: u32) -> Result<Vec<String>, DataError> {
    let agent = agent();
    let base = base_url.trim_end_matches('/');
    let mut out = Vec::new();
    let mut cursor: Option<String> = None;
    for _ in 0..pages {
        let mut url = format!("{base}/api/exports/runs?limit=1000&start={start}");
        if let Some(c) = &cursor {
            url.push_str(&format!("&cursor={c}"));
        }
        let mut resp = agent.get(&url).call().map_err(|e| DataError::Http(format!("{url}: {e}")))?;
        let next = resp.headers().get("x-next-cursor").and_then(|v| v.to_str().ok()).map(str::to_string);
        let bytes = resp
            .body_mut()
            .with_config()
            .limit(256 * 1024 * 1024)
            .read_to_vec()
            .map_err(|e| DataError::Http(format!("{url}: {e}")))?;
        out.push(decode_maybe_gzip(&bytes)?);
        match next {
            Some(c) if !c.is_empty() => cursor = Some(c),
            _ => break,
        }
    }
    Ok(out)
}

/// The export is served as a gzip file; accept plain text too.
pub fn decode_maybe_gzip(bytes: &[u8]) -> Result<String, DataError> {
    if bytes.starts_with(&[0x1f, 0x8b]) {
        let mut text = String::new();
        flate2::read::GzDecoder::new(bytes).read_to_string(&mut text)?;
        Ok(text)
    } else {
        String::from_utf8(bytes.to_vec()).map_err(|e| DataError::Parse(e.to_string()))
    }
}

/// A fetched pack plus non-fatal notes for the user.
pub struct Fetched {
    pub pack: DataPack,
    pub notes: Vec<String>,
}

/// Adds community lineups and stats to a Codex pack. Never fails the update.
pub fn add_community_data(pack: &mut DataPack, pages: &[String], notes: &mut Vec<String>) {
    let fights = codex_runs::fights_from_pages(pages);
    let s = codex_runs::enrich(pack, &fights);
    let layouts: Vec<_> = pages.iter().flat_map(|p| codex_runs::layouts(p)).collect();
    let acts = codex_runs::enrich_layouts(pack, &layouts);
    notes.push(format!("Community runs: room layouts measured for {acts} act numbers."));
    notes.push(format!(
        "Community runs: {} fights read, {} encounters with exact lineups, weak fights per act {:?}.",
        s.fights, s.encounters, s.weak_fights
    ));
    let decks = crate::benchmarks::add(pack, pages, &crate::bundled_playbook());
    notes.push(format!(
        "Community runs: {decks} decks rebuilt at the acts they cleared, {} benchmarks.",
        pack.benchmarks.len()
    ));
}

/// Fetches and converts a full data pack from a source. Blocking: run it off
/// the UI thread.
pub fn fetch(source: &Source) -> Result<Fetched, DataError> {
    match source {
        Source::SpireCodex { base_url, run_pages, channel } => {
            let (responses, version) = codex_responses(base_url, *channel)?;
            let mut pack = codex::to_pack(&responses, version, Some(now_rfc3339()))?;
            let mut notes = Vec::new();
            if *run_pages > 0 {
                let start = rfc3339(unix_now() - RUN_WINDOW_DAYS * 86_400);
                match codex_run_pages(base_url, &start, *run_pages) {
                    Ok(pages) => add_community_data(&mut pack, &pages, &mut notes),
                    Err(e) => notes.push(format!("Skipped community runs: {e}")),
                }
            }
            Ok(Fetched { pack, notes })
        }
        Source::PackUrl { url } => {
            let mut pack = crate::parse_pack(&get(&agent(), url)?)?;
            pack.fetched_at.get_or_insert_with(now_rfc3339);
            Ok(Fetched { pack, notes: Vec::new() })
        }
    }
}

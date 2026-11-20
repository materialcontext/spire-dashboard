//! Network access. Only ever called from an explicit user action (the
//! "Update data" button or the `refresh-bundle` tool); nothing polls.

use std::io::Read;
use std::time::Duration;

use spire_core::DataPack;
use ureq::tls::{RootCerts, TlsConfig};

use crate::codex::{self, Channel, Responses};
use crate::{DataError, codex_events, codex_runs, now_rfc3339, rfc3339, unix_now};

/// Default pages of community runs (1000 runs each) read per update: card
/// lines need many boss fights to say anything.
pub const DEFAULT_RUN_PAGES: u32 = 12;
/// The default before card lines. Nothing in the app sets the page count,
/// so a saved 3 is that old default, not a choice; it reads as the new one.
const OLD_DEFAULT_RUN_PAGES: u32 = 3;
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

impl Source {
    /// A source saved with the old default page count reads with the new one.
    pub fn upgraded(self) -> Source {
        match self {
            Source::SpireCodex { base_url, run_pages: OLD_DEFAULT_RUN_PAGES, channel } => {
                Source::SpireCodex { base_url, run_pages: DEFAULT_RUN_PAGES, channel }
            }
            other => other,
        }
    }
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
    // Card metrics only rank suggestions: the update goes on without them.
    if let Ok(body) = get(&agent, &format!("{base}/api/{}", codex::CARD_METRICS_PATH)) {
        responses.insert(codex::CARD_METRICS, body);
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
/// returning decompressed JSONL text per page, and the error that stopped
/// it early, if any (the pages read before it are kept).
pub fn codex_run_pages(base_url: &str, start: &str, pages: u32) -> (Vec<String>, Option<DataError>) {
    let agent = agent();
    let url = format!("{}/api/exports/runs", base_url.trim_end_matches('/'));
    let mut out = Vec::new();
    let mut cursor: Option<String> = None;
    for _ in 0..pages {
        match run_page(&agent, &url, start, cursor.as_deref()) {
            Ok((text, next)) => {
                out.push(text);
                match next {
                    Some(c) if !c.is_empty() && cursor.as_ref() != Some(&c) => cursor = Some(c),
                    _ => break,
                }
            }
            Err(e) => return (out, Some(e)),
        }
    }
    (out, None)
}

/// One page of the run export, and the cursor to the next.
fn run_page(
    agent: &ureq::Agent,
    url: &str,
    start: &str,
    cursor: Option<&str>,
) -> Result<(String, Option<String>), DataError> {
    let err = |e: &dyn std::fmt::Display| DataError::Http(format!("{url} (cursor {cursor:?}): {e}"));
    // `query` percent-encodes: the cursor and timestamp can hold `+` or `=`.
    let mut req = agent.get(url).query("limit", "1000").query("start", start);
    if let Some(c) = cursor {
        req = req.query("cursor", c);
    }
    let mut resp = req.call().map_err(|e| err(&e))?;
    let next = resp.headers().get("x-next-cursor").and_then(|v| v.to_str().ok()).map(str::to_string);
    let bytes = resp.body_mut().with_config().limit(256 * 1024 * 1024).read_to_vec().map_err(|e| err(&e))?;
    Ok((decode_maybe_gzip(&bytes)?, next))
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
    let events = codex_events::enrich(pack, &codex_events::sightings(pages, &codex_events::curses(pack)));
    notes.push(format!("Community runs: event relics found for {events} events, and how often each event comes up."));
    let playbook = crate::bundled_playbook();
    let (decks, bosses) = crate::benchmarks::add(pack, pages, &playbook);
    notes.push(format!(
        "Community runs: {decks} A{} decks rebuilt going into each boss, {} benchmarks, {} composition risks.",
        playbook.composition.ascension,
        pack.benchmarks.len(),
        pack.risks.len()
    ));
    for r in &pack.risks {
        notes.push(format!(
            "  Act {}: {} — boss won {:.0}% vs {:.0}% ({:.0} decks)",
            r.act + 1,
            spire_core::analysis::composition::phrase(r.measure, r.side),
            r.rate * 100.0,
            r.typical * 100.0,
            r.decks
        ));
    }
    let lines = crate::card_lines::add(pack, &bosses, pages);
    notes.push(format!("Community runs: {} boss fights, {lines} card lines.", bosses.len()));
}

/// Where the bundle's measured layers stay because the download measured
/// fewer decks (`Catalog::from_layers` keeps the larger sample).
fn kept_bundled(pack: &DataPack) -> Vec<String> {
    use spire_core::catalog::{benchmark_decks, line_fights};
    let bundled = crate::bundled_pack();
    let mut notes = Vec::new();
    let (new, old) = (benchmark_decks(&pack.benchmarks), benchmark_decks(&bundled.benchmarks));
    if new < old {
        notes.push(format!("Composition ranges and risks: the bundled ones stay ({old} decks against {new})."));
    }
    let (new, old) = (line_fights(&pack.card_lines), line_fights(&bundled.card_lines));
    if new < old {
        notes.push(format!("Card lines: the bundled ones stay ({old} boss fights against {new})."));
    }
    notes
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
                let (pages, stopped) = codex_run_pages(base_url, &start, *run_pages);
                if let Some(e) = stopped {
                    notes.push(format!("Community runs: stopped after {} of {run_pages} pages: {e}", pages.len()));
                }
                if !pages.is_empty() {
                    add_community_data(&mut pack, &pages, &mut notes);
                    notes.extend(kept_bundled(&pack));
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

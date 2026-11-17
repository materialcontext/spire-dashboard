//! Network access. Only ever called from an explicit user action (the
//! "Update data" button or the `refresh-bundle` tool); nothing polls.

use std::time::Duration;

use spire_core::DataPack;
use ureq::tls::{RootCerts, TlsConfig};

use crate::codex::{self, Responses};
use crate::{DataError, now_rfc3339};

/// Where to fetch from. New sources (a mirror, a different API) become new
/// variants with their own adapter module.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Source {
    /// Spire Codex REST API at `base_url`.
    SpireCodex { base_url: String },
    /// A URL serving a complete `DataPack` JSON document.
    PackUrl { url: String },
}

impl Default for Source {
    fn default() -> Self {
        Source::SpireCodex { base_url: codex::DEFAULT_BASE_URL.into() }
    }
}

impl Source {
    pub fn describe(&self) -> String {
        match self {
            Source::SpireCodex { base_url } => format!("Spire Codex ({base_url})"),
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
pub fn codex_responses(base_url: &str) -> Result<(Responses, Option<String>), DataError> {
    let agent = agent();
    let base = base_url.trim_end_matches('/');
    let mut responses = Responses::new();
    for endpoint in codex::ENDPOINTS {
        responses.insert(endpoint, get(&agent, &format!("{base}/api/{endpoint}"))?);
    }
    // The version is informational; don't fail the update without it.
    let version = get(&agent, &format!("{base}/api/versions")).ok().and_then(|b| codex::latest_stable_version(&b));
    Ok((responses, version))
}

/// Fetches and converts a full data pack from a source. Blocking: run it off
/// the UI thread.
pub fn fetch(source: &Source) -> Result<DataPack, DataError> {
    match source {
        Source::SpireCodex { base_url } => {
            let (responses, version) = codex_responses(base_url)?;
            codex::to_pack(&responses, version, Some(now_rfc3339()))
        }
        Source::PackUrl { url } => {
            let mut pack = crate::parse_pack(&get(&agent(), url)?)?;
            pack.fetched_at.get_or_insert_with(now_rfc3339);
            Ok(pack)
        }
    }
}

//! Data plumbing for the dashboard: bundled data, local storage, and
//! data-source adapters. Everything with IO lives here; `spire-core` stays pure.
//!
//! Catalog layers, lowest priority first:
//!
//! 1. **bundled**: `data/catalog.json`, compiled into the binary.
//! 2. **downloaded**: the last successful update, saved in the data dir.
//! 3. **curated**: `data/patterns.json`, sourced fixes for gaps in the
//!    monster data, applied only while the gap remains (see [`curated`]).
//! 4. **user**: optional `user.json` in the data dir for hand-made fixes.
//!
//! Updates only happen when the user presses the button: see [`fetch`].

pub mod benchmarks;
pub mod codex;
pub mod codex_events;
pub mod codex_runs;
pub mod curated;
pub mod fetch;
pub mod game_link;
pub mod game_run;
mod run_file;
pub mod store;

use spire_core::{DataPack, Playbook};

#[derive(Debug)]
pub enum DataError {
    Io(std::io::Error),
    Parse(String),
    Http(String),
    Missing(&'static str),
    Format { found: u32, supported: u32 },
}

impl std::fmt::Display for DataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataError::Io(e) => write!(f, "file error: {e}"),
            DataError::Parse(e) => write!(f, "could not read data: {e}"),
            DataError::Http(e) => write!(f, "download failed: {e}"),
            DataError::Missing(what) => write!(f, "missing {what}"),
            DataError::Format { found, supported } => {
                write!(f, "data pack format {found} is newer than supported ({supported}); update the app")
            }
        }
    }
}

impl std::error::Error for DataError {}

impl From<std::io::Error> for DataError {
    fn from(e: std::io::Error) -> Self {
        DataError::Io(e)
    }
}

const BUNDLED_CATALOG: &str = include_str!("../../../data/catalog.json");
const BUNDLED_PLAYBOOK: &str = include_str!("../../../data/playbook.json");

/// Parses a data pack and checks its format version.
pub fn parse_pack(json: &str) -> Result<DataPack, DataError> {
    let pack: DataPack = serde_json::from_str(json).map_err(|e| DataError::Parse(e.to_string()))?;
    if pack.format > spire_core::catalog::DATA_PACK_FORMAT {
        return Err(DataError::Format { found: pack.format, supported: spire_core::catalog::DATA_PACK_FORMAT });
    }
    Ok(pack)
}

pub fn parse_playbook(json: &str) -> Result<Playbook, DataError> {
    serde_json::from_str(json).map_err(|e| DataError::Parse(format!("playbook: {e}")))
}

/// The data pack compiled into the app.
pub fn bundled_pack() -> DataPack {
    parse_pack(BUNDLED_CATALOG).expect("bundled catalog.json is valid (checked by tests)")
}

/// The bundled data with the curated fixes over it: the catalog the app
/// starts from before any download (for tools and tests).
pub fn bundled_catalog() -> spire_core::Catalog {
    let mut layers = vec![bundled_pack()];
    layers.extend(curated::pack(&layers));
    spire_core::Catalog::from_layers(layers)
}

/// The default playbook compiled into the app.
pub fn bundled_playbook() -> Playbook {
    parse_playbook(BUNDLED_PLAYBOOK).expect("bundled playbook.json is valid (checked by tests)")
}

/// Current time as an RFC 3339 UTC timestamp (second precision).
pub fn now_rfc3339() -> String {
    rfc3339(unix_now())
}

pub fn unix_now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs()) as i64
}

/// Formats Unix seconds as an RFC 3339 UTC timestamp.
pub fn rfc3339(secs: i64) -> String {
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Civil-from-days (H. Hinnant), valid for the proleptic Gregorian calendar.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_data_parses() {
        let pack = bundled_pack();
        assert!(!pack.cards.is_empty());
        assert!(!pack.characters.is_empty());
        let playbook = bundled_playbook();
        assert!(!playbook.role_rules.is_empty());
    }

    #[test]
    fn bundled_starting_decks_resolve() {
        let cat = spire_core::Catalog::from_layers([bundled_pack()]);
        for c in cat.characters.values() {
            for card in &c.starting_deck {
                assert!(cat.cards.contains_key(card), "{} starts with unknown card {card}", c.id);
            }
            for relic in &c.starting_relics {
                assert!(cat.relics.contains_key(relic), "{} starts with unknown relic {relic}", c.id);
            }
        }
    }

    #[test]
    fn thorns_is_chip_and_healing_relics_are_mitigation() {
        use spire_core::playbook::Role;
        let cat = bundled_catalog();
        let pb = bundled_playbook();
        let card = |name: &str| cat.cards.values().find(|c| c.name == name).unwrap();
        assert!(pb.roles_of(card("Abrasive")).contains(&Role::Chip));
        assert!(pb.roles_of(card("Cho Barbs")).contains(&Role::Chip));
        let relic = |name: &str| {
            let r = cat.relics.values().find(|r| r.name == name).unwrap();
            pb.relic_roles(&r.text, &spire_core::mechanics::read(&r.text, &[], false))
        };
        assert!(relic("Burning Blood").contains(&Role::Mitigation));
        assert!(relic("Mango").contains(&Role::Mitigation), "raising max HP");
        assert!(!relic("Leafy Poultice").contains(&Role::Mitigation), "losing max HP isn't");
    }

    #[test]
    fn role_criteria_say_how_a_card_gets_each_role() {
        use spire_core::playbook::Role;
        let pb = bundled_playbook();
        let m = pb.role_criteria(Role::Mitigation);
        assert!(m.contains("Block") && m.contains("costs more than 1 energy"), "{m}");
        assert!(pb.role_criteria(Role::Survival).contains("Block 9+"));
        assert!(pb.role_criteria(Role::FastDamage).contains("is a starter Strike"));
    }

    #[test]
    fn playbook_card_overrides_name_real_cards() {
        let cat = spire_core::Catalog::from_layers([bundled_pack()]);
        let pb = bundled_playbook();
        for id in pb.card_roles.keys().chain(pb.card_tags.keys()) {
            assert!(cat.cards.contains_key(id), "playbook card_roles or card_tags names unknown card {id}");
        }
    }

    #[test]
    fn relic_fits_name_relics_a_question_room_can_give() {
        let cat = spire_core::Catalog::from_layers([bundled_pack()]);
        for fit in &bundled_playbook().relic_fits {
            let relic =
                cat.relics.get(&fit.relic).unwrap_or_else(|| panic!("relic_fits names unknown relic {}", fit.relic));
            let chest = cat.acts.values().any(|a| a.chest_rarity.contains_key(&relic.rarity));
            let event = cat.events.values().any(|e| e.relics.iter().any(|r| r.relic == fit.relic));
            assert!(chest || event, "neither a chest nor an event gives {} ({:?})", fit.relic, relic.rarity);
            assert!(fit.cards.is_some() || fit.short_of.is_some(), "{} has neither cards nor short_of", fit.relic);
        }
    }

    #[test]
    fn rejects_newer_format() {
        let json = r#"{"format": 999, "source": "future"}"#;
        assert!(matches!(parse_pack(json), Err(DataError::Format { .. })));
    }

    #[test]
    fn formats_known_dates() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339(1_795_000_000), "2026-11-18T11:06:40Z");
    }

    #[test]
    fn timestamp_shape() {
        let t = now_rfc3339();
        assert_eq!(t.len(), 20);
        assert!(t.ends_with('Z'));
    }
}

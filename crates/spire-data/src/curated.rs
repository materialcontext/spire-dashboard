//! Curated fixes for gaps in the source data, each with its sources.
//!
//! Spire Codex leaves some monster patterns incomplete: a move with no next
//! move (Entomancer), a sleep with no wake (Lagavulin Matriarch), no
//! pattern at all (the Decimillipede segments), or no HP for a boss that
//! revives into phases (Test Subject). `data/patterns.json` fills those
//! gaps from research. Every entry names its sources, and says whether
//! each was read directly or only reported (by search summaries of pages
//! that couldn't be read from here). Nothing is taken from memory.
//!
//! The fixes are their own catalog layer, over the Codex data (bundled or
//! downloaded) and under the user's own overrides. An entry applies only
//! while its gap is still there, so when Codex fills one, Codex wins;
//! `refresh-bundle` reports entries that are no longer needed.

use serde::Deserialize;
use spire_core::DataPack;
use spire_core::analysis::pattern;
use spire_core::catalog::{AttackPattern, Monster, Phase};
use spire_core::ids::MonsterId;

const PATTERNS: &str = include_str!("../../../data/patterns.json");

#[derive(Deserialize)]
struct File {
    entries: Vec<Entry>,
}

/// One monster's fix.
#[derive(Clone, Debug, Deserialize)]
pub struct Entry {
    pub monster: MonsterId,
    /// A pattern, for a monster whose pattern is missing or loses its order.
    #[serde(default)]
    pub pattern: Option<AttackPattern>,
    /// HP and phases, for a monster with no HP that revives into phases.
    #[serde(default)]
    pub hp: Option<Phase>,
    #[serde(default)]
    pub phases: Vec<Phase>,
    pub sources: Vec<Source>,
    #[serde(default)]
    pub note: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Source {
    pub url: String,
    /// What the source says, in its words or close to them.
    pub says: String,
    /// The page was read directly (not only reported in a search summary).
    pub read: bool,
}

/// The curated entries.
pub fn entries() -> Vec<Entry> {
    serde_json::from_str::<File>(PATTERNS).map(|f| f.entries).expect("data/patterns.json is valid (checked by tests)")
}

/// Whether the source's monster still has the gap the entry fills.
fn needed(entry: &Entry, m: &Monster) -> bool {
    let pattern_gap = entry.pattern.is_some() && {
        let plan = pattern::plan(m, false, 50, pattern::HORIZON);
        plan.unknown || plan.order_unknown_from.is_some()
    };
    let hp_gap = (entry.hp.is_some() || !entry.phases.is_empty()) && m.hp_min == 0 && m.phases.is_empty();
    pattern_gap || hp_gap
}

fn apply(entry: &Entry, m: &Monster) -> Monster {
    let mut out = m.clone();
    if let Some(p) = &entry.pattern {
        out.pattern = Some(p.clone());
    }
    if let Some(hp) = &entry.hp {
        out.hp_min = hp.hp_min;
        out.hp_max = hp.hp_max;
        out.hp_min_asc = hp.hp_min_asc;
        out.hp_max_asc = hp.hp_max_asc;
    }
    if !entry.phases.is_empty() {
        out.phases = entry.phases.clone();
    }
    out
}

/// The monster as the given layers have it (the last layer that has it).
fn resolve<'a>(layers: &'a [DataPack], id: &MonsterId) -> Option<&'a Monster> {
    layers.iter().rev().find_map(|p| p.monsters.iter().find(|m| m.id == *id))
}

/// The curated layer over `layers`: the fixes that still apply, as patched
/// monsters. `None` when none applies.
pub fn pack(layers: &[DataPack]) -> Option<DataPack> {
    let monsters: Vec<Monster> = entries()
        .iter()
        .filter_map(|e| resolve(layers, &e.monster).filter(|m| needed(e, m)).map(|m| apply(e, m)))
        .collect();
    (!monsters.is_empty()).then(|| DataPack {
        format: spire_core::catalog::DATA_PACK_FORMAT,
        source: "curated".into(),
        monsters,
        ..DataPack::default()
    })
}

/// Entries the given pack no longer needs (its data fills the gap), or
/// whose monster it doesn't have: for `refresh-bundle` to report.
pub fn report(pack: &DataPack) -> Vec<String> {
    entries()
        .iter()
        .filter_map(|e| match pack.monsters.iter().find(|m| m.id == e.monster) {
            None => Some(format!("{}: not in the data any more", e.monster)),
            Some(m) if !needed(e, m) => Some(format!("{}: the data fills this gap now", e.monster)),
            Some(_) => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use spire_core::Catalog;

    #[test]
    fn every_entry_cites_a_source_and_fills_a_gap() {
        let bundled = crate::bundled_pack();
        for e in entries() {
            assert!(!e.sources.is_empty(), "{}: no sources", e.monster);
            assert!(e.sources.iter().all(|s| s.url.starts_with("https://") && !s.says.is_empty()), "{}", e.monster);
            let m = bundled.monsters.iter().find(|m| m.id == e.monster).expect("monster in the bundle");
            assert!(needed(&e, m), "{}: the bundled data has no gap here any more", e.monster);
            let fixed = apply(&e, m);
            if e.pattern.is_some() {
                let plan = pattern::plan(&fixed, false, 50, pattern::HORIZON);
                assert!(!plan.unknown && plan.order_unknown_from.is_none(), "{}: still a gap", e.monster);
            }
        }
    }

    #[test]
    fn the_curated_layer_patches_the_gaps() {
        let layers = vec![crate::bundled_pack()];
        let curated = pack(&layers).expect("gaps to fill");
        let cat = Catalog::from_layers(layers.into_iter().chain([curated]));
        let ento = &cat.monsters[&MonsterId::new("ENTOMANCER")];
        let plan = pattern::plan(ento, false, 50, 6);
        let names: Vec<&str> = plan.turns.iter().map(|t| t.options[0].name.as_str()).collect();
        assert_eq!(&names[..4], &["Beeeees!", "Spear!", "Pheromone Spit", "Beeeees!"]);
        let test_subject = &cat.monsters[&MonsterId::new("TEST_SUBJECT")];
        assert_eq!((test_subject.hp_min, test_subject.phases.len()), (100, 2));
    }
}

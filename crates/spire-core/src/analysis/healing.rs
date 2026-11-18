//! Healing by the game's rules: Ancients between acts and resting.
//!
//! The percentages come from [`crate::catalog::GameRules`]; amounts round
//! down, as the community run data shows.

use crate::catalog::Catalog;
use crate::playbook::contains_ci;
use crate::run::RunState;

/// HP a rest would heal now: the rules' percent of max HP, plus relics that
/// "heal an additional N HP" when you Rest, never more than is missing.
pub fn rest_heal(state: &RunState, catalog: &Catalog) -> i32 {
    let base = state.max_hp * catalog.rules.rest_heal_pct as i32 / 100;
    let bonus: i32 = state
        .relics
        .iter()
        .filter_map(|r| catalog.relics.get(r))
        .filter(|r| contains_ci(&r.text, "whenever you rest"))
        .filter_map(|r| additional_heal(&r.text))
        .sum();
    (base + bonus).min((state.max_hp - state.hp).max(0))
}

/// "…heal an additional 15 HP" → 15.
fn additional_heal(text: &str) -> Option<i32> {
    let lower = text.to_lowercase();
    let rest = &lower[lower.find("heal an additional")? + "heal an additional".len()..];
    rest.split_whitespace().next()?.parse().ok()
}

//! Left panel: the deck, relics and potions as they stand.

use std::collections::BTreeMap;

use eframe::egui::{self, RichText, Ui};
use spire_core::analysis::deck::classify;
use spire_core::catalog::CardType;
use spire_core::run::{Change, DeckCard};

use crate::actions::Action;
use crate::app::ViewCtx;

pub fn show(ui: &mut Ui, v: &ViewCtx, out: &mut Vec<Action>) {
    let s = v.state;
    ui.heading(format!("Deck ({})", s.deck.len()));
    ui.label(RichText::new("Hover a card for text. Buttons queue changes on the node log.").small().weak());

    // Group identical cards: (type, name) → (deck card, count, roles).
    let classified = classify(&s.deck, v.catalog, v.playbook);
    let mut groups: BTreeMap<(CardType, String), (DeckCard, u32, String, String)> = BTreeMap::new();
    for c in &classified {
        let name = c.name();
        let entry = groups.entry((c.card_type(), name)).or_insert_with(|| {
            let roles: Vec<&str> = c.roles.iter().map(|r| r.short()).collect();
            let text = c.card.map_or_else(
                || "Not in the catalog".to_string(),
                |card| format!("{} · cost {}\n{}", card.card_type.label(), card.cost_label(), card.text),
            );
            (c.deck.clone(), 0, roles.join(", "), text)
        });
        entry.1 += 1;
    }

    let mut current = None;
    for ((t, name), (deck_card, count, roles, text)) in &groups {
        if current != Some(*t) {
            current = Some(*t);
            ui.add_space(4.0);
            ui.label(RichText::new(t.label()).strong());
        }
        ui.horizontal(|ui| {
            let label = if *count > 1 { format!("{count}× {name}") } else { name.clone() };
            let label = match &deck_card.enchantment {
                Some(e) => format!("{label} [{}]", v.catalog.enchantment_name(e)),
                None => label,
            };
            let hover = if roles.is_empty() { text.clone() } else { format!("{text}\n\nRoles: {roles}") };
            ui.label(label).on_hover_text(hover);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("x").on_hover_text("Remove").clicked() {
                    out.push(Action::Draft(Change::RemoveCard(deck_card.clone())));
                }
                if !deck_card.upgraded && ui.small_button("+").on_hover_text("Upgrade").clicked() {
                    out.push(Action::Draft(Change::UpgradeCard(deck_card.clone())));
                }
                if !roles.is_empty() {
                    ui.label(RichText::new(roles).small().weak());
                }
            });
        });
    }

    ui.add_space(8.0);
    ui.separator();
    ui.heading(format!("Relics ({})", s.relics.len()));
    for r in &s.relics {
        ui.horizontal(|ui| {
            let text = v.catalog.relics.get(r).map_or("", |x| x.text.as_str());
            ui.label(v.catalog.relic_name(r)).on_hover_text(text);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("x").on_hover_text("Lose relic").clicked() {
                    out.push(Action::Draft(Change::RemoveRelic(r.clone())));
                }
            });
        });
    }

    ui.add_space(8.0);
    ui.separator();
    ui.heading(format!("Potions ({}/{})", s.potions.len(), s.potion_slots));
    for p in &s.potions {
        ui.horizontal(|ui| {
            let text = v.catalog.potions.get(p).map_or("", |x| x.text.as_str());
            ui.label(v.catalog.potion_name(p)).on_hover_text(text);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("used").clicked() {
                    out.push(Action::Draft(Change::RemovePotion(p.clone())));
                }
            });
        });
    }
}

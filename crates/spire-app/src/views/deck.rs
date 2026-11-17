//! Left panel: the deck, relics and potions as they stand.

use eframe::egui::{self, RichText, Ui};
use spire_core::analysis::DeckRow;
use spire_core::playbook::Role;
use spire_core::run::Change;

use crate::actions::Action;
use crate::app::ViewCtx;
use crate::widgets::culled;

pub fn show(ui: &mut Ui, v: &ViewCtx, out: &mut Vec<Action>) {
    let s = v.state;
    ui.heading(format!("Deck ({})", s.deck.len()));
    ui.label(RichText::new("Hover a card for text. Buttons queue changes on the node log.").small().weak());

    // Rows are grouped once per run change (Dashboard::rows), not per frame.
    let mut current = None;
    for row in &v.dash.rows {
        if current != Some(row.card_type) {
            current = Some(row.card_type);
            ui.add_space(4.0);
            ui.label(RichText::new(row.card_type.label()).strong());
        }
        culled(ui, ("deck-row", v.generation, &row.name, &row.card.enchantment), |ui| {
            ui.horizontal(|ui| {
                let mut label = if row.count > 1 { format!("{}× {}", row.count, row.name) } else { row.name.clone() };
                if let Some(e) = &row.card.enchantment {
                    label = format!("{label} [{}]", v.catalog.enchantment_name(e));
                }
                // Tooltip text is only built while hovered.
                ui.label(label).on_hover_ui(|ui| card_tooltip(ui, v, row));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Frameless until hovered: two framed buttons per row were
                    // ~70 rounded fills per frame for little visual value.
                    if ui.add(row_button("x")).on_hover_text("Remove").clicked() {
                        out.push(Action::Draft(Change::RemoveCard(row.card.clone())));
                    }
                    if !row.card.upgraded && ui.add(row_button("+")).on_hover_text("Upgrade").clicked() {
                        out.push(Action::Draft(Change::UpgradeCard(row.card.clone())));
                    }
                    if !row.roles.is_empty() {
                        ui.label(RichText::new(roles_label(&row.roles)).small().weak());
                    }
                });
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
                if ui.add(row_button("x")).on_hover_text("Lose relic").clicked() {
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
                if ui.add(row_button("used")).clicked() {
                    out.push(Action::Draft(Change::RemovePotion(p.clone())));
                }
            });
        });
    }
}

fn row_button(text: &str) -> egui::Button<'_> {
    egui::Button::new(text).small().frame_when_inactive(false)
}

fn roles_label(roles: &[Role]) -> String {
    roles.iter().map(|r| r.short()).collect::<Vec<_>>().join(", ")
}

fn card_tooltip(ui: &mut Ui, v: &ViewCtx, row: &DeckRow) {
    match v.catalog.cards.get(&row.card.card) {
        Some(card) => {
            ui.label(format!("{} · cost {}", card.card_type.label(), card.cost_label()));
            let text = if row.card.upgraded { card.upgrade_text.as_ref().unwrap_or(&card.text) } else { &card.text };
            ui.label(text);
        }
        None => {
            ui.label("Not in the catalog");
        }
    }
    if !row.roles.is_empty() {
        ui.label(format!("Roles: {}", roles_label(&row.roles)));
    }
}

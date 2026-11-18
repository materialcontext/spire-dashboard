//! Left sidebar: the deck, relics and potions as they stand.

use eframe::egui::{self, RichText, Sense, Ui, vec2};
use spire_core::analysis::DeckRow;
use spire_core::catalog::CardType;
use spire_core::playbook::Role;
use spire_core::run::Change;

use crate::actions::Action;
use crate::app::ViewCtx;
use crate::style::{self, color::*};
use crate::widgets::{culled, eyebrow, hairline, stripe};

pub fn show(ui: &mut Ui, v: &ViewCtx, out: &mut Vec<Action>) {
    let s = v.state;
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label(eyebrow("Deck"));
        ui.label(RichText::new(s.deck.len().to_string()).small().color(text_faint()));
    });
    hairline(ui);
    ui.label(RichText::new("Right-click a card to upgrade, enchant or remove it.").small().color(text_faint()));

    // Rows are grouped once per run change (Dashboard::rows), not per frame.
    let mut current = None;
    for row in &v.dash.rows {
        if current != Some(row.card_type) {
            current = Some(row.card_type);
            let count: u32 = v.dash.rows.iter().filter(|r| r.card_type == row.card_type).map(|r| r.count).sum();
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!("{}  {count}", type_plural(row.card_type)))
                    .small()
                    .color(style::card_type_color(row.card_type)),
            );
        }
        culled(ui, ("deck-row", row), |ui| card_row(ui, v, row, out));
    }

    section(ui, "Relics", s.relics.len());
    for r in &s.relics {
        ui.horizontal(|ui| {
            let text = v.catalog.relics.get(r).map_or("", |x| x.text.as_str());
            let name = RichText::new(v.catalog.relic_name(r));
            if crate::widgets::link(ui, name).on_hover_text(text).clicked() {
                out.push(Action::Open(crate::actions::Link::Relic(r.clone())));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(row_button("x")).on_hover_text("Lose relic").clicked() {
                    out.push(Action::Draft(Change::RemoveRelic(r.clone())));
                }
            });
        });
    }

    section(ui, "Potions", s.potions.len());
    if s.potions.is_empty() {
        ui.label(RichText::new(format!("{} empty slots", s.potion_slots)).small().color(text_faint()));
    }
    for p in &s.potions {
        ui.horizontal(|ui| {
            let text = v.catalog.potions.get(p).map_or("", |x| x.text.as_str());
            ui.label(RichText::new(v.catalog.potion_name(p)).color(info())).on_hover_text(text);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(row_button("Use")).on_hover_text("Mark as used").clicked() {
                    out.push(Action::Draft(Change::RemovePotion(p.clone())));
                }
            });
        });
    }
    ui.add_space(12.0);
}

fn section(ui: &mut Ui, title: &str, count: usize) {
    ui.add_space(14.0);
    ui.horizontal(|ui| {
        ui.label(eyebrow(title));
        ui.label(RichText::new(count.to_string()).small().color(text_faint()));
    });
    hairline(ui);
}

fn type_plural(t: CardType) -> &'static str {
    match t {
        CardType::Attack => "Attacks",
        CardType::Skill => "Skills",
        CardType::Power => "Powers",
        CardType::Status => "Statuses",
        CardType::Curse => "Curses",
        CardType::Quest => "Quests",
        CardType::Other => "Other",
    }
}

fn card_row(ui: &mut Ui, v: &ViewCtx, row: &DeckRow, out: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        stripe(ui, style::card_type_color(row.card_type), 18.0);
        let cost = v.catalog.cards.get(&row.card.card).map_or("?".to_string(), |c| c.cost_label_at(row.card.upgraded));
        ui.add_sized([14.0, 18.0], egui::Label::new(RichText::new(cost).small().color(accent())));
        let name = if row.count > 1 { format!("{}× {}", row.count, row.name) } else { row.name.clone() };
        let color = if row.card.upgraded { good() } else { text() };
        // Tooltip text is only built while hovered.
        let r = crate::widgets::link(ui, RichText::new(name).color(color)).on_hover_ui(|ui| card_tooltip(ui, v, row));
        if r.clicked() {
            out.push(Action::Open(crate::actions::Link::Card(row.card.card.clone())));
        }
        // Right-click: everything you can do to this card.
        r.context_menu(|ui| card_menu(ui, v, row, out));
        if let Some(e) = &row.card.enchantment {
            ui.label(RichText::new(v.catalog.enchantment_name(e)).small().color(info()));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            // Frameless until hovered: framed buttons on every row were ~70
            // filled rects per frame for little visual value.
            if row.eternal {
                // Eternal: can't be removed or transformed.
                ui.label(RichText::new("Eternal").small().color(text_faint()))
                    .on_hover_text("Can't be removed or transformed");
            } else if ui.add(row_button("x")).on_hover_text("Remove from deck").clicked() {
                out.push(Action::Draft(Change::RemoveCard(row.card.clone())));
            }
            if !row.card.upgraded && ui.add(row_button("+")).on_hover_text("Upgrade").clicked() {
                out.push(Action::Draft(Change::UpgradeCard(row.card.clone())));
            }
            ui.add_space(4.0);
            role_marks(ui, &row.roles);
        });
    });
}

/// One small square per role the card fills, in that role's colour.
fn role_marks(ui: &mut Ui, roles: &[Role]) {
    for r in roles.iter().rev() {
        let (rect, resp) = ui.allocate_exact_size(vec2(7.0, 7.0), Sense::hover());
        ui.painter().rect_filled(rect, 0.0, style::role_color(*r));
        resp.on_hover_text(r.label());
    }
}

fn card_menu(ui: &mut Ui, v: &ViewCtx, row: &DeckRow, out: &mut Vec<Action>) {
    if !row.card.upgraded && ui.button("Upgrade").clicked() {
        out.push(Action::Draft(Change::UpgradeCard(row.card.clone())));
        ui.close();
    }
    let card = v.catalog.cards.get(&row.card.card);
    match (&row.card.enchantment, card) {
        (Some(e), _) => {
            // Enchantments are permanent: one per card, never removed.
            ui.label(RichText::new(format!("Enchanted: {}", v.catalog.enchantment_name(e))).small().color(text_dim()));
        }
        (None, Some(card)) => {
            let fits: Vec<_> = v.catalog.enchantments.values().filter(|e| e.fits(card)).collect();
            if fits.is_empty() {
                ui.label(RichText::new("No enchantment fits").small().color(text_faint()));
            } else {
                ui.menu_button("Enchant", |ui| {
                    for e in fits {
                        if ui.button(&e.name).on_hover_text(&e.text).clicked() {
                            out.push(Action::Draft(Change::Enchant {
                                card: row.card.clone(),
                                enchantment: e.id.clone(),
                            }));
                            ui.close();
                        }
                    }
                });
            }
        }
        (None, None) => {}
    }
    if row.eternal {
        ui.label(RichText::new("Eternal: can't be removed").small().color(text_faint()));
    } else if ui.button("Remove from deck").clicked() {
        out.push(Action::Draft(Change::RemoveCard(row.card.clone())));
        ui.close();
    }
    ui.separator();
    if ui.button("Open in Cards").clicked() {
        out.push(Action::Open(crate::actions::Link::Card(row.card.card.clone())));
        ui.close();
    }
}

fn row_button(text: &str) -> egui::Button<'_> {
    egui::Button::new(RichText::new(text).small().color(text_faint())).frame_when_inactive(false)
}

fn card_tooltip(ui: &mut Ui, v: &ViewCtx, row: &DeckRow) {
    match v.catalog.cards.get(&row.card.card) {
        Some(card) => {
            ui.label(RichText::new(&row.name).strong());
            ui.label(
                RichText::new(format!(
                    "{} · {:?} · cost {}",
                    card.card_type.label(),
                    card.rarity,
                    card.cost_label_at(row.card.upgraded)
                ))
                .small()
                .color(text_dim()),
            );
            ui.label(card.text_at(row.card.upgraded));
            let keywords = card.keywords_at(row.card.upgraded);
            if !keywords.is_empty() {
                ui.label(RichText::new(keywords.join(" · ")).small().color(info()));
            }
        }
        None => {
            ui.label("Not in the game data");
        }
    }
    if !row.roles.is_empty() {
        ui.add_space(4.0);
        for r in &row.roles {
            ui.label(RichText::new(r.label()).small().color(style::role_color(*r)));
        }
    }
}

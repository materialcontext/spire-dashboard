//! Right sidebar: report what happened on the floor you just finished.

use eframe::egui::{self, RichText, Ui};
use spire_core::catalog::RoomType;
use spire_core::run::{Change, DeckCard, NodeKind};

use crate::actions::Action;
use crate::app::ViewCtx;
use crate::draft::NodeDraft;
use crate::style::color::*;
use crate::widgets::{combo_pick, eyebrow, hairline, primary_button, search_pick, segmented};

fn describe(v: &ViewCtx, c: &Change) -> (String, egui::Color32) {
    let card = |d: &DeckCard| {
        let name = v.catalog.card_name(&d.card);
        if d.upgraded { format!("{name}+") } else { name }
    };
    match c {
        Change::AddCard(d) => (format!("Add {}", card(d)), good()),
        Change::RemoveCard(d) => (format!("Remove {}", card(d)), bad()),
        Change::UpgradeCard(d) => (format!("Upgrade {}", card(d)), good()),
        Change::Enchant { card: d, enchantment } => {
            (format!("Enchant {} with {}", card(d), v.catalog.enchantment_name(enchantment)), info())
        }
        Change::Transform { from, to } => (format!("Transform {} into {}", card(from), card(to)), info()),
        Change::AddRelic(r) => (format!("Gain {}", v.catalog.relic_name(r)), text()),
        Change::RemoveRelic(r) => (format!("Lose {}", v.catalog.relic_name(r)), bad()),
        Change::AddPotion(p) => (format!("Gain {}", v.catalog.potion_name(p)), info()),
        Change::RemovePotion(p) => (format!("Use {}", v.catalog.potion_name(p)), text_dim()),
        Change::SetHp(n) => (format!("Set HP to {n}"), text_dim()),
        Change::SetMaxHp(n) => (format!("Set max HP to {n}"), text_dim()),
        Change::SetGold(n) => (format!("Set gold to {n}"), text_dim()),
        Change::SetFloor(n) => (format!("Set floor to {n}"), text_dim()),
        Change::SetPotionSlots(n) => (format!("Set potion slots to {n}"), text_dim()),
    }
}

fn step(ui: &mut Ui, n: u32, title: &str) {
    ui.add_space(crate::style::space::s());
    ui.horizontal(|ui| {
        ui.label(RichText::new(n.to_string()).small().strong().color(text_dim()));
        ui.label(eyebrow(title));
    });
}

pub fn show(ui: &mut Ui, v: &ViewCtx, d: &mut NodeDraft, out: &mut Vec<Action>) {
    let s = v.state;
    ui.add_space(crate::style::space::s());
    ui.label(crate::widgets::title(format!("Log floor {}", s.floor + 1)));
    ui.label(RichText::new("After finishing a floor, record what happened.").small().color(text_faint()));
    hairline(ui);

    step(ui, 1, "Room");
    let rooms: Vec<(NodeKind, &str)> = NodeKind::ALL.iter().map(|k| (*k, k.label())).collect();
    segmented(ui, &mut d.kind, &rooms);
    ui.label(RichText::new("Keys: 1–8 room · Ctrl+Enter commit · Ctrl+Z undo").small().color(text_faint()));
    let length = spire_core::analysis::floors::act_length(s, v.catalog);
    let position = match length {
        Some(l) => format!("Act floor {} of {l}", s.act_floor + 1),
        None => format!("Act floor {}", s.act_floor + 1),
    };
    let hint = match d.expected {
        Some(k) => format!("{position} · always {}", k.label()),
        None => position,
    };
    ui.label(RichText::new(hint).small().color(if d.expected.is_some() { text_dim() } else { text_faint() }));

    if let Some(act) = &s.act {
        let room = match d.kind {
            NodeKind::Unknown => Some(RoomType::Monster),
            k => k.room(),
        };
        let event_room = matches!(d.kind, NodeKind::Unknown | NodeKind::Ancient);
        if room.is_some() || event_room {
            step(ui, 2, "What was there");
        }
        let width = ui.available_width();
        if let Some(room) = room {
            let label = d.encounter.as_ref().map_or_else(
                || {
                    if d.kind == NodeKind::Unknown {
                        "Fight (if it became one)".into()
                    } else {
                        "Choose encounter".into()
                    }
                },
                |e| v.catalog.encounter_name(e),
            );
            combo_pick(ui, "encounter", label, width, &mut d.encounter, || {
                let mut list: Vec<_> =
                    v.catalog.encounters_in(act, room).map(|e| (Some(e.id.clone()), e.name.clone())).collect();
                list.sort_by(|a, b| a.1.cmp(&b.1));
                std::iter::once((None, "None".to_string())).chain(list).collect()
            });
        }
        if event_room {
            let label = d.event.as_ref().map_or("Choose event".to_string(), |e| v.catalog.event_name(e));
            let ancient = d.kind == NodeKind::Ancient;
            combo_pick(ui, "event", label, width, &mut d.event, || {
                // Alphabetical, so a known name is quick to find.
                let mut list: Vec<_> = if ancient {
                    let ancients = v.catalog.acts.get(act).map(|a| a.ancients.as_slice()).unwrap_or_default();
                    ancients.iter().map(|id| (Some(id.clone()), v.catalog.event_name(id))).collect()
                } else {
                    v.catalog.events_in(act).map(|e| (Some(e.id.clone()), e.name.clone())).collect()
                };
                list.sort_by(|a, b| a.1.cmp(&b.1));
                std::iter::once((None, "None".to_string())).chain(list).collect()
            });
        }
    }

    step(ui, 3, "After the floor");
    egui::Grid::new("vitals").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(RichText::new("HP").color(text_dim()));
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut d.hp).range(0..=d.max_hp.max(1)));
            // Resting: one click sets the healed HP (30% of max, plus relics).
            if d.kind == NodeKind::Rest {
                let heal = spire_core::analysis::healing::rest_heal(s, v.catalog);
                let tip = format!(
                    "Rest heals {}% of max HP, plus relic bonuses: {heal} HP from {}.",
                    v.catalog.rules.rest_heal_pct, s.hp
                );
                if heal > 0
                    && ui
                        .add(egui::Button::new(RichText::new(format!("Rest +{heal}")).small().color(good())))
                        .on_hover_text(tip)
                        .clicked()
                {
                    d.hp = (s.hp + heal).min(d.max_hp);
                }
            }
        });
        ui.end_row();
        ui.label(RichText::new("Max HP").color(text_dim()));
        ui.add(egui::DragValue::new(&mut d.max_hp).range(1..=999));
        ui.end_row();
        ui.label(RichText::new("Gold").color(text_dim()));
        ui.add(egui::DragValue::new(&mut d.gold).range(0..=9999));
        ui.end_row();
    });

    step(ui, 4, "Gained");
    ui.horizontal(|ui| {
        ui.label(RichText::new("Card").color(text_dim()));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.checkbox(&mut d.add_upgraded, RichText::new("upgraded").small());
        });
    });
    let pool = s.character.as_ref().map(|c| c.as_str().to_lowercase()).unwrap_or_default();
    // Character cards first, then everything else (colorless, curses, other pools).
    let cards = v
        .catalog
        .cards
        .values()
        .filter(|c| c.color == pool)
        .chain(v.catalog.cards.values().filter(|c| c.color != pool))
        .map(|c| (c.id.as_str(), c.name.as_str(), c.text.as_str()));
    if let Some(id) = search_pick(ui, &mut d.card_query, "Search cards", cards) {
        let card = DeckCard { card: id.into(), upgraded: d.add_upgraded, enchantment: None };
        d.changes.push(Change::AddCard(card));
    }
    ui.label(RichText::new("Relic").color(text_dim()));
    let relics = v.catalog.relics.values().map(|r| (r.id.as_str(), r.name.as_str(), r.text.as_str()));
    if let Some(id) = search_pick(ui, &mut d.relic_query, "Search relics", relics) {
        d.changes.push(Change::AddRelic(id.into()));
    }
    ui.label(RichText::new("Potion").color(text_dim()));
    let potions = v.catalog.potions.values().map(|p| (p.id.as_str(), p.name.as_str(), p.text.as_str()));
    if let Some(id) = search_pick(ui, &mut d.potion_query, "Search potions", potions) {
        d.changes.push(Change::AddPotion(id.into()));
    }

    step(ui, 5, "Review");
    if d.changes.is_empty() {
        ui.label(
            RichText::new(
                "No card, relic or potion changes yet. Right-click a deck card to upgrade, enchant or remove it.",
            )
            .small()
            .color(text_faint()),
        );
    }
    let mut remove = None;
    for (i, c) in d.changes.iter().enumerate() {
        ui.horizontal(|ui| {
            let (text, color) = describe(v, c);
            ui.label(RichText::new(text).color(color));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(RichText::new("x").small())
                            .frame_when_inactive(false)
                            .min_size(egui::vec2(24.0, 24.0)),
                    )
                    .clicked()
                {
                    remove = Some(i);
                }
            });
        });
    }
    if let Some(i) = remove {
        d.changes.remove(i);
    }

    ui.add_space(crate::style::space::m());
    if primary_button(ui, &format!("Commit floor {}", s.floor + 1), true)
        .on_hover_text(format!("Commit floor ({})", crate::widgets::shortcut("Enter")))
        .clicked()
    {
        out.push(Action::Record(d.to_node(s)));
    }
    ui.add_space(crate::style::space::xs());
    ui.horizontal(|ui| {
        let has_changes = !d.is_empty_against(s);
        if ui
            .add_enabled(has_changes, egui::Button::new("Apply as correction"))
            .on_hover_text("Apply the changes without advancing the floor")
            .clicked()
        {
            out.push(Action::Record(d.to_adjustment(s)));
        }
        if ui.add(egui::Button::new(RichText::new("Clear").color(text_dim())).frame_when_inactive(false)).clicked() {
            *d = NodeDraft::for_next(s, v.catalog);
        }
    });
    ui.add_space(crate::style::space::m());
}

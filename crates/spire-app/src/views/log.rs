//! Right panel: report what happened at the node you just finished.

use eframe::egui::{self, RichText, Ui};
use spire_core::catalog::RoomType;
use spire_core::run::{Change, DeckCard, NodeKind};

use crate::actions::Action;
use crate::app::ViewCtx;
use crate::draft::NodeDraft;
use crate::widgets::search_pick;

fn describe(v: &ViewCtx, c: &Change) -> String {
    let card = |d: &DeckCard| {
        let name = v.catalog.card_name(&d.card);
        if d.upgraded { format!("{name}+") } else { name }
    };
    match c {
        Change::AddCard(d) => format!("Add {}", card(d)),
        Change::RemoveCard(d) => format!("Remove {}", card(d)),
        Change::UpgradeCard(d) => format!("Upgrade {}", card(d)),
        Change::Enchant { card: d, enchantment } => {
            format!("Enchant {} with {}", card(d), v.catalog.enchantment_name(enchantment))
        }
        Change::Transform { from, to } => format!("Transform {} into {}", card(from), card(to)),
        Change::AddRelic(r) => format!("Gain {}", v.catalog.relic_name(r)),
        Change::RemoveRelic(r) => format!("Lose {}", v.catalog.relic_name(r)),
        Change::AddPotion(p) => format!("Gain {}", v.catalog.potion_name(p)),
        Change::RemovePotion(p) => format!("Use {}", v.catalog.potion_name(p)),
        Change::SetHp(n) => format!("HP → {n}"),
        Change::SetMaxHp(n) => format!("Max HP → {n}"),
        Change::SetGold(n) => format!("Gold → {n}"),
        Change::SetFloor(n) => format!("Floor → {n}"),
        Change::SetPotionSlots(n) => format!("Potion slots → {n}"),
    }
}

pub fn show(ui: &mut Ui, v: &ViewCtx, d: &mut NodeDraft, out: &mut Vec<Action>) {
    let s = v.state;
    ui.heading(format!("Log floor {}", s.floor + 1));
    ui.label(RichText::new("Fill in after finishing a node, then commit.").small().weak());
    ui.add_space(4.0);

    ui.horizontal_wrapped(|ui| {
        for k in NodeKind::ALL {
            ui.selectable_value(&mut d.kind, k, k.label());
        }
    });

    // What was there: an encounter for fights (or a ? that became one), an event for ?/Ancient.
    if let Some(act) = &s.act {
        let room = match d.kind {
            NodeKind::Unknown => Some(RoomType::Monster),
            k => k.room(),
        };
        if let Some(room) = room {
            let label = d.encounter.as_ref().map_or("Encounter…".to_string(), |e| v.catalog.encounter_name(e));
            egui::ComboBox::from_id_salt("encounter").selected_text(label).width(ui.available_width() - 8.0).show_ui(
                ui,
                |ui| {
                    ui.selectable_value(&mut d.encounter, None, "(none / not a fight)");
                    for e in v.catalog.encounters_in(act, room) {
                        ui.selectable_value(&mut d.encounter, Some(e.id.clone()), &e.name);
                    }
                },
            );
        }
        if matches!(d.kind, NodeKind::Unknown | NodeKind::Ancient) {
            let label = d.event.as_ref().map_or("Event…".to_string(), |e| v.catalog.event_name(e));
            egui::ComboBox::from_id_salt("event").selected_text(label).width(ui.available_width() - 8.0).show_ui(
                ui,
                |ui| {
                    ui.selectable_value(&mut d.event, None, "(none)");
                    if d.kind == NodeKind::Ancient {
                        let act = v.catalog.acts.get(act);
                        for id in act.map(|a| a.ancients.as_slice()).unwrap_or_default() {
                            ui.selectable_value(&mut d.event, Some(id.clone()), v.catalog.event_name(id));
                        }
                    } else {
                        for e in v.catalog.events_in(act) {
                            ui.selectable_value(&mut d.event, Some(e.id.clone()), &e.name);
                        }
                    }
                },
            );
        }
    }

    ui.add_space(6.0);
    egui::Grid::new("vitals").num_columns(2).show(ui, |ui| {
        ui.label("HP");
        ui.add(egui::DragValue::new(&mut d.hp).range(0..=d.max_hp.max(1)));
        ui.end_row();
        ui.label("Max HP");
        ui.add(egui::DragValue::new(&mut d.max_hp).range(1..=999));
        ui.end_row();
        ui.label("Gold");
        ui.add(egui::DragValue::new(&mut d.gold).range(0..=9999));
        ui.end_row();
    });

    ui.add_space(6.0);
    ui.label(RichText::new("Add card").strong());
    ui.checkbox(&mut d.add_upgraded, "upgraded");
    let pool = s.character.as_ref().map(|c| c.as_str().to_lowercase()).unwrap_or_default();
    // Character cards first, then everything else (colorless, curses, other pools).
    let cards = v
        .catalog
        .cards
        .values()
        .filter(|c| c.color == pool)
        .chain(v.catalog.cards.values().filter(|c| c.color != pool))
        .map(|c| (c.id.as_str(), c.name.as_str(), c.text.as_str()));
    if let Some(id) = search_pick(ui, &mut d.card_query, "search cards", cards) {
        let card = DeckCard { card: id.into(), upgraded: d.add_upgraded, enchantment: None };
        d.changes.push(Change::AddCard(card));
    }

    ui.label(RichText::new("Gain relic").strong());
    let relics = v.catalog.relics.values().map(|r| (r.id.as_str(), r.name.as_str(), r.text.as_str()));
    if let Some(id) = search_pick(ui, &mut d.relic_query, "search relics", relics) {
        d.changes.push(Change::AddRelic(id.into()));
    }

    ui.label(RichText::new("Gain potion").strong());
    let potions = v.catalog.potions.values().map(|p| (p.id.as_str(), p.name.as_str(), p.text.as_str()));
    if let Some(id) = search_pick(ui, &mut d.potion_query, "search potions", potions) {
        d.changes.push(Change::AddPotion(id.into()));
    }

    ui.add_space(6.0);
    ui.separator();
    ui.label(RichText::new("Pending changes").strong());
    if d.changes.is_empty() {
        ui.label(RichText::new("None. Use the deck panel's + / x to upgrade or remove.").small().weak());
    }
    let mut remove = None;
    for (i, c) in d.changes.iter().enumerate() {
        ui.horizontal(|ui| {
            if ui.small_button("x").clicked() {
                remove = Some(i);
            }
            ui.label(describe(v, c));
        });
    }
    if let Some(i) = remove {
        d.changes.remove(i);
    }

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui.button(RichText::new(format!("Commit {}", d.kind.label())).strong()).clicked() {
            out.push(Action::Record(d.to_node(s)));
        }
        let has_changes = !d.is_empty_against(s);
        if ui
            .add_enabled(has_changes, egui::Button::new("Apply as correction"))
            .on_hover_text("Apply the changes without advancing the floor")
            .clicked()
        {
            out.push(Action::Record(d.to_adjustment(s)));
        }
    });
    if ui.small_button("Clear").clicked() {
        *d = NodeDraft::from_state(s);
    }
}

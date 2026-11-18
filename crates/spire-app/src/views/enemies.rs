//! Enemy catalog: every encounter, its lineups, and each monster's moves.

use eframe::egui::{self, RichText, Sense, Ui, vec2};
use spire_core::analysis::threats::{Scaling, preview};
use spire_core::catalog::{Encounter, RoomType};
use spire_core::ids::{ActId, EncounterId, MonsterId};
use spire_core::playbook::contains_ci;

use crate::app::ViewCtx;
use crate::style::color::*;
use crate::widgets::{eyebrow, hairline, panel, segmented};

#[derive(Default)]
pub struct EnemyBrowser {
    pub query: String,
    pub act: Option<ActId>,
    pub room: Option<RoomType>,
    pub selected: Option<EncounterId>,
    /// Ascension the stats are shown at; `None` follows the current run.
    pub ascension: Option<u32>,
}

impl EnemyBrowser {
    fn ascension(&self, v: &ViewCtx) -> u32 {
        self.ascension.unwrap_or(v.state.ascension)
    }
}

fn act_order(v: &ViewCtx, e: &Encounter) -> (u8, String) {
    let act = e.act.as_ref().and_then(|a| v.catalog.acts.get(a));
    (act.map_or(9, |a| a.index), act.map_or_else(|| "Events".into(), |a| a.name.clone()))
}

pub fn filters(ui: &mut Ui, v: &ViewCtx, b: &mut EnemyBrowser) {
    ui.add_space(crate::style::space::s());
    ui.label(eyebrow("Find enemies"));
    hairline(ui);
    ui.add(egui::TextEdit::singleline(&mut b.query).hint_text("Encounter or monster").desired_width(f32::INFINITY));

    ui.add_space(crate::style::space::s());
    ui.label(eyebrow("Act"));
    let mut acts: Vec<_> = v.catalog.acts.values().collect();
    acts.sort_by_key(|a| (a.index, a.name.clone()));
    let mut options: Vec<(Option<ActId>, &str)> = vec![(None, "All")];
    options.extend(acts.iter().map(|a| (Some(a.id.clone()), a.name.as_str())));
    let mut act = b.act.clone();
    if segmented(ui, &mut act, &options) {
        b.act = act;
    }

    ui.add_space(crate::style::space::s());
    ui.label(eyebrow("Room"));
    let rooms = [
        (None, "All"),
        (Some(RoomType::Monster), "Enemy"),
        (Some(RoomType::Elite), "Elite"),
        (Some(RoomType::Boss), "Boss"),
    ];
    segmented(ui, &mut b.room, &rooms);

    ui.add_space(crate::style::space::s());
    ui.label(eyebrow("Stats at ascension"));
    let max = v.catalog.rules.max_ascension;
    let current = b.ascension(v);
    egui::ComboBox::from_id_salt("enemy-asc")
        .selected_text(if b.ascension.is_none() { format!("A{current} (this run)") } else { format!("A{current}") })
        .width(ui.available_width())
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut b.ascension, None, "Follow this run");
            for a in 0..=max {
                ui.selectable_value(&mut b.ascension, Some(a), format!("A{a}"));
            }
        });
    let scaling =
        Scaling { tough: v.catalog.rules.tough_enemies(current), deadly: v.catalog.rules.deadly_enemies(current) };
    let note = match (scaling.tough, scaling.deadly) {
        (true, true) => "Ascension HP and damage",
        (true, false) => "Ascension HP, normal damage",
        _ => "Normal HP and damage",
    };
    ui.label(RichText::new(note).small().color(text_faint()));
}

pub fn list(ui: &mut Ui, v: &ViewCtx, b: &mut EnemyBrowser) {
    let q = b.query.trim().to_string();
    let mut list: Vec<&Encounter> = v
        .catalog
        .encounters
        .values()
        .filter(|e| b.act.as_ref().is_none_or(|a| e.act.as_ref() == Some(a)))
        .filter(|e| b.room.is_none_or(|r| e.room == r))
        .filter(|e| {
            q.is_empty()
                || contains_ci(&e.name, &q)
                || e.monsters.iter().any(|m| contains_ci(&v.catalog.monster_name(m), &q))
        })
        .collect();
    list.sort_by_key(|e| (act_order(v, e), e.room, e.name.clone()));

    ui.horizontal(|ui| {
        ui.label(eyebrow("Encounters"));
        ui.label(RichText::new(list.len().to_string()).small().color(text_faint()));
    });
    hairline(ui);
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        let mut heading: Option<(String, RoomType)> = None;
        for e in list {
            let group = (act_order(v, e).1, e.room);
            if heading.as_ref() != Some(&group) {
                ui.add_space(crate::style::space::s());
                ui.label(RichText::new(format!("{} · {}", group.0, group.1.label())).small().color(text_faint()));
                heading = Some(group);
            }
            let selected = b.selected.as_ref() == Some(&e.id);
            let h = ui.spacing().interact_size.y;
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::click());
            if ui.is_rect_visible(rect) {
                let painter = ui.painter();
                if selected || resp.hovered() {
                    painter.rect_filled(rect, 0.0, if selected { surface_hi() } else { surface() });
                }
                let mid = rect.center().y;
                painter.text(
                    egui::pos2(rect.min.x + 8.0, mid),
                    egui::Align2::LEFT_CENTER,
                    &e.name,
                    egui::TextStyle::Body.resolve(ui.style()),
                    if v.state.encounters_seen.contains(&e.id) { text_dim() } else { text() },
                );
                let tag = match (e.weak, v.state.encounters_seen.contains(&e.id)) {
                    (_, true) => "fought",
                    (true, false) => "early pool",
                    _ => "",
                };
                painter.text(
                    egui::pos2(rect.max.x - 6.0, mid),
                    egui::Align2::RIGHT_CENTER,
                    tag,
                    egui::TextStyle::Small.resolve(ui.style()),
                    text_faint(),
                );
            }
            if resp.clicked() {
                b.selected = Some(e.id.clone());
            }
        }
    });
}

pub fn detail(ui: &mut Ui, v: &ViewCtx, b: &EnemyBrowser) {
    ui.add_space(crate::style::space::s());
    let Some(enc) = b.selected.as_ref().and_then(|id| v.catalog.encounters.get(id)) else {
        ui.label(eyebrow("Encounter"));
        hairline(ui);
        ui.label(RichText::new("Select an encounter to see its lineups and every monster's moves.").color(text_dim()));
        return;
    };
    let asc = b.ascension(v);
    let scaling = Scaling { tough: v.catalog.rules.tough_enemies(asc), deadly: v.catalog.rules.deadly_enemies(asc) };
    let act = enc.act.as_ref().and_then(|a| v.catalog.acts.get(a));
    ui.label(crate::widgets::title(&enc.name));
    ui.label(
        RichText::new(format!(
            "{} · {}{}",
            act.map_or("Event fight", |a| a.name.as_str()),
            enc.room.label(),
            if enc.weak { " · early pool" } else { "" }
        ))
        .color(text_dim()),
    );
    ui.add_space(crate::style::space::s());
    let p = preview(enc, v.catalog, v.playbook, act.map_or(0, |a| a.index), scaling, &v.dash.deck, false);
    let title = if v.state.started() { "Against your deck" } else { "Threat" };
    panel(ui, title, Some(&format!("A{asc}")), |ui| {
        crate::views::threats::preview_body(ui, &p);
    });

    // Every monster that can take part: lineup members, then spawns.
    let mut monsters: Vec<&MonsterId> = Vec::new();
    for m in enc.lineups.iter().flat_map(|l| l.monsters.iter()).chain(enc.monsters.iter()).chain(enc.spawns.iter()) {
        if !monsters.contains(&m) {
            monsters.push(m);
        }
    }
    ui.add_space(crate::style::space::s());
    ui.label(eyebrow("Monsters"));
    hairline(ui);
    for id in monsters {
        let Some(m) = v.catalog.monsters.get(id) else { continue };
        let (min, max) = m.hp_at(scaling.tough);
        let hp = if max > min {
            format!("{min}–{max} HP")
        } else if min == 0 {
            "HP varies".into()
        } else {
            format!("{min} HP")
        };
        ui.add_space(crate::style::space::s());
        ui.horizontal(|ui| {
            ui.label(RichText::new(&m.name).family(crate::style::strong_family()).color(text()));
            if enc.spawns.contains(id) {
                ui.label(RichText::new("joins mid-fight").small().color(warn()));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(hp).small().color(text_dim()));
            });
        });
        if !m.innate_powers.is_empty() {
            let powers: Vec<String> = m.innate_powers.iter().map(|p| format!("{} {}", p.power, p.amount)).collect();
            ui.label(RichText::new(format!("Starts with {}", powers.join(", "))).small().color(text_faint()));
        }
        egui::Grid::new(("moves", id)).num_columns(3).spacing([14.0, 2.0]).striped(false).show(ui, |ui| {
            for mv in &m.moves {
                ui.label(RichText::new(&mv.name).small().color(text()));
                ui.label(RichText::new(&mv.intent).small().color(text_faint()));
                let per = if scaling.deadly { mv.damage_asc.or(mv.damage) } else { mv.damage };
                let mut parts = Vec::new();
                if let Some(d) = per {
                    parts.push(match mv.hits {
                        Some(h) if h > 1 => format!("{d}×{h} dmg"),
                        _ => format!("{d} dmg"),
                    });
                }
                if let Some(bl) = mv.block {
                    parts.push(format!("{bl} block"));
                }
                ui.label(RichText::new(parts.join(", ")).small().color(if per.is_some() { bad() } else { text_dim() }));
                ui.end_row();
            }
        });
    }
}

//! Starting a run and entering an act.

use eframe::egui::{self, RichText, Sense, Stroke, StrokeKind, Ui, vec2};

use crate::actions::Action;
use crate::app::{UiState, ViewCtx};
use crate::style::{self, color::*};
use crate::widgets::{eyebrow, panel, primary_button, segmented};

pub fn show(ui: &mut Ui, v: &ViewCtx, st: &mut UiState, out: &mut Vec<Action>) {
    ui.add_space(28.0);
    // A centred column with left-aligned content, like a form.
    let width = ui.available_width().min(760.0);
    ui.horizontal(|ui| {
        ui.add_space(((ui.available_width() - width) / 2.0).max(0.0));
        ui.vertical(|ui| {
            ui.set_width(width);
            ui.label(RichText::new("New run").size(28.0).color(text()));
            ui.label(
                RichText::new(
                    "Pick a character and ascension. After each floor, log what changed and the dashboard updates.",
                )
                .color(text_dim()),
            );
            ui.add_space(18.0);

            ui.label(eyebrow("Character"));
            ui.add_space(4.0);
            let characters: Vec<_> = v.catalog.characters.values().collect();
            let cols = 3;
            let gap = 10.0;
            let width = (ui.available_width() - gap * (cols as f32 - 1.0)) / cols as f32;
            for row in characters.chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for c in row {
                        let picked = st.new_character.as_ref() == Some(&c.id);
                        if character_card(ui, v, c, picked, width).clicked() {
                            st.new_character = Some(c.id.clone());
                        }
                    }
                });
                ui.add_space(gap);
            }

            ui.add_space(8.0);
            ui.label(eyebrow("Ascension"));
            ui.add_space(4.0);
            let max = v.catalog.rules.max_ascension;
            st.new_ascension = st.new_ascension.min(max);
            let labels: Vec<(u32, String)> = (0..=max).map(|n| (n, n.to_string())).collect();
            let options: Vec<(u32, &str)> = labels.iter().map(|(n, s)| (*n, s.as_str())).collect();
            segmented(ui, &mut st.new_ascension, &options);
            if let Some(level) = v.catalog.ascensions.get(&st.new_ascension) {
                ui.add_space(4.0);
                ui.label(RichText::new(&level.name).strong().color(if st.new_ascension > 0 {
                    accent()
                } else {
                    text()
                }));
                ui.label(RichText::new(&level.text).small().color(text_dim()));
                if st.new_ascension > 1 {
                    ui.label(RichText::new("Includes every lower level.").small().color(text_faint()));
                }
            }

            ui.add_space(18.0);
            ui.allocate_ui(vec2(260.0, 36.0), |ui| {
                if primary_button(ui, "Start run", st.new_character.is_some()).clicked()
                    && let Some(character) = st.new_character.clone()
                {
                    out.push(Action::NewRun { character, ascension: st.new_ascension });
                }
            });
        });
    });
}

fn character_card(
    ui: &mut Ui,
    v: &ViewCtx,
    c: &spire_core::catalog::Character,
    picked: bool,
    width: f32,
) -> egui::Response {
    let accent = style::character_color(c.id.as_str());
    let (rect, response) = ui.allocate_exact_size(vec2(width, 92.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let border = if picked {
            accent
        } else if response.hovered() {
            border_hi()
        } else {
            border()
        };
        painter.rect_stroke(rect, 0.0, Stroke::new(if picked { 2.0 } else { 1.0 }, border), StrokeKind::Inside);
        painter.rect_filled(egui::Rect::from_min_size(rect.min, vec2(4.0, rect.height())), 0.0, accent);
        let x = rect.min.x + 16.0;
        let name = c.name.trim_start_matches("The ");
        painter.text(
            egui::pos2(x, rect.min.y + 12.0),
            egui::Align2::LEFT_TOP,
            name,
            egui::FontId::new(18.0, style::strong_family()),
            text(),
        );
        let relic = c.starting_relics.first().map(|r| v.catalog.relic_name(r)).unwrap_or_default();
        painter.text(
            egui::pos2(x, rect.min.y + 40.0),
            egui::Align2::LEFT_TOP,
            format!("{} HP  ·  {} gold  ·  {} cards", c.max_hp, c.gold, c.starting_deck.len()),
            egui::FontId::proportional(13.0),
            text_dim(),
        );
        painter.text(
            egui::pos2(x, rect.min.y + 60.0),
            egui::Align2::LEFT_TOP,
            relic,
            egui::FontId::proportional(13.0),
            accent,
        );
    }
    response.on_hover_text(&c.description)
}

/// Act picker: shown when the run has no current act (start, or after a boss).
pub fn enter_act(ui: &mut Ui, v: &ViewCtx, out: &mut Vec<Action>) {
    let s = v.state;
    let next = if s.nodes.is_empty() { 0 } else { s.act_index + 1 };
    act_picker(ui, v, next, out);
}

pub fn act_picker(ui: &mut Ui, v: &ViewCtx, index: u8, out: &mut Vec<Action>) {
    let acts: Vec<_> = v.catalog.acts_at(index).collect();
    panel(ui, &format!("Enter Act {}", index + 1), Some("Pick the act, then its boss from the map"), |ui| {
        if acts.is_empty() {
            ui.label(RichText::new("No act data for this act. Update data from the Data window.").color(text_dim()));
            return;
        }
        for act in acts {
            ui.add_space(4.0);
            ui.label(RichText::new(&act.name).heading().color(text()));
            ui.horizontal_wrapped(|ui| {
                for boss in &act.bosses {
                    if ui.button(v.catalog.encounter_name(boss)).clicked() {
                        out.push(Action::EnterAct { act: act.id.clone(), index, boss: Some(boss.clone()) });
                    }
                }
                if ui.button(RichText::new("Boss not shown yet").color(text_dim())).clicked() {
                    out.push(Action::EnterAct { act: act.id.clone(), index, boss: None });
                }
            });
            ui.add_space(6.0);
        }
    });
}

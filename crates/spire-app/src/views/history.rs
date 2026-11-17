//! History: runs logged in this app, with floor-by-floor replay.
//!
//! Runs are read from disk when the tab opens (and again after a run ends),
//! never per frame. Everything shown is derived from each run's event log.

use eframe::egui::{self, RichText, Ui};
use spire_core::analysis::deck;
use spire_core::analysis::sim::{DeckSim, simulate};
use spire_core::analysis::timeline::{Timeline, timeline};
use spire_core::ids::CharacterId;
use spire_core::run::{Outcome, Run, RunState, RunSummary};

use crate::app::ViewCtx;
use crate::style::{self, color::*};
use crate::views::analytics;
use crate::widgets::{eyebrow, hairline, panel, segmented, stat};

const GAP: f32 = 12.0;

pub struct ArchivedRun {
    pub id: String,
    pub run: Run,
    pub summary: RunSummary,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OutcomeFilter {
    #[default]
    All,
    Victory,
    Died,
    Abandoned,
}

impl OutcomeFilter {
    fn matches(self, o: Option<&Outcome>) -> bool {
        match self {
            OutcomeFilter::All => true,
            OutcomeFilter::Victory => matches!(o, Some(Outcome::Victory)),
            OutcomeFilter::Died => matches!(o, Some(Outcome::Died { .. })),
            OutcomeFilter::Abandoned => matches!(o, Some(Outcome::Abandoned)),
        }
    }
}

/// The selected run, analysed once, and the floor being replayed.
struct Detail {
    id: String,
    timeline: Timeline,
    /// Index into `timeline.points`.
    floor: usize,
    /// State and deck simulation at `floor`, recomputed when it changes.
    at: Option<(usize, RunState, DeckSim)>,
}

#[derive(Default)]
pub struct HistoryBrowser {
    runs: Vec<ArchivedRun>,
    loaded: bool,
    character: Option<CharacterId>,
    outcome: OutcomeFilter,
    selected: Option<String>,
    detail: Option<Detail>,
    show_all_odds: bool,
}

impl HistoryBrowser {
    pub fn needs_load(&self) -> bool {
        !self.loaded
    }

    /// Replaces the list with runs read from disk (newest first).
    pub fn load(&mut self, runs: Vec<(String, Run)>) {
        self.runs = runs.into_iter().map(|(id, run)| ArchivedRun { summary: run.summary(), id, run }).collect();
        self.loaded = true;
        if self.selected.as_ref().is_none_or(|s| !self.runs.iter().any(|r| r.id == *s)) {
            self.selected = self.runs.first().map(|r| r.id.clone());
        }
    }

    /// Selects a run by id (its file name without `.json`).
    pub fn select(&mut self, id: &str) {
        self.selected = Some(id.to_string());
    }

    /// Re-read from disk next time the tab shows (a run was just filed).
    pub fn invalidate(&mut self) {
        self.loaded = false;
    }

    fn visible(&self) -> impl Iterator<Item = &ArchivedRun> {
        self.runs.iter().filter(|r| {
            self.outcome.matches(r.summary.outcome.as_ref())
                && self.character.as_ref().is_none_or(|c| r.summary.character.as_ref() == Some(c))
        })
    }
}

fn outcome_color(o: Option<&Outcome>) -> egui::Color32 {
    match o {
        Some(Outcome::Victory) => good(),
        Some(Outcome::Died { .. }) => bad(),
        _ => text_dim(),
    }
}

fn character_name(v: &ViewCtx, s: &RunSummary) -> String {
    let Some(c) = &s.character else { return "?".into() };
    v.catalog.characters.get(c).map_or_else(|| c.to_string(), |x| x.name.trim_start_matches("The ").to_string())
}

fn date(s: &RunSummary) -> &str {
    s.started_at.as_deref().map_or("", |d| d.get(..10).unwrap_or(d))
}

/// Left panel: filters, totals and the run list.
pub fn list(ui: &mut Ui, v: &ViewCtx, h: &mut HistoryBrowser) {
    ui.add_space(8.0);
    ui.label(eyebrow("Run history"));
    hairline(ui);
    if h.runs.is_empty() {
        ui.label(
            RichText::new("No finished runs yet. Runs you end or abandon in this app are filed here.")
                .color(text_dim()),
        );
        return;
    }
    let mut characters: Vec<CharacterId> = h.runs.iter().filter_map(|r| r.summary.character.clone()).collect();
    characters.sort();
    characters.dedup();
    let mut options: Vec<(Option<CharacterId>, String)> = vec![(None, "All".into())];
    options.extend(characters.into_iter().map(|c| {
        let name = v
            .catalog
            .characters
            .get(&c)
            .map_or_else(|| c.to_string(), |x| x.name.trim_start_matches("The ").to_string());
        (Some(c), name)
    }));
    let options: Vec<(Option<CharacterId>, &str)> = options.iter().map(|(c, n)| (c.clone(), n.as_str())).collect();
    segmented(ui, &mut h.character, &options);
    ui.add_space(4.0);
    segmented(
        ui,
        &mut h.outcome,
        &[
            (OutcomeFilter::All, "All"),
            (OutcomeFilter::Victory, "Wins"),
            (OutcomeFilter::Died, "Deaths"),
            (OutcomeFilter::Abandoned, "Abandoned"),
        ],
    );

    let shown: Vec<&ArchivedRun> = h.visible().collect();
    let wins = shown.iter().filter(|r| matches!(r.summary.outcome, Some(Outcome::Victory))).count();
    let avg_floor = if shown.is_empty() {
        0.0
    } else {
        shown.iter().map(|r| r.summary.floor as f32).sum::<f32>() / shown.len() as f32
    };
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        stat(ui, "Runs", shown.len().to_string(), text());
        ui.add_space(12.0);
        let rate = if shown.is_empty() { 0.0 } else { wins as f32 * 100.0 / shown.len() as f32 };
        stat(ui, "Wins", format!("{wins} ({rate:.0}%)"), text());
        ui.add_space(12.0);
        stat(ui, "Avg floor", format!("{avg_floor:.1}"), text());
    });
    hairline(ui);

    let mut pick = None;
    egui::ScrollArea::vertical().show(ui, |ui| {
        for r in &shown {
            let s = &r.summary;
            let selected = h.selected.as_ref() == Some(&r.id);
            let name = character_name(v, s);
            let resp = ui
                .scope(|ui| {
                    ui.horizontal(|ui| {
                        let color = s.character.as_ref().map_or(text_dim(), |c| style::character_color(c.as_str()));
                        crate::widgets::stripe(ui, color, 30.0);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            ui.horizontal(|ui| {
                                let title = RichText::new(format!("{name} · A{}", s.ascension));
                                ui.label(if selected { title.color(accent()).strong() } else { title.color(text()) });
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let outcome = s.outcome.as_ref();
                                    ui.label(
                                        RichText::new(outcome.map_or("Ongoing", Outcome::label))
                                            .small()
                                            .color(outcome_color(outcome)),
                                    );
                                });
                            });
                            ui.label(
                                RichText::new(format!("{} · floor {} · {} cards", date(s), s.floor, s.deck_size))
                                    .small()
                                    .color(text_faint()),
                            );
                        });
                    });
                })
                .response
                .interact(egui::Sense::click());
            if resp.clicked() {
                pick = Some(r.id.clone());
            }
            ui.add_space(4.0);
        }
    });
    if let Some(id) = pick {
        h.selected = Some(id);
    }
}

/// Centre: the selected run's charts and a floor-by-floor replay.
pub fn detail(ui: &mut Ui, v: &ViewCtx, h: &mut HistoryBrowser) {
    let Some(selected) = h.selected.clone() else {
        ui.label(RichText::new("Pick a run on the left.").color(text_dim()));
        return;
    };
    let Some(archived) = h.runs.iter().find(|r| r.id == selected) else { return };
    if h.detail.as_ref().is_none_or(|d| d.id != selected) {
        let timeline = timeline(&archived.run, v.catalog, v.playbook);
        let floor = timeline.points.len().saturating_sub(1);
        h.detail = Some(Detail { id: selected.clone(), timeline, floor, at: None });
    }
    let Some(d) = h.detail.as_mut() else { return };

    let s = &archived.summary;
    let name = character_name(v, s);
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("{name} · Ascension {}", s.ascension)).heading().color(text()));
        let outcome = s.outcome.as_ref();
        let mut word = outcome.map_or("Ongoing", Outcome::label).to_string();
        if let Some(Outcome::Died { encounter: Some(e) }) = outcome {
            word = format!("Died to {}", v.catalog.encounter_name(e));
        }
        ui.label(RichText::new(word).color(outcome_color(outcome)));
    });
    ui.label(
        RichText::new(format!("{} · reached floor {} · act {}", date(s), s.floor, s.act_index + 1))
            .small()
            .color(text_faint()),
    );
    ui.add_space(GAP);

    if d.timeline.points.is_empty() {
        ui.label(RichText::new("No floors were logged in this run.").color(text_dim()));
        return;
    }
    let wide = ui.available_width() > 900.0;
    let left = |ui: &mut Ui, d: &mut Detail| {
        analytics::run_section(ui, &format!("hist-{}", d.id), v.catalog, v.playbook, &d.timeline, None);
    };
    let right = |ui: &mut Ui, d: &mut Detail, show_all: &mut bool| {
        replay(ui, v, &archived.run, d);
        ui.add_space(GAP);
        if let Some((_, _, sim)) = &d.at {
            analytics::deck_section(ui, &format!("hist-{}", d.id), sim, show_all);
        }
    };
    if wide {
        ui.columns(2, |cols| {
            let [a, b] = cols else { return };
            left(a, d);
            right(b, d, &mut h.show_all_odds);
        });
    } else {
        right(ui, d, &mut h.show_all_odds);
        ui.add_space(GAP);
        left(ui, d);
    }
}

/// A floor slider and the run as it stood after that floor.
fn replay(ui: &mut Ui, v: &ViewCtx, run: &Run, d: &mut Detail) {
    let last = d.timeline.points.len() - 1;
    panel(ui, "Replay", Some("the run after each floor"), |ui| {
        floor_strip(ui, v, &d.timeline, &mut d.floor);
        ui.horizontal(|ui| {
            if ui.add_enabled(d.floor > 0, egui::Button::new("Previous")).clicked() {
                d.floor -= 1;
            }
            if ui.add_enabled(d.floor < last, egui::Button::new("Next")).clicked() {
                d.floor += 1;
            }
            let p = &d.timeline.points[d.floor];
            ui.label(RichText::new(format!("Floor {} · {}", p.floor, p.kind.label())).strong().color(text()));
        });
        let point = &d.timeline.points[d.floor];
        if d.at.as_ref().is_none_or(|(i, _, _)| *i != d.floor) {
            let state = run.state_at(point.event_index);
            let sim = simulate(&state, v.catalog, &v.playbook.simulation);
            d.at = Some((d.floor, state, sim));
        }
        let Some((_, state, _)) = &d.at else { return };
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            stat(ui, "HP", format!("{}/{}", state.hp, state.max_hp), style::hp_color(state.hp_fraction()));
            ui.add_space(16.0);
            stat(ui, "Gold", state.gold.to_string(), text());
            ui.add_space(16.0);
            stat(ui, "Deck", state.deck.len().to_string(), text());
            ui.add_space(16.0);
            stat(ui, "Act", (state.act_index + 1).to_string(), text());
        });
        if let Some(e) = &point.encounter {
            let lost = if point.hp_lost > 0 { format!(", lost {} HP", point.hp_lost) } else { String::new() };
            ui.label(RichText::new(format!("Fought {}{lost}", v.catalog.encounter_name(e))).color(text_dim()));
        }
        ui.add_space(6.0);
        ui.label(eyebrow("Deck"));
        let rows = deck::deck_rows(&deck::classify(&state.deck, v.catalog, v.playbook));
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            for r in &rows {
                let name = if r.count > 1 { format!("{}× {}", r.count, r.name) } else { r.name.clone() };
                ui.label(RichText::new(name).color(style::card_type_color(r.card_type)));
            }
        });
        if !state.relics.is_empty() {
            ui.add_space(6.0);
            ui.label(eyebrow("Relics"));
            let relics: Vec<String> = state.relics.iter().map(|r| v.catalog.relic_name(r)).collect();
            ui.label(RichText::new(relics.join(", ")).color(text_dim()));
        }
    });
}

/// One column per logged floor, as tall as HP was after it; click to replay.
fn floor_strip(ui: &mut Ui, v: &ViewCtx, t: &Timeline, floor: &mut usize) {
    let n = t.points.len();
    let width = ui.available_width();
    let col = (width / n as f32).clamp(4.0, 16.0);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(col * n as f32, 36.0), egui::Sense::click());
    let painter = ui.painter();
    for (i, p) in t.points.iter().enumerate() {
        let x = rect.left() + i as f32 * col;
        let frac = if p.max_hp > 0 { (p.hp as f32 / p.max_hp as f32).clamp(0.0, 1.0) } else { 0.0 };
        let h = (rect.height() - 4.0) * frac.max(0.04);
        let bar =
            egui::Rect::from_min_max(egui::pos2(x + 1.0, rect.bottom() - h), egui::pos2(x + col - 1.0, rect.bottom()));
        painter.rect_filled(bar, 0.0, style::hp_color(frac).gamma_multiply(if i == *floor { 1.0 } else { 0.55 }));
        if i == *floor {
            let whole = egui::Rect::from_min_max(egui::pos2(x, rect.top()), egui::pos2(x + col, rect.bottom()));
            painter.rect_stroke(whole, 0.0, egui::Stroke::new(1.5, accent()), egui::StrokeKind::Inside);
        }
        if p.kind == spire_core::run::NodeKind::Boss {
            painter.hline(x..=x + col, rect.top(), egui::Stroke::new(2.0, text_dim()));
        }
    }
    let at = |pos: egui::Pos2| (((pos.x - rect.left()) / col) as usize).min(n - 1);
    if let Some(pos) = response.hover_pos() {
        let p = &t.points[at(pos)];
        let what = p.encounter.as_ref().map_or_else(|| p.kind.label().to_string(), |e| v.catalog.encounter_name(e));
        response.clone().on_hover_text(format!("Floor {} · {what}\nHP {}/{}", p.floor, p.hp, p.max_hp));
    }
    if response.clicked()
        && let Some(pos) = response.interact_pointer_pos()
    {
        *floor = at(pos);
    }
}

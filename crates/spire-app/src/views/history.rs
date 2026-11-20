//! History: runs logged in this app, with floor-by-floor replay.
//!
//! Runs are read from disk when the tab opens (and again after a run ends),
//! never per frame. Everything shown is derived from each run's event log.

use eframe::egui::{self, RichText, Ui};
use spire_core::analysis::deck;
use spire_core::analysis::report::{RunReport, report};
use spire_core::analysis::sim::{DeckSim, simulate};
use spire_core::analysis::timeline::{Timeline, timeline};
use spire_core::ids::CharacterId;
use spire_core::run::{Outcome, Run, RunState, RunSummary};

use crate::actions::Action;
use crate::app::ViewCtx;
use crate::style::{self, color::*};
use crate::views::{analytics, report as report_view};
use crate::widgets::{caption, eyebrow, hairline, panel, secondary, segmented, stat};

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
    /// How it went, answer first.
    report: RunReport,
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
    /// The Edit run pop-over, while open.
    editing: Option<Editing>,
}

/// A run being edited: a draft copy of its log, saved only on request.
struct Editing {
    id: String,
    original: Run,
    draft: Run,
    notes: String,
    confirm_delete: bool,
}

impl HistoryBrowser {
    pub fn needs_load(&self) -> bool {
        !self.loaded
    }

    /// Replaces the list with runs read from disk (newest first).
    pub fn load(&mut self, runs: Vec<(String, Run)>) {
        self.runs = runs.into_iter().map(|(id, run)| ArchivedRun { summary: run.summary(), id, run }).collect();
        self.loaded = true;
        // The selected run may have been edited: analyse it again.
        self.detail = None;
        if self.selected.as_ref().is_none_or(|s| !self.runs.iter().any(|r| r.id == *s)) {
            self.selected = self.runs.first().map(|r| r.id.clone());
        }
    }

    /// Selects a run by id (its file name without `.json`), clearing the
    /// filters so it's in the list.
    pub fn select(&mut self, id: &str) {
        self.selected = Some(id.to_string());
        self.character = None;
        self.outcome = OutcomeFilter::All;
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
    ui.add_space(crate::style::space::s());
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
    ui.add_space(crate::style::space::xs());
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
    ui.add_space(crate::style::space::s());
    ui.horizontal(|ui| {
        stat(ui, "Runs", shown.len().to_string(), text());
        ui.add_space(crate::style::space::m());
        let rate = if shown.is_empty() { 0.0 } else { wins as f32 * 100.0 / shown.len() as f32 };
        stat(ui, "Wins", format!("{wins} ({rate:.0}%)"), text());
        ui.add_space(crate::style::space::m());
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
                            ui.label(caption(format!("{} · floor {} · {} cards", date(s), s.floor, s.deck_size)));
                        });
                    });
                })
                .response
                .interact(egui::Sense::click());
            if resp.clicked() {
                pick = Some(r.id.clone());
            }
            ui.add_space(crate::style::space::xs());
        }
    });
    if let Some(id) = pick {
        h.selected = Some(id);
    }
}

/// Centre: the selected run's charts and a floor-by-floor replay.
pub fn detail(ui: &mut Ui, v: &ViewCtx, h: &mut HistoryBrowser, out: &mut Vec<Action>) {
    edit_dialog(ui, v, h, out);
    let Some(selected) = h.selected.clone() else {
        ui.label(RichText::new("Pick a run on the left.").color(text_dim()));
        return;
    };
    let Some(archived) = h.runs.iter().find(|r| r.id == selected) else { return };
    if h.detail.as_ref().is_none_or(|d| d.id != selected) {
        let timeline = timeline(&archived.run, v.catalog, v.playbook);
        let report = report(&archived.run, v.catalog, v.playbook);
        let floor = timeline.points.len().saturating_sub(1);
        h.detail = Some(Detail { id: selected.clone(), timeline, report, floor, at: None });
    }
    let Some(d) = h.detail.as_mut() else { return };

    let s = &archived.summary;
    let name = character_name(v, s);
    ui.horizontal(|ui| {
        ui.label(crate::widgets::title(format!("{name} · Ascension {}", s.ascension)));
        let outcome = s.outcome.as_ref();
        let mut word = outcome.map_or("Ongoing", Outcome::label).to_string();
        if let Some(Outcome::Died { encounter: Some(e) }) = outcome {
            word = format!("Died to {}", v.catalog.encounter_name(e));
        }
        ui.label(RichText::new(word).color(outcome_color(outcome)));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Edit run…").on_hover_text("Fix the ascension, result or floors, or add notes").clicked() {
                h.editing = Some(Editing {
                    id: archived.id.clone(),
                    original: archived.run.clone(),
                    draft: archived.run.clone(),
                    notes: archived.run.notes.clone().unwrap_or_default(),
                    confirm_delete: false,
                });
            }
        });
    });
    if let Some(notes) = archived.run.notes.as_deref().filter(|n| !n.trim().is_empty()) {
        ui.label(RichText::new(notes).color(text_dim()));
    }
    ui.label(caption(format!("{} · reached floor {} · act {}", date(s), s.floor, s.act_index + 1)));
    ui.add_space(GAP);

    if d.timeline.points.is_empty() {
        ui.label(RichText::new("No floors were logged in this run.").color(text_dim()));
        return;
    }
    let wide = ui.available_width() > 900.0;
    report_view::summary(ui, v, &d.report);
    ui.add_space(GAP);
    let left = |ui: &mut Ui, d: &mut Detail| {
        let salt = format!("hist-{}", d.id);
        analytics::run_section(ui, &salt, v.catalog, &d.timeline, &d.report.turning);
    };
    let right = |ui: &mut Ui, d: &mut Detail, show_all: &mut bool| {
        replay(ui, v, &archived.run, d);
        ui.add_space(GAP);
        let at = d.timeline.points.get(d.floor).map_or(0, |p| p.floor);
        if let Some(floor) = report_view::decisions(ui, v, &d.report, at)
            && let Some(i) = d.timeline.points.iter().position(|p| p.floor == floor)
        {
            d.floor = i;
        }
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
        ui.add_space(crate::style::space::s());
        ui.horizontal(|ui| {
            stat(ui, "HP", format!("{}/{}", state.hp, state.max_hp), style::hp_color(state.hp_fraction()));
            ui.add_space(crate::style::space::l());
            stat(ui, "Gold", state.gold.to_string(), text());
            ui.add_space(crate::style::space::l());
            stat(ui, "Deck", state.deck.len().to_string(), text());
            ui.add_space(crate::style::space::l());
            stat(ui, "Act", (state.act_index + 1).to_string(), text());
        });
        if let Some(e) = &point.encounter {
            let lost = if point.hp_lost > 0 { format!(", lost {} HP", point.hp_lost) } else { String::new() };
            ui.label(RichText::new(format!("Fought {}{lost}", v.catalog.encounter_name(e))).color(text_dim()));
        }
        ui.add_space(crate::style::space::s());
        ui.label(eyebrow("Deck"));
        let rows = deck::deck_rows(&deck::classify_run(state, v.catalog, v.playbook));
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            for r in &rows {
                let name = if r.count > 1 { format!("{}× {}", r.count, r.name) } else { r.name.clone() };
                ui.label(RichText::new(name).color(style::card_type_color(r.card_type)));
            }
        });
        if !state.relics.is_empty() {
            ui.add_space(crate::style::space::s());
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

/// The Edit run pop-over: ascension, result, notes and floors, on a draft
/// that's written to disk only on Save.
fn edit_dialog(ui: &mut Ui, v: &ViewCtx, h: &mut HistoryBrowser, out: &mut Vec<Action>) {
    let Some(e) = h.editing.as_mut() else { return };
    let mut close = false;
    let modal = crate::widgets::modal("edit-run").show(ui.ctx(), |ui| {
        ui.set_width(460.0);
        ui.label(crate::widgets::title("Edit run"));
        let s = e.draft.summary();
        ui.label(secondary(format!("{} · started {}", character_name(v, &s), date(&s))));
        hairline(ui);

        // Ascension: the start is resolved again for the new level.
        ui.horizontal(|ui| {
            ui.label(RichText::new("Ascension").color(text_dim()));
            let mut asc = s.ascension;
            egui::ComboBox::from_id_salt("edit-asc").selected_text(asc.to_string()).show_ui(ui, |ui| {
                for n in 0..=v.catalog.rules.max_ascension {
                    ui.selectable_value(&mut asc, n, n.to_string());
                }
            });
            if asc != s.ascension
                && let Some(run) = e.draft.with_ascension(v.catalog, asc)
            {
                e.draft = run;
            }
        });
        ui.label(caption("Starting HP, potion slots and the ascension curse follow the new level."));

        // Result.
        ui.add_space(crate::style::space::s());
        ui.label(eyebrow("Result"));
        let mut kind = match &s.outcome {
            Some(Outcome::Victory) => 0,
            Some(Outcome::Died { .. }) => 1,
            _ => 2,
        };
        let before = kind;
        segmented(ui, &mut kind, &[(0, "Victory"), (1, "Died"), (2, "Abandoned")]);
        let mut killer = match &s.outcome {
            Some(Outcome::Died { encounter }) => encounter.clone(),
            _ => None,
        };
        let old_killer = killer.clone();
        if kind == 1 {
            let fights: Vec<spire_core::EncounterId> =
                e.draft.floors().iter().rev().filter_map(|(_, _, n)| n.encounter.clone()).collect();
            let label = killer.as_ref().map_or("Unknown".to_string(), |k| v.catalog.encounter_name(k));
            egui::ComboBox::from_id_salt("edit-killer")
                .selected_text(format!("Died to {label}"))
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut killer, None, "Unknown");
                    for f in fights {
                        let name = v.catalog.encounter_name(&f);
                        ui.selectable_value(&mut killer, Some(f), name);
                    }
                });
        }
        if kind != before || killer != old_killer {
            let outcome = match kind {
                0 => Outcome::Victory,
                1 => Outcome::Died { encounter: killer },
                _ => Outcome::Abandoned,
            };
            e.draft = e.draft.with_outcome(Some(outcome));
        }

        // Notes.
        ui.add_space(crate::style::space::s());
        ui.label(eyebrow("Notes"));
        ui.add(
            egui::TextEdit::multiline(&mut e.notes)
                .hint_text("Anything worth remembering about this run")
                .desired_rows(2)
                .desired_width(f32::INFINITY),
        );

        // Floors.
        ui.add_space(crate::style::space::s());
        ui.label(eyebrow("Floors"));
        let floors: Vec<(usize, u32, spire_core::run::NodeKind, Option<spire_core::EncounterId>, String)> = e
            .draft
            .floors()
            .into_iter()
            .map(|(i, f, n)| {
                let what = n.event.as_ref().map(|ev| v.catalog.event_name(ev)).unwrap_or_default();
                (i, f, n.kind, n.encounter.clone(), what)
            })
            .collect();
        let mut remove = None;
        let mut refight = None;
        egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
            if floors.is_empty() {
                ui.label(secondary("No floors were logged."));
            }
            for (i, f, kind, encounter, what) in &floors {
                ui.horizontal(|ui| {
                    ui.label(secondary(format!("{f:>2}  {}", kind.label())));
                    match kind.room() {
                        Some(room) => {
                            let mut pick = encounter.clone();
                            let label = pick.as_ref().map_or("Unknown fight".into(), |x| v.catalog.encounter_name(x));
                            egui::ComboBox::from_id_salt(("edit-floor", *i)).selected_text(label).width(220.0).show_ui(
                                ui,
                                |ui| {
                                    let mut options: Vec<_> =
                                        v.catalog.encounters.values().filter(|x| x.room == room).collect();
                                    options.sort_by(|a, b| a.name.cmp(&b.name));
                                    for x in options {
                                        ui.selectable_value(&mut pick, Some(x.id.clone()), &x.name);
                                    }
                                },
                            );
                            if pick != *encounter {
                                refight = Some((*i, pick));
                            }
                        }
                        None => {
                            ui.label(RichText::new(what).small().color(text()));
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let x =
                            egui::Button::new(RichText::new("Remove").small().color(bad())).frame_when_inactive(false);
                        if ui.add(x).on_hover_text("Remove this floor from the run").clicked() {
                            remove = Some(*i);
                        }
                    });
                });
            }
        });
        if let Some((i, pick)) = refight {
            e.draft = e.draft.with_floor_encounter(i, pick);
        }
        if let Some(i) = remove {
            e.draft = e.draft.without_floor(i);
        }

        ui.add_space(crate::style::space::m());
        let notes = Some(e.notes.trim().to_string()).filter(|n| !n.is_empty());
        let edited = Run { notes: notes.clone(), ..e.draft.clone() };
        let changed = edited != e.original;
        if e.confirm_delete {
            ui.label(RichText::new("Delete this run from history for good?").color(bad()));
            let (delete, keep) = crate::widgets::dialog_row(
                ui,
                |ui| crate::widgets::destructive_button(ui, "Delete run"),
                |ui| crate::widgets::dialog_button(ui, "Keep it"),
            );
            if delete {
                out.push(Action::DeleteHistory(e.id.clone()));
                close = true;
            } else if keep {
                e.confirm_delete = false;
            }
        } else {
            ui.horizontal(|ui| {
                let (save, cancel) = crate::widgets::dialog_row(
                    ui,
                    |ui| primary_save(ui, changed),
                    |ui| crate::widgets::dialog_button(ui, "Cancel"),
                );
                if save {
                    out.push(Action::SaveHistory { id: e.id.clone(), run: Box::new(edited) });
                    close = true;
                } else if cancel {
                    close = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let b = egui::Button::new(RichText::new("Delete run…").color(bad())).frame_when_inactive(false);
                    if ui.add(b).clicked() {
                        e.confirm_delete = true;
                    }
                });
            });
        }
    });
    if close || modal.should_close() {
        h.editing = None;
    }
}

fn primary_save(ui: &mut Ui, enabled: bool) -> egui::Response {
    let label = RichText::new("Save changes").family(style::strong_family());
    let b = egui::Button::new(label.color(if enabled { on_accent() } else { text_faint() }))
        .fill(if enabled { accent() } else { egui::Color32::TRANSPARENT })
        .min_size(egui::vec2(0.0, 32.0));
    ui.add_enabled(enabled, b)
}

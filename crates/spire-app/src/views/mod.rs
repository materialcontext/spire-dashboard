//! Dashboard views. Each is a function of read-only data that pushes
//! [`Action`]s; UI-only state is passed in separately.
//!
//! Layout: header across the top; the run (deck, relics, potions) on the
//! left; the floor log on the right; analysis in the middle, two columns
//! when there is room.

pub mod act;
pub mod analytics;
pub mod breakdown;
pub mod cards;
pub mod data;
pub mod deck;
pub mod dialogs;
pub mod enemies;
pub mod history;
pub mod log;
pub mod lookout;
pub mod picks;
pub mod relics;
pub mod report;
pub mod start;
pub mod threats;
pub mod top;
pub mod turns;

use eframe::egui::{self, Ui};

use crate::actions::Action;
use crate::app::{UiState, ViewCtx};
use crate::draft::NodeDraft;
use crate::widgets::culled;

const GAP: f32 = 12.0;

/// Top-level navigation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Run,
    Analytics,
    History,
    Cards,
    Relics,
    Enemies,
}

impl Tab {
    pub const ALL: [Tab; 6] = [Tab::Run, Tab::Analytics, Tab::History, Tab::Cards, Tab::Relics, Tab::Enemies];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Run => "Run",
            Tab::Analytics => "Analytics",
            Tab::History => "History",
            Tab::Cards => "Cards",
            Tab::Relics => "Relics",
            Tab::Enemies => "Enemies",
        }
    }
}

/// The whole window. `settings` draws the Settings contents, for the
/// platforms where Settings is a page of the window (Windows' System look).
pub fn root(
    ui: &mut Ui,
    v: &ViewCtx,
    draft: &mut NodeDraft,
    st: &mut UiState,
    out: &mut Vec<Action>,
    settings: &mut dyn FnMut(&mut Ui, &mut UiState, &mut Vec<Action>),
) {
    let side = egui::Frame::new().inner_margin(egui::Margin { left: 16, right: 16, top: 0, bottom: 0 });
    let native = crate::style::current().native;
    // macOS: a sidebar of tabs down the left, under the traffic lights.
    let mac = native == Some(crate::style::Native::Mac);
    if mac {
        sidebar(ui, st);
        // The toolbar sits on the window background too, not the sidebar
        // material.
        content_layer(ui);
    }
    egui::Panel::top("top").frame(side).show(ui, |ui| top::show(ui, v, st, out));
    if !mac {
        content_layer(ui);
    }
    dialogs::show(ui, v, st, out);
    toast(ui, st, out);

    // Windows: Settings is a page in the window.
    if native == Some(crate::style::Native::Windows) && st.show_data {
        let page = egui::Frame::new().inner_margin(egui::Margin::symmetric(24, 16));
        egui::CentralPanel::default().frame(page).show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.set_max_width(720.0);
                ui.label(crate::widgets::title("Settings"));
                ui.add_space(crate::style::space::m());
                settings(ui, st, out);
            });
        });
        return;
    }

    let center = egui::Frame::new().inner_margin(egui::Margin::same(GAP as i8));
    match st.tab {
        Tab::Run => {}
        Tab::Analytics => {
            egui::CentralPanel::default().frame(center).show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| match v.analytics {
                    Some(a) => analytics::show(ui, v, a, st),
                    None => {
                        ui.label("Preparing analytics…");
                    }
                });
            });
            return;
        }
        Tab::History => {
            egui::Panel::left("history-list").frame(side).resizable(true).default_size(300.0).show(ui, |ui| {
                history::list(ui, v, &mut st.history);
            });
            egui::CentralPanel::default().frame(center).show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| history::detail(ui, v, &mut st.history, out));
            });
            return;
        }
        Tab::Cards => {
            egui::Panel::left("card-filters").frame(side).resizable(true).default_size(260.0).show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| cards::filters(ui, v, &mut st.cards));
            });
            egui::Panel::right("card-detail").frame(side).resizable(true).default_size(420.0).show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| cards::detail(ui, v, &mut st.cards, out));
            });
            egui::CentralPanel::default()
                .frame(egui::Frame::new().inner_margin(egui::Margin::same(GAP as i8)))
                .show(ui, |ui| cards::list(ui, v, &mut st.cards));
            return;
        }
        Tab::Relics => {
            egui::Panel::left("relic-filters").frame(side).resizable(true).default_size(260.0).show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| relics::filters(ui, v, &mut st.relics));
            });
            egui::Panel::right("relic-detail").frame(side).resizable(true).default_size(420.0).show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| relics::detail(ui, v, &mut st.relics));
            });
            egui::CentralPanel::default()
                .frame(egui::Frame::new().inner_margin(egui::Margin::same(GAP as i8)))
                .show(ui, |ui| relics::list(ui, v, &mut st.relics));
            return;
        }
        Tab::Enemies => {
            egui::Panel::left("enemy-filters").frame(side).resizable(true).default_size(260.0).show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| enemies::filters(ui, v, &mut st.enemies));
            });
            egui::Panel::right("enemy-detail").frame(side).resizable(true).default_size(460.0).show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| enemies::detail(ui, v, &mut st.enemies));
            });
            egui::CentralPanel::default()
                .frame(egui::Frame::new().inner_margin(egui::Margin::same(GAP as i8)))
                .show(ui, |ui| enemies::list(ui, v, &mut st.enemies));
            return;
        }
    }

    if !v.state.started() {
        egui::CentralPanel::default().frame(side).show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| start::show(ui, v, st, out));
        });
        return;
    }

    egui::Panel::left("run").frame(side).resizable(true).default_size(290.0).min_size(240.0).show(ui, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| deck::show(ui, v, out));
    });
    egui::Panel::right("log").frame(side).resizable(true).default_size(330.0).min_size(280.0).show(ui, |ui| {
        // Until the act and its boss are set, the log is locked and dimmed so
        // nothing gets logged against the wrong act.
        let locked = log_locked(v) || v.following;
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_enabled_ui(!locked, |ui| log::show(ui, v, draft, out));
        });
        if v.following {
            dim(
                ui,
                "Following the game",
                "Floors log themselves as you play. Turn off Follow the game in Settings to log by hand.",
            );
        } else if locked {
            let why = if v.state.act.is_none() { "Choose the act first" } else { "Choose the boss first" };
            dim(ui, why, "The floor log unlocks once it's set.");
        }
    });
    egui::CentralPanel::default().frame(center).show(ui, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            follow_conflict(ui, st, out);
            if v.state.act.is_none() {
                start::enter_act(ui, v, out);
                return;
            }
            let g = v.generation;
            // On the boss floor the boss is the next fight: its card takes the
            // width, and the Next fight panel steps aside.
            let boss_floor = spire_core::analysis::floors::expected_room(v.state, v.catalog)
                == Some(spire_core::catalog::NodeKind::Boss);
            if boss_floor {
                culled(ui, ("act-wide", g, v.combat.map(|c| c.generation)), |ui| act::show(ui, v, out));
                ui.add_space(GAP);
                // Next fight steps aside; the relic and card suggestions stay.
                if ui.available_width() > 680.0 {
                    ui.columns(2, |cols| {
                        culled(&mut cols[0], ("breakdown", g, v.combat.map(|c| c.generation)), |ui| {
                            breakdown::show(ui, v)
                        });
                        if lookout::show(&mut cols[1], v, out) {
                            cols[1].add_space(GAP);
                        }
                        picks::show(&mut cols[1], v, out);
                    });
                } else {
                    breakdown::show(ui, v);
                    ui.add_space(GAP);
                    if lookout::show(ui, v, out) {
                        ui.add_space(GAP);
                    }
                    picks::show(ui, v, out);
                }
            } else if ui.available_width() > 680.0 {
                ui.columns(2, |cols| {
                    culled(&mut cols[0], ("act", g, v.combat.map(|c| c.generation)), |ui| act::show(ui, v, out));
                    cols[0].add_space(GAP);
                    culled(&mut cols[0], ("breakdown", g, v.combat.map(|c| c.generation)), |ui| breakdown::show(ui, v));
                    if let Some(e) = v.current_event {
                        lookout::current_event(&mut cols[1], v, e, out);
                        cols[1].add_space(GAP);
                    }
                    if let Some(t) = v.current_fight {
                        threats::current(&mut cols[1], v, t, out);
                        cols[1].add_space(GAP);
                    }
                    let playing = v.current_fight.map(|t| &t.encounter);
                    culled(&mut cols[1], ("threats", g, st.threat_tab, playing), |ui| threats::show(ui, v, st, out));
                    cols[1].add_space(GAP);
                    if lookout::show(&mut cols[1], v, out) {
                        cols[1].add_space(GAP);
                    }
                    picks::show(&mut cols[1], v, out);
                });
            } else {
                act::show(ui, v, out);
                ui.add_space(GAP);
                breakdown::show(ui, v);
                ui.add_space(GAP);
                if let Some(e) = v.current_event {
                    lookout::current_event(ui, v, e, out);
                    ui.add_space(GAP);
                }
                if let Some(t) = v.current_fight {
                    threats::current(ui, v, t, out);
                    ui.add_space(GAP);
                }
                threats::show(ui, v, st, out);
                ui.add_space(GAP);
                if lookout::show(ui, v, out) {
                    ui.add_space(GAP);
                }
                picks::show(ui, v, out);
            }
        });
    });
}

/// The look's content layer under the panels: Fluent's layer over Mica
/// (translucent, so Mica shows through it), or macOS's window background
/// beside the sidebar. Nothing for looks without one.
fn content_layer(ui: &Ui) {
    let layer = crate::style::current().layer;
    // Opaque and the same as the cleared background: nothing to draw.
    if layer.a() == 0 || (layer == crate::style::color::bg() && !crate::style::material()) {
        return;
    }
    let rect = ui.available_rect_before_wrap();
    ui.painter().rect_filled(rect, 0.0, layer);
}

/// macOS's source list: the six tabs down the left, rows 28 tall with a
/// rounded selection, below the title bar (where the traffic lights sit).
fn sidebar(ui: &mut Ui, st: &mut UiState) {
    use crate::style::color::*;
    let look = crate::style::current();
    // A sidebar tint: a touch darker than the window in light mode, a touch
    // lighter in dark (the solid stand-in for the sidebar material).
    let fill =
        if look.light { crate::style::color::bg().gamma_multiply(0.96) } else { egui::Color32::from_white_alpha(8) };
    let frame = egui::Frame::new().fill(fill).inner_margin(egui::Margin { left: 10, right: 10, top: 0, bottom: 10 });
    egui::Panel::left("sidebar").frame(frame).exact_size(200.0).resizable(false).show(ui, |ui| {
        // The title bar strip: drag to move, double-click to zoom.
        let (strip, drag) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 52.0), egui::Sense::click_and_drag());
        let _ = strip;
        if drag.double_clicked() {
            let max = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
        } else if drag.drag_started_by(egui::PointerButton::Primary) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        ui.spacing_mut().item_spacing.y = 2.0;
        for (n, tab) in Tab::ALL.into_iter().enumerate() {
            let selected = st.tab == tab;
            let (rect, r) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 28.0), egui::Sense::click());
            r.widget_info(|| egui::WidgetInfo::selected(egui::Role::Tab, true, selected, tab.label()));
            if selected {
                ui.painter().rect_filled(rect, crate::style::radius(), surface_hi());
            }
            let font = egui::TextStyle::Body.resolve(ui.style());
            ui.painter().text(
                egui::pos2(rect.min.x + 10.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                tab.label(),
                font,
                text(),
            );
            let r = r.on_hover_text(format!("{} ({})", tab.label(), crate::widgets::shortcut(&(n + 1).to_string())));
            if r.clicked() {
                st.tab = tab;
            }
        }
    });
}

/// The game is on a different run than the one being logged by hand.
fn follow_conflict(ui: &mut Ui, st: &UiState, out: &mut Vec<Action>) {
    let Some(game) = &st.follow.conflict else { return };
    let note = format!(
        "The game is on another run ({} floors in). Follow it, and this run is filed in history as Abandoned.",
        game.floors
    );
    crate::widgets::callout(ui, "The game is on another run", Some(&note), |ui| {
        ui.horizontal(|ui| {
            if crate::widgets::choice_button(ui, "Follow the game's run").clicked() {
                out.push(Action::FollowGameRun);
            }
            if ui.button("Stop following").clicked() {
                out.push(Action::SetFollowGame(false));
            }
        });
    });
    ui.add_space(GAP);
}

/// Whether the floor log waits on the act or its boss being chosen.
pub fn log_locked(v: &ViewCtx) -> bool {
    log_locked_state(v.state)
}

pub fn log_locked_state(s: &spire_core::run::RunState) -> bool {
    s.started() && (s.act.is_none() || s.boss.is_none())
}

/// Dims a locked panel and says why. Painted over the content, so the
/// locked controls stay visible but plainly out of play.
fn dim(ui: &mut Ui, why: &str, detail: &str) {
    use crate::style::color::*;
    let rect = ui.max_rect().expand2(egui::vec2(14.0, 0.0));
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, bg().gamma_multiply(0.82));
    let title = egui::TextStyle::Heading.resolve(ui.style());
    let small = egui::TextStyle::Body.resolve(ui.style());
    let top = rect.top() + 90.0;
    painter.text(egui::pos2(rect.center().x, top), egui::Align2::CENTER_CENTER, why, title, accent());
    // The explanation wraps to the panel, centred under the title.
    let mut job = egui::text::LayoutJob::simple(detail.to_string(), small, text_dim(), rect.width() - 48.0);
    job.halign = egui::Align::Center;
    let galley = painter.layout_job(job);
    painter.galley(egui::pos2(rect.center().x, top + 18.0), galley, text_dim());
}

/// The commit notice, bottom centre, with Undo.
fn toast(ui: &mut Ui, st: &UiState, out: &mut Vec<Action>) {
    use crate::style::color::*;
    let Some(t) = &st.toast else { return };
    // Fades in over 83 ms when motion is on; a repaint is asked for only
    // while it's fading.
    let opacity = if crate::style::motion() {
        let now = ui.ctx().input(|i| i.time);
        let o = ((now - t.since) as f32 / crate::style::FADE).clamp(0.0, 1.0);
        if o < 1.0 {
            ui.ctx().request_repaint();
        }
        o
    } else {
        1.0
    };
    egui::Area::new(egui::Id::unique("toast"))
        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -24.0))
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            ui.multiply_opacity(opacity);
            egui::Frame::new()
                .fill(surface_hi())
                .stroke(egui::Stroke::new(1.0, border_hi()))
                .inner_margin(egui::Margin::symmetric(16, 8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&t.text).color(text()));
                        ui.add_space(crate::style::space::m());
                        if ui.button(egui::RichText::new("Undo").color(accent())).clicked() {
                            out.push(Action::Undo);
                            out.push(Action::DismissToast);
                        }
                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new("x").color(text_dim()))
                                    .frame(false)
                                    .min_size(egui::vec2(24.0, 24.0)),
                            )
                            .clicked()
                        {
                            out.push(Action::DismissToast);
                        }
                    });
                });
        });
}

//! Analytics: the run floor by floor, and what the deck does turn by turn.
//!
//! The chart functions take plain analysis results, so the History tab draws
//! archived runs with the same code.

use eframe::egui::{self, RichText, Ui, vec2};
use egui_plot::{Bar, BarChart, FilledArea, HoverPosition, Line, LineStyle, MarkerShape, Plot, Points, VLine};
use spire_core::analysis::sim::DeckSim;
use spire_core::analysis::timeline::Timeline;
use spire_core::catalog::{CardType, Catalog};
use spire_core::playbook::{Playbook, Role};

use crate::app::{Analytics, UiState, ViewCtx};
use crate::style::{self, color::*};
use crate::widgets::{Key, keys, panel, sparkline, stat};

const GAP: f32 = 12.0;
/// Draw-odds rows shown before "Show all".
const ODDS_SHOWN: usize = 10;

pub fn show(ui: &mut Ui, v: &ViewCtx, a: &Analytics, st: &mut UiState) {
    if !v.state.started() {
        ui.label(RichText::new("Start a run to see its analytics.").color(text_dim()));
        return;
    }
    let wide = ui.available_width() > 900.0;
    let run = |ui: &mut Ui| {
        run_section(ui, "live", v.catalog, v.playbook, &a.timeline, Some(v.state.act_index));
    };
    let deck = |ui: &mut Ui, st: &mut UiState| deck_section(ui, "live", &a.sim, &mut st.show_all_odds);
    if wide {
        ui.columns(2, |cols| {
            run(&mut cols[0]);
            deck(&mut cols[1], st);
        });
    } else {
        run(ui);
        ui.add_space(GAP);
        deck(ui, st);
    }
}

// ---------------------------------------------------------------- run

/// Vitals, fights, role growth, dead cards and elites for one run.
/// `act` picks which act's role targets to compare against.
pub fn run_section(ui: &mut Ui, salt: &str, catalog: &Catalog, playbook: &Playbook, t: &Timeline, act: Option<u8>) {
    if t.points.is_empty() {
        panel(ui, "This run", None, |ui| {
            ui.label(RichText::new("Log a floor to start the charts.").color(text_dim()));
        });
        return;
    }
    panel(ui, "HP by floor", None, |ui| vitals(ui, salt, t));
    ui.add_space(GAP);
    panel(ui, "Gold by floor", None, |ui| gold(ui, salt, t));
    ui.add_space(GAP);
    panel(ui, "Damage taken per fight", None, |ui| fights(ui, salt, catalog, t));
    ui.add_space(GAP);
    let act = act.unwrap_or_else(|| t.points.last().map_or(0, |p| p.act_index));
    let note = format!("Act {} targets", act + 1);
    panel(ui, "Deck roles by floor", Some(&note), |ui| roles(ui, playbook, t, act));
    ui.add_space(GAP);
    panel(ui, "Dead cards and elites", None, |ui| dead_and_elites(ui, salt, t));
}

fn plot(id: String, height: f32) -> Plot<'static> {
    Plot::new(id)
        .height(height)
        .allow_drag(false)
        .allow_zoom(false)
        .allow_scroll(false)
        .allow_boxed_zoom(false)
        .allow_double_click_reset(false)
        .show_background(false)
        .grid_color(border())
        .y_axis_min_width(28.0)
        .set_margin_fraction(vec2(0.02, 0.12))
}

/// Act changes as faint dashed lines, so dips line up with where they happened.
fn act_lines(t: &Timeline) -> Vec<VLine> {
    t.points
        .windows(2)
        .filter(|w| w[0].act_index != w[1].act_index)
        .map(|w| VLine::new("", w[1].floor as f64 - 0.5).color(border_hi()).style(LineStyle::dashed_dense()))
        .collect()
}

fn floor_label(name: &str, unit: &str) -> impl Fn(&HoverPosition<'_>) -> Option<String> + use<> {
    let (name, unit) = (name.to_string(), unit.to_string());
    move |pos| match pos {
        HoverPosition::NearDataPoint { plot_name, position, .. } => {
            let what = if plot_name.is_empty() { name.as_str() } else { plot_name };
            Some(format!("Floor {:.0}\n{what}: {:.0}{unit}", position.x, position.y))
        }
        HoverPosition::Elsewhere { .. } => None,
    }
}

fn vitals(ui: &mut Ui, salt: &str, t: &Timeline) {
    let hp: Vec<[f64; 2]> = t.points.iter().map(|p| [p.floor as f64, p.hp as f64]).collect();
    let max: Vec<[f64; 2]> = t.points.iter().map(|p| [p.floor as f64, p.max_hp as f64]).collect();
    keys(
        ui,
        &[
            Key { color: style::hp_color(1.0), name: "HP", dashed: false },
            Key { color: text_dim(), name: "Max HP", dashed: true },
        ],
    );
    plot(format!("{salt}-hp"), 160.0).include_y(0.0).label_formatter(floor_label("HP", "")).show(ui, |p| {
        for l in act_lines(t) {
            p.vline(l);
        }
        p.line(Line::new("Max HP", max).color(text_dim()).style(LineStyle::dashed_loose()).width(1.5));
        p.line(Line::new("HP", hp).color(style::hp_color(1.0)).width(2.0));
    });
}

fn gold(ui: &mut Ui, salt: &str, t: &Timeline) {
    let gold: Vec<[f64; 2]> = t.points.iter().map(|p| [p.floor as f64, p.gold as f64]).collect();
    plot(format!("{salt}-gold"), 130.0).include_y(0.0).label_formatter(floor_label("Gold", "")).show(ui, |p| {
        for l in act_lines(t) {
            p.vline(l);
        }
        p.line(Line::new("", gold).color(style::card_type_color(CardType::Power)).width(2.0));
    });
}

fn fights(ui: &mut Ui, salt: &str, catalog: &Catalog, t: &Timeline) {
    let fights: Vec<_> = t.fights().collect();
    if fights.is_empty() {
        ui.label(RichText::new("No fights logged yet.").color(text_dim()));
        return;
    }
    if let Some((yours, theirs)) = t.fight_damage {
        let diff = yours - theirs;
        let (word, color) = if diff <= 0.0 { ("less", good()) } else { ("more", warn()) };
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(format!("You lose {yours:.1} HP a fight;")).color(text()));
            ui.label(RichText::new(format!("{:.1} {word} than other players", diff.abs())).color(color));
            ui.label(RichText::new(format!("({theirs:.1}) in the same fights.")).color(text_dim()));
        });
    }
    let bars: Vec<Bar> = fights
        .iter()
        .map(|p| {
            let name = p.encounter.as_ref().map_or_else(|| p.kind.label().to_string(), |e| catalog.encounter_name(e));
            Bar::new(p.floor as f64, p.hp_lost.max(0) as f64).name(name).width(0.7)
        })
        .collect();
    let community: Vec<[f64; 2]> =
        fights.iter().filter_map(|p| Some([p.floor as f64, p.community_lost? as f64])).collect();
    let bar_color = style::role_color(Role::FrontloadDamage);
    keys(
        ui,
        &[
            Key { color: bar_color, name: "HP you lost", dashed: false },
            Key { color: text(), name: "Players' average (diamond)", dashed: false },
        ],
    );
    plot(format!("{salt}-fights"), 150.0)
        .include_y(0.0)
        .label_formatter(|pos| match pos {
            HoverPosition::NearDataPoint { plot_name, position, .. } => {
                Some(format!("Floor {:.0}\n{plot_name}: {:.1} HP", position.x, position.y))
            }
            HoverPosition::Elsewhere { .. } => None,
        })
        .show(ui, |p| {
            for l in act_lines(t) {
                p.vline(l);
            }
            p.bar_chart(BarChart::new("You lost", bars).color(bar_color));
            if !community.is_empty() {
                p.points(
                    Points::new("Players' average", community)
                        .shape(MarkerShape::Diamond)
                        .radius(4.5)
                        .filled(true)
                        .color(text()),
                );
            }
        });
}

/// One row per role: a sparkline of cards added over the run, and now vs target.
fn roles(ui: &mut Ui, playbook: &Playbook, t: &Timeline, act: u8) {
    let targets = playbook.act(act).map(|a| a.targets.as_slice()).unwrap_or_default();
    let last = t.points.last();
    egui::Grid::new("role-growth").num_columns(3).spacing([10.0, 6.0]).show(ui, |ui| {
        for role in Role::ALL {
            let series: Vec<f32> = t.points.iter().map(|p| p.role(role) as f32).collect();
            let target = targets.iter().find(|x| x.role == role).map(|x| x.min);
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(vec2(8.0, 8.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 0.0, style::role_color(role));
                ui.label(RichText::new(role.label()).color(text_dim()));
            });
            sparkline(ui, &series, target.map(|t| t as f32), style::role_color(role), vec2(140.0, 22.0));
            let now = last.map_or(0, |p| p.role(role));
            let text = match target {
                Some(t) => format!("{now} of {t}"),
                None => now.to_string(),
            };
            let color = match target {
                Some(t) if now < t => warn(),
                Some(_) => good(),
                None => text_dim(),
            };
            ui.label(RichText::new(text).color(color));
            ui.end_row();
        }
    });
    ui.label(
        RichText::new("Cards added for each role (starter Strikes and Defends not counted). Dashed: the act target.")
            .small()
            .color(text_faint()),
    );
}

fn dead_and_elites(ui: &mut Ui, salt: &str, t: &Timeline) {
    let share: Vec<[f64; 2]> = t.points.iter().map(|p| [p.floor as f64, p.dead_share() as f64 * 100.0]).collect();
    let last = t.points.last().map_or(0.0, |p| p.dead_share() * 100.0);
    ui.horizontal(|ui| {
        stat(ui, "Dead cards now", format!("{last:.0}%"), if last >= 15.0 { warn() } else { text() });
        ui.add_space(crate::style::space::xl());
        for (i, n) in t.elites_by_act.iter().enumerate() {
            stat(ui, &format!("Act {} elites", i + 1), n.to_string(), text());
            ui.add_space(crate::style::space::l());
        }
    });
    plot(format!("{salt}-dead"), 90.0)
        .include_y(0.0)
        .include_y(20.0)
        .y_axis_formatter(|m, _| format!("{:.0}%", m.value))
        .label_formatter(floor_label("Dead cards", "%"))
        .show(ui, |p| {
            for l in act_lines(t) {
                p.vline(l);
            }
            p.line(Line::new("", share).color(style::card_type_color(CardType::Curse)).width(2.0));
        });
    ui.label(RichText::new("Share of the deck that is curses, statuses or unplayable.").small().color(text_faint()));
}

// ---------------------------------------------------------------- deck

/// Simulated turns for one deck: output per turn, cycling, draw odds, energy fit.
pub fn deck_section(ui: &mut Ui, salt: &str, sim: &DeckSim, show_all: &mut bool) {
    if sim.turns.is_empty() {
        panel(ui, "Deck", None, |ui| {
            ui.label(RichText::new("No deck to simulate.").color(text_dim()));
        });
        return;
    }
    let note = format!("{} simulated fights", sim.trials);
    panel(ui, "Damage and block per turn", Some(&note), |ui| {
        per_turn(ui, salt, sim);
        relics_line(ui, sim);
    });
    ui.add_space(GAP);
    panel(ui, "Draw and energy", None, |ui| draw_and_energy(ui, sim));
    ui.add_space(GAP);
    panel(ui, "Odds of drawing each card", Some("by end of turn"), |ui| odds(ui, sim, show_all));
}

fn per_turn(ui: &mut Ui, salt: &str, sim: &DeckSim) {
    let xs: Vec<f64> = (1..=sim.turns.len()).map(|t| t as f64).collect();
    let series = |f: fn(&spire_core::analysis::sim::TurnOutput) -> spire_core::analysis::sim::Spread| {
        let s: Vec<_> = sim.turns.iter().map(f).collect();
        (
            s.iter().zip(&xs).map(|(s, x)| [*x, s.mean as f64]).collect::<Vec<_>>(),
            s.iter().map(|s| s.low as f64).collect::<Vec<_>>(),
            s.iter().map(|s| s.high as f64).collect::<Vec<_>>(),
        )
    };
    let (dmg, dmg_lo, dmg_hi) = series(|t| t.damage);
    let (blk, blk_lo, blk_hi) = series(|t| t.block);
    let first = sim.turns[0];
    let last = sim.turns[sim.turns.len() - 1];
    ui.horizontal(|ui| {
        stat(ui, "Damage, turn 1", format!("{:.0}", first.damage.mean), text());
        ui.add_space(crate::style::space::l());
        stat(ui, &format!("Damage, turn {}", sim.turns.len()), format!("{:.0}", last.damage.mean), text());
        ui.add_space(crate::style::space::l());
        stat(ui, "Block, turn 1", format!("{:.0}", first.block.mean), text());
        ui.add_space(crate::style::space::l());
        stat(ui, &format!("Block, turn {}", sim.turns.len()), format!("{:.0}", last.block.mean), text());
    });
    let (red, blue) = (style::role_color(Role::FrontloadDamage), style::role_color(Role::FrontloadDefense));
    keys(
        ui,
        &[
            Key { color: red, name: "Damage, playing to attack", dashed: false },
            Key { color: blue, name: "Block, playing to defend", dashed: false },
        ],
    );
    plot(format!("{salt}-turns"), 170.0)
        .include_y(0.0)
        .x_axis_formatter(|m, _| if m.value.fract() == 0.0 { format!("T{:.0}", m.value) } else { String::new() })
        .label_formatter(|pos| match pos {
            HoverPosition::NearDataPoint { plot_name, position, .. } if !plot_name.is_empty() => {
                Some(format!("Turn {:.0}\n{plot_name}: {:.1}", position.x, position.y))
            }
            _ => None,
        })
        .show(ui, |p| {
            p.add(FilledArea::new("", &xs, &dmg_lo, &dmg_hi).fill_color(red.gamma_multiply(0.18)).allow_hover(false));
            p.add(FilledArea::new("", &xs, &blk_lo, &blk_hi).fill_color(blue.gamma_multiply(0.18)).allow_hover(false));
            p.line(Line::new("Damage", dmg).color(red).width(2.0));
            p.line(Line::new("Block", blk).color(blue).width(2.0));
        });
    ui.label(
        RichText::new(format!(
            "Shaded: the middle half of fights. {} cards and {} energy a turn; potions, enchantments and enemy \
             actions aren't simulated.",
            sim.hand_size, sim.energy
        ))
        .small()
        .color(text_faint()),
    );
}

/// Which of the run's relics the simulation includes, with the rest on hover.
fn relics_line(ui: &mut Ui, sim: &DeckSim) {
    let (yes, no) = (&sim.relics_simulated, &sim.relics_not_simulated);
    if yes.is_empty() && no.is_empty() {
        return;
    }
    let text = format!("Relics simulated: {} of {}", yes.len(), yes.len() + no.len());
    ui.label(RichText::new(text).small().color(text_dim())).on_hover_ui(|ui| {
        if !yes.is_empty() {
            ui.label(RichText::new(format!("Simulated: {}", yes.join(", "))).small());
        }
        if !no.is_empty() {
            ui.label(RichText::new(format!("Not simulated: {}", no.join(", "))).small().color(text_dim()));
        }
        ui.label(RichText::new("See each relic in the Relics tab for what is read.").small().color(text_faint()));
    });
}

fn draw_and_energy(ui: &mut Ui, sim: &DeckSim) {
    ui.horizontal_wrapped(|ui| {
        stat(ui, "Turnover", format!("{:.1} turns", sim.turnover), text())
            .on_hover_text("Turns to draw every card once. Lower means your best cards come back sooner.");
        ui.add_space(crate::style::space::l());
        let dead = sim.dead_draws * 100.0;
        stat(ui, "Dead draws", format!("{dead:.0}%"), if dead >= 10.0 { warn() } else { text() })
            .on_hover_text("Share of drawn cards that can't be played: curses, statuses, unplayables.");
        ui.add_space(crate::style::space::l());
        stat(ui, "Unspent energy", format!("{:.1} a turn", sim.unspent_energy), text())
            .on_hover_text("Energy left over with nothing worth playing. High: room for costlier cards.");
        ui.add_space(crate::style::space::l());
        let wanted = sim.stranded_energy;
        stat(ui, "Energy short", format!("{wanted:.1} a turn"), if wanted >= 3.0 { warn() } else { text() })
            .on_hover_text("Cost of playable cards stuck in hand. High: more energy or cheaper cards would help.");
    });
    if !sim.resources.is_empty() {
        ui.add_space(crate::style::space::s());
        ui.horizontal_wrapped(|ui| {
            for (r, n) in &sim.resources {
                stat(ui, r.label(), format!("{n:.1} a turn"), text());
                ui.add_space(crate::style::space::l());
            }
        });
    }
}

fn odds(ui: &mut Ui, sim: &DeckSim, show_all: &mut bool) {
    let rows = if *show_all { sim.odds.len() } else { sim.odds.len().min(ODDS_SHOWN) };
    egui::Grid::new("draw-odds").num_columns(5).striped(false).spacing([14.0, 4.0]).show(ui, |ui| {
        ui.label(RichText::new("Card").small().color(text_faint()));
        ui.label(RichText::new("Copies").small().color(text_faint()));
        for t in 1..=3 {
            ui.label(RichText::new(format!("Turn {t}")).small().color(text_faint()));
        }
        ui.end_row();
        for o in &sim.odds[..rows] {
            ui.label(RichText::new(&o.name).color(if o.card.upgraded { good() } else { text() }));
            ui.label(RichText::new(o.copies.to_string()).color(text_dim()));
            for p in o.by_turn {
                ui.label(RichText::new(format!("{:.0}%", p * 100.0)).color(text_dim()));
            }
            ui.end_row();
        }
    });
    if sim.odds.len() > ODDS_SHOWN {
        let label = if *show_all { "Show fewer".to_string() } else { format!("Show all {}", sim.odds.len()) };
        if ui.add(egui::Button::new(RichText::new(label).small().color(accent())).frame(false)).clicked() {
            *show_all = !*show_all;
        }
    }
}

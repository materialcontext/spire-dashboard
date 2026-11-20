//! Lookout: shown only when a `?` room this act can still give something
//! the deck needs. Two panels, one shape: relics crucial to the deck, and
//! event options doing what it needs (removing a curse, raising max HP).

use eframe::egui::{RichText, Ui};
use spire_core::analysis::{Lookout, LookoutRoom, LookoutSource, Wanted};
use spire_core::catalog::{EffectKind, EventEffect};

use crate::actions::{Action, Link};
use crate::app::ViewCtx;
use crate::style::color::*;
use crate::widgets::{caption, link, panel, secondary};

/// Nothing at all when there's nothing worth fishing for.
pub fn show(ui: &mut Ui, v: &ViewCtx, out: &mut Vec<Action>) -> bool {
    let relics: Vec<&Lookout> = v.dash.lookout.iter().filter(|l| matches!(l.wanted, Wanted::Relics(_))).collect();
    let events: Vec<&Lookout> = v.dash.lookout.iter().filter(|l| matches!(l.wanted, Wanted::Effects(_))).collect();
    if !relics.is_empty() {
        lookout_panel(ui, v, "Fish for relics", &relics, out);
    }
    if !events.is_empty() {
        if !relics.is_empty() {
            ui.add_space(crate::style::space::m());
        }
        lookout_panel(ui, v, "Watch for events", &events, out);
    }
    !(relics.is_empty() && events.is_empty())
}

fn lookout_panel(ui: &mut Ui, v: &ViewCtx, title: &str, entries: &[&Lookout], out: &mut Vec<Action>) {
    panel(ui, title, Some("in ? rooms this act"), |ui| {
        for (i, l) in entries.iter().enumerate() {
            if i > 0 {
                ui.add_space(crate::style::space::s());
            }
            entry(ui, v, l, out);
        }
    });
}

fn relic_name(v: &ViewCtx, id: &spire_core::ids::RelicId) -> String {
    v.catalog.relics.get(id).map_or_else(|| id.to_string(), |r| r.name.clone())
}

/// A relic link, its text on hover.
fn relic_link(ui: &mut Ui, v: &ViewCtx, id: &spire_core::ids::RelicId, text: RichText, out: &mut Vec<Action>) {
    let tip = v.catalog.relics.get(id).map_or("", |r| r.text.as_str());
    if link(ui, text).on_hover_text(tip).clicked() {
        out.push(Action::Open(Link::Relic(id.clone())));
    }
}

fn entry(ui: &mut Ui, v: &ViewCtx, l: &Lookout, out: &mut Vec<Action>) {
    let title = RichText::new(&l.title).family(crate::style::strong_family()).color(warn());
    match l.relics() {
        [only] => relic_link(ui, v, only, title, out),
        _ => {
            ui.label(title);
        }
    }
    ui.label(RichText::new(&l.why).color(text()));
    if l.relics().len() > 1 {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.label(secondary("Would help:"));
            for (i, id) in l.relics().iter().enumerate() {
                let sep = if i + 1 < l.relics().len() { "," } else { "" };
                relic_link(ui, v, id, RichText::new(format!("{}{sep}", relic_name(v, id))).color(text()), out);
            }
        });
    }
    for s in l.sources.iter().take(SOURCES) {
        source(ui, v, l, s);
    }
    // The rest, likeliest first, by name on hover.
    let rest: Vec<String> = l.sources.iter().skip(SOURCES).map(|s| room_name(&s.room).to_string()).collect();
    if !rest.is_empty() {
        let n = rest.len();
        ui.label(caption(format!("and {n} less likely {}", if n == 1 { "place" } else { "places" })))
            .on_hover_text(rest.join(", "));
    }
}

/// Sources shown per warning; the rest are named on hover.
const SOURCES: usize = 3;

fn room_name(room: &LookoutRoom) -> &str {
    match room {
        LookoutRoom::Event { name, .. } => name,
        LookoutRoom::Treasure => "Treasure room",
    }
}

/// "1 run in N" from a share of runs.
fn one_in(share: f32) -> String {
    format!("1 run in {:.0}", (1.0 / share).round().max(1.0))
}

/// "Trash Heap · 1 of 5 at random · 1 run in 9 meets it", or "Treasure
/// room · …", with the option, what it costs and what it requires
/// underneath.
fn source(ui: &mut Ui, v: &ViewCtx, l: &Lookout, s: &LookoutSource) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let (name, met) = match &s.room {
            LookoutRoom::Event { name, .. } => (name.as_str(), "meets it"),
            LookoutRoom::Treasure => ("Treasure room", "finds one behind a ?"),
        };
        ui.label(RichText::new(name).strong().color(text()));
        // Which relics, when the warning names several and this gives some.
        if l.relics().len() > 1 {
            let names: Vec<String> = s.gives.iter().map(|id| relic_name(v, id)).collect();
            ui.label(secondary(format!("· {}", names.join(" or "))));
        }
        match (&s.room, &l.wanted) {
            (LookoutRoom::Event { one_of, .. }, Wanted::Relics(_)) if *one_of > 1 => {
                ui.label(secondary(format!("· 1 of {one_of} at random")));
            }
            // An option that doesn't always deliver.
            (LookoutRoom::Event { .. }, Wanted::Effects(_)) if s.odds < 0.95 => {
                ui.label(secondary(format!("· does it {:.0}% of the time", s.odds * 100.0)))
                    .on_hover_text("Share of community runs taking this option in which it did.");
            }
            (LookoutRoom::Treasure, _) => {
                ui.label(secondary(format!(
                    "· its relic is one of these about 1 time in {:.0}",
                    (1.0 / s.odds).round()
                )))
                .on_hover_text("By this act's chest rarities, at random among the relics of that rarity you can get.");
            }
            _ => {}
        }
        let odds = match s.met {
            Some(m) if m > 0.0 => format!("· {} {met}", one_in(m)),
            _ => "· how often it comes up isn't known".into(),
        };
        ui.label(caption(odds)).on_hover_text("From community runs that played this act.");
    });
    if let LookoutRoom::Event { option, requires, .. } = &s.room {
        if let Some((title, text)) = option.as_ref().filter(|(_, t)| !t.is_empty()) {
            ui.label(caption(format!("{title}: {text}")));
        }
        costs(ui, v, &s.costs);
        if !requires.is_empty() {
            ui.label(caption(requires.join(" · ")));
        }
    }
}

/// "Costs about 8 HP, −13 max HP", in red when the HP it costs would leave
/// the run at or below zero.
fn costs(ui: &mut Ui, v: &ViewCtx, costs: &[EventEffect]) {
    if costs.is_empty() {
        return;
    }
    let words: Vec<String> = costs
        .iter()
        .map(|c| {
            let what = c.kind.describe(c.amount);
            if c.rate < 0.95 { format!("{what} ({:.0}% of the time)", c.rate * 100.0) } else { what }
        })
        .collect();
    let deadly = costs.iter().any(|c| c.kind == EffectKind::Damage && c.amount.round() as i32 >= v.state.hp);
    let text = RichText::new(format!("Costs {}", words.join(", "))).small();
    ui.label(if deadly { text.color(bad()) } else { text.color(text_dim()) })
        .on_hover_text("What taking it cost community runs, on average.");
}

/// The event the game is in, above Next fight (following only): its
/// options, with any that give a relic the lookout flags as crucial or do
/// what an event warning is after.
pub fn current_event(ui: &mut Ui, v: &ViewCtx, e: &spire_core::catalog::Event, out: &mut Vec<Action>) {
    panel(ui, "Current event", None, |ui| {
        ui.label(RichText::new(&e.name).family(crate::style::strong_family()).color(text()));
        for o in &e.options {
            ui.add_space(crate::style::space::xs());
            ui.label(RichText::new(&o.title).strong().color(text()));
            if !o.text.is_empty() {
                ui.label(secondary(&o.text));
            }
            // A relic this option gives that the lookout wants.
            let crucial = e
                .relics
                .iter()
                .filter(|r| r.option.as_deref() == Some(o.title.as_str()))
                .filter(|r| v.dash.lookout.iter().any(|l| l.relics().contains(&r.relic)));
            for r in crucial {
                let text =
                    RichText::new(format!("{}: crucial for this deck", relic_name(v, &r.relic))).small().color(warn());
                relic_link(ui, v, &r.relic, text, out);
            }
            // An event warning this option answers.
            let answers = v.dash.lookout.iter().filter(|l| {
                l.sources.iter().any(|s| match &s.room {
                    LookoutRoom::Event { event, option: Some((title, _)), .. } => {
                        matches!(l.wanted, Wanted::Effects(_)) && *event == e.id && *title == o.title
                    }
                    _ => false,
                })
            });
            for l in answers {
                ui.label(RichText::new(format!("{}: what this deck needs", l.title)).small().color(warn()))
                    .on_hover_text(&l.why);
            }
            let paid: Vec<EventEffect> = o.effects.iter().filter(|c| c.kind.is_cost()).copied().collect();
            costs(ui, v, &paid);
        }
    });
}

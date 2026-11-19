//! Relic lookout: shown only when a `?` room this act can still give a
//! relic that's crucial to the deck.

use eframe::egui::{RichText, Ui};
use spire_core::analysis::{Lookout, RelicRoom, RelicSource};

use crate::actions::{Action, Link};
use crate::app::ViewCtx;
use crate::style::color::*;
use crate::widgets::{caption, link, panel, secondary};

/// Nothing at all when there's nothing worth fishing for.
pub fn show(ui: &mut Ui, v: &ViewCtx, out: &mut Vec<Action>) -> bool {
    if v.dash.lookout.is_empty() {
        return false;
    }
    panel(ui, "Fish for relics", Some("in ? rooms this act"), |ui| {
        for (i, l) in v.dash.lookout.iter().enumerate() {
            if i > 0 {
                ui.add_space(crate::style::space::s());
            }
            entry(ui, v, l, out);
        }
    });
    true
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
    match l.relics.as_slice() {
        [only] => relic_link(ui, v, only, title, out),
        _ => {
            ui.label(title);
        }
    }
    ui.label(RichText::new(&l.why).color(text()));
    if l.relics.len() > 1 {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.label(secondary("Would help:"));
            for (i, id) in l.relics.iter().enumerate() {
                let sep = if i + 1 < l.relics.len() { "," } else { "" };
                relic_link(ui, v, id, RichText::new(format!("{}{sep}", relic_name(v, id))).color(text()), out);
            }
        });
    }
    for s in &l.sources {
        source(ui, v, l, s);
    }
}

/// "1 run in N" from a share of runs.
fn one_in(share: f32) -> String {
    format!("1 run in {:.0}", (1.0 / share).round().max(1.0))
}

/// "Trash Heap · 1 of 5 at random · 1 run in 9 meets it", or "Treasure
/// room · …", with the option's cost and requirements underneath.
fn source(ui: &mut Ui, v: &ViewCtx, l: &Lookout, s: &RelicSource) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let (name, met) = match &s.room {
            RelicRoom::Event { name, .. } => (name.as_str(), "meets it"),
            RelicRoom::Treasure => ("Treasure room", "finds one behind a ?"),
        };
        ui.label(RichText::new(name).strong().color(text()));
        // Which relics, when the warning names several and this gives some.
        if l.relics.len() > 1 {
            let names: Vec<String> = s.gives.iter().map(|id| relic_name(v, id)).collect();
            ui.label(secondary(format!("· {}", names.join(" or "))));
        }
        match &s.room {
            RelicRoom::Event { one_of, .. } if *one_of > 1 => {
                ui.label(secondary(format!("· 1 of {one_of} at random")));
            }
            RelicRoom::Treasure => {
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
    if let RelicRoom::Event { option, requires, .. } = &s.room {
        if let Some((title, text)) = option.as_ref().filter(|(_, t)| !t.is_empty()) {
            ui.label(caption(format!("{title}: {text}")));
        }
        if !requires.is_empty() {
            ui.label(caption(requires.join(" · ")));
        }
    }
}

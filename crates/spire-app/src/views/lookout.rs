//! Relic lookout: shown only when an event this act can still give a relic
//! that's crucial to the deck.

use eframe::egui::{RichText, Ui};
use spire_core::analysis::{Lookout, RelicSource};

use crate::actions::{Action, Link};
use crate::app::ViewCtx;
use crate::style::color::*;
use crate::widgets::{caption, link, panel, secondary};

/// Nothing at all when there's nothing worth fishing for.
pub fn show(ui: &mut Ui, v: &ViewCtx, out: &mut Vec<Action>) -> bool {
    if v.dash.lookout.is_empty() {
        return false;
    }
    let title = if v.dash.lookout.len() == 1 { "Relic to fish for" } else { "Relics to fish for" };
    panel(ui, title, Some("in ? rooms this act"), |ui| {
        for (i, l) in v.dash.lookout.iter().enumerate() {
            if i > 0 {
                ui.add_space(crate::style::space::s());
            }
            relic(ui, l, out);
        }
    });
    true
}

fn relic(ui: &mut Ui, l: &Lookout, out: &mut Vec<Action>) {
    let name = RichText::new(&l.name).family(crate::style::strong_family()).color(warn());
    if link(ui, name).on_hover_text(&l.text).clicked() {
        out.push(Action::Open(Link::Relic(l.relic.clone())));
    }
    ui.label(RichText::new(&l.why).color(text()));
    for s in &l.sources {
        source(ui, s);
    }
}

/// "Trash Heap · 1 of 5 at random · 1 run in 17 meets it", with the
/// option's cost underneath.
fn source(ui: &mut Ui, s: &RelicSource) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        ui.label(RichText::new(&s.name).strong().color(text()));
        if s.one_of > 1 {
            ui.label(secondary(format!("· 1 of {} at random", s.one_of)));
        }
        let odds = match s.met {
            Some(m) if m > 0.0 => format!("· 1 run in {:.0} meets it", (1.0 / m).round().max(1.0)),
            _ => "· how often it comes up isn't known".into(),
        };
        ui.label(caption(odds)).on_hover_text("From community runs that played this act.");
    });
    if let Some(text) = s.option_text.as_ref().filter(|t| !t.is_empty()) {
        let option = s.option.as_deref().unwrap_or("Option");
        ui.label(caption(format!("{option}: {text}")));
    }
    // The act limits are already applied; the rest (gold, HP) is up to the run.
    let requires: Vec<&str> = s.requires.iter().map(String::as_str).filter(|r| !r.starts_with("Act ")).collect();
    if !requires.is_empty() {
        ui.label(caption(requires.join(" · ")));
    }
}

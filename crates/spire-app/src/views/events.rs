//! `?` event watchlist.

use eframe::egui::{self, RichText, Ui};

use spire_core::analysis::EventWatch;

use crate::app::ViewCtx;
use crate::widgets::{chip, culled, opportunity_color, section};

pub fn show(ui: &mut Ui, v: &ViewCtx) {
    section(ui, "? events to look for", |ui| {
        if !v.dash.needs.is_empty() {
            ui.label(RichText::new("This run wants").strong());
            ui.horizontal_wrapped(|ui| {
                for n in v.dash.needs.iter().filter(|n| n.weight >= 2) {
                    chip(ui, n.opportunity.label(), opportunity_color(ui, n.opportunity)).on_hover_text(&n.reason);
                }
            });
            ui.add_space(4.0);
        }

        let (fresh, seen): (Vec<_>, Vec<_>) = v.dash.events.iter().partition(|e| !e.seen);
        for e in &fresh {
            // The watchlist is long and mostly below the fold: skip layout
            // of entries that are scrolled out of view.
            culled(ui, ("event", v.generation, &e.event), |ui| event_entry(ui, v, e));
        }
        if !seen.is_empty() {
            egui::CollapsingHeader::new(format!("Already seen ({})", seen.len())).show(ui, |ui| {
                for e in seen {
                    ui.label(RichText::new(&e.name).weak());
                }
            });
        }
    });
}

fn event_entry(ui: &mut Ui, v: &ViewCtx, e: &EventWatch) {
    ui.add_space(3.0);
    ui.horizontal(|ui| {
        let name = RichText::new(&e.name).strong();
        // Options are only laid out while hovered.
        ui.label(name).on_hover_ui(|ui| {
            for o in v.catalog.events.get(&e.event).map_or(&[][..], |ev| ev.options.as_slice()) {
                ui.label(format!("{}: {}", o.title, o.text));
            }
        });
        if e.score > 0 {
            ui.label(RichText::new(format!("fit {}", e.score)).small().color(ui.visuals().hyperlink_color));
        }
    });
    ui.horizontal_wrapped(|ui| {
        for o in &e.opportunities {
            let hit = e.matches.iter().find(|n| n.opportunity == *o);
            let color = if hit.is_some() { opportunity_color(ui, *o) } else { ui.visuals().weak_text_color() };
            let r = chip(ui, o.label(), color);
            if let Some(n) = hit {
                r.on_hover_text(&n.reason);
            }
        }
        for r in &e.risks {
            chip(ui, r.label(), opportunity_color(ui, *r));
        }
    });
}

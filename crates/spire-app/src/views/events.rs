//! `?` event watchlist.

use eframe::egui::{self, RichText, Ui};
use spire_core::analysis::EventWatch;

use crate::app::ViewCtx;
use crate::style::color::*;
use crate::widgets::{chip, culled, eyebrow, opportunity_color, panel};

pub fn show(ui: &mut Ui, v: &ViewCtx) {
    let (fresh, seen): (Vec<_>, Vec<_>) = v.dash.events.iter().partition(|e| !e.seen);
    let note = format!("{} possible this act", fresh.len());
    panel(ui, "Event watchlist", Some(&note), |ui| {
        let wants: Vec<_> = v.dash.needs.iter().filter(|n| n.weight >= 2).collect();
        if !wants.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("This run wants").small().color(text_dim()));
                for n in wants {
                    chip(ui, n.opportunity.label(), opportunity_color(n.opportunity)).on_hover_text(&n.reason);
                }
            });
            ui.add_space(4.0);
        }
        for e in &fresh {
            // The watchlist is long and mostly below the fold: skip layout
            // of entries that are scrolled out of view.
            // Keyed by content, not generation: entries that a commit
            // doesn't change keep their cached size.
            culled(ui, ("event", e), |ui| event_entry(ui, v, e));
        }
        if !seen.is_empty() {
            ui.add_space(6.0);
            egui::CollapsingHeader::new(eyebrow(&format!("Already seen · {}", seen.len())))
                .id_salt("seen-events")
                .show(ui, |ui| {
                    for e in seen {
                        ui.label(RichText::new(&e.name).color(text_faint()));
                    }
                });
        }
    });
}

fn event_entry(ui: &mut Ui, v: &ViewCtx, e: &EventWatch) {
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        // Options are only laid out while hovered.
        ui.label(RichText::new(&e.name).family(crate::style::strong_family()).color(text())).on_hover_ui(|ui| {
            for o in v.catalog.events.get(&e.event).map_or(&[][..], |ev| ev.options.as_slice()) {
                ui.label(RichText::new(&o.title).strong());
                ui.label(RichText::new(&o.text).small().color(text_dim()));
            }
        });
        if e.score > 0 {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(format!("fit {}", e.score)).small().strong().color(accent()));
            });
        }
    });
    ui.horizontal_wrapped(|ui| {
        for o in &e.opportunities {
            let hit = e.matches.iter().find(|n| n.opportunity == *o);
            let color = if hit.is_some() { opportunity_color(*o) } else { text_faint() };
            let r = chip(ui, o.label(), color);
            if let Some(n) = hit {
                r.on_hover_text(&n.reason);
            }
        }
        for r in &e.risks {
            chip(ui, r.label(), opportunity_color(*r));
        }
    });
}

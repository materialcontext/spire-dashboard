//! Deck composition: by card type (one stacked bar) and by macro role.

use eframe::egui::{RichText, Sense, Ui, vec2};
use spire_core::catalog::CardType;
use spire_core::playbook::Role;

use crate::app::ViewCtx;
use crate::style;
use crate::widgets::{bar, caption, panel};

pub fn show(ui: &mut Ui, v: &ViewCtx) {
    let d = &v.dash.deck;
    let plan = v.playbook.act(v.state.act_index);
    panel(ui, "Deck composition", Some(&format!("{} cards", d.total)), |ui| {
        // Card types as one proportional bar with a legend underneath.
        let width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(vec2(width, 12.0), Sense::hover());
        let total = d.total.max(1) as f32;
        let mut x = rect.min.x;
        for t in CardType::ALL {
            let n = d.of_type(t);
            if n == 0 {
                continue;
            }
            let w = rect.width() * n as f32 / total;
            let seg =
                eframe::egui::Rect::from_min_size(eframe::egui::pos2(x, rect.min.y), vec2(w - 1.0, rect.height()));
            ui.painter().rect_filled(seg, 0.0, style::card_type_color(t));
            x += w;
        }
        ui.horizontal_wrapped(|ui| {
            for t in CardType::ALL {
                let n = d.of_type(t);
                if n > 0 {
                    ui.label(RichText::new(format!("{} {n}", t.label())).small().color(style::card_type_color(t)));
                }
            }
        });

        ui.add_space(crate::style::space::s());
        ui.label(caption("Roles · bright: cards you added · faded: starter Strikes/Defends · tick: target this act"));
        let target = |r: Role| plan.and_then(|p| p.targets.iter().find(|t| t.role == r)).map(|t| t.min);
        let max_role =
            Role::ALL.iter().map(|r| (d.role(*r) + d.starter_role(*r)).max(target(*r).unwrap_or(0))).max().unwrap_or(1);
        for r in Role::ALL {
            bar(ui, r.label(), d.role(r), d.starter_role(r), max_role, target(r), style::role_color(r));
        }
        // Draw and energy aren't roles: they help every role on bad draws.
        bar(ui, "Draw", d.draw, 0, max_role, None, style::tag_color());
        bar(ui, "Energy gain", d.energy, 0, max_role, None, style::tag_color());

        ui.add_space(crate::style::space::xs());
        ui.label(caption(format!("{} upgraded · {} with no role · {} unknown", d.upgraded, d.no_role, d.unknown)));
    });
}

//! Deck breakdown by card type and by macro role.

use eframe::egui::{RichText, Ui};
use spire_core::catalog::CardType;
use spire_core::playbook::Role;

use crate::app::ViewCtx;
use crate::widgets::{bar, section};

pub fn show(ui: &mut Ui, v: &ViewCtx) {
    let d = &v.dash.deck;
    let plan = v.playbook.act(v.state.act_index);
    section(ui, "Deck breakdown", |ui| {
        ui.label(RichText::new("By type").strong());
        let max_type = d.by_type.values().copied().max().unwrap_or(1);
        for t in CardType::ALL {
            let n = d.of_type(t);
            if n > 0 || matches!(t, CardType::Attack | CardType::Skill | CardType::Power | CardType::Curse) {
                let color = if t == CardType::Curse && n > 0 {
                    ui.visuals().error_fg_color
                } else {
                    ui.visuals().selection.bg_fill
                };
                bar(ui, t.label(), n, 0, max_type, None, color);
            }
        }

        ui.add_space(6.0);
        ui.label(RichText::new("By role").strong());
        ui.label(
            RichText::new("Solid: cards you added. Faded: starter Strikes/Defends. Tick: target for this act.")
                .small()
                .weak(),
        );
        let target = |r: Role| plan.and_then(|p| p.targets.iter().find(|t| t.role == r)).map(|t| t.min);
        let max_role =
            Role::ALL.iter().map(|r| (d.role(*r) + d.starter_role(*r)).max(target(*r).unwrap_or(0))).max().unwrap_or(1);
        for r in Role::ALL {
            bar(ui, r.label(), d.role(r), d.starter_role(r), max_role, target(r), ui.visuals().hyperlink_color);
        }
        bar(
            ui,
            "Area damage",
            d.aoe,
            0,
            max_role,
            plan.map(|p| p.needs_aoe).filter(|n| *n > 0),
            ui.visuals().hyperlink_color,
        );

        ui.add_space(4.0);
        ui.label(
            RichText::new(format!("{} upgraded · {} with no role · {} unknown", d.upgraded, d.no_role, d.unknown))
                .small()
                .weak(),
        );
    });
}

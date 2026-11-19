//! Deck composition: by card type (one stacked bar) and by macro role.

use eframe::egui::{RichText, Sense, Ui, vec2};
use spire_core::analysis::benchmark::Measure;
use spire_core::analysis::tags::Tag;
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
        ui.label(caption("Roles")).on_hover_text(
            "Bright: cards you added. Faded: starter Strikes and Defends. Tick: this act's target. \
             Hover a role for how well the deck does it (simulated).",
        );
        let target = |r: Role| plan.and_then(|p| p.targets.iter().find(|t| t.role == r)).map(|t| t.min);
        let max_role =
            Role::ALL.iter().map(|r| (d.role(*r) + d.starter_role(*r)).max(target(*r).unwrap_or(0))).max().unwrap_or(1);
        let jobs = v.combat.map(|c| &c.jobs);
        for r in Role::ALL {
            let row = bar(ui, r.label(), d.role(r), d.starter_role(r), max_role, target(r), style::role_color(r));
            // Built only while hovered: what the deck does, then how a card
            // counts for the role.
            row.on_hover_ui(|ui| {
                ui.set_max_width(360.0);
                if let Some(j) = jobs {
                    ui.label(RichText::new(j.describe(r)).strong());
                }
                ui.label(v.playbook.role_criteria(r));
            });
        }
        if let Some(j) = jobs {
            ui.label(caption(format!("Bad draws: {:.0}% of usual damage", j.floor * 100.0)))
                .on_hover_text("What the deck deals on 1 hand in 4, against its average.");
        }
        // Setup piling up and resources running short hold every role back.
        for w in v.combat.map(|c| c.balance.as_slice()).unwrap_or_default() {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.label(RichText::new(w.store.label()).small().strong().color(style::color::warn()));
                ui.label(RichText::new(&w.text).small()).on_hover_text(&w.detail);
            });
        }
        // Draw and energy aren't roles: they help every role on bad draws.
        bar(ui, "Draw", d.tag(Tag::Draw), 0, max_role, None, style::tag_color_of(Tag::Draw));
        bar(ui, "Energy gain", d.tag(Tag::Energy), 0, max_role, None, style::tag_color_of(Tag::Energy));
        let counts: Vec<String> = [(d.upgraded, "upgraded"), (d.no_role, "with no role"), (d.unknown, "unknown")]
            .into_iter()
            .filter(|(n, _)| *n > 0)
            .map(|(n, what)| format!("{n} {what}"))
            .collect();
        if !counts.is_empty() {
            ui.label(caption(counts.join(" · ")));
        }
        if let Some(b) = v.combat.and_then(|c| c.bench.as_ref()) {
            benchmark(ui, v, b);
        }
    });
}

/// "Mitigation  5 · cleared decks 6–10", shortfalls first, then the rest
/// in one line.
fn benchmark(ui: &mut Ui, v: &ViewCtx, b: &crate::app::Bench) {
    ui.add_space(crate::style::space::s());
    let every = if b.all_characters { " (all characters)" } else { "" };
    ui.label(
        RichText::new(format!("Vs. {} decks that cleared Act {}{every}", b.decks, v.state.act_index + 1)).strong(),
    )
    .on_hover_text(
        "Community runs' decks at the end of the act, middle half shown. Listed: where your deck is \
         worse than 3 in 4 of them. Every character's decks are used when this one has too few.",
    );
    let short: Vec<_> = b.rows.iter().filter(|r| r.short).collect();
    if short.is_empty() {
        ui.label(RichText::new("No shortfalls").small().color(style::color::good()));
    }
    for r in short {
        ui.horizontal(|ui| {
            let label = ui.add_sized(
                [crate::widgets::BAR_LABEL_WIDTH, 16.0],
                eframe::egui::Label::new(RichText::new(r.measure.label()).small().color(style::color::warn()))
                    .truncate(),
            );
            // Built only while hovered.
            label.on_hover_ui(|ui| {
                ui.label(match r.measure {
                    Measure::Job(role) => {
                        let tags: Vec<&str> = v.playbook.tags_for(role).iter().map(|t| t.label()).collect();
                        format!("Look for: {}", tags.join(", "))
                    }
                    Measure::Tag(t) => t.describe().to_string(),
                    Measure::Starter => "Removing Strikes and Defends helps here".to_string(),
                });
            });
            ui.label(
                RichText::new(format!(
                    "{} · cleared {}–{}",
                    value(r.measure, r.yours),
                    value(r.measure, r.range.p25),
                    value(r.measure, r.range.p75)
                ))
                .small(),
            );
        });
    }
    let fine: Vec<&str> = b.rows.iter().filter(|r| !r.short).map(|r| r.measure.label()).collect();
    if !fine.is_empty() {
        ui.label(caption(format!("{} more in range or better", fine.len()))).on_hover_text(fine.join(", "));
    }
}

/// A measure's value in its unit: shares as percentages, jobs as the job
/// lines word them.
fn value(m: Measure, x: f32) -> String {
    match m {
        Measure::Job(Role::ScalingDamage) => format!("×{x:.1}"),
        Measure::Job(Role::Aoe) => format!("{:.0}%", x * 100.0),
        Measure::Job(Role::Chip) => format!("{x:.1} hits"),
        Measure::Job(_) => format!("{x:.0}"),
        _ => format!("{:.0}%", x * 100.0),
    }
}

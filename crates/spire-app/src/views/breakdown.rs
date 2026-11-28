//! Deck composition: by card type (one stacked bar) and by macro role.

use eframe::egui::{RichText, Sense, Ui, vec2};
use spire_core::analysis::composition::{Composition, Row};
use spire_core::analysis::tags::Tag;
use spire_core::catalog::CardType;
use spire_core::catalog::Measure;
use spire_core::playbook::Role;

use crate::app::ViewCtx;
use crate::style;
use crate::widgets::{bar, caption, panel, range_bar};

pub fn show(ui: &mut Ui, v: &ViewCtx) {
    let d = &v.dash.deck;
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
        let comp = v.combat.and_then(|c| c.composition.as_ref());
        let jobs = v.combat.map(|c| &c.jobs);
        match comp {
            Some(c) => roles(ui, v, c),
            None => {
                // No benchmark for this act (or not simulated yet): counts.
                ui.label(caption("Roles"))
                    .on_hover_text("Bright: cards you added. Faded: starter Strikes and Defends.");
                let max = Role::ALL.iter().map(|r| d.role(*r) + d.starter_role(*r)).max().unwrap_or(1);
                for r in Role::ALL {
                    let row = bar(ui, r.label(), d.role(r), d.starter_role(r), max, style::role_color(r));
                    row.on_hover_ui(|ui| {
                        ui.set_max_width(360.0);
                        if let Some(j) = jobs {
                            ui.label(RichText::new(j.describe(r)).strong());
                        }
                        ui.label(v.playbook.role_criteria(r));
                    });
                }
            }
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
        let max_role = Role::ALL.iter().map(|r| d.role(*r) + d.starter_role(*r)).max().unwrap_or(1);
        bar(ui, "Draw", d.tag(Tag::Draw), 0, max_role, style::tag_color_of(Tag::Draw));
        bar(ui, "Energy gain", d.tag(Tag::Energy), 0, max_role, style::tag_color_of(Tag::Energy));
        // Stars only matter to decks that make them (the Regent's).
        if d.star_makers > 0 {
            bar(ui, "Star gain", d.star_makers, 0, max_role, style::star_color());
        }
        let counts: Vec<String> = [(d.upgraded, "upgraded"), (d.no_role, "with no role"), (d.unknown, "unknown")]
            .into_iter()
            .filter(|(n, _)| *n > 0)
            .map(|(n, what)| format!("{n} {what}"))
            .collect();
        if !counts.is_empty() {
            ui.label(caption(counts.join(" · ")));
        }
    });
}

/// Each role's job on the range of decks that beat this act's boss, then
/// damage to Block, deck size and dead draws. Warnings in the warning
/// colour; the numbers, the role's share of the deck and the evidence on
/// hover.
fn roles(ui: &mut Ui, v: &ViewCtx, c: &Composition) {
    let d = &v.dash.deck;
    let who = if c.all_characters { "all characters'" } else { "this character's" };
    ui.label(caption("Roles")).on_hover_text(format!(
        "Winners: A10 decks ({who}, {:.0} effective) that beat the Act {} boss. Band: middle half; tick: \
         median. Bars in the deck's mark colours; an amber outline: a side that loses the boss more often.",
        c.effective,
        c.act + 1
    ));
    fn flagged(r: Option<&Row>) -> Option<&Row> {
        r.filter(|r| r.flag.is_some())
    }
    for role in Role::ALL {
        let Some(job) = c.row(Measure::Job(role)) else { continue };
        let share = c.row(Measure::Share(role));
        // A warning on the job or on the role's share of the deck.
        let warning = flagged(Some(job)).or(flagged(share));
        let cards = d.role(role) + d.starter_role(role);
        let text = format!("{} · {cards}", job.measure.format(job.value));
        let row = range_bar(ui, role.label(), job.value, job.range, style::role_color(role), warning.is_some(), text);
        row.on_hover_ui(|ui| {
            ui.set_max_width(380.0);
            if let Some(w) = warning {
                ui.label(RichText::new(w.summary()).strong().color(style::color::warn()));
                if let Some(e) = w.evidence() {
                    ui.label(e);
                }
                ui.add_space(crate::style::space::xs());
            }
            ui.label(RichText::new(Measure::Job(role).describe()).strong());
            // The warned row's numbers are already in its warning.
            let warned = |m: Measure| warning.is_some_and(|w| w.measure == m);
            if !warned(job.measure) {
                ui.label(job.numbers());
            }
            if let Some(s) = share.filter(|s| !warned(s.measure)) {
                ui.label(format!("{cards} cards, {}; winners {}", s.measure.format(s.value), s.measure.range(s.range)));
            }
            ui.add_space(crate::style::space::xs());
            // The role's name and purpose are in the line above.
            let criteria = v.playbook.role_criteria(role);
            ui.label(caption(criteria.split_once('\n').map_or(criteria.as_str(), |(_, rest)| rest)));
        });
    }
    for m in [Measure::Balance, Measure::Size, Measure::Dead] {
        let Some(r) = c.row(m) else { continue };
        let row =
            range_bar(ui, &m.label(), r.value, r.range, style::color::text_dim(), r.flag.is_some(), m.format(r.value));
        row.on_hover_ui(|ui| {
            ui.set_max_width(380.0);
            if r.flag.is_some() {
                ui.label(RichText::new(r.summary()).strong().color(style::color::warn()));
                if let Some(e) = r.evidence() {
                    ui.label(e);
                }
            }
            ui.label(RichText::new(m.describe()).strong());
            if r.flag.is_none() {
                ui.label(r.numbers());
            }
        });
    }
}

//! Relic catalog: filter, browse, and see what the deck simulator makes of each relic.

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use eframe::egui::{self, RichText, Sense, Ui, vec2};
use spire_core::catalog::{Catalog, Rarity, Relic};
use spire_core::ids::RelicId;
use spire_core::mechanics::{self, Mechanics};
use spire_core::playbook::{Role, contains_ci};

use crate::app::ViewCtx;
use crate::style::{self, color::*};
use crate::widgets::{caption, eyebrow, hairline, segmented, stripe};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SimFilter {
    #[default]
    Any,
    Simulated,
    NotSimulated,
}

/// Each relic's roles, for relics that play any.
type RoleMap = Arc<BTreeMap<RelicId, Vec<Role>>>;

/// UI-only state of the relic catalog.
#[derive(Default)]
pub struct RelicBrowser {
    pub query: String,
    pub pool: Option<String>,
    pub rarity: Option<Rarity>,
    pub sim: SimFilter,
    pub in_run: bool,
    /// Only relics that play this role.
    pub role: Option<Role>,
    pub selected: Option<RelicId>,
    /// Filter key and matching relics, rebuilt only when the key changes.
    cache: Option<(u64, Vec<RelicId>)>,
    /// What the simulator reads from the selected relic.
    reading: Option<(RelicId, Mechanics)>,
    /// Every relic's roles, read once per data generation.
    roles: Option<(u64, RoleMap)>,
}

/// What the simulator reads from a relic's text.
pub fn reading(catalog: &Catalog, relic: &Relic) -> Mechanics {
    let orbs: Vec<_> = catalog.orbs.values().cloned().collect();
    mechanics::read(&relic.text, &orbs, false)
}

impl RelicBrowser {
    fn key(&self, v: &ViewCtx) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (&self.query, &self.pool, self.rarity, self.sim, self.in_run, self.role, v.generation).hash(&mut h);
        h.finish()
    }

    /// Roles of every relic, from what the simulator reads in its text.
    fn roles(&mut self, v: &ViewCtx) -> RoleMap {
        if self.roles.as_ref().is_none_or(|(g, _)| *g != v.generation) {
            let map = v
                .catalog
                .relics
                .values()
                .map(|r| (r.id.clone(), v.playbook.relic_roles(&r.text, &reading(v.catalog, r))))
                .filter(|(_, roles)| !roles.is_empty())
                .collect();
            self.roles = Some((v.generation, Arc::new(map)));
        }
        self.roles.as_ref().map(|(_, m)| m.clone()).unwrap_or_default()
    }

    fn matches(&mut self, v: &ViewCtx) -> &[RelicId] {
        let key = self.key(v);
        if self.cache.as_ref().is_none_or(|(k, _)| *k != key) {
            let role = self.role;
            let roles = self.roles(v);
            let q = self.query.trim();
            let mut list: Vec<&Relic> = v
                .catalog
                .relics
                .values()
                .filter(|r| q.is_empty() || contains_ci(&r.name, q) || contains_ci(&r.text, q))
                .filter(|r| self.pool.as_ref().is_none_or(|p| &r.pool == p))
                .filter(|r| self.rarity.is_none_or(|x| r.rarity == x))
                .filter(|r| !self.in_run || v.state.relics.contains(&r.id))
                .filter(|r| role.is_none_or(|x| roles.get(&r.id).is_some_and(|rs| rs.contains(&x))))
                .filter(|r| match self.sim {
                    SimFilter::Any => true,
                    SimFilter::Simulated => !reading(v.catalog, r).is_empty(),
                    SimFilter::NotSimulated => reading(v.catalog, r).is_empty(),
                })
                .collect();
            list.sort_by(|a, b| (&a.pool != "shared", &a.pool, &a.name).cmp(&(&b.pool != "shared", &b.pool, &b.name)));
            self.cache = Some((key, list.into_iter().map(|r| r.id.clone()).collect()));
        }
        self.cache.as_ref().map(|(_, l)| l.as_slice()).unwrap_or_default()
    }
}

fn label(s: &str) -> String {
    let mut s = s.to_string();
    if let Some(f) = s.get_mut(0..1) {
        f.make_ascii_uppercase();
    }
    s
}

fn pool_color(pool: &str) -> egui::Color32 {
    if pool == "shared" { text_faint() } else { style::character_color(&pool.to_uppercase()) }
}

pub fn filters(ui: &mut Ui, v: &ViewCtx, b: &mut RelicBrowser) {
    ui.add_space(crate::style::space::s());
    ui.label(eyebrow("Find relics"));
    hairline(ui);
    ui.add(egui::TextEdit::singleline(&mut b.query).hint_text("Name or text").desired_width(f32::INFINITY));

    ui.add_space(crate::style::space::s());
    ui.label(eyebrow("Pool"));
    let mut pools: Vec<&str> = v.catalog.relics.values().map(|r| r.pool.as_str()).collect();
    pools.sort();
    pools.dedup();
    egui::ComboBox::from_id_salt("relic-pool")
        .selected_text(b.pool.as_deref().map_or("All pools".into(), label))
        .width(ui.available_width())
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut b.pool, None, "All pools");
            for p in pools {
                ui.selectable_value(&mut b.pool, Some(p.to_string()), label(p));
            }
        });

    ui.add_space(crate::style::space::s());
    ui.label(eyebrow("Rarity"));
    let rarities = [
        (None, "All"),
        (Some(Rarity::Starter), "Starter"),
        (Some(Rarity::Common), "Common"),
        (Some(Rarity::Uncommon), "Uncommon"),
        (Some(Rarity::Rare), "Rare"),
        (Some(Rarity::Shop), "Shop"),
        (Some(Rarity::Event), "Event"),
        (Some(Rarity::Ancient), "Ancient"),
    ];
    segmented(ui, &mut b.rarity, &rarities);

    ui.add_space(crate::style::space::s());
    ui.label(eyebrow("Deck simulator"));
    segmented(
        ui,
        &mut b.sim,
        &[(SimFilter::Any, "Any"), (SimFilter::Simulated, "Simulated"), (SimFilter::NotSimulated, "Not simulated")],
    );

    ui.add_space(crate::style::space::s());
    ui.label(eyebrow("Role"));
    let mut roles: Vec<(Option<Role>, &str)> = vec![(None, "Any")];
    roles.extend(Role::ALL.iter().map(|r| (Some(*r), r.label())));
    segmented(ui, &mut b.role, &roles);

    ui.add_space(crate::style::space::s());
    ui.checkbox(&mut b.in_run, "Only relics in this run");

    ui.add_space(crate::style::space::s());
    if ui.add(egui::Button::new(RichText::new("Clear filters").color(text_dim())).frame_when_inactive(false)).clicked()
    {
        *b = RelicBrowser { selected: b.selected.take(), ..RelicBrowser::default() };
    }
}

pub fn list(ui: &mut Ui, v: &ViewCtx, b: &mut RelicBrowser) {
    let ids = b.matches(v).to_vec();
    let roles = b.roles(v);
    ui.horizontal(|ui| {
        ui.label(eyebrow("Relics"));
        ui.label(caption(format!("{} of {}", ids.len(), v.catalog.relics.len())));
    });
    hairline(ui);
    if ids.is_empty() {
        ui.label(RichText::new("No relics match these filters.").color(text_dim()));
        return;
    }
    let row_h = ui.spacing().interact_size.y;
    egui::ScrollArea::vertical().auto_shrink(false).show_rows(ui, row_h, ids.len(), |ui, range| {
        for id in &ids[range] {
            let Some(relic) = v.catalog.relics.get(id) else { continue };
            let selected = b.selected.as_ref() == Some(id);
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), row_h), Sense::click());
            if ui.is_rect_visible(rect) {
                crate::widgets::row_backplate(ui, &resp, &relic.name, selected);
                let painter = ui.painter();
                let stripe_r = egui::Rect::from_min_size(rect.min + vec2(0.0, 3.0), vec2(3.0, row_h - 6.0));
                painter.rect_filled(stripe_r, 0.0, pool_color(&relic.pool));
                let font = egui::TextStyle::Body.resolve(ui.style());
                let small = egui::TextStyle::Small.resolve(ui.style());
                let mid = rect.center().y;
                let owned = v.state.relics.contains(id);
                // Owned relics read stronger, not in the interactive accent.
                let font = if owned { egui::FontId::new(font.size, style::strong_family()) } else { font };
                painter.text(egui::pos2(rect.min.x + 12.0, mid), egui::Align2::LEFT_CENTER, &relic.name, font, text());
                let right = format!("{} · {:?}", label(&relic.pool), relic.rarity);
                let at = painter.text(
                    egui::pos2(rect.max.x - 6.0, mid),
                    egui::Align2::RIGHT_CENTER,
                    right,
                    small,
                    text_faint(),
                );
                // One square per role, left of the pool and rarity.
                let mut x = at.min.x - 10.0;
                for r in roles.get(id).into_iter().flatten().rev() {
                    let sq = egui::Rect::from_center_size(egui::pos2(x, mid), vec2(7.0, 7.0));
                    painter.rect_filled(sq, 0.0, style::role_color(*r));
                    x -= 10.0;
                }
            }
            if resp.clicked() {
                b.selected = Some(id.clone());
            }
        }
    });
}

pub fn detail(ui: &mut Ui, v: &ViewCtx, b: &mut RelicBrowser) {
    ui.add_space(crate::style::space::s());
    let Some(relic) = b.selected.as_ref().and_then(|id| v.catalog.relics.get(id)) else {
        ui.label(
            RichText::new("Select a relic to see its text and what the deck simulator does with it.").color(text_dim()),
        );
        return;
    };
    if b.reading.as_ref().is_none_or(|(id, _)| *id != relic.id) {
        b.reading = Some((relic.id.clone(), reading(v.catalog, relic)));
    }
    ui.horizontal(|ui| {
        stripe(ui, pool_color(&relic.pool), 26.0);
        ui.label(crate::widgets::title(&relic.name));
    });
    ui.label(RichText::new(format!("{} · {:?}", label(&relic.pool), relic.rarity)).color(text_dim()));
    if v.state.relics.contains(&relic.id) {
        ui.label(RichText::new("In your run").small().strong().color(text()));
    }
    ui.add_space(crate::style::space::s());
    ui.label(eyebrow("Text"));
    hairline(ui);
    ui.label(&relic.text);
    if let Some((_, m)) = &b.reading {
        let roles = v.playbook.relic_roles(&relic.text, m);
        if !roles.is_empty() {
            ui.add_space(crate::style::space::s());
            ui.label(eyebrow("Role"));
            ui.horizontal_wrapped(|ui| {
                for r in &roles {
                    crate::widgets::chip(ui, r.label(), style::role_color(*r)).on_hover_ui(|ui| {
                        ui.set_max_width(360.0);
                        ui.label(v.playbook.relic_criteria(*r));
                    });
                }
            });
        }
        ui.add_space(crate::style::space::s());
        simulator_reading(ui, m);
    }
}

/// "In the deck simulator": what was understood, and what wasn't.
pub fn simulator_reading(ui: &mut Ui, m: &Mechanics) {
    ui.label(eyebrow("In the deck simulator"));
    hairline(ui);
    if m.is_empty() {
        ui.label(RichText::new("Not simulated.").color(text_dim()));
    }
    for line in m.describe() {
        ui.label(RichText::new(line).color(good()));
    }
    if !m.unread.is_empty() {
        ui.add_space(crate::style::space::xs());
        ui.label(caption("Not simulated:"));
        for s in &m.unread {
            ui.label(caption(s));
        }
    }
}

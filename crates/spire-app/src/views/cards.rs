//! Card catalog: filter, browse, inspect, and set your own roles.

use std::hash::{Hash, Hasher};

use eframe::egui::{self, RichText, Sense, Ui, vec2};
use spire_core::analysis::deck::{CardTag, tags};
use spire_core::catalog::{Card, CardType, Rarity};
use spire_core::ids::CardId;
use spire_core::playbook::{Role, RoleSource, contains_ci};

use crate::actions::Action;
use crate::app::ViewCtx;
use crate::style::{self, color::*};
use crate::widgets::{eyebrow, hairline, segmented, stripe};

/// Keywords worth filtering by.
const KEYWORDS: [&str; 10] =
    ["Draw", "Energy gain", "Eternal", "Exhaust", "Ethereal", "Retain", "Innate", "Unplayable", "Sly", "X cost"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CostFilter {
    Any,
    Zero,
    One,
    Two,
    ThreePlus,
    X,
    Unplayable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RoleFilter {
    Any,
    Role(Role),
    NoRole,
}

/// UI-only state of the card catalog.
pub struct CardBrowser {
    pub query: String,
    pub pool: Option<String>,
    pub card_type: Option<CardType>,
    pub rarity: Option<Rarity>,
    pub cost: CostFilter,
    pub role: RoleFilter,
    pub keyword: Option<&'static str>,
    pub selected: Option<CardId>,
    /// The detail pane shows the upgraded card.
    pub upgraded: bool,
    /// Filter key and matching cards, rebuilt only when the key changes.
    cache: Option<(u64, Vec<CardId>)>,
}

impl Default for CardBrowser {
    fn default() -> Self {
        Self {
            query: String::new(),
            pool: None,
            card_type: None,
            rarity: None,
            cost: CostFilter::Any,
            role: RoleFilter::Any,
            keyword: None,
            selected: None,
            upgraded: false,
            cache: None,
        }
    }
}

impl CardBrowser {
    fn key(&self, generation: u64) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (&self.query, &self.pool, self.card_type, self.rarity, self.cost, self.role, self.keyword, generation)
            .hash(&mut h);
        h.finish()
    }

    fn keeps(&self, v: &ViewCtx, c: &Card) -> bool {
        let q = self.query.trim();
        (q.is_empty() || contains_ci(&c.name, q) || contains_ci(&c.text, q))
            && self.pool.as_ref().is_none_or(|p| &c.color == p)
            && self.card_type.is_none_or(|t| c.card_type == t)
            && self.rarity.is_none_or(|r| c.rarity == r)
            && match self.cost {
                CostFilter::Any => true,
                CostFilter::Zero => !c.x_cost && c.cost == Some(0),
                CostFilter::One => !c.x_cost && c.cost == Some(1),
                CostFilter::Two => !c.x_cost && c.cost == Some(2),
                CostFilter::ThreePlus => !c.x_cost && c.cost.is_some_and(|n| n >= 3),
                CostFilter::X => c.x_cost,
                CostFilter::Unplayable => c.is_unplayable(),
            }
            && match self.keyword {
                None => true,
                Some("X cost") => c.x_cost,
                Some("Draw") => tags(c, false).contains(&CardTag::Draw),
                Some("Energy gain") => tags(c, false).contains(&CardTag::Energy),
                Some(k) => c.keywords.iter().any(|x| x.eq_ignore_ascii_case(k)),
            }
            && match self.role {
                RoleFilter::Any => true,
                RoleFilter::Role(r) => v.playbook.roles_of(c).contains(&r),
                RoleFilter::NoRole => v.playbook.roles_of(c).is_empty(),
            }
    }

    /// Matching cards, sorted by pool, type, then name.
    fn matches(&mut self, v: &ViewCtx) -> &[CardId] {
        let key = self.key(v.generation);
        if self.cache.as_ref().is_none_or(|(k, _)| *k != key) {
            let mut list: Vec<&Card> = v.catalog.cards.values().filter(|c| self.keeps(v, c)).collect();
            list.sort_by(|a, b| (&a.color, a.card_type, &a.name).cmp(&(&b.color, b.card_type, &b.name)));
            self.cache = Some((key, list.into_iter().map(|c| c.id.clone()).collect()));
        }
        self.cache.as_ref().map(|(_, l)| l.as_slice()).unwrap_or_default()
    }
}

fn pool_label(pool: &str) -> String {
    let mut s = pool.to_string();
    if let Some(f) = s.get_mut(0..1) {
        f.make_ascii_uppercase();
    }
    s
}

pub fn filters(ui: &mut Ui, v: &ViewCtx, b: &mut CardBrowser) {
    ui.add_space(crate::style::space::s());
    ui.label(eyebrow("Find cards"));
    hairline(ui);
    ui.add(egui::TextEdit::singleline(&mut b.query).hint_text("Name or text").desired_width(f32::INFINITY));

    ui.add_space(crate::style::space::s());
    ui.label(eyebrow("Pool"));
    let mut pools: Vec<&str> = v.catalog.cards.values().map(|c| c.color.as_str()).collect();
    pools.sort();
    pools.dedup();
    egui::ComboBox::from_id_salt("pool")
        .selected_text(b.pool.as_deref().map_or("All pools".into(), pool_label))
        .width(ui.available_width())
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut b.pool, None, "All pools");
            for p in pools {
                ui.selectable_value(&mut b.pool, Some(p.to_string()), pool_label(p));
            }
        });

    ui.add_space(crate::style::space::s());
    ui.label(eyebrow("Type"));
    let types = [
        (None, "All"),
        (Some(CardType::Attack), "Attack"),
        (Some(CardType::Skill), "Skill"),
        (Some(CardType::Power), "Power"),
        (Some(CardType::Curse), "Curse"),
        (Some(CardType::Status), "Status"),
    ];
    segmented(ui, &mut b.card_type, &types);

    ui.add_space(crate::style::space::s());
    ui.label(eyebrow("Rarity"));
    let rarities = [
        (None, "All"),
        (Some(Rarity::Basic), "Basic"),
        (Some(Rarity::Common), "Common"),
        (Some(Rarity::Uncommon), "Uncommon"),
        (Some(Rarity::Rare), "Rare"),
        (Some(Rarity::Ancient), "Ancient"),
    ];
    segmented(ui, &mut b.rarity, &rarities);

    ui.add_space(crate::style::space::s());
    ui.label(eyebrow("Cost"));
    let costs = [
        (CostFilter::Any, "Any"),
        (CostFilter::Zero, "0"),
        (CostFilter::One, "1"),
        (CostFilter::Two, "2"),
        (CostFilter::ThreePlus, "3+"),
        (CostFilter::X, "X"),
        (CostFilter::Unplayable, "Unplayable"),
    ];
    segmented(ui, &mut b.cost, &costs);

    ui.add_space(crate::style::space::s());
    ui.label(eyebrow("Role"));
    let mut roles: Vec<(RoleFilter, &str)> = vec![(RoleFilter::Any, "Any")];
    roles.extend(Role::ALL.iter().map(|r| (RoleFilter::Role(*r), r.short())));
    roles.push((RoleFilter::NoRole, "No role"));
    segmented(ui, &mut b.role, &roles);

    ui.add_space(crate::style::space::s());
    ui.label(eyebrow("Keyword"));
    let mut keywords: Vec<(Option<&'static str>, &str)> = vec![(None, "Any")];
    keywords.extend(KEYWORDS.iter().map(|k| (Some(*k), *k)));
    segmented(ui, &mut b.keyword, &keywords);

    ui.add_space(crate::style::space::s());
    if ui.add(egui::Button::new(RichText::new("Clear filters").color(text_dim())).frame_when_inactive(false)).clicked()
    {
        *b = CardBrowser { selected: b.selected.take(), ..CardBrowser::default() };
    }
}

pub fn list(ui: &mut Ui, v: &ViewCtx, b: &mut CardBrowser) {
    let ids = b.matches(v).to_vec();
    ui.horizontal(|ui| {
        ui.label(eyebrow("Cards"));
        ui.label(RichText::new(format!("{} of {}", ids.len(), v.catalog.cards.len())).small().color(text_faint()));
    });
    hairline(ui);
    if ids.is_empty() {
        ui.label(RichText::new("No cards match these filters.").color(text_dim()));
        return;
    }
    let row_h = ui.spacing().interact_size.y;
    // Only the visible rows are laid out: 700+ cards stay cheap.
    egui::ScrollArea::vertical().auto_shrink(false).show_rows(ui, row_h, ids.len(), |ui, range| {
        for id in &ids[range] {
            let Some(card) = v.catalog.cards.get(id) else { continue };
            let selected = b.selected.as_ref() == Some(id);
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), row_h), Sense::click());
            if ui.is_rect_visible(rect) {
                crate::widgets::row_backplate(ui, &resp, &card.name, selected);
                let painter = ui.painter();
                let stripe_r = egui::Rect::from_min_size(rect.min + vec2(0.0, 3.0), vec2(3.0, row_h - 6.0));
                painter.rect_filled(stripe_r, 0.0, style::card_type_color(card.card_type));
                let font = egui::TextStyle::Body.resolve(ui.style());
                let small = egui::TextStyle::Small.resolve(ui.style());
                let mid = rect.center().y;
                painter.text(
                    egui::pos2(rect.min.x + 12.0, mid),
                    egui::Align2::LEFT_CENTER,
                    card.cost_label(),
                    small.clone(),
                    text_dim(),
                );
                painter.text(egui::pos2(rect.min.x + 30.0, mid), egui::Align2::LEFT_CENTER, &card.name, font, text());
                let right = format!("{} · {:?}", pool_label(&card.color), card.rarity);
                painter.text(egui::pos2(rect.max.x - 6.0, mid), egui::Align2::RIGHT_CENTER, right, small, text_faint());
            }
            if resp.clicked() {
                b.selected = Some(id.clone());
            }
        }
    });
}

pub fn detail(ui: &mut Ui, v: &ViewCtx, b: &mut CardBrowser, out: &mut Vec<Action>) {
    ui.add_space(crate::style::space::s());
    let Some(card) = b.selected.as_ref().and_then(|id| v.catalog.cards.get(id)) else {
        ui.label(eyebrow("Card"));
        hairline(ui);
        ui.label(RichText::new("Select a card to see its text, roles and enchantments.").color(text_dim()));
        return;
    };
    let changes = card.upgrade_changes();
    let can_upgrade = !changes.is_empty() || card.upgrade_text.as_ref().is_some_and(|t| !t.trim().is_empty());
    let up = b.upgraded && can_upgrade;
    ui.horizontal(|ui| {
        stripe(ui, style::card_type_color(card.card_type), 26.0);
        let name = if up { format!("{}+", card.name) } else { card.name.clone() };
        ui.label(crate::widgets::title(name).color(if up { good() } else { text() }));
        if can_upgrade {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                crate::widgets::segmented(ui, &mut b.upgraded, &[(false, "Base"), (true, "Upgraded")]);
            });
        }
    });
    ui.label(
        RichText::new(format!(
            "{} · {:?} · cost {} · {}",
            card.card_type.label(),
            card.rarity,
            card.cost_label_at(up),
            pool_label(&card.color)
        ))
        .color(text_dim()),
    );
    let keywords = card.keywords_at(up);
    let card_tags = tags(card, up);
    if !keywords.is_empty() || !card_tags.is_empty() {
        ui.horizontal_wrapped(|ui| {
            for k in keywords {
                let eternal = k.eq_ignore_ascii_case("Eternal");
                crate::widgets::chip(ui, k, if eternal { bad() } else { info() });
            }
            for t in card_tags {
                crate::widgets::chip(ui, t.describe(), style::role_color(Role::Consistency))
                    .on_hover_text("Usually counted as Consistency / acceleration; this says what it does.");
            }
        });
    }
    let in_deck = v.state.deck.iter().filter(|d| d.card == card.id).count();
    if in_deck > 0 {
        ui.label(RichText::new(format!("{in_deck} in your deck")).small().strong().color(text()));
    }

    ui.add_space(crate::style::space::s());
    ui.label(eyebrow("Text"));
    hairline(ui);
    ui.label(card.text_at(up));
    if !changes.is_empty() {
        ui.add_space(crate::style::space::xs());
        ui.label(RichText::new(format!("Upgrade: {}", changes.join(" · "))).small().color(good()));
    } else if !can_upgrade {
        ui.label(RichText::new("No upgrade").small().color(text_faint()));
    }

    if card.card_type == CardType::Power {
        ui.add_space(crate::style::space::s());
        let orbs: Vec<_> = v.catalog.orbs.values().cloned().collect();
        crate::views::relics::simulator_reading(ui, &spire_core::mechanics::read(card.text_at(up), &orbs, true));
    }

    ui.add_space(crate::style::space::s());
    ui.label(eyebrow("Roles"));
    hairline(ui);
    let source = v.playbook.role_source(card);
    let current = v.playbook.roles_of(card);
    match &source {
        RoleSource::Personal(_) => {
            ui.label(RichText::new("Your override").small().strong().color(text()));
        }
        RoleSource::Playbook(_) => {
            ui.label(RichText::new("Set in the playbook").small().color(text_dim()));
        }
        RoleSource::Rules(matched) if matched.is_empty() => {
            ui.label(RichText::new("No rule matches this card").small().color(text_dim()));
        }
        RoleSource::Rules(matched) => {
            ui.label(RichText::new("From the role rules:").small().color(text_dim()));
            for (role, why) in matched {
                ui.label(RichText::new(format!("{}: {why}", role.short())).small().color(style::role_color(*role)));
            }
        }
    }
    ui.add_space(crate::style::space::xs());
    let mut edited = current.clone();
    let mut changed = false;
    for r in Role::ALL {
        let mut on = edited.contains(&r);
        let label = RichText::new(r.label()).color(style::role_color(r));
        if ui.checkbox(&mut on, label).changed() {
            changed = true;
            if on {
                edited.push(r);
            } else {
                edited.retain(|x| *x != r);
            }
        }
    }
    if changed {
        edited.sort();
        out.push(Action::SetPersonalRoles { card: card.id.clone(), roles: Some(edited) });
    }
    if matches!(source, RoleSource::Personal(_))
        && ui.add(egui::Button::new(RichText::new("Reset to the playbook").color(text_dim()))).clicked()
    {
        out.push(Action::SetPersonalRoles { card: card.id.clone(), roles: None });
    }
    ui.label(
        RichText::new("Changes are saved as your own overrides, separate from the shipped rules.")
            .small()
            .color(text_faint()),
    );

    ui.add_space(crate::style::space::s());
    ui.label(eyebrow("Enchantments that fit"));
    hairline(ui);
    let fits: Vec<_> = v.catalog.enchantments.values().filter(|e| e.fits(card)).collect();
    if fits.is_empty() {
        ui.label(RichText::new("None").color(text_dim()));
    }
    ui.horizontal_wrapped(|ui| {
        for e in fits {
            crate::widgets::chip(ui, e.name.as_str(), info()).on_hover_text(&e.text);
        }
    });
}

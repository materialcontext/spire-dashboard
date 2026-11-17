//! Benchmark scenarios shared by the headless frame bench
//! (`examples/frame_bench.rs`) and the in-window render bench
//! (`SPIRE_BENCH=<scenario>[:frames] spire-dashboard`).
//!
//! A scenario is a run fixture, a window size, and what changes each frame:
//! pointer motion, wheel scrolling, typing, clicks, or app-state changes.

use std::sync::OnceLock;
use std::time::Instant;

/// Set by `main` so the render bench can report launch-to-first-frame time.
pub static PROCESS_START: OnceLock<Instant> = OnceLock::new();

use eframe::egui::{self, Event, Modifiers, MouseWheelUnit, Pos2, Rect, TouchPhase, Vec2, pos2, vec2};
use spire_core::catalog::{CardType, Catalog, Rarity, RoomType};
use spire_core::ids::*;
use spire_core::run::{Change, DeckCard, NodeKind, NodeReport, Outcome};
use spire_core::{Run, RunEvent};
use spire_data::store::Store;

use crate::actions::Action;
use crate::app::SpireApp;

pub const DEFAULT_SIZE: Vec2 = vec2(1400.0, 900.0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scenario {
    /// Character select, no run.
    Start,
    /// Mid act 1, 12-card deck, no input: the cost of any repaint.
    Idle,
    /// Act 3, 35-card deck, 12 relics: the heaviest steady state.
    IdleLate,
    /// Late run at 900 px wide (single-column layout).
    Narrow,
    /// Late run with the Data window open.
    DataWindow,
    /// Pointer sweeping across the deck and threat panels.
    HoverSweep,
    /// Pointer resting on a deck card until its tooltip shows.
    Tooltip,
    /// Mouse-wheel scrolling the central panel.
    Scroll,
    /// Typing card names into the card search, one keystroke per frame.
    Typing,
    /// Encounter dropdown opened by a click.
    ComboOpen,
    /// Window width changing every frame.
    Resize,
    /// Commit a node, then undo, alternating: recompute + save.
    CommitUndo,
    /// Card catalog with a card selected.
    CardsTab,
    /// Enemy catalog with an elite selected.
    EnemiesTab,
    /// Analytics for the late run: charts and the deck simulation.
    AnalyticsTab,
    /// History with three archived runs, the newest selected.
    HistoryTab,
}

impl Scenario {
    pub const ALL: [Scenario; 16] = [
        Scenario::Start,
        Scenario::Idle,
        Scenario::IdleLate,
        Scenario::Narrow,
        Scenario::DataWindow,
        Scenario::HoverSweep,
        Scenario::Tooltip,
        Scenario::Scroll,
        Scenario::Typing,
        Scenario::ComboOpen,
        Scenario::Resize,
        Scenario::CommitUndo,
        Scenario::CardsTab,
        Scenario::EnemiesTab,
        Scenario::AnalyticsTab,
        Scenario::HistoryTab,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Scenario::Start => "start",
            Scenario::Idle => "idle",
            Scenario::IdleLate => "idle-late",
            Scenario::Narrow => "narrow",
            Scenario::DataWindow => "data-window",
            Scenario::HoverSweep => "hover-sweep",
            Scenario::Tooltip => "tooltip",
            Scenario::Scroll => "scroll",
            Scenario::Typing => "typing",
            Scenario::ComboOpen => "combo-open",
            Scenario::Resize => "resize",
            Scenario::CommitUndo => "commit-undo",
            Scenario::CardsTab => "cards",
            Scenario::EnemiesTab => "enemies",
            Scenario::AnalyticsTab => "analytics",
            Scenario::HistoryTab => "history",
        }
    }

    pub fn parse(name: &str) -> Option<Scenario> {
        Scenario::ALL.into_iter().find(|s| s.name() == name)
    }

    /// Whether the scenario needs synthetic pointer/keyboard input (and so
    /// only runs headlessly).
    pub fn needs_input(self) -> bool {
        matches!(self, Scenario::HoverSweep | Scenario::Tooltip | Scenario::Scroll | Scenario::ComboOpen)
    }

    fn late(self) -> bool {
        !matches!(self, Scenario::Start | Scenario::Idle)
    }

    /// Window size for frame `i`.
    pub fn size(self, i: usize) -> Vec2 {
        match self {
            Scenario::Narrow => vec2(900.0, 700.0),
            Scenario::Resize => vec2(1000.0 + (i % 40) as f32 * 10.0, 900.0),
            _ => DEFAULT_SIZE,
        }
    }

    /// A fresh app in a throwaway data dir, set up for this scenario.
    pub fn app(self) -> SpireApp {
        let dir = std::env::temp_dir().join(format!("spire-bench-{}-{}", self.name(), std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        if self == Scenario::HistoryTab {
            let store = Store::at(&dir);
            let cat = store.load().catalog;
            for run in archived_runs(&cat) {
                store.archive_run(&run).expect("archive bench run");
            }
        }
        let mut app = SpireApp::from_store(Store::at(dir));
        if self != Scenario::Start {
            let run = if self.late() { late_run(app.catalog()) } else { mid_run(app.catalog()) };
            app.set_run(run);
        }
        if self == Scenario::DataWindow {
            app.ui_state().show_data = true;
        }
        if self == Scenario::CardsTab {
            app.open("cards", Some("BASH"));
        }
        if self == Scenario::EnemiesTab {
            app.open("enemies", Some("KNIGHTS_ELITE"));
        }
        if self == Scenario::AnalyticsTab {
            app.open("analytics", None);
        }
        if self == Scenario::HistoryTab {
            app.open("history", None);
        }
        app
    }

    /// State changes applied before frame `i` (works with or without a window).
    pub fn before_frame(self, app: &mut SpireApp, i: usize, ctx: &egui::Context) {
        match self {
            Scenario::Typing => {
                const WORDS: [&str; 4] = ["strike", "defend", "inflame", "bash"];
                let word = WORDS[(i / 8) % WORDS.len()];
                let len = (i % 8).min(word.len());
                app.draft().card_query = word[..len].to_string();
            }
            Scenario::CommitUndo => {
                let action = if i.is_multiple_of(2) {
                    Action::Record(RunEvent::Node(NodeReport {
                        kind: NodeKind::Monster,
                        encounter: None,
                        event: None,
                        changes: vec![Change::SetGold(500)],
                    }))
                } else {
                    Action::Undo
                };
                app.apply(action, ctx);
            }
            _ => {}
        }
    }

    /// Synthetic input events for frame `i` (headless only).
    pub fn events(self, i: usize) -> Vec<Event> {
        match self {
            Scenario::HoverSweep => {
                // Down the deck panel, then down the threat column.
                let x = if (i / 60).is_multiple_of(2) { 120.0 } else { 900.0 };
                vec![Event::PointerMoved(pos2(x, 60.0 + (i % 60) as f32 * 13.0))]
            }
            // Move once and rest: tooltips need the pointer still for a moment.
            Scenario::Tooltip if i == 1 => vec![Event::PointerMoved(pos2(70.0, 142.0))],
            Scenario::Scroll => vec![
                Event::PointerMoved(pos2(700.0, 500.0)),
                Event::MouseWheel {
                    unit: MouseWheelUnit::Point,
                    delta: vec2(0.0, if (i / 30).is_multiple_of(2) { -24.0 } else { 24.0 }),
                    phase: TouchPhase::Move,
                    source: egui::MouseWheelSource::Wheel,
                    modifiers: Modifiers::NONE,
                },
            ],
            // egui hit-tests against the previous frame, so click after layout exists.
            Scenario::ComboOpen if i == 2 => click(pos2(1235.0, 276.0)),
            _ => Vec::new(),
        }
    }

    pub fn raw_input(self, i: usize, time: f64) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.size(i))),
            time: Some(time),
            predicted_dt: 1.0 / 60.0,
            events: self.events(i),
            ..Default::default()
        }
    }
}

fn click(pos: Pos2) -> Vec<Event> {
    let button = |pressed| Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    vec![Event::PointerMoved(pos), button(true), button(false)]
}

fn character_cards<'a>(cat: &'a Catalog, color: &'a str) -> impl Iterator<Item = &'a CardId> + 'a {
    cat.cards
        .values()
        .filter(move |c| c.color == color && matches!(c.rarity, Rarity::Common | Rarity::Uncommon | Rarity::Rare))
        .map(|c| &c.id)
}

fn node(kind: NodeKind, encounter: Option<&EncounterId>, changes: Vec<Change>) -> RunEvent {
    RunEvent::Node(NodeReport { kind, encounter: encounter.cloned(), event: None, changes })
}

/// Ironclad, act 1 (Overgrowth), four nodes in, 12 cards.
pub fn mid_run(cat: &Catalog) -> Run {
    let mut run = Run::default();
    let hero = CharacterId::new("IRONCLAD");
    run.push(RunEvent::start(cat, &hero, 3).expect("bundled Ironclad"));
    let act = ActId::new("OVERGROWTH");
    let boss = cat.acts.get(&act).and_then(|a| a.bosses.first().cloned());
    run.push(RunEvent::EnterAct { act: act.clone(), index: 0, boss });
    let fights: Vec<&EncounterId> = cat.encounters_in(&act, RoomType::Monster).map(|e| &e.id).take(3).collect();
    for (i, card) in character_cards(cat, "ironclad").take(2).enumerate() {
        run.push(node(
            NodeKind::Monster,
            fights.get(i).copied(),
            vec![Change::AddCard(DeckCard::new(card.clone())), Change::SetHp(70 - 6 * i as i32)],
        ));
    }
    run.push(node(NodeKind::Rest, None, vec![]));
    run
}

/// Ironclad deep in act 3: 35 cards, 12 relics, 3 potions, many fights seen.
pub fn late_run(cat: &Catalog) -> Run {
    let mut run = mid_run(cat);
    let cards: Vec<CardId> = character_cards(cat, "ironclad").skip(2).take(23).cloned().collect();
    let relics: Vec<RelicId> = cat.relics.keys().take(11).cloned().collect();
    let potions: Vec<PotionId> = cat.potions.keys().take(3).cloned().collect();
    let curse = cat.cards.values().find(|c| c.card_type == CardType::Curse).map(|c| c.id.clone());

    let mut changes: Vec<Change> = cards
        .iter()
        .enumerate()
        .map(|(i, c)| Change::AddCard(DeckCard { card: c.clone(), upgraded: i % 3 == 0, enchantment: None }))
        .chain(relics.into_iter().map(Change::AddRelic))
        .chain(potions.into_iter().map(Change::AddPotion))
        .chain(curse.map(|c| Change::AddCard(DeckCard::new(c))))
        .collect();
    changes.push(Change::SetMaxHp(92));
    changes.push(Change::SetHp(61));
    changes.push(Change::SetGold(240));
    run.push(RunEvent::Adjust(changes));

    // HP and gold move fight by fight, so the analytics charts have shape.
    let (mut hp, mut gold) = (61, 240);
    for (index, act) in [(1u8, "HIVE"), (2, "GLORY")] {
        let act = ActId::new(act);
        let boss = cat.acts.get(&act).and_then(|a| a.bosses.first().cloned());
        run.push(RunEvent::EnterAct { act: act.clone(), index, boss });
        let seen: Vec<EncounterId> = cat.encounters_in(&act, RoomType::Monster).map(|e| e.id.clone()).take(5).collect();
        for (i, e) in seen.iter().enumerate() {
            hp = (hp - 4 - (i as i32 * 7) % 11).max(12);
            gold += 12 + (i as i32 * 5) % 9;
            run.push(node(NodeKind::Monster, Some(e), vec![Change::SetHp(hp), Change::SetGold(gold)]));
        }
        hp = (hp + 26).min(92);
        run.push(node(NodeKind::Rest, None, vec![Change::SetHp(hp)]));
        let elite = cat.encounters_in(&act, RoomType::Elite).map(|e| e.id.clone()).next();
        hp = (hp - 19).max(12);
        gold += 30;
        run.push(node(NodeKind::Elite, elite.as_ref(), vec![Change::SetHp(hp), Change::SetGold(gold)]));
    }
    run
}

/// A few finished runs, for the History tab.
pub fn archived_runs(cat: &Catalog) -> Vec<Run> {
    let stamp = |n: u32| Some(format!("2026-11-{n:02}T18:00:00Z"));
    let mut won = late_run(cat);
    won.started_at = stamp(10);
    won.push(RunEvent::End { outcome: Outcome::Victory, at: stamp(10) });
    let mut died = mid_run(cat);
    died.started_at = stamp(12);
    let killer = cat.encounters_in(&ActId::new("OVERGROWTH"), RoomType::Elite).map(|e| e.id.clone()).next();
    died.push(RunEvent::End { outcome: Outcome::Died { encounter: killer }, at: stamp(12) });
    let mut quit = mid_run(cat);
    quit.started_at = stamp(14);
    quit.push(RunEvent::End { outcome: Outcome::Abandoned, at: stamp(14) });
    vec![won, died, quit]
}

// ---------------------------------------------------------------- stats

#[derive(Clone, Debug, Default)]
pub struct Stats {
    pub median: f64,
    pub p95: f64,
    pub max: f64,
    pub mean: f64,
}

/// Summary of millisecond samples.
pub fn stats(samples: &[f64]) -> Stats {
    if samples.is_empty() {
        return Stats::default();
    }
    let mut s = samples.to_vec();
    s.sort_by(f64::total_cmp);
    let at = |q: f64| s[((s.len() - 1) as f64 * q).round() as usize];
    Stats { median: at(0.5), p95: at(0.95), max: s[s.len() - 1], mean: s.iter().sum::<f64>() / s.len() as f64 }
}

// ---------------------------------------------------------------- in-window bench

/// `SPIRE_BENCH=<scenario>[:frames]`: render continuously, then print timings.
#[derive(Clone, Copy, Debug)]
pub struct RenderBench {
    pub scenario: Scenario,
    pub frames: usize,
}

impl RenderBench {
    pub fn parse(spec: &str) -> Option<RenderBench> {
        let (name, frames) = spec.split_once(':').unwrap_or((spec, "300"));
        let scenario = Scenario::parse(name)?;
        if scenario.needs_input() {
            eprintln!("scenario {name} needs synthetic input; use the headless frame_bench");
            return None;
        }
        Some(RenderBench { scenario, frames: frames.parse().unwrap_or(300) })
    }

    pub fn into_app(self) -> RenderBenchApp {
        RenderBenchApp {
            app: self.scenario.app(),
            bench: self,
            frame: 0,
            last: None,
            ui_ms: Vec::new(),
            cpu_ms: Vec::new(),
            interval_ms: Vec::new(),
            created: Instant::now(),
            first_frame_ms: None,
        }
    }
}

pub struct RenderBenchApp {
    app: SpireApp,
    bench: RenderBench,
    frame: usize,
    last: Option<Instant>,
    ui_ms: Vec<f64>,
    cpu_ms: Vec<f64>,
    interval_ms: Vec<f64>,
    created: Instant,
    first_frame_ms: Option<f64>,
}

const WARMUP: usize = 30;

impl eframe::App for RenderBenchApp {
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        crate::style::clear_color(visuals)
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let now = Instant::now();
        let ms = |d: std::time::Duration| d.as_secs_f64() * 1000.0;
        let origin = PROCESS_START.get().copied().unwrap_or(self.created);
        self.first_frame_ms.get_or_insert_with(|| ms(now - origin));
        let measuring = self.frame >= WARMUP;
        if measuring {
            if let Some(last) = self.last {
                self.interval_ms.push(ms(now - last));
            }
            if let Some(cpu) = frame.info().cpu_usage {
                self.cpu_ms.push(f64::from(cpu) * 1000.0);
            }
        }
        self.last = Some(now);

        let ctx = ui.ctx().clone();
        if self.frame == 0 && self.bench.scenario == Scenario::Narrow {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(self.bench.scenario.size(0)));
        }
        let t = Instant::now();
        self.bench.scenario.before_frame(&mut self.app, self.frame, &ctx);
        self.app.frame(ui);
        if measuring {
            self.ui_ms.push(ms(t.elapsed()));
        }

        self.frame += 1;
        if self.frame == WARMUP + self.bench.frames {
            let (u, c, i) = (stats(&self.ui_ms), stats(&self.cpu_ms), stats(&self.interval_ms));
            // Peak resident memory (Linux; best effort).
            let peak_kb = std::fs::read_to_string("/proc/self/status")
                .ok()
                .and_then(|s| s.lines().find(|l| l.starts_with("VmHWM:")).map(|l| l[6..].trim().to_string()))
                .unwrap_or_default();
            println!(
                "render-bench {} frames={} first_frame_ms={:.1} peak_rss={peak_kb} | ui median {:.3} p95 {:.3} | cpu(ui+tess+gl) median {:.3} p95 {:.3} max {:.3} | interval median {:.3} p95 {:.3} | fps {:.0}",
                self.bench.scenario.name(),
                self.bench.frames,
                self.first_frame_ms.unwrap_or(0.0),
                u.median,
                u.p95,
                c.median,
                c.p95,
                c.max,
                i.median,
                i.p95,
                1000.0 / i.mean.max(1e-9),
            );
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else {
            ctx.request_repaint();
        }
    }
}

// ---------------------------------------------------------------- idle check

/// `SPIRE_IDLE_CHECK=<seconds>`: runs the normal app with no input and
/// reports how many frames it drew. A reactive UI should draw only the
/// first frame or two; anything more means something requests repaints.
pub struct IdleCheckApp {
    app: SpireApp,
    frames: u64,
    started: Option<Instant>,
    seconds: f64,
}

impl IdleCheckApp {
    pub fn new(app: SpireApp, seconds: f64) -> Self {
        Self { app, frames: 0, started: None, seconds }
    }
}

impl eframe::App for IdleCheckApp {
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        crate::style::clear_color(visuals)
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.frames += 1;
        let ctx = ui.ctx().clone();
        if self.started.is_none() {
            self.started = Some(Instant::now());
            let seconds = self.seconds;
            let waker = ctx.clone();
            // Wake once at the end to report; this adds exactly one frame.
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs_f64(seconds));
                waker.request_repaint();
            });
        }
        self.app.frame(ui);
        if self.started.is_some_and(|s| s.elapsed().as_secs_f64() >= self.seconds) {
            println!(
                "idle-check seconds={:.1} frames={} (1 startup + 1 report = 2 is ideal)",
                self.seconds, self.frames
            );
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

//! Data window: what data is loaded, and the manual update button.

use eframe::egui::{self, RichText};
use spire_data::codex::Channel;
use spire_data::fetch::Source;
use spire_data::store::{Settings, Store};

use crate::actions::Action;
use crate::app::{UiState, ViewCtx};
use crate::style::color::*;
use crate::widgets::{eyebrow, hairline, primary_button, segmented};

/// Settings where the platform keeps it: a window of its own on macOS
/// (opened from the app menu, ⌘,), a page of the main window on Windows'
/// System look (drawn by `views::root` through [`contents`]), and an
/// in-app window elsewhere.
pub fn window(
    ctx: &egui::Context,
    v: &ViewCtx,
    store: &Store,
    settings: &Settings,
    st: &mut UiState,
    out: &mut Vec<Action>,
) {
    match crate::style::current().native {
        Some(crate::style::Native::Windows) => {}
        Some(crate::style::Native::Mac) => viewport(ctx, v, store, settings, st, out),
        None => {
            let mut open = true;
            egui::Window::new(RichText::new("Settings").family(crate::style::strong_family()))
                .open(&mut open)
                .default_width(520.0)
                .resizable(false)
                .collapsible(false)
                .show(ctx, |ui| contents(ui, v, store, settings, st, out));
            if !open {
                out.push(Action::ToggleDataWindow);
            }
        }
    }
}

/// macOS: a separate Settings window titled for the app, without zoom or
/// minimise (Apple's Settings window conventions). Closing it, or Esc,
/// puts it away.
fn viewport(
    ctx: &egui::Context,
    v: &ViewCtx,
    store: &Store,
    settings: &Settings,
    st: &mut UiState,
    out: &mut Vec<Action>,
) {
    let builder = egui::ViewportBuilder::default()
        .with_title("Spire Dashboard Settings")
        .with_inner_size([560.0, 640.0])
        .with_resizable(false)
        .with_minimize_button(false)
        .with_maximize_button(false);
    ctx.show_viewport_immediate(egui::ViewportId::from_hash_of("settings"), builder, |ui, _class| {
        let frame = egui::Frame::new().fill(bg()).inner_margin(egui::Margin::symmetric(20, 16));
        egui::CentralPanel::default().frame(frame).show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| contents(ui, v, store, settings, st, out));
        });
        let closing = ui.ctx().input(|i| i.viewport().close_requested() || i.key_pressed(egui::Key::Escape));
        if closing {
            out.push(Action::ToggleDataWindow);
        }
    });
}

/// The Settings contents, wherever they're shown.
pub fn contents(
    ui: &mut egui::Ui,
    v: &ViewCtx,
    store: &Store,
    settings: &Settings,
    st: &mut UiState,
    out: &mut Vec<Action>,
) {
    {
        {
            ui.label(eyebrow("Appearance"));
            hairline(ui);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let current = crate::style::current().name;
                for (name, label, blurb) in crate::style::looks() {
                    let selected = name == current;
                    let label = RichText::new(label).color(if selected { on_accent() } else { text() });
                    let button = egui::Button::new(label)
                        .fill(if selected { accent() } else { egui::Color32::TRANSPARENT })
                        .stroke(egui::Stroke::new(1.0, if selected { accent() } else { border() }));
                    if ui.add(button).on_hover_text(blurb).clicked() && !selected {
                        out.push(Action::SetLook(name));
                    }
                }
            });
            ui.label(RichText::new(crate::style::current().blurb).small().color(text_dim()));
            // The System look follows light and dark itself.
            let system = crate::style::is_system();
            let mut follow = settings.match_system;
            if !system && ui.checkbox(&mut follow, "Match system light/dark").changed() {
                out.push(Action::SetMatchSystem(follow));
            }
            if settings.match_system && !system {
                let name = |n: Option<&str>, d: &str| {
                    let n = n.unwrap_or(d);
                    crate::style::find_look(n).map_or(n.to_string(), |l| l.label.to_string())
                };
                ui.label(
                    RichText::new(format!(
                        "Light: {} · Dark: {}. Pick a look above to change the one for its mode.",
                        name(settings.light_look.as_deref(), "grimoire"),
                        name(settings.dark_look.as_deref(), crate::style::DEFAULT_LOOK)
                    ))
                    .small()
                    .color(text_faint()),
                );
            }
            ui.add_space(crate::style::space::xs());
            let mut large = crate::style::large();
            if ui.checkbox(&mut large, "Larger text").changed() {
                out.push(Action::SetLargeText(large));
            }
            ui.label(
                RichText::new(
                    "Bigger type, roomier spacing, filled chips and brighter secondary text. Works with any look.",
                )
                .small()
                .color(text_faint()),
            );
            // macOS keeps its own title bar with the System look.
            if !crate::chrome::unified() {
                let mut system = !crate::chrome::custom();
                if ui.checkbox(&mut system, "Use the system title bar").changed() {
                    out.push(Action::SetSystemTitleBar(system));
                }
                ui.label(
                    RichText::new(
                        "Off: the header doubles as the title bar, so desktop themes can't restyle or hide it.",
                    )
                    .small()
                    .color(text_faint()),
                );
            }
            if crate::style::is_system() && cfg!(windows) {
                ui.label(
                    RichText::new("Mica shows behind the window with the system title bar, from the next start.")
                        .small()
                        .color(text_faint()),
                );
            }
            ui.add_space(crate::style::space::s());

            monitored(ui, settings, st, v.following, out);
            ui.add_space(crate::style::space::s());

            ui.label(eyebrow("Game data"));
            hairline(ui);
            for (i, l) in v.catalog.layers.iter().enumerate() {
                let role = if i == 0 {
                    "Bundled"
                } else if l.source == "user" {
                    "Your overrides"
                } else {
                    "Downloaded"
                };
                ui.horizontal(|ui| {
                    ui.label(RichText::new(role).strong().color(text()));
                    if let Some(g) = &l.game_version {
                        ui.label(RichText::new(g).small().color(text_dim()));
                    }
                    if let Some(t) = &l.fetched_at {
                        ui.label(RichText::new(t).small().color(text_faint()));
                    }
                });
                ui.label(RichText::new(&l.summary).small().color(text_dim()));
                ui.add_space(crate::style::space::xs());
            }

            ui.add_space(crate::style::space::s());
            ui.label(eyebrow("Update"));
            hairline(ui);
            if let Source::SpireCodex { channel, .. } = &settings.source {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Game branch").color(text_dim()));
                    let mut c = *channel;
                    if segmented(ui, &mut c, &[(Channel::Beta, "Beta"), (Channel::Stable, "Stable")]) {
                        out.push(Action::SetChannel(c));
                    }
                });
            }
            ui.horizontal(|ui| {
                ui.label(RichText::new("Source").color(text_dim()));
                let r = ui.add(egui::TextEdit::singleline(&mut st.source_url).desired_width(f32::INFINITY));
                if r.lost_focus() {
                    out.push(Action::SetSourceUrl(st.source_url.trim().to_string()));
                }
            });
            ui.label(
                RichText::new("A base URL reads the Spire Codex API; a URL ending in .json reads a data pack.")
                    .small()
                    .color(text_faint()),
            );
            ui.add_space(crate::style::space::s());
            let label = if st.updating { "Updating…" } else { "Update data now" };
            if primary_button(ui, label, !st.updating).clicked() {
                out.push(Action::UpdateData);
            }
            ui.horizontal(|ui| {
                if ui.button("Revert to bundled").clicked() {
                    out.push(Action::RevertData);
                }
                ui.label(RichText::new("Nothing updates unless you press Update.").small().color(text_faint()));
            });

            ui.add_space(crate::style::space::s());
            ui.label(eyebrow("Playbook"));
            hairline(ui);
            ui.label(
                RichText::new(
                    "Role rules, act targets and event tags live in editable JSON. Export, edit, then reload.",
                )
                .small()
                .color(text_dim()),
            );
            ui.horizontal(|ui| {
                if ui.button("Export playbook").clicked() {
                    out.push(Action::ExportPlaybook);
                }
                if ui.button("Reload from disk").clicked() {
                    out.push(Action::ReloadData);
                }
            });

            ui.add_space(crate::style::space::s());
            ui.label(RichText::new(format!("Data folder: {}", store.dir().display())).small().color(text_faint()));
            if !st.messages.is_empty() {
                ui.add_space(crate::style::space::xs());
                for m in st.messages.iter().rev() {
                    ui.label(RichText::new(m).small().color(text_dim()));
                }
            }
        }
    }
}

/// Monitored runs: follow the game through the companion mod.
fn monitored(ui: &mut egui::Ui, settings: &Settings, st: &UiState, following: bool, out: &mut Vec<Action>) {
    ui.label(eyebrow("Monitored runs"));
    hairline(ui);
    let mut on = settings.follow_game;
    if ui.checkbox(&mut on, "Follow the game").changed() {
        out.push(Action::SetFollowGame(on));
    }
    ui.label(
        RichText::new(
            "Floors log themselves as you play. Needs the Spire Dashboard Link mod in the game's mods \
             folder (see the README). It only reads.",
        )
        .small()
        .color(text_faint()),
    );
    if !settings.follow_game {
        return;
    }
    let f = &st.follow;
    match &f.link {
        None if f.no_link => {
            ui.label(RichText::new("Waiting for the mod: start the game with mods enabled.").small().color(warn()));
        }
        None => {
            ui.label(RichText::new("Looking for the game…").small().color(text_dim()));
        }
        Some(link) => {
            let game = link.game_version.as_deref().unwrap_or("unknown version");
            let line = format!("Connected: Link {} · game {game}", link.mod_version);
            ui.label(RichText::new(line).small().color(good()));
            let run = match (link.in_progress, following, f.waiting) {
                (true, true, _) => "Following the game's run.",
                (true, false, _) if f.conflict.is_some() => "The game is on another run: see the banner above.",
                (true, false, _) => "A run is in progress.",
                (false, _, true) => "No run in progress. The next run you start will be followed.",
                (false, ..) => "No run in progress.",
            };
            ui.label(RichText::new(run).small().color(text_dim()));
            if let Some(p) = &f.problem {
                ui.label(RichText::new(p).small().color(warn()));
                if let Some(path) = link.run_save.as_ref().or(link.save_dir.as_ref()) {
                    ui.label(RichText::new(path.display().to_string()).small().color(text_faint()));
                }
            }
            if link.modded_profile() {
                ui.label(
                    RichText::new(
                        "With mods on, the game plays in its separate modded profile (its own unlocks and history).",
                    )
                    .small()
                    .color(text_faint()),
                );
            }
        }
    }
}

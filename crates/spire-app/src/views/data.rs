//! Data window: what data is loaded, and the manual update button.

use eframe::egui::{self, RichText};
use spire_data::codex::Channel;
use spire_data::fetch::Source;
use spire_data::store::{Settings, Store};

use crate::actions::Action;
use crate::app::{UiState, ViewCtx};
use crate::style::color::*;
use crate::widgets::{eyebrow, hairline, primary_button, segmented};

pub fn window(
    ctx: &egui::Context,
    v: &ViewCtx,
    store: &Store,
    settings: &Settings,
    st: &mut UiState,
    out: &mut Vec<Action>,
) {
    let mut open = true;
    egui::Window::new(RichText::new("Game data").strong())
        .open(&mut open)
        .default_width(520.0)
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.label(eyebrow("Loaded"));
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
                        ui.label(RichText::new(g).small().color(accent()));
                    }
                    if let Some(t) = &l.fetched_at {
                        ui.label(RichText::new(t).small().color(text_faint()));
                    }
                });
                ui.label(RichText::new(&l.summary).small().color(text_dim()));
                ui.add_space(4.0);
            }

            ui.add_space(8.0);
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
            ui.add_space(6.0);
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

            ui.add_space(8.0);
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

            ui.add_space(8.0);
            ui.label(RichText::new(format!("Data folder: {}", store.dir().display())).small().color(text_faint()));
            if !st.messages.is_empty() {
                ui.add_space(4.0);
                for m in st.messages.iter().rev() {
                    ui.label(RichText::new(m).small().color(text_dim()));
                }
            }
        });
    if !open {
        out.push(Action::ToggleDataWindow);
    }
}

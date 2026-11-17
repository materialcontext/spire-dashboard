//! Data window: what data is loaded, and the manual update button.

use eframe::egui::{self, RichText};
use spire_data::store::{Settings, Store};

use crate::actions::Action;
use crate::app::{UiState, ViewCtx};

pub fn window(
    ctx: &egui::Context,
    v: &ViewCtx,
    store: &Store,
    settings: &Settings,
    st: &mut UiState,
    out: &mut Vec<Action>,
) {
    let mut open = true;
    egui::Window::new("Game data").open(&mut open).default_width(520.0).show(ctx, |ui| {
        ui.label(RichText::new("Loaded layers (later overrides earlier)").strong());
        for l in &v.catalog.layers {
            ui.label(format!(
                "• {}{}{}",
                l.source,
                l.game_version.as_ref().map_or(String::new(), |g| format!(" · game {g}")),
                l.fetched_at.as_ref().map_or(String::new(), |t| format!(" · fetched {t}")),
            ));
            ui.label(RichText::new(format!("   {}", l.summary)).small().weak());
        }

        ui.add_space(8.0);
        ui.label(RichText::new("Update").strong());
        ui.label(format!("Source: {}", settings.source.describe()));
        ui.horizontal(|ui| {
            ui.label("URL");
            let r = ui.add(egui::TextEdit::singleline(&mut st.source_url).desired_width(300.0));
            if r.lost_focus() {
                out.push(Action::SetSourceUrl(st.source_url.trim().to_string()));
            }
        });
        ui.label(
            RichText::new("A base URL uses the Spire Codex API; a URL ending in .json is read as a data pack.")
                .small()
                .weak(),
        );
        ui.horizontal(|ui| {
            let label = if st.updating { "Updating…" } else { "Update data now" };
            if ui.add_enabled(!st.updating, egui::Button::new(label)).clicked() {
                out.push(Action::UpdateData);
            }
            if ui.button("Revert to bundled").clicked() {
                out.push(Action::RevertData);
            }
        });

        ui.add_space(8.0);
        ui.label(RichText::new("Playbook").strong());
        ui.label(
            RichText::new("Role rules, act targets and event tags are editable JSON. Export it, edit, then reload.")
                .small()
                .weak(),
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
        ui.label(RichText::new(format!("Data folder: {}", store.dir().display())).small());
        for m in st.messages.iter().rev() {
            ui.label(RichText::new(m).small().weak());
        }
    });
    if !open {
        out.push(Action::ToggleDataWindow);
    }
}

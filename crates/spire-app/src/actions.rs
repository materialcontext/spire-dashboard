//! Everything the UI can ask for. Views only read state and push actions;
//! [`crate::app::SpireApp::apply`] is the one place that changes anything.

use spire_core::ids::*;
use spire_core::run::{Change, RunEvent};

#[derive(Clone, Debug)]
pub enum Action {
    NewRun {
        character: CharacterId,
        ascension: u32,
    },
    /// Abandon the current run (filed in history as Abandoned).
    AbandonRun,
    EndRun(spire_core::run::Outcome),
    EnterAct {
        act: ActId,
        index: u8,
        boss: Option<EncounterId>,
    },
    Record(RunEvent),
    Undo,
    /// Queue a change on the node being logged.
    Draft(Change),
    UpdateData,
    RevertData,
    ExportPlaybook,
    ReloadData,
    SetSourceUrl(String),
    SetChannel(spire_data::codex::Channel),
    SetLook(&'static str),
    /// Set (Some) or clear (None) your own role override for a card.
    SetPersonalRoles {
        card: CardId,
        roles: Option<Vec<spire_core::playbook::Role>>,
    },
    SetLargeText(bool),
    ToggleDataWindow,
}

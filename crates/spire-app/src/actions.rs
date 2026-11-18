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
    /// Follow the system's light/dark mode.
    SetMatchSystem(bool),
    /// Use the system title bar (true) or the app's own (false).
    SetSystemTitleBar(bool),
    ToggleDataWindow,
    /// Jump to a catalog entry.
    Open(Link),
    DismissToast,
    /// Replace a run in history with an edited copy.
    SaveHistory {
        id: String,
        run: Box<spire_core::Run>,
    },
    /// Delete a run from history for good.
    DeleteHistory(String),
    /// Show a native context menu (macOS) and apply what's picked.
    ContextMenu(Vec<MenuEntry>),
}

/// A context-menu entry and what picking it does.
#[derive(Clone, Debug)]
pub enum MenuEntry {
    Item(String, Action),
    Note(String),
    Submenu(String, Vec<MenuEntry>),
    Separator,
}

impl MenuEntry {
    /// The platform menu, and each item's action in the same order.
    pub fn split(entries: &[MenuEntry]) -> (Vec<spire_platform::menu::Entry>, Vec<Action>) {
        use spire_platform::menu::Entry;
        let mut actions = Vec::new();
        fn walk(es: &[MenuEntry], actions: &mut Vec<Action>) -> Vec<Entry> {
            es.iter()
                .map(|e| match e {
                    MenuEntry::Item(label, a) => {
                        actions.push(a.clone());
                        Entry::Item(label.clone())
                    }
                    MenuEntry::Note(label) => Entry::Note(label.clone()),
                    MenuEntry::Submenu(label, children) => Entry::Submenu(label.clone(), walk(children, actions)),
                    MenuEntry::Separator => Entry::Separator,
                })
                .collect()
        }
        let menu = walk(entries, &mut actions);
        (menu, actions)
    }
}

/// Something a name in the UI can link to.
#[derive(Clone, Debug, PartialEq)]
pub enum Link {
    Card(CardId),
    Encounter(EncounterId),
    Relic(RelicId),
}

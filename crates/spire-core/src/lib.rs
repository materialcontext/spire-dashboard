//! Domain core for the StS2 macro dashboard.
//!
//! Pure functions over immutable data: no IO, no UI, no network. Three ideas:
//!
//! - [`catalog`]: game facts (cards, monsters, acts…), layered from data packs.
//! - [`playbook`]: strategy knowledge as data (role rules, act plans).
//! - [`run`]: the player's run as an event log, folded into a state.
//!
//! [`analysis`] combines the three into what the dashboard shows.

pub mod analysis;
pub mod catalog;
pub mod ids;
pub mod mechanics;
pub mod playbook;
pub mod run;

pub use catalog::{Catalog, DataPack};
pub use ids::*;
pub use playbook::Playbook;
pub use run::{Run, RunEvent, RunState};

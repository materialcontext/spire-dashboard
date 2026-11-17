//! Where the run is within the act, and which room the next floor must be.

use crate::catalog::{Catalog, NodeKind};
use crate::run::RunState;

/// Floors in the current act: the measured length, plus one extra boss
/// floor in a double-boss act.
pub fn act_length(state: &RunState, catalog: &Catalog) -> Option<u32> {
    let act = catalog.acts.get(state.act.as_ref()?)?;
    let extra = u32::from(catalog.rules.double_boss(state.ascension, state.act_index));
    act.length.map(|l| l + extra)
}

/// The room the next floor always has, if the act's layout fixes it.
///
/// Fixed rooms are measured from community runs (e.g. every act opens with
/// an Ancient; a Rest Site always precedes the boss). In a double-boss act
/// the extra boss floor sits between the Rest Site and the last boss.
pub fn expected_room(state: &RunState, catalog: &Catalog) -> Option<NodeKind> {
    let act = catalog.acts.get(state.act.as_ref()?)?;
    let next = state.act_floor as i32;
    let double = catalog.rules.double_boss(state.ascension, state.act_index);
    if let Some(f) = act.fixed_rooms.iter().find(|f| f.at >= 0 && f.at == next) {
        return Some(f.room);
    }
    let length = act_length(state, catalog)? as i32;
    let from_end = next - length; // -1 = last floor
    if double && from_end == -2 {
        return Some(NodeKind::Boss);
    }
    // In a double-boss act, floors before the extra boss shift by one.
    let lookup = if double && from_end < -2 { from_end + 1 } else { from_end };
    act.fixed_rooms.iter().find(|f| f.at < 0 && f.at == lookup).map(|f| f.room)
}

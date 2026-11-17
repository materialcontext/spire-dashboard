//! Keyboard accelerators drive the same actions as the mouse.

use eframe::egui::{self, Event, Key, Modifiers};
use spire_app::bench::Scenario;
use spire_core::run::NodeKind;

fn press(key: Key, modifiers: Modifiers) -> Vec<Event> {
    vec![Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }]
}

#[test]
fn keys_pick_rooms_commit_and_undo() {
    let s = Scenario::Idle;
    let ctx = egui::Context::default();
    let mut app = s.app();
    let floor = app.run().state().floor;
    let mut frame = |events: Vec<Event>, app: &mut spire_app::app::SpireApp| {
        let mut input = s.raw_input(0, 0.0);
        input.events = events;
        ctx.run_ui(input, |ui| app.frame(ui)).textures_delta.clear();
    };
    frame(vec![], &mut app);

    frame(press(Key::Num3, Modifiers::NONE), &mut app);
    assert_eq!(app.draft().kind, NodeKind::ALL[2], "3 picks the third room");

    frame(press(Key::Enter, Modifiers::COMMAND), &mut app);
    assert_eq!(app.run().state().floor, floor + 1, "Ctrl+Enter commits the floor");

    frame(press(Key::Z, Modifiers::COMMAND), &mut app);
    assert_eq!(app.run().state().floor, floor, "Ctrl+Z undoes it");
}

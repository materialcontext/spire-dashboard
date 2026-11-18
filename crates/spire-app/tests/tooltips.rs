//! A tooltip that grows from one frame to the next stays inside the window.

use eframe::egui::{self, Pos2, RawInput, Rect, pos2, vec2};

fn frame(ctx: &egui::Context, text: &str, fix: bool) -> Option<Rect> {
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0))),
        events: vec![egui::Event::PointerMoved(pos2(390.0, 150.0))],
        ..Default::default()
    };
    let mut id = None;
    let mut out = ctx.run_ui(input, |ui| {
        let r = ui.allocate_rect(ui.max_rect(), egui::Sense::hover());
        let tip = egui::Tooltip::always_open(ui.ctx().clone(), r.layer_id, r.id, egui::PopupAnchor::Pointer);
        if let Some(shown) = tip.show(|ui| ui.label(text)) {
            id = Some(shown.response.layer_id.id);
        }
        if fix {
            spire_app::chrome::keep_tooltips_inside(ui.ctx());
        }
    });
    out.textures_delta.clear(); // no GPU here
    ctx.memory(|m| m.area_rect(id?))
}

fn grows(fix: bool) -> Rect {
    let ctx = egui::Context::default();
    for _ in 0..3 {
        frame(&ctx, "Floor 9", fix);
    }
    frame(&ctx, "Floor 10\nDamage taken: 1234 (most this act)", fix).expect("tooltip shown")
}

#[test]
fn a_growing_tooltip_stays_inside_the_window() {
    let window = Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0));
    // Without the check egui places it for last frame's size: it spills.
    assert!(!window.contains_rect(grows(false)), "the case this guards against should reproduce");
    let fixed = grows(true);
    assert!(window.expand(0.5).contains_rect(fixed), "{fixed:?}");
}

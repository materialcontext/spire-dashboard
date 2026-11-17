//! Every character the app can show must exist in the bundled fonts;
//! a missing glyph renders as an empty box.

use eframe::egui;

/// Symbols used in UI strings (bullets, arrows, separators, ellipses).
const UI_SYMBOLS: &str = "•×–…·%+";

#[test]
fn bundled_fonts_cover_ui_and_game_text() {
    let ctx = egui::Context::default();
    ctx.run_ui(egui::RawInput::default(), |_| {}).textures_delta.clear();
    let pack = spire_data::bundled_pack();
    let mut text: String = UI_SYMBOLS.to_string();
    for c in &pack.cards {
        text.push_str(&c.name);
        text.push_str(&c.text);
    }
    for r in &pack.relics {
        text.push_str(&r.name);
        text.push_str(&r.text);
    }
    for p in &pack.potions {
        text.push_str(&p.name);
        text.push_str(&p.text);
    }
    for e in &pack.events {
        text.push_str(&e.name);
        for o in &e.options {
            text.push_str(&o.title);
            text.push_str(&o.text);
        }
    }
    for m in &pack.monsters {
        text.push_str(&m.name);
    }
    let mut chars: Vec<char> = text.chars().filter(|c| !c.is_control()).collect();
    chars.sort();
    chars.dedup();
    let font = egui::FontId::proportional(14.0);
    let missing: String = ctx.fonts_mut(|f| chars.iter().copied().filter(|c| !f.has_glyph(&font, *c)).collect());
    assert!(missing.is_empty(), "no glyph for: {missing:?}");
}

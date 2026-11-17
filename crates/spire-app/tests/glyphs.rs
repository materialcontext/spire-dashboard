//! Every character the app can show must exist in the bundled fonts;
//! a missing glyph renders as an empty box.

use eframe::egui;

/// Every non-ASCII character inside string literals in the UI source, so a
/// new symbol in a label is checked without anyone remembering to list it.
fn ui_symbols() -> String {
    fn walk(dir: &std::path::Path, out: &mut String) {
        for entry in std::fs::read_dir(dir).expect("read src dir").flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let src = std::fs::read_to_string(&path).expect("read source");
                for line in src.lines() {
                    let code = line.split("//").next().unwrap_or("");
                    let mut in_str = false;
                    for c in code.chars() {
                        if c == '"' {
                            in_str = !in_str;
                        } else if in_str && !c.is_ascii() {
                            out.push(c);
                        }
                    }
                }
            }
        }
    }
    let mut out = String::new();
    walk(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut out);
    out
}

#[test]
fn bundled_fonts_cover_ui_and_game_text() {
    let ctx = egui::Context::default();
    ctx.run_ui(egui::RawInput::default(), |_| {}).textures_delta.clear();
    let pack = spire_data::bundled_pack();
    let mut text: String = ui_symbols();
    assert!(text.contains('·'), "scanner should find the UI's separators");
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

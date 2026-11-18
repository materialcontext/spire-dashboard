//! Headless frame benchmark: times egui's layout pass (`run_ui`) and paint
//! (tessellation) for every scenario, plus startup. No window or GPU needed.
//!
//! ```text
//! cargo run --release -p spire-app --example frame_bench -- [--frames N] [--scenario NAME] [--ppp 1.0] [--json PATH] [--no-startup]
//! taskset -c 0 cargo run ...   # one core, to approximate minimal hardware
//! ```

use std::time::Instant;

use eframe::egui;
use spire_app::bench::{Scenario, stats};

struct Row {
    name: &'static str,
    ui: Vec<f64>,
    tess: Vec<f64>,
    shapes: usize,
    prims: usize,
    verts: usize,
    indices: usize,
    /// Measured frames that uploaded texture data (font atlas growth).
    tex_frames: usize,
}

/// The look under test: `SPIRE_LOOK` (with `SPIRE_SYSTEM=windows|macos`
/// for the System look on any OS), else the default theme.
fn style(ctx: &egui::Context) {
    match std::env::var("SPIRE_LOOK") {
        Ok(look) => spire_app::style::set_look(ctx, &look),
        Err(_) => spire_app::style::configure(ctx),
    }
}

fn ms(d: std::time::Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn run(scenario: Scenario, frames: usize, ppp: f32) -> Row {
    let ctx = egui::Context::default();
    style(&ctx);
    ctx.set_pixels_per_point(ppp);
    let mut app = scenario.app();
    let mut time = 0.0;
    let warmup = 30;
    let mut row = Row {
        name: scenario.name(),
        ui: vec![],
        tess: vec![],
        shapes: 0,
        prims: 0,
        verts: 0,
        indices: 0,
        tex_frames: 0,
    };
    for i in 0..warmup + frames {
        time += 1.0 / 60.0;
        let mut input = scenario.raw_input(i, time);
        input.viewports.entry(egui::ViewportId::ROOT).or_default().native_pixels_per_point = Some(ppp);
        let t = Instant::now();
        let out = ctx.run_ui(input, |ui| {
            scenario.before_frame(&mut app, i, ui.ctx());
            app.frame(ui);
        });
        let ui_ms = ms(t.elapsed());
        let mut out = out;
        let shapes = out.shapes.len();
        let tex = !out.textures_delta.set.is_empty();
        out.textures_delta.clear(); // no GPU here; drop uploads deliberately
        let t = Instant::now();
        let prims = ctx.tessellate(out.shapes, out.pixels_per_point);
        let tess_ms = ms(t.elapsed());
        if i >= warmup {
            row.ui.push(ui_ms);
            row.tex_frames += usize::from(tex);
            row.tess.push(tess_ms);
            row.shapes = row.shapes.max(shapes);
            row.prims = row.prims.max(prims.len());
            let (v, ix) = prims.iter().fold((0, 0), |(v, ix), p| match &p.primitive {
                egui::epaint::Primitive::Mesh(m) => (v + m.vertices.len(), ix + m.indices.len()),
                egui::epaint::Primitive::Callback(_) => (v, ix),
            });
            row.verts = row.verts.max(v);
            row.indices = row.indices.max(ix);
        }
    }
    row
}

fn startup(reps: usize) -> (f64, f64) {
    let mut load = Vec::new();
    let mut first = Vec::new();
    for _ in 0..reps {
        let t = Instant::now();
        let mut app = Scenario::IdleLate.app();
        load.push(ms(t.elapsed()));
        let ctx = egui::Context::default();
        style(&ctx);
        let t = Instant::now();
        let mut out = ctx.run_ui(Scenario::IdleLate.raw_input(0, 0.0), |ui| app.frame(ui));
        out.textures_delta.clear();
        let _ = ctx.tessellate(out.shapes, out.pixels_per_point);
        first.push(ms(t.elapsed()));
    }
    (stats(&load).median, stats(&first).median)
}

/// Vertices per shape kind for one frame of a scenario (`--shapes NAME`).
fn shape_breakdown(scenario: Scenario) {
    let ctx = egui::Context::default();
    style(&ctx);
    let mut app = scenario.app();
    let mut out = None;
    for i in 0..40 {
        let mut o = ctx.run_ui(scenario.raw_input(i, i as f64 / 60.0), |ui| {
            scenario.before_frame(&mut app, i, ui.ctx());
            app.frame(ui);
        });
        o.textures_delta.clear();
        out = Some(o);
    }
    let out = out.expect("ran frames");
    let mut kinds: std::collections::BTreeMap<String, (usize, usize, f64)> = Default::default();
    fn kind(s: &egui::Shape) -> String {
        match s {
            egui::Shape::Rect(r) => format!(
                "rect{}{}{}",
                if r.fill.a() > 0 { " fill" } else { "" },
                if r.stroke.width > 0.0 { " stroke" } else { "" },
                if r.corner_radius.is_same() && r.corner_radius.nw > 0 { " rounded" } else { "" }
            ),
            egui::Shape::Text(_) => "text".into(),
            egui::Shape::LineSegment { .. } => "line".into(),
            egui::Shape::Path(_) => "path".into(),
            egui::Shape::Mesh(_) => "mesh".into(),
            egui::Shape::Vec(_) => "vec".into(),
            egui::Shape::Circle(_) => "circle".into(),
            other => format!("{:?}", std::mem::discriminant(other)),
        }
    }
    for cs in out.shapes {
        let k = kind(&cs.shape);
        let prims = ctx.tessellate(vec![cs], 1.0);
        let (mut v, mut area) = (0usize, 0f64);
        for p in &prims {
            if let egui::epaint::Primitive::Mesh(m) = &p.primitive {
                v += m.vertices.len();
                // Pixels the rasteriser shades: sum of triangle areas (clipped
                // coarsely to the primitive's clip rect).
                for t in m.indices.as_chunks::<3>().0 {
                    let [a, b, c] = [0, 1, 2].map(|i| m.vertices[t[i] as usize].pos);
                    let tri = ((b - a).x * (c - a).y - (b - a).y * (c - a).x).abs() as f64 / 2.0;
                    let bb = egui::Rect::from_points(&[a, b, c]);
                    let vis = bb.intersect(p.clip_rect);
                    let frac = if bb.area() > 0.0 { (vis.area().max(0.0) / bb.area()) as f64 } else { 0.0 };
                    area += tri * frac;
                }
            }
        }
        let e = kinds.entry(k).or_default();
        e.0 += 1;
        e.1 += v;
        e.2 += area;
    }
    let total: f64 = kinds.values().map(|k| k.2).sum();
    for (k, (n, v, a)) in &kinds {
        println!("{k:<28} shapes {n:>5}  verts {v:>7}  px {:>9.0} ({:>4.1}%)", a, 100.0 * a / total.max(1.0));
    }
    println!("total shaded px {total:.0} = {:.2}x window", total / f64::from(scenario.size(0).x * scenario.size(0).y));
}

/// Times the non-UI work behind a run change (`--components`).
fn components() {
    use spire_core::analysis::Dashboard;
    let store_dir = std::env::temp_dir().join(format!("spire-bench-components-{}", std::process::id()));
    let store = spire_data::store::Store::at(&store_dir);
    let loaded = store.load();
    let run = spire_app::bench::late_run(&loaded.catalog);
    let time = |name: &str, f: &mut dyn FnMut()| {
        let samples: Vec<f64> = (0..200)
            .map(|_| {
                let t = Instant::now();
                f();
                ms(t.elapsed())
            })
            .collect();
        let s = stats(&samples);
        println!("{name:<28} median {:.4} ms  p95 {:.4} ms", s.median, s.p95);
    };
    let state = run.state();
    time("replay run log", &mut || {
        std::hint::black_box(run.state());
    });
    time("Dashboard::compute", &mut || {
        std::hint::black_box(Dashboard::compute(&state, &loaded.catalog, &loaded.playbook));
    });
    time("serialize run (pretty)", &mut || {
        std::hint::black_box(serde_json::to_string_pretty(&run).unwrap());
    });
    time("save run (write+rename)", &mut || store.save_run(&run).unwrap());
    time("store.load (all layers)", &mut || {
        std::hint::black_box(store.load());
    });
    let _ = std::fs::remove_dir_all(store_dir);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned();
    let frames: usize = arg("--frames").and_then(|s| s.parse().ok()).unwrap_or(300);
    let ppp: f32 = arg("--ppp").and_then(|s| s.parse().ok()).unwrap_or(1.0);
    let only = arg("--scenario").and_then(|s| Scenario::parse(&s));
    if args.iter().any(|a| a == "--components") {
        components();
        return;
    }
    if let Some(s) = arg("--shapes").and_then(|s| Scenario::parse(&s)) {
        shape_breakdown(s);
        return;
    }

    let (load, first) = if args.iter().any(|a| a == "--no-startup") { (0.0, 0.0) } else { startup(5) };
    println!("startup: data load + analysis {load:.2} ms · first frame (fonts, layout, tessellate) {first:.2} ms");
    println!(
        "{:<12} {:>8} {:>8} {:>8} | {:>8} {:>8} | {:>8} | {:>6} {:>5} {:>7} {:>7} {:>4}",
        "scenario",
        "ui med",
        "ui p95",
        "ui max",
        "tess med",
        "tess p95",
        "sum med",
        "shapes",
        "prims",
        "verts",
        "indices",
        "tex"
    );
    let mut json = Vec::new();
    for s in Scenario::ALL.into_iter().filter(|s| only.is_none_or(|o| o == *s)) {
        let r = run(s, frames, ppp);
        let (u, t) = (stats(&r.ui), stats(&r.tess));
        let sum: Vec<f64> = r.ui.iter().zip(&r.tess).map(|(a, b)| a + b).collect();
        let sm = stats(&sum);
        println!(
            "{:<12} {:>8.3} {:>8.3} {:>8.3} | {:>8.3} {:>8.3} | {:>8.3} | {:>6} {:>5} {:>7} {:>7} {:>4}",
            r.name,
            u.median,
            u.p95,
            u.max,
            t.median,
            t.p95,
            sm.median,
            r.shapes,
            r.prims,
            r.verts,
            r.indices,
            r.tex_frames
        );
        json.push(format!(
            r#"{{"scenario":"{}","ui_median":{:.4},"ui_p95":{:.4},"ui_max":{:.4},"tess_median":{:.4},"tess_p95":{:.4},"shapes":{},"prims":{},"verts":{},"indices":{}}}"#,
            r.name, u.median, u.p95, u.max, t.median, t.p95, r.shapes, r.prims, r.verts, r.indices
        ));
    }
    if let Some(path) = arg("--json") {
        let body = format!(
            "{{\"startup_load_ms\":{load:.3},\"first_frame_ms\":{first:.3},\"ppp\":{ppp},\"scenarios\":[{}]}}\n",
            json.join(",")
        );
        std::fs::write(&path, body).expect("write json");
    }
}

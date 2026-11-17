//! StS2 macro dashboard.

use spire_app::{app::SpireApp, bench::RenderBench};

fn main() -> eframe::Result {
    let _ = spire_app::bench::PROCESS_START.set(std::time::Instant::now());
    // `SPIRE_BENCH=<scenario>[:frames]` renders a scenario continuously,
    // prints frame timings, and exits. See `bench.rs`.
    let bench = std::env::var("SPIRE_BENCH").ok().and_then(|s| RenderBench::parse(&s));
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Spire Dashboard")
            .with_inner_size([1400.0, 900.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    let mut options = options;
    // The UI is flat colours with no gradients, so dithering only costs
    // per-pixel shader work.
    options.dithering = false;
    // Benchmarks measure frame cost, so don't wait for the display.
    options.glow_options.vsync = bench.is_none();
    eframe::run_native(
        "spire-dashboard",
        options,
        Box::new(move |cc| {
            // SPIRE_LOOK overrides the saved appearance (for reviews and screenshots).
            let settings = spire_data::store::Store::default_location().settings();
            // SPIRE_LARGE=1 forces Larger text on (screenshots); else the saved choice.
            let large = std::env::var("SPIRE_LARGE").map_or(settings.large_text, |v| v == "1");
            spire_app::style::set_large(&cc.egui_ctx, large);
            let look = std::env::var("SPIRE_LOOK").ok().or(settings.look);
            spire_app::style::set_look(&cc.egui_ctx, look.as_deref().unwrap_or(spire_app::style::DEFAULT_LOOK));
            let idle = std::env::var("SPIRE_IDLE_CHECK").ok().and_then(|s| s.parse::<f64>().ok());
            let app: Box<dyn eframe::App> = match (bench, idle) {
                (Some(b), _) => Box::new(b.into_app()),
                (None, Some(secs)) => Box::new(spire_app::bench::IdleCheckApp::new(
                    SpireApp::from_store(spire_data::store::Store::default_location()),
                    secs,
                )),
                (None, None) => {
                    let mut app = SpireApp::from_store(spire_data::store::Store::default_location());
                    if let Ok(tab) = std::env::var("SPIRE_TAB") {
                        app.open(&tab, std::env::var("SPIRE_SELECT").ok().as_deref());
                    }
                    Box::new(app)
                }
            };
            Ok(app)
        }),
    )
}

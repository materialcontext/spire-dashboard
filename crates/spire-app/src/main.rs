//! StS2 macro dashboard.

use spire_app::{app::SpireApp, bench::RenderBench};

fn main() -> eframe::Result {
    let _ = spire_app::bench::PROCESS_START.set(std::time::Instant::now());
    // `SPIRE_BENCH=<scenario>[:frames]` renders a scenario continuously,
    // prints frame timings, and exits. See `bench.rs`.
    let bench = std::env::var("SPIRE_BENCH").ok().and_then(|s| RenderBench::parse(&s));
    let settings = spire_data::store::Store::default_location().settings();
    // SPIRE_LOOK overrides the saved appearance (for reviews and screenshots).
    let look = std::env::var("SPIRE_LOOK").ok().or(settings.look.clone());
    let look = look.as_deref().unwrap_or(spire_app::style::DEFAULT_LOOK).to_string();
    let system_look = look == spire_app::style::SYSTEM_LOOK && spire_app::style::native().is_some();
    let appearance = spire_app::platform::appearance();
    // The window's frame is decided before it exists (see `chrome.rs`):
    // - macOS, System look: the real title bar, transparent over full-size
    //   content, traffic lights in place, the app's toolbar beside them.
    // - Otherwise the app draws its own title bar unless the player chose
    //   the system one.
    let unified = system_look && spire_app::style::native() == Some(spire_app::style::Native::Mac);
    spire_app::chrome::init(!settings.system_title_bar);
    spire_app::chrome::init_unified(unified);
    // A transparent window so a system material can show: Mica (Windows,
    // System look, system frame) or the sidebar material (macOS, System
    // look, unless Reduce Transparency is on).
    let mica = spire_app::chrome::wants_material(system_look, settings.system_title_bar, &appearance);
    let vibrancy = unified && cfg!(target_os = "macos") && appearance.transparency;
    let transparent = mica || vibrancy;
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title("Spire Dashboard")
        .with_inner_size([1400.0, 900.0])
        .with_min_inner_size([900.0, 600.0])
        .with_decorations(unified || settings.system_title_bar)
        .with_transparent(transparent);
    if unified {
        viewport = viewport.with_fullsize_content_view(true).with_titlebar_shown(false).with_title_shown(false);
    }
    let options = eframe::NativeOptions { viewport, ..Default::default() };
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
            // SPIRE_LARGE=1 forces Larger text on (screenshots); else the saved choice.
            // The system's own preferences: reduced motion always; text size
            // only as the first-run default for Larger text.
            let prefs = spire_app::platform::prefs();
            spire_app::style::set_reduce_motion(&cc.egui_ctx, prefs.reduce_motion);
            let first_run = !spire_data::store::Store::default_location().has_settings();
            let default_large = if first_run { prefs.text_scale > 1.0 } else { settings.large_text };
            let large = std::env::var("SPIRE_LARGE").map_or(default_large, |v| v == "1");
            spire_app::style::set_large(&cc.egui_ctx, large);
            spire_app::style::set_appearance(&cc.egui_ctx, appearance);
            spire_app::style::set_look(&cc.egui_ctx, &look);
            if mica {
                let dark = !spire_app::style::current().light;
                spire_app::chrome::material(spire_app::chrome::Window::Creation(cc), dark);
            }
            if vibrancy {
                spire_app::chrome::sidebar_material(cc);
            }
            // Windows: snap layouts on the app-drawn maximise button.
            if cfg!(windows) {
                spire_app::chrome::install_snap(cc);
            }
            let idle = std::env::var("SPIRE_IDLE_CHECK").ok().and_then(|s| s.parse::<f64>().ok());
            let app: Box<dyn eframe::App> = match (bench, idle) {
                (Some(b), _) => Box::new(b.into_app()),
                (None, Some(secs)) => Box::new(spire_app::bench::IdleCheckApp::new(
                    SpireApp::from_store(spire_data::store::Store::default_location()),
                    secs,
                )),
                (None, None) => {
                    let mut app = SpireApp::from_store(spire_data::store::Store::default_location());
                    // macOS: the native menu bar.
                    let ctx = cc.egui_ctx.clone();
                    let tabs: Vec<&str> = spire_app::views::Tab::ALL.iter().map(|t| t.label()).collect();
                    let menu = spire_app::platform::menu::MenuBar::install(&tabs, cc, move || ctx.request_repaint());
                    app.set_menu(menu);
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

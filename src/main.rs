mod app;
mod apps;
mod clutter;
mod junk;
mod pages;
mod perf;
mod protection;
mod scan;
mod space;
mod theme;

use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    let mut screenshot: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--screenshot" {
            screenshot = args.next().map(PathBuf::from);
            if screenshot.is_none() {
                eprintln!("--screenshot braucht einen Dateinamen");
                std::process::exit(2);
            }
        }
    }

    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title("Blitzblank")
        .with_inner_size([1120.0, 740.0])
        .with_min_inner_size([900.0, 600.0]);

    if screenshot.is_some() {
        viewport = viewport.with_active(false);
        #[cfg(target_os = "windows")]
        {
            viewport = viewport.with_window_level(eframe::egui::WindowLevel::AlwaysOnBottom);
        }
    }

    #[allow(unused_mut)]
    let mut options = eframe::NativeOptions {
        viewport,
        run_and_return: false,
        ..Default::default()
    };

    #[cfg(target_os = "macos")]
    if screenshot.is_some() {
        options.event_loop_builder = Some(Box::new(|builder| {
            use winit::platform::macos::EventLoopBuilderExtMacOS;
            builder.with_activate_ignoring_other_apps(false);
        }));
    }

    eframe::run_native(
        "Blitzblank",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, screenshot)))),
    )
}

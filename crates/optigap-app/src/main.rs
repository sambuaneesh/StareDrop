mod app;
mod benchmark_page;
mod receiver_page;
mod sender_page;
mod settings;
mod ui_components;

use app::OptiGapApp;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "optigap=info,optigap_app=info".into()),
        )
        .init();

    let native_options = eframe::NativeOptions::default();
    if let Err(err) = eframe::run_native(
        "OptiGap (Phase 0/1 MVP)",
        native_options,
        Box::new(|cc| Box::new(OptiGapApp::new(cc))),
    ) {
        eprintln!("Failed to start OptiGap app: {err}");
    }
}

mod app;
mod benchmark_page;
mod receiver_page;
mod sender_page;
mod settings;
mod ui_components;

use app::StareDropApp;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "staredrop=info,staredrop_app=info".into()),
        )
        .init();

    let native_options = eframe::NativeOptions::default();
    if let Err(err) = eframe::run_native(
        "StareDrop (Phase 0/1 MVP)",
        native_options,
        Box::new(|cc| Box::new(StareDropApp::new(cc))),
    ) {
        eprintln!("Failed to start StareDrop app: {err}");
    }
}

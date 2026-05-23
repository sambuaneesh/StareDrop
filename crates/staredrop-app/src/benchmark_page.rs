use eframe::egui;

#[derive(Default)]
pub struct BenchmarkPageState;

impl BenchmarkPageState {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Benchmark (Phase 4)");
        ui.label("Benchmark logging UI will be implemented in later phases.");
    }
}

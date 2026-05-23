use eframe::egui;

pub fn section_header(ui: &mut egui::Ui, text: &str) {
    ui.heading(text);
    ui.separator();
}

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use staredrop_codec_qr::{encode_text_to_qr_luma, render_luma_to_rgba};

#[derive(Debug, Clone)]
pub struct SenderConfig {
    pub payload_text: String,
    pub source_label: String,
}

pub struct SenderPageState {
    config: SenderConfig,
    qr_texture: Option<TextureHandle>,
    status: String,
    qr_pixels: [usize; 2],
}

impl SenderPageState {
    pub fn new(config: SenderConfig, cc: &eframe::CreationContext<'_>) -> Self {
        let mut state = Self {
            config,
            qr_texture: None,
            status: String::new(),
            qr_pixels: [0, 0],
        };
        state.regenerate_qr(&cc.egui_ctx);
        state
    }

    fn regenerate_qr(&mut self, ctx: &egui::Context) {
        if self.config.payload_text.trim().is_empty() {
            self.status = "Empty payload. Pass --text or --input-file.".to_string();
            return;
        }

        match encode_text_to_qr_luma(&self.config.payload_text) {
            Ok(luma) => {
                let rgba = render_luma_to_rgba(&luma);
                let image = ColorImage::from_rgba_unmultiplied(
                    [luma.width() as usize, luma.height() as usize],
                    &rgba,
                );
                self.qr_pixels = [luma.width() as usize, luma.height() as usize];
                self.qr_texture =
                    Some(ctx.load_texture("sender_qr_texture", image, TextureOptions::NEAREST));
                self.status = "Displaying QR payload".to_string();
            }
            Err(err) => {
                self.status = format!("Failed to encode QR: {err}");
            }
        }
    }

    pub fn ui_fullscreen(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context, show_overlay: bool) {
        let rect = ui.max_rect();
        ui.painter().rect_filled(rect, 0.0, egui::Color32::BLACK);

        if let Some(texture) = &self.qr_texture {
            let full = ui.available_size();
            let max_side = full.x.min(full.y) * 0.92;
            let size = egui::vec2(max_side, max_side);
            let offset = egui::vec2((full.x - size.x) * 0.5, (full.y - size.y) * 0.5);
            let qr_rect = egui::Rect::from_min_size(rect.min + offset, size);
            ui.painter().image(
                texture.id(),
                qr_rect,
                egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }

        if show_overlay {
            egui::Area::new("sender_overlay".into())
                .fixed_pos(egui::pos2(16.0, 16.0))
                .show(ui.ctx(), |ui| {
                    egui::Frame::default()
                        .fill(egui::Color32::from_black_alpha(170))
                        .rounding(egui::Rounding::same(6.0))
                        .inner_margin(egui::Margin::same(8.0))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("StareDrop Sender")
                                    .color(egui::Color32::LIGHT_GREEN)
                                    .strong(),
                            );
                            ui.label(format!("Source: {}", self.config.source_label));
                            ui.label(format!("Payload bytes: {}", self.config.payload_text.len()));
                            ui.label(format!(
                                "QR pixels: {} x {}",
                                self.qr_pixels[0], self.qr_pixels[1]
                            ));
                            ui.label(format!("Status: {}", self.status));
                            ui.separator();
                            ui.label("Controls: Q/Esc quit");
                        });
                });
        }
    }
}

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use staredrop_codec_qr::{encode_text_to_qr_luma, render_luma_to_rgba};

#[derive(Default)]
pub struct SenderPageState {
    text_input: String,
    last_encoded_text: String,
    qr_texture: Option<TextureHandle>,
    status: String,
}

impl SenderPageState {
    pub fn ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("Static QR Sender (Phase 1)");
        ui.label("Enter text and render as QR code.");
        ui.add_space(8.0);

        ui.label("Text payload");
        ui.text_edit_multiline(&mut self.text_input);

        let clicked = ui.button("Generate QR").clicked();
        let text_changed = self.text_input != self.last_encoded_text;
        if clicked || (text_changed && !self.text_input.is_empty()) {
            self.regenerate_qr(ctx);
        }

        if let Some(texture) = &self.qr_texture {
            let size = texture.size_vec2();
            ui.image((texture.id(), size));
        } else {
            ui.label("No QR yet.");
        }

        if !self.status.is_empty() {
            ui.separator();
            ui.label(&self.status);
        }
    }

    fn regenerate_qr(&mut self, ctx: &egui::Context) {
        if self.text_input.trim().is_empty() {
            self.status = "Type text before generating.".to_string();
            return;
        }

        match encode_text_to_qr_luma(&self.text_input) {
            Ok(luma) => {
                let rgba = render_luma_to_rgba(&luma);
                let image = ColorImage::from_rgba_unmultiplied(
                    [luma.width() as usize, luma.height() as usize],
                    &rgba,
                );
                let handle = ctx.load_texture("sender_qr_texture", image, TextureOptions::NEAREST);
                self.qr_texture = Some(handle);
                self.last_encoded_text = self.text_input.clone();
                self.status = "QR generated. Point receiver camera at this window.".to_string();
            }
            Err(err) => {
                self.status = format!("Failed to encode QR: {err}");
            }
        }
    }
}

use std::time::{Duration, Instant};

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use staredrop_codec_qr::{encode_text_to_qr_luma, render_luma_to_rgba};

use crate::transfer::SenderPlan;

#[derive(Debug, Clone)]
pub struct SenderConfig {
    pub plan: SenderPlan,
    pub fps: f32,
}

pub struct SenderPageState {
    config: SenderConfig,
    qr_texture: Option<TextureHandle>,
    status: String,
    qr_pixels: [usize; 2],
    current_frame_index: usize,
    loop_count: u64,
    last_switch_at: Instant,
}

impl SenderPageState {
    pub fn new(config: SenderConfig, cc: &eframe::CreationContext<'_>) -> Self {
        let mut state = Self {
            config,
            qr_texture: None,
            status: String::new(),
            qr_pixels: [0, 0],
            current_frame_index: 0,
            loop_count: 0,
            last_switch_at: Instant::now(),
        };
        state.render_current_frame(&cc.egui_ctx);
        state
    }

    fn frame_count(&self) -> usize {
        match &self.config.plan {
            SenderPlan::Text { .. } => 1,
            SenderPlan::File { frames, .. } => frames.len().max(1),
        }
    }

    fn current_frame_text(&self) -> &str {
        match &self.config.plan {
            SenderPlan::Text { frame_text } => frame_text,
            SenderPlan::File { frames, .. } => &frames[self.current_frame_index],
        }
    }

    fn render_current_frame(&mut self, ctx: &egui::Context) {
        match encode_text_to_qr_luma(self.current_frame_text()) {
            Ok(luma) => {
                let rgba = render_luma_to_rgba(&luma);
                let image = ColorImage::from_rgba_unmultiplied(
                    [luma.width() as usize, luma.height() as usize],
                    &rgba,
                );
                self.qr_pixels = [luma.width() as usize, luma.height() as usize];
                self.qr_texture =
                    Some(ctx.load_texture("sender_qr_texture", image, TextureOptions::NEAREST));
                self.status = "Displaying frame".to_string();
            }
            Err(err) => {
                self.status = format!("Failed to encode QR frame: {err}");
            }
        }
    }

    fn tick_animation(&mut self, ctx: &egui::Context) {
        if self.frame_count() <= 1 {
            return;
        }

        let fps = self.config.fps.max(0.5);
        let frame_dur = Duration::from_secs_f32(1.0 / fps);
        if self.last_switch_at.elapsed() < frame_dur {
            return;
        }

        self.current_frame_index += 1;
        if self.current_frame_index >= self.frame_count() {
            self.current_frame_index = 0;
            self.loop_count += 1;
        }
        self.last_switch_at = Instant::now();
        self.render_current_frame(ctx);
    }

    pub fn ui_fullscreen(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, show_overlay: bool) {
        self.tick_animation(ctx);
        let fps = self.config.fps.max(0.5);
        ctx.request_repaint_after(Duration::from_secs_f32(1.0 / fps));

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
                            match &self.config.plan {
                                SenderPlan::Text { frame_text } => {
                                    ui.label("Mode: Text");
                                    ui.label(format!("Payload bytes: {}", frame_text.len()));
                                }
                                SenderPlan::File { manifest, frames } => {
                                    ui.label("Mode: File transfer");
                                    ui.label(format!("File: {}", manifest.file_name));
                                    ui.label(format!(
                                        "Progress frame: {}/{}",
                                        self.current_frame_index + 1,
                                        frames.len()
                                    ));
                                    ui.label(format!("Loop count: {}", self.loop_count));
                                    ui.label(format!("Chunk count: {}", manifest.total_chunks));
                                }
                            }
                            ui.label(format!("FPS: {:.2}", self.config.fps));
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

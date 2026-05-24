use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use image::DynamicImage;
use staredrop_codec_grid::encode_color_grid_frame;
use staredrop_codec_qr::{encode_text_to_qr_luma, render_luma_to_rgba};
use staredrop_protocol::frame_json::{DataFrameV1, serialize_data_frame};

use crate::transfer::{SenderBuildOptions, SenderPlan, build_file_sender_plan};
use crate::visual_codec::VisualCodecConfig;

#[derive(Debug, Clone)]
pub struct SenderConfig {
    pub plan: SenderPlan,
    pub fps: f32,
    pub visual_codec: VisualCodecConfig,
    pub source_file: Option<PathBuf>,
    pub requested_chunk_size: Option<usize>,
}

pub struct SenderPageState {
    config: SenderConfig,
    frame_texture: Option<TextureHandle>,
    status: String,
    frame_pixels: [usize; 2],
    current_frame_index: usize,
    loop_count: u64,
    last_switch_at: Instant,
    active_grid_side: Option<u16>,
    active_chunk_size: Option<usize>,
}

impl SenderPageState {
    pub fn new(config: SenderConfig, cc: &eframe::CreationContext<'_>) -> Self {
        let mut state = Self {
            config,
            frame_texture: None,
            status: String::new(),
            frame_pixels: [0, 0],
            current_frame_index: 0,
            loop_count: 0,
            last_switch_at: Instant::now(),
            active_grid_side: None,
            active_chunk_size: None,
        };
        if !matches!(state.config.visual_codec, VisualCodecConfig::ColorGrid(_)) {
            state.render_current_frame(&cc.egui_ctx);
        }
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
        match self.config.visual_codec {
            VisualCodecConfig::Qr => match encode_text_to_qr_luma(self.current_frame_text()) {
                Ok(luma) => {
                    let rgba = render_luma_to_rgba(&luma);
                    let image = ColorImage::from_rgba_unmultiplied(
                        [luma.width() as usize, luma.height() as usize],
                        &rgba,
                    );
                    self.frame_pixels = [luma.width() as usize, luma.height() as usize];
                    self.frame_texture = Some(ctx.load_texture(
                        "sender_frame_texture",
                        image,
                        TextureOptions::NEAREST,
                    ));
                    self.status = "Displaying QR frame".to_string();
                }
                Err(err) => {
                    self.status = format!("Failed to encode QR frame: {err}");
                }
            },
            VisualCodecConfig::ColorGrid(grid) => {
                let grid_side = self
                    .active_grid_side
                    .unwrap_or_else(|| grid.grid_side_for_square_points(1080.0));
                let grid_cfg = grid.config_for_grid_side(grid_side);
                match encode_color_grid_frame(self.current_frame_text().as_bytes(), grid_cfg) {
                    Ok(encoded) => {
                        let rgba = DynamicImage::ImageRgb8(encoded.image.clone())
                            .to_rgba8()
                            .into_raw();
                        let image = ColorImage::from_rgba_unmultiplied(
                            [
                                encoded.image.width() as usize,
                                encoded.image.height() as usize,
                            ],
                            &rgba,
                        );
                        self.frame_pixels = [
                            encoded.image.width() as usize,
                            encoded.image.height() as usize,
                        ];
                        self.frame_texture = Some(ctx.load_texture(
                            "sender_frame_texture",
                            image,
                            TextureOptions::NEAREST,
                        ));
                        let max_payload = grid_cfg.max_payload_bytes();
                        let utilization = if max_payload == 0 {
                            0.0
                        } else {
                            (encoded.payload_bytes as f64 / max_payload as f64) * 100.0
                        };
                        self.status = format!(
                            "Displaying color-grid frame ({}x{}, payload {} B / {} B, {:.1}% utilized)",
                            grid_side, grid_side, encoded.payload_bytes, max_payload, utilization
                        );
                    }
                    Err(err) => {
                        self.status = format!("Failed to encode color-grid frame: {err}");
                    }
                }
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

    fn update_color_grid_layout(&mut self, ctx: &egui::Context, square_points: f32) {
        let VisualCodecConfig::ColorGrid(params) = self.config.visual_codec else {
            return;
        };

        let grid_side = params.grid_side_for_square_points(square_points);
        let side_changed = self.active_grid_side != Some(grid_side);

        if side_changed {
            self.active_grid_side = Some(grid_side);

            if self.config.requested_chunk_size.is_none() {
                if let Some(path) = self.config.source_file.clone() {
                    let payload_limit = params.config_for_grid_side(grid_side).max_payload_bytes();
                    let chunk_size = max_color_grid_chunk_size(payload_limit).max(1);
                    if self.active_chunk_size != Some(chunk_size) {
                        match build_file_sender_plan(&path, SenderBuildOptions { chunk_size }) {
                            Ok(plan) => {
                                self.config.plan = plan;
                                self.active_chunk_size = Some(chunk_size);
                                self.current_frame_index = 0;
                                self.loop_count = 0;
                                self.status = format!(
                                    "Auto chunk-size selected: {} bytes (grid {}x{})",
                                    chunk_size, grid_side, grid_side
                                );
                            }
                            Err(err) => {
                                self.status = format!("Failed to rebuild sender plan: {err}");
                            }
                        }
                    }
                }
            }

            self.render_current_frame(ctx);
        }
    }

    pub fn ui_fullscreen(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, show_overlay: bool) {
        let full = ui.available_size();
        let max_side = full.x.min(full.y) * 0.92;
        self.update_color_grid_layout(ctx, max_side);
        self.tick_animation(ctx);
        let fps = self.config.fps.max(0.5);
        ctx.request_repaint_after(Duration::from_secs_f32(1.0 / fps));

        let rect = ui.max_rect();
        ui.painter().rect_filled(rect, 0.0, egui::Color32::BLACK);

        if let Some(texture) = &self.frame_texture {
            let size = egui::vec2(max_side, max_side);
            let offset = egui::vec2((full.x - size.x) * 0.5, (full.y - size.y) * 0.5);
            let frame_rect = egui::Rect::from_min_size(rect.min + offset, size);
            ui.painter().image(
                texture.id(),
                frame_rect,
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
                            ui.label(format!(
                                "Visual codec: {}",
                                self.config.visual_codec.as_str()
                            ));
                            if let Some(side) = self.active_grid_side {
                                ui.label(format!("Auto grid: {} x {}", side, side));
                            }
                            if let Some(chunk_size) = self.active_chunk_size {
                                ui.label(format!("Auto chunk-size: {}", chunk_size));
                            }
                            ui.label(format!("FPS: {:.2}", self.config.fps));
                            ui.label(format!(
                                "Frame pixels: {} x {}",
                                self.frame_pixels[0], self.frame_pixels[1]
                            ));
                            ui.label(format!("Status: {}", self.status));
                            ui.separator();
                            ui.label("Controls: Q/Esc quit");
                        });
                });
        }
    }
}

fn max_color_grid_chunk_size(max_payload_bytes: usize) -> usize {
    if max_payload_bytes <= 64 {
        return 1;
    }

    let mut low = 1usize;
    let mut high = (max_payload_bytes.saturating_mul(3) / 4).max(1);
    let mut best = 1usize;

    while low <= high {
        let mid = low + (high - low) / 2;
        if color_grid_chunk_fits(mid, max_payload_bytes) {
            best = mid;
            low = mid.saturating_add(1);
        } else {
            if mid == 0 {
                break;
            }
            high = mid - 1;
        }
    }
    best
}

fn color_grid_chunk_fits(chunk_size: usize, max_payload_bytes: usize) -> bool {
    let payload = vec![0u8; chunk_size];
    let frame = DataFrameV1 {
        magic: "STAREDROP".to_string(),
        version: 1,
        frame_type: "DATA".to_string(),
        session_id: "00000000-0000-0000-0000-000000000000".to_string(),
        file_id: "00000000-0000-0000-0000-000000000000".to_string(),
        file_name: "payload.bin".to_string(),
        file_size: payload.len() as u64,
        chunk_index: 0,
        total_chunks: 1,
        payload_base64: BASE64.encode(&payload),
        crc32: staredrop_protocol::crc::crc32(&payload),
    };
    match serialize_data_frame(&frame) {
        Ok(text) => text.len() <= max_payload_bytes,
        Err(_) => false,
    }
}

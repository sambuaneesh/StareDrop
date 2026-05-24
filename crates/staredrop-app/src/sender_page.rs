use std::{
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use image::DynamicImage;
use staredrop_codec_grid::encode_color_grid_frame;
use staredrop_codec_qr::{encode_text_to_qr_luma, render_luma_to_rgba};

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
            let _ = state.render_current_frame(&cc.egui_ctx);
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

    fn render_current_frame(&mut self, ctx: &egui::Context) -> bool {
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
                    true
                }
                Err(err) => {
                    self.status = format!("Failed to encode QR frame: {err}");
                    false
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
                        true
                    }
                    Err(err) => {
                        self.status = format!("Failed to encode color-grid frame: {err}");
                        false
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

        let prev_idx = self.current_frame_index;
        let prev_loop = self.loop_count;
        let mut next_idx = self.current_frame_index + 1;
        let mut next_loop = self.loop_count;
        if next_idx >= self.frame_count() {
            next_idx = 0;
            next_loop += 1;
        }
        self.current_frame_index = next_idx;
        self.loop_count = next_loop;
        self.last_switch_at = Instant::now();
        if !self.render_current_frame(ctx) {
            self.current_frame_index = prev_idx;
            self.loop_count = prev_loop;
        }
    }

    fn update_color_grid_layout(&mut self, ctx: &egui::Context, square_points: f32) {
        let VisualCodecConfig::ColorGrid(params) = self.config.visual_codec else {
            return;
        };

        let grid_side = params.grid_side_for_square_points(square_points);
        let side_changed = self.active_grid_side != Some(grid_side);

        if side_changed {
            self.active_grid_side = Some(grid_side);
            let payload_limit = params.config_for_grid_side(grid_side).max_payload_bytes();

            if self.config.requested_chunk_size.is_none() {
                if let Some(path) = self.config.source_file.clone() {
                    match find_best_color_grid_plan(&path, payload_limit) {
                        Ok((chunk_size, plan)) => {
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
                            self.status = format!("Failed to build fitting color-grid plan: {err}");
                        }
                    }
                }
            } else if !plan_fits_payload_limit(&self.config.plan, payload_limit) {
                self.status = format!(
                    "Configured chunk-size does not fit current grid capacity ({payload_limit} B max frame payload). Reduce --chunk-size or --pixel-size."
                );
            }

            let _ = self.render_current_frame(ctx);
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

fn find_best_color_grid_plan(path: &PathBuf, payload_limit: usize) -> Result<(usize, SenderPlan)> {
    if payload_limit < 128 {
        bail!(
            "payload limit {} is too small for protocol frames",
            payload_limit
        );
    }

    let file_size = fs::metadata(path)
        .with_context(|| format!("failed to stat input file {}", path.display()))?
        .len() as usize;
    let mut low = 1usize;
    let mut high = file_size.max(1).min(payload_limit).max(1);
    let mut best: Option<(usize, SenderPlan)> = None;

    while low <= high {
        let mid = low + (high - low) / 2;
        let plan = build_file_sender_plan(path, SenderBuildOptions { chunk_size: mid })
            .with_context(|| format!("failed building sender plan for chunk-size {}", mid))?;
        if plan_fits_payload_limit(&plan, payload_limit) {
            best = Some((mid, plan));
            low = mid.saturating_add(1);
        } else {
            if mid == 0 {
                break;
            }
            high = mid - 1;
        }
    }

    if let Some(found) = best {
        return Ok(found);
    }

    let fallback = build_file_sender_plan(path, SenderBuildOptions { chunk_size: 1 })?;
    if plan_fits_payload_limit(&fallback, payload_limit) {
        return Ok((1, fallback));
    }
    bail!(
        "no fitting chunk-size found for payload limit {}; increase frame capacity or reduce pixel-size",
        payload_limit
    );
}

fn plan_fits_payload_limit(plan: &SenderPlan, payload_limit: usize) -> bool {
    match plan {
        SenderPlan::Text { frame_text } => frame_text.len() <= payload_limit,
        SenderPlan::File { frames, .. } => frames.iter().all(|f| f.len() <= payload_limit),
    }
}

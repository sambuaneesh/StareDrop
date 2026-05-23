use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use staredrop_camera::{CameraCapture, CameraDeviceInfo, list_cameras};
use staredrop_codec_qr::decode_first_qr_text;

#[derive(Debug, Clone)]
pub struct ReceiverConfig {
    pub camera_index: usize,
    pub auto_start: bool,
    pub print_decoded: bool,
}

pub struct ReceiverPageState {
    config: ReceiverConfig,
    devices: Vec<CameraDeviceInfo>,
    capture: Option<CameraCapture>,
    scanning: bool,
    preview_texture: Option<TextureHandle>,
    decoded_text: String,
    status: String,
    frames_seen: u64,
    decode_hits: u64,
    printed_last: String,
}

impl ReceiverPageState {
    pub fn new(config: ReceiverConfig) -> Self {
        let mut state = Self {
            config,
            devices: Vec::new(),
            capture: None,
            scanning: false,
            preview_texture: None,
            decoded_text: String::new(),
            status: String::new(),
            frames_seen: 0,
            decode_hits: 0,
            printed_last: String::new(),
        };
        state.refresh_devices();
        if state.config.auto_start {
            state.start();
        } else {
            state.status = "Ready. Press Space to start scanning.".to_string();
        }
        state
    }

    pub fn ui_fullscreen(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, show_overlay: bool) {
        self.handle_keys(ctx);

        if self.scanning {
            self.poll_camera(ctx);
            ctx.request_repaint();
        }

        let rect = ui.max_rect();
        ui.painter().rect_filled(rect, 0.0, egui::Color32::BLACK);
        if let Some(texture) = &self.preview_texture {
            let preview_size = texture.size_vec2();
            let full = ui.available_size();
            let scale = (full.x / preview_size.x).min(full.y / preview_size.y);
            let draw_size = preview_size * scale;
            let offset = egui::vec2((full.x - draw_size.x) * 0.5, (full.y - draw_size.y) * 0.5);
            let draw_rect = egui::Rect::from_min_size(rect.min + offset, draw_size);
            ui.painter().image(
                texture.id(),
                draw_rect,
                egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }

        if show_overlay {
            egui::Area::new("receiver_overlay".into())
                .fixed_pos(egui::pos2(16.0, 16.0))
                .show(ctx, |ui| {
                    egui::Frame::default()
                        .fill(egui::Color32::from_black_alpha(170))
                        .rounding(egui::Rounding::same(6.0))
                        .inner_margin(egui::Margin::same(8.0))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("StareDrop Receiver")
                                    .color(egui::Color32::LIGHT_BLUE)
                                    .strong(),
                            );
                            ui.label(format!("Camera index: {}", self.config.camera_index));
                            ui.label(format!("Frames captured: {}", self.frames_seen));
                            ui.label(format!("Frames decoded: {}", self.decode_hits));
                            ui.label(format!("Status: {}", self.status));
                            if !self.decoded_text.is_empty() {
                                ui.separator();
                                ui.label(format!("Last decoded: {}", self.decoded_text));
                            }
                            ui.separator();
                            ui.label("Controls: Space start/stop, R refresh cameras, Q/Esc quit");
                        });
                });
        }
    }

    fn handle_keys(&mut self, ctx: &egui::Context) {
        let toggle = ctx.input(|i| i.key_pressed(egui::Key::Space));
        if toggle {
            if self.scanning {
                self.stop();
            } else {
                self.start();
            }
        }

        let refresh = ctx.input(|i| i.key_pressed(egui::Key::R));
        if refresh {
            self.refresh_devices();
            self.status = "Camera list refreshed.".to_string();
        }
    }

    fn refresh_devices(&mut self) {
        match list_cameras() {
            Ok(devices) => {
                self.devices = devices;
            }
            Err(err) => {
                self.devices.clear();
                self.status = format!("Camera enumeration failed: {err}");
            }
        }
    }

    fn start(&mut self) {
        if self.devices.is_empty() {
            self.status = "No cameras available. Press R to refresh.".to_string();
            return;
        }

        if self.config.camera_index >= self.devices.len() {
            self.status = format!(
                "Camera index {} not available ({} device(s)).",
                self.config.camera_index,
                self.devices.len()
            );
            return;
        }

        let index = self.devices[self.config.camera_index].index;
        match CameraCapture::open(index) {
            Ok(capture) => {
                self.capture = Some(capture);
                self.scanning = true;
                self.status = "Scanning started.".to_string();
            }
            Err(err) => {
                self.status = format!("Failed to open camera: {err}");
            }
        }
    }

    fn stop(&mut self) {
        self.scanning = false;
        self.capture = None;
        self.status = "Scanning stopped.".to_string();
    }

    fn poll_camera(&mut self, ctx: &egui::Context) {
        let Some(capture) = self.capture.as_mut() else {
            return;
        };
        match capture.frame() {
            Ok(frame) => {
                self.frames_seen += 1;

                let rgba = frame.to_rgba_bytes();
                let image = ColorImage::from_rgba_unmultiplied(
                    [frame.width() as usize, frame.height() as usize],
                    &rgba,
                );

                if let Some(texture) = self.preview_texture.as_mut() {
                    texture.set(image, TextureOptions::LINEAR);
                } else {
                    self.preview_texture = Some(ctx.load_texture(
                        "receiver_preview_texture",
                        image,
                        TextureOptions::LINEAR,
                    ));
                }

                if let Ok(Some(text)) = decode_first_qr_text(&frame.to_gray()) {
                    self.decode_hits += 1;
                    self.status = "QR decoded.".to_string();
                    self.decoded_text = text;

                    if self.config.print_decoded && self.decoded_text != self.printed_last {
                        println!("{}", self.decoded_text);
                        self.printed_last = self.decoded_text.clone();
                    }
                }
            }
            Err(err) => {
                self.status = format!("Camera frame error: {err}");
            }
        }
    }
}

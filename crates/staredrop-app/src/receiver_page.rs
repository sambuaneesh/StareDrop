use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use staredrop_camera::{CameraCapture, CameraDeviceInfo, list_cameras};
use staredrop_codec_qr::decode_first_qr_text;

pub struct ReceiverPageState {
    devices: Vec<CameraDeviceInfo>,
    selected_index: usize,
    capture: Option<CameraCapture>,
    scanning: bool,
    preview_texture: Option<TextureHandle>,
    decoded_text: String,
    status: String,
    frames_seen: u64,
    decode_hits: u64,
}

impl ReceiverPageState {
    pub fn new() -> Self {
        let mut state = Self {
            devices: Vec::new(),
            selected_index: 0,
            capture: None,
            scanning: false,
            preview_texture: None,
            decoded_text: String::new(),
            status: String::new(),
            frames_seen: 0,
            decode_hits: 0,
        };
        state.refresh_devices();
        state
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("Camera Receiver (Phase 1)");
        ui.label("Open a camera stream and decode QR text from sender screen.");
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            if ui.button("Refresh Cameras").clicked() {
                self.refresh_devices();
            }

            if self.scanning {
                if ui.button("Stop Scanning").clicked() {
                    self.stop();
                }
            } else if ui.button("Start Scanning").clicked() {
                self.start();
            }
        });

        egui::ComboBox::from_label("Camera")
            .selected_text(
                self.devices
                    .get(self.selected_index)
                    .map(|d| d.human_name.as_str())
                    .unwrap_or("No camera"),
            )
            .show_ui(ui, |ui| {
                for (idx, device) in self.devices.iter().enumerate() {
                    ui.selectable_value(&mut self.selected_index, idx, &device.human_name);
                }
            });

        if self.scanning {
            self.poll_camera(ctx);
            ctx.request_repaint();
        }

        ui.separator();
        if let Some(texture) = &self.preview_texture {
            let size = texture.size_vec2();
            let max_w = ui.available_width().min(size.x);
            let scale = if size.x > 0.0 { max_w / size.x } else { 1.0 };
            ui.image((texture.id(), size * scale));
        } else {
            ui.label("No camera preview yet.");
        }

        ui.separator();
        ui.label(format!("Frames captured: {}", self.frames_seen));
        ui.label(format!("Frames decoded: {}", self.decode_hits));
        ui.label(format!("Last decoded text: {}", self.decoded_text));
        if !self.status.is_empty() {
            ui.label(format!("Status: {}", self.status));
        }
    }

    fn refresh_devices(&mut self) {
        match list_cameras() {
            Ok(devices) => {
                self.devices = devices;
                if self.selected_index >= self.devices.len() {
                    self.selected_index = 0;
                }
                self.status = format!("Found {} camera device(s).", self.devices.len());
            }
            Err(err) => {
                self.devices.clear();
                self.selected_index = 0;
                self.status = format!("Camera enumeration failed: {err}");
            }
        }
    }

    fn start(&mut self) {
        if self.devices.is_empty() {
            self.status = "No cameras available.".to_string();
            return;
        }
        let index = self.devices[self.selected_index].index;
        match CameraCapture::open(index) {
            Ok(capture) => {
                self.capture = Some(capture);
                self.scanning = true;
                self.status = "Camera stream opened.".to_string();
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
                    self.decoded_text = text;
                }
            }
            Err(err) => {
                self.status = format!("Camera frame error: {err}");
            }
        }
    }
}

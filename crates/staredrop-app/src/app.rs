use eframe::egui;

use crate::receiver_page::{ReceiverConfig, ReceiverPageState};
use crate::sender_page::{SenderConfig, SenderPageState};

#[derive(Debug, Clone)]
pub enum LaunchMode {
    Sender(SenderConfig),
    Receiver(ReceiverConfig),
}

enum RuntimeMode {
    Sender(SenderPageState),
    Receiver(ReceiverPageState),
}

pub struct StareDropApp {
    mode: RuntimeMode,
    show_overlay: bool,
}

impl StareDropApp {
    pub fn new(cc: &eframe::CreationContext<'_>, mode: LaunchMode, show_overlay: bool) -> Self {
        let mode = match mode {
            LaunchMode::Sender(config) => RuntimeMode::Sender(SenderPageState::new(config, cc)),
            LaunchMode::Receiver(config) => RuntimeMode::Receiver(ReceiverPageState::new(config)),
        };
        Self { mode, show_overlay }
    }

    fn handle_global_shortcuts(&mut self, ctx: &egui::Context) {
        let wants_quit = ctx.input(|i| i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::Q));
        if wants_quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

impl eframe::App for StareDropApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_global_shortcuts(ctx);

        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| match &mut self.mode {
                RuntimeMode::Sender(sender) => sender.ui_fullscreen(ui, ctx, self.show_overlay),
                RuntimeMode::Receiver(receiver) => receiver.ui_fullscreen(ui, ctx, self.show_overlay),
            });
    }

    fn persist_egui_memory(&self) -> bool {
        false
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 1.0]
    }
}

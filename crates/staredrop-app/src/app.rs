use eframe::egui;

use crate::{
    benchmark_page::BenchmarkPageState, receiver_page::ReceiverPageState,
    sender_page::SenderPageState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TopTab {
    Sender,
    Receiver,
    Benchmark,
}

pub struct StareDropApp {
    selected_tab: TopTab,
    sender: SenderPageState,
    receiver: ReceiverPageState,
    benchmark: BenchmarkPageState,
}

impl StareDropApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            selected_tab: TopTab::Sender,
            sender: SenderPageState::default(),
            receiver: ReceiverPageState::new(),
            benchmark: BenchmarkPageState::default(),
        }
    }
}

impl eframe::App for StareDropApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_nav").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.selected_tab, TopTab::Sender, "Sender");
                ui.selectable_value(&mut self.selected_tab, TopTab::Receiver, "Receiver");
                ui.selectable_value(&mut self.selected_tab, TopTab::Benchmark, "Benchmark");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.selected_tab {
            TopTab::Sender => self.sender.ui(ui, ctx),
            TopTab::Receiver => self.receiver.ui(ui, ctx),
            TopTab::Benchmark => self.benchmark.ui(ui),
        });
    }
}

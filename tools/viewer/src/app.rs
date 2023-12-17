use crate::{ui::ReplayView, SystemCommand, UserCommand};
use crossbeam_channel::{Receiver, Sender};
use eframe::egui;
use std::time::Instant;

pub struct App {
    pub(crate) frame_time_history: egui::util::History<f32>,

    pub(crate) usrcmd_sender: Sender<UserCommand>,
    usrcmd_receiver: Receiver<UserCommand>,
    pub(crate) syscmd_sender: Sender<SystemCommand>,
    syscmd_receiver: Receiver<SystemCommand>,

    pub(crate) replay_view: Option<ReplayView>,
}

impl App {
    pub fn new(egui_ctx: &egui::Context) -> Self {
        // TODO: should this be here? probaby not?
        crate::ui::egui_styles::apply(egui_ctx);
        egui_extras::install_image_loaders(egui_ctx);

        let (usrcmd_sender, usrcmd_receiver) = crossbeam_channel::unbounded();
        let (syscmd_sender, syscmd_receiver) = crossbeam_channel::unbounded();

        Self {
            frame_time_history: egui::util::History::new(1..100, 0.5),

            usrcmd_sender,
            usrcmd_receiver,
            syscmd_sender,
            syscmd_receiver,

            replay_view: None,
        }
    }

    fn ui(&mut self, egui_ctx: &egui::Context, _frame: &eframe::Frame) {
        let main_panel_frame = egui::Frame::default();
        egui::CentralPanel::default()
            .frame(main_panel_frame)
            .show(egui_ctx, |ui| {
                crate::ui::TopPanel::ui(ui, self);

                if let Some(replay_view) = self.replay_view.as_mut() {
                    replay_view.ui(ui);
                } else {
                    crate::ui::WelcomeView::ui(ui, self);
                    crate::ui::PanView::ui(ui);
                }
            });
    }

    fn run_pending_user_commands(&mut self) {
        while let Some(cmd) = self.usrcmd_receiver.try_recv().ok() {
            cmd.handle(self);
        }
    }

    fn run_pending_system_commands(&mut self) {
        while let Some(cmd) = self.syscmd_receiver.try_recv().ok() {
            cmd.handle(self);
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, egui_ctx: &egui::Context, frame: &mut eframe::Frame) {
        let frame_start = Instant::now();

        if let Some(replay_view) = self.replay_view.as_mut() {
            replay_view.update();
        }

        self.ui(egui_ctx, frame);

        // run pending commands last (so we don't have to wait for a repaint
        // before they are run)
        self.run_pending_user_commands();
        self.run_pending_system_commands();

        // frame time measurer - must be last
        self.frame_time_history.add(
            egui_ctx.input(|i| i.time),
            frame_start.elapsed().as_secs_f32(),
        );
    }
}

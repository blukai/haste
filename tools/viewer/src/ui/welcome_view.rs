use crate::{user_command::UserCommand, App};
use eframe::{egui, emath, epaint};

pub(crate) struct WelcomeView;

impl WelcomeView {
    pub(crate) fn ui(ui: &mut egui::Ui, app: &mut App) {
        egui::CentralPanel::default()
            .frame(Self::frame())
            .show_inside(ui, |ui| {
                Self::open_file_button_ui(app, ui);
            });
    }

    fn open_file_button_ui(app: &mut App, ui: &mut egui::Ui) {
        let button_size = emath::vec2(128.0, 24.0);
        let button = egui::Button::new("open file…");

        let available_size = ui.available_size();
        let top = (available_size.y / 2.0) - (button_size.y / 2.0);
        let left = (available_size.x / 2.0) - (button_size.x / 2.0);

        ui.vertical(|ui| {
            ui.add_space(top);

            ui.horizontal(|ui| {
                ui.add_space(left);

                if ui.add_sized(button_size, button).clicked() {
                    app.usrcmd_sender.send(UserCommand::OpenFile).ok();
                }
            });
        });
    }

    // design tokens

    fn background() -> epaint::Color32 {
        // TODO: get from visuals.panel_fill
        epaint::Color32::from_rgb(13, 16, 17)
    }

    fn inner_margin() -> egui::Margin {
        egui::Margin::same(8.0)
    }

    fn frame() -> egui::Frame {
        egui::Frame {
            fill: Self::background(),
            inner_margin: Self::inner_margin(),
            ..Default::default()
        }
    }
}

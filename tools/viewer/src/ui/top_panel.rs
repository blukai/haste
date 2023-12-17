use crate::app::App;
use eframe::{egui, epaint};

pub(crate) struct TopPanel;

impl TopPanel {
    pub(crate) fn ui(ui: &mut egui::Ui, app: &mut App) {
        egui::TopBottomPanel::top("top_panel")
            .frame(Self::frame())
            .exact_height(Self::height())
            .show_inside(ui, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.set_height(Self::height());

                    Self::frame_time_label_ui(app, ui);
                });
            });
    }

    fn frame_time_label_ui(app: &mut App, ui: &mut egui::Ui) {
        if let Some(frame_time) = app.frame_time_history.average() {
            let ms = frame_time * 1e3;
            let text = format!("{ms:07.4} ms");

            let visuals = ui.visuals();
            let color = visuals.weak_text_color();

            ui.label(egui::RichText::new(text).color(color))
                .on_hover_text("cpu time used for each frame");
        }
    }

    // design tokens

    fn height() -> f32 {
        40.0
    }

    fn inner_margin() -> egui::Margin {
        egui::Margin::symmetric(8.0, 0.0)
    }

    fn background() -> epaint::Color32 {
        // NOTE: this color is stolen from figma
        epaint::Color32::from_gray(20)
    }

    fn frame() -> egui::Frame {
        egui::Frame {
            inner_margin: Self::inner_margin(),
            fill: Self::background(),
            ..Default::default()
        }
    }
}

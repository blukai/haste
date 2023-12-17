use eframe::{egui, emath};

pub(crate) struct ListItem {
    text: egui::WidgetText,
    selected: bool,
}

impl ListItem {
    pub(crate) fn new(text: impl Into<egui::WidgetText>) -> Self {
        Self {
            text: text.into(),
            selected: false,
        }
    }

    pub(crate) fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub(crate) fn show(self, ui: &mut egui::Ui) -> egui::Response {
        let desired_size = emath::vec2(ui.available_width(), Self::height());
        let (rect, mut response) = ui.allocate_at_least(desired_size, egui::Sense::click());

        if !ui.is_rect_visible(rect) {
            return response;
        }

        // NOTE: following line sets inactive bg color of a list item from
        // default grey to none
        ui.visuals_mut().widgets.inactive.bg_fill = Default::default();
        let visuals = ui.style().interact_selectable(&response, self.selected);

        let text_wrap_width = ui.available_width() - Self::inner_margin_x() * 2.0;
        let mut text_job =
            self.text
                .into_text_job(ui.style(), egui::FontSelection::Default, egui::Align::LEFT);
        text_job.job.wrap = egui::text::TextWrapping::truncate_at_width(text_wrap_width);

        let text_galley = ui.fonts(|f| text_job.into_galley(f));
        let text_pos = egui::Align2::LEFT_CENTER
            .align_size_within_rect(
                text_galley.size(),
                rect.shrink2(emath::vec2(Self::inner_margin_x(), 0.0)),
            )
            .min;

        // NOTE: i'm not actually sure what this does, prob allows to do
        // keyboard navigation?
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::SelectableLabel, text_galley.text())
        });

        // NOTE: elided is ~= truncated :shrug:
        if text_galley.galley.elided {
            response = response.on_hover_ui(|ui| {
                ui.label(text_galley.text());
            });
        }

        ui.painter()
            .rect_filled(rect, egui::Rounding::ZERO, visuals.bg_fill);
        text_galley.paint_with_visuals(ui.painter(), text_pos, &visuals);

        response
    }

    // design tokens

    fn height() -> f32 {
        24.0
    }

    fn inner_margin_x() -> f32 {
        // TODO: spacing design tokens
        8.0
    }
}

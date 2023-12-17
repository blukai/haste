use eframe::{egui, epaint};

// TODO: theme tokens (most importantly spacing! etc..)
//
// NOTE: some inspiration can be drawn from
// https://github.com/catppuccin/egui/blob/main/src/lib.rs

pub(crate) fn apply(egui_ctx: &egui::Context) {
    let mut egui_style = egui::Style {
        visuals: egui::Visuals::dark(),
        ..Default::default()
    };

    // NOTE: separator lines, panel lines, etc
    // egui_style.visuals.widgets.noninteractive.bg_stroke.color = epaint::Color32::from_gray(40);

    // NOTE: this enforces monospace fonts for everything
    egui_style.text_styles.iter_mut().for_each(|(_, font_id)| {
        font_id.family = egui::FontFamily::Monospace;
    });

    // NOTE: following lines turn off strokes around buttons
    egui_style.visuals.widgets.inactive.bg_stroke = Default::default();
    egui_style.visuals.widgets.hovered.bg_stroke = Default::default();
    egui_style.visuals.widgets.active.bg_stroke = Default::default();
    egui_style.visuals.widgets.open.bg_stroke = Default::default();

    // NOTE: buttons look better when they don't change size
    // TODO: it might make more sense to implement a custom button and set those
    // values in the appropriate scope
    egui_style.visuals.widgets.hovered.expansion = 0.0;
    egui_style.visuals.widgets.active.expansion = 0.0;

    // NOTE: egui's popup shadows look lame
    egui_style.visuals.popup_shadow = egui::epaint::Shadow::NONE;

    // TODO: unhardcode welcome view, replay view's pannel backgrounds
    egui_style.visuals.panel_fill = epaint::Color32::from_rgb(13, 16, 17);

    // NOTE: nicer text colors, stoken from rerun
    // non-interactive text
    egui_style.visuals.widgets.noninteractive.fg_stroke.color =
        epaint::Color32::from_rgb(125, 140, 146);
    // button text
    egui_style.visuals.widgets.inactive.fg_stroke.color = epaint::Color32::from_rgb(202, 216, 222);
    // strong text and active button text
    egui_style.visuals.widgets.active.fg_stroke.color = epaint::Color32::WHITE;

    // NOTE: following fill is used for highlighted or emphasized items, such as
    // current navigation items
    egui_style.visuals.selection.bg_fill = epaint::Color32::from_rgb(0, 61, 161);

    egui_ctx.set_style(egui_style);
}

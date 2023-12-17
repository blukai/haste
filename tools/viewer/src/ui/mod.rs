// TODO: move everything out of ui directory

// header

mod top_panel;
pub(crate) use top_panel::TopPanel;

// main

mod welcome_view;
pub(crate) use welcome_view::WelcomeView;

mod replay_view;
pub(crate) use replay_view::ReplayView;

// footer

// misc

pub(crate) mod egui_styles;
pub(crate) mod icons;

mod list_item;
pub(crate) use list_item::ListItem;

// playgound

mod zoom_pan_area;
pub(crate) use zoom_pan_area::PanView;

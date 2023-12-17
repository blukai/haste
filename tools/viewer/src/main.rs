use eframe::egui;
use viewer::App;

// NOTE: scaling is super wierd on x11. by default when i run the thing
// everything is 2x bigger - that is because egui uses winit and winit on x11
// first looks at WINIT_X11_SCALE_FACTOR (on my system it's not set by default),
// then it looks at Xft.dpi in ~/.Xresources and there it's set to 190 which is
// sort of 96 (the default) multiplied by 2 (pretty sure i set this value years
// ago).
//
// when i start brave with --force-device-scale-factor=1 flag what is supposed
// to be 24px is actually 27. usually I start brawe with scale factor of 1.3
// which results in 24px becoming 34 thus 34/24 results in
// WINIT_X11_SCALE_FACTOR=1.416.
//
// to learn more about how winit determines scale factor go to
// https://docs.rs/winit/latest/winit/dpi/
//
// TODO: maybe force winit to ignore x11's

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "viewer",
        options,
        Box::new(|cc| Box::new(App::new(&cc.egui_ctx))),
    )
}

use eframe::egui;

pub(crate) struct Icon {
    name: &'static str,
    data: &'static [u8],
}

impl Icon {
    pub(crate) const fn new(name: &'static str, data: &'static [u8]) -> Self {
        Self { name, data }
    }

    pub(crate) fn as_image(&self) -> egui::Image<'static> {
        egui::Image::new(egui::ImageSource::Bytes {
            uri: self.name.into(),
            bytes: self.data.into(),
        })
    }
}

pub(crate) const PLAY: Icon = Icon::new(
    "bytes://icons/play.svg",
    include_bytes!("../../assets/icons/play.svg"),
);

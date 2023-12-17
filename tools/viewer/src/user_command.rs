use crate::{data_source::DataSource, App, SystemCommand};

#[derive(Clone)]
pub(crate) enum UserCommand {
    OpenFile,
}

impl UserCommand {
    pub(crate) fn handle(self, app: &mut App) {
        match self {
            Self::OpenFile => Self::handle_open_file(app),
        }
    }

    // NOTE: handle_open_file is partially based on
    // https://sourcegraph.com/github.com/aurelilia/gamegirl/-/blob/gamegirl-egui/src/input/file_dialog.rs?L23:8-23:12#tab=references
    #[cfg(not(target_arch = "wasm32"))]
    fn handle_open_file(app: &mut App) {
        let syscmd_sender = app.syscmd_sender.clone();
        std::thread::spawn(move || {
            let future = async {
                let maybe_file = rfd::AsyncFileDialog::new()
                    .add_filter("dota 2 replay file", &["dem"])
                    .pick_file()
                    .await;
                if let Some(file) = maybe_file {
                    syscmd_sender
                        .send(SystemCommand::LoadDataSource(DataSource::FilePath(
                            file.path().to_path_buf(),
                        )))
                        .ok();
                }
            };
            pollster::block_on(future);
        });
    }

    #[cfg(target_arch = "wasm32")]
    fn handle_open_file(app: &mut App) {
        unimplemented!()
    }
}

use crate::{data_source::DataSource, ui::ReplayView, App};

#[derive(Clone)]
pub(crate) enum SystemCommand {
    LoadDataSource(DataSource),
}

impl SystemCommand {
    pub(crate) fn handle(self, app: &mut App) {
        match self {
            Self::LoadDataSource(data_source) => Self::handle_load_data_source(app, data_source),
        }
    }

    fn handle_load_data_source(app: &mut App, data_source: DataSource) {
        app.replay_view = Some(ReplayView::new(data_source));
    }
}

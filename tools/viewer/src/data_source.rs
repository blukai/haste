use std::path::PathBuf;

#[derive(Clone)]
pub(crate) enum DataSource {
    #[cfg(not(target_arch = "wasm32"))]
    FilePath(PathBuf),
    FileContents {
        file_name: String,
        bytes: Vec<u8>,
    },
}

impl DataSource {
    pub(crate) fn file_name(&self) -> &str {
        match self {
            Self::FilePath(file_path) => {
                // TODO: can this fail? if so - why and how?
                file_path
                    .file_name()
                    .map(|os_str| os_str.to_str().unwrap_or_default())
                    .unwrap_or_default()
            }
            Self::FileContents { file_name, .. } => file_name.as_str(),
        }
    }
}

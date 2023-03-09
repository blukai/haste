#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("invalid header id (want {want:?}, got {got:?})")]
    InvalidDemHeader { want: [u8; 8], got: [u8; 8] },
    #[error("invalid file info offset")]
    InvalidFileInfoOffset,
    #[error("unknown message command {0}")]
    UnknownDemoCommand(u32),
    #[error("invalid varint")]
    InvalidVarint,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Snap(snap::Error),
    #[error("buffer overflow")]
    BufferOverflow,
}

pub type Result<T> = std::result::Result<T, Error>;

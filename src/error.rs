use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid header id (want {want:?}, got {got:?})")]
    InvalidHeaderId { want: [u8; 8], got: [u8; 8] },
    #[error("invalid file info offset")]
    InvalidFileInfoOffset,
    #[error("unknown message command {0}")]
    UnknownDemoCommand(u32),
    #[error("invalid varint")]
    InvalidVarint,
    #[error("buffer overflow")]
    BufferOverflow,
}

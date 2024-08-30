#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("unknown char {0}")]
    UnknownChar(char),
    #[error("unexpected eof")]
    UnexpectedEof,
    #[error("unexpected token at {0}")]
    UnexpectedToken(u16),
}

pub type Result<T> = std::result::Result<T, Error>;

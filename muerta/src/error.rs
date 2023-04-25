#[derive(thiserror::Error, Debug)]
pub enum Error {
    // std
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
    // 3rd party crates
    #[error(transparent)]
    Decode(#[from] prost::DecodeError),
    #[error(transparent)]
    Snap(#[from] snap::Error),
    // crate
    #[error(transparent)]
    BitReader(#[from] crate::bitreader::BitReaderError),
    #[error(transparent)]
    Dem(#[from] crate::dem::DemError),
    #[error(transparent)]
    Parser(#[from] crate::parser::ParserError),
    #[error(transparent)]
    VarInt(#[from] crate::varint::VarIntError),
    // common
    #[error("missing req'd value")]
    Required {
        #[backtrace]
        backtrace: std::backtrace::Backtrace,
    },
}

macro_rules! required {
    () => {{
        crate::error::Error::Required {
            backtrace: std::backtrace::Backtrace::capture(),
        }
    }};
}
pub(crate) use required;

pub type Result<T> = std::result::Result<T, Error>;

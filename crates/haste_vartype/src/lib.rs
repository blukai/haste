//! overengineered piece of crap.

mod error;
mod parser;
mod span;
mod tokenizer;

pub use error::Error;
pub use parser::{parse, Expr, Lit};
pub use span::Span;
pub use tokenizer::{Token, TokenKind, Tokenizer};

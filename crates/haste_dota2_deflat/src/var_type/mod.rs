mod tokenizer;
pub use tokenizer::{Token, Tokenizer};

mod parser;
pub use parser::{ident_atom, parse, ArrayLength, Decl, IdentAtom};

// cargo test --package haste_dota2_deflat --lib -- var_type --nocapture
#[cfg(test)]
mod tests;

mod tokenizer;
pub use tokenizer::{Token, Tokenizer};

mod parser;
pub use parser::{parse, ArrayLength, Decl};

// cargo test --package haste_dota2_deflat --lib -- var_type --nocapture
#[cfg(test)]
mod tests;

mod tokenizer;

mod parser;
pub use parser::{ident_atom, parse, ArrayLength, IdentAtom, TypeDecl};

// cargo test --package haste_dota2_deflat --lib -- var_type --nocapture
#[cfg(test)]
mod tests;

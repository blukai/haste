use super::tokenizer::{Token, Tokenizer};
use std::iter::{Peekable, Rev};

pub use haste_dota2_deflat_atoms::var_type::IdentAtom;
pub use haste_dota2_deflat_atoms::var_type_ident_atom as ident_atom;

#[derive(Debug, PartialEq, Clone)]
pub enum ArrayLength {
    Ident(IdentAtom),
    Number(usize),
}

// TODO: get rid of Box (/ do not allocate heap memory)

#[derive(Debug, PartialEq, Clone)]
pub enum TypeDecl {
    Ident(IdentAtom),
    Pointer(Box<Self>),
    Template {
        ident: IdentAtom,
        argument: Box<Self>,
    },
    Array {
        type_decl: Box<Self>,
        length: ArrayLength,
    },
}

impl Default for TypeDecl {
    fn default() -> Self {
        Self::Ident(IdentAtom::from(""))
    }
}

// ----

#[inline]
fn parse_ident(rev_iter: &mut Peekable<Rev<Tokenizer>>) -> TypeDecl {
    match rev_iter.next() {
        Some(Token::Ident(ident)) => TypeDecl::Ident(IdentAtom::from(ident)),
        _ => unreachable!(),
    }
}

#[inline]
fn parse_template(rev_iter: &mut Peekable<Rev<Tokenizer>>) -> TypeDecl {
    assert_eq!(rev_iter.next(), Some(Token::RAngle));
    let argument = parse_any(rev_iter);
    assert_eq!(rev_iter.next(), Some(Token::LAngle));

    let ident = match rev_iter.next() {
        Some(Token::Ident(ident)) => ident,
        _ => unreachable!(),
    };

    TypeDecl::Template {
        ident: IdentAtom::from(ident),
        argument: Box::new(argument),
    }
}

#[inline]
fn parse_array(rev_iter: &mut Peekable<Rev<Tokenizer>>) -> TypeDecl {
    assert_eq!(rev_iter.next(), Some(Token::RSquare));
    let length = match rev_iter.next() {
        Some(Token::Ident(ident)) => ArrayLength::Ident(IdentAtom::from(ident)),
        Some(Token::Number(number)) => ArrayLength::Number(
            // TODO: don't panic
            number.parse::<usize>().unwrap(),
        ),
        _ => unreachable!(),
    };
    assert_eq!(rev_iter.next(), Some(Token::LSquare));

    let type_decl = parse_any(rev_iter);

    TypeDecl::Array {
        type_decl: Box::new(type_decl),
        length,
    }
}

#[inline]
fn parse_pointer(rev_iter: &mut Peekable<Rev<Tokenizer>>) -> TypeDecl {
    assert_eq!(rev_iter.next(), Some(Token::Asterisk));

    let type_decl = parse_any(rev_iter);

    TypeDecl::Pointer(Box::new(type_decl))
}

#[inline]
fn parse_any(rev_iter: &mut Peekable<Rev<Tokenizer>>) -> TypeDecl {
    match rev_iter.peek() {
        Some(Token::Ident(_)) => parse_ident(rev_iter),
        Some(Token::RAngle) => parse_template(rev_iter),
        Some(Token::RSquare) => parse_array(rev_iter),
        Some(Token::Asterisk) => parse_pointer(rev_iter),
        _ => todo!(),
    }
}

pub fn parse(input: &str) -> TypeDecl {
    let mut tokens = Tokenizer::new(input).into_iter().rev().peekable();
    parse_any(&mut tokens)
}

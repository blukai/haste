use super::tokenizer::{Token, Tokenizer};
use std::iter::{Peekable, Rev};

pub use haste_dota2_atoms::var_type_ident::VarTypeIdentAtom as IdentAtom;
pub use haste_dota2_atoms::var_type_ident_atom as ident_atom;

#[derive(Debug, PartialEq, Clone)]
pub enum ArrayLength {
    Ident(IdentAtom),
    Number(usize),
}

// TODO: get rid of Box (/ do not allocate heap memory)

#[derive(Debug, PartialEq, Clone)]
pub enum Decl {
    Ident(IdentAtom),
    Pointer(Box<Self>),
    Template {
        ident: IdentAtom,
        argument: Box<Self>,
    },
    Array {
        decl: Box<Self>,
        length: ArrayLength,
    },
}

impl Default for Decl {
    fn default() -> Self {
        Self::Ident(IdentAtom::from(""))
    }
}

// ----

#[inline]
fn parse_ident(rev_iter: &mut Peekable<Rev<Tokenizer>>) -> Decl {
    match rev_iter.next() {
        Some(Token::Ident(ident)) => Decl::Ident(IdentAtom::from(ident)),
        _ => unreachable!(),
    }
}

#[inline]
fn parse_template(rev_iter: &mut Peekable<Rev<Tokenizer>>) -> Decl {
    assert_eq!(rev_iter.next(), Some(Token::RAngle));
    let argument = parse_any(rev_iter);
    assert_eq!(rev_iter.next(), Some(Token::LAngle));

    let ident = match rev_iter.next() {
        Some(Token::Ident(ident)) => ident,
        _ => unreachable!(),
    };

    Decl::Template {
        ident: IdentAtom::from(ident),
        argument: Box::new(argument),
    }
}

#[inline]
fn parse_array(rev_iter: &mut Peekable<Rev<Tokenizer>>) -> Decl {
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

    let decl = parse_any(rev_iter);

    Decl::Array {
        decl: Box::new(decl),
        length,
    }
}

#[inline]
fn parse_pointer(rev_iter: &mut Peekable<Rev<Tokenizer>>) -> Decl {
    assert_eq!(rev_iter.next(), Some(Token::Asterisk));

    let decl = parse_any(rev_iter);

    Decl::Pointer(Box::new(decl))
}

#[inline]
fn parse_any(rev_iter: &mut Peekable<Rev<Tokenizer>>) -> Decl {
    match rev_iter.peek() {
        Some(Token::Ident(_)) => parse_ident(rev_iter),
        Some(Token::RAngle) => parse_template(rev_iter),
        Some(Token::RSquare) => parse_array(rev_iter),
        Some(Token::Asterisk) => parse_pointer(rev_iter),
        _ => todo!(),
    }
}

pub fn parse(input: &str) -> Decl {
    let mut tokens = Tokenizer::new(input).rev().peekable();
    parse_any(&mut tokens)
}

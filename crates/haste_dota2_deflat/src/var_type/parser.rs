use super::tokenizer::{Token, Tokenizer};
use std::iter::{Peekable, Rev};

#[derive(Debug, PartialEq, Clone)]
pub enum ArrayLength<'a> {
    Ident(&'a str),
    Number(usize),
}

#[derive(Debug, PartialEq, Clone)]
pub enum Decl<'a> {
    Ident(&'a str),
    Pointer(Box<Decl<'a>>),
    Template {
        ident: &'a str,
        argument: Box<Decl<'a>>,
    },
    Array {
        decl: Box<Decl<'a>>,
        length: ArrayLength<'a>,
    },
}

impl<'a> Default for Decl<'a> {
    fn default() -> Self {
        Self::Ident("")
    }
}

// ----

#[inline]
fn parse_ident<'a>(rev_iter: &mut Peekable<Rev<Tokenizer<'a>>>) -> Decl<'a> {
    match rev_iter.next() {
        Some(Token::Ident(ident)) => Decl::Ident(ident),
        _ => unreachable!(),
    }
}

#[inline]
fn parse_template<'a>(rev_iter: &mut Peekable<Rev<Tokenizer<'a>>>) -> Decl<'a> {
    assert_eq!(rev_iter.next(), Some(Token::RAngle));
    let argument = parse_any(rev_iter);
    assert_eq!(rev_iter.next(), Some(Token::LAngle));

    let ident = match rev_iter.next() {
        Some(Token::Ident(ident)) => ident,
        _ => unreachable!(),
    };

    Decl::Template {
        ident,
        argument: Box::new(argument),
    }
}

#[inline]
fn parse_array<'a>(rev_iter: &mut Peekable<Rev<Tokenizer<'a>>>) -> Decl<'a> {
    assert_eq!(rev_iter.next(), Some(Token::RSquare));
    let length = match rev_iter.next() {
        Some(Token::Ident(ident)) => ArrayLength::Ident(ident),
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
fn parse_pointer<'a>(rev_iter: &mut Peekable<Rev<Tokenizer<'a>>>) -> Decl<'a> {
    assert_eq!(rev_iter.next(), Some(Token::Asterisk));

    let decl = parse_any(rev_iter);

    Decl::Pointer(Box::new(decl))
}

#[inline]
fn parse_any<'a>(rev_iter: &mut Peekable<Rev<Tokenizer<'a>>>) -> Decl<'a> {
    match rev_iter.peek() {
        Some(Token::Ident(_)) => parse_ident(rev_iter),
        Some(Token::RAngle) => parse_template(rev_iter),
        Some(Token::RSquare) => parse_array(rev_iter),
        Some(Token::Asterisk) => parse_pointer(rev_iter),
        _ => todo!(),
    }
}

pub fn parse<'a>(input: &'a str) -> Decl<'a> {
    let mut tokens = Tokenizer::new(input).rev().peekable();
    parse_any(&mut tokens)
}

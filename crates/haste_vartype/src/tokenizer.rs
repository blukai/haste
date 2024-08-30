use dungers_charsor::Charsor;

use crate::{Error, Result, Span};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TokenKind<'a> {
    LAngle,         // <
    RAngle,         // >
    LSquare,        // [
    RSquare,        // ]
    Asterisk,       // *
    Ident(&'a str), // uint64, MAX_ABILITY_DRAFT_ABILITIES, etc..
    Lit(&'a str),   // 256
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Token<'a> {
    pub kind: TokenKind<'a>,
    pub span: Span,
}

impl<'a> Token<'a> {
    fn new(kind: TokenKind<'a>, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone)]
pub struct Tokenizer<'a> {
    cursor: Charsor<'a>,
}

#[inline(always)]
fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic()
}

#[inline(always)]
fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

impl<'a> Tokenizer<'a> {
    #[inline]
    pub fn new(input: &'a str) -> Self {
        Self {
            cursor: Charsor::new(input),
        }
    }

    #[inline]
    fn emit(&mut self, kind: TokenKind<'a>) -> Token<'a> {
        let start = self.cursor.prev_offset();
        let end = self.cursor.offset();

        Token::new(kind, Span::new(start as u16, end as u16))
    }

    #[inline]
    fn emit_ident(&mut self) -> Token<'a> {
        let start = self.cursor.prev_offset();
        self.cursor.eat_while(is_ident_continue);
        let end = self.cursor.offset();

        Token::new(
            TokenKind::Ident(self.cursor.slice_range(start..end)),
            Span::new(start as u16, end as u16),
        )
    }

    #[inline]
    fn emit_lit(&mut self) -> Token<'a> {
        let start = self.cursor.prev_offset();
        self.cursor.eat_while(|ch| ch.is_ascii_digit());
        let end = self.cursor.offset();

        Token::new(
            TokenKind::Lit(self.cursor.slice_range(start..end)),
            Span::new(start as u16, end as u16),
        )
    }

    #[inline]
    fn eat_whitespace(&mut self) {
        self.cursor.eat_while(|ch| ch.is_whitespace());
    }

    #[inline(always)]
    fn next_token(&mut self) -> Option<Result<Token<'a>>> {
        loop {
            let Some(ch) = self.cursor.next() else {
                break None;
            };

            match ch {
                '<' => break Some(Ok(self.emit(TokenKind::LAngle))),
                '>' => break Some(Ok(self.emit(TokenKind::RAngle))),
                '[' => break Some(Ok(self.emit(TokenKind::LSquare))),
                ']' => break Some(Ok(self.emit(TokenKind::RSquare))),
                '*' => break Some(Ok(self.emit(TokenKind::Asterisk))),
                ch if is_ident_start(ch) => break Some(Ok(self.emit_ident())),
                ch if ch.is_ascii_digit() => break Some(Ok(self.emit_lit())),
                ch if ch.is_whitespace() => {
                    self.eat_whitespace();
                }
                _ => break Some(Err(Error::UnknownChar(ch))),
            }
        }
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Result<Token<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}

#[cfg(test)]
mod tests {
    use expect_test::expect;

    use super::*;

    #[test]
    fn it_works() {
        const INPUT: &str = r#"
uint64[256]
CDOTAGameManager*
CNetworkUtlVectorBase< CHandle< CBasePlayerController > >
CHandle< CDOTASpecGraphPlayerData >[24]
CDOTA_AbilityDraftAbilityState[MAX_ABILITY_DRAFT_ABILITIES]
            "#;

        let tokenizer = Tokenizer::new(INPUT);
        let tokens: Vec<Result<Token>> = tokenizer.collect();

        let expected = expect![[r#"
            [
                Ok(
                    Token {
                        kind: Ident(
                            "uint64",
                        ),
                        span: Span { 1, 7 },
                    },
                ),
                Ok(
                    Token {
                        kind: LSquare,
                        span: Span { 7, 8 },
                    },
                ),
                Ok(
                    Token {
                        kind: Lit(
                            "256",
                        ),
                        span: Span { 8, 11 },
                    },
                ),
                Ok(
                    Token {
                        kind: RSquare,
                        span: Span { 11, 12 },
                    },
                ),
                Ok(
                    Token {
                        kind: Ident(
                            "CDOTAGameManager",
                        ),
                        span: Span { 13, 29 },
                    },
                ),
                Ok(
                    Token {
                        kind: Asterisk,
                        span: Span { 29, 30 },
                    },
                ),
                Ok(
                    Token {
                        kind: Ident(
                            "CNetworkUtlVectorBase",
                        ),
                        span: Span { 31, 52 },
                    },
                ),
                Ok(
                    Token {
                        kind: LAngle,
                        span: Span { 52, 53 },
                    },
                ),
                Ok(
                    Token {
                        kind: Ident(
                            "CHandle",
                        ),
                        span: Span { 54, 61 },
                    },
                ),
                Ok(
                    Token {
                        kind: LAngle,
                        span: Span { 61, 62 },
                    },
                ),
                Ok(
                    Token {
                        kind: Ident(
                            "CBasePlayerController",
                        ),
                        span: Span { 63, 84 },
                    },
                ),
                Ok(
                    Token {
                        kind: RAngle,
                        span: Span { 85, 86 },
                    },
                ),
                Ok(
                    Token {
                        kind: RAngle,
                        span: Span { 87, 88 },
                    },
                ),
                Ok(
                    Token {
                        kind: Ident(
                            "CHandle",
                        ),
                        span: Span { 89, 96 },
                    },
                ),
                Ok(
                    Token {
                        kind: LAngle,
                        span: Span { 96, 97 },
                    },
                ),
                Ok(
                    Token {
                        kind: Ident(
                            "CDOTASpecGraphPlayerData",
                        ),
                        span: Span { 98, 122 },
                    },
                ),
                Ok(
                    Token {
                        kind: RAngle,
                        span: Span { 123, 124 },
                    },
                ),
                Ok(
                    Token {
                        kind: LSquare,
                        span: Span { 124, 125 },
                    },
                ),
                Ok(
                    Token {
                        kind: Lit(
                            "24",
                        ),
                        span: Span { 125, 127 },
                    },
                ),
                Ok(
                    Token {
                        kind: RSquare,
                        span: Span { 127, 128 },
                    },
                ),
                Ok(
                    Token {
                        kind: Ident(
                            "CDOTA_AbilityDraftAbilityState",
                        ),
                        span: Span { 129, 159 },
                    },
                ),
                Ok(
                    Token {
                        kind: LSquare,
                        span: Span { 159, 160 },
                    },
                ),
                Ok(
                    Token {
                        kind: Ident(
                            "MAX_ABILITY_DRAFT_ABILITIES",
                        ),
                        span: Span { 160, 187 },
                    },
                ),
                Ok(
                    Token {
                        kind: RSquare,
                        span: Span { 187, 188 },
                    },
                ),
            ]
        "#]];
        expected.assert_debug_eq(&tokens);
    }
}

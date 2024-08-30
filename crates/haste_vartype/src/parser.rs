use crate::{Error, Result, Token, TokenKind, Tokenizer};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Lit<'a> {
    Str(&'a str),
    Num(usize),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Expr<'a> {
    Ident(&'a str),
    Lit(Lit<'a>),
    Pointer(Box<Expr<'a>>),
    Template {
        expr: Box<Expr<'a>>,
        arg: Box<Expr<'a>>,
    },
    Array {
        expr: Box<Expr<'a>>,
        len: Box<Expr<'a>>,
    },
}

#[derive(Debug, Clone)]
struct Parser<'a> {
    tokenizer: Tokenizer<'a>,
    prev_token: Option<Token<'a>>,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            tokenizer: Tokenizer::new(input),
            prev_token: None,
        }
    }

    #[inline(always)]
    fn next_token(&mut self) -> Result<Option<Token<'a>>> {
        if let Some(token) = self.prev_token.take() {
            Ok(Some(token))
        } else {
            self.tokenizer.next().transpose()
        }
    }

    #[inline(always)]
    fn expect_token<F>(&mut self, f: F) -> Result<Token<'a>>
    where
        F: FnOnce(&TokenKind<'a>) -> bool,
    {
        match self.next_token()? {
            Some(token) if f(&token.kind) => Ok(token),
            Some(token) => Err(Error::UnexpectedToken(token.span.start)),
            None => Err(Error::UnexpectedEof),
        }
    }

    fn parse(&mut self) -> Result<Expr<'a>> {
        let first = self.expect_token(|k| matches!(k, TokenKind::Ident(_)))?;
        let TokenKind::Ident(ident) = first.kind else {
            unreachable!();
        };
        let mut expr = Expr::Ident(ident);
        while let Some(next) = self.next_token()? {
            match next.kind {
                TokenKind::LAngle => {
                    let arg = self.parse()?;

                    let _rangle = self.expect_token(|k| matches!(k, TokenKind::RAngle))?;
                    expr = Expr::Template {
                        expr: Box::new(expr),
                        arg: Box::new(arg),
                    };
                }
                TokenKind::LSquare => {
                    let identorlit = self.expect_token(|_| true)?;
                    let len = match identorlit.kind {
                        TokenKind::Ident(ident) => Expr::Ident(ident),
                        TokenKind::Lit(lit) => {
                            let num = lit
                                .parse::<usize>()
                                .map_err(|_| Error::UnexpectedToken(identorlit.span.start))?;
                            Expr::Lit(Lit::Num(num))
                        }
                        _ => return Err(Error::UnexpectedToken(identorlit.span.start)),
                    };

                    let _rsquare = self.expect_token(|k| matches!(k, TokenKind::RSquare))?;
                    expr = Expr::Array {
                        expr: Box::new(expr),
                        len: Box::new(len),
                    };
                }
                TokenKind::Asterisk => {
                    expr = Expr::Pointer(Box::new(expr));
                }
                TokenKind::RAngle | TokenKind::RSquare => {
                    self.prev_token = Some(next);
                    break;
                }
                _ => return Err(Error::UnexpectedToken(next.span.start)),
            }
        }

        Ok(expr)
    }
}

pub fn parse<'a>(input: &'a str) -> Result<Expr<'a>> {
    Parser::new(input).parse()
}

#[cfg(test)]
mod tests {
    use expect_test::expect;

    use super::*;

    #[test]
    fn unexpected_eof() {
        let result = parse("");
        let expected = expect![[r#"
            Err(
                UnexpectedEof,
            )
        "#]];
        expected.assert_debug_eq(&result);
    }

    #[test]
    fn unexpected_token() {
        let result = parse("42");
        let expected = expect![[r#"
            Err(
                UnexpectedToken(
                    0,
                ),
            )
        "#]];
        expected.assert_debug_eq(&result);
    }

    #[test]
    fn it_works() -> Result<()> {
        const INPUTS: [&'static str; 5] = [
            "uint64[256]",
            "CDOTAGameManager*",
            "CNetworkUtlVectorBase< CHandle< CBasePlayerController > >",
            "CHandle< CDOTASpecGraphPlayerData >[24]",
            "CDOTA_AbilityDraftAbilityState[MAX_ABILITY_DRAFT_ABILITIES]",
        ];

        let mut outputs: Vec<Result<Expr<'static>>> = Vec::with_capacity(INPUTS.len());

        INPUTS.iter().for_each(|input| outputs.push(parse(input)));

        let expected = expect![[r#"
            [
                Ok(
                    Array {
                        expr: Ident(
                            "uint64",
                        ),
                        len: Lit(
                            Num(
                                256,
                            ),
                        ),
                    },
                ),
                Ok(
                    Pointer(
                        Ident(
                            "CDOTAGameManager",
                        ),
                    ),
                ),
                Ok(
                    Template {
                        expr: Ident(
                            "CNetworkUtlVectorBase",
                        ),
                        arg: Template {
                            expr: Ident(
                                "CHandle",
                            ),
                            arg: Ident(
                                "CBasePlayerController",
                            ),
                        },
                    },
                ),
                Ok(
                    Array {
                        expr: Template {
                            expr: Ident(
                                "CHandle",
                            ),
                            arg: Ident(
                                "CDOTASpecGraphPlayerData",
                            ),
                        },
                        len: Lit(
                            Num(
                                24,
                            ),
                        ),
                    },
                ),
                Ok(
                    Array {
                        expr: Ident(
                            "CDOTA_AbilityDraftAbilityState",
                        ),
                        len: Ident(
                            "MAX_ABILITY_DRAFT_ABILITIES",
                        ),
                    },
                ),
            ]
        "#]];
        expected.assert_debug_eq(&outputs);

        Ok(())
    }
}

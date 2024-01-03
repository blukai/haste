#[derive(Debug, PartialEq)]
pub enum Token<'a> {
    Ident(&'a str),
    // TODO: is there a more appropritate name for numeric values?
    Number(&'a str),
    LAngle,   // <
    RAngle,   // >
    LSquare,  // [
    RSquare,  // ]
    Asterisk, // *
}

// ----

pub struct Tokenizer<'a> {
    input: &'a str,
    bytes: &'a [u8],
    // NOTE: fwd stands for forward; it starts at 0, bump happens AFTER read.
    fwd_pos: usize,
    // NOTE: bwd stands for backward; it starts at len, bump happens BEFORE read.
    bwd_pos: usize,
}

impl<'a> Tokenizer<'a> {
    #[inline]
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            bytes: input.as_bytes(),
            fwd_pos: 0,
            bwd_pos: input.len(),
        }
    }
}

// ----

trait DirectedTokenizer<'a> {
    fn bump(&mut self);
    fn backup(&mut self);
    fn consume(&mut self) -> Option<char>;
    fn consume_while<P: FnMut(char) -> bool>(&mut self, predicate: P) -> &'a str;
}

// ----

struct ForwardTokenizer<'a, 'tokenizer>(&'a mut Tokenizer<'tokenizer>);

impl<'a, 'tokenizer> DirectedTokenizer<'tokenizer> for ForwardTokenizer<'a, 'tokenizer> {
    #[inline]
    fn bump(&mut self) {
        self.0.fwd_pos += 1;
    }

    #[inline]
    fn backup(&mut self) {
        self.0.fwd_pos -= 1;
    }

    #[inline]
    fn consume(&mut self) -> Option<char> {
        self.0
            .bytes
            .get(self.0.fwd_pos)
            .map(|b| *b as char)
            .and_then(|ch| {
                self.bump();
                Some(ch)
            })
    }

    #[inline]
    fn consume_while<P: FnMut(char) -> bool>(&mut self, mut predicate: P) -> &'tokenizer str {
        let start = self.0.fwd_pos;
        while let Some(ch) = self.consume() {
            if !predicate(ch) {
                self.backup();
                break;
            }
        }
        let end = self.0.fwd_pos;
        &self.0.input[start..end]
    }
}

// ----

struct BackwardTokenizer<'a, 'tokenizer>(&'a mut Tokenizer<'tokenizer>);

impl<'a, 'tokenizer> DirectedTokenizer<'tokenizer> for BackwardTokenizer<'a, 'tokenizer> {
    #[inline]
    fn bump(&mut self) {
        self.0.bwd_pos -= 1;
    }

    #[inline]
    fn backup(&mut self) {
        self.0.bwd_pos += 1;
    }

    #[inline]
    fn consume(&mut self) -> Option<char> {
        if self.0.bwd_pos > 0 {
            self.bump();
            self.0.bytes.get(self.0.bwd_pos).map(|b| *b as char)
        } else {
            None
        }
    }

    #[inline]
    fn consume_while<P: FnMut(char) -> bool>(&mut self, mut predicate: P) -> &'tokenizer str {
        let end = self.0.bwd_pos;
        while let Some(ch) = self.consume() {
            if !predicate(ch) {
                self.backup();
                break;
            }
        }
        let start = self.0.bwd_pos;
        &self.0.input[start..end]
    }
}

// ----

#[inline(always)]
fn next_token<'a>(mut tokenizer: impl DirectedTokenizer<'a>) -> Option<Token<'a>> {
    while let Some(ch) = tokenizer.consume() {
        match ch {
            ch if ch.is_ascii_alphanumeric() => {
                tokenizer.backup();
                let slice = tokenizer.consume_while(|ch| ch.is_ascii_alphanumeric() || ch.eq(&'_'));
                // NOTE: if slice starts with a number -> it's not an identifier
                match slice.chars().next() {
                    Some(first) if first.is_ascii_digit() => return Some(Token::Number(slice)),
                    _ => return Some(Token::Ident(slice)),
                }
            }
            '<' => return Some(Token::LAngle),
            '>' => return Some(Token::RAngle),
            '[' => return Some(Token::LSquare),
            ']' => return Some(Token::RSquare),
            '*' => return Some(Token::Asterisk),
            ' ' => {}
            _ => unreachable!("unexpected char: {ch}"),
        }
    }
    None
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Token<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        next_token(ForwardTokenizer(self))
    }
}

impl<'a> DoubleEndedIterator for Tokenizer<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        next_token(BackwardTokenizer(self))
    }
}

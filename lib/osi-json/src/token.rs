//! # JSON Token Handling
//!
//! XXX

/// ## Tokenizer Errors
///
/// XXX
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    /// Custom error for use by handler closures. Never created by the engine.
    Handler,
    /// Given character is not valid outside JSON values.
    CharacterInvalid(char),
    /// Given keyword is not valid in JSON.
    KeywordUnknown(alloc::string::String),
    /// Data ended with an unclosed string.
    StringIncomplete,
    /// Specified unescaped character is not valid in a string.
    StringCharacterInvalid(char),
    /// Specified escaped character is not a valid string escape code.
    StringEscapeInvalid(char),
    /// String escape sequence is incomplete.
    StringEscapeIncomplete,
    /// Unpaired lead or trail surrogates are not valid in strings.
    StringSurrogateUnpaired,
    /// String escape sequence produces invalid Unicode Scalar Value.
    StringEscapeUnicode,
}

/// ## JSON Token
///
/// XXX
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Token<'ctx> {
    Whitespace(&'ctx str),
    Colon,
    Comma,
    ArrayOpen,
    ArrayClose,
    ObjectOpen,
    ObjectClose,
    Null,
    True,
    False,
    Number,
    String(&'ctx str, &'ctx str),
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum State {
    None,
    Whitespace,
    Keyword,
    Number,
    String,
    StringEscape,
    StringUnicode(u8, u32),
    StringSurrogate(u32),
    StringSurrogateEscape(u32),
    StringSurrogateUnicode(u32, u8, u32),
}

/// ## Tokenizer Engine
///
/// The tokenizer engine takes an input stream of Unicode Scalar Values
/// and produces a stream of JSON tokens.
///
/// A single engine can be used to tokenize any number of JSON values. Once
/// a value has been fully tokenized, the engine is automatically reset.
#[derive(Clone, Debug)]
pub struct Engine {
    acc: alloc::string::String,
    acc_str: alloc::string::String,
    state: State,
}

impl Engine {
    /// ## Create New Tokenizer Engine
    ///
    /// XXX
    pub fn new() -> Self {
        Self {
            acc: alloc::string::String::new(),
            acc_str: alloc::string::String::new(),
            state: State::None,
        }
    }

    /// ## Reset Engine
    ///
    /// Reset the engine to the same state as when it was created. Internal
    /// buffers might remain allocated for performance reasons. However, any
    /// data is cleared.
    pub fn reset(&mut self) {
        self.acc.clear();
        self.acc_str.clear();
        self.state = State::None;
    }

    // Helper to parse JSON keywords to tokens. Returns `None` if the
    // keyword is not valid.
    fn keyword(data: &str) -> Option<Token> {
        match data {
            "null" => Some(Token::Null),
            "true" => Some(Token::True),
            "false" => Some(Token::False),
            _ => None,
        }
    }

    // Helper to convert a JSON escape code to its character value. Any
    // unknown escape codes are returned verbatim. It is up to the caller
    // to validate whether they are wanted or not.
    fn escape(data: char) -> char {
        match data {
            'b' => '\u{0008}',
            'f' => '\u{000c}',
            'n' => '\u{000a}',
            'r' => '\u{000d}',
            't' => '\u{0009}',
            v => v,
        }
    }

    fn fail(
        &mut self,
        err: Error,
    ) -> Error {
        self.acc.clear();
        self.acc_str.clear();
        self.state = State::None;
        err
    }

    fn raise<
        HandlerFn: FnMut(Token) -> Result<(), Error>,
        TokenFn: FnOnce(&Self) -> Result<Token, Error>,
    >(
        &mut self,
        handler: &mut HandlerFn,
        token_fn: TokenFn,
    ) -> Result<(), Error> {
        let r = token_fn(self).and_then(|v| handler(v));
        self.acc.clear();
        self.acc_str.clear();
        self.state = State::None;
        r
    }

    /// ## Push a Character into the Engine
    ///
    /// XXX
    pub fn push<
        HandlerFn: FnMut(Token) -> Result<(), Error>,
    >(
        &mut self,
        ch: Option<char>,
        handler: &mut HandlerFn,
    ) -> Result<(), Error> {
        // First try to push the next character into the current token
        // handler. If either no token is currently parsed, or if the
        // token cannot consume the character, it is returned as unhandled
        // and the token is finalized.
        let rem = match self.state {
            // If no token is currently parsed, return the character as
            // unhandled. It will then be parsed as start of a new token.
            State::None => ch,

            // If we currently parse whitespace, coalesce as much of it
            // into a single token as possible. Once the first
            // non-whitespace is encountered, finalize the token and return
            // the next character as unhandled.
            State::Whitespace => match ch {
                Some(v @ ' ')
                | Some(v @ '\n')
                | Some(v @ '\r')
                | Some(v @ '\t') => {
                    self.acc.push(v);
                    None
                },
                v => {
                    self.raise(handler, |v| Ok(Token::Whitespace(&v.acc)))?;
                    v
                },
            },

            // If we parse a keyword, we collect as many characters as
            // possible. Any alphanumeric character is collected (but the
            // token cannot start with one). Once a non-compatible
            // character is encountered, the token is finalized and the
            // character is returned as unhandled.
            // The keyword is parsed into one of the possible keywords
            // only when finalized. This allows error-reporting to consider
            // the entire identifier, instead of just a single unsupported
            // character.
            State::Keyword => match ch {
                Some(v @ '_' | v @ 'a'..='z'
                    | v @ 'A'..='Z' | v @ '0'..='9') => {
                    self.acc.push(v);
                    None
                },
                v => {
                    self.raise(
                        handler,
                        |v| Self::keyword(&v.acc)
                            .ok_or(Error::KeywordUnknown(v.acc.clone())),
                    )?;
                    v
                },
            },

            // XXX
            State::Number => match ch {
                _ => ch,
            },

            // When parsing a string, append all characters to the string.
            // If a quote is found, finalize the string. If a backslash is
            // found, parse the following characters as one of the possible
            // escape sequences.
            // Note that escape sequences are quite strict, and any
            // incomplete sequence causes the parser to fail.
            State::String => match ch {
                Some('"') => {
                    self.raise(
                        handler,
                        |v| Ok(Token::String(
                            &v.acc,
                            &v.acc_str,
                        )),
                    )?;
                    None
                },
                Some(v @ '\\') => {
                    self.acc.push(v);
                    self.state = State::StringEscape;
                    None
                },
                Some(v @ '\x20'..='\x21'
                    // '\x22' is '"'
                    | v @ '\x23'..='\x5b'
                    // '\x5c' is '\\'
                    | v @ '\x5d'..='\u{d7ff}'
                    // '\u{d800}'..='\u{dfff}' are surrogates
                    | v @ '\u{e000}'..='\u{10ffff}') => {
                    self.acc.push(v);
                    self.acc_str.push(v);
                    None
                },
                Some(v @ '\x00'..='\x1f') => {
                    return Err(self.fail(Error::StringCharacterInvalid(v)));
                },
                None => {
                    return Err(self.fail(Error::StringIncomplete));
                },
            },
            State::StringEscape => match ch {
                Some(v @ '"' | v @ '\\' | v @ '/' | v @ 'b'
                    | v @ 'f' | v @ 'n' | v @ 'r' | v @ 't') => {
                    self.acc.push(v);
                    self.acc_str.push(Self::escape(v));
                    self.state = State::String;
                    None
                },
                Some(v @ 'u') => {
                    self.acc.push(v);
                    self.state = State::StringUnicode(0, 0);
                    None
                },
                Some(v) => {
                    return Err(self.fail(Error::StringEscapeInvalid(v)));
                },
                None => {
                    return Err(self.fail(Error::StringIncomplete));
                },
            },

            // A unicode escape sequence always uses the form `\uXXXX`. No
            // shorter version is allowed. The `StringUnicode` state
            // remembers the number of digits parsed, as well as the
            // current value.
            // If a unicode escape encodes a lead-surrogate, it must be
            // followed immediately by an escape that encodes a
            // trail-surrogate. In this case the `StringSurrogate` state is
            // entered.
            // If the Unicode Code Point is not a valid Unicode Scalar
            // Value, nor a valid surrogate pair, it is rejected as
            // invalid.
            State::StringUnicode(num, value) => match ch {
                Some(v @ '0'..='9' | v @ 'a'..='f' | v @ 'A'..='F') => {
                    let value = (value << 4) | v.to_digit(16).unwrap();
                    self.acc.push(v);
                    if num < 3 {
                        // Increase the number of parsed digits by one and
                        // continue parsing until we got 4 total.
                        self.state = State::StringUnicode(num + 1, value);
                    } else if value >= 0xd800 && value <= 0xdbff {
                        // Got a lead-surrogate. It must be followed by a
                        // trail-surrogate immediately.
                        self.state = State::StringSurrogate(value);
                    } else if value >= 0xdc00 && value <= 0xdfff {
                        // Got an unpaired trail-surrogate. This is not
                        // allowed, so reject it straight away.
                        return Err(self.fail(Error::StringSurrogateUnpaired));
                    } else if let Some(v) = char::from_u32(value) {
                        // Got a valid Unicode Scalar Value.
                        self.acc_str.push(v);
                        self.state = State::String;
                    } else {
                        // Code-point is not a Unicode Scalar Value.
                        return Err(self.fail(Error::StringEscapeUnicode));
                    }
                    None
                },
                Some(_) => {
                    return Err(self.fail(Error::StringEscapeIncomplete));
                },
                None => {
                    return Err(self.fail(Error::StringIncomplete));
                },
            },
            State::StringSurrogate(lead) => match ch {
                Some(v @ '\\') => {
                    self.acc.push(v);
                    self.state = State::StringSurrogateEscape(lead);
                    None
                },
                Some(_) => {
                    return Err(self.fail(Error::StringSurrogateUnpaired));
                },
                None => {
                    return Err(self.fail(Error::StringIncomplete));
                },
            },
            State::StringSurrogateEscape(lead) => match ch {
                Some(v @ 'u') => {
                    self.acc.push(v);
                    self.state = State::StringSurrogateUnicode(lead, 0, 0);
                    None
                },
                Some(_) => {
                    return Err(self.fail(Error::StringSurrogateUnpaired));
                },
                None => {
                    return Err(self.fail(Error::StringIncomplete));
                },
            },
            State::StringSurrogateUnicode(lead, num, trail) => match ch {
                Some(v @ '0'..='9' | v @ 'a'..='f' | v @ 'A'..='F') => {
                    let trail = (trail << 4) | v.to_digit(16).unwrap();
                    self.acc.push(v);
                    if num < 3 {
                        // Increase the number of parsed digits by one and
                        // continue parsing until we got 4 total.
                        self.state = State::StringSurrogateUnicode(lead, num + 1, trail);
                    } else if trail >= 0xd800 && trail <= 0xdbff {
                        // This is another lead-surrogate, but we expected
                        // a trail-surrogate. Reject it.
                        return Err(self.fail(Error::StringSurrogateUnpaired));
                    } else if trail >= 0xdc00 && trail <= 0xdfff {
                        // This is a trail-surrogate following a
                        // lead-surrogate. This finalizes the surrogate
                        // pair and the string can continue normally.
                        self.acc_str.push(
                            char::from_u32(
                                0x10000
                                + ((lead - 0xd800) << 10)
                                + (trail - 0xdc00),
                            ).unwrap(),
                        );
                        self.state = State::String;
                    } else if let Some(_) = char::from_u32(trail) {
                        // We expected a trail-surrogate, but got a
                        // Unicode Scalar Value. Reject this.
                        return Err(self.fail(Error::StringSurrogateUnpaired));
                    } else {
                        // Code-point is not a Unicode Scalar Value.
                        return Err(self.fail(Error::StringEscapeUnicode));
                    }
                    None
                },
                Some(_) => {
                    return Err(self.fail(Error::StringEscapeIncomplete));
                },
                None => {
                    return Err(self.fail(Error::StringIncomplete));
                },
            },
        };

        // If the character was not handled, start parsing a new token
        // with it as first character.
        if let Some(v) = rem {
            match v {
                ' ' | '\n' | '\r' | '\t' => {
                    self.acc.push(v);
                    self.state = State::Whitespace;
                },
                ':' => {
                    self.raise(handler, |_| Ok(Token::Colon))?;
                },
                ',' => {
                    self.raise(handler, |_| Ok(Token::Comma))?;
                },
                '[' => {
                    self.raise(handler, |_| Ok(Token::ArrayOpen))?;
                },
                ']' => {
                    self.raise(handler, |_| Ok(Token::ArrayClose))?;
                },
                '{' => {
                    self.raise(handler, |_| Ok(Token::ObjectOpen))?;
                },
                '}' => {
                    self.raise(handler, |_| Ok(Token::ObjectClose))?;
                },
                '_' | 'a'..='z' | 'A'..='Z' => {
                    self.acc.push(v);
                    self.state = State::Keyword;
                },
                '-' | '0'..='9' => {
                    self.acc.push(v);
                    self.state = State::Number;
                },
                '"' => {
                    self.state = State::String;
                },
                v => return Err(self.fail(Error::CharacterInvalid(v))),
            };
        }

        Ok(())
    }

    /// ## Push a String into the Engine
    ///
    /// XXX
    pub fn push_str<
        HandlerFn: FnMut(Token) -> Result<(), Error>,
    >(
        &mut self,
        data: &str,
        handler: &mut HandlerFn,
    ) -> Result<(), Error> {
        for ch in data.chars() {
            self.push(Some(ch), handler)?;
        }
        Ok(())
    }

    /// ## Parse a String with the Engine
    ///
    /// Push the entire string into the tokenizer engine, followed by an
    /// End-Of-Input marker. See `Self::push()` for details.
    ///
    /// Note that this does not clear the engine before pushing the string
    /// into it. Hence, make sure to call this on a clean engine, unless
    /// it is meant to be pushed on top of the previous input.
    pub fn parse_str<
        HandlerFn: FnMut(Token) -> Result<(), Error>,
    >(
        &mut self,
        data: &str,
        handler: &mut HandlerFn,
    ) -> Result<(), Error> {
        for ch in data.chars() {
            self.push(Some(ch), handler)?;
        }
        self.push(None, handler)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Assert that the specified input tokenizes to the specified stream of
    // tokens. This also verifies that the tokenizer finishes successfully.
    fn assert_tokenize(
        from: &str,
        to: &alloc::vec::Vec<Token>,
    ) {
        let mut iter = to.iter();

        Engine::new().parse_str(
            from,
            &mut |v| {
                assert_eq!(
                    v,
                    *iter.next().unwrap(),
                );
                Ok(())
            },
        ).unwrap();

        assert_eq!(iter.next(), None);
    }

    // Whitespace Token Test
    //
    // Verify a set of whitespace input and check that the tokenizer correctly
    // groups it into whitespace tokens.
    #[test]
    fn token_whitespace() {
        assert_tokenize(
            "",
            &alloc::vec![],
        );
        assert_tokenize(
            " ",
            &alloc::vec![Token::Whitespace(" ")],
        );
        assert_tokenize(
            " \n\r\t",
            &alloc::vec![Token::Whitespace(" \n\r\t")],
        );
        assert_tokenize(
            " \n\r\tnull\t\r\n ",
            &alloc::vec![
                Token::Whitespace(" \n\r\t"),
                Token::Null,
                Token::Whitespace("\t\r\n "),
            ],
        );
    }

    // String Token Test
    //
    // Verify the string tokenizer on predefined input. Verify that it provides
    // both raw and decoded string data.
    #[test]
    fn token_string() {
        assert_tokenize(
            r#""foobar""#,
            &alloc::vec![
                Token::String("foobar", "foobar"),
            ],
        );
        assert_tokenize(
            r#""foo\nbar""#,
            &alloc::vec![
                Token::String(r#"foo\nbar"#, "foo\nbar"),
            ],
        );
    }
}

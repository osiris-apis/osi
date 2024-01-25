//! # JSON Tokenizer
//!
//! XXX

/// ## Tokenizer Flags
///
/// A set of flags that modify the behavior of the tokenizer engine. See
/// each flag description for details. Note that flags are constant over
/// the lifetime of a tokenizer and have to be specified when created.
pub type Flag = u32;

/// ## Allow Leading Zeroes
///
/// When set, the JSON tokenizer allows leading zeroes in JSON Number Values.
/// These leading zeroes will have no effect and are ignored.
pub const FLAG_ALLOW_LEADING_ZERO: Flag =       0x00000001;

/// ## Allow Plus Sign
///
/// When set, the JSON tokenizer allows leading plus signs in JSON Number
/// Values. These have no effect and are ignored.
pub const FLAG_ALLOW_PLUS_SIGN: Flag =          0x00000002;

/// ## Tokenizer Status
///
/// After every operation that advances the tokenizer, the latter will report
/// its current status as one of the possible items of this type.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Status {
    /// No token is currently parsed.
    #[default]
    Done,
    /// A token is still being parsed and requires more data.
    Busy,
}

/// ## Operational Report
///
/// This enum is used to report operational results of the tokenizer to
/// the caller. It is returned by all operations that advance the tokenizer.
///
/// The tokenizer only ever returns values of type
/// `core::ops::ControlFlow::Continue(Status)`, signaling that it is ready
/// to consume more data and continue operation. The status information can
/// be used to deduce the state of the tokenizer.
///
/// If a token handler is invoked by the tokenizer and returns values of type
/// `core::ops::ControlFlow::Break(T)`, it will cause a tokenizer reset, but
/// is otherwise propagated verbatim to the caller.
pub type Report<T> = core::ops::ControlFlow<T, Status>;

/// ## Number Signs
///
/// This enum is used to represent the sign a JSON Number Value carries.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Sign {
    /// Plus sign used for positive numbers.
    #[default]
    Plus,
    /// Minus sign used for negative numbers.
    Minus,
}

/// ## Error Tokens
///
/// This is the payload used by `Token::Error`. It conveys tokenizer errors to
/// the token handler, but otherwise allows the tokenizer to proceed and thus
/// report more possible errors in the same run.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Error<'ctx> {
    /// Given character is not valid outside JSON values.
    CharacterInvalid(char),
    /// Given character is not valid in the given combination.
    CharacterStray(char),
    /// Given whitespace character is not allowed in JSON.
    WhitespaceInvalid(char),
    /// Given keyword is not valid in JSON.
    KeywordUnknown(&'ctx str),
    /// Data ended with an unfinished number.
    NumberIncomplete,
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
    /// Comments are not supported by JSON.
    Comment(&'ctx str),
}

/// ## JSON Token
///
/// This enum represents all tokens that can be reported by the tokenizer. This
/// includes standard JSON tokens, but also extended tokens used for better
/// error diagnotics.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Token<'ctx> {
    /// Special token to report errors
    Error(Error<'ctx>),
    /// JSON colon token
    Colon,
    /// JSON comma token
    Comma,
    /// JSON open-array token
    ArrayOpen,
    /// JSON close-array token
    ArrayClose,
    /// JSON open-object token
    ObjectOpen,
    /// JSON close-object token
    ObjectClose,
    /// JSON null keyword
    Null,
    /// JSON true keyword
    True,
    /// JSON false keyword
    False,
    /// Block of continuous whitespace
    Whitespace(&'ctx str),
    /// JSON number value
    Number(&'ctx str, &'ctx [u8], Sign, usize, usize, Sign, usize),
    /// JSON string value
    String(&'ctx str, &'ctx str),
}

// ## Tokenizer State
//
// The internal state of the tokenizer. `State::None` is used when the
// tokenizer finished a token and has no associated state. Otherwise, the
// state identifies the current token and possible metadata.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
enum State {
    #[default]
    None,
    Slash,
    Keyword,
    Whitespace,
    NumberIntegerNone(Sign),
    NumberIntegerSome(Sign, usize),
    NumberIntegerZero(Sign),
    NumberFractionNone(Sign, usize),
    NumberFractionSome(Sign, usize, usize),
    NumberExponentNone(Sign, usize, usize),
    NumberExponentSign(Sign, usize, usize, Sign),
    NumberExponentSome(Sign, usize, usize, Sign, usize),
    String,
    StringEscape,
    StringUnicode(u8, u32),
    StringSurrogate(u32),
    StringSurrogateEscape(u32),
    StringSurrogateUnicode(u32, u8, u32),
    CommentLine,
}

/// ## Tokenizer Engine
///
/// The tokenizer engine takes an input stream of Unicode Scalar Values
/// and produces a stream of JSON tokens.
///
/// A single engine can be used to tokenize any number of JSON values. Once
/// a value has been fully tokenized, the engine is automatically reset and
/// ready to parse the next token.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct Tokenizer {
    flags: Flag,
    acc: alloc::string::String,
    acc_str: alloc::string::String,
    acc_num: alloc::vec::Vec<u8>,
    state: State,
}

impl Tokenizer {
    /// ## Create New Tokenizer
    ///
    /// Create a new tokenizer engine with the given parameters.
    ///
    /// A single tokenizer engine can be used to tokenize an arbitrary amount
    /// of JSON data.
    pub fn with(flags: Flag) -> Self {
        Self {
            flags: flags,
            ..Default::default()
        }
    }

    /// ## Create New Tokenizer
    ///
    /// Create a new tokenizer engine with the default parameters. See
    /// `Self::with()` for a detailed description.
    pub fn new() -> Self {
        Self::with(0)
    }

    // Clear current buffers and prepare for the next token. This should be
    // called after a token was finished.
    fn prepare(&mut self) {
        self.acc.clear();
        self.acc.shrink_to(4096);
        self.acc_str.clear();
        self.acc_str.shrink_to(4096);
        self.acc_num.clear();
        self.acc_num.shrink_to(4096);
        self.state = State::None;
    }

    /// ## Reset Tokenizer
    ///
    /// Reset the engine to the same state as when it was created. Internal
    /// buffers might remain allocated for performance reasons. However, any
    /// data is cleared.
    pub fn reset(&mut self) {
        self.prepare();
    }

    /// ## Report Status
    ///
    /// Report the status of the tokenizer engine. If a token is currently
    /// being processed, this will yield `Status::Busy`. Otherwise, it will
    /// yield `Status::Done`.
    pub fn status(&self) -> Status {
        match self.state {
            State::None => Status::Done,
            _ => Status::Busy,
        }
    }

    fn advance_misc<
        HandlerValue,
        HandlerFn: FnMut(Token) -> core::ops::ControlFlow<HandlerValue>,
    >(
        &mut self,
        ch: Option<char>,
        handler: &mut HandlerFn,
    ) -> core::ops::ControlFlow<HandlerValue, Option<char>> {
        match self.state {
            // Slashes can start line-comments, multi-line comments, as well
            // as be part of normal keywords. None of this is supported by
            // JSON, but we try to be a bit clever to get better diagnostics.
            State::Slash => match ch {
                Some(v @ '/') => {
                    self.acc.push(v);
                    self.state = State::CommentLine;
                    core::ops::ControlFlow::Continue(None)
                },
                Some(v) => {
                    self.acc.push(v);
                    self.state = State::Keyword;
                    core::ops::ControlFlow::Continue(None)
                },
                None => {
                    handler(Token::Error(Error::CharacterStray('/')))?;
                    self.prepare();
                    core::ops::ControlFlow::Continue(None)
                },
            },

            // If we parse a keyword we collect as many characters as possible.
            // Any alphanumeric character is collected. Once a non-compatible
            // character is encountered, the token is finalized and the
            // character is returned as unhandled. The keyword is parsed into
            // one of the possible keywords only when finalized. This allows
            // error-reporting to consider the entire identifier, instead of
            // just a single unsupported character. Similarly, we parse a lot
            // of characters into the token, which do not necessarily form
            // valid JSON keywords, but which lead to less obscure errors.
            State::Keyword => match ch {
                Some(v @ '_' | v @ 'a'..='z'
                    | v @ 'A'..='Z' | v @ '0'..='9') => {
                    self.acc.push(v);
                    core::ops::ControlFlow::Continue(None)
                },
                v => {
                    handler(
                        match self.acc.as_str() {
                            "null" => Token::Null,
                            "true" => Token::True,
                            "false" => Token::False,
                            _ => Token::Error(Error::KeywordUnknown(&self.acc)),
                        }
                    )?;
                    self.prepare();
                    core::ops::ControlFlow::Continue(v)
                },
            },

            // Merge as much whitespace into a single whitespace token as
            // possible. Once the first non-whitespace token is found, signal
            // the whitespace token and return the next character as unhandled.
            State::Whitespace => match ch {
                Some(v @ ' ')
                | Some(v @ '\n')
                | Some(v @ '\r')
                | Some(v @ '\t') => {
                    self.acc.push(v);
                    core::ops::ControlFlow::Continue(None)
                },
                Some(v) if v.is_whitespace() => {
                    handler(Token::Error(Error::WhitespaceInvalid(v)))?;
                    self.acc.push(v);
                    core::ops::ControlFlow::Continue(None)
                },
                v => {
                    handler(Token::Whitespace(&self.acc))?;
                    self.prepare();
                    core::ops::ControlFlow::Continue(v)
                },
            },

            // Line comments can start with '#' or '//' and are simply ignored
            // until the next new-line character. JSON does not support
            // comments, but we parse them for better diagnostics.
            State::CommentLine => match ch {
                Some('\n') => {
                    handler(Token::Error(Error::Comment(&self.acc)))?;
                    self.prepare();
                    core::ops::ControlFlow::Continue(ch)
                },
                Some(v) => {
                    self.acc.push(v);
                    core::ops::ControlFlow::Continue(None)
                },
                None => {
                    handler(Token::Error(Error::Comment(&self.acc)))?;
                    self.prepare();
                    core::ops::ControlFlow::Continue(None)
                },
            },

            _ => core::unreachable!(),
        }
    }

    fn advance_number<
        HandlerValue,
        HandlerFn: FnMut(Token) -> core::ops::ControlFlow<HandlerValue>,
    >(
        &mut self,
        ch: Option<char>,
        handler: &mut HandlerFn,
    ) -> core::ops::ControlFlow<HandlerValue, Option<char>> {
        // Parsing numbers is just a matter of parsing the components one
        // after another, where some components are optional. As usual, we
        // keep the unmodified number in the accumulator. However, we also
        // push all digits into a separate accumulator and remember how
        // many digits each component occupies. This allows much simpler
        // number conversions later on.
        match self.state {
            State::NumberIntegerNone(sign_int) => match ch {
                Some(v @ '0'..='9') => {
                    self.acc.push(v);
                    self.acc_num.push(
                        u8::try_from(v.to_digit(10).unwrap()).unwrap(),
                    );
                    if v == '0' {
                        self.state = State::NumberIntegerZero(sign_int);
                    } else {
                        self.state = State::NumberIntegerSome(sign_int, 1);
                    }
                    core::ops::ControlFlow::Continue(None)
                },
                Some(v) => {
                    handler(Token::Error(Error::CharacterStray(v)))?;
                    self.prepare();
                    core::ops::ControlFlow::Continue(Some(v))
                },
                None => {
                    handler(Token::Error(Error::NumberIncomplete))?;
                    self.prepare();
                    core::ops::ControlFlow::Continue(None)
                },
            },

            State::NumberIntegerSome(sign_int, n_int) => match ch {
                Some(v @ '0'..='9') => {
                    self.acc.push(v);
                    self.acc_num.push(
                        u8::try_from(v.to_digit(10).unwrap()).unwrap(),
                    );
                    self.state = State::NumberIntegerSome(sign_int, n_int + 1);
                    core::ops::ControlFlow::Continue(None)
                },
                Some('.') => {
                    self.state = State::NumberFractionNone(sign_int, n_int);
                    core::ops::ControlFlow::Continue(None)
                },
                Some(v @ 'e' | v @ 'E') => {
                    self.acc.push(v);
                    self.state = State::NumberExponentNone(
                        sign_int, n_int, 0,
                    );
                    core::ops::ControlFlow::Continue(None)
                },
                v => {
                    handler(Token::Number(
                        &self.acc, self.acc_num.as_slice(), sign_int, n_int, 0, Sign::Plus, 0,
                    ))?;
                    self.prepare();
                    core::ops::ControlFlow::Continue(v)
                },
            },
            State::NumberIntegerZero(sign_int) => match ch {
                Some('.') => {
                    self.state = State::NumberFractionNone(sign_int, 1);
                    core::ops::ControlFlow::Continue(None)
                },
                Some(v @ 'e' | v @ 'E') => {
                    self.acc.push(v);
                    self.state = State::NumberExponentNone(
                        sign_int, 1, 0,
                    );
                    core::ops::ControlFlow::Continue(None)
                },
                v => {
                    handler(Token::Number(
                        &self.acc, self.acc_num.as_slice(), sign_int, 1, 0, Sign::Plus, 0,
                    ))?;
                    self.prepare();
                    core::ops::ControlFlow::Continue(v)
                },
            },

            State::NumberFractionNone(sign_int, n_int) => match ch {
                Some(v @ '0'..='9') => {
                    self.acc.push(v);
                    self.acc_num.push(
                        u8::try_from(v.to_digit(10).unwrap()).unwrap(),
                    );
                    self.state = State::NumberFractionSome(sign_int, n_int, 1);
                    core::ops::ControlFlow::Continue(None)
                },
                Some(v) => {
                    handler(Token::Error(Error::CharacterStray(v)))?;
                    self.prepare();
                    core::ops::ControlFlow::Continue(Some(v))
                },
                None => {
                    handler(Token::Error(Error::NumberIncomplete))?;
                    self.prepare();
                    core::ops::ControlFlow::Continue(None)
                },
            },

            State::NumberFractionSome(sign_int, n_int, n_frac) => match ch {
                Some(v @ 'e' | v @ 'E') => {
                    self.acc.push(v);
                    self.state = State::NumberExponentNone(
                        sign_int, n_int, n_frac,
                    );
                    core::ops::ControlFlow::Continue(None)
                },
                Some(v @ '0'..='9') => {
                    self.acc.push(v);
                    self.acc_num.push(
                        u8::try_from(v.to_digit(10).unwrap()).unwrap(),
                    );
                    self.state = State::NumberFractionSome(sign_int, n_int, n_frac + 1);
                    core::ops::ControlFlow::Continue(None)
                },
                v => {
                    handler(Token::Number(
                        &self.acc, self.acc_num.as_slice(), sign_int, n_int, n_frac, Sign::Plus, 0,
                    ))?;
                    self.prepare();
                    core::ops::ControlFlow::Continue(v)
                },
            },

            State::NumberExponentNone(sign_int, n_int, n_frac) => match ch {
                Some(v @ '+') => {
                    self.acc.push(v);
                    self.state = State::NumberExponentSign(
                        sign_int, n_int, n_frac, Sign::Plus,
                    );
                    core::ops::ControlFlow::Continue(None)
                },
                Some(v @ '-') => {
                    self.acc.push(v);
                    self.state = State::NumberExponentSign(
                        sign_int, n_int, n_frac, Sign::Minus,
                    );
                    core::ops::ControlFlow::Continue(None)
                },
                Some(v @ '0'..='9') => {
                    self.acc.push(v);
                    self.acc_num.push(
                        u8::try_from(v.to_digit(10).unwrap()).unwrap(),
                    );
                    self.state = State::NumberExponentSome(
                        sign_int, n_int, n_frac, Sign::Plus, 1,
                    );
                    core::ops::ControlFlow::Continue(None)
                },
                Some(v) => {
                    handler(Token::Error(Error::CharacterStray(v)))?;
                    self.prepare();
                    core::ops::ControlFlow::Continue(Some(v))
                },
                None => {
                    handler(Token::Error(Error::NumberIncomplete))?;
                    self.prepare();
                    core::ops::ControlFlow::Continue(None)
                },
            },

            State::NumberExponentSign(sign_int, n_int, n_frac, sign_exp) => match ch {
                Some(v @ '0'..='9') => {
                    self.acc.push(v);
                    self.acc_num.push(
                        u8::try_from(v.to_digit(10).unwrap()).unwrap(),
                    );
                    self.state = State::NumberExponentSome(
                        sign_int, n_int, n_frac, sign_exp, 1,
                    );
                    core::ops::ControlFlow::Continue(None)
                },
                Some(v) => {
                    handler(Token::Error(Error::CharacterStray(v)))?;
                    self.prepare();
                    core::ops::ControlFlow::Continue(Some(v))
                },
                None => {
                    handler(Token::Error(Error::NumberIncomplete))?;
                    self.prepare();
                    core::ops::ControlFlow::Continue(None)
                },
            },

            State::NumberExponentSome(sign_int, n_int, n_frac, sign_exp, n_exp) => match ch {
                Some(v @ '0'..='9') => {
                    self.acc.push(v);
                    self.acc_num.push(
                        u8::try_from(v.to_digit(10).unwrap()).unwrap(),
                    );
                    self.state = State::NumberExponentSome(
                        sign_int, n_int, n_frac, sign_exp, n_exp + 1,
                    );
                    core::ops::ControlFlow::Continue(None)
                },
                v => {
                    handler(Token::Number(
                        &self.acc, self.acc_num.as_slice(), sign_int, n_int, n_frac, sign_exp, n_exp,
                    ))?;
                    self.prepare();
                    core::ops::ControlFlow::Continue(v)
                },
            },

            _ => core::unreachable!(),
        }
    }

    fn advance_string<
        HandlerValue,
        HandlerFn: FnMut(Token) -> core::ops::ControlFlow<HandlerValue>,
    >(
        &mut self,
        ch: Option<char>,
        handler: &mut HandlerFn,
    ) -> core::ops::ControlFlow<HandlerValue, Option<char>> {
        // Strings must be terminated with a quote. Therefore, we can handle
        // `None` early for all string states.
        let ch_value = match ch {
            None => {
                handler(Token::Error(Error::StringIncomplete))?;
                self.prepare();
                return core::ops::ControlFlow::Continue(None);
            },
            Some(v) => v,
        };

        // Parsing a string is just a matter of pushing characters into the
        // accumulator and tracking escape-sequences. Unicode-escapes make up
        // most of the complexity, since we must track surrogate pairs to avoid
        // strings with non-paired surrogate escapes.
        match self.state {
            State::String => match ch_value {
                '"' => {
                    handler(Token::String(&self.acc, &self.acc_str))?;
                    self.prepare();
                    core::ops::ControlFlow::Continue(None)
                },
                v @ '\\' => {
                    self.acc.push(v);
                    self.state = State::StringEscape;
                    core::ops::ControlFlow::Continue(None)
                },
                v @ '\x20'..='\x21'
                // '\x22' is '"'
                | v @ '\x23'..='\x5b'
                // '\x5c' is '\\'
                | v @ '\x5d'..='\u{d7ff}'
                // '\u{d800}'..='\u{dfff}' are surrogates
                | v @ '\u{e000}'..='\u{10ffff}' => {
                    self.acc.push(v);
                    self.acc_str.push(v);
                    core::ops::ControlFlow::Continue(None)
                },
                v @ '\x00'..='\x1f' => {
                    handler(Token::Error(Error::StringCharacterInvalid(v)))?;
                    self.acc.push(v);
                    self.acc_str.push(v);
                    core::ops::ControlFlow::Continue(None)
                },
            },

            State::StringEscape => match ch_value {
                v @ '"' | v @ '\\' | v @ '/' | v @ 'b'
                | v @ 'f' | v @ 'n' | v @ 'r' | v @ 't' => {
                    self.acc.push(v);
                    self.acc_str.push(
                        match v {
                            'b' => '\u{0008}',
                            'f' => '\u{000c}',
                            'n' => '\u{000a}',
                            'r' => '\u{000d}',
                            't' => '\u{0009}',
                            v => v,
                        },
                    );
                    self.state = State::String;
                    core::ops::ControlFlow::Continue(None)
                },
                v @ 'u' => {
                    self.acc.push(v);
                    self.state = State::StringUnicode(0, 0);
                    core::ops::ControlFlow::Continue(None)
                },
                v => {
                    handler(Token::Error(Error::StringEscapeInvalid(v)))?;
                    self.acc.push(v);
                    self.acc_str.push(v);
                    core::ops::ControlFlow::Continue(None)
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
            State::StringUnicode(num, value) => match ch_value {
                v @ '0'..='9' | v @ 'a'..='f' | v @ 'A'..='F' => {
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
                        handler(Token::Error(Error::StringSurrogateUnpaired))?;
                    } else if let Some(v) = char::from_u32(value) {
                        // Got a valid Unicode Scalar Value.
                        self.acc_str.push(v);
                        self.state = State::String;
                    } else {
                        // Code-point is not a Unicode Scalar Value.
                        handler(Token::Error(Error::StringEscapeUnicode))?;
                    }
                    core::ops::ControlFlow::Continue(None)
                },
                v => {
                    handler(Token::Error(Error::StringEscapeIncomplete))?;
                    self.acc.push(v);
                    core::ops::ControlFlow::Continue(None)
                },
            },

            State::StringSurrogate(lead) => match ch_value {
                v @ '\\' => {
                    self.acc.push(v);
                    self.state = State::StringSurrogateEscape(lead);
                    core::ops::ControlFlow::Continue(None)
                },
                v => {
                    handler(Token::Error(Error::StringSurrogateUnpaired))?;
                    self.acc.push(v);
                    core::ops::ControlFlow::Continue(None)
                },
            },

            State::StringSurrogateEscape(lead) => match ch_value {
                v @ 'u' => {
                    self.acc.push(v);
                    self.state = State::StringSurrogateUnicode(lead, 0, 0);
                    core::ops::ControlFlow::Continue(None)
                },
                v => {
                    handler(Token::Error(Error::StringSurrogateUnpaired))?;
                    self.acc.push(v);
                    core::ops::ControlFlow::Continue(None)
                },
            },

            State::StringSurrogateUnicode(lead, num, trail) => match ch_value {
                v @ '0'..='9' | v @ 'a'..='f' | v @ 'A'..='F' => {
                    let trail = (trail << 4) | v.to_digit(16).unwrap();
                    self.acc.push(v);
                    if num < 3 {
                        // Increase the number of parsed digits by one and
                        // continue parsing until we got 4 total.
                        self.state = State::StringSurrogateUnicode(lead, num + 1, trail);
                    } else if trail >= 0xd800 && trail <= 0xdbff {
                        // This is another lead-surrogate, but we expected
                        // a trail-surrogate. Reject it.
                        handler(Token::Error(Error::StringSurrogateUnpaired))?;
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
                        handler(Token::Error(Error::StringSurrogateUnpaired))?;
                    } else {
                        // Code-point is not a Unicode Scalar Value.
                        handler(Token::Error(Error::StringEscapeUnicode))?;
                    }
                    core::ops::ControlFlow::Continue(None)
                },
                v => {
                    handler(Token::Error(Error::StringEscapeIncomplete))?;
                    self.acc.push(v);
                    core::ops::ControlFlow::Continue(None)
                },
            },

            _ => core::unreachable!(),
        }
    }

    /// ## Push a Character into the Tokenizer
    ///
    /// Push a single character into the tokenizer and process it. This
    /// will advance the tokenizer state machine and report successfully
    /// parsed tokens to the specified handler closure.
    ///
    /// The tokenizer engine will always continue parsing and is always
    /// ready for more input. Errors are reported as special tokens and
    /// the tokenizer will try its best to recover and proceed. This
    /// allows reporting multiple errors in a single run. It is up to
    /// the caller to decide whether to ultimately reject the input or
    /// use the best-effort result of the tokenizer.
    ///
    /// Whenever a token is successfully parsed, the specified handler
    /// is invoked with the token as parameter. If the handler returns
    /// `core::ops::ControlFlow::Break(_)`, the tokenizer will reset its
    /// internal state and propagate the value the caller immediately.
    /// If the handler returns `core::ops::ControlFlow::Continue(())`,
    /// the tokenizer will continue its operation as normal.
    ///
    /// Some JSON Tokens are open-ended, meaning that the tokenizer needs
    /// to know about the end of the input. Hence, pushing `None` into
    /// the tokenizer will be interpreted as End-of-Input and finalize
    /// or cancel the final token.
    pub fn push<
        HandlerValue,
        HandlerFn: FnMut(Token) -> core::ops::ControlFlow<HandlerValue>,
    >(
        &mut self,
        ch: Option<char>,
        handler: &mut HandlerFn,
    ) -> Report<HandlerValue> {
        // First try to push the next character into the current token
        // handler. If either no token is currently parsed, or if the
        // token cannot consume the character, it is returned as unhandled
        // and the token is finalized.
        let rem = match self.state {
            State::None => core::ops::ControlFlow::Continue(ch),

            State::Slash
            | State::Keyword
            | State::Whitespace
            | State::CommentLine => {
                self.advance_misc(ch, handler)
            },

            State::NumberIntegerNone(_)
            | State::NumberIntegerSome(_, _)
            | State::NumberIntegerZero(_)
            | State::NumberFractionNone(_, _)
            | State::NumberFractionSome(_, _, _)
            | State::NumberExponentNone(_, _, _)
            | State::NumberExponentSign(_, _, _, _)
            | State::NumberExponentSome(_, _, _, _, _) => {
                self.advance_number(ch, handler)
            },

            State::String
            | State::StringEscape
            | State::StringUnicode(_, _)
            | State::StringSurrogate(_)
            | State::StringSurrogateEscape(_)
            | State::StringSurrogateUnicode(_, _, _) => {
                self.advance_string(ch, handler)
            },
        };

        match rem {
            // A handler yielded a break value. Reset the engine and propagate
            // the break to the caller. A break cannot be recovered from, so
            // the engine is always reset and prepared for a new run.
            core::ops::ControlFlow::Break(v) => {
                self.reset();
                return Report::Break(v);
            },

            // The character was successfully parsed, or signaled end-of-input.
            // No need to start a new token, but we can simply return to the
            // caller.
            core::ops::ControlFlow::Continue(None) => {},

            // Either no previous token was handled, or this character
            // finalized it. Either way, the character starts a new token.
            core::ops::ControlFlow::Continue(Some(v)) => match v {
                ':' => {
                    handler(Token::Colon)?;
                },
                ',' => {
                    handler(Token::Comma)?;
                },
                '[' => {
                    handler(Token::ArrayOpen)?;
                },
                ']' => {
                    handler(Token::ArrayClose)?;
                },
                '{' => {
                    handler(Token::ObjectOpen)?;
                },
                '}' => {
                    handler(Token::ObjectClose)?;
                },
                'a'..='z' | 'A'..='Z' => {
                    self.acc.push(v);
                    self.state = State::Keyword;
                },
                ' ' | '\n' | '\r' | '\t' => {
                    self.acc.push(v);
                    self.state = State::Whitespace;
                },
                '-' => {
                    self.acc.push(v);
                    self.state = State::NumberIntegerNone(Sign::Minus);
                },
                '0'..='9' => {
                    self.acc.push(v);
                    self.acc_num.push(
                        u8::try_from(v.to_digit(10).unwrap()).unwrap(),
                    );
                    if v == '0' {
                        self.state = State::NumberIntegerZero(Sign::Plus);
                    } else {
                        self.state = State::NumberIntegerSome(Sign::Plus, 1);
                    }
                },
                '"' => {
                    self.state = State::String;
                },
                '#' => {
                    self.state = State::CommentLine;
                },
                '\'' => {
                    // Single quotes are not allowed, so raise an error but
                    // then treat it as part of a keyword. We could try to
                    // parse it as single-quote string, but it is unclear
                    // whether it would yield better diagnostics.
                    handler(Token::Error(Error::CharacterInvalid(v)))?;
                    self.acc.push(v);
                    self.state = State::Keyword;
                },
                '(' | ')' => {
                    // Parentheses are not allowed, so raise an error and
                    // treat them as part of a keyword. We could try to
                    // match them and ignore anything in between to get
                    // better diagnostics. But for now we just do the
                    // simple thing and treat it as keyword.
                    handler(Token::Error(Error::CharacterInvalid(v)))?;
                    self.acc.push(v);
                    self.state = State::Keyword;
                },
                '+' => {
                    // A leading plus-sign is not allowed for JSON Number
                    // Values, yet it is very reasonable to support it. Raise
                    // an error, unless explicitly allowed, and then continue
                    // parsing the number.
                    if (self.flags & FLAG_ALLOW_PLUS_SIGN) == 0 {
                        handler(Token::Error(Error::CharacterInvalid(v)))?;
                    }
                    self.acc.push(v);
                    self.state = State::NumberIntegerNone(Sign::Plus);
                },
                '/' => {
                    // Slashes are not allowed, but are often used to start
                    // comments or combine expressions in other languages.
                    // Hence, try to be clever and do the same, so we get
                    // improved diagnostics.
                    self.acc.push(v);
                    self.state = State::Slash;
                },
                '=' => {
                    // Raise errors about equal signs, but then treat them as
                    // colons, as they usually serve similar purposes.
                    handler(Token::Error(Error::CharacterInvalid(v)))?;
                    handler(Token::Colon)?;
                },
                '`' => {
                    // Backticks are not allowed, but treat them as part of a
                    // keyword for diagnotics. We could try to match them and
                    // ignore anything in between, but it is unclear whether it
                    // would benefit diagnostics.
                    handler(Token::Error(Error::CharacterInvalid(v)))?;
                    self.acc.push(v);
                    self.state = State::Keyword;
                },
                '!' | '$' | '%' | '&' | '*' | '.' | '<' | '>'
                | '?' | '@' | '\\' | '^' | '_' | '|' | '~' => {
                    // Raise errors about these punctuation characters, but
                    // continue as if they are part of keywords, given that
                    // they are often used in special keywords elsewhere.
                    handler(Token::Error(Error::CharacterInvalid(v)))?;
                    self.acc.push(v);
                    self.state = State::Keyword;
                },
                v if v.is_ascii_punctuation() => {
                    // Raise errors about stray unsupported punctuation
                    // characters, but otherwise ignore them and continue.
                    handler(Token::Error(Error::CharacterInvalid(v)))?;
                },
                v if v.is_control() => {
                    // Raise errors about stray control characters, but
                    // otherwise ignore them and continue.
                    handler(Token::Error(Error::CharacterInvalid(v)))?;
                },
                v if v.is_whitespace() => {
                    // Raise errors about unsupported whitespace characters,
                    // but then include them in the whitespace token verbatim.
                    handler(Token::Error(Error::WhitespaceInvalid(v)))?;
                    self.acc.push(v);
                    self.state = State::Whitespace;
                },
                v if v.is_alphanumeric() => {
                    // Unsupported alphanumeric characters are simply treated
                    // as keywords. They will eventually lead to invalid
                    // keyword tokens, so no need to raise errors here.
                    self.acc.push(v);
                    self.state = State::Keyword;
                },
                v => {
                    // Any other character we simply treat as invalid and
                    // ignore it. There is nothing reasonable to do about it,
                    // as all other things have been handled before.
                    handler(Token::Error(Error::CharacterInvalid(v)))?;
                },
            },
        }

        Report::Continue(self.status())
    }

    /// ## Push a String into the Tokenizer
    ///
    /// Push an entire string into the tokenizer and process it. This is
    /// equivalent to iterating over the characters and pushing them into
    /// the tokenizer individually. See `Self::push()` for details.
    pub fn push_str<
        HandlerValue,
        HandlerFn: FnMut(Token) -> core::ops::ControlFlow<HandlerValue>,
    >(
        &mut self,
        data: &str,
        handler: &mut HandlerFn,
    ) -> Report<HandlerValue> {
        for ch in data.chars() {
            self.push(Some(ch), handler)?;
        }
        Report::Continue(self.status())
    }

    /// ## Parse a String with the Tokenizer
    ///
    /// Push the entire string into the tokenizer engine, followed by an
    /// End-Of-Input marker. See `Self::push()` for details.
    ///
    /// Note that this does not clear the engine before pushing the string
    /// into it. Hence, make sure to call this on a clean engine, unless
    /// it is meant to be pushed on top of the previous input.
    ///
    /// This will finalize the input and thus always reset the tokenizer
    /// before returning. Moreover, the tokenizer will always report a
    /// status of `Status::Done` when finished.
    pub fn parse_str<
        HandlerValue,
        HandlerFn: FnMut(Token) -> core::ops::ControlFlow<HandlerValue>,
    >(
        &mut self,
        data: &str,
        handler: &mut HandlerFn,
    ) -> Report<HandlerValue> {
        for ch in data.chars() {
            self.push(Some(ch), handler)?;
        }
        self.push(None, handler)?;
        Report::Continue(Status::Done)
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

        Tokenizer::new().parse_str(
            from,
            &mut |v| -> core::ops::ControlFlow<()> {
                assert_eq!(
                    v,
                    *iter.next().unwrap(),
                );
                core::ops::ControlFlow::Continue(())
            },
        );

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

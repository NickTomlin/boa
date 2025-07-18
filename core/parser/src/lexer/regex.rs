//! Boa's lexing for ECMAScript regex literals.

use crate::lexer::{Cursor, Error, Token, TokenKind, Tokenizer};
use crate::source::ReadChar;
use bitflags::bitflags;
use boa_ast::{Position, PositionGroup};
use boa_interner::{Interner, Sym};
use regress::{Flags, Regex};
use std::fmt::{Display, Write};
use std::str::{self, FromStr};

/// Regex literal lexing.
///
/// Lexes Division, Assigndiv or Regex literal.
///
/// Expects: Initial '/' to already be consumed by cursor.
///
/// More information:
///  - [ECMAScript reference][spec]
///  - [MDN documentation][mdn]
///
/// [spec]: https://tc39.es/ecma262/#sec-literals-regular-expression-literals
/// [mdn]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Regular_Expressions
#[derive(Debug, Clone, Copy)]
pub(super) struct RegexLiteral {
    // If there is more cases than only `/=`
    // then use `Option<u8>` or (more correct) `Option<enum>`
    init_with_eq: bool,
}

impl RegexLiteral {
    /// `init_with_eq` is '=' after `/` already consumed?
    pub(super) fn new(init_with_eq: bool) -> Self {
        Self { init_with_eq }
    }
}

impl<R> Tokenizer<R> for RegexLiteral {
    fn lex(
        &mut self,
        cursor: &mut Cursor<R>,
        start_pos: PositionGroup,
        interner: &mut Interner,
    ) -> Result<Token, Error>
    where
        R: ReadChar,
    {
        let mut body = Vec::new();
        if self.init_with_eq {
            body.push(u32::from(b'='));
        }

        let mut is_class_char = false;

        // Lex RegularExpressionBody.
        loop {
            match cursor.next_char()? {
                None => {
                    // Abrupt end.
                    return Err(Error::syntax(
                        "abrupt end on regular expression",
                        cursor.pos(),
                    ));
                }
                Some(b) => {
                    match b {
                        // /
                        0x2F if !is_class_char => break, // RegularExpressionBody finished.
                        // [
                        0x5B => {
                            is_class_char = true;
                            body.push(b);
                        }
                        // ]
                        0x5D if is_class_char => {
                            is_class_char = false;
                            body.push(b);
                        }
                        // \n | \r | \u{2028} | \u{2029}
                        0xA | 0xD | 0x2028 | 0x2029 => {
                            // Not allowed in Regex literal.
                            return Err(Error::syntax(
                                "new lines are not allowed in regular expressions",
                                cursor.pos(),
                            ));
                        }
                        // \
                        0x5C => {
                            // Escape sequence
                            body.push(b);
                            if let Some(sc) = cursor.next_char()? {
                                match sc {
                                    // \n | \r | \u{2028} | \u{2029}
                                    0xA | 0xD | 0x2028 | 0x2029 => {
                                        // Not allowed in Regex literal.
                                        return Err(Error::syntax(
                                            "new lines are not allowed in regular expressions",
                                            cursor.pos(),
                                        ));
                                    }
                                    b => body.push(b),
                                }
                            } else {
                                // Abrupt end of regex.
                                return Err(Error::syntax(
                                    "abrupt end on regular expression",
                                    cursor.pos(),
                                ));
                            }
                        }
                        _ => body.push(b),
                    }
                }
            }
        }

        let mut flags = Vec::new();
        let flags_start = cursor.pos();
        cursor.take_while_ascii_pred(&mut flags, &char::is_alphabetic)?;

        // SAFETY: We have already checked that the bytes are valid UTF-8.
        let flags_str = unsafe { str::from_utf8_unchecked(flags.as_slice()) };

        let mut body_utf16 = Vec::new();

        // We convert the body to UTF-16 since it may contain code points that are not valid UTF-8.
        // We already know that the body is valid UTF-16. Casting is fine.
        #[allow(clippy::cast_possible_truncation)]
        for cp in &body {
            let cp = *cp;
            if cp <= 0xFFFF {
                body_utf16.push(cp as u16);
            } else {
                let cp = cp - 0x1_0000;
                let high = 0xD800 | ((cp >> 10) as u16);
                let low = 0xDC00 | ((cp as u16) & 0x3FF);
                body_utf16.push(high);
                body_utf16.push(low);
            }
        }

        if let Err(error) = Regex::from_unicode(body.into_iter(), flags_str) {
            return Err(Error::syntax(
                format!("Invalid regular expression literal: {error}"),
                start_pos,
            ));
        }

        Ok(Token::new_by_position_group(
            TokenKind::regular_expression_literal(
                interner.get_or_intern(body_utf16.as_slice()),
                parse_regex_flags(flags_str, flags_start, interner)?,
            ),
            start_pos,
            cursor.pos_group(),
        ))
    }
}

bitflags! {
    /// Flags of a regular expression.
    #[derive(Debug, Default, Copy, Clone)]
    pub struct RegExpFlags: u8 {
        /// Whether to test the regular expression against all possible matches in a string,
        /// or only against the first.
        const GLOBAL = 0b0000_0001;

        /// Whether to ignore case while attempting a match in a string.
        const IGNORE_CASE = 0b0000_0010;

        /// Whether or not to search in strings across multiple lines.
        const MULTILINE = 0b0000_0100;

        /// Whether `.` matches newlines or not.
        const DOT_ALL = 0b0000_1000;

        /// Whether or not Unicode features are enabled.
        const UNICODE = 0b0001_0000;

        /// Whether or not the search is sticky.
        const STICKY = 0b0010_0000;

        /// Whether the regular expression result exposes the start and end indices of
        /// captured substrings.
        const HAS_INDICES = 0b0100_0000;

        /// Whether or not UnicodeSets features are enabled.
        const UNICODE_SETS = 0b1000_0000;
    }
}

impl FromStr for RegExpFlags {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut flags = Self::default();
        for c in s.bytes() {
            let new_flag = match c {
                b'g' => Self::GLOBAL,
                b'i' => Self::IGNORE_CASE,
                b'm' => Self::MULTILINE,
                b's' => Self::DOT_ALL,
                b'u' => Self::UNICODE,
                b'y' => Self::STICKY,
                b'd' => Self::HAS_INDICES,
                b'v' => Self::UNICODE_SETS,
                _ => return Err(format!("invalid regular expression flag {}", char::from(c))),
            };

            if flags.contains(new_flag) {
                return Err(format!(
                    "repeated regular expression flag {}",
                    char::from(c)
                ));
            }
            flags.insert(new_flag);
        }

        if flags.contains(Self::UNICODE) && flags.contains(Self::UNICODE_SETS) {
            return Err("cannot use both 'u' and 'v' flags".into());
        }

        Ok(flags)
    }
}

fn parse_regex_flags(s: &str, start: Position, interner: &mut Interner) -> Result<Sym, Error> {
    match RegExpFlags::from_str(s) {
        Err(message) => Err(Error::Syntax(message.into(), start)),
        Ok(flags) => Ok(interner.get_or_intern(flags.to_string().as_str())),
    }
}

impl Display for RegExpFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.contains(Self::HAS_INDICES) {
            f.write_char('d')?;
        }
        if self.contains(Self::GLOBAL) {
            f.write_char('g')?;
        }
        if self.contains(Self::IGNORE_CASE) {
            f.write_char('i')?;
        }
        if self.contains(Self::MULTILINE) {
            f.write_char('m')?;
        }
        if self.contains(Self::DOT_ALL) {
            f.write_char('s')?;
        }
        if self.contains(Self::UNICODE) {
            f.write_char('u')?;
        }
        if self.contains(Self::STICKY) {
            f.write_char('y')?;
        }
        if self.contains(Self::UNICODE_SETS) {
            f.write_char('v')?;
        }
        Ok(())
    }
}

impl From<RegExpFlags> for Flags {
    fn from(value: RegExpFlags) -> Self {
        Self {
            icase: value.contains(RegExpFlags::IGNORE_CASE),
            multiline: value.contains(RegExpFlags::MULTILINE),
            dot_all: value.contains(RegExpFlags::DOT_ALL),
            unicode: value.contains(RegExpFlags::UNICODE),
            unicode_sets: value.contains(RegExpFlags::UNICODE_SETS),
            ..Self::default()
        }
    }
}

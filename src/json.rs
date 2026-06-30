//! A small JSON reader and writer over [`Value`].
//!
//! The writer matches `JSON.stringify`: the same string escaping, the same
//! `key: value` spacing under indentation, the same handling of non-finite
//! numbers. The reader accepts any standard JSON document. Neither side ever
//! sees a cycle, because flatted text is a flat array of acyclic nodes.

use crate::value::{Number, Object, Value};

/// A JSON parse error with a byte offset into the input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// Human readable reason.
    pub message: String,
    /// Byte offset where parsing stopped.
    pub position: usize,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at byte {}", self.message, self.position)
    }
}

impl std::error::Error for ParseError {}

/// How to indent serialized output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Indent {
    /// No indentation. Compact output, no spaces.
    None,
    /// Indent each level by this literal prefix.
    With(String),
}

/// Serialize one acyclic [`Value`] to JSON text.
///
/// `indent` controls pretty printing. With [`Indent::None`] the output is
/// compact. Otherwise each nesting level repeats the indent string, and object
/// keys get `": "` after them, matching `JSON.stringify(value, null, space)`.
pub fn write(value: &Value, indent: &Indent) -> String {
    let mut out = String::new();
    match indent {
        Indent::None => write_compact(value, &mut out),
        Indent::With(unit) => write_pretty(value, unit, 0, &mut out),
    }
    out
}

fn write_compact(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(n) => n.write(out),
        Value::Str(s) => write_string(s, out),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.borrow().iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_compact(item, out);
            }
            out.push(']');
        }
        Value::Object(object) => {
            out.push('{');
            for (i, (key, val)) in object.borrow().iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_string(key, out);
                out.push(':');
                write_compact(val, out);
            }
            out.push('}');
        }
    }
}

fn write_pretty(value: &Value, unit: &str, depth: usize, out: &mut String) {
    match value {
        Value::Array(items) => {
            let items = items.borrow();
            if items.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push('\n');
                push_indent(unit, depth + 1, out);
                write_pretty(item, unit, depth + 1, out);
            }
            out.push('\n');
            push_indent(unit, depth, out);
            out.push(']');
        }
        Value::Object(object) => {
            let object = object.borrow();
            if object.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push('{');
            for (i, (key, val)) in object.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push('\n');
                push_indent(unit, depth + 1, out);
                write_string(key, out);
                out.push_str(": ");
                write_pretty(val, unit, depth + 1, out);
            }
            out.push('\n');
            push_indent(unit, depth, out);
            out.push('}');
        }
        other => write_compact(other, out),
    }
}

fn push_indent(unit: &str, depth: usize, out: &mut String) {
    for _ in 0..depth {
        out.push_str(unit);
    }
}

/// Escape and quote a string the way `JSON.stringify` does.
///
/// Quotes, backslashes, and the C0 control range get escaped. The short forms
/// `\b \t \n \f \r` are used where they exist, otherwise `\u00xx`. Everything
/// else, including non-ASCII, passes through as UTF-8.
fn write_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{09}' => out.push_str("\\t"),
            '\u{0a}' => out.push_str("\\n"),
            '\u{0c}' => out.push_str("\\f"),
            '\u{0d}' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Parse JSON text into a [`Value`].
///
/// Accepts a single JSON document. Trailing non-whitespace is an error.
pub fn read(text: &str) -> Result<Value, ParseError> {
    let mut parser = Parser {
        bytes: text.as_bytes(),
        text,
        pos: 0,
    };
    parser.skip_ws();
    let value = parser.parse_value()?;
    parser.skip_ws();
    if parser.pos != parser.bytes.len() {
        return Err(parser.error("trailing characters after JSON value"));
    }
    Ok(value)
}

struct Parser<'a> {
    bytes: &'a [u8],
    text: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn error(&self, message: &str) -> ParseError {
        ParseError {
            message: message.to_string(),
            position: self.pos,
        }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() {
            match self.bytes[self.pos] {
                b' ' | b'\t' | b'\n' | b'\r' => self.pos += 1,
                _ => break,
            }
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn parse_value(&mut self) -> Result<Value, ParseError> {
        match self.peek() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => Ok(Value::Str(self.parse_string()?)),
            Some(b't') => self.parse_literal("true", Value::Bool(true)),
            Some(b'f') => self.parse_literal("false", Value::Bool(false)),
            Some(b'n') => self.parse_literal("null", Value::Null),
            Some(c) if c == b'-' || c.is_ascii_digit() => self.parse_number(),
            _ => Err(self.error("expected a JSON value")),
        }
    }

    fn parse_literal(&mut self, word: &str, value: Value) -> Result<Value, ParseError> {
        if self.text[self.pos..].starts_with(word) {
            self.pos += word.len();
            Ok(value)
        } else {
            Err(self.error("invalid literal"))
        }
    }

    fn parse_array(&mut self) -> Result<Value, ParseError> {
        self.pos += 1; // consume '['
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Value::array(items));
        }
        loop {
            self.skip_ws();
            items.push(self.parse_value()?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(self.error("expected ',' or ']' in array")),
            }
        }
        Ok(Value::array(items))
    }

    fn parse_object(&mut self) -> Result<Value, ParseError> {
        self.pos += 1; // consume '{'
        let mut object = Object::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Value::object(object));
        }
        loop {
            self.skip_ws();
            if self.peek() != Some(b'"') {
                return Err(self.error("expected string key in object"));
            }
            let key = self.parse_string()?;
            self.skip_ws();
            if self.peek() != Some(b':') {
                return Err(self.error("expected ':' after object key"));
            }
            self.pos += 1;
            self.skip_ws();
            let value = self.parse_value()?;
            object.insert(key, value);
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(self.error("expected ',' or '}' in object")),
            }
        }
        Ok(Value::object(object))
    }

    fn parse_string(&mut self) -> Result<String, ParseError> {
        self.pos += 1; // consume opening quote
        let mut s = String::new();
        loop {
            let c = match self.peek() {
                Some(c) => c,
                None => return Err(self.error("unterminated string")),
            };
            match c {
                b'"' => {
                    self.pos += 1;
                    return Ok(s);
                }
                b'\\' => {
                    self.pos += 1;
                    self.parse_escape(&mut s)?;
                }
                c if c < 0x20 => return Err(self.error("control character in string")),
                _ => {
                    // Copy one UTF-8 char from the source.
                    let rest = &self.text[self.pos..];
                    let ch = rest.chars().next().unwrap();
                    s.push(ch);
                    self.pos += ch.len_utf8();
                }
            }
        }
    }

    fn parse_escape(&mut self, out: &mut String) -> Result<(), ParseError> {
        let c = match self.peek() {
            Some(c) => c,
            None => return Err(self.error("unterminated escape")),
        };
        self.pos += 1;
        match c {
            b'"' => out.push('"'),
            b'\\' => out.push('\\'),
            b'/' => out.push('/'),
            b'b' => out.push('\u{08}'),
            b'f' => out.push('\u{0c}'),
            b'n' => out.push('\n'),
            b'r' => out.push('\r'),
            b't' => out.push('\t'),
            b'u' => {
                let cp = self.parse_hex4()?;
                if (0xD800..=0xDBFF).contains(&cp) {
                    // High surrogate. Expect a low surrogate next.
                    if self.peek() == Some(b'\\') {
                        self.pos += 1;
                        if self.peek() == Some(b'u') {
                            self.pos += 1;
                            let low = self.parse_hex4()?;
                            if (0xDC00..=0xDFFF).contains(&low) {
                                let combined = 0x10000 + ((cp - 0xD800) << 10) + (low - 0xDC00);
                                match char::from_u32(combined) {
                                    Some(ch) => out.push(ch),
                                    None => return Err(self.error("invalid surrogate pair")),
                                }
                                return Ok(());
                            }
                        }
                    }
                    return Err(self.error("unpaired high surrogate"));
                }
                match char::from_u32(cp) {
                    Some(ch) => out.push(ch),
                    None => return Err(self.error("invalid unicode escape")),
                }
            }
            _ => return Err(self.error("invalid escape character")),
        }
        Ok(())
    }

    fn parse_hex4(&mut self) -> Result<u32, ParseError> {
        if self.pos + 4 > self.bytes.len() {
            return Err(self.error("truncated unicode escape"));
        }
        // Read four hex digits as bytes. Slicing the string here would panic
        // when a multi-byte character starts inside the four-byte window.
        let mut value = 0u32;
        for _ in 0..4 {
            let digit = match self.bytes[self.pos] {
                b @ b'0'..=b'9' => b - b'0',
                b @ b'a'..=b'f' => b - b'a' + 10,
                b @ b'A'..=b'F' => b - b'A' + 10,
                _ => return Err(self.error("invalid hex in unicode escape")),
            };
            value = value * 16 + u32::from(digit);
            self.pos += 1;
        }
        Ok(value)
    }

    /// Advance over a run of ASCII digits. Return how many were consumed.
    fn consume_digits(&mut self) -> usize {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.pos += 1;
        }
        self.pos - start
    }

    fn parse_number(&mut self) -> Result<Value, ParseError> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }

        // Integer part. JSON allows a single `0` or a run that does not start
        // with `0`. So `01` is rejected, `0` and `10` are fine.
        match self.peek() {
            Some(b'0') => {
                self.pos += 1;
                if matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                    return Err(self.error("leading zero in number"));
                }
            }
            Some(c) if c.is_ascii_digit() => {
                self.consume_digits();
            }
            _ => return Err(self.error("expected a digit in number")),
        }

        let mut is_float = false;
        if self.peek() == Some(b'.') {
            is_float = true;
            self.pos += 1;
            // A decimal point needs at least one digit after it.
            if self.consume_digits() == 0 {
                return Err(self.error("missing digit after decimal point"));
            }
        }
        if matches!(self.peek(), Some(b'e') | Some(b'E')) {
            is_float = true;
            self.pos += 1;
            if matches!(self.peek(), Some(b'+') | Some(b'-')) {
                self.pos += 1;
            }
            // An exponent needs at least one digit.
            if self.consume_digits() == 0 {
                return Err(self.error("missing digit in exponent"));
            }
        }
        let slice = &self.text[start..self.pos];
        if is_float {
            slice
                .parse::<f64>()
                .map(|f| Value::Number(Number::Float(f)))
                .map_err(|_| self.error("invalid number"))
        } else if let Ok(n) = slice.parse::<i64>() {
            Ok(Value::Number(Number::Int(n)))
        } else if let Ok(n) = slice.parse::<u64>() {
            Ok(Value::Number(Number::UInt(n)))
        } else {
            slice
                .parse::<f64>()
                .map(|f| Value::Number(Number::Float(f)))
                .map_err(|_| self.error("invalid number"))
        }
    }
}

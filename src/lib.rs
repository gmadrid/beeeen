use std::collections::HashMap;
use std::io::Read;
use std::iter::Peekable;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BEError {
    #[error("unexpected EOF")]
    EOFError,

    // TODO: propagate these.
    #[error("unknown IO error.")]
    IOError,

    #[error("keys must be strings, got {0:?}")]
    KeyNotString(BEValue),

    #[error("Leading '0' not permitted in integer")]
    LeadZeroError,

    #[error("missing prefix character, expected: {1}, found: {0}")]
    MissingPrefixError(u8, u8),

    #[error("missing separator character, expected: {1}, found: {0}")]
    MissingSeparatorError(u8, u8),

    #[error("missing suffix character, expected: {1}, found: {0}")]
    MissingSuffixError(u8, u8),

    #[error("key, '{0}', is missing a value")]
    MissingValueError(String),

    #[error("ParseIntError: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),

    #[error("unexpected character: {0}")]
    UnexpectedCharError(char),

    #[error("Utf8Error: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),
}

pub type Result<T> = std::result::Result<T, BEError>;

const COLON_CHAR: u8 = 0x3a;
const D_CHAR: u8 = 0x64;
const E_CHAR: u8 = 0x65;
const I_CHAR: u8 = 0x69;
const L_CHAR: u8 = 0x6c;
const MINUS_SIGN: u8 = 0x2d;
const ZERO_CHAR: u8 = 0x30;

#[derive(Debug)]
pub enum BEValue {
    BEDict(HashMap<String, BEValue>),
    BEInteger(i32),
    BEList(Vec<BEValue>),
    BEString(String),
}

impl BEValue {
    pub fn string(&self) -> String {
        match self {
            BEValue::BEString(s) => s.clone(),
            _ => panic!("string() called on non-string"),
        }
    }

    pub fn integer(&self) -> i32 {
        match self {
            BEValue::BEInteger(i) => *i,
            _ => panic!("integer() called on non-integer"),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            BEValue::BEInteger(_) => 1,
            BEValue::BEString(s) => s.len(),
            BEValue::BEList(l) => l.len(),
            BEValue::BEDict(d) => d.len(),
        }
    }
}

// Peek returns a complicated Option<Result<u8>>.
// This enum wraps that value in a slightly more descriptive type.
#[derive(Debug)]
enum PeekedValue {
    EOF,
    ASCII(u8),
}

pub struct BEReader<R>
where
    R: Read,
{
    chars: Peekable<std::io::Bytes<R>>,
}

impl<R> BEReader<R>
where
    R: Read,
{
    pub fn new(read: R) -> Self {
        BEReader {
            chars: read.bytes().peekable(),
        }
    }

    fn peeked_char(&mut self) -> Result<PeekedValue> {
        match self.chars.peek() {
            None => Ok(PeekedValue::EOF),
            // TODO: get this error out of here somehow.
            Some(Err(_)) => Err(BEError::IOError),
            Some(Ok(ch)) => Ok(PeekedValue::ASCII(*ch)),
        }
    }

    fn next_char_no_eof(&mut self) -> Result<u8> {
        match self.chars.next() {
            None => Err(BEError::EOFError),
            // TODO: propagate this test_read_error
            Some(Err(_)) => Err(BEError::IOError),
            Some(Ok(ch)) => Ok(ch),
        }
    }

    pub fn next_value(&mut self) -> Result<Option<BEValue>> {
        match self.peeked_char()? {
            PeekedValue::EOF => Ok(None),
            PeekedValue::ASCII(ch) if ch.is_ascii_digit() => Ok(Some(self.read_string()?)),
            PeekedValue::ASCII(I_CHAR) => Ok(Some(self.read_integer()?)),
            PeekedValue::ASCII(L_CHAR) => Ok(Some(self.read_list()?)),
            PeekedValue::ASCII(D_CHAR) => Ok(Some(self.read_dict()?)),
            PeekedValue::ASCII(ch) => Err(BEError::UnexpectedCharError(ch as char)),
        }
    }

    fn read_dict(&mut self) -> Result<BEValue> {
        let prefix = self.next_char_no_eof()?;
        if D_CHAR != prefix {
            return Err(BEError::MissingPrefixError(prefix, D_CHAR));
        }

        let mut dict = HashMap::new();

        loop {
            let key = match self.peeked_char()? {
                PeekedValue::EOF => return Err(BEError::EOFError),
                PeekedValue::ASCII(E_CHAR) => break,
                _ => {
                    let value = self.next_value()?;
                    match value {
                        None => return Err(BEError::EOFError),
                        Some(BEValue::BEString(s)) => s,
                        Some(v) => {
                            return Err(BEError::KeyNotString(v));
                        }
                    }
                }
            };

            let value = match self.peeked_char()? {
                PeekedValue::EOF => return Err(BEError::EOFError),
                PeekedValue::ASCII(E_CHAR) => return Err(BEError::MissingValueError(key)),
                _ => {
                    let value = self.next_value()?;
                    match value {
                        None => return Err(BEError::EOFError),
                        Some(v) => v,
                    }
                }
            };

            dict.insert(key, value);
        }

        Ok(BEValue::BEDict(dict))
    }

    fn read_list(&mut self) -> Result<BEValue> {
        let prefix = self.next_char_no_eof()?;
        if L_CHAR != prefix {
            return Err(BEError::MissingPrefixError(prefix, L_CHAR));
        }

        let mut result = Vec::default();
        loop {
            let peek = self.peeked_char()?;
            match peek {
                PeekedValue::EOF => return Err(BEError::EOFError),
                PeekedValue::ASCII(E_CHAR) => break,
                _ => {
                    let value = self.next_value()?;
                    match value {
                        None => return Err(BEError::EOFError),
                        Some(v) => result.push(v),
                    }
                }
            }
        }

        Ok(BEValue::BEList(result))
    }

    fn read_integer(&mut self) -> Result<BEValue> {
        let prefix = self.next_char_no_eof()?;
        if I_CHAR != prefix {
            return Err(BEError::MissingPrefixError(prefix, I_CHAR));
        }
        let value = self.read_raw_integer()?;

        let suffix = self.next_char_no_eof()?;
        if E_CHAR != suffix {
            return Err(BEError::MissingSuffixError(suffix, E_CHAR));
        }
        Ok(BEValue::BEInteger(value))
    }

    fn read_string(&mut self) -> Result<BEValue> {
        // TODO: deal with range check
        let len = self.read_raw_integer()? as usize;

        let separator = self.next_char_no_eof()?;
        if COLON_CHAR != separator {
            return Err(BEError::MissingSeparatorError(separator, COLON_CHAR));
        }

        let mut buf = [0u8; 256];
        for index in 0..len {
            buf[index] = self.next_char_no_eof()?;
        }
        Ok(BEValue::BEString(
            std::str::from_utf8(&buf[0..len])?.to_string(),
        ))
    }

    fn read_raw_integer(&mut self) -> Result<i32> {
        // TODO: deal with range check
        let mut buf = [0u8; 100];
        let mut index = 0;
        let mut minus = 1i32;
        let mut lead_zero = false;

        // Check for minus sign.
        if let PeekedValue::ASCII(MINUS_SIGN) = self.peeked_char()? {
            self.chars.next();
            minus = -1;
        }

        loop {
            match self.peeked_char()? {
                PeekedValue::EOF => {
                    if index == 0 {
                        return Err(BEError::EOFError);
                    }
                    break;
                }
                PeekedValue::ASCII(ch) if ch.is_ascii_digit() => {
                    if index > 0 && lead_zero {
                        return Err(BEError::LeadZeroError);
                    }
                    if index == 0 && ch == ZERO_CHAR {
                        lead_zero = true;
                    }
                    buf[index] = ch;
                    index += 1;
                    self.chars.next();
                }
                _ => break,
            }
        }

        let value: i32 = str::parse(std::str::from_utf8(&buf[0..index])?)?;
        Ok(value * minus)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reader(s: &'static str) -> BEReader<impl Read> {
        BEReader::new(s.as_bytes())
    }

    #[test]
    fn test_empty() {
        let mut ber = BEReader::new("".as_bytes());
        let value = ber.next_value().unwrap();

        assert!(value.is_none());
    }

    #[test]
    fn test_read_error() {
        // TODO: write this.
    }

    #[test]
    fn test_read_integer() {
        let mut ber = reader("i45e");
        let value = ber.next_value().unwrap();
        assert_eq!(value.unwrap().integer(), 45);

        let mut ber = reader("i-45e");
        let value = ber.next_value().unwrap();
        assert_eq!(value.unwrap().integer(), -45);

        let mut ber = reader("i0e");
        let value = ber.next_value().unwrap();
        assert_eq!(value.unwrap().integer(), 0);
    }

    #[test]
    fn test_integer_missing_e() {
        // Missing suffix.
        let mut ber = reader("i32");
        let value = ber.next_value();
        assert!(value.is_err());
    }

    #[test]
    fn test_leading_zero() {
        // Leading zero not allowed.
        let mut ber = reader("i032e");
        let value = ber.next_value();
        assert!(value.is_err());
    }

    #[test]
    fn test_read_string() {
        // Empty string
        let mut ber = BEReader::new("0:".as_bytes());
        let value = ber.next_value().unwrap();
        assert_eq!(value.unwrap().string(), "");

        // One digit length
        let mut ber = BEReader::new("7:unicorn".as_bytes());
        let value = ber.next_value().unwrap();
        assert_eq!(value.unwrap().string(), "unicorn");

        // Two digit length
        let mut ber = BEReader::new("12:unicornfarts".as_bytes());
        let value = ber.next_value().unwrap();
        assert_eq!(value.unwrap().string(), "unicornfarts");

        // String longer than length
        let mut ber = BEReader::new("11:unicornfarts".as_bytes());
        let value = ber.next_value().unwrap();
        assert_eq!(value.unwrap().string(), "unicornfart");

        // String containing a number
        let mut ber = reader("4:1234");
        let value = ber.next_value().unwrap().unwrap();
        assert_eq!(value.string(), "1234");

        // TODO: add some error cases here.
    }

    #[test]
    fn test_read_list() {
        // empty list
        let mut ber = reader("le");
        let value = ber.next_value().unwrap().unwrap();
        assert_eq!(value.len(), 0);

        let mut ber = reader("l3:fooe");
        let value = ber.next_value().unwrap().unwrap();
        assert_eq!(value.len(), 1);

        let mut ber = reader("li32ei45ee");
        let value = ber.next_value().unwrap().unwrap();
        assert_eq!(value.len(), 2);

        let mut ber = reader("li-88e4:quuxi23ee");
        let value = ber.next_value().unwrap().unwrap();
        assert_eq!(value.len(), 3);

        // test indexing and check values.
    }

    #[test]
    fn test_read_dict() {
        // empty test_read_dict
        let mut ber = reader("de");
        let value = ber.next_value().unwrap().unwrap();
        assert_eq!(value.len(), 0);

        let mut ber = reader("d3:one4:worde");
        let value = ber.next_value().unwrap().unwrap();
        assert_eq!(value.len(), 1);

        let mut ber = reader("d3:two5:words2:toi32ee");
        let value = ber.next_value().unwrap().unwrap();
        assert_eq!(value.len(), 2);

        // Errors to check:
        // - odd number of values,
        // - keys out of order,
        // - non string keys,
    }
}

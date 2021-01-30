use std::borrow::Cow;
use std::collections::HashMap;
use std::io::Read;
use std::iter::Peekable;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BEError {
    #[error("unexpected EOF")]
    EOFError,

    #[error("IO Error: {0:?}")]
    IOError(#[from] std::io::Error),

    #[error("keys must be strings, got {0:?}")]
    KeyNotString(BEValue),

    #[error("key, \"{0:?}\", is not in lexicographical order")]
    KeysOutOfOrder(String),

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

    #[error("negative zero not permitted")]
    NegativeZeroError,

    #[error("negative string lengths are not permitted")]
    NegativeStringLength(i64),

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

#[derive(PartialEq, Eq)]
pub enum BEValue {
    BEDict(HashMap<Vec<u8>, BEValue>),
    BEInteger(i64),
    BEList(Vec<BEValue>),
    BEString(Vec<u8>),
}

fn maybe_string<'s>(bytes: &'s [u8], quoted: bool) -> Cow<'s, str> {
    let maybe = std::str::from_utf8(bytes);
    match maybe {
        Err(_) => Cow::Owned(format!("[{} bytes]", bytes.len())),
        Ok(s) => {
            if quoted {
                Cow::Owned(format!("\"{}\"", s))
            } else {
                Cow::Borrowed(s)
            }
        }
    }
}

// We implement Debug by hand so that we can get the two-way treatment of BEString:
// if it's a valid UTF-8 string, we output a string. If not, then we just output the string length.
impl std::fmt::Debug for BEValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BEValue::BEDict(hsh) => {
                // Sort the keys first, since beencoded dicts store keys in lex order.
                let mut key_vec = hsh.iter().collect::<Vec<(&Vec<u8>, &BEValue)>>();
                key_vec.sort_by(|&(k1, _), &(k2, _)| k1.cmp(k2));
                f.debug_map()
                    .entries(key_vec.iter().map(|(k, v)| (maybe_string(k, false), v)))
                    .finish()
            }
            BEValue::BEInteger(int) => f.write_str(&format!("{}", int)),
            BEValue::BEString(s) => f.write_str(&maybe_string(s, true)),
            BEValue::BEList(lst) => f.debug_list().entries(lst.iter()).finish(),
        }
    }
}

impl BEValue {
    pub fn string(&self) -> Cow<str> {
        match self {
            BEValue::BEString(s) => String::from_utf8_lossy(s),
            _ => panic!("string() called on non-string"),
        }
    }

    pub fn integer(&self) -> i64 {
        match self {
            BEValue::BEInteger(i) => *i,
            _ => panic!("integer() called on non-integer"),
        }
    }

    pub fn is_string(&self) -> bool {
        match self {
            BEValue::BEString(_) => true,
            _ => false,
        }
    }

    pub fn is_integer(&self) -> bool {
        match self {
            BEValue::BEInteger(_) => true,
            _ => false,
        }
    }

    pub fn is_list(&self) -> bool {
        match self {
            BEValue::BEList(_) => true,
            _ => false,
        }
    }

    pub fn is_dict(&self) -> bool {
        match self {
            BEValue::BEDict(_) => true,
            _ => false,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        match self {
            BEValue::BEInteger(_) => 1,
            BEValue::BEString(s) => s.len(),
            BEValue::BEList(l) => l.len(),
            BEValue::BEDict(d) => d.len(),
        }
    }

    pub fn keys(&self) -> impl Iterator<Item = &Vec<u8>> {
        match self {
            BEValue::BEDict(d) => d.keys(),
            _ => panic!("Not a dict"),
        }
    }

    pub fn get(&self, key: &[u8]) -> Option<&BEValue> {
        match self {
            BEValue::BEDict(d) => d.get(key),
            _ => panic!("Not a dict"),
        }
    }
}

impl std::ops::Index<usize> for BEValue {
    type Output = BEValue;

    fn index(&self, index: usize) -> &Self::Output {
        if let BEValue::BEList(lst) = self {
            &lst[index]
        } else {
            panic!("Cannot index in a non-BEList BEValue.");
        }
    }
}

impl std::ops::Index<&[u8]> for BEValue {
    type Output = BEValue;

    fn index(&self, index: &[u8]) -> &Self::Output {
        if let BEValue::BEDict(dict) = self {
            &dict[index]
        } else {
            panic!("Cannot lookup in a non-BEDict BEValue.");
        }
    }
}

impl std::ops::Index<&str> for BEValue {
    type Output = BEValue;

    fn index(&self, index: &str) -> &Self::Output {
        &self[index.as_bytes()]
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
            Some(Err(_)) => {
                // The result of peek() is unowned, so we are unable to get the error out of it
                // and keep the borrow checker happy. So, we call next() on the assumption that it
                // will return the same value as the last call to peek(), and we can take the
                // error from there.
                let result = self.chars.next();
                if let Some(Err(e)) = result {
                    Err(BEError::from(e))
                } else {
                    panic!("Inconsistent result. next() != peek(). {:?}", result);
                }
            }
            Some(Ok(ch)) => Ok(PeekedValue::ASCII(*ch)),
        }
    }

    fn peek_char_no_eof(&mut self) -> Result<u8> {
        match self.peeked_char()? {
            PeekedValue::EOF => Err(BEError::EOFError),
            PeekedValue::ASCII(ch) => Ok(ch),
        }
    }

    fn next_char_no_eof(&mut self) -> Result<u8> {
        match self.chars.next() {
            None => Err(BEError::EOFError),
            Some(Err(e)) => Err(BEError::IOError(e)),
            Some(Ok(ch)) => Ok(ch),
        }
    }

    fn next_value_no_eof(&mut self) -> Result<BEValue> {
        match self.next_value()? {
            None => Err(BEError::EOFError),
            Some(v) => Ok(v),
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

    fn check_next_char<ErrFn>(&mut self, expected: u8, errf: ErrFn) -> Result<()>
    where
        ErrFn: FnOnce(u8, u8) -> BEError,
    {
        let ch = self.next_char_no_eof()?;
        if expected != ch {
            Err(errf(ch, expected))
        } else {
            Ok(())
        }
    }

    fn check_prefix(&mut self, expected: u8) -> Result<()> {
        self.check_next_char(expected, BEError::MissingPrefixError)
    }

    fn read_dict(&mut self) -> Result<BEValue> {
        self.check_prefix(D_CHAR)?;

        let mut dict = HashMap::new();
        let mut last_key: Option<Vec<u8>> = None;
        loop {
            let key = match self.peek_char_no_eof()? {
                E_CHAR => {
                    self.next_char_no_eof()?;
                    break;
                }
                _ => match self.next_value_no_eof()? {
                    BEValue::BEString(s) => s,
                    v => return Err(BEError::KeyNotString(v)),
                },
            };

            let value = match self.peek_char_no_eof()? {
                E_CHAR => {
                    return Err(BEError::MissingValueError(
                        String::from_utf8_lossy(&key).to_string(),
                    ));
                }
                _ => self.next_value_no_eof()?,
            };

            if let Some(last) = last_key {
                if key <= last {
                    // if key.as_ref() >= last {
                    return Err(BEError::KeysOutOfOrder(
                        String::from_utf8_lossy(&key).to_string(),
                    ));
                }
            }
            last_key = Some(key.clone());
            dict.insert(key, value);
        }

        Ok(BEValue::BEDict(dict))
    }

    fn read_list(&mut self) -> Result<BEValue> {
        self.check_prefix(L_CHAR)?;

        let mut result = Vec::default();
        loop {
            match self.peek_char_no_eof()? {
                E_CHAR => {
                    self.next_char_no_eof()?;
                    break;
                }
                _ => result.push(self.next_value_no_eof()?),
            }
        }

        Ok(BEValue::BEList(result))
    }

    fn read_integer(&mut self) -> Result<BEValue> {
        self.check_prefix(I_CHAR)?;

        let value = self.read_raw_integer()?;
        self.check_next_char(E_CHAR, BEError::MissingSuffixError)?;
        Ok(BEValue::BEInteger(value))
    }

    fn read_string(&mut self) -> Result<BEValue> {
        let len = self.read_raw_integer()?;
        if len < 0 {
            // This should never happen, because the beencode format doesn't allow it.
            // (Strings must start with a digit which precludes having a '-'.)
            // But, let's check for it anyway.
            return Err(BEError::NegativeStringLength(len));
        }
        let len = len as usize;

        self.check_next_char(COLON_CHAR, BEError::MissingSeparatorError)?;

        let mut buf = [0u8; 100000];
        #[allow(clippy::needless_range_loop)]
        for index in 0..len {
            buf[index] = self.next_char_no_eof()?;
        }

        Ok(BEValue::BEString((&buf[0..len]).into()))
    }

    fn read_raw_integer(&mut self) -> Result<i64> {
        let mut buf = [0u8; 100];
        let mut index = 0;
        let mut minus = 1i64;
        let mut lead_zero = false;

        // Check for minus sign.
        if let PeekedValue::ASCII(MINUS_SIGN) = self.peeked_char()? {
            self.chars.next();
            minus = -1;
        }

        loop {
            match self.peek_char_no_eof()? {
                ch if ch.is_ascii_digit() => {
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

        let value: i64 = str::parse(std::str::from_utf8(&buf[0..index])?)?;
        if value == 0 && minus < 0 {
            return Err(BEError::NegativeZeroError);
        }
        Ok(value * minus)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_error0 {
        ($e:expr, $p:path) => {
            match $e {
                Err($p) => {}
                _ => {
                    assert!(false)
                }
            }
        };
    }

    macro_rules! assert_error1 {
        ($e:expr, $p:path, $v:expr) => {
            match $e {
                Err($p(v)) => {
                    assert_eq!(v, $v);
                }
                _ => {
                    assert!(false);
                }
            }
        };
    }

    macro_rules! assert_error2 {
        ($e:expr, $p:path, $v1:expr, $v2:expr) => {
            match $e {
                Err($p(v1, v2)) => {
                    assert_eq!(v1, $v1);
                    assert_eq!(v2, $v2);
                }
                _ => {
                    assert!(false)
                }
            }
        };
    }

    fn reader(s: &'static str) -> BEReader<impl Read> {
        BEReader::new(s.as_bytes())
    }

    fn value_for_string(s: &str) -> BEValue {
        BEReader::new(s.as_bytes()).next_value().unwrap().unwrap()
    }

    fn make_string() -> BEValue {
        value_for_string("4:quux")
    }

    fn make_integer() -> BEValue {
        value_for_string("i42e")
    }

    fn make_empty_list() -> BEValue {
        value_for_string("le")
    }

    fn make_empty_dict() -> BEValue {
        value_for_string("de")
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
        assert_error0!(value, BEError::EOFError);

        // Missing suffix with more chars.
        let mut ber = reader("i32i33e");
        let value = ber.next_value();
        assert_error2!(value, BEError::MissingSuffixError, 0x69, E_CHAR);
    }

    #[test]
    fn test_leading_zero() {
        // Leading zero not allowed.
        let mut ber = reader("i032e");
        let value = ber.next_value();
        assert_error0!(value, BEError::LeadZeroError);
    }

    #[test]
    fn test_negative_zero() {
        let mut ber = reader("i-0e");
        let value = ber.next_value();
        assert_error0!(value, BEError::NegativeZeroError);
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
    }

    #[test]
    fn test_missing_colon() {
        let mut ber = reader("3foo");
        let value = ber.next_value();
        assert_error2!(value, BEError::MissingSeparatorError, 0x66, 0x3a);
    }

    #[test]
    fn test_negative_string_length() {
        let mut ber = reader("-10:impossible_");

        // The beencode format makes this impossible, so we have to test it with the
        // private helper function.
        let value = ber.read_string();

        assert_error1!(value, BEError::NegativeStringLength, -10);
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
        assert_eq!(value[0].integer(), -88);
        assert_eq!(value[1].string(), "quux");
        assert_eq!(value[2].integer(), 23);
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

        let mut ber = reader("d2:toi32e3:two5:wordse");
        let value = ber.next_value().unwrap().unwrap();
        assert_eq!(value.len(), 2);
        assert_eq!(value["two"].string(), "words");
        assert_eq!(value["to"].integer(), 32);
    }

    #[test]
    fn test_keys_out_of_order() {
        let mut ber = reader("d3:zzz5:words3:aaai7ee");
        let value = ber.next_value();

        assert_error1!(value, BEError::KeysOutOfOrder, "aaa");
    }

    #[test]
    fn test_missing_value() {
        let mut ber = reader("d3:two5:words7:missinge");
        let value = ber.next_value();

        assert_error1!(value, BEError::MissingValueError, "missing");
    }

    #[test]
    fn test_non_string_key() {
        let mut ber = reader("di666e5:words7:secondei42ee");
        let value = ber.next_value();

        assert_error1!(value, BEError::KeyNotString, BEValue::BEInteger(666));
    }

    #[test]
    fn test_list_of_lists() {
        let mut ber = reader("ll5:mooreel3:bar4:quuxee");
        let value = ber.next_value().unwrap().unwrap();
        assert_eq!(2, value.len());

        assert_eq!(value[0][0].string(), "moore");
        assert_eq!(value[1][0].string(), "bar");
        assert_eq!(value[1][1].string(), "quux");
    }

    #[test]
    fn test_is_string() {
        assert!(make_string().is_string());
        assert!(!make_integer().is_string());
        assert!(!make_empty_list().is_string());
        assert!(!make_empty_dict().is_string());
    }

    #[test]
    fn test_is_integer() {
        assert!(!make_string().is_integer());
        assert!(make_integer().is_integer());
        assert!(!make_empty_list().is_integer());
        assert!(!make_empty_dict().is_integer());
    }

    #[test]
    fn test_is_list() {
        assert!(!make_string().is_list());
        assert!(!make_integer().is_list());
        assert!(make_empty_list().is_list());
        assert!(!make_empty_dict().is_list());
    }

    #[test]
    fn test_is_dict() {
        assert!(!make_string().is_dict());
        assert!(!make_integer().is_dict());
        assert!(!make_empty_list().is_dict());
        assert!(make_empty_dict().is_dict());
    }

    #[test]
    fn test_illegal_prefix() {
        let mut ber = reader("y");
        let result = ber.next_value();
        assert_error1!(result, BEError::UnexpectedCharError, 'y');
    }
}

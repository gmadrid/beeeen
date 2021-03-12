use crate::{BEError, BEValue, Result};
use std::collections::HashMap;
use std::io::Read;
use std::iter::Peekable;

const COLON_CHAR: u8 = 0x3a;
const D_CHAR: u8 = 0x64;
const E_CHAR: u8 = 0x65;
const I_CHAR: u8 = 0x69;
const L_CHAR: u8 = 0x6c;
const MINUS_SIGN: u8 = 0x2d;
const ZERO_CHAR: u8 = 0x30;

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
    pub(crate) chars: Peekable<std::io::Bytes<R>>,
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

    // This is only pub(crate) so that we can make a unit test that uses it.
    // Do NOT use it.
    // TODO: move the tests into this file.
    pub(crate) fn read_string(&mut self) -> Result<BEValue> {
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

use serde::de::{self, MapAccess, SeqAccess};
use serde::{forward_to_deserialize_any, Deserialize};

use super::{Error, Result};

pub struct Deserializer<'de> {
    bytes: &'de [u8],
}

impl<'de> Deserializer<'de> {
    pub fn from_bytes(bytes: &'de [u8]) -> Self {
        Deserializer { bytes }
    }
}

pub fn from_bytes<'a, T>(bytes: &'a [u8]) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_bytes(bytes);
    let t = T::deserialize(&mut deserializer)?;
    if deserializer.bytes.is_empty() {
        Ok(t)
    } else {
        Err(Error::TrailingInput)
    }
}

impl<'de> Deserializer<'de> {
    fn peek_byte(&mut self) -> Result<u8> {
        self.bytes.first().copied().ok_or(Error::Eof)
    }

    fn next_byte(&mut self) -> Result<u8> {
        let byte = self.peek_byte()?;
        self.bytes = &self.bytes[1..];
        Ok(byte)
    }

    fn parse_bytes(&mut self) -> Result<&'de [u8]> {
        let length = self.parse_raw_integer()? as usize;
        let colon = self.next_byte()?;
        if colon != b':' {
            return Err(Error::MissingColon(colon));
        }
        let result = &self.bytes[..length];
        self.bytes = &self.bytes[length..];
        Ok(result)
    }

    fn parse_str(&mut self) -> Result<&'de str> {
        let bytes = self.parse_bytes()?;
        Ok(std::str::from_utf8(bytes)?)
    }

    // parse raw_integer should NOT check/consume the terminating 'e'
    fn parse_raw_integer(&mut self) -> Result<u64> {
        // TODO: detect unexpected '0' prefix.
        // TODO: detect empty string.
        let mut val = 0u64;
        loop {
            let b = self.peek_byte()?;
            if !b.is_ascii_digit() {
                break;
            }
            val = val * 10 + (self.next_byte()? - b'0') as u64
        }
        Ok(val)
    }

    fn parse_unsigned(&mut self) -> Result<u64> {
        if self.peek_byte()? == b'-' {
            return Err(Error::UnexpectedSigned);
        }

        let b = self.next_byte()?;
        if b != b'i' {
            return Err(Error::UnexpectedPrefix(b as char, 'i'));
        }

        let val = self.parse_raw_integer()?;
        if self.next_byte()? != b'e' {
            return Err(Error::ExpectedNumEnd);
        }
        Ok(val)
    }

    fn parse_signed(&mut self) -> Result<i64> {
        let b = self.next_byte()?;
        if b != b'i' {
            return Err(Error::UnexpectedPrefix(b as char, 'i'));
        }

        let sign = self.peek_byte()?;
        let multiplier: i64 = if sign == b'-' {
            self.next_byte()?;
            -1
        } else {
            1
        };

        let uval = self.parse_raw_integer()?;
        if self.next_byte()? != b'e' {
            return Err(Error::ExpectedNumEnd);
        }
        Ok(multiplier * uval as i64)
        // // TODO: do this with parse_raw_integer
        // // TODO: detect unexpected '0' prefix.
        // // TODO: detect empty string.
        // // TODO: check for overflow.
        // let mut val = 0i64;
        // loop {
        //     let b = self.peek_byte();
        //     if b.is_err() {
        //         if let Err(Error::Eof) = b {
        //             break;
        //         } else {
        //             b?;
        //         }
        //     }
        //     val = val * 10 + (self.next_byte()? - b'0') as i64
        // }
        // Ok(multiplier * val)
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    forward_to_deserialize_any!(i8 i16 i32 i64 u8 u16 u32 u64);

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.peek_byte()? {
            // TODO: match all data types here.
            b'i' => {
                if self.bytes[1] == b'-' {
                    visitor.visit_i64(self.parse_signed()?)
                } else {
                    visitor.visit_u64(self.parse_unsigned()?)
                }
            }
            mismatch => Err(Error::UnrecognizedPrefix(mismatch)),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_bool(self.parse_unsigned()? != 0)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_borrowed_str(self.parse_str()?)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_borrowed_bytes(self.parse_bytes()?)
        // visitor.visit_bytes(self.parse_bytes()?)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_unit_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        if self.next_byte()? != b'l' {
            return Err(Error::ExpectedList);
        }

        let value = visitor.visit_seq(List::new(&mut self))?;

        if self.next_byte()? != b'e' {
            return Err(Error::ExpectedListEnd);
        }
        Ok(value)
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_map<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        if self.next_byte()? != b'd' {
            return Err(Error::ExpectedMap);
        }

        let value = visitor.visit_map(Map::new(&mut self))?;

        if self.next_byte()? != b'e' {
            return Err(Error::ExpectedMapEnd);
        }
        Ok(value)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        // TODO: write a unit test for this.
        self.deserialize_any(visitor)
    }
}

struct List<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> List<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        List { de }
    }
}

impl<'de, 'a> SeqAccess<'de> for List<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: de::DeserializeSeed<'de>,
    {
        if self.de.peek_byte()? == b'e' {
            return Ok(None);
        }

        seed.deserialize(&mut *self.de).map(Some)
    }
}

struct Map<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> Map<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Map { de }
    }
}

impl<'de, 'a> MapAccess<'de> for Map<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: de::DeserializeSeed<'de>,
    {
        if self.de.peek_byte()? == b'e' {
            return Ok(None);
        }

        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: de::DeserializeSeed<'de>,
    {
        // TODO: should I check for 'e' here?
        seed.deserialize(&mut *self.de)
    }
}

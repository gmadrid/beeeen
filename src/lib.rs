use thiserror::Error as ThisError;

mod de;
mod ser;

#[derive(Debug, ThisError, PartialEq)]
pub enum SerbeError {
    #[error("error from serde: {0}")]
    Message(String),

    #[error("reached end of input before finishing")]
    Eof,

    #[error("expected a 'l' to start the list")]
    ExpectedList,

    #[error("expected a 'e' to end the list")]
    ExpectedListEnd,

    #[error("expected a 'e' to end the number")]
    ExpectedNumEnd,

    #[error("expected a 'd' to start the map")]
    ExpectedMap,

    #[error("expected a 'e' to end the map")]
    ExpectedMapEnd,

    #[error("expected colon, ':', to separate length from bytes. Found {0}")]
    MissingColon(u8),

    #[error("every number must have at least one digit")]
    NoDigitsInNumber,

    #[error("trailing input remains after deserializing")]
    TrailingInput,

    #[error("unrecognized prefix character, '{0}'")]
    UnrecognizedPrefix(u8),

    #[error("expected {1}, found: {0}")]
    UnexpectedPrefix(char, char),

    #[error("unexpected negative sign for unsigned value")]
    UnexpectedSigned,

    #[error("integers cannot start with '0' unless they are 0")]
    UnexpectedZeroPrefix,

    #[error("Utf8Error: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),
}

impl std::convert::From<std::io::Error> for SerbeError {
    fn from(err: std::io::Error) -> Self {
        Self::Message(err.to_string())
    }
}

impl serde::de::Error for SerbeError {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        SerbeError::Message(msg.to_string())
    }
}

impl serde::ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        SerbeError::Message(msg.to_string())
    }
}

pub type Error = SerbeError;
pub type Result<T> = std::result::Result<T, Error>;

pub use de::from_bytes;
pub use ser::to_bytes;

#[cfg(test)]
mod test {
    use super::*;
    use serde::{Deserialize, Serialize};

    macro_rules! assert_round_trip {
        ($v:expr, $t:ty) => {
            let val: $t = $v;
            assert_eq!(val, from_bytes::<$t>(&to_bytes(&val).unwrap()).unwrap());
        };
    }

    #[test]
    fn test_bool() {
        let val: bool = from_bytes(b"i0e").unwrap();
        assert_eq!(false, val);

        let val: bool = from_bytes(b"i1e").unwrap();
        assert_eq!(true, val);

        let val: bool = from_bytes(b"i32e").unwrap();
        assert_eq!(true, val);

        assert_round_trip!(false, bool);
        assert_round_trip!(true, bool);
    }

    #[test]
    fn test_unsigned() {
        let val: u8 = from_bytes(b"i5e").unwrap();
        assert_eq!(5, val);
        let val: u16 = from_bytes(b"i55e").unwrap();
        assert_eq!(55, val);
        let val: u64 = from_bytes(b"i1234567890e").unwrap();
        assert_eq!(1234567890, val);

        assert_round_trip!(54, u8);
        assert_round_trip!(700, u16);
        assert_round_trip!(123457, u32);
        assert_round_trip!(12345678999, u64);

        // TODO: check overflow
    }

    #[test]
    fn test_signed() {
        let val: i8 = from_bytes(b"i5e").unwrap();
        assert_eq!(5, val);
        let val: i16 = from_bytes(b"i55e").unwrap();
        assert_eq!(55, val);
        let val: i16 = from_bytes(b"i-55e").unwrap();
        assert_eq!(-55, val);
        let val: i64 = from_bytes(b"i-1234567890e").unwrap();
        assert_eq!(-1234567890, val);

        assert_round_trip!(54, i8);
        assert_round_trip!(-54, i8);
        assert_round_trip!(700, i16);
        assert_round_trip!(-700, i16);
        assert_round_trip!(123457, i32);
        assert_round_trip!(-123457, i32);
        assert_round_trip!(12345678999, i64);
        assert_round_trip!(-12345678999, i64);
    }

    #[test]
    fn test_missing_e() {
        assert_eq!(Error::Eof, from_bytes::<u32>(b"i56").unwrap_err(),);
        assert_eq!(Error::Eof, from_bytes::<i32>(b"i-65").unwrap_err(),);
    }

    #[test]
    fn test_empty_digits() {
        assert_eq!(
            Error::NoDigitsInNumber,
            from_bytes::<u32>(b"ie").unwrap_err(),
        );
        assert_eq!(
            Error::NoDigitsInNumber,
            from_bytes::<i32>(b"ie").unwrap_err(),
        );
    }

    #[test]
    fn test_sign_mismatch() {
        // u16 can't be negative.
        assert!(from_bytes::<u16>(b"i-7e").is_err());
    }

    #[test]
    fn test_leading_zero() {
        assert_eq!(
            Error::UnexpectedZeroPrefix,
            from_bytes::<u16>(b"i05e").unwrap_err()
        );
        assert_eq!(
            Error::UnexpectedZeroPrefix,
            from_bytes::<i16>(b"i-05e").unwrap_err()
        );

        // Multiple zeros are not allowed.
        assert_eq!(
            Error::UnexpectedZeroPrefix,
            from_bytes::<i16>(b"i00e").unwrap_err()
        );
    }

    #[test]
    fn test_leading_negative_in_unsigned() {
        assert_eq!(
            Error::Message("invalid value: integer `-5`, expected u16".to_string()),
            from_bytes::<u16>(b"i-5e").unwrap_err()
        );
    }

    #[test]
    fn test_string() {
        let val: String = from_bytes(b"4:yarn").unwrap();
        assert_eq!(val, "yarn");

        let val: String = from_bytes(b"0:").unwrap();
        assert_eq!(val, "");

        assert_round_trip!("".to_string(), String);
        assert_round_trip!("hellion".to_string(), String);
    }

    #[test]
    fn test_missing_colon() {
        assert_eq!(
            Error::MissingColon(b'l'),
            from_bytes::<&str>(b"5lucky").unwrap_err()
        );
    }

    #[test]
    fn test_str() {
        let val: &str = from_bytes(b"4:yarn").unwrap();
        assert_eq!(val, "yarn");

        let val: &str = from_bytes(b"0:").unwrap();
        assert_eq!(val, "");

        assert_round_trip!("", &str);
        assert_round_trip!("whoopie", &str);
    }

    #[test]
    fn test_arr() {
        let val: Vec<u32> = from_bytes(b"li1ei0ei32ei45ei0ei4ee").unwrap();
        assert_eq!(vec![1u32, 0, 32, 45, 0, 4], val);

        let val: Vec<u32> = from_bytes(b"le").unwrap();
        assert!(val.is_empty());

        let val: Vec<&str> = from_bytes(b"l3:foo6:foobar4:quuxe").unwrap();
        assert_eq!(vec!["foo", "foobar", "quux"], val);

        assert_round_trip!(vec![], Vec<u32>);
        assert_round_trip!(vec![5u16, 700u16, 999u16, 0xfdbau16], Vec<u16>);
        assert_round_trip!(vec!["one", "two", "three"], Vec<&str>);
    }

    #[test]
    fn test_missing_l() {
        assert_eq!(
            Error::ExpectedList,
            from_bytes::<Vec<u16>>(b"i0ei0ei0ee").unwrap_err()
        );
    }

    #[test]
    fn test_missing_list_e() {
        // Missing 'e' manifests as EOF because the list is never terminated.
        assert_eq!(
            Error::Eof,
            from_bytes::<Vec<u16>>(b"li0ei0ei0e").unwrap_err()
        );
    }

    #[test]
    fn test_struct() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct TestStruct {
            inteight: i8,
            s: String,
        }

        let val: TestStruct = from_bytes(b"d8:inteighti33e1:s4:worde").unwrap();
        assert_eq!(
            TestStruct {
                inteight: 33,
                s: "word".to_string(),
            },
            val
        );

        assert_round_trip!(
            TestStruct {
                inteight: 45,
                s: "elfen".to_string(),
            },
            TestStruct
        );

        // TODO: test ignored fields
    }

    #[test]
    fn test_structs_with_option() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct TestWithOption<'a> {
            i: Option<u8>,
            s: Option<&'a str>,
        }
        let val: TestWithOption = from_bytes(b"de").unwrap();
        assert_eq!(TestWithOption { i: None, s: None }, val);

        let val: TestWithOption = from_bytes(b"d1:ii8ee").unwrap();
        assert_eq!(
            TestWithOption {
                i: Some(8),
                s: None
            },
            val
        );

        let val: TestWithOption = from_bytes(b"d1:s6:floppye").unwrap();
        assert_eq!(
            TestWithOption {
                i: None,
                s: Some("floppy")
            },
            val
        );

        let val: TestWithOption = from_bytes(b"d1:ii34e1:s6:floppye").unwrap();
        assert_eq!(
            TestWithOption {
                i: Some(34),
                s: Some("floppy")
            },
            val
        );

        let bytes = to_bytes(&TestWithOption { i: None, s: None }).unwrap();
        println!(
            "BYTES: {} == {:?}",
            std::str::from_utf8(&bytes).unwrap(),
            bytes
        );
        assert_round_trip!(TestWithOption { i: None, s: None }, TestWithOption);
    }
}

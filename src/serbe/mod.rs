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

    #[error("trailing input remains after deserializing")]
    TrailingInput,

    #[error("unrecognized prefix character, '{0}'")]
    UnrecognizedPrefix(u8),

    #[error("expected {1}, found: {0}")]
    UnexpectedPrefix(char, char),

    #[error("unexpected negative sign for signed value")]
    UnexpectedSigned,

    #[error("Utf8Error: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),
}

impl serde::de::Error for SerbeError {
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

#[cfg(test)]
mod test {
    use super::*;
    use serde::Deserialize;

    #[test]
    fn test_bool() {
        let val: bool = from_bytes(b"i0e").unwrap();
        assert_eq!(false, val);

        let val: bool = from_bytes(b"i1e").unwrap();
        assert_eq!(true, val);

        let val: bool = from_bytes(b"i32e").unwrap();
        assert_eq!(true, val);
    }

    #[test]
    fn test_unsigned() {
        let val: u8 = from_bytes(b"i5e").unwrap();
        assert_eq!(5, val);
        let val: u16 = from_bytes(b"i55e").unwrap();
        assert_eq!(55, val);
        let val: u64 = from_bytes(b"i1234567890e").unwrap();
        assert_eq!(1234567890, val);

        // TODO: check leading ZERO
        // TODO: check leading negative
        // TODO: check empty digits
        // TODO: check overflow
        // TODO: test missing 'e'
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
    }

    #[test]
    fn test_sign_mismatch() {
        // u16 can't be negative.
        assert!(from_bytes::<u16>(b"i-7e").is_err());
    }

    #[test]
    fn test_string() {
        let val: String = from_bytes(b"4:yarn").unwrap();
        assert_eq!(val, "yarn");

        let val: String = from_bytes(b"0:").unwrap();
        assert_eq!(val, "");

        // TODO check missing colon
    }

    #[test]
    fn test_str() {
        let val: &str = from_bytes(b"4:yarn").unwrap();
        assert_eq!(val, "yarn");

        let val: &str = from_bytes(b"0:").unwrap();
        assert_eq!(val, "");

        // TODO check missing colon
    }

    #[test]
    fn test_arr() {
        let val: Vec<u32> = from_bytes(b"li1ei0ei32ei45ei0ei4ee").unwrap();
        assert_eq!(vec![1u32, 0, 32, 45, 0, 4], val);

        let val: Vec<u32> = from_bytes(b"le").unwrap();
        assert!(val.is_empty());

        let val: Vec<&str> = from_bytes(b"l3:foo6:foobar4:quuxe").unwrap();
        assert_eq!(vec!["foo", "foobar", "quux"], val);

        // TODO: test missing 'l'
        // TODO: test missing 'e'
    }

    #[test]
    fn test_struct() {
        #[derive(Deserialize, PartialEq, Debug)]
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

        let val: TestStruct = from_bytes(b"d4:fakei0e8:inteighti33e1:s4:worde").unwrap();
    }

    #[test]
    fn test_structs_with_option() {
        #[derive(Deserialize, PartialEq, Debug)]
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
    }
}

use crate::BEValue;
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

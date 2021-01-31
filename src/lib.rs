mod beerror;
mod bereader;
mod bevalue;

pub use beerror::BEError;
pub use bereader::BEReader;
pub use bevalue::BEValue;

pub type Result<T> = std::result::Result<T, BEError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

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
        assert_error2!(value, BEError::MissingSuffixError, 'i' as u8, 'e' as u8);
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
        assert_error2!(value, BEError::MissingSeparatorError, 'f' as u8, ':' as u8);
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

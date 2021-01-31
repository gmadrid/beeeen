use std::borrow::Cow;
use std::collections::HashMap;

#[derive(PartialEq, Eq)]
pub enum BEValue {
    BEDict(HashMap<Vec<u8>, BEValue>),
    BEInteger(i64),
    BEList(Vec<BEValue>),
    BEString(Vec<u8>),
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
        matches!(self, BEValue::BEString(_))
    }

    pub fn is_integer(&self) -> bool {
        matches!(self, BEValue::BEInteger(_))
    }

    pub fn is_list(&self) -> bool {
        matches!(self, BEValue::BEList(_))
    }

    pub fn is_dict(&self) -> bool {
        matches!(self, BEValue::BEDict(_))
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the length of the value, defined as:
    ///   Integer: 1
    ///   String:  string length
    ///   List:    number of elements in the list
    ///   Dict:    number of key,value pairs in the dict.
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

    pub fn get(&self, key: impl AsRef<[u8]>) -> Option<&BEValue> {
        match self {
            BEValue::BEDict(d) => d.get(key.as_ref()),
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

fn maybe_string(bytes: &[u8], quoted: bool) -> Cow<str> {
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

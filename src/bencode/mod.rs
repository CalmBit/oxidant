//! A module for bencoding support - decodes bencoded data into a `BCObject`,
//! which encapsulates the underlying form.
#![cfg_attr(feature = "cargo-clippy", deny(pedantic))]

use std::collections::BTreeMap;
use std::error::Error;

#[derive(Debug)]
pub enum BCObject {
    String(String),
    Integer(i64),
    List(Vec<BCObject>),
    Dictionary(BTreeMap<String, BCObject>),
}

impl PartialEq for BCObject {
    fn eq(&self, other: &Self) -> bool {
        match (&self, other) {
            (BCObject::String(x), BCObject::String(y)) => x == y,
            (BCObject::Integer(x), BCObject::Integer(y)) => x == y,
            (BCObject::List(v1), BCObject::List(v2)) => {
                if v1.len() == v2.len() {
                    for x in 0..v1.len() {
                        if v1[x] == v2[x] {
                            continue;
                        }
                        return false;
                    }
                    true
                } else {
                    false
                }
            }
            (BCObject::Dictionary(m1), BCObject::Dictionary(m2)) => {
                if m1.len() == m2.len() {
                    for k in m1.keys() {
                        if m2.contains_key(k) && m1[k] == m2[k] {
                            continue;
                        }
                        return false;
                    }
                    true
                } else {
                    false
                }
            }
            (_, _) => false,
        }
    }
}

type PeekableCharIterator<'a> = ::std::iter::Peekable<std::str::Chars<'a>>;

impl BCObject {
    fn parse_dictionary(iter: &mut PeekableCharIterator) -> Result<Self, String> {
        // Are we actually dealing with a dicctionary? If so, let's go past the point
        // of the dictionary delimiter.
        if let Some('d') = iter.next() {
            // Set up a BTreeMap to store our items and keys.
            let mut m: BTreeMap<String, Self> = BTreeMap::new();

            // 1. Are we still looking at an item in our iterator?
            // 2. Is the next item not an ending element?
            // If both are true, let's assume we've got an item and parse it out,
            while iter.peek().is_some() && iter.peek().unwrap() != &'e' {
                // First, set up a container for the key.
                let mut key: String;

                // Is there actually a string here for the key?
                match Self::parse_string(iter) {
                    Ok(k) => {
                        // Was the object a string?  We're using parse_string,
                        // so we shouldn't really ever need to have this error triggered,
                        // but it's a good sanity check all the same.
                        if let BCObject::String(k) = k {
                            key = k;
                        } else {
                            return Err("key was not a string type - abort".to_string());
                        }
                    }
                    Err(e) => return Err(e),
                }

                // Alright, now try to get a value to go under our key.
                match Self::parse(iter) {
                    Ok(v) => {
                        m.insert(key, v);
                    }
                    Err(e) => return Err(e),
                }
            }

            // Once the loop has exited, let's make sure we haven't exhausted the list - there
            // should still, at _least_, be our `e` for the ending delimiter.
            if iter.peek().is_none() {
                return Err("premature end of dictionary string".to_string());
            }

            // Move to the ending delimeter, as to not mess up future calculations.
            iter.next();

            // Return our complete Dictionary object, with requisite map.
            return Ok(BCObject::Dictionary(m));
        }

        // Whoops, looks like what we found wasn't a dictionary - make a big noise.  
        Err("tried to parse a dictionary - not a dictionary".to_string())
    }

    fn parse_list(iter: &mut PeekableCharIterator) -> Result<Self, String> {
        // Are we actually dealing with a list? If so, let's go past the point
        // of the list delimiter.
        if let Some('l') = iter.next() {
            // Set up a vector to store our list items.
            let mut v: Vec<Self> = Vec::new();

            // 1. Are we still looking at an item in our iterator?
            // 2. Is the next item not an ending element?
            // If both are true, let's assume we've got an item and parse it out,
            // and push it into our vector.
            while iter.peek().is_some() && iter.peek().unwrap() != &'e' {
                v.push(Self::parse(iter).unwrap());
            }

            // Once the loop has exited, let's make sure we haven't exhausted the list - there
            // should still, at _least_, be our `e` for the ending delimiter.
            if iter.peek().is_none() {
                return Err("premature end of list string".to_string());
            }

            // Move to the ending delimeter, as to not mess up future calculations.
            iter.next();

            // Return our complete List object, with requisite vector.
            return Ok(BCObject::List(v));
        }

        // Whoops, looks like what we found wasn't a list - make a big noise.
        return Err("tried to parse a list - not a list".to_string());
    }

    fn parse_integer(iter: &mut PeekableCharIterator) -> Result<Self, String> {
        // Are we actually dealing with an integer? If so, let's go past the point
        // of the integer delimiter.
        if let Some('i') = iter.next() {
            // Create a `String` buffer in order to hold our future integer.
            let mut i = String::new();

            // 1. Are we still looking at an item in our iterator?
            // 2. Is the next item not an ending element?
            // If both are true, let's assume we've got a character
            // and push it into our buffer.
            while iter.peek().is_some() && iter.peek().unwrap() != &'e' {
                i.push(iter.next().unwrap());
            }

            // Once the loop has exited, let's make sure we haven't exhausted the list - there
            // should still, at _least_, be our `e` for the ending delimiter.
            if iter.peek().is_none() {
                return Err("premature end of integer string".to_string())
            }

            // If our integer is larger than two characters, and the beginning of the
            // integer is a negative zero, we can assume we don't want it - even
            // a plain negative zero is invalid.
            if i.len() >= 2 && &i[0..2] == "-0" {
                return Err("integer cannot start with or consist of -0".to_string())
            }

            // Otherwise, if our integer is larger than one digit, and starts with
            // a zero, we can assume we don't want it. No leading zeros, although zero
            // _itself_ is fine.
            if i.len() > 1 && &i[0..1] == "0" {
                return Err("integer cannot start with leading 0".to_string())
            }

            // Move to the ending delimeter, as to not mess up future calculations.
            iter.next();

            // Attempt to parse out the integer from our buffer.
            let int = i.parse::<i64>();

            // Match it, and make sure we've got an integer - return the integer object if
            // we do, an Error if we don't.
            return match int {
                Ok(i) => Ok(BCObject::Integer(i)),
                Err(e) => Err(e.description().to_string()),
            };
        }

        // Whoops, looks like what we found wasn't an integer - make a big noise.
        Err("tried to parse an integer - not an integer".to_string())
    }

    fn parse_string(iter: &mut PeekableCharIterator) -> Result<Self, String> {
        // Parsing strings is a little different, but still similar to other types.

        // Set up a buffer for the _length_ portion of our string object.
        let mut len = String::new();

        // Strings are in <len>:<data> form - read until we either run out of
        // data or hit the delimeter that marks the end of the length portion.
        while iter.peek().is_some() && iter.peek().unwrap() != &':' {
            len.push(iter.next().unwrap());
        }

        // Once the loop has exited, let's make sure we haven't exhausted the list.
        if iter.peek().is_none() {
            return Err("premature end of string".to_string());
        }

        // Now, parse out the length of the string.
        let len = len.parse::<i64>();
        // Set up a buffer for it, too.
        let mut buff: String = String::new();

        // If we've got a functioning length, let's iterate over it and get the
        // rest of our string.
        match len {
            Ok(i) => {
                iter.next();

                for x in 0..i {
                    if let Some(s) = iter.next() {
                        buff.push(s);
                        continue;
                    }

                    // If we hit the bottom of this, our iterator returned None - 
                    // this means there was still data we were expecting, but wasn't there.
                    // Make some noise!
                    return Err(format!(
                        "premature end of string after len - {} chars remaining",
                        i - x
                    ));
                }
                // We can't exactly know if our string was too long, but what we do know is that we
                // at least had the specified amount of data, and that's good enough.
                return Ok(BCObject::String(buff));
            }
            Err(e) => Err(e.description().to_string()),
        }
    }

    fn parse(iter: &mut PeekableCharIterator) -> Result<Self, String> {
        let c = iter.peek().unwrap().clone();
        match c {
            'i' => return Self::parse_integer(iter),
            'd' => return Self::parse_dictionary(iter),
            'l' => return Self::parse_list(iter),
            '0'...'9' => return Self::parse_string(iter),
            c => return Err(format!("not implemented: {}", c)),
        }
    }

    pub fn parse_blob(blob: &str) -> Result<Self, String> {
        Self::parse(&mut blob.chars().peekable())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bencode_integer_parse() {
        let s = "i623e";
        assert_eq!(
            BCObject::Integer(623),
            BCObject::parse_integer(&mut s.chars().peekable()).unwrap()
        );
    }

    #[test]
    fn test_bencode_integer_negative_parse() {
        let s = "i-2131e";
        assert_eq!(
            BCObject::Integer(-2131),
            BCObject::parse_integer(&mut s.chars().peekable()).unwrap()
        );
    }

    #[test]
    fn test_bencode_integer_zero() {
        let s = "i0e";
        assert_eq!(BCObject::Integer(0), BCObject::parse_integer(&mut s.chars().peekable()).unwrap());
    }

    #[test]
    fn test_bencode_integer_no_premature_end() {
        let bad = "i324";
        assert!(BCObject::parse_integer(&mut bad.chars().peekable()).is_err());
    }

    #[test]
    fn test_bencode_integer_no_missing_leading_character() {
        let bad = "812";
        assert!(BCObject::parse_integer(&mut bad.chars().peekable()).is_err());
    }

    #[test]
    fn test_bencode_integer_no_negative_zero() {
        let bad= "i-0e";
        assert!(BCObject::parse_integer(&mut bad.chars().peekable()).is_err());
    }

    #[test]
    fn test_bencode_integer_no_leading_zero() {
        let bad= "i0123e";
        assert!(BCObject::parse_integer(&mut bad.chars().peekable()).is_err());
    }

    #[test]
    fn test_bencode_integer_no_negative_leading_zero() {
        let bad= "i-0123e";
        assert!(BCObject::parse_integer(&mut bad.chars().peekable()).is_err());
    }

    #[test]
    fn test_bencode_string_parse() {
        let s = "11:hello world";
        assert_eq!(
            BCObject::String("hello world".to_string()),
            BCObject::parse_string(&mut s.chars().peekable()).unwrap()
        );
    }

    #[test]
    fn test_bencode_string_no_premature_end() {
        let bad = "11:hello w";
        assert!(BCObject::parse_string(&mut bad.chars().peekable()).is_err());
    }

    #[test]
    fn test_bencode_string_no_missing_leading_len() {
        let bad = ":hello";
        assert!(BCObject::parse_string(&mut bad.chars().peekable()).is_err());
    }

    #[test]
    fn test_bencode_string_no_missing_leading_delimiter() {
        let bad = "hello";
        assert!(BCObject::parse_string(&mut bad.chars().peekable()).is_err());
    }

    #[test]
    fn test_bencode_list_parse() {
        let s = "li123ei456ei789ee";
        let v = vec![
            BCObject::Integer(123),
            BCObject::Integer(456),
            BCObject::Integer(789),
        ];
        assert_eq!(
            BCObject::List(v),
            BCObject::parse_list(&mut s.chars().peekable()).unwrap()
        );
    }

    #[test]
    fn test_bencode_list_nested() {
        let s = "li123ei456ei789el4:12344:5678ee";
        let v = vec![
            BCObject::Integer(123),
            BCObject::Integer(456),
            BCObject::Integer(789),
            BCObject::List(vec![
                BCObject::String("1234".to_string()),
                BCObject::String("5678".to_string()),
            ]),
        ];
        assert_eq!(
            BCObject::List(v),
            BCObject::parse_list(&mut s.chars().peekable()).unwrap()
        );
    }

    #[test]
    fn test_bencode_list_no_premature_end() {
        let bad = "li123ei456ei789e";
        assert!(BCObject::parse_list(&mut bad.chars().peekable()).is_err());
    }

    #[test]
    fn test_bencode_list_no_missing_leading_character() {
        let bad = "i123ei456ei789e";
        assert!(BCObject::parse_list(&mut bad.chars().peekable()).is_err());
    }

    #[test]
    fn test_bencode_dictionary_parse() {
        let s = "d5:hello5:world5:valuei123ee";
        let mut m: BTreeMap<String, BCObject> = BTreeMap::new();
        m.insert("hello".to_string(), BCObject::String("world".to_string()));
        m.insert("value".to_string(), BCObject::Integer(123));
        assert_eq!(
            BCObject::Dictionary(m),
            BCObject::parse_dictionary(&mut s.chars().peekable()).unwrap()
        );
    }

    #[test]
    fn test_bencode_dictionary_nested() {
        let s = "d5:hellod4:name5:worldee";
        let mut m: BTreeMap<String, BCObject> = BTreeMap::new();
        let mut m2: BTreeMap<String, BCObject> = BTreeMap::new();
        m2.insert("name".to_string(), BCObject::String("world".to_string()));
        m.insert("hello".to_string(), BCObject::Dictionary(m2));
        assert_eq!(
            BCObject::Dictionary(m),
            BCObject::parse_dictionary(&mut s.chars().peekable()).unwrap()
        );
    }

    #[test]
    fn test_bencode_dictionary_no_premature_end() {
        let bad = "d5:hello5:world5:valuei123e";
        assert!(BCObject::parse_dictionary(&mut bad.chars().peekable()).is_err());
    }

    #[test]
    fn test_bencode_dictionary_no_missing_leading_character() {
        let bad = "5:hello5:world5:valuei123e";
        assert!(BCObject::parse_dictionary(&mut bad.chars().peekable()).is_err());
    }
}

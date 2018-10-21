use std::fmt;
use std::str::FromStr;
use serde;

// This is a struct so that we can change how Ids are created
// and store relevant state to do so if we need to.
pub struct IdGen {}

impl IdGen {
    pub fn new() -> IdGen {
        IdGen{}
    }

    // This doesn't need mut self at the moment, but eventually
    // we'll prob make it more efficient by keeping state:
    pub fn make_id(&mut self) -> Id {
        loop {
            // don't allow 0 ID: this is seen as empty, or no id.
            // we only really distinguish so that we can print the
            // "no id" differently when displaying.
            let new_id = rand::random();
            if new_id == [0; 16] { continue };
            return Id { val: new_id }
        }
    }
}

#[derive(Debug,Eq,PartialEq,Ord,PartialOrd,Clone,Copy,Hash)]
pub struct Id {
    val: [u8; 16]
}

impl Id {
    pub fn none() -> Id {
        Id { val: [0; 16] }
    }
}

// How to get an Id from a string:
impl FromStr for Id {
    type Err = base64::DecodeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut v: Vec<u8> = Vec::with_capacity(16);
        base64::decode_config_buf(s, base64::URL_SAFE_NO_PAD, &mut v)?;
        if v.len() != 16 { return Err(base64::DecodeError::InvalidLength) }
        let mut a = [0;16];
        a.copy_from_slice(&v);
        Ok(Id { val: a })
    }
}

// How to turn an Id into a string:
fn to_str(id: &Id) -> String {
    base64::encode_config(&id.val, base64::URL_SAFE_NO_PAD)
}

// Display trait just turns Id into string rep:
impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.val == [0; 16] {
            write!(f, "<<no_id>>")
        } else {
            write!(f, "{}", to_str(self))
        }
    }
}

// Serializing treats Id struct as a string
impl serde::Serialize for Id {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&to_str(self))
    }
}

// Deserializing assumes Id will come from a string
impl<'de> serde::Deserialize<'de> for Id {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Id::from_str(&s).map_err(serde::de::Error::custom)
    }
}
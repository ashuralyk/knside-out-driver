use std::error::Error;
use std::fmt;

use derive_more::Display;
use hex::FromHexError;
use serde::{de, Deserialize, Serialize};

use crate::types::error::{ErrorType, KoError};
use crate::KoResult;

const HEX_PREFIX: &str = "0x";
const HEX_PREFIX_UPPER: &str = "0X";

#[derive(Display, Debug)]
enum HexError {
    _InvalidPrefix,

    #[display(fmt = "Decode hex error {}", _0)]
    Decode(FromHexError),
}

impl Error for HexError {}

impl From<FromHexError> for HexError {
    fn from(err: FromHexError) -> Self {
        HexError::Decode(err)
    }
}

impl From<HexError> for KoError {
    fn from(e: HexError) -> Self {
        KoError::new(ErrorType::Hex, Box::new(e))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Hex(String);

impl Hex {
    pub fn empty() -> Self {
        Hex(String::from(HEX_PREFIX))
    }

    pub fn is_empty(&self) -> bool {
        self.0.len() == 2
    }

    pub fn encode<T: AsRef<[u8]>>(src: T) -> Self {
        let mut s = HEX_PREFIX.to_string();
        s.push_str(&hex::encode(src));
        Hex(s)
    }

    pub fn decode(s: String) -> KoResult<Vec<u8>> {
        let s = if Self::is_prefixed(s.as_str()) {
            &s[2..]
        } else {
            s.as_str()
        };

        Ok(hex::decode(s).map_err(HexError::Decode)?)
    }

    pub fn from_string(s: String) -> KoResult<Self> {
        let s = if Self::is_prefixed(s.as_str()) {
            s
        } else {
            HEX_PREFIX.to_string() + &s
        };

        let _ = hex::decode(&s[2..]).map_err(HexError::Decode)?;
        Ok(Hex(s))
    }

    pub fn as_string(&self) -> String {
        self.0.to_owned()
    }

    pub fn as_string_trim0x(&self) -> String {
        (self.0[2..]).to_owned()
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        hex::decode(&self.0[2..]).expect("impossible, already checked in from_string")
    }

    fn is_prefixed(s: &str) -> bool {
        s.starts_with(HEX_PREFIX) || s.starts_with(HEX_PREFIX_UPPER)
    }
}

impl Default for Hex {
    fn default() -> Self {
        Hex(String::from("0x0000000000000000"))
    }
}

impl Serialize for Hex {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

struct HexVisitor;

impl<'de> de::Visitor<'de> for HexVisitor {
    type Value = Hex;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("Expect a hex string")
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Hex::from_string(v).map_err(|e| de::Error::custom(e.to_string()))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Hex::from_string(v.to_owned()).map_err(|e| de::Error::custom(e.to_string()))
    }
}

impl<'de> Deserialize<'de> for Hex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_string(HexVisitor)
    }
}

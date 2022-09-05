use ckb_types::bytes::Bytes;
use ckb_types::packed::Byte32;
use ckb_types::prelude::{Pack, Unpack};
use serde::{Deserialize, Serialize};
use std::convert::{AsRef, TryFrom};
use std::fmt::{self, Debug, Display};

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct H256(ckb_types::H256);

impl H256 {
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    pub fn as_bytes32(&self) -> &[u8; 32] {
        &self.0 .0
    }
}

impl From<H256> for ckb_types::H256 {
    fn from(hash: H256) -> Self {
        hash.0
    }
}

impl<'a> From<&'a H256> for &'a ckb_types::H256 {
    fn from(hash: &'a H256) -> Self {
        &hash.0
    }
}

impl From<[u8; 32]> for H256 {
    fn from(bytes: [u8; 32]) -> Self {
        H256(bytes.into())
    }
}

impl From<ckb_types::H256> for H256 {
    fn from(hash: ckb_types::H256) -> Self {
        H256(hash)
    }
}

impl AsRef<[u8]> for H256 {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl Pack<Byte32> for H256 {
    fn pack(&self) -> Byte32 {
        self.0.pack()
    }
}

impl Unpack<H256> for Byte32 {
    fn unpack(&self) -> H256 {
        H256(self.unpack())
    }
}

impl TryFrom<Bytes> for H256 {
    type Error = String;

    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        let hash = ckb_types::H256::from_slice(&value).map_err(|err| err.to_string())?;
        Ok(H256(hash))
    }
}

impl Display for H256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self))
    }
}

impl Debug for H256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

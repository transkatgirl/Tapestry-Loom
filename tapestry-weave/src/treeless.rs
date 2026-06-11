use universal_weave::rkyv::{
    Archive, Deserialize, Serialize, from_bytes, rancor::Error, to_bytes, util::AlignedVec,
};

use crate::v1::metadata::WeaveMetadata;

pub const FILE_EXTENSION: &str = "tapestrytext";

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct TextOnlyDocument {
    pub content: Vec<u8>,
    pub metadata: WeaveMetadata,
}

impl TextOnlyDocument {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        from_bytes::<_, Error>(bytes)
    }
    pub fn to_bytes(&self) -> Result<AlignedVec, Error> {
        to_bytes::<Error>(self)
    }
}

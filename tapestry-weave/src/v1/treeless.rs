use jiff::Zoned;
use universal_weave::rkyv::{
    Archive, Deserialize, Serialize, access, access_unchecked, deserialize, from_bytes,
    from_bytes_unchecked, rancor::Error, to_bytes, util::AlignedVec,
};

use super::metadata::{ConvertedFrom, MetadataMap, WeaveMetadata};

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
    pub unsafe fn from_bytes_unchecked(bytes: &[u8]) -> Result<Self, Error> {
        unsafe { from_bytes_unchecked::<_, Error>(bytes) }
    }
    pub fn from_archived(value: &ArchivedTextOnlyDocument) -> Result<Self, Error> {
        deserialize(value)
    }
    pub fn to_bytes(&self) -> Result<AlignedVec, Error> {
        to_bytes::<Error>(self)
    }
}

impl From<Vec<u8>> for TextOnlyDocument {
    fn from(value: Vec<u8>) -> Self {
        let timestamp = Zoned::now();

        Self {
            content: value,
            metadata: WeaveMetadata {
                title: None,
                description: None,
                created: timestamp.clone(),
                converted_from: vec![ConvertedFrom {
                    source: "Plaintext".to_string(),
                    source_version: None,
                    timestamp,
                }],
                metadata: MetadataMap::default(),
            },
        }
    }
}

impl ArchivedTextOnlyDocument {
    pub fn from_bytes(bytes: &[u8]) -> Result<&Self, Error> {
        access(bytes)
    }
    pub unsafe fn from_bytes_unchecked(bytes: &[u8]) -> &Self {
        unsafe { access_unchecked(bytes) }
    }
}

use universal_weave::{
    indexmap::IndexMap,
    rkyv::{
        Archive, Deserialize, Serialize, from_bytes, rancor::Error, to_bytes, util::AlignedVec,
    },
};

pub const FILE_EXTENSION: &str = "tapestrytext";

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct TextOnlyDocument {
    pub content: Vec<u8>,
    pub metadata: IndexMap<String, String>,
}

impl TextOnlyDocument {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        from_bytes::<_, Error>(bytes)
    }
    pub fn to_bytes(&self) -> Result<AlignedVec, Error> {
        to_bytes::<Error>(self)
    }
}

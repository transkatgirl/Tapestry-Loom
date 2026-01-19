// TODO: v1 format using IndependentWeave
// TODO: Duplicate nodes before splitting/merging/moving
// TODO: Add temporary nodes which are not actually stored in the IndependentWeave?

use universal_weave::{rkyv::rancor::Error, versioning::VersionedBytes};

pub use foldhash;
pub use ulid;
pub use universal_weave;

pub mod hashers;
pub mod treeless;
pub mod v0;
//pub mod v1;

pub const VERSIONED_WEAVE_FILE_EXTENSION: &str = "tapestry";

#[non_exhaustive]
pub enum VersionedWeave {
    V0(v0::TapestryWeave),
    //V1(v1::TapestryWeave),
}

const FORMAT_IDENTIFIER: [u8; 24] = *b"VersionedTapestryWeave__";

impl VersionedWeave {
    pub fn from_bytes(value: &[u8]) -> Option<Result<Self, Error>> {
        if let Some(versioned) = VersionedBytes::try_from_bytes(value, FORMAT_IDENTIFIER) {
            match versioned.version {
                0 => Some(v0::TapestryWeave::from_unversioned_bytes(versioned.data).map(Self::V0)),
                //1 => Some(v1::TapestryWeave::from_unversioned_bytes(versioned.data).map(Self::V1)),
                _ => None,
            }
        } else {
            None
        }
    }
    pub fn into_latest(self) -> v0::TapestryWeave {
        match self {
            Self::V0(weave) => weave,
        }
    }
    /*pub fn into_latest(self) -> v1::TapestryWeave {
        match self {
            Self::V0(weave) => v1::TapestryWeave::from(weave),
            Self::V1(weave) => weave,
        }
    }*/
    pub fn to_bytes(self) -> Result<Vec<u8>, Error> {
        let (version, bytes) = match self {
            Self::V0(weave) => (0, weave.to_unversioned_bytes()?),
            //Self::V1(weave) => (1, weave.to_unversioned_bytes()?),
        };

        Ok(to_versioned_bytes(version, &bytes))
    }
}

fn to_versioned_bytes(version: u64, data: &[u8]) -> Vec<u8> {
    let versioned = VersionedBytes {
        format_identifier: FORMAT_IDENTIFIER,
        version,
        data,
    };

    let mut output = Vec::with_capacity(versioned.output_length());
    versioned.to_bytes(&mut output);

    output
}

// TODO:
// - Implement v1 format based on IndependentWeave
//   - Implement diff-based tree updates
//   - Implement prefix-based deduplication?
//   - Implement support for editor undo/redo
//   - Implement event-based invalidation support for multi-user weaves

// Useful reference for future v1 format: https://github.com/transkatgirl/Tapestry-Loom/blob/a232fbbb4119a8a9047ca67a8f1b0cfb772c5bb1/weave/src/document/content/mod.rs

#![allow(clippy::missing_safety_doc)]

// TODO: Implement streaming serialization

use rkyv::util::AlignedVec;
use universal_weave::{rkyv::rancor::Error, versioning::VersionedBytes};

pub use foldhash;

#[cfg(feature = "v0")]
pub use ulid;

#[cfg(feature = "v1")]
pub use jiff;

pub use universal_weave;

pub mod hashers;

#[cfg(feature = "v0")]
pub mod v0;

#[cfg(feature = "v1")]
pub mod v1;

pub mod wrappers;

pub const VERSIONED_WEAVE_FILE_EXTENSION: &str = "tapestry";
const FORMAT_IDENTIFIER: [u8; 24] = *b"VersionedTapestryWeave__";

#[allow(clippy::non_minimal_cfg)]
#[non_exhaustive]
#[cfg(any(feature = "v1"))]
pub enum ArchivedVersionedWeave<'a> {
    #[cfg(feature = "v1")]
    /// WIP
    V1Dependent(v1::dependent::ArchivedTapestryWeave<'a>),
    #[cfg(feature = "v1")]
    /// WIP
    V1Independent(v1::independent::ArchivedTapestryWeave<'a>),
}

#[allow(clippy::non_minimal_cfg)]
#[cfg(any(feature = "v1"))]
impl<'a> ArchivedVersionedWeave<'a> {
    pub fn from_bytes(value: &'a [u8]) -> Option<Result<Self, Error>> {
        if let Some(versioned) = VersionedBytes::try_from_bytes(value, FORMAT_IDENTIFIER) {
            match versioned.version {
                #[cfg(feature = "v1")]
                1 => Some(
                    v1::dependent::ArchivedTapestryWeave::from_unversioned_bytes(versioned.data)
                        .map(Self::V1Dependent),
                ),
                #[cfg(feature = "v1")]
                2 => Some(
                    v1::independent::ArchivedTapestryWeave::from_unversioned_bytes(versioned.data)
                        .map(Self::V1Independent),
                ),
                _ => None,
            }
        } else {
            None
        }
    }
    pub unsafe fn from_bytes_unchecked(value: &'a [u8]) -> Option<Self> {
        if let Some(versioned) = VersionedBytes::try_from_bytes(value, FORMAT_IDENTIFIER) {
            match versioned.version {
                #[cfg(feature = "v1")]
                1 => Some(Self::V1Dependent(unsafe {
                    v1::dependent::ArchivedTapestryWeave::from_unversioned_bytes_unchecked(
                        versioned.data,
                    )
                })),
                #[cfg(feature = "v1")]
                2 => Some(Self::V1Independent(unsafe {
                    v1::independent::ArchivedTapestryWeave::from_unversioned_bytes_unchecked(
                        versioned.data,
                    )
                })),
                _ => None,
            }
        } else {
            None
        }
    }
}

#[allow(clippy::large_enum_variant)]
#[non_exhaustive]
pub enum VersionedWeave {
    #[cfg(feature = "v0")]
    V0(v0::TapestryWeave),
    #[cfg(feature = "v1")]
    /// WIP
    V1Dependent(v1::dependent::TapestryWeave),
    #[cfg(feature = "v1")]
    /// WIP
    V1Independent(v1::independent::TapestryWeave),
}

impl VersionedWeave {
    pub fn from_bytes(value: &[u8]) -> Option<Result<Self, Error>> {
        if let Some(versioned) = VersionedBytes::try_from_bytes(value, FORMAT_IDENTIFIER) {
            match versioned.version {
                #[cfg(feature = "v0")]
                0 => Some(v0::TapestryWeave::from_unversioned_bytes(versioned.data).map(Self::V0)),
                #[cfg(feature = "v1")]
                1 => Some(
                    v1::dependent::TapestryWeave::from_unversioned_bytes(versioned.data)
                        .map(Self::V1Dependent),
                ),
                #[cfg(feature = "v1")]
                2 => Some(
                    v1::independent::TapestryWeave::from_unversioned_bytes(versioned.data)
                        .map(Self::V1Independent),
                ),
                _ => None,
            }
        } else {
            None
        }
    }
    pub unsafe fn from_bytes_unchecked(value: &[u8]) -> Option<Result<Self, Error>> {
        if let Some(versioned) = VersionedBytes::try_from_bytes(value, FORMAT_IDENTIFIER) {
            match versioned.version {
                #[cfg(feature = "v0")]
                0 => Some(
                    unsafe { v0::TapestryWeave::from_unversioned_bytes_unchecked(versioned.data) }
                        .map(Self::V0),
                ),
                #[cfg(feature = "v1")]
                1 => Some(
                    unsafe {
                        v1::dependent::TapestryWeave::from_unversioned_bytes_unchecked(
                            versioned.data,
                        )
                    }
                    .map(Self::V1Dependent),
                ),
                #[cfg(feature = "v1")]
                2 => Some(
                    unsafe {
                        v1::independent::TapestryWeave::from_unversioned_bytes_unchecked(
                            versioned.data,
                        )
                    }
                    .map(Self::V1Independent),
                ),
                _ => None,
            }
        } else {
            None
        }
    }
    #[allow(unreachable_patterns)]
    #[cfg(feature = "v0")]
    pub fn into_v0(self) -> Option<v0::TapestryWeave> {
        match self {
            Self::V0(weave) => Some(weave),
            _ => None,
        }
    }
    #[cfg(all(feature = "v0", feature = "v1"))]
    pub fn into_v1_dependent(self) -> Option<v1::dependent::TapestryWeave> {
        match self {
            Self::V0(weave) => Some(v1::dependent::TapestryWeave::from(weave)),
            Self::V1Dependent(weave) => Some(weave),
            _ => None,
        }
    }
    #[cfg(all(feature = "v0", feature = "v1"))]
    pub fn into_v1_independent(self) -> Option<v1::independent::TapestryWeave> {
        match self {
            Self::V0(weave) => Some(v1::independent::TapestryWeave::from(weave)),
            Self::V1Dependent(weave) => Some(v1::independent::TapestryWeave::from(weave)),
            Self::V1Independent(weave) => Some(weave),
        }
    }
    #[cfg(all(feature = "v0", feature = "v1"))]
    pub fn into_latest(self) -> v1::dependent::TapestryWeave {
        match self {
            Self::V0(weave) => v1::dependent::TapestryWeave::from(weave),
            Self::V1Dependent(weave) => weave,
            Self::V1Independent(_) => unimplemented!(),
        }
    }
    pub fn to_bytes(self) -> Result<Vec<u8>, Error> {
        let (version, bytes): (u64, AlignedVec) = match self {
            #[cfg(feature = "v0")]
            Self::V0(weave) => (0, weave.to_unversioned_bytes()?),
            #[cfg(feature = "v1")]
            Self::V1Dependent(weave) => (1, weave.to_unversioned_bytes()?),
            #[cfg(feature = "v1")]
            Self::V1Independent(weave) => (2, weave.to_unversioned_bytes()?),
        };

        Ok(to_versioned_bytes(version, &bytes))
    }
}

// FIXME - This function results in an unnecessary memory copy
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

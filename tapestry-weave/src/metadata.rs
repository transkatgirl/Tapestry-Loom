//! Document metadata representations.

use foldhash::fast::RandomState;
use jiff::Zoned;
use serde::{Deserialize as SerdeDeserialize, Serialize as SerdeSerialize};
use universal_weave::{
    indexmap::IndexMap,
    rkyv::{Archive, Deserialize, Serialize},
};

use super::util::{AsBinaryZoned, IAsVec};

/// User-readable metadata.
pub type MetadataMap = IndexMap<String, String, RandomState>;

/// Machine-readable metadata.
///
/// Fields not recognized by the application should not be displayed to the user.
pub type AuxMetadataMap = IndexMap<String, Vec<u8>, RandomState>;

/// Document-wide metadata.
#[derive(
    SerdeSerialize, SerdeDeserialize, Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq,
)]
pub struct WeaveMetadata {
    /// The title of the document.
    pub title: Option<String>,
    /// The document's description or notes.
    pub description: Option<String>,
    /// The instant the document was created.
    #[rkyv(with = AsBinaryZoned)]
    pub created: Zoned,
    /// A list of format conversions that the document has undergone, ordered from oldest to newest.
    pub converted_from: Vec<ConvertedFrom>,

    /// User-readable metadata associated with the document.
    #[rkyv(with = IAsVec)]
    pub metadata: MetadataMap,

    /// Machine-readable metadata associated with the document.
    ///
    /// Fields not recognized by the application should not be displayed to the user.
    #[rkyv(with = IAsVec)]
    pub aux_metadata: AuxMetadataMap,
}

impl WeaveMetadata {
    /// Creates a new, empty `WeaveMetadata` with the current timestamp.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            title: None,
            description: None,
            created: Zoned::now(),
            converted_from: Vec::new(),
            metadata: MetadataMap::default(),
            aux_metadata: AuxMetadataMap::default(),
        }
    }
    /// Returns `true` if all user-readable metadata fields are empty.
    pub fn is_empty(&self) -> bool {
        self.title.as_ref().map(|v| v.is_empty()).unwrap_or(true)
            && self
                .description
                .as_ref()
                .map(|v| v.is_empty())
                .unwrap_or(true)
            && self.converted_from.is_empty()
            && self.metadata.is_empty()
    }
}

impl ArchivedWeaveMetadata {
    /// Returns `true` if all user-readable metadata fields are empty.
    pub fn is_empty(&self) -> bool {
        self.title.as_ref().map(|v| v.is_empty()).unwrap_or(true)
            && self
                .description
                .as_ref()
                .map(|v| v.is_empty())
                .unwrap_or(true)
            && self.converted_from.is_empty()
            && self.metadata.is_empty()
    }
}

/// A format conversion that a document has undergone.
#[derive(
    SerdeSerialize, SerdeDeserialize, Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq,
)]
pub struct ConvertedFrom {
    /// The name of the source format.
    pub source: String,
    /// The version of the source format.
    pub source_version: Option<String>,

    /// The software used to perform the format conversion.
    pub converter: String,
    /// The version of the software used to perform the format conversion.
    pub converter_version: Option<String>,

    /// The instant the document was converted.
    #[rkyv(with = AsBinaryZoned)]
    pub timestamp: Zoned,
}

impl ConvertedFrom {
    /// Returns `true` if the format conversion was performed by this library.
    pub fn is_converter_native(&self) -> bool {
        self.converter == env!("CARGO_PKG_NAME")
    }
    /// Returns `true` if the format conversion was performed by the current version of this library.
    pub fn is_converter_native_current_version(&self) -> bool {
        self.converter == env!("CARGO_PKG_NAME")
            && self.converter_version.as_deref() == Some(env!("CARGO_PKG_VERSION"))
    }
    pub(crate) fn from_plaintext() -> Self {
        ConvertedFrom {
            source: "Plain Text".to_string(),
            source_version: None,
            converter: env!("CARGO_PKG_NAME").to_string(),
            converter_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            timestamp: Zoned::now(),
        }
    }
    /// Returns `true` if the conversion was from plaintext.
    pub fn is_from_plaintext(&self) -> bool {
        self.source == "Plain Text" && self.source_version.is_none()
    }
    pub(crate) fn from_version(version: u64) -> Self {
        ConvertedFrom {
            source: "Tapestry Loom".to_string(),
            source_version: Some(version.to_string()),
            converter: env!("CARGO_PKG_NAME").to_string(),
            converter_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            timestamp: Zoned::now(),
        }
    }
    /// Returns `true` if the conversion was from an older version of the Tapestry Loom format.
    pub fn is_from_older(&self) -> bool {
        self.source == "Tapestry Loom"
            && self
                .source_version
                .as_deref()
                .and_then(|v| v.parse::<u64>().ok())
                .is_some_and(|value| value < crate::weave::FORMAT_VERSION)
    }
    /// Returns `true` if the conversion was from the current version of the Tapestry Loom format.
    pub fn is_from_current(&self) -> bool {
        self.source == "Tapestry Loom"
            && self
                .source_version
                .as_deref()
                .and_then(|v| v.parse::<u64>().ok())
                .is_some_and(|value| value == crate::weave::FORMAT_VERSION)
    }
    /// Returns `true` if the conversion was from a future (not yet implemented by this library) version of the Tapestry Loom format.
    pub fn is_from_newer(&self) -> bool {
        self.source == "Tapestry Loom"
            && self
                .source_version
                .as_deref()
                .and_then(|v| v.parse::<u64>().ok())
                .is_some_and(|value| value > crate::weave::FORMAT_VERSION)
    }
}

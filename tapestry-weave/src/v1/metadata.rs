#[cfg(feature = "v0")]
use std::str::FromStr;

use foldhash::fast::RandomState;
use jiff::Zoned;

use universal_weave::{
    indexmap::IndexMap,
    rkyv::{Archive, Deserialize, Serialize},
};

#[cfg(feature = "v0")]
use chrono::DateTime;

#[cfg(feature = "v0")]
use jiff::fmt::rfc2822::DateTimeParser;

#[cfg(feature = "serde")]
use serde::{Deserialize as SerdeDeserialize, Serialize as SerdeSerialize};

use super::super::wrappers::{AsBinaryZoned, IAsVec};

pub type MetadataMap = IndexMap<String, String, RandomState>;
pub type AuxMetadataMap = IndexMap<String, Vec<u8>, RandomState>;

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(SerdeSerialize, SerdeDeserialize))]
pub struct WeaveMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    #[rkyv(with = AsBinaryZoned)]
    pub created: Zoned,
    pub converted_from: Vec<ConvertedFrom>,

    #[rkyv(with = IAsVec)]
    pub metadata: MetadataMap,
}

impl WeaveMetadata {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            title: None,
            description: None,
            created: Zoned::now(),
            converted_from: Vec::new(),
            metadata: MetadataMap::default(),
        }
    }
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

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(SerdeSerialize, SerdeDeserialize))]
pub struct ConvertedFrom {
    pub source: String,
    pub source_version: Option<String>,

    pub converter: String,
    pub converter_version: Option<String>,

    #[rkyv(with = AsBinaryZoned)]
    pub timestamp: Zoned,
}

impl ConvertedFrom {
    pub fn is_converter_native(&self) -> bool {
        self.converter == env!("CARGO_PKG_NAME")
    }
    pub fn is_converter_native_current_version(&self) -> bool {
        self.converter == env!("CARGO_PKG_NAME")
            && self.converter_version.as_deref() == Some(env!("CARGO_PKG_VERSION"))
    }

    pub fn from_plaintext(timestamp: Zoned) -> Self {
        ConvertedFrom {
            source: "Plaintext".to_string(),
            source_version: None,
            converter: env!("CARGO_PKG_NAME").to_string(),
            converter_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            timestamp,
        }
    }
    pub fn is_from_plaintext(&self) -> bool {
        self.source == "Plaintext" && self.source_version.is_none()
    }
    #[cfg(feature = "v0")]
    pub fn from_v0(timestamp: Zoned) -> Self {
        ConvertedFrom {
            source: "Tapestry Loom".to_string(),
            source_version: Some("0".to_string()),
            converter: env!("CARGO_PKG_NAME").to_string(),
            converter_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            timestamp,
        }
    }
    pub fn is_from_v0(&self) -> bool {
        self.source == "Tapestry Loom" && self.source_version.as_deref() == Some("0")
    }
    pub fn from_v1_dependent(timestamp: Zoned) -> Self {
        ConvertedFrom {
            source: "Tapestry Loom".to_string(),
            source_version: Some("1.dependent".to_string()),
            converter: env!("CARGO_PKG_NAME").to_string(),
            converter_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            timestamp,
        }
    }
    pub fn is_from_v1_dependent(&self) -> bool {
        self.source == "Tapestry Loom" && self.source_version.as_deref() == Some("1.dependent")
    }
    pub fn from_v1_independent(timestamp: Zoned) -> Self {
        ConvertedFrom {
            source: "Tapestry Loom".to_string(),
            source_version: Some("1.independent".to_string()),
            converter: env!("CARGO_PKG_NAME").to_string(),
            converter_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            timestamp,
        }
    }
    pub fn is_from_v1_independent(&self) -> bool {
        self.source == "Tapestry Loom" && self.source_version.as_deref() == Some("1.independent")
    }
}

#[cfg(feature = "v0")]
const PARSER: DateTimeParser = DateTimeParser::new();

#[cfg(feature = "v0")]
impl From<MetadataMap> for WeaveMetadata {
    fn from(mut value: MetadataMap) -> Self {
        let conversion_timestamp = value
            .shift_remove("converted")
            .and_then(|value| Zoned::from_str(&value).ok());
        let source = value.shift_remove("converted_from");
        let source_version = value.shift_remove("converted_from_version");

        let mut converted_from = Vec::with_capacity(2);

        if source.is_some() || source_version.is_some() || conversion_timestamp.is_some() {
            converted_from.push(ConvertedFrom {
                source: source.unwrap_or_else(|| "Unknown".to_string()),
                source_version,
                converter: "Unknown (likely migration-assistant)".to_string(),
                converter_version: None,
                timestamp: conversion_timestamp.unwrap_or_default(),
            });
        }

        converted_from.push(ConvertedFrom::from_v0(Zoned::now()));

        WeaveMetadata {
            title: value.shift_remove("title"),
            description: value
                .shift_remove("description")
                .or_else(|| value.shift_remove("notes")),
            created: value
                .shift_remove("created")
                .and_then(|value| {
                    DateTime::parse_from_rfc3339(&value)
                        .ok()
                        .and_then(|v| PARSER.parse_zoned(v.to_rfc2822()).ok())
                })
                .unwrap_or_default(),
            converted_from,
            metadata: value,
        }
    }
}

#[cfg(feature = "v0")]
use std::str::FromStr;

use foldhash::fast::RandomState;
use jiff::Zoned;
use universal_weave::{
    indexmap::IndexMap,
    rkyv::{Archive, Deserialize, Serialize},
};

use crate::wrappers::AsBinaryZoned;

pub type MetadataMap = IndexMap<String, String, RandomState>;

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct WeaveMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    #[rkyv(with = AsBinaryZoned)]
    pub created: Zoned,
    pub converted_from: Vec<ConvertedFrom>,

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
            metadata: IndexMap::default(),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.description
            .as_ref()
            .map(|v| v.is_empty())
            .unwrap_or(true)
            && self.title.as_ref().map(|v| v.is_empty()).unwrap_or(true)
            && self.converted_from.is_empty()
            && self.metadata.is_empty()
    }
}

impl ArchivedWeaveMetadata {
    pub fn is_empty(&self) -> bool {
        self.description
            .as_ref()
            .map(|v| v.is_empty())
            .unwrap_or(true)
            && self.title.as_ref().map(|v| v.is_empty()).unwrap_or(true)
            && self.converted_from.is_empty()
            && self.metadata.is_empty()
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct ConvertedFrom {
    pub source: String,
    pub source_version: Option<String>,

    #[rkyv(with = AsBinaryZoned)]
    pub timestamp: Zoned,
}

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
                timestamp: conversion_timestamp.unwrap_or_default(),
            });
        }

        converted_from.push(ConvertedFrom {
            source: "TapestryLoomBeta".to_string(),
            source_version: Some("0".to_string()),
            timestamp: Zoned::now(),
        });

        WeaveMetadata {
            title: value.shift_remove("title"),
            description: value
                .shift_remove("description")
                .or_else(|| value.shift_remove("notes")),
            created: value
                .shift_remove("created")
                .and_then(|value| Zoned::from_str(&value).ok())
                .unwrap_or_default(),
            converted_from,
            metadata: value,
        }
    }
}

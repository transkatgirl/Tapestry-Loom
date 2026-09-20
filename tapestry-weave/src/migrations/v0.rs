use std::hash::BuildHasherDefault;

use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use chrono::DateTime;
use foldhash::fast::RandomState;
use jiff::{
    Timestamp, Zoned,
    tz::{Offset, TimeZone},
};
use nanorand::{Rng, WyRand};
use serde::{Deserialize as SerdeDeserialize, Serialize as SerdeSerialize};
use ulid::Ulid;
#[allow(deprecated)]
use universal_weave::{
    MetadataWeave, Weave,
    dependent::{DependentWeave, legacy_dependent::DependentWeave as LegacyDependentWeave},
    indexmap::{IndexMap, IndexSet},
    rkyv::{Archive, Deserialize, Serialize},
};

use crate::{
    content::{
        CounterfactualToken, Creator, InnerNodeContent as NewInnerNodeContent, InnerNodeToken,
        Model as NewModel, NodeContent as NewNodeContent, OriginalToken, UNKNOWN_MODEL_LABEL,
    },
    metadata::{AuxMetadataMap, ConvertedFrom, WeaveMetadata as NewWeaveMetadata},
    util::{RandomIdHasher, UniqueIdentifierRemapper},
    weave::{LongId, ShortId, TapestryNode as NewTapestryNode, TapestryWeave as NewTapestryWeave},
};

pub const FORMAT_VERSION: u64 = 0;

#[derive(
    SerdeSerialize, SerdeDeserialize, Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq,
)]
pub struct NodeContent {
    pub content: InnerNodeContent,
    pub metadata: MetadataMap,
    pub model: Option<Model>,
}

#[derive(
    SerdeSerialize, SerdeDeserialize, Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq,
)]
pub enum InnerNodeContent {
    Snippet(Vec<u8>),
    Tokens(Vec<(Vec<u8>, MetadataMap)>),
}

#[derive(
    SerdeSerialize, SerdeDeserialize, Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq,
)]
pub struct Model {
    pub label: String,
    pub metadata: MetadataMap,
}

#[allow(deprecated)]
pub type TapestryWeave =
    LegacyDependentWeave<u128, NodeContent, MetadataMap, BuildHasherDefault<RandomIdHasher>>;
//pub type TapestryNode = DependentNode<u128, NodeContent, BuildHasherDefault<RandomIdHasher>>;
pub type MetadataMap = IndexMap<String, String, RandomState>;

const MIN_PROBABILITY: f32 = 0.00005;

pub fn deserialize_counterfactual_logprobs(logprobs: &str) -> Option<Vec<(Vec<u8>, MetadataMap)>> {
    serde_json::from_str::<Vec<(String, MetadataMap)>>(logprobs)
        .ok()
        .map(|logprobs| {
            logprobs
                .into_iter()
                .filter_map(|(data, metadata)| {
                    BASE64_URL_SAFE_NO_PAD
                        .decode(data)
                        .ok()
                        .map(|data| (data, metadata))
                })
                .collect()
        })
}

impl From<InnerNodeContent> for NewInnerNodeContent {
    fn from(value: InnerNodeContent) -> Self {
        match value {
            InnerNodeContent::Snippet(snippet) => Self::Snippet(snippet),
            InnerNodeContent::Tokens(tokens) => Self::Tokens(
                tokens
                    .into_iter()
                    .map(|(token, mut metadata)| {
                        let mut modified = metadata
                            .shift_remove("original_length")
                            .and_then(|value| value.parse::<usize>().ok())
                            .map(|original_length| original_length != token.len())
                            .unwrap_or(false);

                        if let Some(value) = metadata.shift_remove("modified")
                            && value == "true"
                        {
                            modified = true;
                        }

                        InnerNodeToken {
                            bytes: token,
                            logprob: metadata.shift_remove("probability").and_then(|value| {
                                value
                                    .parse::<f32>()
                                    .ok()
                                    .map(|p| p.max(MIN_PROBABILITY).ln())
                            }),
                            id: if !modified {
                                metadata
                                    .shift_remove("token_id")
                                    .and_then(|value| value.parse::<u64>().ok())
                            } else {
                                None
                            },
                            entropy: None,
                            counterfactual: metadata
                                .shift_remove("counterfactual")
                                .and_then(|value| {
                                    deserialize_counterfactual_logprobs(&value).map(
                                        |counterfactual| {
                                            counterfactual
                                                .into_iter()
                                                .map(|(token, mut metadata)| CounterfactualToken {
                                                    bytes: token,
                                                    logprob: metadata
                                                        .shift_remove("probability")
                                                        .and_then(|value| {
                                                            value.parse::<f32>().ok().map(|p| {
                                                                p.max(MIN_PROBABILITY).ln()
                                                            })
                                                        }),
                                                    id: metadata.shift_remove("token_id").and_then(
                                                        |value| value.parse::<u64>().ok(),
                                                    ),
                                                })
                                                .collect()
                                        },
                                    )
                                })
                                .unwrap_or_default(),
                            original: if modified {
                                OriginalToken::Unknown
                            } else {
                                OriginalToken::Unmodified
                            },
                        }
                    })
                    .collect(),
            ),
        }
    }
}

impl From<Model> for Creator {
    fn from(mut value: Model) -> Self {
        if value.label.to_lowercase() == "unknown model"
            || value.label.to_lowercase() == "unknown"
            || value.label.to_lowercase() == "n/a"
            || value.label.is_empty()
        {
            value.label = UNKNOWN_MODEL_LABEL.to_string();
        }

        Self::Model(
            if value.label == UNKNOWN_MODEL_LABEL && value.metadata.is_empty() {
                None
            } else {
                Some(NewModel {
                    label: value.label,
                    color: value.metadata.shift_remove("color"),
                    identifier: None,
                    seed: None,
                    system_fingerprint: None,
                    finish_reason: None,
                    metadata: value.metadata,
                })
            },
        )
    }
}

impl From<NodeContent> for NewNodeContent {
    fn from(mut value: NodeContent) -> Self {
        value.metadata.shift_remove("confidence");
        value.metadata.shift_remove("confidence_k");
        value.metadata.shift_remove("confidence_n");

        let mut creator = value.model.map(Creator::from).unwrap_or(Creator::Unknown);

        if let Creator::Model(Some(model)) = &mut creator {
            if let InnerNodeContent::Tokens(tokens) = &mut value.content {
                let mut model_id = None;

                for (_, metadata) in tokens {
                    if let Some(value) = metadata.shift_remove("model_id") {
                        if let Some(existing_id) = &model_id
                            && *existing_id != value
                        {
                            model_id = None;
                            break;
                        } else {
                            model_id = Some(value);
                        }
                    }
                }

                if let Some(model_id) = model_id
                    .and_then(|id| Ulid::from_string(&id).ok())
                    .and_then(|id| LongId::new(id.0))
                {
                    model.identifier = Some(model_id);
                }
            }

            model.seed = value
                .metadata
                .shift_remove("seed")
                .and_then(|id| id.parse::<u64>().ok());

            model.system_fingerprint = value.metadata.shift_remove("system_fingerprint");
            model.finish_reason = value.metadata.shift_remove("finish_reason");
        }

        let content = NewInnerNodeContent::from(value.content);

        let mut modified = if let NewInnerNodeContent::Tokens(tokens) = &content {
            tokens.iter().any(|token| token.original.is_modified())
        } else {
            false
        };

        if let Some(value) = value.metadata.shift_remove("modified")
            && value.to_lowercase() == "true"
        {
            modified = true;
        }

        Self {
            timestamp: Zoned::default(),
            modified,
            metadata: value.metadata,
            creator,
            content,
        }
    }
}

impl From<TapestryWeave> for NewTapestryWeave {
    fn from(value: TapestryWeave) -> Self {
        let mut value = DependentWeave::from(value);

        let mut output = NewTapestryWeave::with_capacity_and_metadata(
            value.capacity(),
            value.metadata.clone().into(),
        );

        let mut identifiers = Vec::with_capacity(value.len());
        value.get_ordered_identifiers(&mut identifiers);

        let mut mapper: UniqueIdentifierRemapper<
            u128,
            ShortId,
            BuildHasherDefault<RandomIdHasher>,
            BuildHasherDefault<RandomIdHasher>,
        > = UniqueIdentifierRemapper::with_capacity(identifiers.len());

        let time_zone = output.metadata().created.time_zone().clone();

        let mut rng = WyRand::new();

        let mut convert_old_identifier = |id| {
            *mapper
                .map_with_initial(id, id as ShortId, || rng.generate())
                .get()
        };

        for identifier in identifiers {
            let node = value.get(&identifier).unwrap().clone();

            let timestamp = Timestamp::try_from(Ulid(node.id).datetime())
                .map(|timestamp| Zoned::new(timestamp, time_zone.clone()))
                .unwrap_or(Zoned::default());

            let mut node = NewTapestryNode {
                id: convert_old_identifier(node.id),
                from: IndexSet::from_iter(node.from.into_iter().map(&mut convert_old_identifier)),
                to: IndexSet::with_capacity_and_hasher(
                    node.to.len(),
                    BuildHasherDefault::default(),
                ),
                active: node.active,
                bookmarked: node.bookmarked,
                contents: node.contents.into(),
            };
            node.contents.timestamp = timestamp;

            assert!(output.insert(node));
        }

        output
    }
}

fn parse_rfc3339(value: &str) -> Option<Zoned> {
    let parsed = DateTime::parse_from_rfc3339(value.trim()).ok()?;
    let timestamp = Timestamp::new(
        parsed.timestamp(),
        i32::try_from(parsed.timestamp_subsec_nanos().min(999_999_999)).ok()?,
    )
    .ok()?;
    let offset = Offset::from_seconds(parsed.offset().local_minus_utc()).ok()?;

    Some(timestamp.to_zoned(TimeZone::fixed(offset)))
}

impl From<MetadataMap> for NewWeaveMetadata {
    fn from(mut value: MetadataMap) -> Self {
        let conversion_timestamp = value
            .shift_remove("converted")
            .and_then(|value| parse_rfc3339(&value));
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

        converted_from.push(ConvertedFrom::from_version(FORMAT_VERSION));

        Self {
            title: value.shift_remove("title"),
            description: value
                .shift_remove("description")
                .or_else(|| value.shift_remove("notes")),
            created: value
                .shift_remove("created")
                .and_then(|value| parse_rfc3339(&value))
                .unwrap_or_default(),
            converted_from,
            metadata: value,
            aux_metadata: AuxMetadataMap::default(),
        }
    }
}

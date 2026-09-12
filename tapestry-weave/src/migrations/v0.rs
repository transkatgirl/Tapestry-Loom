use std::{hash::BuildHasherDefault, num::NonZeroU128};

use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use foldhash::fast::RandomState;
use jiff::{Timestamp, Zoned};
use nanorand::{Rng, WyRand};
use ulid::Ulid;
#[allow(deprecated)]
use universal_weave::{
    DiscreteContentResult, DiscreteContents, MetadataWeave, Weave,
    dependent::{DependentWeave, legacy_dependent::DependentWeave as LegacyDependentWeave},
    indexmap::{IndexMap, IndexSet},
    rkyv::{Archive, Deserialize, Serialize},
};

#[cfg(feature = "serde")]
use serde::{Deserialize as SerdeDeserialize, Serialize as SerdeSerialize};

use crate::{
    content::{
        CounterfactualToken, Creator, InnerNodeContent as NewInnerNodeContent, InnerNodeToken,
        Model as NewModel, NodeContent as NewNodeContent, OriginalToken, UNKNOWN_MODEL_LABEL,
    },
    hashers::{RandomIdHasher, UlidHasher},
    metadata::AuxMetadataMap,
    weave::{TapestryNode as NewTapestryNode, TapestryWeave as NewTapestryWeave},
    wrappers::UniqueIdentifierRemapper,
};

pub(crate) const FORMAT_VERSION: u64 = 0;

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(SerdeSerialize, SerdeDeserialize))]
pub struct NodeContent {
    pub content: InnerNodeContent,
    pub metadata: MetadataMap,
    pub model: Option<Model>,
}

impl DiscreteContents for NodeContent {
    fn len(&self) -> usize {
        self.content.len()
    }
    fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
    fn split(mut self, at: usize) -> DiscreteContentResult<Self> {
        match self.content.split(at) {
            DiscreteContentResult::Two(left, right) => {
                self.content = left;

                let right_content = NodeContent {
                    content: right,
                    metadata: self.metadata.clone(),
                    model: self.model.clone(),
                };

                DiscreteContentResult::Two(self, right_content)
            }
            DiscreteContentResult::One(center) => {
                self.content = center;
                DiscreteContentResult::One(self)
            }
        }
    }
    fn merge(mut self, mut value: Self) -> DiscreteContentResult<Self> {
        if self.metadata != value.metadata || self.model != value.model {
            return DiscreteContentResult::Two(self, value);
        }

        match self.content.merge(value.content) {
            DiscreteContentResult::Two(left, right) => {
                self.content = left;
                value.content = right;

                DiscreteContentResult::Two(self, value)
            }
            DiscreteContentResult::One(center) => {
                self.content = center;
                DiscreteContentResult::One(self)
            }
        }
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(SerdeSerialize, SerdeDeserialize))]
pub enum InnerNodeContent {
    Snippet(Vec<u8>),
    Tokens(Vec<(Vec<u8>, MetadataMap)>),
}

impl InnerNodeContent {
    fn len(&self) -> usize {
        match self {
            Self::Snippet(snippet) => snippet.len(),
            Self::Tokens(tokens) => tokens.iter().map(|token| token.0.len()).sum(),
        }
    }
    fn is_empty(&self) -> bool {
        match self {
            Self::Snippet(snippet) => snippet.is_empty(),
            Self::Tokens(tokens) => tokens.iter().all(|token| token.0.is_empty()),
        }
    }
    fn split(self, at: usize) -> DiscreteContentResult<Self> {
        if at == 0 {
            return DiscreteContentResult::One(self);
        }

        match self {
            Self::Snippet(mut snippet) => {
                if snippet.len() <= at {
                    return DiscreteContentResult::One(Self::Snippet(snippet));
                }

                let right = snippet.split_off(at);
                snippet.shrink_to_fit();

                DiscreteContentResult::Two(Self::Snippet(snippet), Self::Snippet(right))
            }
            Self::Tokens(tokens) => {
                if tokens.iter().map(|token| token.0.len()).sum::<usize>() <= at {
                    return DiscreteContentResult::One(Self::Tokens(tokens));
                }

                let mut content_index = 0;

                let location = tokens.iter().enumerate().find_map(|(location, token)| {
                    if content_index + token.0.len() > at {
                        return Some(location);
                    }
                    content_index += token.0.len();

                    None
                });

                if let Some(location) = location {
                    let mut left = tokens;
                    let mut right = left.split_off(location);
                    left.shrink_to_fit();

                    let mut left_token = right[0].0.clone();
                    let right_token = left_token.split_off(at - content_index);

                    debug_assert!(!right_token.is_empty() || left_token.is_empty());

                    if !left_token.is_empty() {
                        left_token.shrink_to_fit();
                        let mut left_metadata = right[0].1.clone();
                        left_metadata.shift_remove("token_id");
                        left.push((left_token, left_metadata));
                        right[0].1.shift_remove("token_id");
                    }
                    right[0].0 = right_token;

                    DiscreteContentResult::Two(Self::Tokens(left), Self::Tokens(right))
                } else {
                    DiscreteContentResult::One(Self::Tokens(tokens))
                }
            }
        }
    }
    fn merge(self, value: Self) -> DiscreteContentResult<Self> {
        match self {
            Self::Snippet(mut left_snippet) => match value {
                Self::Snippet(mut right_snippet) => {
                    left_snippet.append(&mut right_snippet);
                    DiscreteContentResult::One(Self::Snippet(left_snippet))
                }
                Self::Tokens(right_tokens) => DiscreteContentResult::Two(
                    Self::Snippet(left_snippet),
                    Self::Tokens(right_tokens),
                ),
            },
            Self::Tokens(mut left_tokens) => match value {
                Self::Snippet(right_snippet) => DiscreteContentResult::Two(
                    Self::Tokens(left_tokens),
                    Self::Snippet(right_snippet),
                ),
                Self::Tokens(mut right_tokens) => {
                    left_tokens.append(&mut right_tokens);
                    DiscreteContentResult::One(Self::Tokens(left_tokens))
                }
            },
        }
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(SerdeSerialize, SerdeDeserialize))]
pub struct Model {
    pub label: String,
    pub metadata: MetadataMap,
}

#[allow(deprecated)]
pub type TapestryWeave =
    LegacyDependentWeave<u128, NodeContent, MetadataMap, BuildHasherDefault<UlidHasher>>;
//pub type TapestryNode = DependentNode<u128, NodeContent, BuildHasherDefault<UlidHasher>>;
pub type MetadataMap = IndexMap<String, String, RandomState>;

/*pub fn serialize_counterfactual_logprobs(logprobs: Vec<(Vec<u8>, MetadataMap)>) -> String {
    let logprobs: Vec<_> = logprobs
        .into_iter()
        .map(|(data, metadata)| (BASE64_URL_SAFE_NO_PAD.encode(data), metadata))
        .collect();

    serde_json::to_string(&logprobs).unwrap()
}*/

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
                        metadata.shift_remove("model_id");
                        metadata.shift_remove("confidence");
                        metadata.shift_remove("confidence_k");

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
                            logprob: metadata
                                .shift_remove("probability")
                                .and_then(|value| value.parse::<f32>().ok())
                                .unwrap_or(f32::NAN)
                                .ln(),
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
                                                .map(|(token, mut metadata)| {
                                                    metadata.shift_remove("model_id");
                                                    metadata.shift_remove("confidence");
                                                    metadata.shift_remove("confidence_k");
                                                    metadata.shift_remove("original_length");
                                                    metadata.shift_remove("modified");

                                                    CounterfactualToken {
                                                        bytes: token,
                                                        logprob: metadata
                                                            .shift_remove("probability")
                                                            .and_then(|value| {
                                                                value.parse::<f32>().ok()
                                                            })
                                                            .unwrap_or(f32::NAN)
                                                            .ln(),
                                                        id: metadata
                                                            .shift_remove("token_id")
                                                            .and_then(|value| {
                                                                value.parse::<u64>().ok()
                                                            }),
                                                        metadata,
                                                    }
                                                })
                                                .collect()
                                        },
                                    )
                                })
                                .unwrap_or_default(),
                            metadata,
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

        if let Creator::Model(Some(model)) = &mut creator
            && let InnerNodeContent::Tokens(tokens) = &mut value.content
        {
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
                .and_then(|id| NonZeroU128::new(id.0))
            {
                model.identifier = Some(model_id);
            }

            model.seed = value
                .metadata
                .shift_remove("seed")
                .and_then(|id| id.parse::<u32>().ok());

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
            aux_metadata: AuxMetadataMap::default(),
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
            u64,
            BuildHasherDefault<UlidHasher>,
            BuildHasherDefault<RandomIdHasher>,
        > = UniqueIdentifierRemapper::with_capacity(identifiers.len());

        let time_zone = output.metadata().created.time_zone().clone();

        let mut rng = WyRand::new();

        let mut convert_old_identifier = |id| {
            *mapper
                .map_with_initial(
                    id,
                    unsafe { std::mem::transmute::<u128, [u64; 2]>(id)[1] },
                    || rng.generate(),
                )
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

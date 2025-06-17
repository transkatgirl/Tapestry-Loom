//! Data formats for storing Weaves.

use std::{
    collections::{HashMap, HashSet},
    io::{Cursor, Read, Write},
};

/* TODO:
- Conversion to/from Weave */

use base64::{engine::general_purpose::URL_SAFE, read::DecoderReader, write::EncoderStringWriter};
use lz4_flex::frame::{FrameDecoder, FrameEncoder};
use rmp_serde::{decode, encode};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use thiserror::Error;
use ulid::Ulid;

use crate::{
    content::FrozenWeave,
    document::{Weave, WeaveSnapshot, WeaveView},
};

/// A compact serializable format intended for storing `Weave` documents.
///
/// The `CompactWeave` format maintains backwards compatibility but not forwards compatibility. It is serialized as MessagePack compressed with LZ4.
#[derive(Serialize, Deserialize)]
pub struct CompactWeave {
    version: u64,

    // Sorted from lowest depth to highest depth
    nodes: Vec<(u128, Node)>,
    active: HashSet<u128>,
    bookmarked: HashSet<u128>,

    models: HashMap<u128, Model>,
}

#[derive(Serialize, Deserialize)]
struct Model {
    label: String,
    color: Option<String>,
}

#[derive(Serialize, Deserialize)]
enum NodeData {
    Text((String, Option<NodeModel>)),
    Token((NodeTokens, Option<NodeModel>)),
    TextToken((Vec<TextToken>, Option<NodeModel>)),
    Diff(NodeDiff),
    Blank,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum TextToken {
    Text(String),
    Token(NodeTokens),
}

impl NodeData {
    fn model(&self) -> Option<&NodeModel> {
        match self {
            NodeData::Text(content) => content.1.as_ref(),
            NodeData::Token(content) => content.1.as_ref(),
            NodeData::TextToken(content) => content.1.as_ref(),
            NodeData::Diff(_content) => None,
            NodeData::Blank => None,
        }
    }
}

// (data, parents)
type Node = (NodeData, Vec<u128>);
// (identifier, parameters)
type NodeModel = (u128, Vec<(String, String)>);
// [bytes, probability]
type NodeTokens = Vec<(ByteBuf, f32)>;
// [index, insert/delete, content] processed in specified order
type NodeDiff = Vec<(u64, bool, String)>;

#[derive(Error, Debug)]
/// An error encountered when working with a `CompactWeave`.
pub enum WeaveError {
    /// The `CompactWeave` could not be loaded from a reader.
    #[error(transparent)]
    Load(#[from] rmp_serde::decode::Error),
    /// The `CompactWeave` could not be saved.
    #[error(transparent)]
    Serialize(#[from] rmp_serde::encode::Error),
    /// The `CompactWeave` could not be saved to a writer.
    #[error(transparent)]
    Save(#[from] lz4_flex::frame::Error),
    /// The `CompactWeave` could be parsed but it's structure is malformed.
    #[error("invalid weave structure: {0}")]
    Structure(String),
    /// The `CompactWeave` has an unsupported version number.
    #[error("unsupported version number: {0}")]
    UnsupportedVersion(u64),
    /// The `CompactWeave` could not be converted into an `InteractiveWeave` document.
    #[error("unable to create InteractiveWeave: {0}")]
    FailedInteractive(String),
    /// The `InteractiveWeave` document could not be converted into a `CompactWeave`.
    #[error("unable to create CompactWeave: {0}")]
    FailedCompact(String),
}

impl CompactWeave {
    fn update(&mut self) -> Result<(), WeaveError> {
        if self.version > 0 {
            return Err(WeaveError::UnsupportedVersion(self.version));
        }

        Ok(())
    }
    /// Load a `CompactWeave` from a reader.
    pub fn load<R: Read>(reader: R) -> Result<Self, WeaveError> {
        let mut decompressor = FrameDecoder::new(reader);
        let mut weave: CompactWeave = decode::from_read(&mut decompressor)?;
        weave.update()?;
        Ok(weave)
    }
    /// Load a `CompactWeave` from a url-safe base64 string.
    pub fn load_base64(input: &str) -> Result<Self, WeaveError> {
        let mut cursor = Cursor::new(input);
        let mut decoder = DecoderReader::new(&mut cursor, &URL_SAFE);
        Self::load(&mut decoder)
    }
    /// Save a `CompactWeave` to a writer.
    pub fn save<W: Write>(&self, writer: W) -> Result<(), WeaveError> {
        let mut compressor = FrameEncoder::new(writer);
        encode::write_named(&mut compressor, self)?;
        compressor.finish()?;

        Ok(())
    }
    /// Save a `CompactWeave` to a url-safe base64 string.
    pub fn save_base64(&self) -> Result<String, WeaveError> {
        let mut encoder = EncoderStringWriter::new(&URL_SAFE);
        self.save(&mut encoder)?;
        Ok(encoder.into_inner())
    }
}

/// A wrapper around interactive representations of Weave documents.
pub enum InteractiveWeave {
    Plain(Weave),
    Frozen(FrozenWeave),
}

impl TryFrom<NodeData> for super::content::NodeContent {
    type Error = WeaveError;
    fn try_from(input: NodeData) -> Result<Self, Self::Error> {
        match input {
            NodeData::Text((content, model)) => Ok(super::content::NodeContent::Text(
                super::content::TextNode {
                    content,
                    model: model.map(|(identifier, parameters)| super::content::NodeModel {
                        id: Ulid(identifier),
                        parameters,
                    }),
                },
            )),
            NodeData::Token((content, model)) => Ok(super::content::NodeContent::Token(
                super::content::TokenNode {
                    content: content
                        .into_iter()
                        .map(|(bytes, probability)| {
                            Ok(super::content::NodeToken {
                                probability: Decimal::try_from(probability).map_err(|_| {
                                    WeaveError::FailedInteractive(
                                        "Unable to parse probability value".to_string(),
                                    )
                                })?,
                                content: bytes.into_vec(),
                            })
                        })
                        .collect::<Result<Vec<_>, WeaveError>>()?,
                    model: model.map(|(identifier, parameters)| super::content::NodeModel {
                        id: Ulid(identifier),
                        parameters,
                    }),
                },
            )),
            NodeData::TextToken((content, model)) => Ok(super::content::NodeContent::TextToken(
                super::content::TextTokenNode {
                    content: content
                        .into_iter()
                        .map(|text_token| match text_token {
                            TextToken::Text(text) => Ok(super::content::TextOrToken::Text(text)),
                            TextToken::Token(tokens) => Ok(super::content::TextOrToken::Token(
                                tokens
                                    .into_iter()
                                    .map(|(bytes, probability)| {
                                        Ok(super::content::NodeToken {
                                            probability: Decimal::try_from(probability).map_err(
                                                |_| {
                                                    WeaveError::FailedInteractive(
                                                        "Unable to parse probability value"
                                                            .to_string(),
                                                    )
                                                },
                                            )?,
                                            content: bytes.into_vec(),
                                        })
                                    })
                                    .collect::<Result<Vec<_>, WeaveError>>()?,
                            )),
                        })
                        .collect::<Result<Vec<_>, WeaveError>>()?,
                    model: model.map(|(identifier, parameters)| super::content::NodeModel {
                        id: Ulid(identifier),
                        parameters,
                    }),
                },
            )),
            NodeData::Diff(_diff) => Err(WeaveError::FailedInteractive(
                "Unsupported Node Content type".to_string(),
            )),
            NodeData::Blank => Ok(super::content::NodeContent::Blank),
        }
    }
}

impl TryFrom<super::content::NodeContent> for NodeData {
    type Error = WeaveError;
    fn try_from(input: super::content::NodeContent) -> Result<Self, Self::Error> {
        Ok(match input {
            super::content::NodeContent::Text(content) => NodeData::Text((
                content.content,
                content.model.map(|model| (model.id.0, model.parameters)),
            )),
            super::content::NodeContent::Token(content) => NodeData::Token((
                content
                    .content
                    .into_iter()
                    .map(|token| -> Result<(ByteBuf, f32), WeaveError> {
                        Ok((
                            ByteBuf::from(token.content),
                            f32::try_from(token.probability).map_err(|_| {
                                WeaveError::FailedCompact(
                                    "Unable to convert probability value to f32".to_string(),
                                )
                            })?,
                        ))
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                content.model.map(|model| (model.id.0, model.parameters)),
            )),
            super::content::NodeContent::TextToken(content) => NodeData::TextToken((
                content
                    .content
                    .into_iter()
                    .map(|token| match token {
                        super::content::TextOrToken::Text(text) => {
                            Ok::<TextToken, WeaveError>(TextToken::Text(text))
                        }
                        super::content::TextOrToken::Token(token) => Ok(TextToken::Token(
                            token
                                .into_iter()
                                .map(|token| -> Result<(ByteBuf, f32), WeaveError> {
                                    Ok((
                                        ByteBuf::from(token.content),
                                        f32::try_from(token.probability).map_err(|_| {
                                            WeaveError::FailedCompact(
                                                "Unable to convert probability value to f32"
                                                    .to_string(),
                                            )
                                        })?,
                                    ))
                                })
                                .collect::<Result<Vec<_>, _>>()?,
                        )),
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                content.model.map(|model| (model.id.0, model.parameters)),
            )),
            super::content::NodeContent::Blank => NodeData::Blank,
        })
    }
}

impl TryFrom<CompactWeave> for InteractiveWeave {
    type Error = WeaveError;
    fn try_from(input: CompactWeave) -> Result<Self, Self::Error> {
        let mut weave = Weave::default();

        let models: HashMap<u128, super::content::Model> = input
            .models
            .into_iter()
            .map(|(id, model)| {
                (
                    id,
                    super::content::Model {
                        id: Ulid(id),
                        label: model.label,
                        color: model.color,
                    },
                )
            })
            .collect();

        let mut tail_diff = None;
        for (identifier, (content, parents)) in input.nodes {
            if let NodeData::Diff(raw_diff) = &content {
                if tail_diff.is_some() {
                    return Err(WeaveError::FailedInteractive(
                        "Unsupported Node Content type".to_string(),
                    ));
                }
                let diff = super::content::Diff {
                    content: raw_diff
                        .clone()
                        .into_iter()
                        .map(|(index, insertion, content)| {
                            Ok(super::content::Modification {
                                index: index.try_into().map_err(|_| {
                                    WeaveError::FailedInteractive(
                                        "Unable to parse Diff index".to_string(),
                                    )
                                })?,
                                r#type: if insertion {
                                    super::content::ModificationType::Insertion
                                } else {
                                    super::content::ModificationType::Deletion
                                },
                                content,
                            })
                        })
                        .collect::<Result<Vec<_>, WeaveError>>()?,
                };

                tail_diff = Some(diff);
                continue;
            }

            let model = content.model().and_then(|m| models.get(&m.0));
            let node = super::content::Node {
                id: Ulid(identifier),
                from: parents.into_iter().map(Ulid).collect(),
                to: HashSet::new(),
                active: input.active.contains(&identifier),
                bookmarked: input.bookmarked.contains(&identifier),
                content: super::content::NodeContent::try_from(content)?,
            };

            weave.add_node(node, model.cloned(), true, false);
        }

        match tail_diff {
            Some(changes) => {
                if weave.get_active_timelines().len() == 1 {
                    FrozenWeave::new(weave, 0, changes)
                        .map(InteractiveWeave::Frozen)
                        .ok_or(WeaveError::FailedInteractive(
                            "Unable to find activated timeline".to_string(),
                        ))
                } else {
                    Err(WeaveError::FailedInteractive(
                        "Unable to find activated timeline".to_string(),
                    ))
                }
            }
            None => Ok(InteractiveWeave::Plain(weave)),
        }
    }
}

impl TryFrom<&InteractiveWeave> for CompactWeave {
    type Error = WeaveError;
    fn try_from(input: &InteractiveWeave) -> Result<Self, Self::Error> {
        let weave = match input {
            InteractiveWeave::Plain(weave) => WeaveSnapshot::from(weave),
            InteractiveWeave::Frozen(weave) => weave.weave(),
        };

        let mut active = HashSet::with_capacity(weave.nodes.len());
        let mut bookmarked = HashSet::with_capacity(weave.nodes.len());

        let mut nodes: Vec<(u128, Node)> = flatten_nodes(&weave)
            .iter()
            .inspect(|node| {
                if node.active {
                    active.insert(node.id.0);
                }
                if node.bookmarked {
                    bookmarked.insert(node.id.0);
                }
            })
            .map(|node| {
                Ok((
                    node.id.0,
                    (
                        NodeData::try_from(node.content.clone())?,
                        node.from.iter().map(|id| id.0).collect(),
                    ),
                ))
            })
            .collect::<Result<Vec<_>, WeaveError>>()?;

        let models: HashMap<u128, Model> = weave
            .models
            .clone()
            .into_values()
            .map(|model| {
                (
                    model.id.0,
                    (Model {
                        label: model.label,
                        color: model.color,
                    }),
                )
            })
            .collect();

        if let InteractiveWeave::Frozen(weave) = input {
            nodes.push((
                Ulid::new().0,
                (
                    NodeData::Diff(
                        weave
                            .diff()
                            .content
                            .clone()
                            .into_iter()
                            .map(|modification| {
                                Ok((
                                    u64::try_from(modification.index).map_err(|_| {
                                        WeaveError::FailedInteractive(
                                            "Unable to convert modification index to u64"
                                                .to_string(),
                                        )
                                    })?,
                                    modification.r#type
                                        == super::content::ModificationType::Insertion,
                                    modification.content,
                                ))
                            })
                            .collect::<Result<Vec<_>, WeaveError>>()?,
                    ),
                    Vec::new(),
                ),
            ));
        }

        Ok(CompactWeave {
            version: 0,
            nodes,
            active,
            bookmarked,
            models,
        })
    }
}

fn flatten_nodes<'a>(weave: &'a WeaveSnapshot) -> Vec<&'a super::content::Node> {
    let mut node_list = Vec::with_capacity(weave.nodes.len());
    let mut node_set = HashSet::with_capacity(weave.nodes.len());

    for node in weave.root_nodes {
        if let Some(node) = weave.nodes.get(node) {
            get_flattened_nodes(weave, node, &mut node_list, &mut node_set);
        }
    }

    node_list
}

fn get_flattened_nodes<'a>(
    weave: &'a WeaveSnapshot,
    node: &'a super::content::Node,
    list: &mut Vec<&'a super::content::Node>,
    identifiers: &mut HashSet<Ulid>,
) {
    if identifiers.insert(node.id) {
        for parent in &node.from {
            if let Some(parent) = weave.nodes.get(parent) {
                if !identifiers.contains(&parent.id) {
                    get_flattened_nodes(weave, parent, list, identifiers);
                }
            }
        }
        list.push(node);
        for child in &node.to {
            if let Some(child) = weave.nodes.get(child) {
                get_flattened_nodes(weave, child, list, identifiers);
            }
        }
    }
}

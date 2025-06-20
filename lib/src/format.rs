//! Data formats for storing Weave documents.

use std::{
    collections::{BTreeSet, HashMap, HashSet},
    io::{Cursor, Read, Write},
};

use base64::{engine::general_purpose::URL_SAFE, read::DecoderReader, write::EncoderStringWriter};
use lz4_flex::frame::{FrameDecoder, FrameEncoder};
use rmp_serde::{decode, encode};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use thiserror::Error;
use ulid::Ulid;

use crate::document::{OwnedWeaveSnapshot, Weave};

/// A compact serializable format intended for storing [`Weave`] documents.
///
/// The [`CompactWeave`] binary format maintains backwards compatibility but not forwards compatibility. It is serialized as MessagePack compressed with LZ4.
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
    Bytes((ByteBuf, Option<NodeModel>)),
    Token((NodeTokens, Option<NodeModel>)),
    TextToken((Vec<TextToken>, Option<NodeModel>)),
    Diff(NodeDiff),
    Blank,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum TextToken {
    Text(String),
    Bytes(ByteBuf),
    Token(NodeTokens),
}

impl NodeData {
    fn model(&self) -> Option<&NodeModel> {
        match self {
            NodeData::Text(content) => content.1.as_ref(),
            NodeData::Bytes(content) => content.1.as_ref(),
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
// [index, modification] processed in specified order
type NodeDiff = Vec<(u64, DiffModification)>;

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum DiffModification {
    Insert(String),
    Delete(u64),
}

#[derive(Error, Debug)]
/// An error encountered when working with a [`CompactWeave`].
pub enum WeaveError {
    /// The [`CompactWeave`] could not be loaded from a reader.
    #[error(transparent)]
    Load(#[from] rmp_serde::decode::Error),
    /// The [`CompactWeave`] could not be saved.
    #[error(transparent)]
    Serialize(#[from] rmp_serde::encode::Error),
    /// The [`CompactWeave`] could not be saved to a writer.
    #[error(transparent)]
    Save(#[from] lz4_flex::frame::Error),
    /// The [`CompactWeave`] could be parsed but it's structure is malformed.
    #[error("invalid weave structure: {0}")]
    Structure(String),
    /// The [`CompactWeave`] has an unsupported version number.
    #[error("unsupported version number: {0}")]
    UnsupportedVersion(u64),
    /// The [`CompactWeave`] could not be converted into an [`Weave`] document.
    #[error("unable to create Weave: {0}")]
    FailedInteractive(String),
    /// The [`Weave`] document could not be converted into a [`CompactWeave`].
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
    /// Load from a reader.
    pub fn load<R: Read>(reader: R) -> Result<Self, WeaveError> {
        let mut decompressor = FrameDecoder::new(reader);
        let mut weave: CompactWeave = decode::from_read(&mut decompressor)?;
        weave.update()?;
        Ok(weave)
    }
    /// Load from a url-safe base64 string.
    pub fn load_base64(input: &str) -> Result<Self, WeaveError> {
        let mut cursor = Cursor::new(input);
        let mut decoder = DecoderReader::new(&mut cursor, &URL_SAFE);
        Self::load(&mut decoder)
    }
    /// Save to a writer.
    pub fn save<W: Write>(&self, writer: W) -> Result<(), WeaveError> {
        let mut compressor = FrameEncoder::new(writer);
        encode::write_named(&mut compressor, self)?;
        compressor.finish()?;

        Ok(())
    }
    /// Save to a url-safe base64 string.
    pub fn save_base64(&self) -> Result<String, WeaveError> {
        let mut encoder = EncoderStringWriter::new(&URL_SAFE);
        self.save(&mut encoder)?;
        Ok(encoder.into_inner())
    }
}

impl TryFrom<DiffModification> for super::content::ModificationContent {
    type Error = WeaveError;
    fn try_from(input: DiffModification) -> Result<Self, Self::Error> {
        Ok(match input {
            DiffModification::Insert(content) => Self::Insertion(content),
            DiffModification::Delete(length) => {
                Self::Deletion(usize::try_from(length).map_err(|_| {
                    WeaveError::FailedInteractive(
                        "Unable to convert modification length to usize".to_string(),
                    )
                })?)
            }
        })
    }
}

impl TryFrom<super::content::ModificationContent> for DiffModification {
    type Error = WeaveError;
    fn try_from(input: super::content::ModificationContent) -> Result<Self, Self::Error> {
        Ok(match input {
            super::content::ModificationContent::Insertion(content) => Self::Insert(content),
            super::content::ModificationContent::Deletion(length) => {
                Self::Delete(u64::try_from(length).map_err(|_| {
                    WeaveError::FailedCompact(
                        "Unable to convert modification length to u64".to_string(),
                    )
                })?)
            }
        })
    }
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
            NodeData::Bytes((content, model)) => Ok(super::content::NodeContent::Bytes(
                super::content::ByteNode {
                    content: content.into_vec(),
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
                            TextToken::Bytes(bytes) => {
                                Ok(super::content::TextOrToken::Bytes(bytes.into_vec()))
                            }
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
            super::content::NodeContent::Bytes(content) => NodeData::Bytes((
                ByteBuf::from(content.content),
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
                        super::content::TextOrToken::Bytes(bytes) => {
                            Ok(TextToken::Bytes(ByteBuf::from(bytes)))
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

impl TryFrom<CompactWeave> for Weave {
    type Error = WeaveError;
    fn try_from(input: CompactWeave) -> Result<Self, Self::Error> {
        let mut weave = Weave::default();

        let mut models: HashMap<u128, super::content::Model> = input
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

        for (identifier, (content, parents)) in input.nodes {
            let model = content.model().and_then(|m| models.remove(&m.0));
            let node = super::content::Node {
                id: Ulid(identifier),
                from: parents.into_iter().map(Ulid).collect(),
                to: HashSet::new(),
                active: input.active.contains(&identifier),
                bookmarked: input.bookmarked.contains(&identifier),
                content: super::content::NodeContent::try_from(content)?,
            };

            if weave.add_node(node, model, true, false).is_none() {
                return Err(WeaveError::FailedInteractive(
                    "Unable to add Node to Weave".to_string(),
                ));
            }
        }

        Ok(weave)
    }
}

impl TryFrom<Weave> for CompactWeave {
    type Error = WeaveError;
    fn try_from(input: Weave) -> Result<Self, Self::Error> {
        let weave = OwnedWeaveSnapshot::from(input);

        let mut active = HashSet::with_capacity(weave.nodes.len());
        let mut bookmarked = HashSet::with_capacity(weave.nodes.len());

        let models: HashMap<u128, Model> = weave
            .models
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

        let nodes: Vec<(u128, Node)> = flatten_nodes(weave.root_nodes, weave.nodes)
            .into_iter()
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
                        NodeData::try_from(node.content)?,
                        node.from.iter().map(|id| id.0).collect(),
                    ),
                ))
            })
            .collect::<Result<Vec<_>, WeaveError>>()?;

        Ok(CompactWeave {
            version: 0,
            nodes,
            active,
            bookmarked,
            models,
        })
    }
}

fn flatten_nodes(
    root_nodes: BTreeSet<Ulid>,
    mut nodes: HashMap<Ulid, super::content::Node>,
) -> Vec<super::content::Node> {
    let mut node_list = Vec::with_capacity(nodes.len());

    for node in root_nodes {
        if let Some(node) = nodes.remove(&node) {
            get_flattened_nodes(&mut nodes, node, &mut node_list);
        }
    }

    node_list
}

fn get_flattened_nodes(
    nodes: &mut HashMap<Ulid, super::content::Node>,
    node: super::content::Node,
    list: &mut Vec<super::content::Node>,
) {
    for parent in &node.from {
        if let Some(parent) = nodes.remove(parent) {
            get_flattened_nodes(nodes, parent, list);
        }
    }
    let children = node.to.clone();
    list.push(node);
    for child in children {
        if let Some(child) = nodes.remove(&child) {
            get_flattened_nodes(nodes, child, list);
        }
    }
}

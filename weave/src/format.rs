//! Data formats for storing Weave documents.

use std::{
    collections::{HashMap, HashSet},
    io::{Read, Write},
};

use lz4_flex::frame::{FrameDecoder, FrameEncoder};
use rmp_serde::{decode, encode};
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use thiserror::Error;
use ulid::Ulid;

use crate::document::{OwnedWeaveSnapshot, Weave, content};

/// A compact serializable format intended for storing [`Weave`] documents.
///
/// The [`CompactWeave`] binary format maintains backwards compatibility but not forwards compatibility. It is serialized as MessagePack compressed with LZ4.
#[derive(Serialize, Deserialize)]
pub struct CompactWeave {
    #[serde(rename = "CompactWeave_version")]
    version: u64,

    // Sorted from lowest depth to highest depth
    nodes: Vec<(u128, Node)>,
    active: HashSet<u128>,
    bookmarked: HashSet<u128>,

    models: HashMap<u128, Model>,

    metadata: HashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
struct Model {
    label: String,
    metadata: HashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
enum NodeData {
    Snippet((ByteBuf, Option<NodeModel>, NodeMetadata)),
    Tokens((NodeTokens, Option<NodeModel>, NodeMetadata)),
    Diff((NodeDiff, Option<NodeModel>, NodeMetadata)),
    Blank,
}

// (data, parents)
type Node = (NodeData, Vec<u128>);
// (identifier, parameters)
type NodeModel = (u128, Vec<(String, String)>);
// [bytes, metadata]
type NodeTokens = Vec<(ByteBuf, NodeMetadata)>;
// [index, modification] processed in specified order
type NodeDiff = Vec<(u64, DiffModification)>;
type NodeMetadata = Option<HashMap<String, String>>;

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum DiffModification {
    Insert(ByteBuf),
    InsertToken(Vec<(ByteBuf, NodeMetadata)>),
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
    /// Load the document from a reader.
    pub fn load<R: Read>(reader: R) -> Result<Self, WeaveError> {
        let mut decompressor = FrameDecoder::new(reader);
        let mut weave: CompactWeave = decode::from_read(&mut decompressor)?;
        weave.update()?;
        Ok(weave)
    }
    /// Save the document to a writer.
    pub fn save<W: Write>(&self, writer: W) -> Result<(), WeaveError> {
        let mut compressor = FrameEncoder::new(writer);
        encode::write_named(&mut compressor, self)?;
        compressor.finish()?;

        Ok(())
    }
    /// Retrieve the metadata associated with the document.
    pub fn metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }
}

impl TryFrom<DiffModification> for content::ModificationContent {
    type Error = WeaveError;
    fn try_from(input: DiffModification) -> Result<Self, Self::Error> {
        Ok(match input {
            DiffModification::Insert(content) => {
                Self::Insertion(bytes::Bytes::from(content.into_vec()))
            }
            DiffModification::InsertToken(content) => Self::TokenInsertion(
                content
                    .into_iter()
                    .map(|(content, metadata)| content::ContentToken {
                        content: bytes::Bytes::from(content.into_vec()),
                        metadata,
                    })
                    .collect(),
            ),
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

impl TryFrom<content::ModificationContent> for DiffModification {
    type Error = WeaveError;
    fn try_from(input: content::ModificationContent) -> Result<Self, Self::Error> {
        Ok(match input {
            content::ModificationContent::Insertion(content) => {
                Self::Insert(ByteBuf::from(content))
            }
            content::ModificationContent::TokenInsertion(content) => Self::InsertToken(
                content
                    .into_iter()
                    .map(|token| (ByteBuf::from(token.content), token.metadata))
                    .collect(),
            ),
            content::ModificationContent::Deletion(length) => {
                Self::Delete(u64::try_from(length).map_err(|_| {
                    WeaveError::FailedCompact(
                        "Unable to convert modification length to u64".to_string(),
                    )
                })?)
            }
        })
    }
}

#[allow(clippy::too_many_lines)]
impl TryFrom<NodeData> for content::NodeContent {
    type Error = WeaveError;
    fn try_from(input: NodeData) -> Result<Self, Self::Error> {
        Ok(match input {
            NodeData::Snippet((content, model, metadata)) => {
                content::NodeContent::Snippet(content::SnippetContent {
                    content: bytes::Bytes::from(content.into_vec()),
                    model: model.map(|(identifier, parameters)| content::ContentModel {
                        id: Ulid(identifier),
                        parameters,
                    }),
                    metadata,
                })
            }
            NodeData::Tokens((content, model, metadata)) => {
                content::NodeContent::Tokens(content::TokenContent {
                    content: content
                        .into_iter()
                        .map(|(bytes, metadata)| {
                            Ok(content::ContentToken {
                                content: bytes::Bytes::from(bytes.into_vec()),
                                metadata,
                            })
                        })
                        .collect::<Result<Vec<_>, WeaveError>>()?,
                    model: model.map(|(identifier, parameters)| content::ContentModel {
                        id: Ulid(identifier),
                        parameters,
                    }),
                    metadata,
                })
            }
            NodeData::Diff((diff, model, metadata)) => {
                content::NodeContent::Diff(content::DiffContent {
                    content: content::Diff {
                        content: diff
                            .into_iter()
                            .map(|(index, content)| {
                                Ok(content::Modification {
                                    index: usize::try_from(index).map_err(|_| {
                                        WeaveError::FailedInteractive(
                                            "Unable to convert modification index to usize"
                                                .to_string(),
                                        )
                                    })?,
                                    content: content::ModificationContent::try_from(content)?,
                                })
                            })
                            .collect::<Result<Vec<_>, WeaveError>>()?,
                    },
                    model: model.map(|(identifier, parameters)| content::ContentModel {
                        id: Ulid(identifier),
                        parameters,
                    }),
                    metadata,
                })
            }
            NodeData::Blank => content::NodeContent::Blank,
        })
    }
}

impl TryFrom<content::NodeContent> for NodeData {
    type Error = WeaveError;
    fn try_from(input: content::NodeContent) -> Result<Self, Self::Error> {
        Ok(match input {
            content::NodeContent::Snippet(content) => NodeData::Snippet((
                ByteBuf::from(content.content),
                content.model.map(|model| (model.id.0, model.parameters)),
                content.metadata,
            )),
            content::NodeContent::Tokens(content) => NodeData::Tokens((
                content
                    .content
                    .into_iter()
                    .map(|token| -> Result<(ByteBuf, NodeMetadata), WeaveError> {
                        Ok((ByteBuf::from(token.content), token.metadata))
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                content.model.map(|model| (model.id.0, model.parameters)),
                content.metadata,
            )),
            content::NodeContent::Diff(content) => NodeData::Diff((
                content
                    .content
                    .content
                    .into_iter()
                    .map(|modification| {
                        Ok((
                            u64::try_from(modification.index).map_err(|_| {
                                WeaveError::FailedCompact(
                                    "Unable to convert modification index to u64".to_string(),
                                )
                            })?,
                            DiffModification::try_from(modification.content)?,
                        ))
                    })
                    .collect::<Result<Vec<_>, WeaveError>>()?,
                content.model.map(|model| (model.id.0, model.parameters)),
                content.metadata,
            )),
            content::NodeContent::Blank => NodeData::Blank,
        })
    }
}

impl TryFrom<CompactWeave> for Weave {
    type Error = WeaveError;
    fn try_from(input: CompactWeave) -> Result<Self, Self::Error> {
        let mut weave = Weave::default();

        weave.metadata = input.metadata;

        weave.reserve(input.nodes.len(), input.models.len());

        for (id, model) in input.models {
            weave.add_model(
                content::Model {
                    id: Ulid(id),
                    label: model.label,
                    metadata: model.metadata,
                },
                Some(input.nodes.len()),
            );
        }

        for (identifier, (content, parents)) in input.nodes {
            let node = content::Node {
                id: Ulid(identifier),
                from: parents.into_iter().map(Ulid).collect(),
                to: HashSet::new(),
                active: input.active.contains(&identifier),
                bookmarked: input.bookmarked.contains(&identifier),
                content: content::NodeContent::try_from(content)?,
            };

            if weave.add_node(node, None, false, false).is_none() {
                return Err(WeaveError::FailedInteractive(
                    "Unable to add Node to Weave".to_string(),
                ));
            }
        }

        weave.shrink_to_fit();

        Ok(weave)
    }
}

impl TryFrom<Weave> for CompactWeave {
    type Error = WeaveError;
    fn try_from(input: Weave) -> Result<Self, Self::Error> {
        let weave = OwnedWeaveSnapshot::from(input);

        let mut active = HashSet::with_capacity(weave.nodes.len());

        let models: HashMap<u128, Model> = weave
            .models
            .into_values()
            .map(|model| {
                (
                    model.id.0,
                    (Model {
                        label: model.label,
                        metadata: model.metadata,
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

        active.shrink_to_fit();

        Ok(CompactWeave {
            version: 0,
            nodes,
            active,
            bookmarked: weave.bookmarked_nodes.into_iter().map(|id| id.0).collect(),
            models,
            metadata: weave.metadata,
        })
    }
}

fn flatten_nodes(
    root_nodes: HashSet<Ulid>,
    mut nodes: HashMap<Ulid, content::Node>,
) -> Vec<content::Node> {
    let mut node_list = Vec::with_capacity(nodes.len());

    for node in root_nodes {
        if let Some(node) = nodes.remove(&node) {
            get_flattened_nodes(&mut nodes, node, &mut node_list);
        }
    }

    node_list
}

fn get_flattened_nodes(
    nodes: &mut HashMap<Ulid, content::Node>,
    node: content::Node,
    list: &mut Vec<content::Node>,
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

//! Data formats for storing Weaves.

use std::{
    collections::{HashMap, HashSet},
    io::{Cursor, Read, Write},
};

/* TODO:
- Conversion to/from Weave
- Unit tests */

use base64::{engine::general_purpose::URL_SAFE, read::DecoderReader, write::EncoderStringWriter};
use lz4_flex::frame::{FrameDecoder, FrameEncoder};
use rmp_serde::{decode, encode};
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use thiserror::Error;
use ulid::Ulid;

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
    #[error("unsupported weave structure: {0}")]
    FailedInteractive(String),
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
    Plain(super::Weave),
    Frozen(super::content::FrozenWeave),
}

impl TryFrom<CompactWeave> for InteractiveWeave {
    type Error = WeaveError;

    fn try_from(input: CompactWeave) -> Result<Self, Self::Error> {
        let mut weave = super::Weave::default();

        let models: HashMap<u128, super::Model> = input
            .models
            .into_iter()
            .map(|(id, model)| {
                (
                    id,
                    super::Model {
                        id: Ulid(id),
                        label: model.label,
                        color: model.color,
                    },
                )
            })
            .collect();

        for (identifier, node) in input.nodes {
            let model = node.0.model().and_then(|m| models.get(&m.0));
            /*let node_content = match node.0 {
                NodeData::Text(content) => {}
            };*/

            //weave.add_node(node, model, skip_loop_check)
        }

        /*let weave = Self::default();


        for (raw_identifier, value) in input.models {
            weave.models.get()
        }
        for (raw_identifier, value) in input.nodes {

        }*/

        todo!()
    }
}

impl From<InteractiveWeave> for CompactWeave {
    fn from(input: InteractiveWeave) -> Self {
        todo!()
    }
}

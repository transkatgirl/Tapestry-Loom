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

/// A compact serializable format intended for storing `Weave` documents.
///
/// The `CompactWeave` format maintains backwards compatibility but not forwards compatibility. It is serialized as MessagePack compressed with LZ4.

#[derive(Serialize, Deserialize)]
pub struct CompactWeave {
    version: u64,
    // Sorted from lowest depth to highest depth
    nodes: Vec<(u128, Node)>,
    active_nodes: HashSet<u128>,
    models: HashMap<u128, Model>,
}

#[derive(Serialize, Deserialize)]
struct Model {
    label: String,
    style: Option<String>,
}

#[derive(Serialize, Deserialize)]
enum NodeData {
    Text((String, Option<NodeModel>)),
    Token((NodeTokens, Option<NodeModel>)),
    TextToken((Vec<TextToken>, Option<NodeModel>)),
    Diff(NodeDiff),
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
pub enum WeaveError {
    /// An error encountered when loading a `CompactWeave`.
    #[error(transparent)]
    Load(#[from] rmp_serde::decode::Error),
    /// An error encountered when saving a `CompactWeave`.
    #[error(transparent)]
    Serialize(#[from] rmp_serde::encode::Error),
    /// An error encountered when saving a `CompactWeave`.
    #[error(transparent)]
    Save(#[from] lz4_flex::frame::Error),
    /// An error encountered when loading a malformed but parseable `CompactWeave`.
    #[error("invalid weave structure: {0}")]
    Structure(String),
    /// An error encountered when converting a `CompactWeave` into a `Weave` document.
    #[error("unsupported weave structure: {0}")]
    Unsupported(String),
}

impl CompactWeave {
    fn update(&mut self) -> Result<(), WeaveError> {
        if self.version > 0 {
            return Err(WeaveError::Structure(
                "version is greater than largest supported version (0)".to_string(),
            ));
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

impl TryFrom<CompactWeave> for super::Weave {
    type Error = WeaveError;

    fn try_from(input: CompactWeave) -> Result<Self, Self::Error> {
        for (identifier, node) in input.nodes {
            let model = node.0.model().and_then(|m| input.models.get(&m.0));
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

impl From<super::Weave> for CompactWeave {
    fn from(input: super::Weave) -> Self {
        todo!()
    }
}

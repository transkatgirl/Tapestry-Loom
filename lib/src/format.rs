//! A stable data format for storing Weaves as compactly as possible.

use std::{
    collections::{HashMap, HashSet},
    io::{/*Cursor,*/ Read, Write},
};

//use base64::{engine::general_purpose::STANDARD, read::DecoderReader, write::EncoderStringWriter};
use lz4_flex::frame::{FrameDecoder, FrameEncoder};
use rmp_serde::{decode, encode};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CompactWeave {
    version: u64,
    pub(crate) nodes: HashMap<u128, Node>,
    pub(crate) active_nodes: HashSet<u128>,
    pub(crate) models: HashMap<u128, Model>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Model {
    pub(crate) label: String,
    pub(crate) style: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub(crate) enum NodeData {
    Text((String, Option<NodeModel>)),
    Token((NodeTokens, Option<NodeModel>)),
    Diff(NodeDiff),
}

// (data, children, relative ordering)
pub(crate) type Node = (NodeData, Vec<u128>, i64);
// (identifier, parameters)
pub(crate) type NodeModel = (u128, HashMap<String, String>);
// [bytes, probability]
pub(crate) type NodeTokens = Vec<(Vec<u8>, f32)>;
// [index, insert/delete, content] processed in specified order
pub(crate) type NodeDiff = Vec<(u64, bool, String)>;

impl CompactWeave {
    fn update(&mut self) -> Result<(), String> {
        if self.version > 0 {
            return Err("Weave is not supported by current version".to_string());
        }

        Ok(())
    }
    pub fn load<R: Read>(reader: R) -> Result<Self, String> {
        let mut decompressor = FrameDecoder::new(reader);
        let mut weave: CompactWeave = decode::from_read(&mut decompressor)
            .map_err(|e| ["Weave parsing failed: ", &e.to_string()].concat())?;
        weave.update()?;
        Ok(weave)
    }
    pub fn save<W: Write>(&self, writer: W) {
        let mut compressor = FrameEncoder::new(writer);
        encode::write_named(&mut compressor, self).unwrap();
        compressor.finish().unwrap();
    }
    /*pub(crate) fn from_bytes(input: &[u8]) -> Result<Self, String> {
        Self::from_reader(input)
    }
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.to_writer(&mut buf);
        buf
    }
    pub(crate) fn from_base64_string(input: &str) -> Result<Self, String> {
        let mut cursor = Cursor::new(input);
        let mut decoder = DecoderReader::new(&mut cursor, &STANDARD);
        Self::from_reader(&mut decoder)
    }
    pub(crate) fn to_base64_string(&self) -> String {
        let mut encoder = EncoderStringWriter::new(&STANDARD);
        self.to_writer(&mut encoder);
        encoder.into_inner()
    }*/
}

use std::{
    collections::HashMap,
    io::{Cursor, Read, Write},
};

use base64::{engine::general_purpose::STANDARD, read::DecoderReader, write::EncoderStringWriter};
use lz4_flex::frame::{FrameDecoder, FrameEncoder};
use rmp_serde::{decode, encode};
use serde::{Deserialize, Serialize};

/// A stable data format for serializing and deserializing Weaves as compactly as possible.

#[derive(Serialize, Deserialize)]
pub(crate) struct Weave {
    version: u128,
    pub(crate) nodes: HashMap<u128, (Node, Vec<u128>)>,
    pub(crate) models: HashMap<u128, Model>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Model {
    pub(crate) label: String,
    pub(crate) style: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub(crate) enum Node {
    Text((String, Option<NodeModel>)),
    Token((NodeTokens, Option<NodeModel>)),
    Diff(String),
}

pub(crate) type NodeModel = (u128, Vec<(String, String)>);
pub(crate) type NodeTokens = Vec<(Vec<u8>, f32)>;

impl Weave {
    fn from_reader<R: Read>(reader: R) -> Result<Self, String> {
        let mut decompressor = FrameDecoder::new(reader);
        decode::from_read(&mut decompressor)
            .map_err(|e| ["Parsing failed: ", &e.to_string()].concat())
    }
    fn to_writer<W: Write>(&self, writer: W) {
        let mut compressor = FrameEncoder::new(writer);
        encode::write_named(&mut compressor, self).unwrap();
        compressor.finish().unwrap();
    }
    pub(crate) fn from_bytes(input: &[u8]) -> Result<Self, String> {
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
    }
}

use jiff::Zoned;
use nanorand::{Rng, WyRand};
use universal_weave::{Weave, indexmap::IndexSet};

use crate::{
    Creator, InnerNodeContent, NodeContent, TapestryNode, TapestryWeave,
    metadata::{AuxMetadataMap, ConvertedFrom, MetadataMap, WeaveMetadata},
};

impl TapestryWeave {
    pub fn from_plaintext(created: Zoned, bytes: Vec<u8>) -> Self {
        let mut output = Self::with_capacity_and_metadata(
            1,
            WeaveMetadata {
                title: None,
                description: None,
                created: created.clone(),
                converted_from: vec![ConvertedFrom::from_plaintext()],
                metadata: MetadataMap::default(),
                aux_metadata: AuxMetadataMap::default(),
            },
        );

        assert!(output.insert(TapestryNode {
            id: WyRand::new().generate(),
            from: IndexSet::default(),
            to: IndexSet::default(),
            active: true,
            bookmarked: false,
            contents: NodeContent {
                timestamp: created,
                modified: false,
                content: InnerNodeContent::Snippet(bytes),
                metadata: MetadataMap::default(),
                aux_metadata: AuxMetadataMap::default(),
                creator: Creator::User(None),
            },
        }));

        output
    }
}

use jiff::Zoned;
use nanorand::{Rng, WyRand};
use universal_weave::{Weave, indexmap::IndexSet};

use crate::{
    content::{Creator, InnerNodeContent, NodeContent},
    metadata::{AuxMetadataMap, ConvertedFrom, MetadataMap, WeaveMetadata},
    weave::{TapestryNode, TapestryWeave},
};

impl TapestryWeave {
    /// Converts a plain-text UTF-8 document into a [`TapestryWeave`].
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
                creator: Creator::User(None),
            },
        }));

        output
    }
}

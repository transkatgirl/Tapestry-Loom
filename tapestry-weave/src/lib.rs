//! An implementation of Tapestry Loom's document format.

use std::hash::{BuildHasher, Hash};

use nanorand::{RandomGen, Rng, WyRand};
use universal_weave::{BuildableNode, Weave, hashbrown::HashMap};

pub use foldhash;

pub use jiff;
pub use nanorand;
pub use universal_weave;

mod migrations;

pub mod content;
pub mod hashers;
pub mod metadata;
pub mod weave;
pub mod wrappers;

pub use content::{
    Author, CounterfactualToken, Creator, InnerNodeContent, InnerNodeToken, Model, NodeContent,
    OriginalToken, UNKNOWN_MODEL_LABEL,
};
pub use weave::{LongId, ShortId, TapestryNode, TapestryWeave};

pub const FILE_EXTENSION: &str = "tapestry";
pub const HEADER_MAGIC_BYTES: [u8; 24] = *b"VersionedTapestryWeave__";

/*

TODO:
- Diff-based tree updates
- Editor undo/redo

Ideas for future formats:
- Longest common prefix deduplication
- Event-based invalidation support for multi-user weaves
- Bookmark labels
- Metadata storage for LLM-as-weaver

See also: https://github.com/transkatgirl/Tapestry-Loom/blob/a232fbbb4119a8a9047ca67a8f1b0cfb772c5bb1/weave/src/document/content/mod.rs

*/

pub fn generate_id<W, K, N, T, S>(rng: &mut WyRand, weave: &W) -> K
where
    W: Weave<K, N, T, Nodes = HashMap<K, N, S>>,
    K: RandomGen<WyRand, 8> + Hash + Copy + Eq + Ord,
    N: BuildableNode<K, T>,
    S: BuildHasher + Default + Clone,
{
    let mut id = rng.generate();

    while weave.contains(&id) {
        id = rng.generate();
    }

    id
}

pub fn generate_ids<W, K, N, T, S, const COUNT: usize>(rng: &mut WyRand, weave: &W) -> [K; COUNT]
where
    W: Weave<K, N, T, Nodes = HashMap<K, N, S>>,
    K: RandomGen<WyRand, 8> + Hash + Copy + Eq + Ord + Default,
    N: BuildableNode<K, T>,
    S: BuildHasher + Default + Clone,
{
    let mut output = [K::default(); COUNT];

    for index in 0..COUNT {
        let mut id = rng.generate();

        while weave.contains(&id) || output.contains(&id) {
            id = rng.generate();
        }

        output[index] = id;
    }

    output
}

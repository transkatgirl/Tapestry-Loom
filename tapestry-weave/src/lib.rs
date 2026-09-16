//! An implementation of Tapestry Loom's document format.

//#![warn(missing_docs)]

use std::hash::{BuildHasher, Hash};

use nanorand::{RandomGen, Rng, WyRand};
use universal_weave::{BuildableNode, Weave, hashbrown::HashMap};

pub use foldhash;
pub use jiff;
pub use nanorand;
pub use similar;
pub use unicode_segmentation;
pub use universal_weave;

mod migrations;

pub mod content;
pub mod metadata;
pub mod util;
pub mod weave;

pub use content::{
    Author, CounterfactualToken, Creator, InnerNodeContent, InnerNodeToken, Model, NodeContent,
    OriginalToken, UNKNOWN_MODEL_LABEL,
};
pub use metadata::{MetadataMap, WeaveMetadata};
pub use weave::{LongId, ShortId, TapestryNode, TapestryWeave};

/*

TODO:

- Content module documentation
- Wrappers
    - TapestryWeave trait??
    - LoggedTapestryWeave
        - Bounded editor undo/redo
        - Modification serialization and deserialization
        - Modification application
        - Rollback
    - Node-specific AuxMetadata

Ideas for future formats:
- Longest common prefix deduplication
- Bookmark labels
- Metadata storage for LLM-as-weaver

See also: https://github.com/transkatgirl/Tapestry-Loom/blob/a232fbbb4119a8a9047ca67a8f1b0cfb772c5bb1/weave/src/document/content/mod.rs

*/

/// Generates a random identifier using `rng` which is guaranteed to not be present in `weave`.
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

/// Generates a random identifier using `rng` which is guaranteed to not be present in `weave` or `generated`.
pub fn generate_unique_id<W, K, N, T, S>(rng: &mut WyRand, weave: &W, generated: &[K]) -> K
where
    W: Weave<K, N, T, Nodes = HashMap<K, N, S>>,
    K: RandomGen<WyRand, 8> + Hash + Copy + Eq + Ord,
    N: BuildableNode<K, T>,
    S: BuildHasher + Default + Clone,
{
    let mut id = rng.generate();

    while weave.contains(&id) || generated.contains(&id) {
        id = rng.generate();
    }

    id
}

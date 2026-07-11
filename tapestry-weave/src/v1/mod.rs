use std::{
    collections::HashMap,
    hash::{BuildHasher, Hash},
};

use nanorand::{RandomGen, Rng, WyRand};
use universal_weave::{Node, Weave};

pub mod content;
pub mod dependent;
pub mod independent;
pub mod metadata;
pub mod treeless;

/*

TODO:
- Diff-based tree updates (see dependent.rs and independent.rs)

Ideas for future formats:
- Longest common prefix deduplication
- Support for editor undo/redo
- Event-based invalidation support for multi-user weaves
- Bookmark labels
- Add support for temporary nodes which are not actually stored in the IndependentWeave?
- Add support for exploring alternate paths in ArchivedWeave
- Metadata storage for LLM-as-weaver

See also: https://github.com/transkatgirl/Tapestry-Loom/blob/a232fbbb4119a8a9047ca67a8f1b0cfb772c5bb1/weave/src/document/content/mod.rs

*/

pub fn generate_unique_id<W, K, N, T, S>(rng: &mut WyRand, weave: &W) -> K
where
    W: Weave<K, N, T, Nodes = HashMap<K, N, S>>,
    K: RandomGen<WyRand, 8> + Hash + Copy + Eq,
    N: Node<K, T>,
    S: BuildHasher + Default + Clone,
{
    let mut id = rng.generate();

    while weave.contains(&id) {
        id = rng.generate();
    }

    id
}

pub fn generate_unique_id_with_list<W, K, N, T, S>(rng: &mut WyRand, weave: &W, ids: &[K]) -> K
where
    W: Weave<K, N, T, Nodes = HashMap<K, N, S>>,
    K: RandomGen<WyRand, 8> + Hash + Copy + Eq,
    N: Node<K, T>,
    S: BuildHasher + Default + Clone,
{
    let mut id = rng.generate();

    while weave.contains(&id) || ids.contains(&id) {
        id = rng.generate();
    }

    id
}

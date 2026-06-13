pub mod content;
pub mod dependent;
pub mod independent;
pub mod metadata;
pub mod treeless;

// TODO:
// - Diff-based tree updates
// - Prefix-based deduplication?
// - Support for editor undo/redo
// - Event-based invalidation support for multi-user weaves
// - Bookmark labels
// - Longest common prefix deduplication
// - Token ID based deduplication
// - Request parameter based deduplication (especially for single-token nodes)
// - Add support for temporary nodes which are not actually stored in the IndependentWeave?

// Useful reference for future formats: https://github.com/transkatgirl/Tapestry-Loom/blob/a232fbbb4119a8a9047ca67a8f1b0cfb772c5bb1/weave/src/document/content/mod.rs

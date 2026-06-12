pub mod content;
pub mod dependent;
pub mod independent;
pub mod metadata;

// TODO:
// - Implement v1 format based on IndependentWeave
//   - Implement diff-based tree updates
//   - Implement prefix-based deduplication?
//   - Implement support for editor undo/redo
//   - Implement event-based invalidation support for multi-user weaves
//   - Bookmark labels

// Useful reference for future formats: https://github.com/transkatgirl/Tapestry-Loom/blob/a232fbbb4119a8a9047ca67a8f1b0cfb772c5bb1/weave/src/document/content/mod.rs

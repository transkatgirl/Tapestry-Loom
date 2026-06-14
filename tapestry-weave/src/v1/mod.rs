pub mod content;
pub mod dependent;
pub mod independent;
pub mod metadata;
pub mod treeless;

/*

TODO:
- Fuzzy deduplication (see content.rs line 511)
- Diff-based tree updates

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

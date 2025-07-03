#![warn(clippy::pedantic)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::doc_markdown)]
#![warn(missing_docs)]

//! An implementation of Tapestry Loom's "Weave" document format.
//!
//! This library implements an interactive in-memory data structure ([`Weave`]) for working with Weave documents, along with a stable binary format for storing Weave documents ([`CompactWeave`]).

pub mod document;
pub mod format;

#[allow(unused_imports)]
use document::Weave;
#[allow(unused_imports)]
use format::CompactWeave;

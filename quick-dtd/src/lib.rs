//! High performant Document Type Definition (DTD) parser.
//!
//! # Features
//!
//! `quick-dtd` supports the following features:
#![cfg_attr(
    feature = "document-features",
    cfg_attr(doc, doc = ::document_features::document_features!(
        // Replicates the default format, but adds an anchor to the feature
        feature_label = "<a id=\"{feature}\" href=\"#{feature}\"><strong><code>{feature}</code></strong></a>"
    ))
)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
// Enable feature requirements in the docs from 1.57
// See https://stackoverflow.com/questions/61417452
#![cfg_attr(docs_rs, feature(doc_auto_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]

mod dtd;
// Helper reusable parsers
mod comment;
mod pi;
mod quoted;

pub use comment::CommentParser;
pub use dtd::{DtdIter, DtdParser, FeedResult};
pub use pi::PiParser;
pub use quoted::{QuotedParser, OneOf};

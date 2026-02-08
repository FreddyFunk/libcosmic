//! Content-type specific loaders for file preview.
//!
//! Most loaders have been moved to individual viewer crates:
//! - Images/SVG: cosmic-view-image
//! - Text: cosmic-view-text
//! - Directory: cosmic-view-directory
//! - Fallback: cosmic-view-fallback
//!
//! PDF loading remains here until a cosmic-view-pdf crate is created.

pub mod pdf;

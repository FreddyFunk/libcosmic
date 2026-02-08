//! Formatting utilities for file sizes and dates.
//!
//! Re-exports formatting functions from `cosmic_view_types`.

// These re-exports are part of the public API even if not used locally
#[allow(unused_imports)]
pub use cosmic_view_types::{format_file_size, format_modified};

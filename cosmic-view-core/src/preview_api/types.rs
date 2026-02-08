//! Core types for the preview API.
//!
//! This module re-exports types from `cosmic_view_types` and provides
//! the `From<FileCategory>` conversion for `PreviewKind`.

use crate::util::mime::FileCategory;

// Re-export all types from cosmic-view-types
pub use cosmic_view_types::{
    ActionId, ActionState, DetailItem, DetailSection, PreviewAction, PreviewConfig,
    PreviewDetails, PreviewKind, PreviewMessage, ThumbnailRenderConfig,
};

// ============================================================================
// Conversions (kept here as they depend on internal FileCategory)
// ============================================================================

impl From<FileCategory> for PreviewKind {
    fn from(category: FileCategory) -> Self {
        match category {
            FileCategory::Image => PreviewKind::Image,
            FileCategory::Svg => PreviewKind::Svg,
            FileCategory::Text => PreviewKind::Text,
            FileCategory::Directory => PreviewKind::Directory,
            FileCategory::Unknown => PreviewKind::Fallback,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preview_kind_from_file_category() {
        assert_eq!(PreviewKind::from(FileCategory::Image), PreviewKind::Image);
        assert_eq!(PreviewKind::from(FileCategory::Svg), PreviewKind::Svg);
        assert_eq!(PreviewKind::from(FileCategory::Text), PreviewKind::Text);
        assert_eq!(PreviewKind::from(FileCategory::Directory), PreviewKind::Directory);
        assert_eq!(PreviewKind::from(FileCategory::Unknown), PreviewKind::Fallback);
    }

    #[test]
    fn test_preview_config_default() {
        let config = PreviewConfig::default();
        assert_eq!(config.background_alpha, 1.0);
        assert_eq!(config.max_memory_mb, 2000);
        assert_eq!(config.max_file_size, 100 * 1024 * 1024);
        assert!(config.themed);
        assert_eq!(config.max_preview_dimension, 4096);
    }

    #[test]
    fn test_thumbnail_render_config_default() {
        let config = ThumbnailRenderConfig::default();
        assert_eq!(config.min_size, 128);
        assert_eq!(config.max_size, 256);
        assert!(!config.themed);
        assert_eq!(config.max_file_size, 100 * 1024 * 1024);
    }
}

//! Core types for the preview API.
//!
//! This module contains all the type definitions used by the preview system:
//! content kinds, actions, details, messages, and configuration.

use std::path::PathBuf;

use crate::util::mime::FileCategory;

// ============================================================================
// Content Kind
// ============================================================================

/// Content type being previewed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PreviewKind {
    /// Raster image (JPEG, PNG, WebP, GIF, BMP, TIFF, etc.)
    Image,
    /// Vector graphics (SVG)
    Svg,
    /// Text/code file
    Text,
    /// PDF document
    Pdf,
    /// 3D model (glTF, FBX, OBJ, STL, etc.)
    Model3D,
    /// Directory
    Directory,
    /// Unsupported file type (shows icon + metadata)
    Fallback,
}

impl From<FileCategory> for PreviewKind {
    fn from(category: FileCategory) -> Self {
        match category {
            FileCategory::Image => PreviewKind::Image,
            FileCategory::Svg => PreviewKind::Svg,
            FileCategory::Text => PreviewKind::Text,
            FileCategory::Pdf => PreviewKind::Pdf,
            FileCategory::Model3D => PreviewKind::Model3D,
            FileCategory::Directory => PreviewKind::Directory,
            FileCategory::Unknown => PreviewKind::Fallback,
        }
    }
}

// ============================================================================
// Actions
// ============================================================================

/// Unique identifier for a preview action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionId {
    // Universal zoom actions (for 2D content)
    /// Zoom in by one step
    ZoomIn,
    /// Zoom out by one step
    ZoomOut,
    /// Reset zoom to 100%
    ZoomReset,

    // 2D content actions
    /// Reset zoom to fit content in viewport
    FitPage,

    // 3D model actions
    /// Toggle texture rendering
    ToggleTextures,
    /// Toggle solid mesh rendering
    ToggleMesh,
    /// Toggle wireframe overlay
    ToggleWireframe,
    /// Reset camera to default view
    ResetCamera,
}

/// Current state of an action.
#[derive(Debug, Clone)]
pub enum ActionState {
    /// Simple trigger action (e.g., zoom in, reset)
    Trigger,
    /// Toggle action with on/off state
    Toggle {
        /// Whether the toggle is currently enabled
        enabled: bool,
    },
    /// Action with a displayable value
    Value {
        /// Current value as a string (e.g., "150%")
        current: String,
    },
}

/// A preview action that can be shown in menus or toolbars.
#[derive(Debug, Clone)]
pub struct PreviewAction {
    /// Unique identifier for this action
    pub id: ActionId,
    /// Display label for the action
    pub label: String,
    /// Keyboard shortcut hint (e.g., "+", "-", "W")
    pub shortcut: Option<String>,
    /// Current state of the action
    pub state: ActionState,
    /// Icon name (freedesktop icon naming)
    pub icon: Option<String>,
}

// ============================================================================
// Details
// ============================================================================

/// A single detail item for display in an info drawer.
#[derive(Debug, Clone)]
pub struct DetailItem {
    /// Label for the detail (e.g., "Format", "Dimensions")
    pub label: String,
    /// Value of the detail (e.g., "JPEG", "1920 x 1080")
    pub value: String,
}

/// A section of related details.
#[derive(Debug, Clone)]
pub struct DetailSection {
    /// Section title (e.g., "File Info", "Geometry")
    pub title: String,
    /// Items in this section
    pub items: Vec<DetailItem>,
}

/// All details about the current preview.
#[derive(Debug, Clone)]
pub struct PreviewDetails {
    /// The kind of content being previewed
    pub kind: PreviewKind,
    /// Sections of details to display
    pub sections: Vec<DetailSection>,
}

// ============================================================================
// Messages
// ============================================================================

/// Messages that can be emitted by the preview widget or triggered by apps.
#[derive(Debug, Clone)]
pub enum PreviewMessage {
    /// An action was triggered
    ActionTriggered(ActionId),
    /// Content finished loading successfully
    Loaded,
    /// Content failed to load
    LoadError(String),
    /// Request to open a file (e.g., from drag-and-drop within the preview)
    OpenFile(PathBuf),
}

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for creating a preview.
#[derive(Debug, Clone)]
pub struct PreviewConfig {
    /// Background alpha (0.0 = transparent, 1.0 = opaque)
    pub background_alpha: f32,
    /// Maximum memory for image decoding (MB)
    pub max_memory_mb: u64,
    /// Maximum file size to attempt loading (bytes)
    pub max_file_size: u64,
    /// Whether to apply theme colors (e.g., for 3D model backgrounds)
    /// Default is true. Set to false for generating theme-independent thumbnails.
    pub themed: bool,
    /// Maximum dimension (width or height) for preview images.
    /// Large images will be decoded at reduced resolution using DCT scaling for JPEG
    /// or post-decode downscaling for other formats. Set to 0 to disable scaling.
    /// Default is 4096, suitable for 4K displays.
    pub max_preview_dimension: u32,
}

impl Default for PreviewConfig {
    fn default() -> Self {
        Self {
            background_alpha: 1.0,
            max_memory_mb: 2000,
            max_file_size: 100 * 1024 * 1024, // 100 MB
            themed: true,
            max_preview_dimension: 4096, // Suitable for 4K displays
        }
    }
}

/// Configuration for rendering a thumbnail image.
#[derive(Debug, Clone)]
pub struct ThumbnailRenderConfig {
    /// Minimum dimension (width or height) for the output image.
    pub min_size: u32,
    /// Maximum dimension (width or height) for the output image.
    /// The actual output will maintain aspect ratio and fit within this bound.
    pub max_size: u32,
    /// Whether to apply theme colors (e.g., for 3D model backgrounds).
    /// Default is false for thumbnail generation to produce theme-independent images.
    pub themed: bool,
    /// Maximum file size to attempt loading (bytes)
    pub max_file_size: u64,
}

impl Default for ThumbnailRenderConfig {
    fn default() -> Self {
        Self {
            min_size: 128,
            max_size: 256,
            themed: false,
            max_file_size: 100 * 1024 * 1024, // 100 MB
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
        assert_eq!(PreviewKind::from(FileCategory::Pdf), PreviewKind::Pdf);
        assert_eq!(PreviewKind::from(FileCategory::Model3D), PreviewKind::Model3D);
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
        assert!(!config.themed); // Default is false for theme-independent thumbnails
        assert_eq!(config.max_file_size, 100 * 1024 * 1024);
    }
}

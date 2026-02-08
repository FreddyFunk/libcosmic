//! Core types for the preview library
//!
//! Most content-specific types have been moved to individual viewer crates:
//! - Image types: cosmic-view-image (ImageFormat, ImageInfo, ImageContent)
//! - Text types: cosmic-view-text (TextInfo, TextContent)
//! - Directory types: cosmic-view-directory (FolderInfo, DirectoryContent)
//! - Fallback types: cosmic-view-fallback (FallbackInfo, FallbackContent)
//!
//! This module now contains only the `LoadedContent` enum which wraps
//! content from all viewer crates.

use crate::loaders::pdf::PdfInfo;
#[cfg(feature = "view3d")]
use cosmic_view_3d::{ModelInfo, SceneData};

// Re-export shared types from cosmic-view-types
pub use cosmic_view_types::{ContentFit, ViewTransform};

// Re-export viewer crate types for backwards compatibility
// These may appear unused locally but are part of the public API
#[cfg(feature = "image")]
#[allow(unused_imports)]
pub use cosmic_view_image::{ImageContent, ImageFormat, ImageInfo};

#[cfg(feature = "text")]
#[allow(unused_imports)]
pub use cosmic_view_text::{TextContent, TextInfo};

#[cfg(feature = "directory")]
#[allow(unused_imports)]
pub use cosmic_view_directory::{DirectoryContent, FolderInfo};

#[cfg(feature = "fallback")]
#[allow(unused_imports)]
pub use cosmic_view_fallback::{FallbackContent, FallbackInfo};

/// State of loaded content (image, text, PDF, 3D model, etc.)
///
/// This enum wraps content from all viewer crates into a single type
/// for use by the Previewer API.
#[derive(Debug, Clone, Default)]
pub enum LoadedContent {
    /// No content loaded
    #[default]
    NotLoaded,
    /// Content is currently being loaded/decoded
    Loading,
    /// Raster image loaded successfully
    #[cfg(feature = "image")]
    Raster {
        handle: cosmic::widget::image::Handle,
        info: cosmic_view_image::ImageInfo,
    },
    /// SVG image loaded successfully
    #[cfg(feature = "image")]
    Svg {
        handle: cosmic::widget::svg::Handle,
        info: cosmic_view_image::ImageInfo,
    },
    /// Text file loaded with syntax highlighting
    #[cfg(feature = "text")]
    Text {
        content: cosmic_view_text::TextContent,
        info: cosmic_view_text::TextInfo,
    },
    /// PDF document loaded (all pages rendered)
    Pdf {
        /// Handles for all rendered pages
        pages: Vec<cosmic::widget::image::Handle>,
        info: PdfInfo,
    },
    /// 3D model loaded (requires view3d feature)
    #[cfg(feature = "view3d")]
    Model3D {
        /// Scene data containing meshes, materials, animations
        scene: Box<SceneData>,
        info: ModelInfo,
    },
    /// Fallback view for unsupported files (shows system thumbnail or icon)
    #[cfg(feature = "fallback")]
    Fallback {
        content: cosmic_view_fallback::FallbackContent,
        info: cosmic_view_fallback::FallbackInfo,
    },
    /// Folder/directory view
    #[cfg(feature = "directory")]
    Folder {
        content: cosmic_view_directory::DirectoryContent,
        info: cosmic_view_directory::FolderInfo,
    },
    /// Error loading content
    Error(String),
}

impl LoadedContent {
    /// Check if content is currently loaded (not loading or error)
    pub fn is_loaded(&self) -> bool {
        !matches!(self, Self::NotLoaded | Self::Loading | Self::Error(_))
    }

    /// Check if this is a folder
    #[cfg(feature = "directory")]
    pub fn is_folder(&self) -> bool {
        matches!(self, Self::Folder { .. })
    }

    /// Check if this is a folder (always false when directory feature disabled)
    #[cfg(not(feature = "directory"))]
    pub fn is_folder(&self) -> bool {
        false
    }

    /// Check if content is currently loading
    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading)
    }

    /// Check if this is a text file
    #[cfg(feature = "text")]
    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text { .. })
    }

    /// Check if this is a text file (always false when text feature disabled)
    #[cfg(not(feature = "text"))]
    pub fn is_text(&self) -> bool {
        false
    }

    /// Check if this is a PDF file
    pub fn is_pdf(&self) -> bool {
        matches!(self, Self::Pdf { .. })
    }

    /// Check if this is a 3D model
    #[cfg(feature = "view3d")]
    pub fn is_model3d(&self) -> bool {
        matches!(self, Self::Model3D { .. })
    }

    /// Check if this is a 3D model (always false when view3d feature disabled)
    #[cfg(not(feature = "view3d"))]
    pub fn is_model3d(&self) -> bool {
        false
    }

    /// Get image info if loaded (not for text or PDF files)
    #[cfg(feature = "image")]
    pub fn info(&self) -> Option<&cosmic_view_image::ImageInfo> {
        match self {
            Self::Raster { info, .. } | Self::Svg { info, .. } => Some(info),
            _ => None,
        }
    }

    /// Get image info if loaded (always None when image feature disabled)
    #[cfg(not(feature = "image"))]
    pub fn info(&self) -> Option<()> {
        None
    }

    /// Get text info if loaded
    #[cfg(feature = "text")]
    pub fn text_info(&self) -> Option<&cosmic_view_text::TextInfo> {
        match self {
            Self::Text { info, .. } => Some(info),
            _ => None,
        }
    }

    /// Get text info if loaded (always None when text feature disabled)
    #[cfg(not(feature = "text"))]
    pub fn text_info(&self) -> Option<()> {
        None
    }

    /// Get PDF info if loaded
    pub fn pdf_info(&self) -> Option<&PdfInfo> {
        match self {
            Self::Pdf { info, .. } => Some(info),
            _ => None,
        }
    }

    /// Get 3D model info if loaded
    #[cfg(feature = "view3d")]
    pub fn model_info(&self) -> Option<&ModelInfo> {
        match self {
            Self::Model3D { info, .. } => Some(info),
            _ => None,
        }
    }

    /// Get 3D model info if loaded (always None when view3d feature disabled)
    #[cfg(not(feature = "view3d"))]
    pub fn model_info(&self) -> Option<()> {
        None
    }

    /// Get 3D model scene data if loaded
    #[cfg(feature = "view3d")]
    pub fn model_scene(&self) -> Option<&SceneData> {
        match self {
            Self::Model3D { scene, .. } => Some(scene),
            _ => None,
        }
    }

    /// Get 3D model scene data if loaded (always None when view3d feature disabled)
    #[cfg(not(feature = "view3d"))]
    pub fn model_scene(&self) -> Option<()> {
        None
    }

    /// Get text content if loaded
    #[cfg(feature = "text")]
    pub fn text_content(&self) -> Option<&cosmic_view_text::TextContent> {
        match self {
            Self::Text { content, .. } => Some(content),
            _ => None,
        }
    }

    /// Get text content if loaded (always None when text feature disabled)
    #[cfg(not(feature = "text"))]
    pub fn text_content(&self) -> Option<()> {
        None
    }

    /// Get fallback info if loaded
    #[cfg(feature = "fallback")]
    pub fn fallback_info(&self) -> Option<&cosmic_view_fallback::FallbackInfo> {
        match self {
            Self::Fallback { info, .. } => Some(info),
            _ => None,
        }
    }

    /// Get fallback info if loaded (always None when fallback feature disabled)
    #[cfg(not(feature = "fallback"))]
    pub fn fallback_info(&self) -> Option<()> {
        None
    }

    /// Get folder info if loaded
    #[cfg(feature = "directory")]
    pub fn folder_info(&self) -> Option<&cosmic_view_directory::FolderInfo> {
        match self {
            Self::Folder { info, .. } => Some(info),
            _ => None,
        }
    }

    /// Get folder info if loaded (always None when directory feature disabled)
    #[cfg(not(feature = "directory"))]
    pub fn folder_info(&self) -> Option<()> {
        None
    }

    /// Get error message if in error state
    pub fn error(&self) -> Option<&str> {
        match self {
            Self::Error(msg) => Some(msg),
            _ => None,
        }
    }
}

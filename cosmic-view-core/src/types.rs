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

// Re-export shared types from cosmic-view-types
pub use cosmic_view_types::{ContentFit, ViewTransform};

/// State of loaded content (image, text, etc.)
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

    /// Get image info if loaded (not for text files)
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

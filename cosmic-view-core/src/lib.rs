// ============================================================================
// Internal Modules
// ============================================================================

mod loaders;
mod preview_api;
mod types;
mod util;
mod widgets;

// ============================================================================
// Re-exports from cosmic-view-types (foundation crate)
// ============================================================================

pub use cosmic_view_types::{
    // Shared types
    ContentFit, ViewTransform,
    // Preview kinds and config
    PreviewKind, PreviewConfig, ThumbnailRenderConfig,
    // Actions
    ActionId, ActionState, PreviewAction,
    // Details
    DetailItem, DetailSection, PreviewDetails,
    // Messages
    PreviewMessage,
    // Viewer trait and types
    ContentViewer, ViewerError, LoadConfig, ViewConfig,
    // Formatting utilities
    format_file_size, format_modified,
};

// ============================================================================
// Public API - Primary Interface
// ============================================================================

/// Unified Preview API - the primary interface for all preview operations.
///
/// The [`Previewer`] struct provides:
/// - [`Previewer::load`] - Load content asynchronously into a [`PreviewState`]
/// - [`Previewer::view`] - Get a widget for rendering
/// - [`Previewer::render_thumbnail`] - Generate raw pixels for thumbnail caching
/// - [`Previewer::actions`] - Get available actions
/// - [`Previewer::details`] - Get file metadata details
pub use preview_api::{PreviewState, Previewer};

// ============================================================================
// Public API - File Type Detection
// ============================================================================

/// MIME detection utilities for determining file types.
pub use util::mime::{detect_mime, get_mime_type, is_model_mime, FileCategory, MimeInfo};

// ============================================================================
// Public API - Content Types
// ============================================================================

/// Content-specific types used by LoadedContent and viewers.
pub use types::LoadedContent;

// Re-export types from viewer crates when features are enabled
#[cfg(feature = "image")]
pub use cosmic_view_image::{ImageContent, ImageFormat, ImageInfo};

#[cfg(feature = "text")]
pub use cosmic_view_text::{TextContent, TextInfo};

#[cfg(feature = "directory")]
pub use cosmic_view_directory::{DirectoryContent, FolderInfo};

#[cfg(feature = "fallback")]
pub use cosmic_view_fallback::{FallbackContent, FallbackInfo};

// Re-export viewer crates for direct access
#[cfg(feature = "image")]
pub use cosmic_view_image::{self as view_image, ImageViewer};

#[cfg(feature = "text")]
pub use cosmic_view_text::{self as view_text, TextViewer};

#[cfg(feature = "directory")]
pub use cosmic_view_directory::{self as view_directory, DirectoryViewer};

#[cfg(feature = "fallback")]
pub use cosmic_view_fallback::{self as view_fallback, FallbackViewer};

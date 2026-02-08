//! Core preview/image viewing library for COSMIC desktop applications
//!
//! This crate provides reusable components for file preview and display
//! that can be shared across COSMIC applications like cosmic-files and
//! the View image viewer.
//!
//! # API Overview
//!
//! The crate provides two main use cases:
//!
//! ## 1. Preview Widgets
//!
//! For interactive preview display in applications, use the [`Previewer`] API:
//!
//! ```rust,ignore
//! use cosmic_view_core::{Previewer, PreviewConfig};
//!
//! // Load a file asynchronously
//! let state = Previewer::load("/path/to/file.jpg", PreviewConfig::default()).await;
//!
//! // Get a widget for rendering
//! let widget = Previewer::view(&state, |msg| MyAppMessage::Preview(msg));
//!
//! // Get available actions and details
//! let actions = Previewer::actions(&state);
//! let details = Previewer::details(&state);
//! ```
//!
//! ## 2. Thumbnail Generation
//!
//! For generating cached thumbnail images (e.g., freedesktop thumbnail cache),
//! use [`Previewer::render_thumbnail`]:
//!
//! ```rust,ignore
//! use cosmic_view_core::{Previewer, ThumbnailRenderConfig};
//!
//! let config = ThumbnailRenderConfig {
//!     min_size: 128,
//!     max_size: 256,
//!     themed: false, // Theme-independent for caching
//!     ..Default::default()
//! };
//!
//! // Returns raw RGBA pixels
//! let (width, height, pixels) = Previewer::render_thumbnail(path, &config)?;
//! // Save to thumbnail cache...
//! ```
//!
//! # Supported Content Types
//!
//! - Images (PNG, JPEG, WebP, GIF, BMP, TIFF, JXL, etc.)
//! - SVG vector graphics
//! - PDF documents
//! - Text files (with syntax highlighting)
//! - 3D models (glTF, OBJ, STL, FBX, 3MF, etc.)
//! - Directories (folder view)
//! - Fallback (icon view for unsupported files)

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
/// - [`Previewer::actions`] / [`Previewer::details`] - Get available actions and file details
pub use preview_api::{PreviewState, Previewer};

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

/// PDF document information.
pub use loaders::pdf::PdfInfo;

// ============================================================================
// Public API - 3D Model Support (re-exported from view-3d crate)
// ============================================================================

/// 3D model types and functionality (requires view3d feature).
#[cfg(feature = "view3d")]
pub use cosmic_view_3d::{
    AnimationData, Model3DViewerConfig, ModelInfo, ModelPrimitive, ModelViewerState, SceneData,
    load_model, model_viewer, model_viewer_default, render_model_thumbnail,
};

// ============================================================================
// Public API - File Type Detection
// ============================================================================

/// MIME detection utilities for determining file types.
pub use util::mime::{detect_mime, get_mime_type, is_model_mime, FileCategory, MimeInfo};

//! Shared types and traits for COSMIC view components.
//!
//! This crate provides the foundation for the cosmic-view ecosystem:
//! - `ContentViewer` trait that all viewer crates implement
//! - Shared types like `ViewTransform`, `ContentFit`, `PreviewKind`
//! - Configuration types for loading and viewing
//! - Detail and action types for UI integration
//! - Formatting utilities

use std::path::Path;

use cosmic::Element;

// Re-export for convenience
pub use cosmic;

// ============================================================================
// Content Fit and View Transform
// ============================================================================

/// Content fit mode for image scaling
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ContentFit {
    /// Fit image within bounds, preserving aspect ratio (letterbox)
    #[default]
    Contain,
    /// Fill bounds completely, preserving aspect ratio (crop)
    Cover,
}

impl From<ContentFit> for cosmic::iced::ContentFit {
    fn from(fit: ContentFit) -> Self {
        match fit {
            ContentFit::Contain => cosmic::iced::ContentFit::Contain,
            ContentFit::Cover => cosmic::iced::ContentFit::Cover,
        }
    }
}

/// Transform state for zoom and pan
#[derive(Debug, Clone, Copy)]
pub struct ViewTransform {
    /// Zoom level (1.0 = 100%, min 0.1, max 10.0)
    pub zoom: f32,
    /// Horizontal pan offset in pixels
    pub offset_x: f32,
    /// Vertical pan offset in pixels
    pub offset_y: f32,
}

impl Default for ViewTransform {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }
}

impl ViewTransform {
    /// Minimum allowed zoom level
    pub const MIN_ZOOM: f32 = 0.1;
    /// Maximum allowed zoom level
    pub const MAX_ZOOM: f32 = 10.0;
    /// Zoom step for keyboard/button controls
    pub const ZOOM_STEP: f32 = 0.25;

    /// Create a new transform with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset transform to default (no zoom, no pan)
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Zoom in by one step
    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom + Self::ZOOM_STEP).min(Self::MAX_ZOOM);
    }

    /// Zoom out by one step
    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom - Self::ZOOM_STEP).max(Self::MIN_ZOOM);
    }

    /// Set zoom level, clamping to valid range
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
    }

    /// Apply scroll wheel zoom at a specific point
    pub fn scroll_zoom(&mut self, delta: f32, _cursor_x: f32, _cursor_y: f32) {
        let factor = if delta > 0.0 { 1.1 } else { 0.9 };
        self.zoom = (self.zoom * factor).clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
    }

    /// Pan by a delta amount
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        self.offset_x += delta_x;
        self.offset_y += delta_y;
    }

    /// Format zoom level for display (e.g., "150%")
    pub fn format_zoom(&self) -> String {
        format!("{}%", (self.zoom * 100.0).round() as i32)
    }
}

// ============================================================================
// Content Kind
// ============================================================================

/// Content type being previewed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PreviewKind {
    /// Raster image (JPEG, PNG, WebP, GIF, BMP, TIFF, etc.)
    Image,
    /// Vector graphics (SVG)
    Svg,
    /// Text/code file
    Text,
    /// Directory
    Directory,
    /// Unsupported file type (shows icon + metadata)
    #[default]
    Fallback,
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
// Viewer Trait Types
// ============================================================================

/// Error type for viewer operations.
#[derive(Debug, Clone)]
pub struct ViewerError(pub String);

impl std::fmt::Display for ViewerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ViewerError {}

impl From<String> for ViewerError {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ViewerError {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<std::io::Error> for ViewerError {
    fn from(e: std::io::Error) -> Self {
        Self(e.to_string())
    }
}

/// Configuration passed to loaders.
#[derive(Debug, Clone, Default)]
pub struct LoadConfig {
    /// Maximum file size to load (bytes). None = no limit.
    pub max_file_size: Option<u64>,
    /// Maximum dimension (width or height) for images. None = no limit.
    pub max_dimension: Option<u32>,
    /// Whether the UI is in dark mode (for syntax highlighting, etc.)
    pub is_dark_theme: bool,
}

/// Configuration passed to view builders.
#[derive(Debug, Clone)]
pub struct ViewConfig {
    /// Background alpha (0.0 = transparent, 1.0 = opaque)
    pub background_alpha: f32,
    /// Content fit mode
    pub content_fit: ContentFit,
}

impl Default for ViewConfig {
    fn default() -> Self {
        Self {
            background_alpha: 1.0,
            content_fit: ContentFit::Contain,
        }
    }
}

// ============================================================================
// ContentViewer Trait
// ============================================================================

/// Trait that all content viewers must implement.
///
/// Each viewer crate (cosmic-view-image, cosmic-view-text, etc.) implements
/// this trait to provide consistent loading, viewing, and interaction handling.
///
/// # Example
///
/// ```ignore
/// pub struct ImageViewer;
///
/// impl ContentViewer for ImageViewer {
///     const KIND: PreviewKind = PreviewKind::Image;
///     type Content = ImageContent;
///     type Info = ImageInfo;
///
///     fn can_handle(mime: &str) -> bool {
///         mime.starts_with("image/")
///     }
///
///     async fn load(path: &Path, config: &LoadConfig) -> Result<(Self::Content, Self::Info), ViewerError> {
///         // Load and decode image...
///     }
///
///     fn view<'a, M: Clone + 'static>(
///         content: &'a Self::Content,
///         info: &'a Self::Info,
///         transform: &ViewTransform,
///         config: &ViewConfig,
///     ) -> Element<'a, M> {
///         // Build widget...
///     }
///
///     fn details(info: &Self::Info) -> Vec<DetailSection> {
///         // Return file info...
///     }
///
///     fn actions() -> Vec<ActionId> {
///         vec![ActionId::ZoomIn, ActionId::ZoomOut, ActionId::ZoomReset]
///     }
/// }
/// ```
pub trait ContentViewer: Send + Sync {
    /// Unique identifier for this viewer.
    const KIND: PreviewKind;

    /// Associated content type (stored in LoadedContent).
    type Content: Clone + Send + 'static;

    /// Associated metadata type.
    type Info: Clone + Send + 'static;

    /// Check if this viewer can handle the given MIME type.
    fn can_handle(mime: &str) -> bool;

    /// Load content asynchronously from a file path.
    fn load(
        path: &Path,
        config: &LoadConfig,
    ) -> impl std::future::Future<Output = Result<(Self::Content, Self::Info), ViewerError>> + Send;

    /// Build the view widget for this content.
    fn view<'a, M: Clone + 'static>(
        content: &'a Self::Content,
        info: &'a Self::Info,
        transform: &ViewTransform,
        config: &ViewConfig,
    ) -> Element<'a, M>;

    /// Handle viewer-specific messages (default: no-op).
    fn update(_content: &mut Self::Content, _msg: PreviewMessage) {}

    /// Generate detail items for the info panel.
    fn details(info: &Self::Info) -> Vec<DetailSection>;

    /// Get available actions for this content type.
    fn actions() -> Vec<ActionId>;
}

// ============================================================================
// Formatting Utilities
// ============================================================================

/// Format bytes using binary units (1024-based): B, KB, MB, GB
pub fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Format timestamp for display (time only if today, full date otherwise)
pub fn format_modified(modified: std::time::SystemTime) -> String {
    let datetime = chrono::DateTime::<chrono::Local>::from(modified);
    let today = chrono::Local::now().date_naive();

    if datetime.date_naive() == today {
        datetime.format("%H:%M").to_string()
    } else {
        datetime.format("%b %d, %Y, %H:%M").to_string()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_transform_default() {
        let transform = ViewTransform::default();
        assert_eq!(transform.zoom, 1.0);
        assert_eq!(transform.offset_x, 0.0);
        assert_eq!(transform.offset_y, 0.0);
    }

    #[test]
    fn test_view_transform_zoom() {
        let mut transform = ViewTransform::default();
        transform.zoom_in();
        assert!((transform.zoom - 1.25).abs() < f32::EPSILON);
        transform.zoom_out();
        assert!((transform.zoom - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_view_transform_clamp() {
        let mut transform = ViewTransform::default();
        transform.set_zoom(100.0);
        assert_eq!(transform.zoom, ViewTransform::MAX_ZOOM);
        transform.set_zoom(0.01);
        assert_eq!(transform.zoom, ViewTransform::MIN_ZOOM);
    }

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(0), "0 B");
        assert_eq!(format_file_size(512), "512 B");
        assert_eq!(format_file_size(1024), "1.0 KB");
        assert_eq!(format_file_size(1024 * 1024), "1.00 MB");
        assert_eq!(format_file_size(1024 * 1024 * 1024), "1.00 GB");
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

    #[test]
    fn test_viewer_error_display() {
        let err = ViewerError("test error".to_string());
        assert_eq!(err.to_string(), "test error");
    }
}

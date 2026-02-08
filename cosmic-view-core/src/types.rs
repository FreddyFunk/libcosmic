//! Core types for the preview library

use crate::loaders::pdf::PdfInfo;
use cosmic_text::Buffer;
#[cfg(feature = "view3d")]
use cosmic_view_3d::{ModelInfo, SceneData};
use cosmic::iced::Color;
use std::path::PathBuf;
use std::sync::Arc;

/// Content fit mode for image scaling
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ContentFit {
    /// Fit image within bounds, preserving aspect ratio (letterbox)
    #[default]
    Contain,
    /// Fill bounds completely, preserving aspect ratio (crop)
    Cover,
}

/// Supported image formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageFormat {
    Jpeg,
    Png,
    WebP,
    Gif,
    Svg,
    Bmp,
    Tiff,
    Ico,
    #[default]
    Unknown,
}

impl ImageFormat {
    /// Get format from file extension
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "jpg" | "jpeg" => Self::Jpeg,
            "png" => Self::Png,
            "webp" => Self::WebP,
            "gif" => Self::Gif,
            "svg" => Self::Svg,
            "bmp" => Self::Bmp,
            "tif" | "tiff" => Self::Tiff,
            "ico" => Self::Ico,
            _ => Self::Unknown,
        }
    }

    /// Get format display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Jpeg => "JPEG",
            Self::Png => "PNG",
            Self::WebP => "WebP",
            Self::Gif => "GIF",
            Self::Svg => "SVG",
            Self::Bmp => "BMP",
            Self::Tiff => "TIFF",
            Self::Ico => "ICO",
            Self::Unknown => "Unknown",
        }
    }

    /// Check if format is a raster image (not vector)
    pub fn is_raster(&self) -> bool {
        matches!(self, Self::Jpeg | Self::Png | Self::WebP | Self::Gif | Self::Bmp | Self::Tiff | Self::Ico)
    }
}

/// Image metadata and information
#[derive(Debug, Clone)]
pub struct ImageInfo {
    /// Path to the image file
    pub path: PathBuf,
    /// Original image width in pixels (full resolution)
    pub width: u32,
    /// Original image height in pixels (full resolution)
    pub height: u32,
    /// Detected image format
    pub format: ImageFormat,
    /// File size in bytes
    pub file_size: u64,
    /// Whether the displayed image is a scaled preview (not full resolution).
    /// When true, a full-resolution version may be loaded in the background.
    pub is_preview: bool,
    /// Displayed width (may differ from original if scaled)
    pub displayed_width: u32,
    /// Displayed height (may differ from original if scaled)
    pub displayed_height: u32,
}

impl ImageInfo {
    /// Format file size for display (e.g., "1.5 MB")
    pub fn format_file_size(&self) -> String {
        crate::util::formatting::format_file_size(self.file_size)
    }

    /// Format dimensions for display (e.g., "1920 x 1080")
    pub fn format_dimensions(&self) -> String {
        format!("{} x {}", self.width, self.height)
    }
}

/// Text file metadata and information
#[derive(Debug, Clone)]
pub struct TextInfo {
    /// Path to the text file
    pub path: PathBuf,
    /// Detected syntax/language name
    pub syntax_name: String,
    /// Number of lines
    pub line_count: usize,
    /// File size in bytes
    pub file_size: u64,
}

impl TextInfo {
    /// Format file size for display (e.g., "1.5 MB")
    pub fn format_file_size(&self) -> String {
        crate::util::formatting::format_file_size(self.file_size)
    }

    /// Format line count for display
    pub fn format_line_count(&self) -> String {
        format!("{} lines", self.line_count)
    }
}

/// Fallback file info for unsupported files showing system icon
#[derive(Debug, Clone)]
pub struct FallbackInfo {
    /// Path to the original file
    pub path: PathBuf,
    /// File name for display
    pub filename: String,
    /// MIME type (if detected)
    pub mime_type: Option<String>,
    /// File size in bytes
    pub file_size: u64,
    /// Whether this is showing a cached thumbnail (true) or generic icon (false)
    pub is_thumbnail: bool,
    /// Last modified timestamp
    pub modified: Option<std::time::SystemTime>,
}

impl FallbackInfo {
    /// Format file size for display (e.g., "1.5 MB")
    pub fn format_file_size(&self) -> String {
        crate::util::formatting::format_file_size(self.file_size)
    }

    /// Format last modified time for display
    pub fn format_modified(&self) -> Option<String> {
        self.modified.map(crate::util::formatting::format_modified)
    }
}

/// Folder/directory info
#[derive(Debug, Clone)]
pub struct FolderInfo {
    /// Path to the directory
    pub path: PathBuf,
    /// Directory name for display
    pub filename: String,
    /// Last modified timestamp
    pub modified: Option<std::time::SystemTime>,
    /// Number of items in directory
    pub children_count: Option<usize>,
}

impl FolderInfo {
    /// Format last modified time for display
    pub fn format_modified(&self) -> Option<String> {
        self.modified.map(crate::util::formatting::format_modified)
    }

    /// Format children count (e.g., "5 items")
    pub fn format_children_count(&self) -> Option<String> {
        let count = self.children_count?;
        if count == 1 {
            Some("1 item".to_string())
        } else {
            Some(format!("{} items", count))
        }
    }
}

/// A styled text span for syntax highlighting
#[derive(Debug, Clone)]
pub struct StyledSpan {
    /// The text content
    pub text: String,
    /// The foreground color
    pub color: Color,
}

/// Highlighted text content (legacy - kept for compatibility)
#[derive(Debug, Clone)]
pub struct HighlightedText {
    /// Lines of styled spans
    pub lines: Vec<Vec<StyledSpan>>,
}

/// Syntax-highlighted text buffer for rendering
///
/// This wraps a cosmic-text Buffer with syntax highlighting applied.
/// The buffer is stored in an Arc for sharing with the widget.
#[derive(Clone)]
pub struct SyntaxBuffer {
    /// The cosmic-text buffer with highlighting
    pub buffer: Arc<Buffer>,
    /// Raw text content (for fallback or re-highlighting on theme change)
    pub content: String,
}

impl SyntaxBuffer {
    /// Create a new syntax buffer from a buffer and its source content.
    pub fn new(buffer: Buffer, content: String) -> Self {
        Self {
            buffer: Arc::new(buffer),
            content,
        }
    }

    /// Get a reference to the buffer Arc for rendering.
    pub fn buffer_ref(&self) -> &Arc<Buffer> {
        &self.buffer
    }
}

impl std::fmt::Debug for SyntaxBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyntaxBuffer")
            .field("content_len", &self.content.len())
            .finish_non_exhaustive()
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

/// State of loaded content (image, text, or PDF)
#[derive(Debug, Clone, Default)]
pub enum LoadedContent {
    /// No content loaded
    #[default]
    NotLoaded,
    /// Content is currently being loaded/decoded
    Loading,
    /// Raster image loaded successfully
    Raster {
        handle: cosmic::widget::image::Handle,
        info: ImageInfo,
    },
    /// SVG image loaded successfully
    Svg {
        handle: cosmic::widget::svg::Handle,
        info: ImageInfo,
    },
    /// Text file loaded with syntax highlighting
    Text {
        /// Syntax-highlighted buffer for rendering
        buffer: SyntaxBuffer,
        info: TextInfo,
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
    Fallback {
        /// Icon handle for the system icon
        icon_handle: cosmic::widget::icon::Handle,
        info: FallbackInfo,
    },
    /// Folder/directory view
    Folder {
        /// Icon handle for the folder icon
        icon_handle: cosmic::widget::icon::Handle,
        info: FolderInfo,
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
    pub fn is_folder(&self) -> bool {
        matches!(self, Self::Folder { .. })
    }

    /// Check if content is currently loading
    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading)
    }

    /// Check if this is a text file
    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text { .. })
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
    pub fn info(&self) -> Option<&ImageInfo> {
        match self {
            Self::Raster { info, .. } | Self::Svg { info, .. } => Some(info),
            _ => None,
        }
    }

    /// Get text info if loaded
    pub fn text_info(&self) -> Option<&TextInfo> {
        match self {
            Self::Text { info, .. } => Some(info),
            _ => None,
        }
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

    /// Get syntax buffer if loaded
    pub fn syntax_buffer(&self) -> Option<&SyntaxBuffer> {
        match self {
            Self::Text { buffer, .. } => Some(buffer),
            _ => None,
        }
    }

    /// Get fallback info if loaded
    pub fn fallback_info(&self) -> Option<&FallbackInfo> {
        match self {
            Self::Fallback { info, .. } => Some(info),
            _ => None,
        }
    }

    /// Get folder info if loaded
    pub fn folder_info(&self) -> Option<&FolderInfo> {
        match self {
            Self::Folder { info, .. } => Some(info),
            _ => None,
        }
    }

    /// Get error message if in error state
    pub fn error(&self) -> Option<&str> {
        match self {
            Self::Error(msg) => Some(msg),
            _ => None,
        }
    }
}

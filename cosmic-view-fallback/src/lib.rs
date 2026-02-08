//! Fallback file viewer for COSMIC desktop applications.
//!
//! This crate provides the [`FallbackViewer`] which implements the
//! [`ContentViewer`] trait for displaying files that don't have a
//! specialized viewer.
//!
//! # Example
//!
//! ```rust,ignore
//! use cosmic_view_fallback::{FallbackViewer, FallbackContent, FallbackInfo};
//! use cosmic_view_types::{ContentViewer, LoadConfig};
//!
//! let (content, info) = FallbackViewer::load(path, &LoadConfig::default()).await?;
//! let widget = FallbackViewer::view(&content, &info, &transform, &config);
//! ```

use std::path::{Path, PathBuf};

use cosmic::widget::{self, icon};
use cosmic::Element;
use cosmic_view_types::{
    ActionId, ContentViewer, DetailItem, DetailSection, LoadConfig, PreviewKind, PreviewMessage,
    ViewConfig, ViewTransform, ViewerError, format_file_size, format_modified,
};

/// Default icon size
const DEFAULT_ICON_SIZE: u16 = 128;

/// Fallback icon name for files without a specific icon
const FALLBACK_MIME_ICON: &str = "text-x-generic";

// ============================================================================
// Types
// ============================================================================

/// Fallback file info for unsupported files showing system icon.
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
        format_file_size(self.file_size)
    }

    /// Format last modified time for display
    pub fn format_modified(&self) -> Option<String> {
        self.modified.map(format_modified)
    }
}

/// Content data for a loaded fallback file.
#[derive(Clone)]
pub struct FallbackContent {
    /// Icon handle for the system icon
    pub icon_handle: icon::Handle,
}

impl std::fmt::Debug for FallbackContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FallbackContent").finish_non_exhaustive()
    }
}

// ============================================================================
// FallbackViewer Implementation
// ============================================================================

/// Viewer for unsupported files.
///
/// Displays a system icon based on MIME type with file name, size, and modified time.
pub struct FallbackViewer;

impl ContentViewer for FallbackViewer {
    const KIND: PreviewKind = PreviewKind::Fallback;

    type Content = FallbackContent;
    type Info = FallbackInfo;

    fn can_handle(_mime: &str) -> bool {
        // Fallback handles everything not claimed by other viewers
        true
    }

    async fn load(
        path: &Path,
        _config: &LoadConfig,
    ) -> Result<(Self::Content, Self::Info), ViewerError> {
        let path = path.to_path_buf();

        // Run filesystem operations in blocking task
        let result = tokio::task::spawn_blocking(move || load_fallback_sync(&path))
            .await
            .map_err(|e| ViewerError(format!("Task join error: {}", e)))?;

        result
    }

    fn view<'a, M: Clone + 'static>(
        content: &'a Self::Content,
        info: &'a Self::Info,
        _transform: &ViewTransform,
        _config: &ViewConfig,
    ) -> Element<'a, M> {
        let filename = info.filename.clone();

        let mut column = widget::column()
            .spacing(16)
            .align_x(cosmic::iced::Alignment::Center)
            .push(widget::icon(content.icon_handle.clone()).size(DEFAULT_ICON_SIZE))
            .push(widget::text::title4(filename));

        // Build details column for additional info
        let mut details = widget::column()
            .spacing(4)
            .align_x(cosmic::iced::Alignment::Center);

        // Show file size
        let file_size = info.format_file_size();
        if !file_size.is_empty() {
            details = details.push(widget::text::body(file_size));
        }

        // Show modified time
        if let Some(modified) = info.format_modified() {
            details = details.push(widget::text::body(modified));
        }

        column = column.push(details);

        widget::container(column)
            .width(cosmic::iced::Length::Fill)
            .height(cosmic::iced::Length::Fill)
            .align_x(cosmic::iced::alignment::Horizontal::Center)
            .align_y(cosmic::iced::alignment::Vertical::Center)
            .into()
    }

    fn update(_content: &mut Self::Content, _msg: PreviewMessage) {
        // Fallback files don't have interactive state
    }

    fn details(info: &Self::Info) -> Vec<DetailSection> {
        let mut items = vec![DetailItem {
            label: "Name".to_string(),
            value: info.filename.clone(),
        }];

        if let Some(mime) = &info.mime_type {
            items.push(DetailItem {
                label: "Type".to_string(),
                value: mime.clone(),
            });
        }

        if info.file_size > 0 {
            items.push(DetailItem {
                label: "Size".to_string(),
                value: info.format_file_size(),
            });
        }

        if let Some(modified) = info.format_modified() {
            items.push(DetailItem {
                label: "Modified".to_string(),
                value: modified,
            });
        }

        vec![DetailSection {
            title: "File Info".to_string(),
            items,
        }]
    }

    fn actions() -> Vec<ActionId> {
        // Fallback files don't have zoom/pan actions
        vec![]
    }
}

// ============================================================================
// Helper Functions (Linux)
// ============================================================================

/// Load fallback file information synchronously (Linux version).
#[cfg(target_os = "linux")]
fn load_fallback_sync(path: &Path) -> Result<(FallbackContent, FallbackInfo), ViewerError> {
    let metadata = std::fs::metadata(path)
        .map_err(|e| ViewerError(format!("Failed to read metadata: {}", e)))?;

    if metadata.is_dir() {
        return Err(ViewerError("Use DirectoryViewer for directories".to_string()));
    }

    let canonical = path
        .canonicalize()
        .map_err(|e| ViewerError(format!("Failed to canonicalize path: {}", e)))?;

    let filename = canonical
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let file_size = metadata.len();

    // Get last modified time
    let modified = metadata.modified().ok();

    // For files, use MIME type to get icon
    let shared_mime = xdg_mime::SharedMimeInfo::new();
    let guess = shared_mime
        .guess_mime_type()
        .path(&canonical)
        .metadata(metadata.clone())
        .guess();

    let mime = guess.mime_type();
    let mime_str = mime.to_string();

    // Look up icon names for this MIME type
    let mut icon_names = shared_mime.lookup_icon_names(mime);

    let icon_handle = if icon_names.is_empty() {
        // No icon found, use fallback
        icon::from_name(FALLBACK_MIME_ICON)
            .size(DEFAULT_ICON_SIZE)
            .handle()
    } else {
        // Use first icon name with fallbacks
        let icon_name = icon_names.remove(0);
        let mut named = icon::from_name(icon_name).size(DEFAULT_ICON_SIZE);

        if !icon_names.is_empty() {
            let fallback_names: Vec<std::borrow::Cow<'_, str>> =
                icon_names.into_iter().map(std::borrow::Cow::from).collect();
            named = named.fallback(Some(icon::IconFallback::Names(fallback_names)));
        }

        named.handle()
    };

    Ok((
        FallbackContent { icon_handle },
        FallbackInfo {
            path: canonical,
            filename,
            mime_type: Some(mime_str),
            file_size,
            is_thumbnail: false,
            modified,
        },
    ))
}

// ============================================================================
// Helper Functions (Non-Linux)
// ============================================================================

/// Load fallback file information synchronously (non-Linux version).
#[cfg(not(target_os = "linux"))]
fn load_fallback_sync(path: &Path) -> Result<(FallbackContent, FallbackInfo), ViewerError> {
    let metadata = std::fs::metadata(path)
        .map_err(|e| ViewerError(format!("Failed to read metadata: {}", e)))?;

    if metadata.is_dir() {
        return Err(ViewerError("Use DirectoryViewer for directories".to_string()));
    }

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();
    let file_size = metadata.len();

    // Get last modified time
    let modified = metadata.modified().ok();

    // Use generic icon
    let icon_handle = icon::from_name(FALLBACK_MIME_ICON)
        .size(DEFAULT_ICON_SIZE)
        .handle();

    Ok((
        FallbackContent { icon_handle },
        FallbackInfo {
            path: path.to_path_buf(),
            filename,
            mime_type: None,
            file_size,
            is_thumbnail: false,
            modified,
        },
    ))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_info_format_file_size() {
        let info = FallbackInfo {
            path: PathBuf::from("/tmp/test.bin"),
            filename: "test.bin".to_string(),
            mime_type: Some("application/octet-stream".to_string()),
            file_size: 1024 * 1024, // 1 MB
            is_thumbnail: false,
            modified: None,
        };
        assert_eq!(info.format_file_size(), "1.00 MB");
    }

    #[test]
    fn test_can_handle() {
        // Fallback handles everything
        assert!(FallbackViewer::can_handle("application/octet-stream"));
        assert!(FallbackViewer::can_handle("unknown/type"));
    }
}

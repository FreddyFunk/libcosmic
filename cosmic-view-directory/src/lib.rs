//! Directory/folder viewer for COSMIC desktop applications.
//!
//! This crate provides the [`DirectoryViewer`] which implements the
//! [`ContentViewer`] trait for displaying directory information.
//!
//! # Example
//!
//! ```rust,ignore
//! use cosmic_view_directory::{DirectoryViewer, DirectoryContent, FolderInfo};
//! use cosmic_view_types::{ContentViewer, LoadConfig};
//!
//! let (content, info) = DirectoryViewer::load(path, &LoadConfig::default()).await?;
//! let widget = DirectoryViewer::view(&content, &info, &transform, &config);
//! ```

use std::path::{Path, PathBuf};

use cosmic::widget::{self, icon};
use cosmic::Element;
use cosmic_view_types::{
    ActionId, ContentViewer, DetailItem, DetailSection, LoadConfig, PreviewKind, PreviewMessage,
    ViewConfig, ViewTransform, ViewerError, format_modified,
};

/// Default icon size for folder display
const DEFAULT_ICON_SIZE: u16 = 128;

// ============================================================================
// Types
// ============================================================================

/// Folder/directory metadata and information.
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
        self.modified.map(format_modified)
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

/// Content data for a loaded directory.
#[derive(Clone)]
pub struct DirectoryContent {
    /// Icon handle for the folder icon
    pub icon_handle: icon::Handle,
}

impl std::fmt::Debug for DirectoryContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DirectoryContent").finish_non_exhaustive()
    }
}

// ============================================================================
// DirectoryViewer Implementation
// ============================================================================

/// Viewer for directories/folders.
///
/// Displays a folder icon with name, item count, and modified time.
pub struct DirectoryViewer;

impl ContentViewer for DirectoryViewer {
    const KIND: PreviewKind = PreviewKind::Directory;

    type Content = DirectoryContent;
    type Info = FolderInfo;

    fn can_handle(mime: &str) -> bool {
        mime == "inode/directory"
    }

    async fn load(
        path: &Path,
        _config: &LoadConfig,
    ) -> Result<(Self::Content, Self::Info), ViewerError> {
        let path = path.to_path_buf();

        // Run filesystem operations in blocking task
        let result = tokio::task::spawn_blocking(move || load_folder_sync(&path))
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

        // Show children count
        if let Some(children) = info.format_children_count() {
            details = details.push(widget::text::body(children));
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
        // Directories don't have interactive state
    }

    fn details(info: &Self::Info) -> Vec<DetailSection> {
        let mut items = vec![DetailItem {
            label: "Name".to_string(),
            value: info.filename.clone(),
        }];

        items.push(DetailItem {
            label: "Type".to_string(),
            value: "Directory".to_string(),
        });

        if let Some(children) = info.format_children_count() {
            items.push(DetailItem {
                label: "Contents".to_string(),
                value: children,
            });
        }

        if let Some(modified) = info.format_modified() {
            items.push(DetailItem {
                label: "Modified".to_string(),
                value: modified,
            });
        }

        vec![DetailSection {
            title: "Folder Info".to_string(),
            items,
        }]
    }

    fn actions() -> Vec<ActionId> {
        // Directories don't have zoom/pan actions
        vec![]
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Load folder information synchronously.
fn load_folder_sync(path: &Path) -> Result<(DirectoryContent, FolderInfo), ViewerError> {
    let metadata = std::fs::metadata(path)
        .map_err(|e| ViewerError(format!("Failed to read metadata: {}", e)))?;

    if !metadata.is_dir() {
        return Err(ViewerError(format!(
            "'{}' is not a directory",
            path.display()
        )));
    }

    let canonical = path
        .canonicalize()
        .map_err(|e| ViewerError(format!("Failed to canonicalize path: {}", e)))?;

    let filename = canonical
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();

    // Get last modified time
    let modified = metadata.modified().ok();

    // Count children
    let children_count = std::fs::read_dir(&canonical)
        .ok()
        .map(|entries| entries.count());

    // Get folder icon
    let icon_handle = icon::from_name("folder").size(DEFAULT_ICON_SIZE).handle();

    Ok((
        DirectoryContent { icon_handle },
        FolderInfo {
            path: canonical,
            filename,
            modified,
            children_count,
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
    fn test_folder_info_format_children() {
        let info = FolderInfo {
            path: PathBuf::from("/tmp"),
            filename: "tmp".to_string(),
            modified: None,
            children_count: Some(1),
        };
        assert_eq!(info.format_children_count(), Some("1 item".to_string()));

        let info = FolderInfo {
            path: PathBuf::from("/tmp"),
            filename: "tmp".to_string(),
            modified: None,
            children_count: Some(5),
        };
        assert_eq!(info.format_children_count(), Some("5 items".to_string()));

        let info = FolderInfo {
            path: PathBuf::from("/tmp"),
            filename: "tmp".to_string(),
            modified: None,
            children_count: None,
        };
        assert_eq!(info.format_children_count(), None);
    }

    #[test]
    fn test_can_handle() {
        assert!(DirectoryViewer::can_handle("inode/directory"));
        assert!(!DirectoryViewer::can_handle("image/png"));
    }
}

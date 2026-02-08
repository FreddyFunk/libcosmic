//! Folder/directory loader
//!
//! This module provides loading and display for directories.
//! It shows a folder icon with the directory name, item count, and modified time.

use crate::types::FolderInfo;
use cosmic::widget::icon;
use cosmic::{widget, Element};
use std::path::Path;

/// Default icon size for folder display
const DEFAULT_ICON_SIZE: u16 = 128;

/// Load folder information
///
/// Returns the folder icon handle and folder info.
pub fn load_folder(path: &Path) -> Result<(icon::Handle, FolderInfo), String> {
    let metadata = std::fs::metadata(path)
        .map_err(|e| format!("Failed to read metadata: {}", e))?;

    if !metadata.is_dir() {
        return Err(format!("'{}' is not a directory", path.display()));
    }

    let canonical = path.canonicalize()
        .map_err(|e| format!("Failed to canonicalize path: {}", e))?;

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
    let icon_handle = icon::from_name("folder")
        .size(DEFAULT_ICON_SIZE)
        .handle();

    Ok((
        icon_handle,
        FolderInfo {
            path: canonical,
            filename,
            modified,
            children_count,
        },
    ))
}

/// Create the folder preview widget
///
/// Displays the folder icon, name, item count, and modified time.
pub fn folder_view<M: 'static>(
    icon_handle: &icon::Handle,
    info: &FolderInfo,
) -> Element<'static, M> {
    let filename = info.filename.clone();

    let mut column = widget::column()
        .spacing(16)
        .align_x(cosmic::iced::Alignment::Center)
        .push(widget::icon(icon_handle.clone()).size(DEFAULT_ICON_SIZE))
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

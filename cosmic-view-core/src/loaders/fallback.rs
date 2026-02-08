//! Fallback loader for unsupported files
//!
//! This module provides icon loading for files that are not supported by any
//! specialized loader. It uses the system icon theme via MIME type lookup.
//!
//! Note: Directories are handled by the folder_loader module.

use crate::types::FallbackInfo;
use cosmic::widget::icon;
use cosmic::{widget, Element};
use std::path::Path;

/// Default icon size
const DEFAULT_ICON_SIZE: u16 = 128;

/// Fallback icon name for files without a specific icon
const FALLBACK_MIME_ICON: &str = "text-x-generic";

/// Load icon for an unsupported file
///
/// Uses the system icon theme to find the appropriate icon based on MIME type.
/// For directories, use folder_loader::load_folder() instead.
#[cfg(target_os = "linux")]
pub fn load_fallback(path: &Path) -> Result<(icon::Handle, FallbackInfo), String> {
    let metadata = std::fs::metadata(path)
        .map_err(|e| format!("Failed to read metadata: {}", e))?;

    if metadata.is_dir() {
        return Err("Use folder_loader for directories".to_string());
    }

    let canonical = path.canonicalize()
        .map_err(|e| format!("Failed to canonicalize path: {}", e))?;

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
    let guess = shared_mime.guess_mime_type()
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
        icon_handle,
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

/// Non-Linux stub implementation
#[cfg(not(target_os = "linux"))]
pub fn load_fallback(path: &Path) -> Result<(icon::Handle, FallbackInfo), String> {
    let metadata = std::fs::metadata(path)
        .map_err(|e| format!("Failed to read metadata: {}", e))?;

    if metadata.is_dir() {
        return Err("Use folder_loader for directories".to_string());
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
    let handle = icon::from_name(FALLBACK_MIME_ICON)
        .size(DEFAULT_ICON_SIZE)
        .handle();

    Ok((
        handle,
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

/// Create the fallback file preview widget
///
/// Displays the file icon, name, size, and modified time.
pub fn fallback_view<M: 'static>(
    icon_handle: &icon::Handle,
    info: &FallbackInfo,
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

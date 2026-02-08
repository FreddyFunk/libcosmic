//! Preview details generation.
//!
//! This module contains methods for generating structured details about
//! previewed content for display in info drawers.

use crate::types::{ContentFit, LoadedContent, ViewTransform};
#[cfg(feature = "directory")]
use cosmic_view_directory::FolderInfo;
#[cfg(feature = "fallback")]
use cosmic_view_fallback::FallbackInfo;
#[cfg(feature = "image")]
use cosmic_view_image::ImageInfo;
#[cfg(feature = "text")]
use cosmic_view_text::TextInfo;
use super::state::PreviewState;
use super::types::{DetailItem, DetailSection, PreviewDetails};

/// Generate details about the current preview content.
#[allow(unused_variables)]
pub fn generate_details(state: &PreviewState) -> PreviewDetails {
    let sections = match state.content() {
        #[cfg(feature = "image")]
        LoadedContent::Raster { info, .. } => {
            image_details(info, state.transform(), state.content_fit())
        }
        #[cfg(feature = "image")]
        LoadedContent::Svg { info, .. } => {
            image_details(info, state.transform(), state.content_fit())
        }
        #[cfg(feature = "text")]
        LoadedContent::Text { info, .. } => text_details(info),
        #[cfg(feature = "fallback")]
        LoadedContent::Fallback { info, .. } => fallback_details(info),
        #[cfg(feature = "directory")]
        LoadedContent::Folder { info, .. } => folder_details(info),
        LoadedContent::NotLoaded | LoadedContent::Loading | LoadedContent::Error(_) => vec![],
    };

    PreviewDetails {
        kind: state.kind(),
        sections,
    }
}

#[cfg(feature = "image")]
fn image_details(info: &ImageInfo, transform: &ViewTransform, fit: ContentFit) -> Vec<DetailSection> {
    vec![
        DetailSection {
            title: "File Info".to_string(),
            items: vec![
                DetailItem {
                    label: "Filename".to_string(),
                    value: info
                        .path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Unknown")
                        .to_string(),
                },
                DetailItem {
                    label: "Format".to_string(),
                    value: info.format.name().to_string(),
                },
                DetailItem {
                    label: "Dimensions".to_string(),
                    value: info.format_dimensions(),
                },
                DetailItem {
                    label: "File Size".to_string(),
                    value: info.format_file_size(),
                },
            ],
        },
        DetailSection {
            title: "View Info".to_string(),
            items: vec![
                DetailItem {
                    label: "Zoom".to_string(),
                    value: transform.format_zoom(),
                },
                DetailItem {
                    label: "Fit Mode".to_string(),
                    value: match fit {
                        ContentFit::Contain => "Contain".to_string(),
                        ContentFit::Cover => "Cover".to_string(),
                    },
                },
            ],
        },
    ]
}

#[cfg(feature = "text")]
fn text_details(info: &TextInfo) -> Vec<DetailSection> {
    vec![DetailSection {
        title: "File Info".to_string(),
        items: vec![
            DetailItem {
                label: "Filename".to_string(),
                value: info
                    .path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown")
                    .to_string(),
            },
            DetailItem {
                label: "Syntax".to_string(),
                value: info.syntax_name.clone(),
            },
            DetailItem {
                label: "Lines".to_string(),
                value: info.format_line_count(),
            },
            DetailItem {
                label: "File Size".to_string(),
                value: info.format_file_size(),
            },
        ],
    }]
}

#[cfg(feature = "fallback")]
fn fallback_details(info: &FallbackInfo) -> Vec<DetailSection> {
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

#[cfg(feature = "directory")]
fn folder_details(info: &FolderInfo) -> Vec<DetailSection> {
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

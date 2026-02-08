//! Preview details generation.
//!
//! This module contains methods for generating structured details about
//! previewed content for display in info drawers.

#[cfg(feature = "view3d")]
use std::sync::Arc;

use crate::loaders::pdf::PdfInfo;
#[cfg(feature = "view3d")]
use cosmic_view_3d::mesh_data::PbrWorkflow;
#[cfg(feature = "view3d")]
use cosmic_view_3d::{ModelInfo, SceneData};
use crate::types::{ContentFit, FallbackInfo, FolderInfo, ImageInfo, LoadedContent, TextInfo, ViewTransform};
#[cfg(feature = "view3d")]
use crate::util::formatting;
use super::state::PreviewState;
use super::types::{DetailItem, DetailSection, PreviewDetails};

/// Generate details about the current preview content.
pub fn generate_details(state: &PreviewState) -> PreviewDetails {
    let sections = match state.content() {
        LoadedContent::Raster { info, .. } => {
            image_details(info, state.transform(), state.content_fit())
        }
        LoadedContent::Svg { info, .. } => {
            image_details(info, state.transform(), state.content_fit())
        }
        LoadedContent::Text { info, .. } => text_details(info),
        LoadedContent::Pdf { info, .. } => pdf_details(info),
        #[cfg(feature = "view3d")]
        LoadedContent::Model3D { info, .. } => model_details(info, state.model_scene()),
        LoadedContent::Fallback { info, .. } => fallback_details(info),
        LoadedContent::Folder { info, .. } => folder_details(info),
        LoadedContent::NotLoaded | LoadedContent::Loading | LoadedContent::Error(_) => vec![],
    };

    PreviewDetails {
        kind: state.kind(),
        sections,
    }
}

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

fn pdf_details(info: &PdfInfo) -> Vec<DetailSection> {
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
                label: "Format".to_string(),
                value: "PDF".to_string(),
            },
            DetailItem {
                label: "Pages".to_string(),
                value: format!("{}", info.page_count),
            },
            DetailItem {
                label: "Page Size".to_string(),
                value: info.format_page_size(),
            },
            DetailItem {
                label: "File Size".to_string(),
                value: info.format_file_size(),
            },
        ],
    }]
}

#[cfg(feature = "view3d")]
fn model_details(info: &ModelInfo, scene: Option<&Arc<SceneData>>) -> Vec<DetailSection> {
    let mut sections = Vec::new();

    // Geometry section
    let vertex_size = info.vertex_count * std::mem::size_of::<[f32; 14]>();
    let index_size = info.triangle_count * 3 * std::mem::size_of::<u32>();
    let total_geometry_size = vertex_size + index_size;

    sections.push(DetailSection {
        title: "Geometry".to_string(),
        items: vec![
            DetailItem {
                label: "Meshes".to_string(),
                value: format!("{}", info.mesh_count),
            },
            DetailItem {
                label: "Vertices".to_string(),
                value: format_number(info.vertex_count),
            },
            DetailItem {
                label: "Triangles".to_string(),
                value: format_number(info.triangle_count),
            },
            DetailItem {
                label: "Memory".to_string(),
                value: formatting::format_file_size(total_geometry_size as u64),
            },
        ],
    });

    // Materials section
    if let Some(scene) = scene {
        let mut mat_items = vec![DetailItem {
            label: "Count".to_string(),
            value: format!("{}", info.material_count),
        }];

        // Analyze materials
        let mut pbr_metallic = 0;
        let mut transparent_count = 0;
        for mat in &scene.materials {
            if matches!(mat.workflow, PbrWorkflow::MetallicRoughness) {
                pbr_metallic += 1;
            }
            if mat.alpha_mode == 2 || mat.opacity < 1.0 {
                transparent_count += 1;
            }
        }

        if pbr_metallic > 0 {
            mat_items.push(DetailItem {
                label: "PBR Metallic".to_string(),
                value: format!("{}", pbr_metallic),
            });
        }
        if transparent_count > 0 {
            mat_items.push(DetailItem {
                label: "Transparent".to_string(),
                value: format!("{}", transparent_count),
            });
        }

        sections.push(DetailSection {
            title: "Materials".to_string(),
            items: mat_items,
        });
    }

    // Animation section
    if info.animation_count > 0 || info.bone_count > 0 {
        sections.push(DetailSection {
            title: "Animation".to_string(),
            items: vec![
                DetailItem {
                    label: "Animations".to_string(),
                    value: format!("{}", info.animation_count),
                },
                DetailItem {
                    label: "Bones".to_string(),
                    value: format!("{}", info.bone_count),
                },
            ],
        });
    }

    // File section
    sections.push(DetailSection {
        title: "File".to_string(),
        items: vec![DetailItem {
            label: "Size".to_string(),
            value: formatting::format_file_size(info.file_size),
        }],
    });

    sections
}

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

/// Format a number with comma separators (e.g., "1,234,567")
#[cfg(feature = "view3d")]
fn format_number(n: usize) -> String {
    let mut s = String::new();
    let n_str = n.to_string();
    let chars: Vec<char> = n_str.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            s.push(',');
        }
        s.push(*c);
    }
    s
}

#[cfg(all(test, feature = "view3d"))]
mod tests {
    use super::*;

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(123), "123");
        assert_eq!(format_number(1234), "1,234");
        assert_eq!(format_number(1234567), "1,234,567");
    }
}

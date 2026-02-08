//! SVG file loading
//!
//! This module provides utilities for loading SVG files as vector graphics.
//! SVGs are loaded directly into iced's svg::Handle for efficient rendering
//! at any scale without quality loss.

use crate::types::ImageInfo;
use cosmic::widget::svg;
use std::path::Path;

/// Maximum SVG file size (5 MB) - SVGs are typically small, large ones may be malicious
pub const MAX_SVG_SIZE: u64 = 5 * 1024 * 1024;

/// Load an SVG file and return a handle for rendering
///
/// Returns the SVG handle and image info on success.
pub fn load_svg_file(path: &Path) -> Result<(svg::Handle, ImageInfo), String> {
    // Check file exists and get metadata
    let metadata = std::fs::metadata(path)
        .map_err(|e| format!("Failed to read file metadata: {}", e))?;

    // Security: Ensure it's a regular file
    if !metadata.is_file() {
        return Err("Path is not a regular file".to_string());
    }

    let file_size = metadata.len();

    // Security: Check file size limit
    if file_size > MAX_SVG_SIZE {
        return Err(format!(
            "SVG file is too large ({:.1} MB). Maximum allowed is {:.0} MB.",
            file_size as f64 / 1_048_576.0,
            MAX_SVG_SIZE as f64 / 1_048_576.0
        ));
    }

    // Create SVG handle from path
    let handle = svg::Handle::from_path(path);

    // For SVG, we don't have intrinsic dimensions easily available
    // Use placeholder dimensions - the actual rendering will be vector-based
    let info = ImageInfo {
        path: path.to_path_buf(),
        width: 0,  // SVG dimensions are determined at render time
        height: 0,
        format: crate::types::ImageFormat::Svg,
        file_size,
        is_preview: false,
        displayed_width: 0,
        displayed_height: 0,
    };

    Ok((handle, info))
}

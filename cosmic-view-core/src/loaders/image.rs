//! Image loading utilities
//!
//! This module provides utilities for detecting image formats,
//! extracting metadata, and decoding images including JPEG XL (JXL).
//!
//! For JPEG files, this module supports DCT scaling during decode,
//! which provides significant performance improvements for large images
//! by decoding at reduced resolution (1/2, 1/4, or 1/8 scale).

use crate::types::{ImageFormat, ImageInfo};
use image::{DynamicImage, ImageDecoder, ImageReader, Limits, RgbaImage};
use jxl_oxide::integration::JxlDecoder;
use rayon::prelude::*;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

/// Check if a file is a JPEG XL image
fn is_jxl_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("jxl"))
        .unwrap_or(false)
}

/// Detect image format from file extension
fn detect_format(path: &Path) -> ImageFormat {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(ImageFormat::from_extension)
        .unwrap_or(ImageFormat::Unknown)
}

/// Detect image format by reading magic bytes from file content
fn detect_format_by_magic_bytes(path: &Path) -> Option<ImageFormat> {
    let mut file = File::open(path).ok()?;
    let mut buffer = [0u8; 16];
    let bytes_read = file.read(&mut buffer).ok()?;

    if bytes_read < 4 {
        return None;
    }

    // PNG: \x89PNG\r\n\x1a\n
    if buffer.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Some(ImageFormat::Png);
    }

    // JPEG: \xff\xd8\xff
    if buffer.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some(ImageFormat::Jpeg);
    }

    // GIF: GIF87a or GIF89a
    if buffer.starts_with(b"GIF87a") || buffer.starts_with(b"GIF89a") {
        return Some(ImageFormat::Gif);
    }

    // WebP: RIFF....WEBP (RIFF at 0, WEBP at 8)
    if bytes_read >= 12 && buffer.starts_with(b"RIFF") && &buffer[8..12] == b"WEBP" {
        return Some(ImageFormat::WebP);
    }

    // BMP: BM
    if buffer.starts_with(b"BM") {
        return Some(ImageFormat::Bmp);
    }

    // TIFF: II (little-endian) or MM (big-endian)
    if buffer.starts_with(&[0x49, 0x49, 0x2A, 0x00])
        || buffer.starts_with(&[0x4D, 0x4D, 0x00, 0x2A])
    {
        return Some(ImageFormat::Tiff);
    }

    // ICO: \x00\x00\x01\x00
    if buffer.starts_with(&[0x00, 0x00, 0x01, 0x00]) {
        return Some(ImageFormat::Ico);
    }

    // SVG: Check for XML/SVG markers (text-based)
    if buffer.starts_with(b"<?xml") || buffer.starts_with(b"<svg") || buffer.starts_with(b"<SVG") {
        return Some(ImageFormat::Svg);
    }

    None
}

/// Detect image format with magic byte verification
///
/// First checks magic bytes, falls back to extension-based detection.
fn detect_format_verified(path: &Path) -> ImageFormat {
    if let Some(format) = detect_format_by_magic_bytes(path) {
        return format;
    }
    detect_format(path)
}

/// Get the dimensions of an image without fully decoding it.
pub fn get_image_dimensions(path: &Path) -> Option<(u32, u32)> {
    if is_jxl_file(path) {
        get_jxl_dimensions(path)
    } else {
        get_standard_image_dimensions(path)
    }
}

/// Get dimensions of standard image formats (non-JXL)
fn get_standard_image_dimensions(path: &Path) -> Option<(u32, u32)> {
    match ImageReader::open(path) {
        Ok(reader) => match reader.into_dimensions() {
            Ok((width, height)) => Some((width, height)),
            Err(e) => {
                tracing::warn!("Failed to get dimensions for {}: {}", path.display(), e);
                None
            }
        },
        Err(e) => {
            tracing::warn!("Failed to open image reader for {}: {}", path.display(), e);
            None
        }
    }
}

/// Get dimensions of a JPEG XL image
fn get_jxl_dimensions(path: &Path) -> Option<(u32, u32)> {
    match File::open(path) {
        Ok(file) => match JxlDecoder::new(file) {
            Ok(decoder) => {
                let (width, height) = decoder.dimensions();
                Some((width, height))
            }
            Err(e) => {
                tracing::warn!("Failed to create JXL decoder for {}: {}", path.display(), e);
                None
            }
        },
        Err(e) => {
            tracing::warn!("Failed to open JXL file {}: {}", path.display(), e);
            None
        }
    }
}

/// Decode an image file to a DynamicImage.
///
/// Handles all supported formats including JXL transparently.
pub fn decode_image(path: &Path, max_alloc: Option<u64>) -> Result<DynamicImage, String> {
    if is_jxl_file(path) {
        decode_jxl_image(path, max_alloc)
    } else {
        decode_standard_image(path, max_alloc)
    }
}

/// Decode a standard image format (non-JXL)
fn decode_standard_image(path: &Path, max_alloc: Option<u64>) -> Result<DynamicImage, String> {
    let reader = ImageReader::open(path)
        .map_err(|e| format!("Failed to open {}: {}", path.display(), e))?;

    let mut reader = reader
        .with_guessed_format()
        .map_err(|e| format!("Failed to guess format for {}: {}", path.display(), e))?;

    if let Some(max) = max_alloc {
        let mut limits = Limits::default();
        limits.max_alloc = Some(max);
        reader.limits(limits);
    }

    reader
        .decode()
        .map_err(|e| format!("Failed to decode {}: {}", path.display(), e))
}

/// Decode a JPEG XL image
fn decode_jxl_image(path: &Path, max_alloc: Option<u64>) -> Result<DynamicImage, String> {
    let file = File::open(path)
        .map_err(|e| format!("Failed to open JXL file {}: {}", path.display(), e))?;

    let mut decoder = JxlDecoder::new(file)
        .map_err(|e| format!("Failed to create JXL decoder for {}: {}", path.display(), e))?;

    if let Some(max) = max_alloc {
        let mut limits = Limits::default();
        limits.max_alloc = Some(max);
        let _ = decoder.set_limits(limits);
    }

    DynamicImage::from_decoder(decoder)
        .map_err(|e| format!("Failed to decode JXL {}: {}", path.display(), e))
}

/// Get image info (metadata) without fully decoding the image
pub fn get_image_info(path: &Path) -> Result<ImageInfo, String> {
    let file_size = std::fs::metadata(path)
        .map(|m| m.len())
        .map_err(|e| format!("Failed to read file metadata: {}", e))?;

    let format = detect_format_verified(path);

    let (width, height) =
        get_image_dimensions(path).ok_or_else(|| "Failed to read image dimensions".to_string())?;

    Ok(ImageInfo {
        path: path.to_path_buf(),
        width,
        height,
        format,
        file_size,
        is_preview: false,
        displayed_width: width,
        displayed_height: height,
    })
}

// ============================================================================
// Optimized JPEG Decoding with DCT Scaling
// ============================================================================

/// Check if the file is a JPEG by reading magic bytes
pub fn is_jpeg_file(path: &Path) -> bool {
    matches!(detect_format_by_magic_bytes(path), Some(ImageFormat::Jpeg))
}

/// Calculate the optimal DCT scale factor for a target dimension.
///
/// Returns the scale divisor (1, 2, 4, or 8) that produces an image
/// at least as large as the target dimension.
fn calculate_jpeg_scale_factor(original: u32, target: u32) -> u16 {
    if target == 0 || original <= target {
        return 1;
    }

    // jpeg-decoder supports 1/1, 1/2, 1/4, 1/8 scaling
    // Choose the smallest scale that still produces >= target pixels
    let ratio = original as f64 / target as f64;

    if ratio <= 1.0 {
        1
    } else if ratio <= 2.0 {
        2
    } else if ratio <= 4.0 {
        4
    } else {
        8
    }
}

/// Decode a JPEG image with optional DCT scaling for faster loading of large images.
///
/// If `max_dimension` is Some and the image is larger, decodes at reduced resolution
/// using DCT scaling (1/2, 1/4, or 1/8 scale). This is much faster than full decode
/// because it skips processing high-frequency DCT coefficients.
///
/// Returns the RGBA image and whether it was scaled (true) or full resolution (false).
pub fn decode_jpeg_scaled(
    path: &Path,
    max_dimension: Option<u32>,
) -> Result<(RgbaImage, bool), String> {
    let file = File::open(path)
        .map_err(|e| format!("Failed to open JPEG file {}: {}", path.display(), e))?;

    let mut decoder = jpeg_decoder::Decoder::new(BufReader::new(file));

    // Read header to get dimensions
    decoder
        .read_info()
        .map_err(|e| format!("Failed to read JPEG header: {}", e))?;

    let info = decoder
        .info()
        .ok_or_else(|| "Failed to get JPEG info after reading header".to_string())?;

    let original_width = info.width as u32;
    let original_height = info.height as u32;

    // Determine if we need to scale
    let (scale_divisor, scaled) = if let Some(max_dim) = max_dimension {
        let max_original = original_width.max(original_height);
        if max_original > max_dim {
            let scale = calculate_jpeg_scale_factor(max_original, max_dim);
            (scale, scale > 1)
        } else {
            (1, false)
        }
    } else {
        (1, false)
    };

    // Apply scaling if needed
    if scale_divisor > 1 {
        let target_w = original_width / scale_divisor as u32;
        let target_h = original_height / scale_divisor as u32;
        decoder
            .scale(target_w as u16, target_h as u16)
            .map_err(|e| format!("Failed to set JPEG scale: {}", e))?;
    }

    // Decode the image
    let pixels = decoder
        .decode()
        .map_err(|e| format!("Failed to decode JPEG: {}", e))?;

    let info = decoder
        .info()
        .ok_or_else(|| "Failed to get JPEG info after decoding".to_string())?;

    let width = info.width as u32;
    let height = info.height as u32;

    // Convert to RGBA using parallel processing for larger images
    let rgba_pixels = convert_to_rgba(&pixels, info.pixel_format, width, height)?;

    let rgba_image = RgbaImage::from_raw(width, height, rgba_pixels)
        .ok_or_else(|| "Failed to create RGBA image from decoded pixels".to_string())?;

    Ok((rgba_image, scaled))
}

/// Decode a JPEG at full resolution (no scaling).
pub fn decode_jpeg_full(path: &Path) -> Result<RgbaImage, String> {
    let (image, _) = decode_jpeg_scaled(path, None)?;
    Ok(image)
}

/// Convert jpeg-decoder pixel data to RGBA format.
///
/// Uses parallel processing for larger images (>1M pixels).
fn convert_to_rgba(
    pixels: &[u8],
    format: jpeg_decoder::PixelFormat,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, String> {
    let pixel_count = width as usize * height as usize;
    let use_parallel = pixel_count > 1_000_000; // Use rayon for >1MP images

    match format {
        jpeg_decoder::PixelFormat::L8 => {
            // Grayscale to RGBA
            if use_parallel {
                Ok(pixels.par_iter().flat_map(|&l| [l, l, l, 255]).collect())
            } else {
                Ok(pixels.iter().flat_map(|&l| [l, l, l, 255]).collect())
            }
        }
        jpeg_decoder::PixelFormat::RGB24 => {
            // RGB to RGBA
            if use_parallel {
                Ok(pixels
                    .par_chunks_exact(3)
                    .flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 255])
                    .collect())
            } else {
                Ok(pixels
                    .chunks_exact(3)
                    .flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 255])
                    .collect())
            }
        }
        jpeg_decoder::PixelFormat::CMYK32 => {
            // CMYK to RGBA (simplified conversion)
            let convert_cmyk = |cmyk: &[u8]| {
                let c = cmyk[0] as f32 / 255.0;
                let m = cmyk[1] as f32 / 255.0;
                let y = cmyk[2] as f32 / 255.0;
                let k = cmyk[3] as f32 / 255.0;
                let r = (255.0 * (1.0 - c) * (1.0 - k)) as u8;
                let g = (255.0 * (1.0 - m) * (1.0 - k)) as u8;
                let b = (255.0 * (1.0 - y) * (1.0 - k)) as u8;
                [r, g, b, 255]
            };

            if use_parallel {
                Ok(pixels.par_chunks_exact(4).flat_map(convert_cmyk).collect())
            } else {
                Ok(pixels.chunks_exact(4).flat_map(convert_cmyk).collect())
            }
        }
        jpeg_decoder::PixelFormat::L16 => {
            // 16-bit grayscale to RGBA (convert to 8-bit)
            if use_parallel {
                Ok(pixels
                    .par_chunks_exact(2)
                    .flat_map(|bytes| {
                        let l = (u16::from_ne_bytes([bytes[0], bytes[1]]) >> 8) as u8;
                        [l, l, l, 255]
                    })
                    .collect())
            } else {
                Ok(pixels
                    .chunks_exact(2)
                    .flat_map(|bytes| {
                        let l = (u16::from_ne_bytes([bytes[0], bytes[1]]) >> 8) as u8;
                        [l, l, l, 255]
                    })
                    .collect())
            }
        }
    }
}

/// Result of a scaled image decode operation.
#[derive(Debug)]
pub struct ScaledDecodeResult {
    /// The decoded RGBA image (possibly at reduced resolution)
    pub image: RgbaImage,
    /// Original image dimensions before any scaling
    pub original_width: u32,
    pub original_height: u32,
    /// Whether the image was decoded at reduced resolution
    pub is_scaled: bool,
}

/// Decode an image with optional scaling for large images.
///
/// For JPEG files, uses DCT scaling for faster decoding.
/// For other formats, decodes at full resolution then downscales if needed.
///
/// Returns the image, original dimensions, and whether it was scaled.
pub fn decode_image_scaled(
    path: &Path,
    max_dimension: Option<u32>,
) -> Result<ScaledDecodeResult, String> {
    // Get original dimensions first
    let (original_width, original_height) =
        get_image_dimensions(path).ok_or_else(|| "Failed to read image dimensions".to_string())?;

    // Check if this is a JPEG that can use DCT scaling
    if is_jpeg_file(path) {
        let (image, is_scaled) = decode_jpeg_scaled(path, max_dimension)?;
        return Ok(ScaledDecodeResult {
            image,
            original_width,
            original_height,
            is_scaled,
        });
    }

    // For non-JPEG formats, decode at full resolution
    let full_image = if is_jxl_file(path) {
        decode_jxl_image(path, None)?
    } else {
        decode_standard_image(path, None)?
    };

    // Check if we need to downscale
    let needs_downscale = if let Some(max_dim) = max_dimension {
        original_width.max(original_height) > max_dim
    } else {
        false
    };

    if needs_downscale {
        let max_dim = max_dimension.unwrap();
        // Calculate target dimensions maintaining aspect ratio
        let scale = max_dim as f64 / original_width.max(original_height) as f64;
        let target_width = (original_width as f64 * scale) as u32;
        let target_height = (original_height as f64 * scale) as u32;

        // Use the image crate's thumbnail function for efficient downscaling
        let thumbnail = full_image.thumbnail(target_width, target_height);
        Ok(ScaledDecodeResult {
            image: thumbnail.to_rgba8(),
            original_width,
            original_height,
            is_scaled: true,
        })
    } else {
        Ok(ScaledDecodeResult {
            image: full_image.to_rgba8(),
            original_width,
            original_height,
            is_scaled: false,
        })
    }
}

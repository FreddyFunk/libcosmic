//! Image viewer for COSMIC desktop applications.
//!
//! This crate provides the [`ImageViewer`] which implements the
//! [`ContentViewer`] trait for viewing raster images and SVGs.
//!
//! # Features
//!
//! - Raster image support: JPEG, PNG, WebP, GIF, BMP, TIFF, ICO
//! - JPEG XL (JXL) support via jxl-oxide
//! - SVG vector graphics
//! - DCT scaling for fast JPEG loading at reduced resolution
//! - Progressive loading with preview then full resolution
//!
//! # Example
//!
//! ```rust,ignore
//! use cosmic_view_image::{ImageViewer, ImageContent, ImageInfo};
//! use cosmic_view_types::{ContentViewer, LoadConfig};
//!
//! let (content, info) = ImageViewer::load(path, &LoadConfig::default()).await?;
//! let widget = ImageViewer::view(&content, &info, &transform, &config);
//! ```

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use cosmic::widget::{self, image as iced_image, svg};
use cosmic::Element;
use cosmic_view_types::{
    ActionId, ContentViewer, DetailItem, DetailSection, LoadConfig, PreviewKind, PreviewMessage,
    ViewConfig, ViewTransform, ViewerError, format_file_size,
};
use image::{DynamicImage, ImageDecoder, ImageReader, Limits, RgbaImage};
use jxl_oxide::integration::JxlDecoder;
use rayon::prelude::*;

/// Maximum SVG file size (5 MB) - SVGs are typically small, large ones may be malicious
pub const MAX_SVG_SIZE: u64 = 5 * 1024 * 1024;

// ============================================================================
// Types
// ============================================================================

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
    Jxl,
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
            "jxl" => Self::Jxl,
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
            Self::Jxl => "JPEG XL",
            Self::Unknown => "Unknown",
        }
    }

    /// Check if format is a raster image (not vector)
    pub fn is_raster(&self) -> bool {
        matches!(
            self,
            Self::Jpeg
                | Self::Png
                | Self::WebP
                | Self::Gif
                | Self::Bmp
                | Self::Tiff
                | Self::Ico
                | Self::Jxl
        )
    }

    /// Check if this is an SVG (vector) format
    pub fn is_svg(&self) -> bool {
        matches!(self, Self::Svg)
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
        format_file_size(self.file_size)
    }

    /// Format dimensions for display (e.g., "1920 x 1080")
    pub fn format_dimensions(&self) -> String {
        format!("{} x {}", self.width, self.height)
    }
}

/// Content data for a loaded image.
#[derive(Clone)]
pub enum ImageContent {
    /// Raster image (JPEG, PNG, etc.)
    Raster {
        /// Image handle for rendering
        handle: iced_image::Handle,
    },
    /// SVG vector graphic
    Svg {
        /// SVG handle for rendering
        handle: svg::Handle,
    },
}

impl std::fmt::Debug for ImageContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Raster { .. } => f.debug_struct("Raster").finish_non_exhaustive(),
            Self::Svg { .. } => f.debug_struct("Svg").finish_non_exhaustive(),
        }
    }
}

// ============================================================================
// ImageViewer Implementation
// ============================================================================

/// Viewer for raster images and SVGs.
///
/// Supports JPEG, PNG, WebP, GIF, BMP, TIFF, ICO, JPEG XL, and SVG formats.
/// For large JPEG images, uses DCT scaling for fast preview loading.
pub struct ImageViewer;

impl ContentViewer for ImageViewer {
    const KIND: PreviewKind = PreviewKind::Image;

    type Content = ImageContent;
    type Info = ImageInfo;

    fn can_handle(mime: &str) -> bool {
        mime.starts_with("image/")
    }

    async fn load(
        path: &Path,
        config: &LoadConfig,
    ) -> Result<(Self::Content, Self::Info), ViewerError> {
        let path = path.to_path_buf();
        let max_dimension = config.max_dimension;

        // Run image loading in blocking task
        let result = tokio::task::spawn_blocking(move || load_image_sync(&path, max_dimension))
            .await
            .map_err(|e| ViewerError(format!("Task join error: {}", e)))?;

        result
    }

    fn view<'a, M: Clone + 'static>(
        content: &'a Self::Content,
        _info: &'a Self::Info,
        _transform: &ViewTransform,
        config: &ViewConfig,
    ) -> Element<'a, M> {
        let content_fit = config.content_fit;

        match content {
            ImageContent::Raster { handle } => {
                let image_widget = widget::image(handle.clone())
                    .content_fit(content_fit.into());

                widget::container(image_widget)
                    .width(cosmic::iced::Length::Fill)
                    .height(cosmic::iced::Length::Fill)
                    .align_x(cosmic::iced::alignment::Horizontal::Center)
                    .align_y(cosmic::iced::alignment::Vertical::Center)
                    .into()
            }
            ImageContent::Svg { handle } => {
                let svg_widget = widget::svg(handle.clone())
                    .content_fit(content_fit.into());

                widget::container(svg_widget)
                    .width(cosmic::iced::Length::Fill)
                    .height(cosmic::iced::Length::Fill)
                    .align_x(cosmic::iced::alignment::Horizontal::Center)
                    .align_y(cosmic::iced::alignment::Vertical::Center)
                    .into()
            }
        }
    }

    fn update(_content: &mut Self::Content, _msg: PreviewMessage) {
        // Images don't have interactive state to update
    }

    fn details(info: &Self::Info) -> Vec<DetailSection> {
        let mut items = vec![
            DetailItem {
                label: "Format".to_string(),
                value: info.format.name().to_string(),
            },
            DetailItem {
                label: "Dimensions".to_string(),
                value: info.format_dimensions(),
            },
            DetailItem {
                label: "Size".to_string(),
                value: info.format_file_size(),
            },
        ];

        if info.is_preview {
            items.push(DetailItem {
                label: "Preview".to_string(),
                value: format!("{} x {} (scaled)", info.displayed_width, info.displayed_height),
            });
        }

        vec![DetailSection {
            title: "Image Info".to_string(),
            items,
        }]
    }

    fn actions() -> Vec<ActionId> {
        vec![
            ActionId::ZoomIn,
            ActionId::ZoomOut,
            ActionId::ZoomReset,
            ActionId::FitPage,
        ]
    }
}

// ============================================================================
// Image Loading Functions
// ============================================================================

/// Load an image synchronously (called from blocking task).
fn load_image_sync(
    path: &Path,
    max_dimension: Option<u32>,
) -> Result<(ImageContent, ImageInfo), ViewerError> {
    let format = detect_format_verified(path);

    if format.is_svg() {
        load_svg_sync(path)
    } else {
        load_raster_sync(path, max_dimension)
    }
}

/// Load a raster image synchronously.
fn load_raster_sync(
    path: &Path,
    max_dimension: Option<u32>,
) -> Result<(ImageContent, ImageInfo), ViewerError> {
    let result = decode_image_scaled(path, max_dimension)
        .map_err(|e| ViewerError(e))?;

    let file_size = std::fs::metadata(path)
        .map(|m| m.len())
        .unwrap_or(0);

    let format = detect_format_verified(path);

    // Save dimensions before consuming the image
    let displayed_width = result.image.width();
    let displayed_height = result.image.height();

    let handle = iced_image::Handle::from_rgba(
        displayed_width,
        displayed_height,
        result.image.into_raw(),
    );

    let info = ImageInfo {
        path: path.to_path_buf(),
        width: result.original_width,
        height: result.original_height,
        format,
        file_size,
        is_preview: result.is_scaled,
        displayed_width,
        displayed_height,
    };

    Ok((ImageContent::Raster { handle }, info))
}

/// Load an SVG file synchronously.
fn load_svg_sync(path: &Path) -> Result<(ImageContent, ImageInfo), ViewerError> {
    // Check file exists and get metadata
    let metadata = std::fs::metadata(path)
        .map_err(|e| ViewerError(format!("Failed to read file metadata: {}", e)))?;

    // Security: Ensure it's a regular file
    if !metadata.is_file() {
        return Err(ViewerError("Path is not a regular file".to_string()));
    }

    let file_size = metadata.len();

    // Security: Check file size limit
    if file_size > MAX_SVG_SIZE {
        return Err(ViewerError(format!(
            "SVG file is too large ({:.1} MB). Maximum allowed is {:.0} MB.",
            file_size as f64 / 1_048_576.0,
            MAX_SVG_SIZE as f64 / 1_048_576.0
        )));
    }

    // Create SVG handle from path
    let handle = svg::Handle::from_path(path);

    // For SVG, we don't have intrinsic dimensions easily available
    // Use placeholder dimensions - the actual rendering will be vector-based
    let info = ImageInfo {
        path: path.to_path_buf(),
        width: 0, // SVG dimensions are determined at render time
        height: 0,
        format: ImageFormat::Svg,
        file_size,
        is_preview: false,
        displayed_width: 0,
        displayed_height: 0,
    };

    Ok((ImageContent::Svg { handle }, info))
}

// ============================================================================
// Format Detection
// ============================================================================

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

    // JPEG XL: \xff\x0a (naked codestream) or container signature
    if buffer.starts_with(&[0xFF, 0x0A]) {
        return Some(ImageFormat::Jxl);
    }
    // JXL container: 0000 000C 4A58 4C20 0D0A 870A
    if bytes_read >= 12
        && buffer[0..4] == [0x00, 0x00, 0x00, 0x0C]
        && buffer[4..8] == [0x4A, 0x58, 0x4C, 0x20]
    {
        return Some(ImageFormat::Jxl);
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

// ============================================================================
// Image Dimension Helpers
// ============================================================================

/// Get the dimensions of an image without fully decoding it.
fn get_image_dimensions(path: &Path) -> Option<(u32, u32)> {
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

// ============================================================================
// Image Decoding
// ============================================================================

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

// ============================================================================
// Optimized JPEG Decoding with DCT Scaling
// ============================================================================

/// Check if the file is a JPEG by reading magic bytes
fn is_jpeg_file(path: &Path) -> bool {
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
fn decode_jpeg_scaled(
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
fn decode_image_scaled(
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

// ============================================================================
// Thumbnail Rendering
// ============================================================================

/// Render an image file to thumbnail pixels.
///
/// Handles both raster images and SVGs. Returns (width, height, rgba_pixels).
///
/// # Arguments
/// * `path` - Path to the image file
/// * `max_size` - Maximum dimension for the thumbnail
pub fn render_thumbnail(path: &Path, max_size: u32) -> Result<(u32, u32, Vec<u8>), String> {
    let format = detect_format_verified(path);

    if format.is_svg() {
        render_svg_thumbnail(path, max_size)
    } else {
        render_raster_thumbnail(path, max_size)
    }
}

/// Render a raster image to thumbnail pixels.
fn render_raster_thumbnail(path: &Path, max_size: u32) -> Result<(u32, u32, Vec<u8>), String> {
    // Decode at scaled size if possible
    let result = decode_image_scaled(path, Some(max_size))?;

    let (orig_w, orig_h) = (result.original_width, result.original_height);
    let (target_w, target_h) = calculate_thumbnail_size(orig_w, orig_h, max_size);

    // If already at target size or smaller, return as-is
    let img = result.image;
    if img.width() <= target_w && img.height() <= target_h {
        return Ok((img.width(), img.height(), img.into_raw()));
    }

    // Resize to target dimensions
    let img = DynamicImage::ImageRgba8(img);
    let thumbnail = img.thumbnail(target_w, target_h).into_rgba8();
    Ok((thumbnail.width(), thumbnail.height(), thumbnail.into_raw()))
}

/// Render an SVG to thumbnail pixels using resvg.
fn render_svg_thumbnail(path: &Path, max_size: u32) -> Result<(u32, u32, Vec<u8>), String> {
    let data = std::fs::read(path)
        .map_err(|e| format!("Failed to read SVG: {}", e))?;

    let options = usvg::Options::default();
    let tree = usvg::Tree::from_data(&data, &options)
        .map_err(|e| format!("Failed to parse SVG: {}", e))?;

    let size = tree.size();
    let (orig_w, orig_h) = (size.width() as u32, size.height() as u32);
    let (target_w, target_h) = calculate_thumbnail_size(orig_w.max(1), orig_h.max(1), max_size);

    let mut pixmap = tiny_skia::Pixmap::new(target_w, target_h)
        .ok_or_else(|| "Failed to create pixmap".to_string())?;

    let scale_x = target_w as f32 / size.width();
    let scale_y = target_h as f32 / size.height();
    let scale = scale_x.min(scale_y);

    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );

    Ok((target_w, target_h, pixmap.take()))
}

/// Calculate thumbnail dimensions maintaining aspect ratio.
fn calculate_thumbnail_size(orig_w: u32, orig_h: u32, max_size: u32) -> (u32, u32) {
    if orig_w <= max_size && orig_h <= max_size {
        return (orig_w, orig_h);
    }

    let aspect = orig_w as f64 / orig_h as f64;
    if orig_w > orig_h {
        let w = max_size;
        let h = (max_size as f64 / aspect).round() as u32;
        (w, h.max(1))
    } else {
        let h = max_size;
        let w = (max_size as f64 * aspect).round() as u32;
        (w.max(1), h)
    }
}

/// Resize raw RGBA pixels to a new size.
///
/// Useful for resizing pixels from other sources (e.g., PDF rendering).
/// Returns (new_width, new_height, new_pixels).
pub fn resize_rgba(
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    max_size: u32,
) -> Result<(u32, u32, Vec<u8>), String> {
    // If already within bounds, return as-is
    if width <= max_size && height <= max_size {
        return Ok((width, height, pixels));
    }

    let img = RgbaImage::from_raw(width, height, pixels)
        .ok_or_else(|| "Failed to create image from pixels".to_string())?;
    let img = DynamicImage::ImageRgba8(img);

    let (target_w, target_h) = calculate_thumbnail_size(width, height, max_size);
    let thumbnail = img.thumbnail(target_w, target_h).into_rgba8();
    Ok((thumbnail.width(), thumbnail.height(), thumbnail.into_raw()))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_format_from_extension() {
        assert_eq!(ImageFormat::from_extension("jpg"), ImageFormat::Jpeg);
        assert_eq!(ImageFormat::from_extension("JPEG"), ImageFormat::Jpeg);
        assert_eq!(ImageFormat::from_extension("png"), ImageFormat::Png);
        assert_eq!(ImageFormat::from_extension("svg"), ImageFormat::Svg);
        assert_eq!(ImageFormat::from_extension("jxl"), ImageFormat::Jxl);
        assert_eq!(ImageFormat::from_extension("xyz"), ImageFormat::Unknown);
    }

    #[test]
    fn test_image_format_is_raster() {
        assert!(ImageFormat::Jpeg.is_raster());
        assert!(ImageFormat::Png.is_raster());
        assert!(ImageFormat::Jxl.is_raster());
        assert!(!ImageFormat::Svg.is_raster());
        assert!(!ImageFormat::Unknown.is_raster());
    }

    #[test]
    fn test_can_handle() {
        assert!(ImageViewer::can_handle("image/jpeg"));
        assert!(ImageViewer::can_handle("image/png"));
        assert!(ImageViewer::can_handle("image/svg+xml"));
        assert!(!ImageViewer::can_handle("text/plain"));
        assert!(!ImageViewer::can_handle("application/pdf"));
    }

    #[test]
    fn test_jpeg_scale_factor() {
        // No scaling needed
        assert_eq!(calculate_jpeg_scale_factor(1000, 2000), 1);
        assert_eq!(calculate_jpeg_scale_factor(1000, 1000), 1);

        // Scale by 2
        assert_eq!(calculate_jpeg_scale_factor(2000, 1001), 2);
        assert_eq!(calculate_jpeg_scale_factor(2000, 1500), 2);

        // Scale by 4
        assert_eq!(calculate_jpeg_scale_factor(4000, 1001), 4);

        // Scale by 8
        assert_eq!(calculate_jpeg_scale_factor(8000, 1000), 8);
        assert_eq!(calculate_jpeg_scale_factor(10000, 1000), 8);
    }
}

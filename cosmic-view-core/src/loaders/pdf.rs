//! PDF file loading and rendering
//!
//! This module re-exports PDF functionality from cosmic-view-pdf when the
//! `pdf` feature is enabled. When disabled, stub types are provided to allow
//! the code to compile, but PDF files will fall through to the fallback renderer.

#[cfg(feature = "pdf")]
pub use cosmic_view_pdf::{
    get_pdf_info, render_pdf_page, render_pdf_pages_limited,
    PdfInfo, PdfRenderResult, RenderedPage,
    MAX_PDF_SIZE, MAX_RENDER_PAGES,
};

// Stub types when PDF feature is disabled
#[cfg(not(feature = "pdf"))]
#[allow(dead_code)]
mod stubs {
    use std::path::Path;

    /// Maximum number of pages to render in one go
    pub const MAX_RENDER_PAGES: usize = 20;

    /// Size limit for PDFs (500 MB)
    pub const MAX_PDF_SIZE: u64 = 500 * 1024 * 1024;

    /// Information about a loaded PDF (stub)
    #[derive(Debug, Clone)]
    pub struct PdfInfo {
        /// Path to the PDF file
        pub path: std::path::PathBuf,
        /// Total number of pages
        pub page_count: usize,
        /// Number of pages actually rendered
        pub rendered_pages: usize,
        /// Current page being displayed (0-indexed)
        pub current_page: usize,
        /// File size in bytes
        pub file_size: u64,
        /// Page dimensions (width, height) at default scale
        pub page_size: (f64, f64),
        /// Whether all pages have been rendered
        pub all_pages_rendered: bool,
    }

    impl PdfInfo {
        /// Format current page info as "Page X of Y"
        pub fn format_page_info(&self) -> String {
            format!("Page {} of {}", self.current_page + 1, self.page_count)
        }

        /// Format file size for display
        pub fn format_file_size(&self) -> String {
            crate::util::formatting::format_file_size(self.file_size)
        }

        /// Format page dimensions for display
        pub fn format_page_size(&self) -> String {
            format!("{:.0} x {:.0}", self.page_size.0, self.page_size.1)
        }
    }

    /// Rendered page data (stub)
    #[derive(Debug, Clone)]
    pub struct RenderedPage {
        pub width: u32,
        pub height: u32,
        pub pixels: Vec<u8>,
    }

    /// Result of rendering PDF pages (stub)
    #[derive(Debug, Clone)]
    pub struct PdfRenderResult {
        pub pages: Vec<RenderedPage>,
        pub all_rendered: bool,
    }

    /// Get PDF metadata (stub - always returns error)
    pub fn get_pdf_info(_path: &Path) -> Result<PdfInfo, String> {
        Err("PDF support requires the 'pdf' feature to be enabled".to_string())
    }

    /// Render a PDF page (stub - always returns error)
    pub fn render_pdf_page(
        _path: &Path,
        _page_index: usize,
        _scale: f64,
    ) -> Result<(u32, u32, Vec<u8>), String> {
        Err("PDF support requires the 'pdf' feature to be enabled".to_string())
    }

    /// Render PDF pages (stub - always returns error)
    pub fn render_pdf_pages_limited(
        _path: &Path,
        _scale: f64,
        _max_pages: Option<usize>,
    ) -> Result<PdfRenderResult, String> {
        Err("PDF support requires the 'pdf' feature to be enabled".to_string())
    }
}

#[cfg(not(feature = "pdf"))]
pub use stubs::*;

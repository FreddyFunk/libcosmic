//! Centralized MIME type detection for all loaders
//!
//! This module provides consistent MIME-based file type detection used by all
//! loaders in view-core. On Linux, it uses xdg_mime for accurate
//! detection. On other platforms, it falls back to extension-based guessing.

use std::path::Path;

/// MIME type categories for routing to appropriate loaders
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileCategory {
    /// Raster images (JPEG, PNG, WebP, GIF, BMP, TIFF, etc.)
    Image,
    /// Vector graphics (SVG)
    Svg,
    /// Text/code files
    Text,
    /// Directories
    Directory,
    /// Unknown/unsupported file type
    Unknown,
}

/// Result of MIME detection
#[derive(Debug, Clone)]
pub struct MimeInfo {
    /// The file category for loader routing
    pub category: FileCategory,
}

/// Detect the MIME type and category for a file path.
///
/// On Linux, uses xdg_mime for accurate content-based detection.
/// On other platforms, falls back to extension-based guessing.
#[cfg(target_os = "linux")]
pub fn detect_mime(path: &Path) -> MimeInfo {
    // Check if it's a directory first
    if let Ok(metadata) = std::fs::metadata(path) {
        if metadata.is_dir() {
            return MimeInfo {
                category: FileCategory::Directory,
            };
        }
    }

    // Use xdg_mime for detection
    let shared_mime = xdg_mime::SharedMimeInfo::new();

    // Build guess with path and optionally metadata
    let metadata = std::fs::metadata(path).ok();
    let mut builder = shared_mime.guess_mime_type();
    let guess = if let Some(meta) = metadata {
        builder.path(path).metadata(meta).guess()
    } else {
        builder.path(path).guess()
    };
    let mime = guess.mime_type();
    let mime_str = mime.to_string();

    let category = categorize_mime(&mime_str, path);

    MimeInfo { category }
}

/// Non-Linux fallback using extension-based detection
#[cfg(not(target_os = "linux"))]
pub fn detect_mime(path: &Path) -> MimeInfo {
    // Check if it's a directory first
    if let Ok(metadata) = std::fs::metadata(path) {
        if metadata.is_dir() {
            return MimeInfo {
                category: FileCategory::Directory,
            };
        }
    }

    // Use mime_guess crate for extension-based detection
    let mime_str = mime_guess::from_path(path)
        .first()
        .map(|m| m.to_string())
        .unwrap_or_else(|| "application/octet-stream".to_string());

    let category = categorize_mime(&mime_str, path);

    MimeInfo { category }
}

/// Categorize a MIME type string into a FileCategory
fn categorize_mime(mime: &str, path: &Path) -> FileCategory {
    // Check for specific MIME types first
    match mime {
        // SVG - check before generic image
        "image/svg+xml" => return FileCategory::Svg,
        _ => {}
    }

    // Check MIME type prefixes
    if mime.starts_with("image/") {
        return FileCategory::Image;
    }

    if mime.starts_with("text/") {
        return FileCategory::Text;
    }

    // Check for code/config files that might not be detected as text
    if is_code_mime(mime) || is_code_extension(path) {
        return FileCategory::Text;
    }

    FileCategory::Unknown
}

/// Check if MIME type indicates code/config (often detected as application/*)
fn is_code_mime(mime: &str) -> bool {
    matches!(
        mime,
        "application/json"
            | "application/xml"
            | "application/javascript"
            | "application/typescript"
            | "application/x-yaml"
            | "application/toml"
            | "application/x-sh"
            | "application/x-shellscript"
            | "application/x-perl"
            | "application/x-python"
            | "application/x-ruby"
            | "application/x-php"
            | "application/sql"
            | "application/x-httpd-php"
            | "application/xhtml+xml"
            | "application/atom+xml"
            | "application/rss+xml"
            | "application/x-latex"
            | "application/x-tex"
    )
}

/// Check if file extension indicates code/config
fn is_code_extension(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    let Some(ext) = ext else {
        // Check for special filenames without extensions
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        return matches!(
            filename,
            "Makefile" | "makefile" | "GNUmakefile"
                | "Dockerfile" | "dockerfile"
                | "Containerfile"
                | "Vagrantfile"
                | "Gemfile"
                | "Rakefile"
                | "Justfile" | "justfile"
                | "Brewfile"
                | "Procfile"
                | "Pipfile"
                | ".gitignore" | ".gitattributes" | ".gitmodules"
                | ".dockerignore"
                | ".editorconfig"
                | ".env" | ".env.local" | ".env.example"
                | "requirements.txt"
                | "constraints.txt"
                | "CMakeLists.txt"
                | "meson.build"
                | "configure.ac"
                | "configure.in"
        );
    };

    // Programming languages and config files
    let code_extensions = [
        "rs", "c", "h", "cpp", "cxx", "cc", "hpp", "hxx", "hh", "go", "py", "pyi", "pyw", "pyx",
        "pxd", "js", "mjs", "cjs", "jsx", "ts", "tsx", "mts", "cts", "java", "kt", "kts", "cs",
        "fs", "fsx", "swift", "m", "mm", "rb", "erb", "rake", "php", "phtml", "php3", "php4",
        "php5", "phps", "pl", "pm", "pod", "t", "sh", "bash", "zsh", "fish", "ksh", "csh", "tcsh",
        "lua", "r", "rmd", "scala", "sc", "hs", "lhs", "ex", "exs", "erl", "hrl", "clj", "cljs",
        "cljc", "edn", "lisp", "lsp", "cl", "el", "ml", "mli", "zig", "nim", "nims", "v", "cr",
        "dart", "jl", "asm", "s", "html", "htm", "xhtml", "css", "scss", "sass", "less", "vue",
        "svelte", "json", "jsonc", "json5", "yaml", "yml", "toml", "xml", "ini", "cfg", "conf",
        "properties", "env", "md", "markdown", "rst", "txt", "text", "log", "adoc", "asciidoc",
        "gradle", "sbt", "cmake", "make", "mk", "sql", "psql", "mysql", "pgsql", "graphql", "gql",
        "proto", "tf", "tfvars", "nix", "dhall", "diff", "patch",
    ];

    code_extensions.contains(&ext.as_str())
}

/// Get the MIME type string for a file based on its extension and type detection.
///
/// This function returns a MIME type string for recognized file types.
pub fn get_mime_type(path: &Path) -> Option<String> {
    // Check if it's a directory first
    if let Ok(metadata) = std::fs::metadata(path) {
        if metadata.is_dir() {
            return Some("inode/directory".to_string());
        }
    }

    // Use platform-specific MIME detection
    #[cfg(target_os = "linux")]
    {
        let shared_mime = xdg_mime::SharedMimeInfo::new();
        let metadata = std::fs::metadata(path).ok();
        let mut builder = shared_mime.guess_mime_type();
        let guess = if let Some(meta) = metadata {
            builder.path(path).metadata(meta).guess()
        } else {
            builder.path(path).guess()
        };
        Some(guess.mime_type().to_string())
    }

    #[cfg(not(target_os = "linux"))]
    {
        mime_guess::from_path(path)
            .first()
            .map(|m| m.to_string())
    }
}

/// Placeholder for backwards compatibility - always returns false
pub fn is_model_mime(_mime: &str) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_detection() {
        assert_eq!(
            detect_mime(Path::new("test.png")).category,
            FileCategory::Image
        );
        assert_eq!(
            detect_mime(Path::new("test.jpg")).category,
            FileCategory::Image
        );
    }

    #[test]
    fn test_svg_detection() {
        assert_eq!(
            detect_mime(Path::new("test.svg")).category,
            FileCategory::Svg
        );
    }

    #[test]
    fn test_text_detection() {
        assert_eq!(
            detect_mime(Path::new("test.rs")).category,
            FileCategory::Text
        );
        assert_eq!(
            detect_mime(Path::new("test.py")).category,
            FileCategory::Text
        );
        assert_eq!(
            detect_mime(Path::new("Makefile")).category,
            FileCategory::Text
        );
    }
}

//! Text file loading with syntax highlighting
//!
//! This module provides utilities for loading text files with syntax
//! highlighting using syntect and cosmic-text, following the cosmic-edit pattern.

use crate::syntax;
use crate::types::{SyntaxBuffer, TextInfo};
use cosmic::iced::advanced::graphics::text::font_system;
use cosmic_text::{Attrs, Buffer, Family, Metrics, Shaping, Wrap};
use std::path::Path;
use syntect::highlighting::{HighlightIterator, HighlightState, Highlighter};
use syntect::parsing::{ParseState, ScopeStack};

/// Maximum text file size (10 MB) - larger files may cause memory issues
pub const MAX_TEXT_SIZE: u64 = 10 * 1024 * 1024;

/// Default font size for syntax-highlighted text
const FONT_SIZE: f32 = 14.0;

/// Line height multiplier
const LINE_HEIGHT: f32 = 1.4;

/// Load a text file with syntax highlighting.
///
/// Returns a SyntaxBuffer containing a cosmic-text Buffer with per-character
/// syntax highlighting colors applied.
pub fn load_text_file(path: &Path, is_dark_theme: bool) -> Result<(SyntaxBuffer, TextInfo), String> {
    // Get file metadata first for validation
    let metadata = std::fs::metadata(path)
        .map_err(|e| format!("Failed to read file metadata: {}", e))?;

    // Security: Ensure it's a regular file
    if !metadata.is_file() {
        return Err("Path is not a regular file".to_string());
    }

    let file_size = metadata.len();

    // Security: Check file size limit
    if file_size > MAX_TEXT_SIZE {
        return Err(format!(
            "File is too large ({:.1} MB). Maximum allowed is {:.0} MB.",
            file_size as f64 / 1_048_576.0,
            MAX_TEXT_SIZE as f64 / 1_048_576.0
        ));
    }

    // Read file content
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let line_count = content.lines().count();

    // Get the syntax system
    let syntax_system = syntax::syntax_system();

    // Find syntax definition based on extension
    let syntax = path
        .extension()
        .and_then(|ext| ext.to_str())
        .and_then(|ext| syntax_system.syntax_set.find_syntax_by_extension(ext))
        .or_else(|| {
            // Try filename for files without extensions (e.g., Makefile, Dockerfile)
            path.file_name()
                .and_then(|name| name.to_str())
                .and_then(|name| syntax_system.syntax_set.find_syntax_by_extension(name))
        })
        .unwrap_or_else(|| syntax_system.syntax_set.find_syntax_plain_text());

    let syntax_name = syntax.name.clone();

    // Create the highlighted buffer
    let buffer = create_highlighted_buffer(&content, syntax, is_dark_theme)?;

    let syntax_buffer = SyntaxBuffer::new(buffer, content);

    let info = TextInfo {
        path: path.to_path_buf(),
        syntax_name,
        line_count,
        file_size,
    };

    Ok((syntax_buffer, info))
}

/// Create a cosmic-text Buffer with syntax highlighting applied.
fn create_highlighted_buffer(
    content: &str,
    syntax: &syntect::parsing::SyntaxReference,
    is_dark: bool,
) -> Result<Buffer, String> {
    let syntax_system = syntax::syntax_system();
    let theme = syntax::theme_for_mode(is_dark)
        .ok_or_else(|| "Failed to load syntax theme".to_string())?;

    // Get font system - need to get the raw cosmic_text FontSystem from iced's wrapper
    let mut font_system_guard = font_system().write().expect("Failed to lock font system");
    let font_system = font_system_guard.raw();

    let metrics = Metrics::new(FONT_SIZE, FONT_SIZE * LINE_HEIGHT);
    let mut buffer = Buffer::new(font_system, metrics);

    // Default attributes with monospace font
    let default_attrs = Attrs::new()
        .family(Family::Monospace)
        .color(cosmic_text::Color::rgba(200, 200, 200, 255));

    // Set up syntax highlighting
    let highlighter = Highlighter::new(theme);
    let mut parse_state = ParseState::new(syntax);
    let mut highlight_state = HighlightState::new(&highlighter, ScopeStack::new());

    // Build all spans with highlighting
    let lines: Vec<&str> = content.lines().collect();
    let mut all_spans: Vec<(String, Attrs)> = Vec::new();

    for (line_idx, line) in lines.iter().enumerate() {
        if line_idx > 0 {
            // Add newline with default attributes
            all_spans.push(("\n".to_string(), default_attrs.clone()));
        }

        if let Ok(ops) = parse_state.parse_line(line, &syntax_system.syntax_set) {
            let highlights: Vec<_> = HighlightIterator::new(
                &mut highlight_state,
                &ops,
                line,
                &highlighter,
            )
            .collect();

            for (style, text) in highlights {
                let fg = style.foreground;
                let color = cosmic_text::Color::rgba(fg.r, fg.g, fg.b, fg.a);

                let mut attrs = default_attrs.clone().color(color);

                if style.font_style.contains(syntect::highlighting::FontStyle::BOLD) {
                    attrs = attrs.weight(cosmic_text::Weight::BOLD);
                }
                if style.font_style.contains(syntect::highlighting::FontStyle::ITALIC) {
                    attrs = attrs.style(cosmic_text::Style::Italic);
                }

                all_spans.push((text.to_string(), attrs));
            }
        } else {
            // Fallback for parse errors
            all_spans.push((line.to_string(), default_attrs.clone()));
        }
    }

    // Set all the rich text at once
    buffer.set_rich_text(
        font_system,
        all_spans.iter().map(|(t, a)| (t.as_str(), a.clone())),
        &default_attrs,
        Shaping::Advanced,
        None,
    );

    // Disable wrapping (horizontal scroll preferred for code)
    buffer.set_wrap(font_system, Wrap::None);

    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syntax_system_loads() {
        // Verify that the syntax system loads without panicking
        let system = syntax::syntax_system();
        assert!(!system.syntax_set.syntaxes().is_empty());
    }

    #[test]
    fn test_syntax_detection() {
        let system = syntax::syntax_system();

        // Test that we can detect syntax for common file types
        let rust_syntax = system.syntax_set.find_syntax_by_extension("rs");
        assert!(rust_syntax.is_some());
        assert_eq!(rust_syntax.unwrap().name, "Rust");

        let python_syntax = system.syntax_set.find_syntax_by_extension("py");
        assert!(python_syntax.is_some());
        assert_eq!(python_syntax.unwrap().name, "Python");
    }
}

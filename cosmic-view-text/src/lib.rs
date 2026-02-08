//! Text file viewer with syntax highlighting for COSMIC desktop applications.
//!
//! This crate provides the [`TextViewer`] which implements the
//! [`ContentViewer`] trait for displaying text files with syntax highlighting.
//!
//! # Example
//!
//! ```rust,ignore
//! use cosmic_view_text::{TextViewer, TextContent, TextInfo};
//! use cosmic_view_types::{ContentViewer, LoadConfig};
//!
//! let (content, info) = TextViewer::load(path, &LoadConfig::default()).await?;
//! let widget = TextViewer::view(&content, &info, &transform, &config);
//! ```

use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use cosmic::iced::advanced::graphics::text::font_system;
use cosmic::iced::{
    advanced::graphics::text::Raw, Color, Length, Point, Rectangle, Size,
};
use cosmic::iced_core::{
    event, layout, mouse,
    renderer::{self, Quad},
    text::Renderer as TextRenderer,
    widget::tree::{self, Tree},
    Border, Clipboard, Element as IcedElement, Event, Layout, Renderer as CoreRenderer, Shell, Widget,
};
use cosmic::widget;
use cosmic::{Element, Renderer, Theme};
use cosmic_text::{Attrs, Buffer, Family, Metrics, Shaping, SyntaxSystem, Wrap};
use cosmic_view_types::{
    ActionId, ContentViewer, DetailItem, DetailSection, LoadConfig, PreviewKind, PreviewMessage,
    ViewConfig, ViewTransform, ViewerError, format_file_size,
};
use syntect::highlighting::{HighlightIterator, HighlightState, Highlighter};
use syntect::parsing::{ParseState, ScopeStack};

// ============================================================================
// Constants
// ============================================================================

/// Maximum text file size (10 MB)
pub const MAX_TEXT_SIZE: u64 = 10 * 1024 * 1024;

/// Default font size for syntax-highlighted text
const FONT_SIZE: f32 = 14.0;

/// Line height multiplier
const LINE_HEIGHT: f32 = 1.4;

/// Padding around the text content
const PADDING: f32 = 16.0;

// ============================================================================
// Global Syntax System
// ============================================================================

/// Global syntax system singleton
static SYNTAX_SYSTEM: OnceLock<SyntaxSystem> = OnceLock::new();

/// Get the global syntax system, initializing it on first access.
pub fn syntax_system() -> &'static SyntaxSystem {
    SYNTAX_SYSTEM.get_or_init(|| {
        // Load extended themes from two-face
        let lazy_theme_set = two_face::theme::LazyThemeSet::from(two_face::theme::extra());
        let mut theme_set = syntect::highlighting::ThemeSet::from(&lazy_theme_set);

        // Add COSMIC themes (dark and light)
        for (theme_name, theme_data) in &[
            ("COSMIC Dark", cosmic_syntax_theme::COSMIC_DARK_TM_THEME),
            ("COSMIC Light", cosmic_syntax_theme::COSMIC_LIGHT_TM_THEME),
        ] {
            let mut cursor = io::Cursor::new(theme_data);
            match syntect::highlighting::ThemeSet::load_from_reader(&mut cursor) {
                Ok(mut theme) => {
                    // Use transparent background so libcosmic handles container styling
                    theme.settings.background = Some(syntect::highlighting::Color {
                        r: 0,
                        g: 0,
                        b: 0,
                        a: 0,
                    });
                    theme.settings.gutter = Some(syntect::highlighting::Color {
                        r: 0,
                        g: 0,
                        b: 0,
                        a: 0,
                    });
                    theme_set.themes.insert(theme_name.to_string(), theme);
                }
                Err(err) => {
                    tracing::error!("Failed to load {:?} syntax theme: {}", theme_name, err);
                }
            }
        }

        SyntaxSystem {
            syntax_set: two_face::syntax::extra_no_newlines(),
            theme_set,
        }
    })
}

/// Get the theme name for the current mode (dark/light)
pub fn theme_name_for_mode(is_dark: bool) -> &'static str {
    if is_dark {
        "COSMIC Dark"
    } else {
        "COSMIC Light"
    }
}

/// Get the syntect theme for the current mode
pub fn theme_for_mode(is_dark: bool) -> Option<&'static syntect::highlighting::Theme> {
    let system = syntax_system();
    let name = theme_name_for_mode(is_dark);
    system.theme_set.themes.get(name)
}

// ============================================================================
// Types
// ============================================================================

/// Text file metadata and information.
#[derive(Debug, Clone)]
pub struct TextInfo {
    /// Path to the text file
    pub path: PathBuf,
    /// Detected syntax/language name
    pub syntax_name: String,
    /// Number of lines
    pub line_count: usize,
    /// File size in bytes
    pub file_size: u64,
}

impl TextInfo {
    /// Format file size for display (e.g., "1.5 MB")
    pub fn format_file_size(&self) -> String {
        format_file_size(self.file_size)
    }

    /// Format line count for display
    pub fn format_line_count(&self) -> String {
        format!("{} lines", self.line_count)
    }
}

/// Content data for a loaded text file.
#[derive(Clone)]
pub struct TextContent {
    /// The cosmic-text buffer with syntax highlighting
    pub buffer: Arc<Buffer>,
    /// Raw text content (for fallback or re-highlighting on theme change)
    pub content: String,
}

impl std::fmt::Debug for TextContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextContent")
            .field("content_len", &self.content.len())
            .finish_non_exhaustive()
    }
}

// ============================================================================
// TextViewer Implementation
// ============================================================================

/// Viewer for text files with syntax highlighting.
pub struct TextViewer;

impl ContentViewer for TextViewer {
    const KIND: PreviewKind = PreviewKind::Text;

    type Content = TextContent;
    type Info = TextInfo;

    fn can_handle(mime: &str) -> bool {
        mime.starts_with("text/") || mime == "application/json" || mime == "application/xml"
    }

    async fn load(
        path: &Path,
        config: &LoadConfig,
    ) -> Result<(Self::Content, Self::Info), ViewerError> {
        let path = path.to_path_buf();
        let is_dark = config.is_dark_theme;

        // Run loading in blocking task
        let result = tokio::task::spawn_blocking(move || load_text_file_sync(&path, is_dark))
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
        // Use our custom syntax text widget
        let text_widget: SyntaxText<'a> = syntax_text(&content.buffer);

        // Copy background alpha for the closure
        let bg_alpha = config.background_alpha;

        // Wrap in a container with the appropriate background
        widget::container(text_widget)
            .width(cosmic::iced::Length::Fill)
            .height(cosmic::iced::Length::Fill)
            .class(cosmic::style::Container::custom(move |theme| {
                let cosmic_theme = theme.cosmic();
                let mut bg = cosmic_theme.bg_color();
                bg.alpha = bg_alpha;
                cosmic::widget::container::Style {
                    background: Some(Color::from(bg).into()),
                    ..Default::default()
                }
            }))
            .into()
    }

    fn update(_content: &mut Self::Content, _msg: PreviewMessage) {
        // Text files don't have interactive state (scroll is handled by widget)
    }

    fn details(info: &Self::Info) -> Vec<DetailSection> {
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

    fn actions() -> Vec<ActionId> {
        // Text files could support zoom in the future
        vec![]
    }
}

// ============================================================================
// Loading Functions
// ============================================================================

/// Load a text file with syntax highlighting synchronously.
fn load_text_file_sync(path: &Path, is_dark_theme: bool) -> Result<(TextContent, TextInfo), ViewerError> {
    // Get file metadata first for validation
    let metadata = std::fs::metadata(path)
        .map_err(|e| ViewerError(format!("Failed to read file metadata: {}", e)))?;

    // Security: Ensure it's a regular file
    if !metadata.is_file() {
        return Err(ViewerError("Path is not a regular file".to_string()));
    }

    let file_size = metadata.len();

    // Security: Check file size limit
    if file_size > MAX_TEXT_SIZE {
        return Err(ViewerError(format!(
            "File is too large ({:.1} MB). Maximum allowed is {:.0} MB.",
            file_size as f64 / 1_048_576.0,
            MAX_TEXT_SIZE as f64 / 1_048_576.0
        )));
    }

    // Read file content
    let content = std::fs::read_to_string(path)
        .map_err(|e| ViewerError(format!("Failed to read file: {}", e)))?;

    let line_count = content.lines().count();

    // Get the syntax system
    let syntax_system = syntax_system();

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

    let text_content = TextContent {
        buffer: Arc::new(buffer),
        content,
    };

    let info = TextInfo {
        path: path.to_path_buf(),
        syntax_name,
        line_count,
        file_size,
    };

    Ok((text_content, info))
}

/// Create a cosmic-text Buffer with syntax highlighting applied.
fn create_highlighted_buffer(
    content: &str,
    syntax: &syntect::parsing::SyntaxReference,
    is_dark: bool,
) -> Result<Buffer, ViewerError> {
    let syntax_system = syntax_system();
    let theme = theme_for_mode(is_dark)
        .ok_or_else(|| ViewerError("Failed to load syntax theme".to_string()))?;

    // Get font system
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

// ============================================================================
// Syntax Text Widget
// ============================================================================

/// A read-only widget that displays syntax-highlighted text using cosmic-text.
pub struct SyntaxText<'a> {
    /// The text buffer with syntax highlighting applied
    buffer: &'a Arc<Buffer>,
}

impl<'a> SyntaxText<'a> {
    /// Create a new syntax text widget from a highlighted buffer.
    pub fn new(buffer: &'a Arc<Buffer>) -> Self {
        Self { buffer }
    }
}

/// Internal widget state for scrolling
#[derive(Debug, Clone, Default)]
struct SyntaxTextState {
    /// Vertical scroll offset
    scroll_offset: f32,
}

impl<Message> Widget<Message, Theme, Renderer> for SyntaxText<'_>
where
    Message: Clone + 'static,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<SyntaxTextState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(SyntaxTextState::default())
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fill,
            height: Length::Fill,
        }
    }

    fn layout(
        &self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let limits = limits.width(Length::Fill).height(Length::Fill);
        layout::Node::new(limits.max())
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        _shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> event::Status {
        let state = tree.state.downcast_mut::<SyntaxTextState>();
        let bounds = layout.bounds();

        // Handle scroll events when cursor is over widget
        if let Some(_position) = cursor.position_over(bounds) {
            if let Event::Mouse(mouse::Event::WheelScrolled { delta }) = event {
                let scroll_amount = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => y * 30.0,
                    mouse::ScrollDelta::Pixels { y, .. } => y,
                };

                // Get content height from buffer
                let content_height = self.buffer.lines.len() as f32 * FONT_SIZE * LINE_HEIGHT;

                // Calculate max scroll
                let visible_height = bounds.height - PADDING * 2.0;
                let max_scroll = (content_height - visible_height).max(0.0);

                // Update scroll offset
                state.scroll_offset = (state.scroll_offset - scroll_amount).clamp(0.0, max_scroll);

                return event::Status::Captured;
            }
        }

        event::Status::Ignored
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<SyntaxTextState>();
        let bounds = layout.bounds();
        let cosmic_theme = theme.cosmic();

        // Draw background
        let bg_color = Color::from(cosmic_theme.bg_color());
        renderer.fill_quad(
            Quad {
                bounds,
                border: Border::default(),
                ..Default::default()
            },
            bg_color,
        );

        // Calculate content area with padding
        let content_bounds = Rectangle {
            x: bounds.x + PADDING,
            y: bounds.y + PADDING,
            width: bounds.width - PADDING * 2.0,
            height: bounds.height - PADDING * 2.0,
        };

        // Clip to content bounds
        let clip_bounds = content_bounds.intersection(viewport).unwrap_or_default();

        // Calculate position with scroll offset
        let position = Point::new(content_bounds.x, content_bounds.y - state.scroll_offset);

        // Use fill_raw to render the cosmic-text buffer
        renderer.fill_raw(Raw {
            buffer: Arc::downgrade(self.buffer),
            position,
            color: Color::WHITE, // Default color, overridden by per-glyph colors
            clip_bounds,
        });
    }
}

impl<'a, Message> From<SyntaxText<'a>> for IcedElement<'a, Message, Theme, Renderer>
where
    Message: Clone + 'static,
{
    fn from(widget: SyntaxText<'a>) -> Self {
        Self::new(widget)
    }
}

/// Create a new syntax text widget.
pub fn syntax_text(buffer: &Arc<Buffer>) -> SyntaxText<'_> {
    SyntaxText::new(buffer)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syntax_system_loads() {
        let system = syntax_system();
        assert!(!system.syntax_set.syntaxes().is_empty());
    }

    #[test]
    fn test_syntax_detection() {
        let system = syntax_system();

        let rust_syntax = system.syntax_set.find_syntax_by_extension("rs");
        assert!(rust_syntax.is_some());
        assert_eq!(rust_syntax.unwrap().name, "Rust");

        let python_syntax = system.syntax_set.find_syntax_by_extension("py");
        assert!(python_syntax.is_some());
        assert_eq!(python_syntax.unwrap().name, "Python");
    }

    #[test]
    fn test_can_handle() {
        assert!(TextViewer::can_handle("text/plain"));
        assert!(TextViewer::can_handle("text/html"));
        assert!(TextViewer::can_handle("application/json"));
        assert!(!TextViewer::can_handle("image/png"));
    }

    #[test]
    fn test_text_info_format() {
        let info = TextInfo {
            path: PathBuf::from("/tmp/test.rs"),
            syntax_name: "Rust".to_string(),
            line_count: 100,
            file_size: 1024,
        };
        assert_eq!(info.format_line_count(), "100 lines");
        assert_eq!(info.format_file_size(), "1.0 KB");
    }
}

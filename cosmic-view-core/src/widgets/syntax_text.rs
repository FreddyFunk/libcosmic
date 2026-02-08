//! Syntax-highlighted text widget for read-only text preview.
//!
//! This widget uses cosmic-text to render syntax-highlighted text
//! following the cosmic-edit approach.

use cosmic::iced::{
    advanced::graphics::text::Raw,
    Color, Length, Point, Rectangle, Size,
};
use cosmic::iced_core::{
    event, layout, mouse,
    renderer::{self, Quad},
    text::Renderer as TextRenderer,
    widget::tree::{self, Tree},
    Border, Clipboard, Element, Event, Layout, Renderer as CoreRenderer, Shell, Widget,
};
use cosmic::{Renderer, Theme};
use cosmic_text::Buffer;
use std::sync::Arc;

/// Default font size for syntax-highlighted text
const FONT_SIZE: f32 = 14.0;

/// Line height multiplier
const LINE_HEIGHT: f32 = 1.4;

/// Padding around the text content
const PADDING: f32 = 16.0;

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
struct State {
    /// Vertical scroll offset
    scroll_offset: f32,
}

impl<Message> Widget<Message, Theme, Renderer> for SyntaxText<'_>
where
    Message: Clone + 'static,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
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
        let state = tree.state.downcast_mut::<State>();
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
                state.scroll_offset = (state.scroll_offset - scroll_amount)
                    .clamp(0.0, max_scroll);

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
        let state = tree.state.downcast_ref::<State>();
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
        // The buffer contains all color information from syntax highlighting
        renderer.fill_raw(Raw {
            buffer: Arc::downgrade(self.buffer),
            position,
            color: Color::WHITE, // Default color, overridden by per-glyph colors
            clip_bounds,
        });
    }
}

impl<'a, Message> From<SyntaxText<'a>> for Element<'a, Message, Theme, Renderer>
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

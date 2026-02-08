//! Self-animating loading indicator widget.
//!
//! This widget displays "Loading." with animated dots that cycle automatically
//! without requiring any external subscription or message handling.

use cosmic::iced_core::layout;
use cosmic::iced_core::renderer::Style;
use cosmic::iced_core::widget::tree::{self, Tree};
use cosmic::iced_core::{
    self as core, Clipboard, Color, Element, Event, Layout, Length, Rectangle, Shell, Size, Widget,
};
use cosmic::iced_core::{alignment, event, mouse, text};
use cosmic::iced::window::{self, RedrawRequest};
use cosmic::iced::time::Instant;
use cosmic::Renderer;
use cosmic::Theme;

use std::time::Duration;

/// Animation interval for dot cycling (300ms per frame)
const ANIMATION_INTERVAL: Duration = Duration::from_millis(300);

/// A self-animating loading indicator that displays "Loading." with cycling dots.
pub struct LoadingIndicator;

impl LoadingIndicator {
    /// Create a new loading indicator.
    pub fn new() -> Self {
        Self
    }
}

impl Default for LoadingIndicator {
    fn default() -> Self {
        Self::new()
    }
}

/// Internal animation state
#[derive(Debug, Clone)]
struct State {
    /// Current animation frame (0, 1, 2)
    frame: u8,
    /// Last animation update time
    last_update: Instant,
}

impl Default for State {
    fn default() -> Self {
        Self {
            frame: 0,
            last_update: Instant::now(),
        }
    }
}

impl<Message> Widget<Message, Theme, Renderer> for LoadingIndicator
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
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> event::Status {
        let state = tree.state.downcast_mut::<State>();

        if let Event::Window(window::Event::RedrawRequested(now)) = event {
            // Check if enough time has passed to advance animation
            if now.duration_since(state.last_update) >= ANIMATION_INTERVAL {
                state.frame = (state.frame + 1) % 3;
                state.last_update = now;
            }

            // Request next frame for continuous animation
            shell.request_redraw(RedrawRequest::NextFrame);
        }

        event::Status::Ignored
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();

        // Animated dots: . -> .. -> ...
        let dots = match state.frame {
            0 => ".  ",
            1 => ".. ",
            _ => "...",
        };

        let loading_text = format!("Loading{}", dots);

        // Get text color from theme
        let cosmic_theme = theme.cosmic();
        let text_color = Color::from(cosmic_theme.palette.neutral_10);

        // Draw the text centered
        core::text::Renderer::fill_text(
            renderer,
            core::Text {
                content: loading_text,
                size: core::Pixels(18.0),
                line_height: text::LineHeight::Relative(1.3),
                font: cosmic::iced::Font::default(),
                bounds: bounds.size(),
                horizontal_alignment: alignment::Horizontal::Center,
                vertical_alignment: alignment::Vertical::Center,
                shaping: text::Shaping::Advanced,
                wrapping: text::Wrapping::None,
            },
            core::Point {
                x: bounds.center_x(),
                y: bounds.center_y(),
            },
            text_color,
            *viewport,
        );
    }
}

impl<'a, Message> From<LoadingIndicator> for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'static,
{
    fn from(indicator: LoadingIndicator) -> Self {
        Self::new(indicator)
    }
}

/// Create a new loading indicator widget.
pub fn loading_indicator() -> LoadingIndicator {
    LoadingIndicator::new()
}

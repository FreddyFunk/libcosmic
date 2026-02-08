//! Unified Preview API
//!
//! This module provides a high-level API for file preview that allows applications
//! to simply provide a file path and configuration, and receive back everything
//! needed for a complete preview experience: widget, actions, and details.
//!
//! # Usage
//!
//! ```rust,ignore
//! use view_core::{Previewer, PreviewConfig, PreviewState, PreviewMessage};
//!
//! // Load a file
//! let config = PreviewConfig::default();
//! let state = Previewer::load_sync("/path/to/file.jpg", config);
//!
//! // Get the widget for rendering
//! let widget = Previewer::view(&state);
//!
//! // Get available actions for the View menu
//! let actions = Previewer::actions(&state);
//!
//! // Get details for the info drawer
//! let details = Previewer::details(&state);
//! ```

use std::path::Path;

use cosmic::widget;
use cosmic::Element;

// Import viewer crates
#[cfg(feature = "image")]
use cosmic_view_image::ImageViewer;
#[cfg(feature = "text")]
use cosmic_view_text::TextViewer;
#[cfg(feature = "directory")]
use cosmic_view_directory::DirectoryViewer;
#[cfg(feature = "fallback")]
use cosmic_view_fallback::FallbackViewer;
use cosmic_view_types::{ContentViewer, LoadConfig};

use crate::types::{ContentFit, LoadedContent};
use crate::util::mime::detect_mime;
use super::details::generate_details;

// Import types from sub-modules
use super::state::PreviewState;
use super::types::{
    ActionId, ActionState, PreviewAction, PreviewConfig,
    PreviewDetails, PreviewKind, PreviewMessage,
};

// ============================================================================
// Previewer API
// ============================================================================

/// The main preview API.
///
/// This struct provides associated functions for working with previews.
/// It follows the Elm architecture where state is separate from operations.
pub struct Previewer;

impl Previewer {
    // ------------------------------------------------------------------------
    // Loading
    // ------------------------------------------------------------------------

    /// Load a file asynchronously and create preview state.
    ///
    /// This is the preferred method for loading files as it doesn't block the UI.
    ///
    /// # Arguments
    /// * `path` - Path to the file to preview
    /// * `config` - Preview configuration
    ///
    /// # Returns
    /// A `PreviewState` ready for use with other `Previewer` methods.
    pub async fn load(path: impl AsRef<Path>, config: PreviewConfig) -> PreviewState {
        let path = path.as_ref().to_path_buf();

        // Run blocking file I/O on a separate thread to keep UI responsive
        tokio::task::spawn_blocking(move || {
            // Detect file type
            let mime_info = detect_mime(&path);
            let kind = PreviewKind::from(mime_info.category);

            // Load content based on type
            let content = Self::load_content(&path, kind, &config);

            PreviewState::new(path, kind, content, config)
        })
        .await
        .unwrap_or_else(|e| {
            tracing::error!("Failed to load preview: {:?}", e);
            PreviewState::empty()
        })
    }

    /// Internal content loading.
    /// Delegates to viewer crates via ContentViewer trait.
    fn load_content(
        path: &Path,
        kind: PreviewKind,
        config: &PreviewConfig,
    ) -> LoadedContent {
        match kind {
            #[cfg(feature = "image")]
            PreviewKind::Image => Self::load_image(path, config),
            #[cfg(feature = "image")]
            PreviewKind::Svg => Self::load_svg(path, config),
            #[cfg(feature = "text")]
            PreviewKind::Text => Self::load_text(path, config),
            #[cfg(feature = "directory")]
            PreviewKind::Directory => Self::load_folder_content(path),
            #[cfg(feature = "fallback")]
            PreviewKind::Fallback => Self::load_fallback_content(path),
            // When features are disabled, route to fallback
            #[cfg(not(feature = "image"))]
            PreviewKind::Image | PreviewKind::Svg => Self::load_fallback_content(path),
            #[cfg(not(feature = "text"))]
            PreviewKind::Text => Self::load_fallback_content(path),
            #[cfg(not(feature = "directory"))]
            PreviewKind::Directory => Self::load_fallback_content(path),
            #[cfg(not(feature = "fallback"))]
            PreviewKind::Fallback => LoadedContent::Error("Fallback viewer not available".to_string()),
        }
    }

    // Individual loader methods using viewer crates

    fn check_file_size(path: &Path, config: &PreviewConfig) -> Result<(), String> {
        if let Ok(metadata) = std::fs::metadata(path) {
            if metadata.len() > config.max_file_size {
                return Err(format!(
                    "File too large: {} bytes (max {} bytes)",
                    metadata.len(),
                    config.max_file_size
                ));
            }
        }
        Ok(())
    }

    #[cfg(feature = "image")]
    fn load_image(path: &Path, config: &PreviewConfig) -> LoadedContent {
        if let Err(e) = Self::check_file_size(path, config) {
            return LoadedContent::Error(e);
        }

        let load_config = LoadConfig {
            max_file_size: Some(config.max_file_size),
            max_dimension: if config.max_preview_dimension > 0 {
                Some(config.max_preview_dimension)
            } else {
                None
            },
            is_dark_theme: cosmic::theme::active().theme_type.is_dark(),
        };

        // Use ImageViewer from cosmic-view-image (blocking call since we're in spawn_blocking)
        let rt = tokio::runtime::Handle::current();
        match rt.block_on(ImageViewer::load(path, &load_config)) {
            Ok((content, info)) => {
                match content {
                    cosmic_view_image::ImageContent::Raster { handle } => {
                        LoadedContent::Raster { handle, info }
                    }
                    cosmic_view_image::ImageContent::Svg { handle } => {
                        LoadedContent::Svg { handle, info }
                    }
                }
            }
            Err(e) => LoadedContent::Error(format!("Failed to load image: {}", e.0)),
        }
    }

    #[cfg(feature = "image")]
    fn load_svg(path: &Path, config: &PreviewConfig) -> LoadedContent {
        // SVG loading is handled by ImageViewer
        Self::load_image(path, config)
    }

    #[cfg(feature = "text")]
    fn load_text(path: &Path, config: &PreviewConfig) -> LoadedContent {
        if let Err(e) = Self::check_file_size(path, config) {
            return LoadedContent::Error(e);
        }

        let load_config = LoadConfig {
            max_file_size: Some(config.max_file_size),
            max_dimension: None,
            is_dark_theme: cosmic::theme::active().theme_type.is_dark(),
        };

        // Use TextViewer from cosmic-view-text
        let rt = tokio::runtime::Handle::current();
        match rt.block_on(TextViewer::load(path, &load_config)) {
            Ok((content, info)) => LoadedContent::Text { content, info },
            Err(e) => LoadedContent::Error(format!("Failed to load text: {}", e.0)),
        }
    }

    #[cfg(feature = "directory")]
    fn load_folder_content(path: &Path) -> LoadedContent {
        let load_config = LoadConfig::default();

        // Use DirectoryViewer from cosmic-view-directory
        let rt = tokio::runtime::Handle::current();
        match rt.block_on(DirectoryViewer::load(path, &load_config)) {
            Ok((content, info)) => LoadedContent::Folder { content, info },
            Err(e) => LoadedContent::Error(format!("Failed to load folder: {}", e.0)),
        }
    }

    #[cfg(not(feature = "directory"))]
    fn load_folder_content(_path: &Path) -> LoadedContent {
        LoadedContent::Error("Directory viewer not available".to_string())
    }

    #[cfg(feature = "fallback")]
    fn load_fallback_content(path: &Path) -> LoadedContent {
        let load_config = LoadConfig::default();

        // Use FallbackViewer from cosmic-view-fallback
        let rt = tokio::runtime::Handle::current();
        match rt.block_on(FallbackViewer::load(path, &load_config)) {
            Ok((content, info)) => LoadedContent::Fallback { content, info },
            Err(e) => LoadedContent::Error(format!("Failed to load fallback: {}", e.0)),
        }
    }

    #[cfg(not(feature = "fallback"))]
    fn load_fallback_content(_path: &Path) -> LoadedContent {
        LoadedContent::Error("Fallback viewer not available".to_string())
    }

    // ------------------------------------------------------------------------
    // View
    // ------------------------------------------------------------------------

    /// Get the widget for rendering the preview.
    ///
    /// The widget is self-contained and handles mouse interaction internally
    /// (zoom, pan). Apps should embed this widget directly.
    ///
    /// # Arguments
    /// * `state` - The preview state to render
    pub fn view<'a, M: Clone + 'static>(state: &'a PreviewState) -> Element<'a, M> {
        match state.content() {
            LoadedContent::NotLoaded | LoadedContent::Loading => {
                crate::widgets::loading_indicator().into()
            }

            #[cfg(feature = "image")]
            LoadedContent::Raster { handle, .. } => {
                Self::image_view(handle.clone(), state.content_fit())
            }

            #[cfg(feature = "image")]
            LoadedContent::Svg { handle, .. } => {
                Self::svg_view(handle.clone(), state.content_fit())
            }

            #[cfg(feature = "text")]
            LoadedContent::Text { content, .. } => Self::text_view(content),

            #[cfg(feature = "fallback")]
            LoadedContent::Fallback { content, info } => {
                FallbackViewer::view(
                    content,
                    info,
                    state.transform(),
                    &cosmic_view_types::ViewConfig {
                        background_alpha: state.background_alpha(),
                        content_fit: state.content_fit(),
                    },
                )
            }

            #[cfg(feature = "directory")]
            LoadedContent::Folder { content, info } => {
                DirectoryViewer::view(
                    content,
                    info,
                    state.transform(),
                    &cosmic_view_types::ViewConfig {
                        background_alpha: state.background_alpha(),
                        content_fit: state.content_fit(),
                    },
                )
            }

            LoadedContent::Error(msg) => Self::error_view(msg),
        }
    }

    // View helper methods

    #[cfg(feature = "image")]
    fn image_view<'a, M: 'static>(handle: widget::image::Handle, fit: ContentFit) -> Element<'a, M> {
        let content_fit: cosmic::iced::ContentFit = fit.into();
        widget::container(widget::image(handle).content_fit(content_fit))
            .width(cosmic::iced::Length::Fill)
            .height(cosmic::iced::Length::Fill)
            .align_x(cosmic::iced::alignment::Horizontal::Center)
            .align_y(cosmic::iced::alignment::Vertical::Center)
            .into()
    }

    #[cfg(feature = "image")]
    fn svg_view<'a, M: 'static>(handle: widget::svg::Handle, fit: ContentFit) -> Element<'a, M> {
        let content_fit: cosmic::iced::ContentFit = fit.into();
        widget::container(widget::svg(handle).content_fit(content_fit))
            .width(cosmic::iced::Length::Fill)
            .height(cosmic::iced::Length::Fill)
            .align_x(cosmic::iced::alignment::Horizontal::Center)
            .align_y(cosmic::iced::alignment::Vertical::Center)
            .into()
    }

    #[cfg(feature = "text")]
    fn text_view<'a, M: Clone + 'static>(content: &'a cosmic_view_text::TextContent) -> Element<'a, M> {
        cosmic_view_text::syntax_text(&content.buffer).into()
    }

    fn error_view<M: 'static>(message: &str) -> Element<'static, M> {
        let message = message.to_string();
        widget::container(
            widget::column()
                .spacing(8)
                .align_x(cosmic::iced::Alignment::Center)
                .push(widget::text::title4("Error"))
                .push(widget::text::body(message)),
        )
        .width(cosmic::iced::Length::Fill)
        .height(cosmic::iced::Length::Fill)
        .align_x(cosmic::iced::alignment::Horizontal::Center)
        .align_y(cosmic::iced::alignment::Vertical::Center)
        .into()
    }

    // ------------------------------------------------------------------------
    // Update
    // ------------------------------------------------------------------------

    /// Update preview state in response to a message.
    ///
    /// # Arguments
    /// * `state` - The preview state to update (mutable)
    /// * `message` - The message to process
    ///
    /// # Returns
    /// `true` if the state was modified and the view should be redrawn.
    pub fn update(state: &mut PreviewState, message: PreviewMessage) -> bool {
        match message {
            PreviewMessage::ActionTriggered(action_id) => Self::execute_action(state, action_id),
        }
    }

    /// Execute an action directly on the preview state.
    ///
    /// # Returns
    /// `true` if the state was modified.
    pub fn execute_action(state: &mut PreviewState, action: ActionId) -> bool {
        match action {
            ActionId::ZoomIn => {
                state.transform_mut().zoom_in();
                true
            }
            ActionId::ZoomOut => {
                state.transform_mut().zoom_out();
                true
            }
            ActionId::ZoomReset => {
                state.reset_transform();
                true
            }
            ActionId::FitPage => {
                state.reset_transform();
                true
            }
        }
    }

    // ------------------------------------------------------------------------
    // Actions
    // ------------------------------------------------------------------------

    /// Get available actions for the current content.
    ///
    /// Returns actions relevant to the current content type with their current state.
    /// Apps can render these as menu items, toolbar buttons, etc.
    pub fn actions(state: &PreviewState) -> Vec<PreviewAction> {
        let mut actions = Vec::new();

        // Zoom actions for 2D content
        actions.push(PreviewAction {
            id: ActionId::ZoomIn,
            label: "Zoom In".to_string(),
            shortcut: Some("+".to_string()),
            state: ActionState::Trigger,
            icon: Some("zoom-in-symbolic".to_string()),
        });
        actions.push(PreviewAction {
            id: ActionId::ZoomOut,
            label: "Zoom Out".to_string(),
            shortcut: Some("-".to_string()),
            state: ActionState::Trigger,
            icon: Some("zoom-out-symbolic".to_string()),
        });
        actions.push(PreviewAction {
            id: ActionId::ZoomReset,
            label: "Reset Zoom".to_string(),
            shortcut: Some("0".to_string()),
            state: ActionState::Value {
                current: state.transform().format_zoom(),
            },
            icon: Some("zoom-original-symbolic".to_string()),
        });

        // Content-specific actions
        match state.kind() {
            PreviewKind::Image | PreviewKind::Svg | PreviewKind::Text => {
                actions.push(PreviewAction {
                    id: ActionId::FitPage,
                    label: "Fit Page".to_string(),
                    shortcut: Some("F".to_string()),
                    state: ActionState::Trigger,
                    icon: Some("zoom-fit-best-symbolic".to_string()),
                });
            }
            PreviewKind::Directory | PreviewKind::Fallback => {
                // No special actions for directories/fallback
            }
        }

        actions
    }

    // ------------------------------------------------------------------------
    // Details
    // ------------------------------------------------------------------------

    /// Get details about the current content for info drawer display.
    ///
    /// Returns structured detail sections that apps can render in their own style.
    pub fn details(state: &PreviewState) -> PreviewDetails {
        generate_details(state)
    }

    // ------------------------------------------------------------------------
    // Full Resolution Loading (for progressive image loading)
    // ------------------------------------------------------------------------

    /// Check if the current image is a scaled preview that can be upgraded to full resolution.
    ///
    /// Returns true if the image was loaded at reduced resolution and a full-resolution
    /// version can be loaded in the background.
    #[cfg(feature = "image")]
    pub fn is_preview_image(state: &PreviewState) -> bool {
        match state.content() {
            LoadedContent::Raster { info, .. } => info.is_preview,
            _ => false,
        }
    }

    #[cfg(not(feature = "image"))]
    pub fn is_preview_image(_state: &PreviewState) -> bool {
        false
    }

    /// Load full resolution version of a scaled preview image asynchronously.
    ///
    /// Call this after displaying the initial scaled preview to load the full resolution
    /// version in the background. When complete, update the state with the result.
    ///
    /// Returns None if the current content is not a scaled preview image.
    #[cfg(feature = "image")]
    pub async fn load_full_resolution(state: &PreviewState) -> Option<LoadedContent> {
        // Check if this is a scaled preview image
        let (path, base_info) = match state.content() {
            LoadedContent::Raster { info, .. } if info.is_preview => {
                (state.path().to_path_buf(), info.clone())
            }
            _ => return None,
        };

        // Load full resolution on blocking thread
        let result = tokio::task::spawn_blocking(move || {
            Self::load_full_resolution_blocking(&path, base_info)
        })
        .await;

        match result {
            Ok(content) => Some(content),
            Err(e) => {
                tracing::error!("Failed to load full resolution: {:?}", e);
                None
            }
        }
    }

    #[cfg(not(feature = "image"))]
    pub async fn load_full_resolution(_state: &PreviewState) -> Option<LoadedContent> {
        None
    }

    /// Blocking version of full resolution loading (for use in spawn_blocking).
    #[cfg(feature = "image")]
    fn load_full_resolution_blocking(
        path: &std::path::Path,
        base_info: cosmic_view_image::ImageInfo,
    ) -> LoadedContent {
        let load_config = LoadConfig {
            max_file_size: None,
            max_dimension: None, // No scaling - full resolution
            is_dark_theme: false,
        };

        // Use ImageViewer for full resolution load
        let rt = tokio::runtime::Handle::current();
        match rt.block_on(ImageViewer::load(path, &load_config)) {
            Ok((content, mut info)) => {
                // Preserve the original dimensions from base_info
                info.width = base_info.width;
                info.height = base_info.height;
                info.is_preview = false;

                match content {
                    cosmic_view_image::ImageContent::Raster { handle } => {
                        tracing::debug!(
                            "Loaded full resolution: {}x{} for {}",
                            info.displayed_width,
                            info.displayed_height,
                            path.display()
                        );
                        LoadedContent::Raster { handle, info }
                    }
                    cosmic_view_image::ImageContent::Svg { handle } => {
                        LoadedContent::Svg { handle, info }
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to decode full resolution: {}", e.0);
                LoadedContent::Error(format!("Failed to load full resolution: {}", e.0))
            }
        }
    }

    /// Upgrade a preview state with full resolution content.
    ///
    /// Call this when the async `load_full_resolution` completes.
    /// Only upgrades if the path matches to avoid race conditions.
    pub fn upgrade_to_full_resolution(
        state: &mut PreviewState,
        path: &std::path::Path,
        content: LoadedContent,
    ) -> bool {
        // Only upgrade if the path matches (user hasn't navigated away)
        if state.path() != path {
            tracing::debug!(
                "Skipping full resolution upgrade: path changed from {} to {}",
                path.display(),
                state.path().display()
            );
            return false;
        }

        // Only upgrade if current content is a preview
        if !Self::is_preview_image(state) {
            return false;
        }

        // Upgrade the content
        state.content = content;
        true
    }

    // ------------------------------------------------------------------------
    // Thumbnail Rendering (for thumbnail cache generation)
    // ------------------------------------------------------------------------

    /// Render a file to an in-memory RGBA image for thumbnail generation.
    ///
    /// This is a stateless function that loads a file, renders it to pixels,
    /// and returns the result. The output image maintains aspect ratio and
    /// fits within the configured min/max size bounds (as close to max as possible).
    ///
    /// # Arguments
    /// * `path` - Path to the file to render
    /// * `config` - Thumbnail render configuration
    ///
    /// # Returns
    /// * `Ok((width, height, pixels))` - RGBA pixels at the final size
    /// * `Err(error)` - Error description if rendering failed
    pub fn render_thumbnail(
        path: &Path,
        config: &super::types::ThumbnailRenderConfig,
    ) -> Result<(u32, u32, Vec<u8>), String> {
        use crate::util::mime::{detect_mime, FileCategory};

        // Check file size
        let metadata = std::fs::metadata(path)
            .map_err(|e| format!("Failed to read metadata: {}", e))?;

        if !metadata.is_dir() && metadata.len() > config.max_file_size {
            return Err(format!(
                "File too large: {} bytes (max {})",
                metadata.len(),
                config.max_file_size
            ));
        }

        // Detect file type
        let mime_info = detect_mime(path);

        match mime_info.category {
            FileCategory::Image | FileCategory::Svg => Self::render_image_thumbnail(path, config),
            FileCategory::Text | FileCategory::Directory | FileCategory::Unknown => {
                Err("Unsupported file type for thumbnail rendering".to_string())
            }
        }
    }

    /// Render an image (raster or SVG) to thumbnail pixels.
    #[cfg(feature = "image")]
    fn render_image_thumbnail(
        path: &Path,
        config: &super::types::ThumbnailRenderConfig,
    ) -> Result<(u32, u32, Vec<u8>), String> {
        cosmic_view_image::render_thumbnail(path, config.max_size)
    }

    #[cfg(not(feature = "image"))]
    fn render_image_thumbnail(
        _path: &Path,
        _config: &super::types::ThumbnailRenderConfig,
    ) -> Result<(u32, u32, Vec<u8>), String> {
        Err("Image thumbnail support requires the 'image' feature".to_string())
    }
}

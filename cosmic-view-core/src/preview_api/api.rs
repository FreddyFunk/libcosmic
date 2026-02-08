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
//! let widget = Previewer::view(&state, |msg| MyAppMessage::Preview(msg));
//!
//! // Get available actions for the View menu
//! let actions = Previewer::actions(&state);
//!
//! // Get details for the info drawer
//! let details = Previewer::details(&state);
//! ```

use std::path::Path;
#[cfg(feature = "view3d")]
use std::sync::Arc;

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

// PDF loader (not yet moved to its own crate)
#[cfg(feature = "pdf")]
use crate::loaders::pdf::{get_pdf_info, render_pdf_pages_limited};
use crate::loaders::pdf::PdfInfo;

// 3D model support
#[cfg(feature = "view3d")]
use cosmic_view_3d::{load_model, SceneData};

use crate::types::{ContentFit, LoadedContent};
use crate::util::mime::detect_mime;
use super::details::generate_details;

// Type alias for 3D scene - either Arc<SceneData> or () depending on feature
#[cfg(feature = "view3d")]
pub type ModelScene = std::sync::Arc<SceneData>;
#[cfg(not(feature = "view3d"))]
pub type ModelScene = ();

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
            let (content, model_scene) = Self::load_content(&path, kind, &config);

            PreviewState::new(path, kind, content, model_scene, config)
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
    ) -> (LoadedContent, Option<ModelScene>) {
        match kind {
            #[cfg(feature = "image")]
            PreviewKind::Image => Self::load_image(path, config),
            #[cfg(feature = "image")]
            PreviewKind::Svg => Self::load_svg(path, config),
            #[cfg(feature = "text")]
            PreviewKind::Text => Self::load_text(path, config),
            PreviewKind::Pdf => Self::load_pdf(path, config),
            PreviewKind::Model3D => Self::load_model_content(path),
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
            PreviewKind::Fallback => (LoadedContent::Error("Fallback viewer not available".to_string()), None),
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
    fn load_image(path: &Path, config: &PreviewConfig) -> (LoadedContent, Option<ModelScene>) {
        if let Err(e) = Self::check_file_size(path, config) {
            return (LoadedContent::Error(e), None);
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
                        (LoadedContent::Raster { handle, info }, None)
                    }
                    cosmic_view_image::ImageContent::Svg { handle } => {
                        (LoadedContent::Svg { handle, info }, None)
                    }
                }
            }
            Err(e) => (LoadedContent::Error(format!("Failed to load image: {}", e.0)), None),
        }
    }

    #[cfg(feature = "image")]
    fn load_svg(path: &Path, config: &PreviewConfig) -> (LoadedContent, Option<ModelScene>) {
        // SVG loading is handled by ImageViewer
        Self::load_image(path, config)
    }

    #[cfg(feature = "text")]
    fn load_text(path: &Path, config: &PreviewConfig) -> (LoadedContent, Option<ModelScene>) {
        if let Err(e) = Self::check_file_size(path, config) {
            return (LoadedContent::Error(e), None);
        }

        let load_config = LoadConfig {
            max_file_size: Some(config.max_file_size),
            max_dimension: None,
            is_dark_theme: cosmic::theme::active().theme_type.is_dark(),
        };

        // Use TextViewer from cosmic-view-text
        let rt = tokio::runtime::Handle::current();
        match rt.block_on(TextViewer::load(path, &load_config)) {
            Ok((content, info)) => (LoadedContent::Text { content, info }, None),
            Err(e) => (LoadedContent::Error(format!("Failed to load text: {}", e.0)), None),
        }
    }

    #[cfg(feature = "pdf")]
    fn load_pdf(path: &Path, config: &PreviewConfig) -> (LoadedContent, Option<ModelScene>) {
        if let Err(e) = Self::check_file_size(path, config) {
            return (LoadedContent::Error(e), None);
        }

        let info = match get_pdf_info(path) {
            Ok(info) => info,
            Err(e) => {
                return (
                    LoadedContent::Error(format!("Failed to get PDF info: {}", e)),
                    None,
                )
            }
        };

        // Render at 1.5x scale for good quality when displayed full-width
        match render_pdf_pages_limited(path, 1.5, Some(10)) {
            Ok(result) => {
                let pages: Vec<widget::image::Handle> = result
                    .pages
                    .into_iter()
                    .map(|page| {
                        widget::image::Handle::from_rgba(page.width, page.height, page.pixels)
                    })
                    .collect();
                let info = PdfInfo {
                    rendered_pages: pages.len(),
                    all_pages_rendered: result.all_rendered,
                    ..info
                };
                (LoadedContent::Pdf { pages, info }, None)
            }
            Err(e) => (
                LoadedContent::Error(format!("Failed to render PDF: {}", e)),
                None,
            ),
        }
    }

    #[cfg(not(feature = "pdf"))]
    fn load_pdf(path: &Path, _config: &PreviewConfig) -> (LoadedContent, Option<ModelScene>) {
        // PDF feature not enabled, fall through to fallback
        Self::load_fallback_content(path)
    }

    #[cfg(feature = "view3d")]
    fn load_model_content(path: &Path) -> (LoadedContent, Option<ModelScene>) {
        let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

        match load_model(path) {
            Ok(scene) => {
                let info = scene.info(file_size);
                let scene_arc = Arc::new(scene);
                (
                    LoadedContent::Model3D {
                        scene: Box::new((*scene_arc).clone()),
                        info,
                    },
                    Some(scene_arc),
                )
            }
            Err(e) => (
                LoadedContent::Error(format!("Failed to load model: {}", e)),
                None,
            ),
        }
    }

    #[cfg(not(feature = "view3d"))]
    fn load_model_content(path: &Path) -> (LoadedContent, Option<ModelScene>) {
        // 3D model feature not enabled, fall through to fallback
        Self::load_fallback_content(path)
    }

    #[cfg(feature = "directory")]
    fn load_folder_content(path: &Path) -> (LoadedContent, Option<ModelScene>) {
        let load_config = LoadConfig::default();

        // Use DirectoryViewer from cosmic-view-directory
        let rt = tokio::runtime::Handle::current();
        match rt.block_on(DirectoryViewer::load(path, &load_config)) {
            Ok((content, info)) => (LoadedContent::Folder { content, info }, None),
            Err(e) => (LoadedContent::Error(format!("Failed to load folder: {}", e.0)), None),
        }
    }

    #[cfg(not(feature = "directory"))]
    fn load_folder_content(_path: &Path) -> (LoadedContent, Option<ModelScene>) {
        (LoadedContent::Error("Directory viewer not available".to_string()), None)
    }

    #[cfg(feature = "fallback")]
    fn load_fallback_content(path: &Path) -> (LoadedContent, Option<ModelScene>) {
        let load_config = LoadConfig::default();

        // Use FallbackViewer from cosmic-view-fallback
        let rt = tokio::runtime::Handle::current();
        match rt.block_on(FallbackViewer::load(path, &load_config)) {
            Ok((content, info)) => (LoadedContent::Fallback { content, info }, None),
            Err(e) => (LoadedContent::Error(format!("Failed to load fallback: {}", e.0)), None),
        }
    }

    #[cfg(not(feature = "fallback"))]
    fn load_fallback_content(_path: &Path) -> (LoadedContent, Option<ModelScene>) {
        (LoadedContent::Error("Fallback viewer not available".to_string()), None)
    }

    // ------------------------------------------------------------------------
    // View
    // ------------------------------------------------------------------------

    /// Get the widget for rendering the preview.
    ///
    /// The widget is self-contained and handles mouse interaction internally
    /// (zoom, pan, 3D rotation). Apps should embed this widget directly.
    ///
    /// # Type Parameters
    /// * `M` - The app's message type
    ///
    /// # Arguments
    /// * `state` - The preview state to render
    /// * `map_message` - Function to map `PreviewMessage` to the app's message type
    pub fn view<'a, M: Clone + 'static>(
        state: &'a PreviewState,
        _map_message: impl Fn(PreviewMessage) -> M + 'a,
    ) -> Element<'a, M> {
        let bg_alpha = state.background_alpha();

        match state.content() {
            LoadedContent::NotLoaded | LoadedContent::Loading => {
                crate::widgets::loading_indicator().into()
            }

            #[cfg(feature = "image")]
            LoadedContent::Raster { handle, .. } => {
                Self::image_view(handle.clone(), state.content_fit(), bg_alpha)
            }

            #[cfg(feature = "image")]
            LoadedContent::Svg { handle, .. } => {
                Self::svg_view(handle.clone(), state.content_fit(), bg_alpha)
            }

            #[cfg(feature = "text")]
            LoadedContent::Text { content, .. } => Self::text_view(content, bg_alpha),

            LoadedContent::Pdf { pages, info } => Self::pdf_view(pages, info, bg_alpha),

            #[cfg(feature = "view3d")]
            LoadedContent::Model3D { .. } => {
                if let Some(scene) = state.model_scene() {
                    Self::model_view(scene.clone(), state.model_config(), bg_alpha)
                } else {
                    Self::error_view("Failed to initialize 3D scene")
                }
            }

            #[cfg(feature = "fallback")]
            LoadedContent::Fallback { content, info } => {
                FallbackViewer::view(
                    content,
                    info,
                    state.transform(),
                    &cosmic_view_types::ViewConfig {
                        background_alpha: bg_alpha,
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
                        background_alpha: bg_alpha,
                        content_fit: state.content_fit(),
                    },
                )
            }

            LoadedContent::Error(msg) => Self::error_view(msg),
        }
    }

    // View helper methods

    #[cfg(feature = "image")]
    fn image_view<'a, M: 'static>(
        handle: widget::image::Handle,
        fit: ContentFit,
        _bg_alpha: f32,
    ) -> Element<'a, M> {
        let content_fit: cosmic::iced::ContentFit = fit.into();
        widget::container(widget::image(handle).content_fit(content_fit))
            .width(cosmic::iced::Length::Fill)
            .height(cosmic::iced::Length::Fill)
            .align_x(cosmic::iced::alignment::Horizontal::Center)
            .align_y(cosmic::iced::alignment::Vertical::Center)
            .into()
    }

    #[cfg(feature = "image")]
    fn svg_view<'a, M: 'static>(
        handle: widget::svg::Handle,
        fit: ContentFit,
        _bg_alpha: f32,
    ) -> Element<'a, M> {
        let content_fit: cosmic::iced::ContentFit = fit.into();
        widget::container(widget::svg(handle).content_fit(content_fit))
            .width(cosmic::iced::Length::Fill)
            .height(cosmic::iced::Length::Fill)
            .align_x(cosmic::iced::alignment::Horizontal::Center)
            .align_y(cosmic::iced::alignment::Vertical::Center)
            .into()
    }

    #[cfg(feature = "text")]
    fn text_view<'a, M: Clone + 'static>(content: &'a cosmic_view_text::TextContent, _bg_alpha: f32) -> Element<'a, M> {
        // Use the syntax-highlighted text widget from cosmic-view-text
        cosmic_view_text::syntax_text(&content.buffer).into()
    }

    fn pdf_view<'a, M: 'static>(
        pages: &[widget::image::Handle],
        _info: &PdfInfo,
        _bg_alpha: f32,
    ) -> Element<'a, M> {
        // Show all pages in a scrollable view, centered horizontally
        let mut column = widget::column()
            .spacing(16)
            .align_x(cosmic::iced::Alignment::Center);

        for page in pages {
            // Each page scales to fit width while maintaining aspect ratio
            // Wrap in centered container so it's centered when window is wider than PDF
            let image = widget::container(
                widget::image(page.clone())
                    .content_fit(cosmic::iced::ContentFit::ScaleDown),
            )
            .width(cosmic::iced::Length::Fill)
            .align_x(cosmic::iced::alignment::Horizontal::Center);

            column = column.push(image);
        }

        // Wrap column in a centered container inside the scrollable
        widget::scrollable(
            widget::container(column)
                .width(cosmic::iced::Length::Fill)
                .align_x(cosmic::iced::alignment::Horizontal::Center),
        )
        .width(cosmic::iced::Length::Fill)
        .height(cosmic::iced::Length::Fill)
        .into()
    }

    #[cfg(feature = "view3d")]
    fn model_view<'a, M: Clone + 'static>(
        scene: std::sync::Arc<SceneData>,
        config: &cosmic_view_3d::Model3DViewerConfig,
        bg_alpha: f32,
    ) -> Element<'a, M> {
        let theme = cosmic::theme::active();
        let cosmic_theme = theme.cosmic();
        let bg = cosmic_theme.bg_color();
        let accent = cosmic_theme.accent_color();

        // When bg_alpha < 1.0 (gallery/overlay context), use transparent background (0.0)
        // so the gallery container's overlay shows through. The container provides the
        // consistent background; we just render the 3D content on top.
        // When bg_alpha = 1.0 (standalone/full view), use the theme's background color.
        let effective_alpha = if bg_alpha < 1.0 { 0.0 } else { bg_alpha };

        let themed_config = cosmic_view_3d::Model3DViewerConfig {
            background_color: [bg.red, bg.green, bg.blue, effective_alpha],
            object_color: [accent.red, accent.green, accent.blue],
            show_textures: config.show_textures,
            show_mesh: config.show_mesh,
            show_wireframe: config.show_wireframe,
        };

        cosmic_view_3d::model_viewer(scene, Some(themed_config))
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
            PreviewMessage::Loaded => false,
            PreviewMessage::LoadError(_) => false,
            PreviewMessage::OpenFile(_) => false, // App should handle this
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
            #[cfg(feature = "view3d")]
            ActionId::ToggleTextures => {
                state.model_config_mut().show_textures = !state.model_config().show_textures;
                true
            }
            #[cfg(feature = "view3d")]
            ActionId::ToggleMesh => {
                state.model_config_mut().show_mesh = !state.model_config().show_mesh;
                true
            }
            #[cfg(feature = "view3d")]
            ActionId::ToggleWireframe => {
                state.model_config_mut().show_wireframe = !state.model_config().show_wireframe;
                true
            }
            #[cfg(feature = "view3d")]
            ActionId::ResetCamera => {
                // Camera reset is handled internally by the 3D widget
                false
            }
            // When view3d is disabled, these actions do nothing
            #[cfg(not(feature = "view3d"))]
            ActionId::ToggleTextures | ActionId::ToggleMesh | ActionId::ToggleWireframe | ActionId::ResetCamera => {
                false
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

        // Zoom actions for 2D content (3D has its own camera, PDF has no zoom for now)
        if state.kind() != PreviewKind::Model3D && state.kind() != PreviewKind::Pdf {
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
        }

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
            PreviewKind::Pdf => {
                // PDF has no zoom actions for now
            }
            #[cfg(feature = "view3d")]
            PreviewKind::Model3D => {
                actions.push(PreviewAction {
                    id: ActionId::ToggleTextures,
                    label: "Show Textures".to_string(),
                    shortcut: Some("T".to_string()),
                    state: ActionState::Toggle {
                        enabled: state.model_config().show_textures,
                    },
                    icon: None,
                });
                actions.push(PreviewAction {
                    id: ActionId::ToggleMesh,
                    label: "Show Mesh".to_string(),
                    shortcut: Some("M".to_string()),
                    state: ActionState::Toggle {
                        enabled: state.model_config().show_mesh,
                    },
                    icon: None,
                });
                actions.push(PreviewAction {
                    id: ActionId::ToggleWireframe,
                    label: "Show Wireframe".to_string(),
                    shortcut: Some("W".to_string()),
                    state: ActionState::Toggle {
                        enabled: state.model_config().show_wireframe,
                    },
                    icon: None,
                });
            }
            #[cfg(not(feature = "view3d"))]
            PreviewKind::Model3D => {
                // Model3D actions not available when view3d feature is disabled
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
    // Convenience accessors (delegate to PreviewState methods)
    // ------------------------------------------------------------------------

    /// Get the file path being previewed.
    pub fn path(state: &PreviewState) -> &std::path::Path {
        state.path()
    }

    /// Get the kind of content being previewed.
    pub fn kind(state: &PreviewState) -> PreviewKind {
        state.kind()
    }

    /// Check if content is still loading.
    pub fn is_loading(state: &PreviewState) -> bool {
        state.is_loading()
    }

    /// Get error message if loading failed.
    pub fn error(state: &PreviewState) -> Option<&str> {
        state.error()
    }

    /// Get the current content fit mode.
    pub fn content_fit(state: &PreviewState) -> ContentFit {
        state.content_fit()
    }

    /// Get the current view transform.
    pub fn transform(state: &PreviewState) -> &crate::types::ViewTransform {
        state.transform()
    }

    /// Get the 3D model configuration (if applicable).
    #[cfg(feature = "view3d")]
    pub fn model_config(state: &PreviewState) -> &cosmic_view_3d::Model3DViewerConfig {
        state.model_config()
    }

    /// Set the background alpha.
    pub fn set_background_alpha(state: &mut PreviewState, alpha: f32) {
        state.set_background_alpha(alpha);
    }

    /// Get access to the 3D scene data (if applicable).
    #[cfg(feature = "view3d")]
    pub fn model_scene(state: &PreviewState) -> Option<&Arc<SceneData>> {
        state.model_scene()
    }

    /// Get access to the loaded content.
    pub fn content(state: &PreviewState) -> &LoadedContent {
        state.content()
    }

    /// Create an empty/not-loaded preview state.
    pub fn empty() -> PreviewState {
        PreviewState::empty()
    }

    /// Check if any content is currently loaded (not NotLoaded or Loading).
    pub fn has_content(state: &PreviewState) -> bool {
        state.has_content()
    }

    /// Set the content directly (for use by loading handlers).
    pub fn set_content(
        state: &mut PreviewState,
        path: std::path::PathBuf,
        content: LoadedContent,
    ) {
        state.set_content(path, content);
    }

    /// Set the state to loading.
    pub fn set_loading(state: &mut PreviewState) {
        state.set_loading();
    }

    /// Set an error state.
    pub fn set_error(state: &mut PreviewState, error: String) {
        state.set_error(error);
    }

    /// Reset the view transform to default.
    pub fn reset_transform(state: &mut PreviewState) {
        state.reset_transform();
    }

    /// Set the 3D model scene data.
    #[cfg(feature = "view3d")]
    pub fn set_model_scene(state: &mut PreviewState, scene: Arc<SceneData>) {
        state.set_model_scene(scene);
    }

    /// Clear the 3D model scene data.
    #[cfg(feature = "view3d")]
    pub fn clear_model_scene(state: &mut PreviewState) {
        state.clear_model_scene();
    }

    /// Set the current path being previewed (without changing content).
    pub fn set_path(state: &mut PreviewState, path: std::path::PathBuf) {
        state.set_path(path);
    }

    /// Get a mutable reference to the model configuration.
    #[cfg(feature = "view3d")]
    pub fn model_config_mut(state: &mut PreviewState) -> &mut cosmic_view_3d::Model3DViewerConfig {
        state.model_config_mut()
    }

    /// Get a mutable reference to the content fit mode.
    pub fn content_fit_mut(state: &mut PreviewState) -> &mut ContentFit {
        state.content_fit_mut()
    }

    /// Get a mutable reference to the view transform.
    pub fn transform_mut(state: &mut PreviewState) -> &mut crate::types::ViewTransform {
        state.transform_mut()
    }

    /// Get the PDF info (if available).
    pub fn pdf_info(state: &PreviewState) -> Option<&PdfInfo> {
        state.pdf_info()
    }

    /// Get a mutable reference to PDF info (if available).
    pub fn pdf_info_mut(state: &mut PreviewState) -> Option<&mut PdfInfo> {
        state.pdf_info_mut()
    }

    /// Set the PDF info.
    pub fn set_pdf_info(state: &mut PreviewState, info: Option<PdfInfo>) {
        state.set_pdf_info(info);
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
            FileCategory::Image => Self::render_image_thumbnail(path, config),
            FileCategory::Svg => Self::render_svg_thumbnail(path, config),
            FileCategory::Pdf => Self::render_pdf_thumbnail(path, config),
            FileCategory::Model3D => Self::render_model_thumbnail(path, config),
            FileCategory::Text | FileCategory::Directory | FileCategory::Unknown => {
                Err("Unsupported file type for thumbnail rendering".to_string())
            }
        }
    }

    /// Render a raster image to thumbnail pixels.
    #[cfg(feature = "image")]
    fn render_image_thumbnail(
        path: &Path,
        config: &super::types::ThumbnailRenderConfig,
    ) -> Result<(u32, u32, Vec<u8>), String> {
        // Use cosmic-view-image's decode function
        let load_config = LoadConfig {
            max_file_size: Some(config.max_file_size),
            max_dimension: None, // Load full for thumbnail generation
            is_dark_theme: false,
        };

        let rt = tokio::runtime::Handle::current();
        let (content, info) = rt.block_on(ImageViewer::load(path, &load_config))
            .map_err(|e| format!("Failed to decode image: {}", e.0))?;

        // Get the raw pixels from the handle
        match content {
            cosmic_view_image::ImageContent::Raster { handle } => {
                // Create a thumbnail from the image data
                let (orig_w, orig_h) = (info.displayed_width, info.displayed_height);
                let (target_w, target_h) = Self::calculate_thumbnail_size(orig_w, orig_h, config);

                // For thumbnail, we need to resize. The handle contains the pixel data
                // but we need to access it differently. Let's use the image crate directly.
                // Note: This is a simplification - in practice we'd extract from the handle
                if let cosmic::widget::image::Handle::Rgba { width, height, pixels, .. } = handle {
                    if let Some(img) = image::RgbaImage::from_raw(width, height, pixels.to_vec()) {
                        let img = image::DynamicImage::ImageRgba8(img);
                        let thumbnail = img.thumbnail(target_w, target_h).into_rgba8();
                        return Ok((thumbnail.width(), thumbnail.height(), thumbnail.into_raw()));
                    }
                }
                Err("Failed to extract pixels from image handle".to_string())
            }
            cosmic_view_image::ImageContent::Svg { .. } => {
                // SVG should be handled separately
                Self::render_svg_thumbnail(path, config)
            }
        }
    }

    #[cfg(not(feature = "image"))]
    fn render_image_thumbnail(
        _path: &Path,
        _config: &super::types::ThumbnailRenderConfig,
    ) -> Result<(u32, u32, Vec<u8>), String> {
        Err("Image thumbnail support requires the 'image' feature".to_string())
    }

    /// Render an SVG to thumbnail pixels.
    fn render_svg_thumbnail(
        path: &Path,
        config: &super::types::ThumbnailRenderConfig,
    ) -> Result<(u32, u32, Vec<u8>), String> {
        // For SVG, we use resvg to render to pixels
        let data = std::fs::read(path)
            .map_err(|e| format!("Failed to read SVG: {}", e))?;

        let options = usvg::Options::default();
        let tree = usvg::Tree::from_data(&data, &options)
            .map_err(|e| format!("Failed to parse SVG: {}", e))?;

        let size = tree.size();
        let (orig_w, orig_h) = (size.width() as u32, size.height() as u32);
        let (target_w, target_h) = Self::calculate_thumbnail_size(orig_w.max(1), orig_h.max(1), config);

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

    /// Render a PDF first page to thumbnail pixels.
    #[cfg(feature = "pdf")]
    fn render_pdf_thumbnail(
        path: &Path,
        config: &super::types::ThumbnailRenderConfig,
    ) -> Result<(u32, u32, Vec<u8>), String> {
        use crate::loaders::pdf::render_pdf_page;

        // Render first page at 0.5 scale, then resize if needed
        let (width, height, pixels) = render_pdf_page(path, 0, 0.5)?;

        // If within bounds, return as-is
        if width <= config.max_size && height <= config.max_size
            && width >= config.min_size && height >= config.min_size
        {
            return Ok((width, height, pixels));
        }

        // Resize to fit bounds
        let img = image::RgbaImage::from_raw(width, height, pixels)
            .ok_or_else(|| "Failed to create image from PDF pixels".to_string())?;
        let img = image::DynamicImage::ImageRgba8(img);

        let (target_w, target_h) = Self::calculate_thumbnail_size(width, height, config);
        let thumbnail = img.thumbnail(target_w, target_h).into_rgba8();
        Ok((thumbnail.width(), thumbnail.height(), thumbnail.into_raw()))
    }

    #[cfg(not(feature = "pdf"))]
    fn render_pdf_thumbnail(
        _path: &Path,
        _config: &super::types::ThumbnailRenderConfig,
    ) -> Result<(u32, u32, Vec<u8>), String> {
        Err("PDF thumbnail support requires the 'pdf' feature".to_string())
    }

    /// Render a 3D model to thumbnail pixels.
    #[cfg(feature = "view3d")]
    fn render_model_thumbnail(
        path: &Path,
        config: &super::types::ThumbnailRenderConfig,
    ) -> Result<(u32, u32, Vec<u8>), String> {
        let scene = cosmic_view_3d::load_model(path)
            .map_err(|e| format!("Failed to load 3D model: {}", e))?;

        // Choose background based on themed setting
        let bg_color = if config.themed {
            let theme = cosmic::theme::active();
            let cosmic_theme = theme.cosmic();
            let bg = cosmic_theme.bg_color();
            [bg.red, bg.green, bg.blue, 1.0]
        } else {
            // Neutral dark gray background for theme-independent thumbnails
            [0.1, 0.1, 0.12, 1.0]
        };

        cosmic_view_3d::render_model_thumbnail(&scene, config.max_size, config.max_size, bg_color)
    }

    #[cfg(not(feature = "view3d"))]
    fn render_model_thumbnail(
        _path: &Path,
        _config: &super::types::ThumbnailRenderConfig,
    ) -> Result<(u32, u32, Vec<u8>), String> {
        Err("3D model thumbnail support requires the 'view3d' feature".to_string())
    }

    /// Calculate thumbnail dimensions maintaining aspect ratio within bounds.
    fn calculate_thumbnail_size(
        orig_w: u32,
        orig_h: u32,
        config: &super::types::ThumbnailRenderConfig,
    ) -> (u32, u32) {
        let aspect = orig_w as f32 / orig_h as f32;
        let max = config.max_size as f32;

        let (w, h) = if aspect > 1.0 {
            // Wider than tall
            (max, max / aspect)
        } else {
            // Taller than wide (or square)
            (max * aspect, max)
        };

        // Ensure we're at least min_size on the smaller dimension
        let min = config.min_size as f32;
        let (w, h) = if w < min && h < min {
            // Both too small, scale up
            if aspect > 1.0 {
                (min * aspect, min)
            } else {
                (min, min / aspect)
            }
        } else {
            (w, h)
        };

        (w.round() as u32, h.round() as u32)
    }
}

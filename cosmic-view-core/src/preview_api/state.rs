//! Preview state management.
//!
//! This module contains the `PreviewState` struct and all accessor/setter methods.

use std::path::{Path, PathBuf};
#[cfg(feature = "view3d")]
use std::sync::Arc;

use crate::loaders::pdf::PdfInfo;
#[cfg(feature = "view3d")]
use cosmic_view_3d::{Model3DViewerConfig, SceneData};
use crate::types::{ContentFit, LoadedContent, ViewTransform};
use super::types::{PreviewConfig, PreviewKind};
use super::api::ModelScene;

// ============================================================================
// Stub types for when view3d feature is disabled
// ============================================================================

/// Stub for Model3DViewerConfig when view3d is not enabled.
#[cfg(not(feature = "view3d"))]
#[derive(Debug, Clone, Default)]
pub struct Model3DViewerConfigStub {
    pub show_textures: bool,
    pub show_mesh: bool,
    pub show_wireframe: bool,
}

// ============================================================================
// Preview State
// ============================================================================

/// Opaque preview state that apps pass around but don't inspect.
///
/// The state is internally managed and updated by the preview system.
/// Apps should:
/// 1. Create it via `Previewer::load()` (async)
/// 2. Pass it to `Previewer::view()` for rendering
/// 3. Pass it to `Previewer::update()` when handling messages
/// 4. Query it via `Previewer::actions()` and `Previewer::details()`
#[derive(Debug, Clone)]
pub struct PreviewState {
    /// File path being previewed
    pub(crate) path: PathBuf,
    /// Kind of content detected
    pub(crate) kind: PreviewKind,
    /// Internal content state
    pub(crate) content: LoadedContent,
    /// View transform for zoomable content
    pub(crate) transform: ViewTransform,
    /// Content fit mode
    pub(crate) content_fit: ContentFit,
    /// 3D model configuration (only when view3d feature enabled)
    #[cfg(feature = "view3d")]
    pub(crate) model_config: Model3DViewerConfig,
    /// 3D scene data (shared with widget, only when view3d feature enabled)
    #[cfg(feature = "view3d")]
    pub(crate) model_scene: Option<Arc<SceneData>>,
    /// PDF info (stored separately during loading before content is set)
    pub(crate) pdf_info: Option<PdfInfo>,
    /// Configuration used to create this preview
    pub(crate) config: PreviewConfig,
}

impl PreviewState {
    /// Create a new preview state with the given parameters.
    #[cfg(feature = "view3d")]
    pub(crate) fn new(
        path: PathBuf,
        kind: PreviewKind,
        content: LoadedContent,
        model_scene: Option<ModelScene>,
        config: PreviewConfig,
    ) -> Self {
        Self {
            path,
            kind,
            content,
            transform: ViewTransform::default(),
            content_fit: ContentFit::Contain,
            model_config: Model3DViewerConfig::default(),
            model_scene,
            pdf_info: None,
            config,
        }
    }

    /// Create a new preview state with the given parameters (no view3d).
    #[cfg(not(feature = "view3d"))]
    pub(crate) fn new(
        path: PathBuf,
        kind: PreviewKind,
        content: LoadedContent,
        _model_scene: Option<ModelScene>,
        config: PreviewConfig,
    ) -> Self {
        Self {
            path,
            kind,
            content,
            transform: ViewTransform::default(),
            content_fit: ContentFit::Contain,
            pdf_info: None,
            config,
        }
    }

    /// Create an empty/not-loaded preview state.
    #[cfg(feature = "view3d")]
    pub fn empty() -> Self {
        Self {
            path: PathBuf::new(),
            kind: PreviewKind::Fallback,
            content: LoadedContent::NotLoaded,
            transform: ViewTransform::default(),
            content_fit: ContentFit::Contain,
            model_config: Model3DViewerConfig::default(),
            model_scene: None,
            pdf_info: None,
            config: PreviewConfig::default(),
        }
    }

    /// Create an empty/not-loaded preview state (no view3d).
    #[cfg(not(feature = "view3d"))]
    pub fn empty() -> Self {
        Self {
            path: PathBuf::new(),
            kind: PreviewKind::Fallback,
            content: LoadedContent::NotLoaded,
            transform: ViewTransform::default(),
            content_fit: ContentFit::Contain,
            pdf_info: None,
            config: PreviewConfig::default(),
        }
    }
}

// ============================================================================
// Accessors (implemented on PreviewState for convenience)
// ============================================================================

impl PreviewState {
    /// Get the file path being previewed.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the kind of content being previewed.
    pub fn kind(&self) -> PreviewKind {
        self.kind
    }

    /// Check if content is still loading.
    pub fn is_loading(&self) -> bool {
        matches!(self.content, LoadedContent::Loading)
    }

    /// Get error message if loading failed.
    pub fn error(&self) -> Option<&str> {
        match &self.content {
            LoadedContent::Error(msg) => Some(msg),
            _ => None,
        }
    }

    /// Get the current content fit mode.
    pub fn content_fit(&self) -> ContentFit {
        self.content_fit
    }

    /// Get the current view transform.
    pub fn transform(&self) -> &ViewTransform {
        &self.transform
    }

    /// Get the 3D model configuration.
    #[cfg(feature = "view3d")]
    pub fn model_config(&self) -> &Model3DViewerConfig {
        &self.model_config
    }

    /// Get access to the 3D scene data (if applicable).
    #[cfg(feature = "view3d")]
    pub fn model_scene(&self) -> Option<&Arc<SceneData>> {
        self.model_scene.as_ref()
    }

    /// Get access to the loaded content.
    pub fn content(&self) -> &LoadedContent {
        &self.content
    }

    /// Check if any content is currently loaded (not NotLoaded or Loading).
    pub fn has_content(&self) -> bool {
        self.content.is_loaded()
    }

    /// Get the PDF info (if available).
    pub fn pdf_info(&self) -> Option<&PdfInfo> {
        self.pdf_info.as_ref()
    }

    /// Get the background alpha from config.
    pub fn background_alpha(&self) -> f32 {
        self.config.background_alpha
    }

}

// ============================================================================
// Setters (implemented on PreviewState for convenience)
// ============================================================================

impl PreviewState {
    /// Set the background alpha.
    pub fn set_background_alpha(&mut self, alpha: f32) {
        self.config.background_alpha = alpha;
    }

    /// Set the content directly (for use by loading handlers).
    /// Also updates the path and infers the PreviewKind from the content.
    pub fn set_content(&mut self, path: PathBuf, content: LoadedContent) {
        self.path = path;
        self.content = content;
        // Infer kind from content
        self.kind = match &self.content {
            LoadedContent::Raster { .. } => PreviewKind::Image,
            LoadedContent::Svg { .. } => PreviewKind::Svg,
            LoadedContent::Text { .. } => PreviewKind::Text,
            LoadedContent::Pdf { .. } => PreviewKind::Pdf,
            #[cfg(feature = "view3d")]
            LoadedContent::Model3D { .. } => PreviewKind::Model3D,
            LoadedContent::Fallback { .. } => PreviewKind::Fallback,
            LoadedContent::Folder { .. } => PreviewKind::Directory,
            LoadedContent::NotLoaded | LoadedContent::Loading | LoadedContent::Error(_) => {
                self.kind // Keep existing kind
            }
        };
    }

    /// Set the state to loading.
    pub fn set_loading(&mut self) {
        self.content = LoadedContent::Loading;
    }

    /// Set an error state.
    pub fn set_error(&mut self, error: String) {
        self.content = LoadedContent::Error(error);
    }

    /// Reset the view transform to default.
    pub fn reset_transform(&mut self) {
        self.transform = ViewTransform::default();
    }

    /// Set the 3D model scene data.
    #[cfg(feature = "view3d")]
    pub fn set_model_scene(&mut self, scene: Arc<SceneData>) {
        self.model_scene = Some(scene);
    }

    /// Clear the 3D model scene data.
    #[cfg(feature = "view3d")]
    pub fn clear_model_scene(&mut self) {
        self.model_scene = None;
    }

    /// Set the current path being previewed (without changing content).
    pub fn set_path(&mut self, path: PathBuf) {
        self.path = path;
    }

    /// Get a mutable reference to the model configuration.
    #[cfg(feature = "view3d")]
    pub fn model_config_mut(&mut self) -> &mut Model3DViewerConfig {
        &mut self.model_config
    }

    /// Stub: Get the 3D model configuration (always returns default when view3d disabled).
    #[cfg(not(feature = "view3d"))]
    pub fn model_config(&self) -> Model3DViewerConfigStub {
        Model3DViewerConfigStub::default()
    }

    /// Stub: Get access to the 3D scene data (always None when view3d disabled).
    #[cfg(not(feature = "view3d"))]
    pub fn model_scene(&self) -> Option<()> {
        None
    }

    /// Stub: Set the 3D model scene data (no-op when view3d disabled).
    #[cfg(not(feature = "view3d"))]
    pub fn set_model_scene(&mut self, _scene: ()) {}

    /// Stub: Clear the 3D model scene data (no-op when view3d disabled).
    #[cfg(not(feature = "view3d"))]
    pub fn clear_model_scene(&mut self) {}

    /// Stub: Get a mutable reference to the model configuration (no-op when view3d disabled).
    #[cfg(not(feature = "view3d"))]
    pub fn model_config_mut(&mut self) -> Model3DViewerConfigStub {
        Model3DViewerConfigStub::default()
    }

    /// Get a mutable reference to the content fit mode.
    pub fn content_fit_mut(&mut self) -> &mut ContentFit {
        &mut self.content_fit
    }

    /// Get a mutable reference to the view transform.
    pub fn transform_mut(&mut self) -> &mut ViewTransform {
        &mut self.transform
    }

    /// Get a mutable reference to PDF info (if available).
    pub fn pdf_info_mut(&mut self) -> Option<&mut PdfInfo> {
        self.pdf_info.as_mut()
    }

    /// Set the PDF info.
    pub fn set_pdf_info(&mut self, info: Option<PdfInfo>) {
        self.pdf_info = info;
    }
}

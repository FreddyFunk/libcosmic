//! Preview state management.
//!
//! This module contains the `PreviewState` struct and all accessor/setter methods.

use std::path::{Path, PathBuf};

use crate::types::{ContentFit, LoadedContent, ViewTransform};
use super::types::{PreviewConfig, PreviewKind};

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
    /// Configuration used to create this preview
    pub(crate) config: PreviewConfig,
}

impl PreviewState {
    /// Create a new preview state with the given parameters.
    pub(crate) fn new(
        path: PathBuf,
        kind: PreviewKind,
        content: LoadedContent,
        config: PreviewConfig,
    ) -> Self {
        Self {
            path,
            kind,
            content,
            transform: ViewTransform::default(),
            content_fit: ContentFit::Contain,
            config,
        }
    }

    /// Create an empty/not-loaded preview state.
    pub fn empty() -> Self {
        Self {
            path: PathBuf::new(),
            kind: PreviewKind::Fallback,
            content: LoadedContent::NotLoaded,
            transform: ViewTransform::default(),
            content_fit: ContentFit::Contain,
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

    /// Get access to the loaded content.
    pub fn content(&self) -> &LoadedContent {
        &self.content
    }

    /// Check if any content is currently loaded (not NotLoaded or Loading).
    pub fn has_content(&self) -> bool {
        self.content.is_loaded()
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
            #[cfg(feature = "image")]
            LoadedContent::Raster { .. } => PreviewKind::Image,
            #[cfg(feature = "image")]
            LoadedContent::Svg { .. } => PreviewKind::Svg,
            #[cfg(feature = "text")]
            LoadedContent::Text { .. } => PreviewKind::Text,
            #[cfg(feature = "fallback")]
            LoadedContent::Fallback { .. } => PreviewKind::Fallback,
            #[cfg(feature = "directory")]
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

    /// Set the current path being previewed (without changing content).
    pub fn set_path(&mut self, path: PathBuf) {
        self.path = path;
    }

    /// Get a mutable reference to the content fit mode.
    pub fn content_fit_mut(&mut self) -> &mut ContentFit {
        &mut self.content_fit
    }

    /// Get a mutable reference to the view transform.
    pub fn transform_mut(&mut self) -> &mut ViewTransform {
        &mut self.transform
    }
}

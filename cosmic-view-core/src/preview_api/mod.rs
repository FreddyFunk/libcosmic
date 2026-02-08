//! Unified preview API for file viewing.

pub mod api;
pub mod details;
pub mod state;
pub mod types;

// Re-export all public types at module level
pub use api::Previewer;
pub use state::PreviewState;
pub use types::{
    ActionId, ActionState, DetailItem, DetailSection, PreviewAction, PreviewConfig,
    PreviewDetails, PreviewKind, PreviewMessage, ThumbnailRenderConfig,
};

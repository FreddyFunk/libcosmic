//! Unified preview API for file viewing.

pub mod api;
pub mod details;
pub mod state;
pub mod types;

// Re-export all public types at module level
pub use api::Previewer;
pub use state::PreviewState;

// These re-exports are part of the public API even if not used locally
#[allow(unused_imports)]
pub use types::{
    ActionId, ActionState, DetailItem, DetailSection, PreviewAction, PreviewConfig,
    PreviewDetails, PreviewKind, PreviewMessage, ThumbnailRenderConfig,
};

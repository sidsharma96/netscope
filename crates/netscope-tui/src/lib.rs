//! Deterministic view state and rendering, with terminal I/O kept at the edge.
/// Real-terminal setup, input, and restoration.
pub mod terminal;
mod view;
pub use view::{
    Action, App, Screen, draw, inspector_lines, protocol_lines, redirect_lines, retry_lines,
};

mod host_view;

mod replay_view;
pub use replay_view::ReplayEditor;

mod comparison_view;

/// Local saved-request file operations.
pub mod saved_file;

mod text_edit;

/// Text boundaries differ from byte positions and screen columns.
#[doc = include_str!("../../../docs/chapters/21-moving-through-text.md")]
pub mod text_editing_lesson {}

mod sse_view;

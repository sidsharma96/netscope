//! Deterministic view state and rendering, with terminal I/O kept at the edge.
mod journey_view;
/// Real-terminal setup, input, and restoration.
pub mod terminal;
mod view;
mod workspace_view;
pub use view::{
    Action, App, Screen, draw, inspector_lines, protocol_lines, redirect_lines, retry_lines,
};
pub use workspace_view::InspectorTab;

mod host_view;

mod replay_view;
pub use replay_view::ReplayEditor;

mod comparison_view;

/// Local saved-request file operations.
pub mod saved_file;

mod text_edit;

mod sse_view;

/// Offline packet and directed stream inspection.
pub mod packet_view;

mod presentation;

mod packet_learning;

mod stream_view;

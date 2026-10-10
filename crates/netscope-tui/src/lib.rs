//! Deterministic view state and rendering, with terminal I/O kept at the edge.
/// Real-terminal setup, input, and restoration.
pub mod terminal;
mod view;
pub use view::{Action, App, Screen, draw, inspector_lines, redirect_lines, retry_lines};

mod host_view;

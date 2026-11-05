//! UI-agnostic heart of Sheen.
//!
//! This crate knows nothing about Windows, Tauri or the webview. Platform
//! services (SMTC, Core Audio, fullscreen detection, ...) translate what they
//! observe into [`SheenEvent`]s; the [`Island`] reducer folds those events into
//! a single [`Presentation`] that any front-end can render, and that the window
//! layer uses for hit-testing.
//!
//! Keeping this layer pure means:
//! * the behaviour of the island (priorities, timeouts, expand/collapse) is
//!   unit-testable on any OS, and
//! * the UI shell (currently Tauri + Svelte) can be replaced without touching
//!   the logic.

pub mod eta;
pub mod event;
pub mod geometry;
pub mod hotkey;
pub mod island;
pub mod transfers;

pub use event::{
    MediaCommand, MediaInfo, OsdKind, SheenEvent, ShelfItem, TransferKind, TransferSample,
    TransferUnit,
};
pub use geometry::{Geometry, HitRegion, Rect, CANVAS_HEIGHT, CANVAS_WIDTH, TOP_MARGIN};
pub use hotkey::Accelerator;
pub use island::{Island, IslandConfig, Mode, Presentation};
pub use transfers::{Transfer, TransferState};

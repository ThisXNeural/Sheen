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

pub mod event;
pub mod geometry;
pub mod island;

pub use event::{MediaCommand, MediaInfo, OsdKind, SheenEvent};
pub use geometry::{Geometry, HitRegion, Rect, CANVAS_HEIGHT, CANVAS_WIDTH, TOP_MARGIN};
pub use island::{Island, IslandConfig, Mode, Presentation};

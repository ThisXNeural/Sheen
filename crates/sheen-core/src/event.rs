use serde::{Deserialize, Serialize};

/// Everything that can change what the island shows.
///
/// Platform services and the UI both speak this language; the [`crate::Island`]
/// reducer is the only thing that interprets it.
#[derive(Debug, Clone, PartialEq)]
pub enum SheenEvent {
    /// The active SMTC session changed or updated. `None` means no session.
    Media(Option<MediaInfo>),
    /// A transient system notification (volume, brightness, mic, ...).
    Osd(OsdKind),
    /// A full-screen app (game, video, presentation) gained/lost the foreground.
    Fullscreen(bool),
    /// The pointer entered/left the physical capsule (reported by the hit-tester).
    Hover(bool),
    /// The user asked to open the expanded card (click on the compact capsule).
    ExpandRequested,
    /// The user asked to close the expanded card.
    CollapseRequested,
}

/// Snapshot of the current media session, as exposed by Windows SMTC.
///
/// Artwork is deliberately *not* part of this struct: it is large, changes
/// rarely and is irrelevant to the island's logic, so the platform layer ships
/// it to the UI on a separate channel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfo {
    /// AppUserModelID of the source app (e.g. `Spotify.exe`, `Chrome`).
    pub source_app: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub playing: bool,
    /// Timeline position when it was sampled, if the source reports one.
    pub position_ms: Option<u64>,
    pub duration_ms: Option<u64>,
    /// Unix time (ms) at which `position_ms` was sampled. The UI interpolates
    /// from here while `playing` is true, so no per-frame IPC is needed.
    pub sampled_at_ms: u64,
}

/// Transient notifications that replace the stock Windows flyouts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum OsdKind {
    /// `level` is 0.0..=1.0.
    Volume {
        level: f32,
        muted: bool,
    },
    /// `level` is 0.0..=1.0.
    Brightness {
        level: f32,
    },
    MicMute {
        muted: bool,
    },
    CapsLock {
        on: bool,
    },
    DeviceBattery {
        name: String,
        percent: u8,
    },
}

/// Commands the UI can send back to the media session.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum MediaCommand {
    TogglePlayPause,
    Next,
    Previous,
    Seek { position_ms: u64 },
}

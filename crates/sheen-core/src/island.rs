//! The island reducer: folds [`SheenEvent`]s into a [`Presentation`].
//!
//! Priority, highest first:
//! 1. **Hidden** — a full-screen app is in the foreground.
//! 2. **Expanded** — the user explicitly opened the media card. An OSD arriving
//!    now is shown *inside* the card instead of collapsing it.
//! 3. **Osd** — a transient system notification, for `osd_duration`.
//! 4. **MediaCompact** — something is playing / paused in SMTC.
//! 5. **Idle** — nothing to show (or Hidden, if the idle pill is disabled).

use crate::event::{MediaInfo, OsdKind, SheenEvent};
use crate::geometry::Geometry;
use serde::Serialize;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct IslandConfig {
    /// How long an OSD stays on screen after the *last* related event.
    pub osd_duration: Duration,
    /// Grace period before the expanded card collapses once the pointer leaves.
    pub collapse_delay: Duration,
    /// Show a small pill when there is nothing else to display.
    pub show_idle_pill: bool,
}

impl Default for IslandConfig {
    fn default() -> Self {
        Self {
            osd_duration: Duration::from_millis(1600),
            collapse_delay: Duration::from_millis(700),
            show_idle_pill: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    Hidden,
    Idle,
    MediaCompact,
    Osd,
    Expanded,
}

/// Everything the UI needs to render one frame of the island.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Presentation {
    pub mode: Mode,
    pub geometry: Geometry,
    /// Pointer is over the capsule (the UI may use it for subtle affordances).
    pub hovering: bool,
    pub media: Option<MediaInfo>,
    /// Active OSD. In `Osd` mode it is the main content; in `Expanded` mode it
    /// is rendered as an inline overlay.
    pub osd: Option<OsdKind>,
}

#[derive(Debug, Clone)]
struct ActiveOsd {
    kind: OsdKind,
    until: Instant,
}

#[derive(Debug, Clone)]
pub struct Island {
    config: IslandConfig,
    media: Option<MediaInfo>,
    osd: Option<ActiveOsd>,
    fullscreen: bool,
    hovering: bool,
    expanded: bool,
    collapse_at: Option<Instant>,
}

impl Island {
    pub fn new(config: IslandConfig) -> Self {
        Self {
            config,
            media: None,
            osd: None,
            fullscreen: false,
            hovering: false,
            expanded: false,
            collapse_at: None,
        }
    }

    /// Apply one event. Returns `true` if the presentation may have changed.
    pub fn handle(&mut self, event: SheenEvent, now: Instant) -> bool {
        let before = self.present();
        match event {
            SheenEvent::Media(media) => {
                if media.is_none() {
                    self.collapse();
                }
                self.media = media;
            }
            SheenEvent::Osd(kind) => {
                self.osd = Some(ActiveOsd {
                    kind,
                    until: now + self.config.osd_duration,
                });
            }
            SheenEvent::Fullscreen(on) => {
                self.fullscreen = on;
                if on {
                    self.collapse();
                }
            }
            SheenEvent::Hover(on) => {
                self.hovering = on;
                if on {
                    self.collapse_at = None;
                } else if self.expanded {
                    self.collapse_at = Some(now + self.config.collapse_delay);
                }
            }
            SheenEvent::ExpandRequested => {
                if self.media.is_some() && !self.fullscreen {
                    self.expanded = true;
                    self.collapse_at = None;
                }
            }
            SheenEvent::CollapseRequested => self.collapse(),
        }
        self.expire(now);
        self.present() != before
    }

    /// Advance timers. Returns `true` if the presentation may have changed.
    pub fn tick(&mut self, now: Instant) -> bool {
        let before = self.present();
        self.expire(now);
        self.present() != before
    }

    /// The next instant at which [`Island::tick`] would change something.
    pub fn next_deadline(&self) -> Option<Instant> {
        let osd = self.osd.as_ref().map(|o| o.until);
        match (osd, self.collapse_at) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }

    pub fn present(&self) -> Presentation {
        let osd = self.osd.as_ref().map(|o| o.kind.clone());
        let mode = if self.fullscreen {
            Mode::Hidden
        } else if self.expanded && self.media.is_some() {
            Mode::Expanded
        } else if osd.is_some() {
            Mode::Osd
        } else if self.media.is_some() {
            Mode::MediaCompact
        } else if self.config.show_idle_pill {
            Mode::Idle
        } else {
            Mode::Hidden
        };

        let geometry = match mode {
            Mode::Hidden => Geometry::HIDDEN,
            Mode::Idle => Geometry::IDLE,
            Mode::MediaCompact if self.hovering => Geometry::MEDIA_PEEK,
            Mode::MediaCompact => Geometry::MEDIA_COMPACT,
            Mode::Osd => Geometry::OSD,
            Mode::Expanded => Geometry::EXPANDED,
        };

        Presentation {
            mode,
            geometry,
            hovering: self.hovering && mode != Mode::Hidden,
            media: self.media.clone(),
            osd: if mode == Mode::Hidden { None } else { osd },
        }
    }

    fn collapse(&mut self) {
        self.expanded = false;
        self.collapse_at = None;
    }

    fn expire(&mut self, now: Instant) {
        if self.osd.as_ref().is_some_and(|o| now >= o.until) {
            self.osd = None;
        }
        if self.collapse_at.is_some_and(|t| now >= t) {
            self.collapse();
        }
    }
}

impl Default for Island {
    fn default() -> Self {
        Self::new(IslandConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn song() -> MediaInfo {
        MediaInfo {
            source_app: "Spotify.exe".into(),
            title: "Song".into(),
            artist: "Artist".into(),
            album: "Album".into(),
            playing: true,
            position_ms: Some(1_000),
            duration_ms: Some(200_000),
            sampled_at_ms: 0,
        }
    }

    fn volume(level: f32) -> SheenEvent {
        SheenEvent::Osd(OsdKind::Volume {
            level,
            muted: false,
        })
    }

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn starts_idle_and_shows_media() {
        let t = Instant::now();
        let mut island = Island::default();
        assert_eq!(island.present().mode, Mode::Idle);

        assert!(island.handle(SheenEvent::Media(Some(song())), t));
        assert_eq!(island.present().mode, Mode::MediaCompact);
    }

    #[test]
    fn idle_pill_can_be_disabled() {
        let island = Island::new(IslandConfig {
            show_idle_pill: false,
            ..Default::default()
        });
        assert_eq!(island.present().mode, Mode::Hidden);
    }

    #[test]
    fn osd_overrides_compact_then_expires() {
        let t = Instant::now();
        let mut island = Island::default();
        island.handle(SheenEvent::Media(Some(song())), t);
        island.handle(volume(0.4), t);
        assert_eq!(island.present().mode, Mode::Osd);
        assert_eq!(island.next_deadline(), Some(t + ms(1600)));

        // A second volume event extends the OSD rather than stacking.
        island.handle(volume(0.5), t + ms(1000));
        assert!(!island.tick(t + ms(1700)));
        assert_eq!(island.present().mode, Mode::Osd);

        assert!(island.tick(t + ms(2600)));
        assert_eq!(island.present().mode, Mode::MediaCompact);
        assert_eq!(island.next_deadline(), None);
    }

    #[test]
    fn osd_is_inline_while_expanded() {
        let t = Instant::now();
        let mut island = Island::default();
        island.handle(SheenEvent::Media(Some(song())), t);
        island.handle(SheenEvent::ExpandRequested, t);
        island.handle(volume(0.7), t);

        let p = island.present();
        assert_eq!(p.mode, Mode::Expanded);
        assert_eq!(p.geometry, Geometry::EXPANDED);
        assert!(matches!(p.osd, Some(OsdKind::Volume { .. })));
    }

    #[test]
    fn cannot_expand_without_media() {
        let mut island = Island::default();
        island.handle(SheenEvent::ExpandRequested, Instant::now());
        assert_eq!(island.present().mode, Mode::Idle);
    }

    #[test]
    fn expanded_collapses_after_pointer_leaves() {
        let t = Instant::now();
        let mut island = Island::default();
        island.handle(SheenEvent::Media(Some(song())), t);
        island.handle(SheenEvent::Hover(true), t);
        island.handle(SheenEvent::ExpandRequested, t);
        island.handle(SheenEvent::Hover(false), t + ms(100));
        assert_eq!(island.next_deadline(), Some(t + ms(800)));

        // Coming back within the grace period cancels the collapse.
        island.handle(SheenEvent::Hover(true), t + ms(500));
        assert!(!island.tick(t + ms(900)));
        assert_eq!(island.present().mode, Mode::Expanded);

        island.handle(SheenEvent::Hover(false), t + ms(1000));
        assert!(island.tick(t + ms(1700)));
        assert_eq!(island.present().mode, Mode::MediaCompact);
    }

    #[test]
    fn fullscreen_hides_everything_and_collapses() {
        let t = Instant::now();
        let mut island = Island::default();
        island.handle(SheenEvent::Media(Some(song())), t);
        island.handle(SheenEvent::ExpandRequested, t);
        island.handle(SheenEvent::Fullscreen(true), t);
        island.handle(volume(0.2), t);

        let p = island.present();
        assert_eq!(p.mode, Mode::Hidden);
        assert_eq!(p.geometry, Geometry::HIDDEN);
        assert!(p.osd.is_none());

        island.handle(SheenEvent::Fullscreen(false), t + ms(10));
        // The card does not pop back open on its own.
        assert_eq!(island.present().mode, Mode::Osd);
    }

    #[test]
    fn losing_media_collapses_card() {
        let t = Instant::now();
        let mut island = Island::default();
        island.handle(SheenEvent::Media(Some(song())), t);
        island.handle(SheenEvent::ExpandRequested, t);
        island.handle(SheenEvent::Media(None), t);
        assert_eq!(island.present().mode, Mode::Idle);

        island.handle(SheenEvent::Media(Some(song())), t);
        assert_eq!(island.present().mode, Mode::MediaCompact);
    }

    #[test]
    fn hover_peeks_compact_capsule() {
        let t = Instant::now();
        let mut island = Island::default();
        island.handle(SheenEvent::Media(Some(song())), t);
        island.handle(SheenEvent::Hover(true), t);
        assert_eq!(island.present().geometry, Geometry::MEDIA_PEEK);
    }

    #[test]
    fn presentation_serializes_camel_case() {
        let mut island = Island::default();
        island.handle(volume(0.25), Instant::now());
        let json = serde_json::to_value(island.present()).unwrap();
        assert_eq!(json["mode"], "osd");
        assert_eq!(json["osd"]["kind"], "volume");
        assert_eq!(json["geometry"]["width"], Geometry::OSD.width);
    }
}

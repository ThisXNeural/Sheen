//! The island reducer: folds [`SheenEvent`]s into a [`Presentation`].
//!
//! Priority, highest first:
//! 1. **Hidden**: a full-screen app is in the foreground.
//! 2. **Shelf**: a file drag is in progress (the island becomes a drop
//!    target), or the user opened the shelf.
//! 3. **Expanded**: the user opened the media card. An OSD arriving now is
//!    shown *inside* the card instead of collapsing it.
//! 4. **Osd**: a transient notification (volume, clipboard, ...).
//! 5. **MediaCompact**: something is playing or paused. Background transfers
//!    show as a hairline progress bar on it.
//! 6. **Transfer**: a download or copy whose source window is out of view.
//! 7. **Idle**: nothing to show (or Hidden, if the idle pill is disabled).

use crate::event::{MediaInfo, OsdKind, SheenEvent, ShelfItem};
use crate::geometry::{Geometry, Rect, MAX_SCALE, MIN_SCALE, REVEAL, SIDE};
use crate::transfers::{Transfer, TransferTracker};
use serde::Serialize;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct IslandConfig {
    /// How long an OSD stays on screen after the *last* related event.
    pub osd_duration: Duration,
    /// An open panel closes this long after the pointer leaves (a click
    /// anywhere else closes it at once).
    pub collapse_delay: Duration,
    /// Show a small pill when there is nothing else to display.
    pub show_idle_pill: bool,
}

impl Default for IslandConfig {
    fn default() -> Self {
        Self {
            osd_duration: Duration::from_millis(1600),
            collapse_delay: Duration::from_secs(4),
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
    Transfer,
    Osd,
    Expanded,
    Shelf,
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
    /// Transfers to show (empty when hidden).
    pub transfers: Vec<Transfer>,
    pub shelf: Vec<ShelfItem>,
    /// A file drag is in progress: the shelf is acting as a drop target.
    pub dragging: bool,
    /// Show the small round shelf button beside the capsule.
    pub shelf_badge: bool,
    /// Size multiplier; the UI scales about the top centre.
    pub scale: f64,
}

#[derive(Debug, Clone)]
struct ActiveOsd {
    kind: OsdKind,
    until: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Panel {
    None,
    Media,
    Shelf,
}

#[derive(Debug, Clone)]
pub struct Island {
    config: IslandConfig,
    media: Option<MediaInfo>,
    osd: Option<ActiveOsd>,
    fullscreen: bool,
    hovering: bool,
    panel: Panel,
    collapse_at: Option<Instant>,
    transfers: TransferTracker,
    file_drag: bool,
    shelf: Vec<ShelfItem>,
    /// Hidden with the close button until the pointer comes back.
    dismissed: bool,
    scale: f64,
}

impl Island {
    pub fn new(config: IslandConfig) -> Self {
        Self {
            config,
            media: None,
            osd: None,
            fullscreen: false,
            hovering: false,
            panel: Panel::None,
            collapse_at: None,
            transfers: TransferTracker::default(),
            file_drag: false,
            shelf: Vec::new(),
            dismissed: false,
            scale: 1.0,
        }
    }

    /// Apply one event. Returns `true` if the presentation may have changed.
    pub fn handle(&mut self, event: SheenEvent, now: Instant) -> bool {
        let before = self.present();
        match event {
            SheenEvent::Media(media) => {
                if media.is_none() && self.panel == Panel::Media {
                    self.collapse();
                }
                self.media = media;
            }
            SheenEvent::Osd(kind) => {
                self.osd = Some(ActiveOsd { kind, until: now + self.config.osd_duration });
            }
            SheenEvent::Fullscreen(on) => {
                self.fullscreen = on;
                if on {
                    self.collapse();
                }
            }
            SheenEvent::Hover(on) => {
                self.hovering = on;
                if on && self.dismissed {
                    self.dismissed = false;
                }
                if on {
                    self.collapse_at = None;
                } else if self.panel != Panel::None {
                    self.collapse_at = Some(now + self.config.collapse_delay);
                }
            }
            SheenEvent::ExpandRequested => {
                if self.media.is_some() && !self.fullscreen {
                    self.open(Panel::Media);
                }
            }
            SheenEvent::ShelfRequested => {
                if !self.fullscreen {
                    self.open(Panel::Shelf);
                }
            }
            SheenEvent::CollapseRequested => self.collapse(),
            SheenEvent::Transfers(samples) => self.transfers.update(samples, now),
            SheenEvent::FileDrag(on) => {
                // A drag that ends over the island was a drop onto the shelf:
                // keep it open so the user sees what landed.
                if !on && self.file_drag && self.hovering {
                    self.open(Panel::Shelf);
                }
                self.file_drag = on;
            }
            SheenEvent::Shelf(items) => self.shelf = items,
            SheenEvent::OutsideClick => {
                if self.panel != Panel::None && !self.file_drag {
                    self.collapse();
                }
            }
            SheenEvent::Dismiss => {
                self.dismissed = true;
                self.collapse();
            }
            SheenEvent::SetScale(s) => {
                if s.is_finite() {
                    self.scale = s.clamp(MIN_SCALE, MAX_SCALE);
                }
            }
        }
        self.expire(now);
        self.present() != before
    }

    /// Advance timers. Returns `true` if the presentation may have changed.
    pub fn tick(&mut self, now: Instant) -> bool {
        let before = self.present();
        self.expire(now);
        self.transfers.tick(now);
        self.present() != before
    }

    /// The next instant at which [`Island::tick`] would change something.
    pub fn next_deadline(&self) -> Option<Instant> {
        [self.osd.as_ref().map(|o| o.until), self.collapse_at, self.transfers.next_deadline()]
            .into_iter()
            .flatten()
            .min()
    }

    pub fn present(&self) -> Presentation {
        let osd = self.osd.as_ref().map(|o| o.kind.clone());
        let transfers = self.transfers.visible();
        let mode = if self.fullscreen || (self.dismissed && !self.file_drag) {
            Mode::Hidden
        } else if self.file_drag || self.panel == Panel::Shelf {
            Mode::Shelf
        } else if self.panel == Panel::Media && self.media.is_some() {
            Mode::Expanded
        } else if osd.is_some() {
            Mode::Osd
        } else if self.media.is_some() {
            Mode::MediaCompact
        } else if !transfers.is_empty() {
            Mode::Transfer
        } else if self.config.show_idle_pill {
            Mode::Idle
        } else {
            Mode::Hidden
        };

        let geometry = match mode {
            Mode::Hidden => Geometry::HIDDEN,
            Mode::Idle => Geometry::IDLE,
            Mode::MediaCompact if self.hovering => Geometry::MEDIA_PEEK,
            Mode::MediaCompact if !transfers.is_empty() => Geometry::MEDIA_COMPACT_TRANSFER,
            Mode::MediaCompact => Geometry::MEDIA_COMPACT,
            Mode::Transfer if self.hovering => Geometry::TRANSFER_PEEK,
            Mode::Transfer => Geometry::TRANSFER,
            Mode::Osd if matches!(osd, Some(OsdKind::Clipboard { .. })) => Geometry::OSD_TALL,
            Mode::Osd => Geometry::OSD,
            Mode::Expanded => Geometry::EXPANDED,
            Mode::Shelf => Geometry::SHELF,
        };

        let hidden = mode == Mode::Hidden;
        Presentation {
            mode,
            geometry,
            hovering: self.hovering && !hidden,
            media: self.media.clone(),
            osd: if hidden { None } else { osd },
            transfers: if hidden { Vec::new() } else { transfers },
            shelf: self.shelf.clone(),
            dragging: self.file_drag && !hidden,
            shelf_badge: !hidden && mode != Mode::Shelf && !self.shelf.is_empty(),
            scale: self.scale,
        }
    }

    /// The part of the canvas that takes the mouse, in unscaled canvas
    /// coordinates times the scale (i.e. what the hit-tester compares with).
    pub fn hit_rect(&self) -> Rect {
        let p = self.present();
        let rect = if self.fullscreen {
            Rect::default()
        } else if p.mode == Mode::Hidden {
            // Dismissed: a thin strip where the island was brings it back.
            if self.dismissed {
                REVEAL
            } else {
                Rect::default()
            }
        } else if p.hovering || p.shelf_badge {
            // Room for the close button and the shelf badge.
            p.geometry.rect().expand_x(SIDE)
        } else {
            p.geometry.rect()
        };
        if rect.is_empty() {
            rect
        } else {
            rect.scaled(self.scale)
        }
    }

    /// Whether a panel is open, so clicks elsewhere should be watched.
    pub fn panel_open(&self) -> bool {
        self.panel != Panel::None
    }

    fn open(&mut self, panel: Panel) {
        self.panel = panel;
        // Opened without the pointer on it (e.g. a drop that just ended
        // elsewhere): close again unless the user comes over.
        self.collapse_at = None;
    }

    fn collapse(&mut self) {
        self.panel = Panel::None;
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
        SheenEvent::Osd(OsdKind::Volume { level, muted: false })
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
        let island = Island::new(IslandConfig { show_idle_pill: false, ..Default::default() });
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
        assert_eq!(island.next_deadline(), Some(t + ms(4100)));

        // Coming back within the grace period cancels the collapse.
        island.handle(SheenEvent::Hover(true), t + ms(500));
        assert!(!island.tick(t + ms(4200)));
        assert_eq!(island.present().mode, Mode::Expanded);

        island.handle(SheenEvent::Hover(false), t + ms(5000));
        assert!(island.tick(t + ms(9000)));
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

    fn download(id: u64, done: u64, in_view: bool) -> crate::TransferSample {
        crate::TransferSample {
            id,
            kind: crate::TransferKind::Download,
            app: "Chrome".into(),
            name: "big.iso".into(),
            done,
            total: Some(1000),
            unit: crate::TransferUnit::Bytes,
            source_in_view: in_view,
        }
    }

    fn item(id: u64) -> ShelfItem {
        ShelfItem {
            id,
            path: format!("C:/f{id}.txt"),
            name: format!("f{id}.txt"),
            kind: "file".into(),
            ext: "txt".into(),
            size: Some(1),
        }
    }

    #[test]
    fn background_transfer_shows_and_peeks() {
        let t = Instant::now();
        let mut island = Island::default();
        island.handle(SheenEvent::Transfers(vec![download(1, 100, true)]), t);
        assert_eq!(island.present().mode, Mode::Idle, "source in view: stay quiet");

        island.handle(SheenEvent::Transfers(vec![download(1, 200, false)]), t + ms(1000));
        let p = island.present();
        assert_eq!(p.mode, Mode::Transfer);
        assert_eq!(p.transfers.len(), 1);
        assert_eq!(p.geometry, Geometry::TRANSFER);

        island.handle(SheenEvent::Hover(true), t + ms(1100));
        assert_eq!(island.present().geometry, Geometry::TRANSFER_PEEK);
    }

    #[test]
    fn media_wins_over_transfers_but_carries_them() {
        let t = Instant::now();
        let mut island = Island::default();
        island.handle(SheenEvent::Media(Some(song())), t);
        island.handle(SheenEvent::Transfers(vec![download(1, 100, false)]), t);
        let p = island.present();
        assert_eq!(p.mode, Mode::MediaCompact);
        assert_eq!(p.transfers.len(), 1);
    }

    #[test]
    fn finished_transfer_flashes_then_clears() {
        let t = Instant::now();
        let mut island = Island::default();
        island.handle(SheenEvent::Transfers(vec![download(1, 990, false)]), t);
        island.handle(SheenEvent::Transfers(vec![]), t + ms(1000));
        assert_eq!(island.present().transfers[0].state, crate::TransferState::Done);
        let deadline = island.next_deadline().unwrap();
        assert!(island.tick(deadline));
        assert_eq!(island.present().mode, Mode::Idle);
    }

    #[test]
    fn file_drag_turns_island_into_drop_target() {
        let t = Instant::now();
        let mut island = Island::default();
        island.handle(SheenEvent::Media(Some(song())), t);
        island.handle(SheenEvent::FileDrag(true), t);
        let p = island.present();
        assert_eq!(p.mode, Mode::Shelf);
        assert!(p.dragging);

        // Drag ends elsewhere: back to media.
        island.handle(SheenEvent::FileDrag(false), t + ms(500));
        assert_eq!(island.present().mode, Mode::MediaCompact);
    }

    #[test]
    fn drop_onto_island_keeps_shelf_open_until_pointer_leaves() {
        let t = Instant::now();
        let mut island = Island::default();
        island.handle(SheenEvent::FileDrag(true), t);
        island.handle(SheenEvent::Hover(true), t);
        island.handle(SheenEvent::Shelf(vec![item(1)]), t);
        island.handle(SheenEvent::FileDrag(false), t);
        assert_eq!(island.present().mode, Mode::Shelf);
        assert!(!island.present().dragging);

        island.handle(SheenEvent::Hover(false), t + ms(100));
        assert!(island.tick(t + ms(4200)));
        let p = island.present();
        assert_eq!(p.mode, Mode::Idle);
        assert!(p.shelf_badge, "badge shows parked files");
        assert!(island.hit_rect().width > Geometry::IDLE.width, "badge is clickable");
    }

    #[test]
    fn shelf_can_be_opened_and_closed() {
        let t = Instant::now();
        let mut island = Island::default();
        island.handle(SheenEvent::ShelfRequested, t);
        assert_eq!(island.present().mode, Mode::Shelf);
        island.handle(SheenEvent::CollapseRequested, t);
        assert_eq!(island.present().mode, Mode::Idle);
    }

    #[test]
    fn outside_click_closes_panels_at_once() {
        let t = Instant::now();
        let mut island = Island::default();
        island.handle(SheenEvent::Media(Some(song())), t);
        island.handle(SheenEvent::ExpandRequested, t);
        assert!(island.panel_open());
        island.handle(SheenEvent::OutsideClick, t + ms(10));
        assert_eq!(island.present().mode, Mode::MediaCompact);
        assert!(!island.panel_open());
    }

    #[test]
    fn dismiss_hides_until_hovered_again() {
        let t = Instant::now();
        let mut island = Island::default();
        island.handle(SheenEvent::Media(Some(song())), t);
        island.handle(SheenEvent::Hover(true), t);
        island.handle(SheenEvent::Dismiss, t);
        assert_eq!(island.present().mode, Mode::Hidden);
        assert_eq!(island.hit_rect(), REVEAL, "the strip stays hoverable");

        // Still over it after clicking: nothing changes until a new hover.
        island.handle(SheenEvent::Hover(false), t + ms(100));
        assert_eq!(island.present().mode, Mode::Hidden);
        island.handle(SheenEvent::Hover(true), t + ms(900));
        assert_eq!(island.present().mode, Mode::MediaCompact);
    }

    #[test]
    fn compact_media_grows_for_a_transfer_badge() {
        let t = Instant::now();
        let mut island = Island::default();
        island.handle(SheenEvent::Media(Some(song())), t);
        assert_eq!(island.present().geometry, Geometry::MEDIA_COMPACT);
        island.handle(SheenEvent::Transfers(vec![download(1, 10, false)]), t);
        assert_eq!(island.present().geometry, Geometry::MEDIA_COMPACT_TRANSFER);
    }

    #[test]
    fn scale_is_clamped_and_applied_to_the_hit_rect() {
        let mut island = Island::default();
        island.handle(SheenEvent::SetScale(5.0), Instant::now());
        assert_eq!(island.present().scale, MAX_SCALE);
        let r = island.hit_rect();
        assert!((r.width - Geometry::IDLE.width * MAX_SCALE).abs() < 1e-9);
    }

    #[test]
    fn clipboard_peek_uses_tall_osd() {
        let t = Instant::now();
        let mut island = Island::default();
        island.handle(
            SheenEvent::Osd(OsdKind::Clipboard { text: Some("hi".into()), image: None, files: 0 }),
            t,
        );
        assert_eq!(island.present().geometry, Geometry::OSD_TALL);
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

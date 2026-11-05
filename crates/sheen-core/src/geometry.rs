//! Single source of truth for the capsule's size and position.
//!
//! The native window is a fixed, fully transparent canvas pinned to the top of
//! the screen. It never resizes; the capsule is animated *inside* it by the UI.
//! That sidesteps the classic flicker of resizing a native window in lock-step
//! with a CSS animation. The cost of the oversized window is paid by making
//! everything outside the capsule click-through, which needs to know where the
//! capsule is — hence this module, shared by the UI payload and the hit-tester.

use serde::Serialize;
use std::time::{Duration, Instant};

/// Logical size of the transparent canvas window.
pub const CANVAS_WIDTH: f64 = 700.0;
pub const CANVAS_HEIGHT: f64 = 280.0;
/// Gap between the top of the screen and the capsule.
pub const TOP_MARGIN: f64 = 8.0;
/// Room beside the capsule for the close button (left) and the shelf badge
/// (right): an 8 px gap plus a 30 px circle.
pub const SIDE: f64 = 38.0;
/// Where a dismissed island can be brought back by hovering.
pub const REVEAL: Rect =
    Rect { x: (CANVAS_WIDTH - 180.0) / 2.0, y: 0.0, width: 180.0, height: 24.0 };
/// Allowed size multipliers (the canvas is sized for the largest).
pub const MIN_SCALE: f64 = 0.8;
pub const MAX_SCALE: f64 = 1.3;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Geometry {
    pub width: f64,
    pub height: f64,
    pub radius: f64,
}

impl Geometry {
    pub const HIDDEN: Geometry = Geometry { width: 0.0, height: 0.0, radius: 0.0 };
    pub const IDLE: Geometry = Geometry { width: 110.0, height: 30.0, radius: 15.0 };
    /// Playing: just the artwork and the level bars.
    pub const MEDIA_COMPACT: Geometry = Geometry { width: 124.0, height: 34.0, radius: 17.0 };
    /// Playing with a background transfer: adds a small percentage badge.
    pub const MEDIA_COMPACT_TRANSFER: Geometry =
        Geometry { width: 184.0, height: 34.0, radius: 17.0 };
    /// Hovering while playing: title, artist and progress.
    pub const MEDIA_PEEK: Geometry = Geometry { width: 320.0, height: 62.0, radius: 24.0 };
    pub const OSD: Geometry = Geometry { width: 300.0, height: 44.0, radius: 22.0 };
    /// Clipboard peek: room for two lines of text or a thumbnail.
    pub const OSD_TALL: Geometry = Geometry { width: 320.0, height: 64.0, radius: 26.0 };
    pub const TRANSFER: Geometry = Geometry { width: 250.0, height: 36.0, radius: 18.0 };
    pub const TRANSFER_PEEK: Geometry = Geometry { width: 340.0, height: 74.0, radius: 26.0 };
    pub const SHELF: Geometry = Geometry { width: 420.0, height: 132.0, radius: 30.0 };
    pub const EXPANDED: Geometry = Geometry { width: 400.0, height: 156.0, radius: 34.0 };

    /// The capsule's rectangle in canvas coordinates (logical px).
    pub fn rect(&self) -> Rect {
        Rect {
            x: (CANVAS_WIDTH - self.width) / 2.0,
            y: TOP_MARGIN,
            width: self.width,
            height: self.height,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn is_empty(&self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }

    pub fn contains(&self, px: f64, py: f64) -> bool {
        !self.is_empty()
            && px >= self.x
            && py >= self.y
            && px < self.x + self.width
            && py < self.y + self.height
    }

    /// Widen by `d` on both sides.
    pub fn expand_x(&self, d: f64) -> Rect {
        Rect { x: self.x - d, width: self.width + 2.0 * d, ..*self }
    }

    /// Scale about the top centre of the canvas, matching the UI's
    /// `transform-origin: 50% 0`.
    pub fn scaled(&self, s: f64) -> Rect {
        let cx = CANVAS_WIDTH / 2.0;
        Rect {
            x: cx + (self.x - cx) * s,
            y: self.y * s,
            width: self.width * s,
            height: self.height * s,
        }
    }

    pub fn union(&self, other: &Rect) -> Rect {
        if self.is_empty() {
            return *other;
        }
        if other.is_empty() {
            return *self;
        }
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let right = (self.x + self.width).max(other.x + other.width);
        let bottom = (self.y + self.height).max(other.y + other.height);
        Rect { x, y, width: right - x, height: bottom - y }
    }
}

/// The area of the canvas that should receive mouse input.
///
/// While the UI's spring animation morphs the capsule from one geometry to the
/// next, the visible shape is somewhere in between. To avoid a click "falling
/// through" a capsule that is still visibly there, the region is the union of
/// the old and new rectangles until the transition has had time to settle.
#[derive(Debug, Clone)]
pub struct HitRegion {
    current: Rect,
    previous: Rect,
    settle_at: Option<Instant>,
    transition: Duration,
}

impl HitRegion {
    pub fn new(transition: Duration) -> Self {
        Self { current: Rect::default(), previous: Rect::default(), settle_at: None, transition }
    }

    pub fn set_target(&mut self, rect: Rect, now: Instant) {
        if rect == self.current {
            return;
        }
        self.previous = self.active(now);
        self.current = rect;
        self.settle_at = Some(now + self.transition);
    }

    /// Rectangle that should be interactive at `now`.
    pub fn active(&self, now: Instant) -> Rect {
        match self.settle_at {
            Some(t) if now < t => self.current.union(&self.previous),
            _ => self.current,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shape_is_centered_and_fits_the_canvas() {
        for g in [
            Geometry::IDLE,
            Geometry::MEDIA_COMPACT,
            Geometry::MEDIA_COMPACT_TRANSFER,
            Geometry::MEDIA_PEEK,
            Geometry::OSD,
            Geometry::OSD_TALL,
            Geometry::TRANSFER,
            Geometry::TRANSFER_PEEK,
            Geometry::SHELF,
            Geometry::EXPANDED,
        ] {
            let r = g.rect();
            assert_eq!(r.x + r.width / 2.0, CANVAS_WIDTH / 2.0);
            // Must fit at the largest size, including the side buttons.
            let big = r.expand_x(SIDE).scaled(MAX_SCALE);
            assert!(big.x >= 0.0 && big.y + big.height <= CANVAS_HEIGHT, "{g:?} must fit");
            assert!(g.radius * 2.0 <= g.height + 0.01, "{g:?} radius too large");
        }
    }

    #[test]
    fn scaling_keeps_the_top_centre_fixed() {
        let r = Geometry::IDLE.rect().scaled(1.2);
        assert!((r.x + r.width / 2.0 - CANVAS_WIDTH / 2.0).abs() < 1e-9);
        assert!((r.y - TOP_MARGIN * 1.2).abs() < 1e-9);
        assert!((r.width - 132.0).abs() < 1e-9);
    }

    #[test]
    fn hidden_rect_contains_nothing() {
        let r = Geometry::HIDDEN.rect();
        assert!(!r.contains(CANVAS_WIDTH / 2.0, TOP_MARGIN));
    }

    #[test]
    fn hit_region_keeps_union_during_transition() {
        let t0 = Instant::now();
        let mut hr = HitRegion::new(Duration::from_millis(400));
        hr.set_target(Geometry::EXPANDED.rect(), t0);
        hr.set_target(Geometry::MEDIA_COMPACT.rect(), t0 + Duration::from_secs(1));

        let mid = t0 + Duration::from_millis(1200);
        let big = Geometry::EXPANDED.rect();
        assert!(hr.active(mid).contains(big.x + 1.0, big.y + big.height - 1.0));

        let settled = t0 + Duration::from_millis(1500);
        assert_eq!(hr.active(settled), Geometry::MEDIA_COMPACT.rect());
    }
}

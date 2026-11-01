//! Pure page state for the pager: committed page, signed fractional offset,
//! finger-following drag and the settle animation.

use std::time::{Duration, Instant};

/// Fling velocity at or above which the release adds half a page of bias.
const FLING: f32 = 300.0;

fn wrap(index: i64, count: usize) -> usize {
    if count == 0 {
        return 0;
    }
    index.rem_euclid(count as i64) as usize
}

/// A committed page plus a signed fractional offset in `(-1, 1)`. Positive
/// offset shows the next page, negative the previous one. The view renders
/// three wrapped slots around `committed`.
pub struct Pager {
    count: usize,
    committed: usize,
    offset: f32,
    drag_origin: usize,
    drag_offset: f32,
    dragging: bool,
    animation: Option<Animation>,
}

struct Animation {
    from: f32,
    to: f32,
    target: i32,
    started: Instant,
    duration: Duration,
}

impl Pager {
    pub fn new(count: usize, page: usize) -> Self {
        Self {
            count,
            committed: page.min(count.saturating_sub(1)),
            offset: 0.0,
            drag_origin: 0,
            drag_offset: 0.0,
            dragging: false,
            animation: None,
        }
    }

    pub fn committed(&self) -> usize {
        self.committed
    }

    pub fn offset(&self) -> f32 {
        self.offset
    }

    pub fn slots(&self) -> [usize; 3] {
        if self.count <= 1 {
            return [0, 0, 0];
        }
        [
            wrap(self.committed as i64 - 1, self.count),
            self.committed,
            wrap(self.committed as i64 + 1, self.count),
        ]
    }

    pub fn is_animating(&self) -> bool {
        self.animation.is_some()
    }

    pub fn is_dragging(&self) -> bool {
        self.dragging
    }

    /// Replaces the page list, keeping `page` when it exists. Stops any motion.
    pub fn set_count(&mut self, count: usize, page: usize) {
        self.count = count;
        self.committed = page.min(count.saturating_sub(1));
        self.offset = 0.0;
        self.dragging = false;
        self.animation = None;
    }

    pub fn begin_drag(&mut self) {
        self.animation = None;
        self.dragging = true;
        self.drag_origin = self.committed;
        // Taking over a settle must continue from its last rendered position,
        // not jump back to the committed page before following the new finger.
        self.drag_offset = self.offset;
    }

    /// `total_dx` is the finger's horizontal travel since the drag began, in
    /// logical pixels. Pages are committed as each page width is crossed.
    pub fn drag(&mut self, total_dx: f32, width: f32) {
        if !self.dragging || self.count == 0 {
            return;
        }
        if self.count == 1 {
            self.committed = 0;
            self.offset = 0.0;
            return;
        }
        let raw = self.drag_offset - total_dx / width.max(1.0);
        let whole = raw.trunc();
        self.committed = wrap(self.drag_origin as i64 + whole as i64, self.count);
        self.offset = if raw == whole { 0.0 } else { raw - whole };
    }

    pub fn end_drag(&mut self, velocity_x: f32, width: f32, now: Instant, duration: Duration) {
        self.dragging = false;
        if self.count == 0 {
            return;
        }
        let bias = if velocity_x.abs() >= FLING {
            (-velocity_x / width.max(1.0) * 0.5).clamp(-0.5, 0.5)
        } else {
            0.0
        };
        let target = (self.offset + bias).round().clamp(-1.0, 1.0) as i32;
        let target = if self.count == 1 { 0 } else { target };
        self.animate(target, now, duration);
    }

    pub fn cancel_drag(&mut self, now: Instant, duration: Duration) {
        self.dragging = false;
        if self.count == 0 {
            self.offset = 0.0;
            return;
        }
        self.animate(0, now, duration);
    }

    pub fn step(&mut self, delta: i32, now: Instant, duration: Duration) {
        if self.count == 0 || delta == 0 {
            return;
        }
        if delta.abs() == 1 {
            self.animate(delta, now, duration);
        } else {
            self.committed = wrap(self.committed as i64 + delta as i64, self.count);
            self.offset = 0.0;
            self.animation = None;
        }
    }

    pub fn settle_to(&mut self, page: usize, now: Instant, duration: Duration) {
        if self.count == 0 || page >= self.count {
            return;
        }
        let forward = (page + self.count - self.committed) % self.count;
        if forward == 0 {
            return;
        }
        if forward == 1 {
            self.animate(1, now, duration);
        } else if forward == self.count - 1 {
            self.animate(-1, now, duration);
        } else {
            self.committed = page;
            self.offset = 0.0;
            self.animation = None;
        }
    }

    /// Advances the settle animation. Returns whether the committed page
    /// changed, so the caller can reset per-page state.
    pub fn tick(&mut self, now: Instant) -> bool {
        let Some(animation) = &self.animation else {
            return false;
        };
        let (from, to, target, started, duration) = (
            animation.from,
            animation.to,
            animation.target,
            animation.started,
            animation.duration,
        );
        let total = duration.as_secs_f32();
        let elapsed = now.saturating_duration_since(started).as_secs_f32();
        let t = if total <= 0.0 {
            1.0
        } else {
            (elapsed / total).clamp(0.0, 1.0)
        };
        let eased = 1.0 - (1.0 - t).powi(3);
        self.offset = from + (to - from) * eased;
        if t < 1.0 {
            return false;
        }
        self.animation = None;
        self.offset = 0.0;
        self.committed = wrap(self.committed as i64 + target as i64, self.count);
        target != 0
    }

    fn animate(&mut self, target: i32, now: Instant, duration: Duration) {
        self.animation = Some(Animation {
            from: self.offset,
            to: target as f32,
            target,
            started: now,
            duration,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: Duration = Duration::from_millis(300);
    const EPS: f32 = 1e-4;

    fn assert_offset(pager: &Pager, expected: f32) {
        assert!(
            (pager.offset() - expected).abs() < EPS,
            "offset {} != {expected}",
            pager.offset()
        );
    }

    #[test]
    fn new_starts_at_page_and_wraps_slots() {
        let pager = Pager::new(3, 1);
        assert_eq!(pager.committed(), 1);
        assert_offset(&pager, 0.0);
        assert_eq!(pager.slots(), [0, 1, 2]);
        assert!(!pager.is_animating());
        assert!(!pager.is_dragging());
    }

    #[test]
    fn slots_wrap_at_both_ends() {
        assert_eq!(Pager::new(3, 0).slots(), [2, 0, 1]);
        assert_eq!(Pager::new(3, 2).slots(), [1, 2, 0]);
    }

    #[test]
    fn short_drag_settles_back() {
        let now = Instant::now();
        let mut pager = Pager::new(3, 0);
        pager.begin_drag();
        pager.drag(-30.0, 300.0);
        assert_offset(&pager, 0.1);
        pager.end_drag(0.0, 300.0, now, D);
        pager.tick(now + D);
        assert_eq!((pager.committed(), pager.offset()), (0, 0.0));
    }

    #[test]
    fn interrupted_settle_keeps_its_position_in_both_directions() {
        for direction in [-1, 1] {
            let now = Instant::now();
            let mut pager = Pager::new(3, 1);
            pager.step(direction, now, D);
            pager.tick(now + D / 2);
            assert_offset(&pager, direction as f32 * 0.875);

            pager.begin_drag();
            assert!(!pager.is_animating());
            assert!(pager.is_dragging());
            assert_eq!(pager.committed(), 1);
            assert_offset(&pager, direction as f32 * 0.875);
            pager.drag(0.0, 300.0);
            assert_offset(&pager, direction as f32 * 0.875);
        }
    }

    #[test]
    fn interrupted_drag_uses_absolute_travel_from_the_preserved_position() {
        let now = Instant::now();
        let mut pager = Pager::new(3, 1);
        pager.step(1, now, D);
        pager.tick(now + D / 2);
        pager.begin_drag();
        pager.drag(30.0, 300.0);
        assert_offset(&pager, 0.775);
        pager.drag(90.0, 300.0);
        assert_offset(&pager, 0.575);
        pager.drag(300.0, 300.0);
        assert_eq!(pager.committed(), 1);
        assert_offset(&pager, -0.125);
    }

    #[test]
    fn interrupted_drag_can_cross_and_wrap_a_page_boundary() {
        for (page, direction, expected_page, expected_offset) in
            [(2, 1, 0, 0.075), (0, -1, 2, -0.075)]
        {
            let now = Instant::now();
            let mut pager = Pager::new(3, page);
            pager.step(direction, now, D);
            pager.tick(now + D / 2);
            pager.begin_drag();
            pager.drag(-60.0 * direction as f32, 300.0);
            assert_eq!(pager.committed(), expected_page);
            assert_offset(&pager, expected_offset);
            pager.end_drag(0.0, 300.0, now + D, D);
            pager.tick(now + D * 2);
            assert_eq!(pager.committed(), expected_page);
            assert_offset(&pager, 0.0);
        }
    }

    #[test]
    fn repeated_interruptions_capture_the_latest_position() {
        let now = Instant::now();
        let mut pager = Pager::new(3, 1);
        pager.step(1, now, D);
        pager.tick(now + D / 2);
        pager.begin_drag();
        pager.drag(112.5, 300.0);
        assert_offset(&pager, 0.5);
        pager.end_drag(0.0, 300.0, now + D, D);
        pager.tick(now + D + D / 2);
        assert_offset(&pager, 0.9375);
        pager.begin_drag();
        pager.drag(75.0, 300.0);
        assert_offset(&pager, 0.6875);
    }

    #[test]
    fn drag_past_half_settles_next() {
        let now = Instant::now();
        let mut pager = Pager::new(3, 0);
        pager.begin_drag();
        pager.drag(-180.0, 300.0);
        assert_offset(&pager, 0.6);
        pager.end_drag(0.0, 300.0, now, D);
        pager.tick(now + D);
        assert_eq!(pager.committed(), 1);
        assert_offset(&pager, 0.0);
    }

    #[test]
    fn backwards_fling_goes_to_previous() {
        let now = Instant::now();
        let mut pager = Pager::new(3, 0);
        pager.begin_drag();
        pager.drag(90.0, 300.0);
        pager.end_drag(400.0, 300.0, now, D);
        pager.tick(now + D);
        assert_eq!(pager.committed(), 2);
    }

    #[test]
    fn forward_fling_adds_half_page_bias() {
        let now = Instant::now();
        let mut pager = Pager::new(3, 0);
        pager.begin_drag();
        pager.drag(-90.0, 300.0);
        pager.end_drag(-400.0, 300.0, now, D);
        pager.tick(now + D);
        assert_eq!(pager.committed(), 1);
    }

    #[test]
    fn long_drag_commits_each_crossed_page() {
        let mut pager = Pager::new(3, 0);
        pager.begin_drag();
        pager.drag(-640.0, 320.0);
        assert_eq!(pager.committed(), 2);
        assert_offset(&pager, 0.0);
        assert_eq!(pager.slots(), [1, 2, 0]);
    }

    #[test]
    fn step_animates_one_page_and_wraps() {
        let now = Instant::now();
        let mut pager = Pager::new(3, 2);
        pager.step(1, now, D);
        assert!(pager.is_animating());
        pager.tick(now + D);
        assert_eq!(pager.committed(), 0);
        assert!(!pager.is_animating());
    }

    #[test]
    fn multi_step_is_immediate() {
        let now = Instant::now();
        let mut pager = Pager::new(3, 0);
        pager.step(-1, now, D);
        assert!(pager.is_animating());
        let mut pager = Pager::new(3, 0);
        pager.step(-3, now, D);
        assert_eq!(pager.committed(), 0);
        assert!(!pager.is_animating());
        let mut pager = Pager::new(5, 0);
        pager.step(2, now, D);
        assert_eq!(pager.committed(), 2);
        assert!(!pager.is_animating());
    }

    #[test]
    fn settle_to_adjacent_animates_and_far_is_immediate() {
        let now = Instant::now();
        let mut pager = Pager::new(5, 1);
        pager.settle_to(2, now, D);
        assert!(pager.is_animating());
        pager.tick(now + D);
        assert_eq!(pager.committed(), 2);

        let mut pager = Pager::new(5, 1);
        pager.settle_to(4, now, D);
        assert_eq!(pager.committed(), 4);
        assert!(!pager.is_animating());
    }

    #[test]
    fn single_page_always_stays_on_page_zero() {
        let now = Instant::now();
        let mut pager = Pager::new(1, 0);
        pager.begin_drag();
        pager.drag(-120.0, 300.0);
        pager.end_drag(-900.0, 300.0, now, D);
        pager.tick(now + D);
        assert_eq!(pager.committed(), 0);
        pager.step(1, now, D);
        pager.tick(now + D);
        assert_eq!(pager.committed(), 0);
    }

    #[test]
    fn empty_pager_is_inert() {
        let now = Instant::now();
        let mut pager = Pager::new(0, 5);
        assert_eq!(pager.committed(), 0);
        assert_eq!(pager.slots(), [0, 0, 0]);
        pager.begin_drag();
        pager.drag(-300.0, 300.0);
        pager.end_drag(-900.0, 300.0, now, D);
        pager.step(1, now, D);
        pager.settle_to(2, now, D);
        pager.tick(now + D);
        assert_eq!(pager.committed(), 0);
        assert!(!pager.is_animating());
    }

    #[test]
    fn set_count_clamps_and_keeps_the_requested_page() {
        let mut pager = Pager::new(3, 1);
        pager.set_count(5, 3);
        assert_eq!(pager.committed(), 3);
        pager.set_count(2, 4);
        assert_eq!(pager.committed(), 1);
        pager.set_count(0, 2);
        assert_eq!(pager.committed(), 0);
        assert!(!pager.is_animating());
    }

    #[test]
    fn resize_mid_animation_does_not_change_the_target() {
        let now = Instant::now();
        let mut pager = Pager::new(4, 0);
        pager.step(1, now, D);
        pager.tick(now + D / 2);
        assert!(pager.is_animating());
        pager.tick(now + D);
        assert_eq!(pager.committed(), 1);
        assert_offset(&pager, 0.0);
    }
}

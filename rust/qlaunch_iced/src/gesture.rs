//! Pure gesture classifier for long press, tap, double tap, drag and
//! pull-down. The launcher's app state applies its outcomes.

use std::time::{Duration, Instant};

use iced::{Point, touch};
use qlaunch_core::model::Id;

pub const LONG_PRESS: Duration = Duration::from_millis(500);
pub const TOUCH_SLOP: f32 = 18.0;
pub const DOUBLE_TAP: Duration = Duration::from_millis(300);
pub const DOUBLE_TAP_SLOP: f32 = 100.0;
pub const PULL_DOWN: f32 = 48.0;
pub const FLING: f32 = 300.0;

/// Where the finger went down. Known only after the widget under it reports
/// the press, which may arrive after the raw touch event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Start {
    Item(Id),
    Blank,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Pressed,
    Dragging,
    Pulled,
    LongPressed,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    None,
    DragStart,
    Drag,
    DragEnd { velocity: f32 },
    PullDown,
    LongPress(Start),
    Tap(Start),
    DoubleTap,
    Cancelled,
}

pub struct Gesture {
    finger: Option<touch::Finger>,
    down: Point,
    down_at: Instant,
    last: Point,
    last_move: Option<(Instant, Point)>,
    velocity: f32,
    sequence: u64,
    phase: Phase,
    target: Option<Start>,
    slop_exceeded: bool,
    last_blank_tap: Option<(Instant, Point)>,
}

impl Default for Gesture {
    fn default() -> Self {
        Self {
            finger: None,
            down: Point::ORIGIN,
            down_at: Instant::now(),
            last: Point::ORIGIN,
            last_move: None,
            velocity: 0.0,
            sequence: 0,
            phase: Phase::Idle,
            target: None,
            slop_exceeded: false,
            last_blank_tap: None,
        }
    }
}

impl Gesture {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn target(&self) -> Option<&Start> {
        self.target.as_ref()
    }

    /// A pointer is down, but its motion has not yet exceeded touch slop.
    /// Scroll notifications in this phase may be jitter in a future page swipe.
    pub fn is_pending(&self) -> bool {
        self.phase == Phase::Pressed && !self.slop_exceeded
    }

    /// True while a release at the current position would be a tap.
    pub fn is_tap_candidate(&self) -> bool {
        self.phase == Phase::Pressed && self.target.is_some() && !self.slop_exceeded
    }

    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Starts a gesture for the first finger. Returns the new sequence; a
    /// second finger while one is active leaves the gesture untouched.
    pub fn pressed(&mut self, finger: touch::Finger, position: Point, at: Instant) -> u64 {
        if self.finger.is_some() {
            return self.sequence;
        }
        self.sequence += 1;
        self.finger = Some(finger);
        self.down = position;
        self.down_at = at;
        self.last = position;
        self.last_move = None;
        self.velocity = 0.0;
        self.phase = Phase::Pressed;
        self.target = None;
        self.slop_exceeded = false;
        self.sequence
    }

    /// Records the widget under the press. A press whose identify message
    /// arrives after the long-press deadline fires the long press at once.
    pub fn identify(&mut self, target: Start) -> Outcome {
        if self.phase != Phase::Pressed {
            return Outcome::None;
        }
        self.target = Some(target.clone());
        let late = Instant::now().saturating_duration_since(self.down_at) >= LONG_PRESS;
        if late && !self.slop_exceeded {
            self.phase = Phase::LongPressed;
            self.last_blank_tap = None;
            Outcome::LongPress(target)
        } else {
            Outcome::None
        }
    }

    pub fn moved(
        &mut self,
        finger: touch::Finger,
        position: Point,
        at: Instant,
        at_top: bool,
    ) -> Outcome {
        if self.finger != Some(finger) {
            return Outcome::None;
        }
        if matches!(self.phase, Phase::Pulled | Phase::LongPressed) {
            return Outcome::None;
        }

        let dx = position.x - self.down.x;
        let dy = position.y - self.down.y;
        if dx.hypot(dy) > TOUCH_SLOP {
            self.slop_exceeded = true;
        }

        let (previous_at, previous) = self.last_move.unwrap_or((self.down_at, self.down));
        let dt = at.saturating_duration_since(previous_at).as_secs_f32();
        if dt > 0.0 {
            let instantaneous = (position.x - previous.x) / dt;
            self.velocity = self.velocity * 0.6 + instantaneous * 0.4;
        }
        self.last_move = Some((at, position));
        self.last = position;

        if self.target.is_some() && at_top && dy > PULL_DOWN && dy > 1.5 * dx.abs() {
            self.phase = Phase::Pulled;
            self.last_blank_tap = None;
            return Outcome::PullDown;
        }
        if self.phase == Phase::Pressed && dx.abs() > TOUCH_SLOP && dx.abs() > 1.5 * dy.abs() {
            self.phase = Phase::Dragging;
            self.last_blank_tap = None;
            return Outcome::DragStart;
        }
        if self.phase == Phase::Dragging {
            return Outcome::Drag;
        }
        Outcome::None
    }

    pub fn released(&mut self, finger: touch::Finger, position: Point, at: Instant) -> Outcome {
        if self.finger != Some(finger) {
            return Outcome::None;
        }
        let outcome = if self.phase == Phase::Dragging {
            Outcome::DragEnd {
                velocity: self.velocity,
            }
        } else if self.phase == Phase::Pressed && self.target.is_some() && !self.slop_exceeded {
            let target = self.target.clone().unwrap_or(Start::Blank);
            match &target {
                Start::Blank => {
                    let double = self.last_blank_tap.is_some_and(|(then, there)| {
                        at.saturating_duration_since(then) <= DOUBLE_TAP
                            && (there.x - position.x).hypot(there.y - position.y) <= DOUBLE_TAP_SLOP
                    });
                    if double {
                        self.last_blank_tap = None;
                        Outcome::DoubleTap
                    } else {
                        self.last_blank_tap = Some((at, position));
                        Outcome::Tap(target)
                    }
                }
                Start::Item(_) => {
                    self.last_blank_tap = None;
                    Outcome::Tap(target)
                }
            }
        } else {
            Outcome::None
        };
        self.reset();
        outcome
    }

    pub fn lost(&mut self, finger: touch::Finger) -> Outcome {
        if self.finger != Some(finger) {
            return Outcome::None;
        }
        let outcome = if self.phase == Phase::Dragging {
            Outcome::Cancelled
        } else {
            Outcome::None
        };
        self.reset();
        outcome
    }

    /// Fires the long press for the current sequence once its deadline has
    /// passed without drift or release.
    pub fn tick(&mut self, sequence: u64, now: Instant) -> Outcome {
        if sequence != self.sequence || self.phase != Phase::Pressed || self.slop_exceeded {
            return Outcome::None;
        }
        let Some(target) = self.target.clone() else {
            return Outcome::None;
        };
        if now.saturating_duration_since(self.down_at) < LONG_PRESS {
            return Outcome::None;
        }
        self.phase = Phase::LongPressed;
        self.last_blank_tap = None;
        Outcome::LongPress(target)
    }

    /// Horizontal travel since the drag began, in logical pixels.
    pub fn drag_dx(&self) -> f32 {
        self.last.x - self.down.x
    }

    fn reset(&mut self) {
        self.finger = None;
        self.phase = Phase::Idle;
        self.target = None;
        self.slop_exceeded = false;
        self.velocity = 0.0;
        self.last_move = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: Duration = Duration::from_millis(1);

    fn at(ms: u64) -> Instant {
        Instant::now() + MS * ms as u32
    }

    fn item() -> Start {
        Start::Item("a".into())
    }

    fn press(g: &mut Gesture, target: Start, t: Instant) -> u64 {
        let seq = g.pressed(touch::Finger(0), Point::new(10.0, 10.0), t);
        assert_eq!(g.identify(target), Outcome::None);
        seq
    }

    fn press_blank(g: &mut Gesture, t: Instant) -> u64 {
        press(g, Start::Blank, t)
    }

    fn tap_blank(g: &mut Gesture, t: Instant) -> Outcome {
        let pos = Point::new(10.0, 10.0);
        g.pressed(touch::Finger(0), pos, t);
        g.identify(Start::Blank);
        g.released(touch::Finger(0), pos, t + 50 * MS)
    }

    #[test]
    fn tap_on_item_is_a_tap() {
        let mut g = Gesture::new();
        let t = at(0);
        press(&mut g, item(), t);
        assert!(g.is_tap_candidate());
        assert_eq!(
            g.released(touch::Finger(0), Point::new(11.0, 10.0), t + 50 * MS),
            Outcome::Tap(item())
        );
        assert_eq!(g.phase(), Phase::Idle);
    }

    #[test]
    fn tap_with_drift_under_slop_is_still_a_tap() {
        let mut g = Gesture::new();
        let t = at(0);
        press(&mut g, item(), t);
        assert_eq!(
            g.moved(touch::Finger(0), Point::new(20.0, 12.0), t + 20 * MS, true),
            Outcome::None
        );
        assert_eq!(
            g.released(touch::Finger(0), Point::new(20.0, 12.0), t + 50 * MS),
            Outcome::Tap(item())
        );
    }

    #[test]
    fn drag_starting_on_item_is_never_a_tap() {
        let mut g = Gesture::new();
        let t = at(0);
        press(&mut g, item(), t);
        assert_eq!(
            g.moved(touch::Finger(0), Point::new(110.0, 10.0), t + 50 * MS, true),
            Outcome::DragStart
        );
        assert_eq!(
            g.moved(touch::Finger(0), Point::new(160.0, 10.0), t + 80 * MS, true),
            Outcome::Drag
        );
        assert!(matches!(
            g.released(touch::Finger(0), Point::new(160.0, 10.0), t + 100 * MS),
            Outcome::DragEnd { .. }
        ));
    }

    #[test]
    fn horizontal_drag_must_dominate_vertical() {
        let mut g = Gesture::new();
        let t = at(0);
        press(&mut g, item(), t);
        assert_eq!(
            g.moved(
                touch::Finger(0),
                Point::new(110.0, 100.0),
                t + 30 * MS,
                true
            ),
            Outcome::None
        );
        assert_eq!(g.phase(), Phase::Pressed);
        assert_eq!(
            g.moved(
                touch::Finger(0),
                Point::new(210.0, 100.0),
                t + 60 * MS,
                true
            ),
            Outcome::DragStart
        );
    }

    #[test]
    fn long_press_fires_at_500ms() {
        let mut g = Gesture::new();
        let t = at(0);
        let seq = press(&mut g, item(), t);
        assert_eq!(g.tick(seq, t + 499 * MS), Outcome::None);
        assert_eq!(g.tick(seq, t + 500 * MS), Outcome::LongPress(item()));
        assert_eq!(g.phase(), Phase::LongPressed);
    }

    #[test]
    fn long_press_cancelled_by_slop_drift() {
        let mut g = Gesture::new();
        let t = at(0);
        let seq = press(&mut g, item(), t);
        g.moved(touch::Finger(0), Point::new(40.0, 10.0), t + 20 * MS, true);
        assert_eq!(g.tick(seq, t + 600 * MS), Outcome::None);
    }

    #[test]
    fn long_press_cancelled_by_early_release() {
        let mut g = Gesture::new();
        let t = at(0);
        let seq = press(&mut g, item(), t);
        assert_eq!(
            g.released(touch::Finger(0), Point::new(10.0, 10.0), t + 400 * MS),
            Outcome::Tap(item())
        );
        assert_eq!(g.tick(seq, t + 600 * MS), Outcome::None);
    }

    #[test]
    fn stale_long_press_timer_is_ignored() {
        let mut g = Gesture::new();
        let t = at(0);
        let first = press(&mut g, item(), t);
        g.released(touch::Finger(0), Point::new(10.0, 10.0), t + 50 * MS);
        let second = press(&mut g, Start::Blank, t + 100 * MS);
        assert_ne!(first, second);
        assert_eq!(g.tick(first, t + 600 * MS), Outcome::None);
        assert_eq!(g.phase(), Phase::Pressed);
    }

    #[test]
    fn long_press_on_blank_reports_blank() {
        let mut g = Gesture::new();
        let t = at(0);
        let seq = press_blank(&mut g, t);
        assert_eq!(g.tick(seq, t + 500 * MS), Outcome::LongPress(Start::Blank));
    }

    #[test]
    fn double_tap_on_blank_within_window() {
        let mut g = Gesture::new();
        let t = at(0);
        assert_eq!(tap_blank(&mut g, t), Outcome::Tap(Start::Blank));
        assert_eq!(tap_blank(&mut g, t + 100 * MS), Outcome::DoubleTap);
    }

    #[test]
    fn slow_second_blank_tap_is_not_a_double() {
        let mut g = Gesture::new();
        let t = at(0);
        assert_eq!(tap_blank(&mut g, t), Outcome::Tap(Start::Blank));
        assert_eq!(tap_blank(&mut g, t + 400 * MS), Outcome::Tap(Start::Blank));
    }

    #[test]
    fn distant_second_blank_tap_is_not_a_double() {
        let mut g = Gesture::new();
        let t = at(0);
        assert_eq!(tap_blank(&mut g, t), Outcome::Tap(Start::Blank));
        let pos = Point::new(200.0, 10.0);
        g.pressed(touch::Finger(0), pos, t + 100 * MS);
        g.identify(Start::Blank);
        assert_eq!(
            g.released(touch::Finger(0), pos, t + 150 * MS),
            Outcome::Tap(Start::Blank)
        );
    }

    #[test]
    fn double_tap_never_fires_from_an_item() {
        let mut g = Gesture::new();
        let t = at(0);
        press(&mut g, item(), t);
        assert_eq!(
            g.released(touch::Finger(0), Point::new(10.0, 10.0), t + 50 * MS),
            Outcome::Tap(item())
        );
        press(&mut g, item(), t + 100 * MS);
        assert_eq!(
            g.released(touch::Finger(0), Point::new(10.0, 10.0), t + 150 * MS),
            Outcome::Tap(item())
        );
    }

    #[test]
    fn pull_down_requires_top_and_48px() {
        let mut g = Gesture::new();
        let t = at(0);
        press_blank(&mut g, t);
        assert_eq!(
            g.moved(touch::Finger(0), Point::new(10.0, 70.0), t + 30 * MS, false),
            Outcome::None
        );
        g.released(touch::Finger(0), Point::new(10.0, 70.0), t + 40 * MS);

        let mut g = Gesture::new();
        press_blank(&mut g, t);
        assert_eq!(
            g.moved(touch::Finger(0), Point::new(10.0, 57.0), t + 30 * MS, true),
            Outcome::None
        );
        assert_eq!(
            g.moved(touch::Finger(0), Point::new(10.0, 59.0), t + 40 * MS, true),
            Outcome::PullDown
        );
    }

    #[test]
    fn pull_down_fires_once_per_gesture() {
        let mut g = Gesture::new();
        let t = at(0);
        press_blank(&mut g, t);
        assert_eq!(
            g.moved(touch::Finger(0), Point::new(10.0, 80.0), t + 30 * MS, true),
            Outcome::PullDown
        );
        assert_eq!(
            g.moved(touch::Finger(0), Point::new(10.0, 140.0), t + 50 * MS, true),
            Outcome::None
        );
        assert_eq!(
            g.released(touch::Finger(0), Point::new(10.0, 140.0), t + 60 * MS),
            Outcome::None
        );
    }

    #[test]
    fn lost_mid_drag_cancels() {
        let mut g = Gesture::new();
        let t = at(0);
        press(&mut g, item(), t);
        assert_eq!(
            g.moved(touch::Finger(0), Point::new(110.0, 10.0), t + 30 * MS, true),
            Outcome::DragStart
        );
        assert_eq!(g.lost(touch::Finger(0)), Outcome::Cancelled);
        assert_eq!(g.phase(), Phase::Idle);
    }

    #[test]
    fn tap_candidate_clears_after_slop_drift() {
        let mut g = Gesture::new();
        let t = at(0);
        press(&mut g, item(), t);
        assert!(g.is_tap_candidate());
        g.moved(touch::Finger(0), Point::new(29.0, 10.0), t + 30 * MS, false);
        assert!(!g.is_tap_candidate());
    }

    #[test]
    fn press_on_chrome_without_target_never_taps() {
        let mut g = Gesture::new();
        let t = at(0);
        g.pressed(touch::Finger(0), Point::new(10.0, 10.0), t);
        assert_eq!(
            g.released(touch::Finger(0), Point::new(10.0, 10.0), t + 50 * MS),
            Outcome::None
        );
        assert_eq!(g.phase(), Phase::Idle);
    }

    #[test]
    fn drag_velocity_is_reported() {
        let mut g = Gesture::new();
        let t = at(0);
        press(&mut g, item(), t);
        g.moved(touch::Finger(0), Point::new(110.0, 10.0), t + 50 * MS, true);
        g.moved(
            touch::Finger(0),
            Point::new(210.0, 10.0),
            t + 120 * MS,
            true,
        );
        let velocity = match g.released(touch::Finger(0), Point::new(210.0, 10.0), t + 140 * MS) {
            Outcome::DragEnd { velocity } => velocity,
            other => panic!("expected drag end, got {other:?}"),
        };
        assert!(velocity > 100.0, "velocity {velocity}");
    }

    #[test]
    fn second_finger_does_not_restart_the_gesture() {
        let mut g = Gesture::new();
        let t = at(0);
        let seq = press(&mut g, item(), t);
        assert_eq!(
            g.pressed(touch::Finger(1), Point::new(300.0, 300.0), t + 10 * MS),
            seq
        );
        assert_eq!(
            g.moved(
                touch::Finger(1),
                Point::new(400.0, 300.0),
                t + 20 * MS,
                true
            ),
            Outcome::None
        );
        assert_eq!(
            g.released(touch::Finger(1), Point::new(400.0, 300.0), t + 30 * MS),
            Outcome::None
        );
        assert_eq!(
            g.released(touch::Finger(0), Point::new(10.0, 10.0), t + 40 * MS),
            Outcome::Tap(item())
        );
    }
}

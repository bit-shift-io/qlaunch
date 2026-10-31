//! App-side frame-work timings. One window covers a drag or page settle;
//! `App::update` starts it and `App::view` records samples.

/// Per-frame phase durations in milliseconds for one motion window. `update`
/// and `view` are measured in `App`; `layout` and `draw` are measured by the
/// probe that wraps the launcher tree.
#[derive(Default)]
pub struct Samples {
    active: bool,
    update: Vec<f32>,
    view: Vec<f32>,
    layout: Vec<f32>,
    draw: Vec<f32>,
}

pub struct Summary {
    pub frames: usize,
    pub update_p50: f32,
    pub update_p95: f32,
    pub update_max: f32,
    pub view_p50: f32,
    pub view_p95: f32,
    pub view_max: f32,
    pub layout_p50: f32,
    pub layout_p95: f32,
    pub layout_max: f32,
    pub draw_p50: f32,
    pub draw_p95: f32,
    pub draw_max: f32,
}

impl Samples {
    /// Opens a window unless one is already open, so repeated calls during a
    /// drag do not discard samples.
    pub fn start(&mut self) {
        if self.active {
            return;
        }
        self.active = true;
        self.update.clear();
        self.view.clear();
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn record_update(&mut self, ms: f32) {
        if self.active {
            self.update.push(ms);
        }
    }

    pub fn record_view(&mut self, ms: f32) {
        if self.active {
            self.view.push(ms);
        }
    }

    pub fn record_layout(&mut self, ms: f32) {
        if self.active {
            self.layout.push(ms);
        }
    }

    pub fn record_draw(&mut self, ms: f32) {
        if self.active {
            self.draw.push(ms);
        }
    }

    /// Closes the window and summarizes it. Returns `None` when no window is
    /// open; otherwise the sample vectors are emptied.
    pub fn finish(&mut self) -> Option<Summary> {
        if !self.active {
            return None;
        }
        self.active = false;
        let update = std::mem::take(&mut self.update);
        let view = std::mem::take(&mut self.view);
        let layout = std::mem::take(&mut self.layout);
        let draw = std::mem::take(&mut self.draw);
        let mut sorted_update = update.clone();
        let mut sorted_view = view.clone();
        let mut sorted_layout = layout.clone();
        let mut sorted_draw = draw.clone();
        for sorted in [
            &mut sorted_update,
            &mut sorted_view,
            &mut sorted_layout,
            &mut sorted_draw,
        ] {
            sorted.sort_by(f32::total_cmp);
        }
        Some(Summary {
            frames: update.len().max(view.len()),
            update_p50: percentile(&sorted_update, 0.50),
            update_p95: percentile(&sorted_update, 0.95),
            update_max: sorted_update.last().copied().unwrap_or(0.0),
            view_p50: percentile(&sorted_view, 0.50),
            view_p95: percentile(&sorted_view, 0.95),
            view_max: sorted_view.last().copied().unwrap_or(0.0),
            layout_p50: percentile(&sorted_layout, 0.50),
            layout_p95: percentile(&sorted_layout, 0.95),
            layout_max: sorted_layout.last().copied().unwrap_or(0.0),
            draw_p50: percentile(&sorted_draw, 0.50),
            draw_p95: percentile(&sorted_draw, 0.95),
            draw_max: sorted_draw.last().copied().unwrap_or(0.0),
        })
    }
}

/// Nearest-rank percentile of an already sorted slice. Empty input is zero.
pub fn percentile(sorted: &[f32], q: f32) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    let rank = (q.clamp(0.0, 1.0) * sorted.len() as f32).ceil() as usize;
    sorted[rank.saturating_sub(1).min(sorted.len() - 1)]
}

impl Summary {
    /// One logcat line, greppable with `frame-work`.
    pub fn log_line(&self) -> String {
        format!(
            "frame-work frames={} update p50={:.2}ms p95={:.2}ms max={:.2}ms \
             view p50={:.2}ms p95={:.2}ms max={:.2}ms \
             layout p50={:.2}ms p95={:.2}ms max={:.2}ms \
             draw p50={:.2}ms p95={:.2}ms max={:.2}ms",
            self.frames,
            self.update_p50,
            self.update_p95,
            self.update_max,
            self.view_p50,
            self.view_p95,
            self.view_max,
            self.layout_p50,
            self.layout_p95,
            self.layout_max,
            self.draw_p50,
            self.draw_p95,
            self.draw_max,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_picks_nearest_rank() {
        assert_eq!(percentile(&[1.0, 2.0, 3.0, 4.0], 0.5), 2.0);
        assert_eq!(percentile(&[1.0, 2.0, 3.0, 4.0], 0.95), 4.0);
    }

    #[test]
    fn empty_percentile_is_zero() {
        assert_eq!(percentile(&[], 0.5), 0.0);
    }

    #[test]
    fn records_only_while_active() {
        let mut samples = Samples::default();
        samples.record_update(9.0);
        samples.record_view(9.0);
        assert!(samples.finish().is_none());
        samples.start();
        samples.record_update(2.0);
        samples.record_update(4.0);
        samples.record_view(6.0);
        let summary = samples.finish().expect("window is open");
        assert_eq!(summary.update_p50, 2.0);
        assert_eq!(summary.update_p95, 4.0);
        assert_eq!(summary.view_max, 6.0);
    }

    #[test]
    fn start_keeps_an_open_window() {
        let mut samples = Samples::default();
        samples.start();
        samples.record_update(3.0);
        samples.start();
        samples.record_update(5.0);
        let summary = samples.finish().expect("window is open");
        assert_eq!(summary.frames, 2);
    }

    #[test]
    fn finish_returns_a_summary_and_deactivates() {
        let mut samples = Samples::default();
        samples.start();
        samples.record_update(1.0);
        samples.record_view(2.0);
        assert!(samples.finish().is_some());
        assert!(!samples.is_active());
        assert!(samples.finish().is_none());
    }

    #[test]
    fn layout_and_draw_are_recorded_beside_update_and_view() {
        let mut samples = Samples::default();
        samples.record_layout(9.0);
        samples.record_draw(9.0);
        assert!(samples.finish().is_none(), "inactive records nothing");
        samples.start();
        samples.record_update(1.0);
        samples.record_view(2.0);
        samples.record_layout(3.0);
        samples.record_draw(4.0);
        let summary = samples.finish().expect("window is open");
        assert_eq!(summary.layout_p50, 3.0);
        assert_eq!(summary.layout_max, 3.0);
        assert_eq!(summary.draw_p50, 4.0);
        assert_eq!(summary.draw_max, 4.0);
        assert!(summary.log_line().contains("layout p50=3.00ms"));
        assert!(summary.log_line().contains("draw p50=4.00ms"));
    }

    #[test]
    fn log_line_prefixes_frame_work() {
        let mut samples = Samples::default();
        samples.start();
        samples.record_update(1.0);
        samples.record_view(2.0);
        let line = samples.finish().unwrap().log_line();
        assert!(line.starts_with("frame-work"));
        assert!(line.contains("update p50=1.00ms"));
        assert!(line.contains("view p50=2.00ms"));
    }
}

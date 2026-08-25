use std::time::{Duration, Instant};

pub(crate) struct FramePacer {
    frame_dur: Duration,
    next_frame: Instant,
}

impl FramePacer {
    pub(crate) fn new(fps: f64) -> Self {
        Self {
            frame_dur: Duration::from_secs_f64(1.0 / fps),
            next_frame: Instant::now(),
        }
    }

    pub(crate) fn wait(&mut self) {
        let now = Instant::now();
        // fell far behind (stall/hiccup): resync to real time instead of
        // burst-running catch-up frames without sleeping
        if now.duration_since(self.next_frame) > self.frame_dur * 2 {
            self.next_frame = now;
        }
        self.next_frame += self.frame_dur;
        if let Some(remaining) = self.next_frame.checked_duration_since(now) {
            std::thread::sleep(remaining);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resyncs_after_long_stall() {
        let mut pacer = FramePacer::new(1000.0);
        // simulate a 50 ms stall: target is far in the past
        pacer.next_frame = Instant::now() - Duration::from_millis(50);
        pacer.wait();
        // next frame must be ~1 ms away, not still ~49 ms behind
        assert!(Instant::now().duration_since(pacer.next_frame) < Duration::from_millis(5));
    }

    #[test]
    fn keeps_drift_compensation_normally() {
        let mut pacer = FramePacer::new(1000.0);
        // slightly behind (< 2 frames of debt): must NOT resync, must catch up
        pacer.next_frame = Instant::now() - Duration::from_millis(1);
        pacer.wait();
        assert!(Instant::now().duration_since(pacer.next_frame) < Duration::from_millis(5));
    }
}

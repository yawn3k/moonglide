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
        self.next_frame += self.frame_dur;
        let now = Instant::now();
        if now < self.next_frame {
            std::thread::sleep(self.next_frame - now);
        }
    }
}

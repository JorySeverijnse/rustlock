use std::time::Duration;

pub struct FadeTimer {
    duration: Duration,
    start_time: std::time::Instant,
}

impl FadeTimer {
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            start_time: std::time::Instant::now(),
        }
    }

    pub fn update(&mut self) -> bool {
        let elapsed = self.start_time.elapsed();
        let progress = elapsed.as_secs_f64() / self.duration.as_secs_f64();
        progress >= 1.0
    }

    pub fn current_alpha(&self) -> f64 {
        let elapsed = self.start_time.elapsed();
        (elapsed.as_secs_f64() / self.duration.as_secs_f64()).min(1.0)
    }
}

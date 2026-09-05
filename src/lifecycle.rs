/// A fixed screen-coordinate baseline; small moves must accumulate against it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct InputBaseline {
    pub tick: u64,
    pub x: i32,
    pub y: i32,
}

impl InputBaseline {
    pub fn ready(self, now: u64) -> bool {
        now.saturating_sub(self.tick) >= 500
    }

    pub fn moved(self, now: u64, x: i32, y: i32) -> bool {
        self.ready(now)
            && ((i64::from(x) - i64::from(self.x)).abs() > 4
                || (i64::from(y) - i64::from(self.y)).abs() > 4)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct Shutdown {
    pub stopping: bool,
    pub live: usize,
}

impl Shutdown {
    pub fn begin(&mut self) -> bool {
        !std::mem::replace(&mut self.stopping, true)
    }

    pub fn created(&mut self) {
        self.live += 1;
    }

    pub fn destroyed(&mut self) -> bool {
        self.live = self.live.saturating_sub(1);
        self.stopping && self.live == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mouse_grace_and_strict_threshold_use_original_baseline() {
        let baseline = InputBaseline {
            tick: 100,
            x: -1920,
            y: 40,
        };
        assert!(!baseline.moved(599, 1000, 1000));
        assert!(!baseline.moved(600, -1916, 44));
        assert!(baseline.moved(600, -1915, 40));
        assert!(baseline.moved(601, -1920, 35));
        assert!(!baseline.moved(50, i32::MAX, i32::MIN));
        let extreme = InputBaseline {
            tick: 0,
            x: i32::MIN,
            y: 0,
        };
        assert!(extreme.moved(500, i32::MAX, 0));
    }

    #[test]
    fn shutdown_is_idempotent_and_quits_after_last_window() {
        let mut shutdown = Shutdown::default();
        for _ in 0..3 {
            shutdown.created();
        }
        assert!(shutdown.begin());
        assert!(!shutdown.begin());
        assert!(!shutdown.destroyed());
        assert!(!shutdown.destroyed());
        assert!(shutdown.destroyed());
        assert_eq!(shutdown.live, 0);
    }
}

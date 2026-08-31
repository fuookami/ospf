use std::cell::Cell;
use std::time::{Duration, Instant};

pub trait Iterator {
    fn iteration(&self) -> usize;
    fn not_better_iteration(&self) -> usize;
    fn time(&self) -> Duration;
    fn next(&self, better: bool);
}

pub struct Iteration {
    iteration: Cell<usize>,
    not_better_iteration: Cell<usize>,
    begin_time: Instant,
}

impl Default for Iteration {
    fn default() -> Self {
        Self {
            iteration: Cell::new(0),
            not_better_iteration: Cell::new(0),
            begin_time: Instant::now(),
        }
    }
}

impl Iterator for Iteration {
    fn iteration(&self) -> usize {
        self.iteration.get()
    }

    fn not_better_iteration(&self) -> usize {
        self.not_better_iteration.get()
    }

    fn time(&self) -> Duration {
        Instant::now() - self.begin_time
    }

    fn next(&self, better: bool) {
        self.iteration.set(self.iteration.get() + 1);
        if (better) {
            self.not_better_iteration.set(0);
        } else {
            self.not_better_iteration
                .set(self.not_better_iteration.get() - 1);
        }
    }
}

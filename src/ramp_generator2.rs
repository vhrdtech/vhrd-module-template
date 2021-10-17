use crate::app;
use embedded_time::Instant;
use embedded_time::duration::Milliseconds;
use core::convert::TryFrom;

pub struct RampGenerator {
    prev_t: Option<Instant<crate::TimMono>>,
    state: State,
    current: i32,
    target: i32,
    rate_up: u32,
    rate_down: u32,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum State {
    RampUp,
    RampDown,
    Hold,
}

impl RampGenerator {
    pub const fn new() -> Self {
        RampGenerator {
            prev_t: None,
            state: State::Hold,
            current: 0,
            target: 0,
            rate_up: 1,
            rate_down: 1
        }
    }

    fn update_state(&mut self) {
        let prev = self.state;
        self.state = if self.current == self.target {
            State::Hold
        } else if self.current > self.target {
            State::RampDown
        } else {
            State::RampUp
        };

        log_debug!("update_state prev:{:?} -> {:?} cur:{} tgt:{}", prev, self.state, self.current, self.target);
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn get_output(&mut self) -> i32 {
        let now: Instant<crate::TimMono> = app::monotonics::TimMono::now();
        if self.state == State::Hold {
            self.prev_t = Some(now);
            self.current
        } else {
            let dt = match self.prev_t {
                Some(prev_t) => {
                    let r = now
                        .checked_duration_since(&prev_t)
                        .map(|dt| Milliseconds::<u32>::try_from(dt).unwrap_or(Milliseconds(0)))
                        .unwrap_or(Milliseconds(0));
                    self.prev_t = Some(now);
                    r
                },
                None => {
                    self.prev_t = Some(now);
                    Milliseconds(0)
                }
            };
            let dv = if self.state == State::RampUp {
                self.rate_up * dt.0 / 1000
            } else {
                self.rate_down * dt.0 / 1000
            };
            let dv = dv as i32;
            self.current = if self.state == State::RampUp {
                if self.current + dv > self.target {
                    self.target
                } else {
                    self.current + dv
                }
            } else { // RampDown
                if self.current - dv < self.target {
                    self.target
                } else {
                    self.current - dv
                }
            };

            log_debug!("dt = {} dv = {}, new_current = {}", dt.0, dv, self.current);

            self.update_state();
            self.current
        }
    }

    pub fn set_rates(&mut self, up_per_s: u32, down_per_s: u32) {
        self.rate_up = up_per_s;
        self.rate_down = down_per_s;
    }

    pub fn set_current(&mut self, current: i32) {
        // log_warn!("set_current {}", current);
        let now: Instant<crate::TimMono> = app::monotonics::TimMono::now();
        self.prev_t = Some(now);
        self.current = current;
        self.update_state();
    }

    pub fn hold_current(&mut self) {
        self.target = self.current;
        self.state = State::Hold;
    }

    pub fn set_target(&mut self, target: i32) {
        // log_warn!("set_target {}", target);
        self.target = target;
        self.update_state();
    }
}
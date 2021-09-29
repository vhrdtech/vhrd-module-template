use crate::prelude::*;
use embedded_time::duration::Microseconds;
use embedded_time::Instant;
use rtic::rtic_monotonic::Milliseconds;
use core::convert::TryFrom;

const RPM_PER_S: u32 = 10;
const EMIT_PERIOD: Milliseconds = Milliseconds(100);
const INPUT_TIMEOUT: Milliseconds = Milliseconds(2000);

#[derive(Debug)]
pub enum Event {
    SetRpmTarget(i32),
    _Internal,
}

#[derive(Copy, Clone, Debug, )]
pub enum State {
    Off,
    Ramp {
        prev_t: Instant<crate::TimMono>,
        last_input_t: Instant<crate::TimMono>,
        current: i32,
        target: i32,
    },
    Hold {
        current: i32,
        last_input_t: Instant<crate::TimMono>,
    }
}

impl State {
    pub const fn new() -> Self {
        State::Off
    }
}

pub fn ramp_generator(cx: app::ramp_generator::Context, e: Event) {
    let state: &mut State = cx.local.state;
    let now: Instant<crate::TimMono> = app::monotonics::TimMono::now();
    log_debug!("ramp e: {:?} s: {:?}", e, state);
    *state = match *state {
        State::Off => {
            match e {
                Event::SetRpmTarget(target) => {
                    if target == 0 {
                        State::Off
                    } else {
                        app::ramp_generator::spawn_after(EMIT_PERIOD, Event::_Internal).ok();
                        State::Ramp {
                            prev_t: now,
                            last_input_t: now,
                            current: 0,
                            target
                        }
                    }
                }
                Event::_Internal => {
                    State::Off
                }
            }
        }
        State::Ramp { prev_t, last_input_t, current, target } => {
            let dt = now.checked_duration_since(&prev_t).map(
                |dt| Milliseconds::<u32>::try_from(dt)
                    .unwrap_or(Milliseconds(0)))
                .unwrap_or(Milliseconds(0));
            let dv = (RPM_PER_S * dt.0 / 1000) as i32;
            let dv = if dv == 0 {
                1
            } else {
                dv
            };

            let input_dt = now.checked_duration_since(&last_input_t).map(
                |dt| Milliseconds::<u32>::try_from(dt)
                    .unwrap_or(Milliseconds(0)))
                .unwrap_or(Milliseconds(0));
            let (target, last_input_t, ignore_spawn) = match e {
                Event::SetRpmTarget(target) => (target, now, true),
                Event::_Internal => {
                    if input_dt > INPUT_TIMEOUT {
                        (0, last_input_t, false)
                    } else {
                        (target, last_input_t, false)
                    }
                }
            };

            let new_current = if target > 0 {
                if current + dv > target {
                    target
                } else {
                    current + dv
                }
            } else {
                if current - dv < target {
                    target
                } else {
                    current - dv
                }
            };
            log_info!("dt: {} dv: {} new_cur: {} input_dt: {}", dt, dv, new_current, input_dt);

            if new_current == target {
                if target == 0 {
                    State::Off
                } else {
                    if !ignore_spawn {
                        app::ramp_generator::spawn_after(EMIT_PERIOD, Event::_Internal).ok();
                    }
                    State::Hold {
                        current: target,
                        last_input_t
                    }
                }

            } else if new_current != 0 {
                if !ignore_spawn {
                    app::ramp_generator::spawn_after(EMIT_PERIOD, Event::_Internal).ok();
                }
                State::Ramp {
                    prev_t: now,
                    last_input_t,
                    current: new_current,
                    target
                }
            } else {
                State::Off
            }
        }
        State::Hold { current, last_input_t } => {
            app::ramp_generator::spawn_after(EMIT_PERIOD, Event::_Internal).ok();
            let input_dt = now.checked_duration_since(&last_input_t).map(
                |dt| Milliseconds::<u32>::try_from(dt)
                    .unwrap_or(Milliseconds(0)))
                .unwrap_or(Milliseconds(0));
            if input_dt > INPUT_TIMEOUT {
                State::Ramp {
                    prev_t: now,
                    last_input_t,
                    current,
                    target: 0
                }
            } else {
                State::Hold { current, last_input_t }
            }
        }
    }
}
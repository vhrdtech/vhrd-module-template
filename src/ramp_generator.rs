use crate::prelude::*;
use core::convert::TryFrom;
use embedded_time::Instant;
use rtic::rtic_monotonic::Milliseconds;

// const RPM_PER_S: u32 = 10;
const EMIT_PERIOD: Milliseconds = Milliseconds(100);
const INPUT_TIMEOUT: Milliseconds = Milliseconds(2000);

#[derive(Debug)]
pub enum Event {
    Reset {
        initial: i32,
        target: i32,
        rate_per_s: u32,
    },
    SetTarget {
        target: i32,
        rate_per_s: u32
    },
    _Internal,
}

#[derive(Copy, Clone, Debug)]
pub enum State {
    Off,
    Ramp {
        prev_t: Instant<crate::TimMono>,
        last_input_t: Instant<crate::TimMono>,
        current: i32,
        target: i32,
        rate_per_s: u32,
    },
    Hold {
        current: i32,
        rate_per_s: u32,
        last_input_t: Instant<crate::TimMono>,
    },
}

impl State {
    pub const fn new() -> Self {
        State::Off
    }
}

pub fn ramp_generator(cx: app::ramp_generator::Context, e: Event) {
    let state: &mut State = cx.local.state;
    let now: Instant<crate::TimMono> = app::monotonics::TimMono::now();
    log_debug!("ramp_generator e: {:?} s: {:?}", e, state);
    if let Event::Reset { initial, target, rate_per_s } = e {
        if target == initial {
            *state = State::Hold {
                current: initial,
                rate_per_s,
                last_input_t: now
            };
        } else {
            *state = State::Ramp {
                prev_t: now,
                last_input_t: now,
                current: initial,
                target,
                rate_per_s
            };
        }
        count_result!(app::ramp_vesc::spawn(crate::ramp_vesc::Event::_RampGenerator(initial)));
        if let State::Off = state {
            count_result!(app::ramp_generator::spawn_after(
                EMIT_PERIOD,
                Event::_Internal
            ));
        }
        return;
    }
    let (new_state, respawn) = match *state {
        State::Off => match e {
            Event::Reset { .. } => unreachable!(),
            Event::SetTarget {target, rate_per_s} => {
                if target == 0 {
                    (State::Off, false)
                } else {
                    (
                        State::Ramp {
                            prev_t: now,
                            last_input_t: now,
                            current: 0,
                            target,
                            rate_per_s,
                        },
                        true,
                    )
                }
            }
            Event::_Internal => (State::Off, false),
        },
        State::Ramp {
            prev_t,
            last_input_t,
            current,
            target,
            rate_per_s,
        } => {
            let dt = now
                .checked_duration_since(&prev_t)
                .map(|dt| Milliseconds::<u32>::try_from(dt).unwrap_or(Milliseconds(0)))
                .unwrap_or(Milliseconds(0));
            let dv = (rate_per_s * dt.0 / 1000) as i32;
            let dv = if dv == 0 { 1 } else { dv };

            let input_dt = now
                .checked_duration_since(&last_input_t)
                .map(|dt| Milliseconds::<u32>::try_from(dt).unwrap_or(Milliseconds(0)))
                .unwrap_or(Milliseconds(0));
            let (target, last_input_t, respawn, rate_per_s) = match e {
                Event::Reset { .. } => unreachable!(),
                Event::SetTarget {target, rate_per_s} => (target, now, false, rate_per_s),
                Event::_Internal => {
                    if input_dt > INPUT_TIMEOUT {
                        (0, last_input_t, true, rate_per_s)
                    } else {
                        (target, last_input_t, true, rate_per_s)
                    }
                }
            };
            count_result!(app::ramp_vesc::spawn(crate::ramp_vesc::Event::_RampGenerator(current)));

            let new_current = if target - current > 0 {
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
            log_info!(
                "dt: {} dv: {} new_cur: {} input_dt: {}",
                dt,
                dv,
                new_current,
                input_dt
            );

            if new_current == target {
                if target == 0 {
                    (State::Off, false)
                } else {
                    (
                        State::Hold {
                            current: target,
                            last_input_t,
                            rate_per_s,
                        },
                        respawn,
                    )
                }
            } else if new_current != 0 {
                (
                    State::Ramp {
                        prev_t: now,
                        last_input_t,
                        current: new_current,
                        target,
                        rate_per_s
                    },
                    respawn,
                )
            } else {
                (State::Off, false)
            }
        }
        State::Hold {
            current,
            last_input_t,
            rate_per_s,
        } => {
            count_result!(app::ramp_vesc::spawn(crate::ramp_vesc::Event::_RampGenerator(current)));

            let (last_input_t, maybe_new_target, respawn, rate_per_s) = match e {
                Event::Reset { .. } => unreachable!(),
                Event::SetTarget {target, rate_per_s} => (now, target, false, rate_per_s),
                Event::_Internal => (last_input_t, current, true, rate_per_s),
            };
            let input_dt = now
                .checked_duration_since(&last_input_t)
                .map(|dt| Milliseconds::<u32>::try_from(dt).unwrap_or(Milliseconds(0)))
                .unwrap_or(Milliseconds(0));
            if input_dt > INPUT_TIMEOUT {
                (
                    State::Ramp {
                        prev_t: now,
                        last_input_t,
                        current,
                        target: 0,
                        rate_per_s
                    },
                    respawn,
                )
            } else {
                if current == maybe_new_target {
                    (
                        State::Hold {
                            current,
                            last_input_t,
                            rate_per_s
                        },
                        respawn,
                    )
                } else {
                    (
                        State::Ramp {
                            prev_t: now,
                            last_input_t,
                            current,
                            target: maybe_new_target,
                            rate_per_s
                        },
                        respawn,
                    )
                }
            }
        }
    };
    *state = new_state;
    if respawn {
        count_result!(app::ramp_generator::spawn_after(
            EMIT_PERIOD,
            Event::_Internal
        ));
        // match app::ramp_generator::spawn_after(EMIT_PERIOD, Event::_Internal) {
        //     Ok(handle) => {
        //         let _:() = handle;
        //     },
        //     Err(e) => {
        //         log_error!("ramp_generator: spawn e: {:?}", e);
        //     }
        // }
    }
}

use crate::prelude::*;
use vhrdcan::{Frame, FrameId};
use embedded_time::Instant;
use embedded_time::duration::Milliseconds;
use core::convert::TryFrom;
use crate::utils::clone_into_array;

const INPUT_TIMEOUT: Milliseconds = Milliseconds(500);
const PI_MAX_DUTY: u32 = 20_000;

const SET_DUTY: u32 = 0;
const SET_CURRENT: u32 = 1;
const VESC_ID: u8 = 7;
const VESC_SET_DUTY_ID: FrameId = FrameId::new_extended((SET_DUTY << 8) | VESC_ID as u32).unwrap();
const VESC_SET_CURRENT_ID: FrameId = FrameId::new_extended((SET_CURRENT << 8) | VESC_ID as u32).unwrap();

pub enum Event {
    FeedbackReceived(Frame<8>)
}

#[derive(Copy, Clone)]
pub enum Mode {
    Off,
    Duty(i32),
    ErpmPI(i32),
    ErpmSearch {
        duty_p5: i32,
        erpm_target: i32
    },
}
impl Mode {
    pub const fn new() -> Self {
        Mode::Off
    }
}

#[derive(Copy, Clone)]
pub struct State {
    mode: Mode,
    last_t: Option<Instant<crate::TimMono>>,
    last_erpm: i32,
    i: i32,
    last_duty: i32,
    found: bool
}
impl State {
    pub const fn new() -> Self {
        State {
            mode: Mode::Off,
            last_t: None,
            last_duty: 0,
            last_erpm: 0,
            i: 0,
            found: false
        }
    }
}

pub fn tf_vesc(mut cx: app::tf_vesc::Context, e: Event) {
    let state: &mut State = cx.local.state;

    if let Event::FeedbackReceived(frame) = e {
        if frame.data().len() != 8 {
            return;
        }
        state.last_erpm = i32::from_be_bytes(clone_into_array(&frame.data()[0..=3]));
        state.last_duty = (i16::from_be_bytes(clone_into_array(&frame.data()[6..=7])) as i32) * 100;

        // log_debug!("telem: erpm:{}, duty: {}", state.last_erpm, state.last_duty);
    }

    let now: Instant<crate::TimMono> = app::monotonics::TimMono::now();
    let input: Option<Mode> = cx.shared.tf_vesc_input.lock(|input| input.take());
    match input {
        Some(mode) => {
            state.mode = mode;
            state.last_t = Some(now);
        },
        None => {
            match state.mode {
                Mode::Off => {}
                Mode::Duty(_) | Mode::ErpmPI(_) | Mode::ErpmSearch {..} => {
                    match state.last_t {
                        Some(last_t) => {
                            let dt = now
                                .checked_duration_since(&last_t)
                                .map(|dt| Milliseconds::<u32>::try_from(dt).unwrap_or(Milliseconds(0)))
                                .unwrap_or(Milliseconds(0));
                            if dt > INPUT_TIMEOUT {
                                state.mode = Mode::Off;
                                log_debug!("tf_vesc: timeout");
                                let current: i32 = 0;
                                let frame = Frame::new(VESC_SET_CURRENT_ID, &current.to_be_bytes()).unwrap();
                                can_send!(cx, frame);

                            }
                        },
                        None => {
                            state.last_t = Some(now);
                        }
                    }
                }
            }
        }
    }

    match state.mode {
        Mode::Off => {}
        Mode::Duty(duty_p5) => {
            let frame = Frame::new(VESC_SET_DUTY_ID, &duty_p5.to_be_bytes()).unwrap();
            can_send!(cx, frame);
        }
        Mode::ErpmPI(erpm) => {
            let e = erpm - state.last_erpm;
            let p = e * 10;
            state.i += e / 100;

            let duty_p5 = p + state.i;
            let duty_p5_clamp = if duty_p5 > PI_MAX_DUTY as i32 {
                PI_MAX_DUTY as i32
            } else if duty_p5 < -(PI_MAX_DUTY as i32) {
                -(PI_MAX_DUTY as i32)
            } else {
                duty_p5
            };
            log_debug!("erpm = {}, duty = {}, target = {}, e = {}, p = {}, i = {}, o = {}, oc = {}", state.last_erpm, state.last_duty,  erpm, e, p, state.i, duty_p5, duty_p5_clamp);

            let frame = Frame::new(VESC_SET_DUTY_ID, &duty_p5_clamp.to_be_bytes()).unwrap();
            can_send!(cx, frame);
        }
        Mode::ErpmSearch { duty_p5, erpm_target } => {
            if state.found {
                can_send!(cx, Frame::new(VESC_SET_DUTY_ID, &state.last_duty.to_be_bytes()).unwrap());
            } else {
                let e = erpm_target - state.last_erpm;
                log_info!("erpm_search: e: {}", e);
                if e.abs() < 100 {
                    state.found = true;
                    can_send!(cx, Frame::new(VESC_SET_DUTY_ID, &state.last_duty.to_be_bytes()).unwrap());
                } else {
                    can_send!(cx, Frame::new(VESC_SET_DUTY_ID, &duty_p5.to_be_bytes()).unwrap());
                }
            }
        }
    }
}

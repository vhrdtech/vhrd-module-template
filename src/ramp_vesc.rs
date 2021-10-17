use crate::app;
use vhrdcan::{FrameId, Frame};
use crate::prelude::*;
use crate::utils::clone_into_array;
use embedded_time::duration::Milliseconds;
use crate::ramp_generator2::RampGenerator;
use embedded_time::Instant;
use core::convert::TryFrom;
use crate::ramp_generator2;

const DUTY_RATE_PER_S: u32 = 500; // 1% = 1000
const DUTY_MIN: u32 = 4_000;

const RPM_RATE_PER_S: u32 = 500;

#[derive(Copy, Clone, Debug)]
pub enum ControlInput {
    SetDutyTarget(i32),
    SetRpmTarget(i32),
}
#[cfg(feature = "vesc-ctrl")]
impl ControlInput {
    pub fn new(source: NodeId, message: Message, payload: &[u8]) -> Option<Self> {
        if source == config::PI_NODE_ID && message.subject_id == config::RMP_RAMP_TARGET_SUBJECT_ID {
            if payload.len() < 4 {
                return None;
            }
            let rpm = i32::from_le_bytes(clone_into_array(&payload[0..=3]));
            Some(ControlInput::SetRpmTarget(rpm))
        } else if source == config::PI_NODE_ID && message.subject_id == config::DUTY_RAMP_TARGET_SUBJECT_ID {
            if payload.len() < 4 {
                return None;
            }
            let duty_p5 = i32::from_le_bytes(clone_into_array(&payload[0..=3]));
            Some(ControlInput::SetDutyTarget(duty_p5))
        } else {
            None
        }
    }
}

pub struct VescFeedback {
    erpm: i32,
    duty_p5: i32,
}
impl VescFeedback {
    pub fn new(frame: Frame<8>) -> Option<Self> {
        if frame.data().len() != 8 {
            return None;
        }
        Some(VescFeedback {
            erpm: i32::from_be_bytes(clone_into_array(&frame.data()[0..=3])),
            duty_p5: (i16::from_be_bytes(clone_into_array(&frame.data()[6..=7])) as i32) * 100
        })
    }
}

#[derive(Debug)]
enum Mode {
    Off,
    Duty,
    Erpm(i32),
}

pub struct State {
    ramp_generator: RampGenerator,
    current_mode: Mode,
    feedback: VescFeedback
}
impl State {
    pub const fn new() -> Self {
        State {
            ramp_generator: RampGenerator::new(),
            current_mode: Mode::Off,
            feedback: VescFeedback {
                erpm: 0,
                duty_p5: 0
            }
        }
    }
}

#[cfg(feature = "vesc-ctrl")]
pub fn ramp_vesc(mut cx: app::ramp_vesc::Context) {
    count_result!(app::ramp_vesc::spawn_after(Milliseconds::new(100u32)));
    let state: &mut State = cx.local.state;

    let is_watchdog_triggered: Option<()> = cx.shared.vesc_watchdog_triggered.lock(|t| t.take());
    if is_watchdog_triggered.is_some() {
        state.current_mode = Mode::Off;
        log_error!("ramp_vesc -> Mode::Off");
    }

    let vesc_feedback: Option<VescFeedback> = cx.shared.vesc_feedback.lock(|f| f.take());
    if let Some(vesc_feedback) = vesc_feedback {
        state.feedback = vesc_feedback;
        // log_debug!("f: {}", state.feedback.erpm);
    }

    let input: Option<ControlInput> = cx.shared.vesc_control_input.lock(|input| input.take());
    // log_debug!("ramp_vesc: {:?} current_mode: {:?}", input, state.current_mode);
    if let Some(input) = input {
        match input {
            ControlInput::SetDutyTarget(duty_p5) => {
                match state.current_mode {
                    Mode::Off => {
                        if duty_p5.abs() < DUTY_MIN as i32 {
                            state.current_mode = Mode::Off;
                            return;
                        }

                        state.current_mode = Mode::Duty;
                        state.ramp_generator.set_rates(500, 300);

                        if duty_p5 > 0 {
                            state.ramp_generator.set_current(DUTY_MIN as i32);
                        } else {
                            state.ramp_generator.set_current(-(DUTY_MIN as i32));
                        }
                        state.ramp_generator.set_target(duty_p5);
                        // cx.shared.vesc_watchdog_input.lock(|wi| *wi = Some(state.ramp_generator.get_output()));
                    }
                    Mode::Duty => {
                        state.ramp_generator.set_target(duty_p5);
                        let o = state.ramp_generator.get_output();
                        // log_debug!("o={} s={:?}", o, state.ramp_generator.state());
                        cx.shared.vesc_watchdog_input.lock(|wi| *wi = Some(o));
                    }
                    Mode::Erpm(_) => {}
                }
            }
            ControlInput::SetRpmTarget(erpm) => {
                match state.current_mode {
                    Mode::Off => {
                        if erpm.abs() < 800 {
                            state.current_mode = Mode::Off;
                            return;
                        }

                        state.current_mode = Mode::Erpm(erpm);
                        state.ramp_generator.set_rates(500, 300);
                        if erpm > 0 {
                            state.ramp_generator.set_current(DUTY_MIN as i32);
                            state.ramp_generator.set_target(25_000);
                        } else {
                            state.ramp_generator.set_current(-(DUTY_MIN as i32));
                            state.ramp_generator.set_target(-25_000);
                        }
                    }
                    Mode::Duty => {}
                    Mode::Erpm(erpm) => {
                        // state.ramp_generator.set_target(duty_p5);
                        if state.ramp_generator.state() == ramp_generator2::State::Hold {
                            cx.shared.vesc_watchdog_input.lock(|wi| *wi = Some(state.ramp_generator.get_output()));
                            return;
                        }
                        let err = erpm - state.feedback.erpm;
                        log_debug!("err: {}", err);
                        if err.abs() < 1500 {
                            log_info!("Slowing search down");
                            state.ramp_generator.set_rates(50, 300);
                        }
                        if err.abs() < 200 {
                            log_info!("Duty found!");
                            state.ramp_generator.hold_current();
                        }
                        let o = state.ramp_generator.get_output();
                        // log_debug!("o={} s={:?}", o, state.ramp_generator.state());
                        cx.shared.vesc_watchdog_input.lock(|wi| *wi = Some(o));
                    }
                }
            }
        }
    }



    // match state.current_mode {
    //     Mode::Off => {}
    //     Mode::Duty => {
    //         let o = state.ramp_generator.get_output();
    //         log_debug!("o={} s={:?}", o, state.ramp_generator.state());
    //         cx.shared.vesc_watchdog_input.lock(|wi| *wi = Some(o));
    //     }
    //     Mode::Rpm(_) => {}
    // }

    // log_debug!("ramp_vesc: e: {:?}", e);
    // match state.current_mode {
    //     Mode::Off => {
    //         match e {
    //             Event::SetDutyTarget(duty_p5) => {
    //                 // cx.shared.tf_vesc_input.lock(|i| *i = Some(crate::tf_vesc::Mode::Duty(duty_p5)));
    //                 state.current_mode = Mode::Duty;
    //                 let duty_p5 = duty_map(duty_p5);
    //                 count_result!(app::ramp_generator::spawn(crate::ramp_generator::Event::Reset { initial: DUTY_MIN as i32, target: duty_p5, rate_per_s: DUTY_RATE_PER_S }));
    //             }
    //             Event::SetRpmTarget(rpm) => {
    //                 state.current_mode = Mode::Rpm(rpm);
    //                 let erpm_max = if rpm > 0 {
    //                     30_000
    //                 } else {
    //                     -30_000
    //                 };
    //                 count_result!(app::ramp_generator::spawn(crate::ramp_generator::Event::Reset { initial: 0, target: erpm_max, rate_per_s: DUTY_RATE_PER_S }));
    //                 // log_info!("set_rpm: {}", rpm);
    //                 // state.current_mode = Mode::Rpm;
    //                 // count_result!(app::ramp_generator::spawn(crate::ramp_generator::Event::SetTarget { target: rpm, rate_per_s: RPM_RATE_PER_S }));
    //             }
    //             Event::_RampGenerator(_) => {
    //                 log_warn!("ramp_vesc: unexpected ramp_generator output");
    //             },
    //             Event::_VescTelemetry(_) => unreachable!()
    //         }
    //     },
    //     Mode::Duty => {
    //         match e {
    //             Event::SetDutyTarget(duty_p5) => {
    //                 let duty_p5 = duty_map(duty_p5);
    //                 count_result!(app::ramp_generator::spawn(crate::ramp_generator::Event::SetTarget { target: duty_p5, rate_per_s: DUTY_RATE_PER_S }));
    //             }
    //             Event::SetRpmTarget(rpm) => {
    //                 log_info!("rpm_vesc: duty->rpm handover");
    //                 count_result!(app::ramp_generator::spawn(crate::ramp_generator::Event::Reset { initial: state.last_known_rpm, target: rpm, rate_per_s: RPM_RATE_PER_S }));
    //             }
    //             Event::_RampGenerator(duty_p5) => {
    //                 let duty_p5 = duty_map(duty_p5);
    //                 cx.shared.tf_vesc_input.lock(|i| *i = Some(crate::tf_vesc::Mode::Duty(duty_p5)));
    //             },
    //             Event::_VescTelemetry(_) => unreachable!()
    //         }
    //     },
    //     Mode::Rpm(erpm) => {
    //         match e {
    //             Event::SetDutyTarget(duty_p5) => {
    //                 log_info!("rpm_vesc: rpm->duty handover");
    //                 count_result!(app::ramp_generator::spawn(crate::ramp_generator::Event::Reset { initial: state.last_known_duty, target: duty_p5, rate_per_s: DUTY_RATE_PER_S }));
    //
    //             }
    //             Event::SetRpmTarget(rpm) => {
    //                 count_result!(app::ramp_generator::spawn(crate::ramp_generator::Event::SetTarget { target: rpm, rate_per_s: RPM_RATE_PER_S }));
    //             }
    //             Event::_RampGenerator(duty_p5) => {
    //                 cx.shared.tf_vesc_input.lock(|i| *i = Some(crate::tf_vesc::Mode::ErpmSearch { duty_p5, erpm_target: erpm }));
    //                 // const SET_ERPM: u32 = 3;
    //                 // const VESC_ID: u8 = 7;
    //                 // let can_id = FrameId::new_extended((SET_ERPM << 8) | VESC_ID as u32).unwrap();
    //                 // let frame = Frame::new(can_id, &rpm.to_be_bytes()).unwrap();
    //                 // can_send!(cx, frame);
    //             }
    //             Event::_VescTelemetry(_) => unreachable!()
    //         }
    //     }
    // }
}

enum WatchdogVescMode {
    Off,
    On(i32)
}

pub struct WatchdogVescState {
    mode: WatchdogVescMode,
    last_t: Option<Instant<crate::TimMono>>
}
impl WatchdogVescState {
    pub const fn new() -> Self {
        WatchdogVescState {
            mode: WatchdogVescMode::Off,
            last_t: None
        }
    }
}

const SET_DUTY: u32 = 0;
const SET_CURRENT: u32 = 1;
const VESC_ID: u8 = 7;
const VESC_SET_DUTY_ID: FrameId = FrameId::new_extended((SET_DUTY << 8) | VESC_ID as u32).unwrap();
const VESC_SET_CURRENT_ID: FrameId = FrameId::new_extended((SET_CURRENT << 8) | VESC_ID as u32).unwrap();

const INPUT_TIMEOUT: Milliseconds = Milliseconds(500);

#[cfg(feature = "vesc-ctrl")]
pub fn watchdog_vesc(mut cx: app::watchdog_vesc::Context) {
    count_result!(app::watchdog_vesc::spawn_after(Milliseconds::new(100u32)));
    let state: &mut WatchdogVescState = cx.local.state;

    let now: Instant<crate::TimMono> = app::monotonics::TimMono::now();
    let input: Option<i32> = cx.shared.vesc_watchdog_input.lock(|input| input.take());
    match input {
        Some(duty_p5) => {
            state.mode = WatchdogVescMode::On(duty_p5);
            state.last_t = Some(now);
            can_send!(cx, Frame::new(VESC_SET_DUTY_ID, &duty_p5.to_be_bytes()).unwrap());
        },
        None => {
            match state.mode {
                WatchdogVescMode::Off => {}
                WatchdogVescMode::On(duty_p5) => {
                    match state.last_t {
                        Some(last_t) => {
                            let dt = now
                                .checked_duration_since(&last_t)
                                .map(|dt| Milliseconds::<u32>::try_from(dt).unwrap_or(Milliseconds(0)))
                                .unwrap_or(Milliseconds(0));
                            if dt > INPUT_TIMEOUT {
                                state.mode = WatchdogVescMode::Off;
                                log_debug!("tf_vesc: timeout");
                                cx.shared.vesc_watchdog_triggered.lock(|t| *t = Some(()));
                                let current: i32 = 0;
                                let frame = Frame::new(VESC_SET_CURRENT_ID, &current.to_be_bytes()).unwrap();
                                can_send!(cx, frame);
                            } else {
                                can_send!(cx, Frame::new(VESC_SET_DUTY_ID, &duty_p5.to_be_bytes()).unwrap());
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
}

fn duty_map(duty_p5: i32) -> i32 {
    if duty_p5 > 0 && duty_p5 <= DUTY_MIN as i32 {
        DUTY_MIN as i32
    } else if duty_p5 < 0 && duty_p5 >= -(DUTY_MIN as i32) {
        -(DUTY_MIN as i32)
    } else {
        duty_p5
    }
}
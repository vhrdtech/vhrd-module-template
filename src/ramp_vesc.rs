use crate::app;
use vhrdcan::{FrameId, Frame};
use crate::prelude::*;

const DUTY_RATE_PER_S: u32 = 500; // 1% = 1000
const DUTY_MIN: u32 = 2000;

const RPM_RATE_PER_S: u32 = 500;

#[derive(Copy, Clone, Debug)]
pub enum Event {
    SetDutyTarget(i32),
    SetRpmTarget(i32),
    _RampGenerator(i32),
}

enum Mode {
    Off,
    Duty,
    Rpm,
}

pub struct State {
    current_mode: Mode,
    // last captured rpm from the bus
    last_known_rpm: i32, // [1]
    // last captured duty from the bus
    last_known_duty: i32, // [% * 10e5], 1% = 1000
}
impl State {
    pub const fn new() -> Self {
        State {
            current_mode: Mode::Off,
            last_known_rpm: 0,
            last_known_duty: 0,
        }
    }
}

pub fn ramp_vesc(mut cx: app::ramp_vesc::Context, e: Event) {
    log_debug!("ramp_vesc: e: {:?}", e);
    let state: &mut State = cx.local.state;
    match state.current_mode {
        Mode::Off => {
            match e {
                Event::SetDutyTarget(duty_p5) => {
                    state.current_mode = Mode::Duty;
                    let duty_p5 = duty_map(duty_p5);
                    count_result!(app::ramp_generator::spawn(crate::ramp_generator::Event::SetTarget { target: duty_p5, rate_per_s: DUTY_RATE_PER_S }));
                }
                Event::SetRpmTarget(rpm) => {
                    state.current_mode = Mode::Rpm;
                    count_result!(app::ramp_generator::spawn(crate::ramp_generator::Event::SetTarget { target: rpm, rate_per_s: RPM_RATE_PER_S }));
                }
                Event::_RampGenerator(_) => {
                    log_warn!("ramp_vesc: unexpected ramp_generator output");
                }
            }
        },
        Mode::Duty => {
            match e {
                Event::SetDutyTarget(duty_p5) => {
                    let duty_p5 = duty_map(duty_p5);
                    count_result!(app::ramp_generator::spawn(crate::ramp_generator::Event::SetTarget { target: duty_p5, rate_per_s: DUTY_RATE_PER_S }));
                }
                Event::SetRpmTarget(rpm) => {
                    log_info!("rpm_vesc: duty->rpm handover");
                }
                Event::_RampGenerator(duty_p5) => {
                    let duty_p5 = duty_map(duty_p5);
                    const SET_DUTY: u32 = 0;
                    const VESC_ID: u8 = 7;
                    let can_id = FrameId::new_extended((SET_DUTY << 8) | VESC_ID as u32).unwrap();
                    let frame = Frame::new(can_id, &duty_p5.to_be_bytes()).unwrap();
                    can_send!(cx, frame);
                }
            }
        },
        Mode::Rpm => {
            match e {
                Event::SetDutyTarget(duty_p5) => {
                    log_info!("rpm_vesc: rpm->duty handover");

                }
                Event::SetRpmTarget(rpm) => {
                    count_result!(app::ramp_generator::spawn(crate::ramp_generator::Event::SetTarget { target: rpm, rate_per_s: RPM_RATE_PER_S }));
                }
                Event::_RampGenerator(rpm) => {
                    const SET_ERPM: u32 = 3;
                    const VESC_ID: u8 = 7;
                    let can_id = FrameId::new_extended((SET_ERPM << 8) | VESC_ID as u32).unwrap();
                    let frame = Frame::new(can_id, &rpm.to_be_bytes()).unwrap();
                    can_send!(cx, frame);
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
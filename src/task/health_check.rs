use crate::{app, config, };
use vhrdcan::frame::Frame;
use vhrdcan::FrameId;
use rtic::Mutex;
use rtic::rtic_monotonic::Milliseconds;
use uavcan_llr::types::TransferId;
use uavcan_llr::tailbyte::TailByte;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Health {
    /// not a typo, fully functioning node
    Norminal = 0b000,
    /// node can perform it's task, but is experiencing troubles
    Warning = 0b001,
    /// node cannot perform it's task
    Failure = 0b010,
}

#[derive(Default)]
pub struct State {
    transfer_id: TransferId,
}

impl State {
    pub const fn new() -> Self {
        State {
            transfer_id: TransferId::new(0).unwrap(),
        }
    }
}

pub fn health_check_task(mut cx: crate::app::health_check_task::Context) {
    let uptime = if config::HEALTH_CHECK_PERIOD == Milliseconds::new(1000u32) {
        cx.shared.uptime.lock(|uptime: &mut u32| {
            *uptime = uptime.saturating_add(1);
            *uptime
        })
    } else {
        0u32 // TODO: impl if health check period is not 1s
    };

    let mut payload = [0u8; 8];
    payload[0..=3].copy_from_slice(&uptime.to_le_bytes());
    payload[4] = cx.shared.health.lock(|h| *h as u8);
    payload[5] = TailByte::single_frame_transfer(cx.local.state.transfer_id).as_byte();
    cx.local.state.transfer_id.increment();

    let id = FrameId::new_extended(0x456).unwrap();
    let frame = Frame::new_move(id, payload, 6).unwrap();
    can_send!(cx, frame);
    app::health_check_task::spawn_after(config::HEALTH_CHECK_PERIOD).ok();
}
use crate::{app, config, };
use vhrdcan::frame::Frame;
use vhrdcan::FrameId;
use rtic::Mutex;
use rtic::rtic_monotonic::Milliseconds;
use uavcan_llr::types::{TransferId, CanId, NodeId, SubjectId, Priority};
use uavcan_llr::tailbyte::TailByte;
use uavcan_llr::slicer::{Slicer, OwnedSlice};

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

    let mut payload = [0u8; 7];
    payload[0..=3].copy_from_slice(&uptime.to_le_bytes());
    payload[4] = cx.shared.health.lock(|h| *h as u8);

    let id = CanId::new_message_kind(NodeId::new(1).unwrap(), SubjectId::new(2).unwrap(), false, Priority::Nominal);
    let frame = Slicer::<8>::new_single(OwnedSlice::new(payload, 5), id, &mut cx.local.state.transfer_id);
    can_send!(cx, frame);

    app::health_check_task::spawn_after(config::HEALTH_CHECK_PERIOD).ok();
}
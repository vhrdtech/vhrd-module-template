use crate::{app, config, };
use vhrdcan::frame::Frame;
use vhrdcan::FrameId;
use rtic::Mutex;
use rtic::rtic_monotonic::Milliseconds;

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
    let id = FrameId::new_extended(0x456).unwrap();
    let frame = Frame::new_move(id, payload, 8).unwrap();
    can_send!(cx, frame);
    app::health_check_task::spawn_after(config::HEALTH_CHECK_PERIOD).ok();
}
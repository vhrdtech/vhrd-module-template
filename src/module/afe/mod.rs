use crate::prelude::*;
use rtic::Mutex;
// #[cfg(feature = "module-afe-hx711")]
// mod hx711_uses {
    use stm32f0xx_hal::gpio::{Floating, Input, Output, PushPull};
    use stm32f0xx_hal::gpio::gpioa::{PA8, PA10};
    use stm32f0xx_hal::gpio::gpiob::{PB6, PB7, PB8};
    use hx711::Hx711;
    use tim_systick_monotonic::MonotonicHandle;
    use embedded_hal::digital::v2::OutputPin;
    use nb::block;
use uavcan_llr::slicer::{Slicer, OwnedSlice};
use core::cell::RefCell;

const MAX_NOT_JUNK_THRUST: i32 = i32::MAX;
const MAX_NOT_JUNK_TORQUE: i32 = i32::MAX;

pub type Hx711Rate = PA8<Output<PushPull>>;
pub type Hx711Instance = Hx711<DummyDelay, PA10<Input<Floating>>, PB6<Output<PushPull>>>;
// }
// #[cfg(feature = "module-afe-hx711")]
// use hx711_uses::*;

#[derive(Default)]
pub struct State {
    thrust_transfer_id: TransferId,
    torque_transfer_id: TransferId,
}

impl State {
    pub const fn new() -> Self {
        State {
            thrust_transfer_id: TransferId::new(0).unwrap(),
            torque_transfer_id: TransferId::new(0).unwrap(),
        }
    }
}

#[cfg(feature = "module-afe-hx711")]
pub fn init_hx711(
    _delay: MonotonicHandle,
    hx_rate: PA8<Input<Floating>>,
    hx_sck: PB6<Input<Floating>>,
    hx_dout: PA10<Input<Floating>>,
    ib1_en: PB7<Input<Floating>>,
    ib2_en: PB8<Input<Floating>>,
) -> (Hx711Rate, Hx711Instance) {
    let (mut hx_rate, hx_sck, mut ib1_en, mut ib2_en) = cortex_m::interrupt::free(|cs| {
        (
            hx_rate.into_push_pull_output(cs),
            hx_sck.into_push_pull_output(cs),
            ib1_en.into_push_pull_output(cs),
            ib2_en.into_push_pull_output(cs),
        )
    });
    // loop {
    //     log_debug!("{}", delay.tim_now());
    //     cortex_m::asm::delay(1_000_000);
    // }
    hx_rate.set_low().ok();
    ib1_en.set_low().ok();
    ib2_en.set_low().ok();
    let hx711 = Hx711::new(DummyDelay{}, hx_dout, hx_sck).unwrap();
    (
        hx_rate,
        hx711
    )
}

static REZERO_FLAG: bare_metal::Mutex<RefCell<bool>> = bare_metal::Mutex::new(RefCell::new(false));

fn zero_afe(hx711: &mut Hx711Instance) -> (i32, i32) {
    let mut buf = [0i32; 8];
    let mut i = 0;
    const READING_IS_JUNK_DELTA: i32 = 1000;

    hx711.set_mode(hx711::Mode::ChAGain128).ok();
    let _skip = block!(hx711.retrieve()).unwrap();
    let _skip = block!(hx711.retrieve()).unwrap();

    for x in buf.iter_mut() {
        *x = block!(hx711.retrieve()).unwrap();
    }
    let mean_dirty: i32 = buf.iter().sum::<i32>() / buf.len() as i32;
    log_debug!("torque_0_dirty: {} buf: {:?}", mean_dirty, buf);
    let mut mean_clean = 0;
    let mut clean_count = 0;
    for x in buf {
        if (x - mean_dirty) < READING_IS_JUNK_DELTA {
            mean_clean += x;
            clean_count += 1;
        }
    }
    let torque_0 = mean_clean / clean_count;
    log_debug!("torque_0: {}", torque_0);

    hx711.set_mode(hx711::Mode::ChBGain32).ok();
    let _skip = block!(hx711.retrieve()).unwrap();
    let _skip = block!(hx711.retrieve()).unwrap();

    for x in buf.iter_mut() {
        *x = block!(hx711.retrieve()).unwrap();
    }
    let mean_dirty: i32 = buf.iter().sum::<i32>() / buf.len() as i32;
    log_debug!("thrust_0_dirty: {} buf: {:?}", mean_dirty, buf);
    let mut mean_clean = 0;
    let mut clean_count = 0;
    for x in buf {
        if (x - mean_dirty) < READING_IS_JUNK_DELTA {
            mean_clean += x;
            clean_count += 1;
        }
    }
    let thrust_0 = mean_clean / clean_count;
    log_debug!("thrust_0: {}", thrust_0);
    (torque_0, thrust_0)
}

#[cfg(feature = "module-afe-hx711")]
pub fn idle(mut cx: app::idle::Context) -> ! {
    let hx711: &mut Hx711Instance = cx.local.hx711;

    let (mut torque_0, mut thrust_0) = zero_afe(hx711);
    loop {
        let rezero = cortex_m::interrupt::free(|cs| REZERO_FLAG.borrow(cs).replace_with(|_| false));
        if rezero {
            log_info!("Zero AFE in loop");
            let zero = zero_afe(hx711);
            torque_0 = zero.0;
            thrust_0 = zero.1;
        }

        hx711.set_mode(hx711::Mode::ChAGain128).ok();
        let skip_1 = block!(hx711.retrieve()).unwrap();
        let torque = nb::block!(hx711.retrieve()).unwrap() - torque_0;
        hx711.set_mode(hx711::Mode::ChBGain32).ok();
        let skip_2 = block!(hx711.retrieve()).unwrap();
        let thrust = nb::block!(hx711.retrieve()).unwrap() - thrust_0;
        log_info!("thrust: {}\ttorque: {}\t{}\t{}", thrust, torque, skip_1, skip_2);

        if torque.abs() <= MAX_NOT_JUNK_TORQUE as i32 {
            let id = CanId::new_message_kind(config::UAVCAN_NODE_ID, SubjectId::new(20).unwrap(), false, Priority::Nominal);
            let frame = Slicer::<8>::new_single(OwnedSlice::from_slice(&torque.to_be_bytes()).unwrap(), id, &mut cx.local.state.torque_transfer_id);
            can_send!(cx, frame);
        }

        if thrust.abs() <= MAX_NOT_JUNK_THRUST as i32 {
            let id = CanId::new_message_kind(config::UAVCAN_NODE_ID, SubjectId::new(21).unwrap(), false, Priority::Nominal);
            let frame = Slicer::<8>::new_single(OwnedSlice::from_slice(&thrust.to_be_bytes()).unwrap(), id, &mut cx.local.state.thrust_transfer_id);
            can_send!(cx, frame);
        }
    }
}

#[cfg(feature = "module-afe-hx711")]
pub struct DummyDelay {}
#[cfg(feature = "module-afe-hx711")]
impl embedded_hal::blocking::delay::DelayUs<u32> for DummyDelay {
    fn delay_us(&mut self, us: u32) {
        cortex_m::asm::delay(us * 8);
    }
}


#[cfg(feature = "module-afe-lmp")]
pub fn init_lmp() {

}

#[cfg(feature = "module-afe-lmp")]
pub fn idle(_cx: app::idle::Context) -> ! {
    loop {
        cortex_m::asm::delay(1_000_000);
    }
}

pub fn handle_message(source: NodeId, message: Message, payload: &[u8]) {
    if source == config::PI_NODE_ID && message.subject_id == config::ZERO_AFE {
        log_info!("Zero AFE");
        cortex_m::interrupt::free(|cs| REZERO_FLAG.borrow(cs).replace(true));
    }
}

pub fn handle_service_request(source: NodeId, service: Service, payload: &[u8]) {

}

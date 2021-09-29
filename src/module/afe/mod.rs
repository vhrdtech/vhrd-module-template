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

#[cfg(feature = "module-afe-hx711")]
pub fn idle(mut cx: app::idle::Context) -> ! {
    let hx711: &mut Hx711Instance = cx.local.hx711;

    const N: i32 = 16;
    let mut val: i32 = 0;
    hx711.set_mode(hx711::Mode::ChAGain128).ok();
    for _ in 0..N {
        val += block!(hx711.retrieve()).unwrap(); // or unwrap, see features below
    }
    let torque_0 = val / N;
    log_debug!("torque_0: {}", torque_0);

    val = 0;
    hx711.set_mode(hx711::Mode::ChBGain32).ok();
    let _ = block!(hx711.retrieve());
    for _ in 0..N {
        val += block!(hx711.retrieve()).unwrap(); // or unwrap, see features below
    }
    let thrust_0 = val / N;
    log_debug!("thrust_0: {}", thrust_0);
    loop {
        hx711.set_mode(hx711::Mode::ChAGain128).ok();
        let skip_1 = block!(hx711.retrieve()).unwrap();
        let torque = nb::block!(hx711.retrieve()).unwrap() - torque_0;
        hx711.set_mode(hx711::Mode::ChBGain32).ok();
        let skip_2 = block!(hx711.retrieve()).unwrap();
        let thrust = nb::block!(hx711.retrieve()).unwrap() - thrust_0;
        log_info!("thrust: {}\ttorque: {}\t{}\t{}", thrust, torque, skip_1, skip_2);

        let id = CanId::new_message_kind(config::UAVCAN_NODE_ID, SubjectId::new(20).unwrap(), false, Priority::Nominal);
        let frame = Slicer::<8>::new_single(OwnedSlice::from_slice(&torque.to_be_bytes()).unwrap(), id, &mut cx.local.state.torque_transfer_id);
        can_send!(cx, frame);

        let id = CanId::new_message_kind(config::UAVCAN_NODE_ID, SubjectId::new(21).unwrap(), false, Priority::Nominal);
        let frame = Slicer::<8>::new_single(OwnedSlice::from_slice(&thrust.to_be_bytes()).unwrap(), id, &mut cx.local.state.thrust_transfer_id);
        can_send!(cx, frame);
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

}

pub fn handle_service_request(source: NodeId, service: Service, payload: &[u8]) {

}

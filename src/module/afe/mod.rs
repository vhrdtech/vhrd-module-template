use crate::app;
use stm32f0xx_hal::gpio::{Floating, Input, Output, PushPull};
use stm32f0xx_hal::gpio::gpioa::{PA8, PA10};
use stm32f0xx_hal::gpio::gpiob::{PB6, PB7, PB8};
use hx711::Hx711;
use tim_systick_monotonic::MonotonicHandle;
use embedded_hal::digital::v2::OutputPin;
use nb::block;

pub type Hx711Rate = PA8<Output<PushPull>>;
pub type Hx711Instance = Hx711<DummyDelay, PA10<Input<Floating>>, PB6<Output<PushPull>>>;

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

pub fn idle(cx: app::idle::Context) -> ! {
    let hx711: &mut Hx711Instance = cx.local.hx711;

    const N: i32 = 16;
    let mut val: i32 = 0;
    hx711.set_mode(hx711::Mode::ChAGain128).ok();
    for _ in 0..N {
        val += block!(hx711.retrieve()).unwrap(); // or unwrap, see features below
    }
    let thrust_0 = val / N;
    log_debug!("thrust_0: {}", thrust_0);

    val = 0;
    hx711.set_mode(hx711::Mode::ChBGain32).ok();
    let _ = block!(hx711.retrieve());
    for _ in 0..N {
        val += block!(hx711.retrieve()).unwrap(); // or unwrap, see features below
    }
    let torque_0 = val / N;
    log_debug!("torque_0: {}", torque_0);
    loop {
        hx711.set_mode(hx711::Mode::ChAGain128).ok();
        let skip_1 = block!(hx711.retrieve()).unwrap();
        let thrust = nb::block!(hx711.retrieve()).unwrap();
        hx711.set_mode(hx711::Mode::ChBGain32).ok();
        let skip_2 = block!(hx711.retrieve()).unwrap();
        let torque = nb::block!(hx711.retrieve()).unwrap();
        log_info!("thrust: {}\ttorque: {}\t{}\t{}", thrust - thrust_0, torque - torque_0, skip_1, skip_2);
    }
}

pub struct DummyDelay {}
impl embedded_hal::blocking::delay::DelayUs<u32> for DummyDelay {
    fn delay_us(&mut self, us: u32) {
        cortex_m::asm::delay(us * 8);
    }
}
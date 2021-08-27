use crate::app;
use stm32f0xx_hal::gpio::{Floating, Input};
use stm32f0xx_hal::gpio::gpiob::PB0;
use embedded_hal::digital::v2::OutputPin;

pub struct Resources {

}

pub fn init(pi_en: PB0<Input<Floating>>) -> Resources {
    let (mut pi_en, ) = cortex_m::interrupt::free(|cs| {
        (
            pi_en.into_push_pull_output(cs),
        )
    });
    pi_en.set_high().ok();

    Resources {

    }
}

pub fn idle(_cx: app::idle::Context) -> ! {
    loop {
        cortex_m::asm::delay(1_000_000);
    }
}

pub fn can_rx_router(_cx: app::can_rx_router::Context) {

}
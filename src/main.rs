#![no_std]
#![no_main]

#[macro_use]
mod logging;
mod error_handlers;
mod vt100;
mod blinker;
mod config;
mod units;

use stm32f0xx_hal as hal;
use stm32f0xx_hal::stm32 as pac;
use rtic::app;

#[app(device = stm32f0xx_hal::stm32, peripherals = true, dispatchers = [TSC])]
mod app {
    use tim_systick_monotonic::TimSystickMonotonic;
    use stm32f0xx_hal::prelude::*;
    use crate::blinker::{blinker_task, BlinkerEvent, BlinkerState};
    // use rtt_target::{rtt_init_default, rprintln, rtt_init_print};
    use super::logging;
    use crate::log_info;
    use crate::blinker::Blinker;

    #[shared]
    struct Shared {
        blinker: Blinker,
    }

    #[local]
    struct Local {}

    #[monotonic(binds = SysTick, default = true)]
    type MyMono = TimSystickMonotonic<8_000_000>;

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        logging::init();
        log_info!("abc {}", 1);

        let cp = cx.core;
        let mut dp: super::pac::Peripherals = cx.device;
        let mono = TimSystickMonotonic::new(cp.SYST, dp.TIM15, dp.TIM17, 8_000_000);
        let mut rcc = dp.RCC.configure().sysclk(8.mhz()).freeze(&mut dp.FLASH);

        let gpioa = dp.GPIOA.split(&mut rcc);
        let (
            led,
            _smth
        ) = cortex_m::interrupt::free(|cs| {
            (
                gpioa.pa6,
                gpioa.pa7
            )
        });
        let mut blinker = Blinker::new(dp.TIM16, led, &rcc);
        blinker.set_global_brigthness_percent(15);
        blinker_task::spawn(BlinkerEvent::SetState(BlinkerState::Breath)).ok();

        (
            Shared{
                blinker,
            },
            Local{},
            init::Monotonics(mono)
        )
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        loop {
            // rprintln!("idle");
            cortex_m::asm::delay(500_000);
        }
    }

    extern "Rust" {
        #[task(shared = [blinker])]
        fn blinker_task(cx: blinker_task::Context, e: crate::blinker::BlinkerEvent);
    }
}

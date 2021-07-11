#![no_std]
#![no_main]

mod error_handlers;

use stm32f0xx_hal::stm32 as pac;
use rtic::app;

use stm32f0xx_hal::prelude::_embedded_hal_gpio_ToggleableOutputPin;
use rtic::Mutex;
use rtic::time::duration::{Seconds, Milliseconds};
use rtt_target::rprintln;
fn blinker(mut cx: app::blinker::Context) {
    rprintln!("blinker");
    cx.shared.led.lock(|led| led.toggle().ok());
    let r = app::blinker::spawn_after(Milliseconds(100_u32));
    rprintln!("spawn:{:?}", r.is_ok());
}

#[app(device = stm32f0xx_hal::stm32, peripherals = true, dispatchers = [TSC])]
mod app {
    use tim_systick_monotonic::TimSystickMonotonic;
    use stm32f0xx_hal::prelude::*;
    use stm32f0xx_hal::gpio::gpioa::PA6;
    use stm32f0xx_hal::gpio::{Output, PushPull};
    use crate::blinker;
    use rtt_target::{rtt_init_default, rprintln, rtt_init_print};

    #[shared]
    struct Shared {
        led: PA6<Output<PushPull>>,
    }

    #[local]
    struct Local {}

    #[monotonic(binds = SysTick, default = true)]
    type MyMono = TimSystickMonotonic<8_000_000>;

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        rtt_init_print!();
        rprintln!("Hey");

        let cp = cx.core;
        let mut dp: super::pac::Peripherals = cx.device;
        let mono = TimSystickMonotonic::new(cp.SYST, dp.TIM15, dp.TIM17, 8_000_000);
        let mut rcc = dp.RCC.configure().sysclk(8.mhz()).freeze(&mut dp.FLASH);

        let gpioa = dp.GPIOA.split(&mut rcc);
        let (
            mut led
        ) = cortex_m::interrupt::free(|cs| {
            (
                gpioa.pa6.into_push_pull_output(cs)
            )
        });
        blinker::spawn().unwrap();

        (
            Shared{
                led
            },
            Local{},
            init::Monotonics(mono)
        )
    }

    #[idle]
    fn idle(cx: idle::Context) -> ! {
        loop {
            rprintln!("idle");
            cortex_m::asm::delay(500_000);
        }
    }

    extern "Rust" {
        #[task(shared = [led])]
        fn blinker(cx: blinker::Context);
    }
}

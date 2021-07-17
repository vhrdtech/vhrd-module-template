#![no_std]
#![no_main]

#[macro_use]
mod logging;
mod error_handlers;
mod vt100;
mod blinker;
mod config;
mod units;
mod canbus;

use stm32f0xx_hal as hal;
use stm32f0xx_hal::stm32 as pac;
use rtic::app;

#[app(device = stm32f0xx_hal::stm32, peripherals = true, dispatchers = [TSC, FLASH])]
mod app {
    use crate::config;
    use tim_systick_monotonic::TimSystickMonotonic;
    use stm32f0xx_hal::prelude::*;
    use crate::blinker::{blinker_task, BlinkerEvent, BlinkerState};
    // use rtt_target::{rtt_init_default, rprintln, rtt_init_print};
    use super::logging;
    use crate::log_info;
    use crate::blinker::Blinker;
    use embedded_time::duration::Milliseconds;
    use crate::canbus;
    use stm32f0xx_hal::gpio::gpiob::{PB3, PB5, PB4};
    use stm32f0xx_hal::gpio::{PushPull, AF0, Output, Alternate};
    use stm32f0xx_hal::spi::Spi;
    use stm32f0xx_hal::gpio::gpioc::PC14;

    #[shared]
    struct Shared {
        blinker: Blinker,

        #[cfg(feature = "can-mcp25625")]
        mcp25625: Option<config::Mcp25625Instance>,
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
        let gpiob = dp.GPIOB.split(&mut rcc);
        let gpioc = dp.GPIOC.split(&mut rcc);
        let (
            led,
            _mcp25625_sck,
            _mcp25625_miso,
            _mcp25625_mosi,
            _mcp25625_cs,

        ) = cortex_m::interrupt::free(|cs| {
            (
                gpioa.pa6,

                gpiob.pb3.into_alternate_af0(cs),
                gpiob.pb4.into_alternate_af0(cs),
                gpiob.pb5.into_alternate_af0(cs),
                gpioc.pc14.into_push_pull_output(cs),


            )
        });

        #[cfg(feature = "can-mcp25625")]
        let mcp25625 = match canbus::mcp25625_init(dp.SPI1, _mcp25625_sck, _mcp25625_miso, _mcp25625_mosi, _mcp25625_cs, &mut rcc) {
            Ok(mcp25625) => {
                log_info!("Mcp25625 init ok");
                Some(mcp25625)
            }
            Err(e) => {
                log_error!("Mcp25625 init error: {:?}", e);
                None
            }
        };


        let mut blinker = Blinker::new(dp.TIM16, led, &rcc);
        blinker.set_global_brigthness_percent(15);
        blinker_task::spawn(BlinkerEvent::SetState(BlinkerState::Breath)).ok();

        // #[used]
        // #[no_mangle]
        // #[export_name = "_COUNTERS"]
        // pub static mut COUNTERS: [u32; 16] = [0; 16];
        //
        // test_task::spawn().ok();
        // test_task2::spawn().ok();

        (
            Shared{
                blinker,
                #[cfg(feature = "can-mcp25625")]
                mcp25625,

            },
            Local{},
            init::Monotonics(mono)
        )
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        loop {
            // rprintln!("idle");
            // extern "C" {
            //     #[link_name = "_COUNTERS"]
            //     static mut COUNTERS: [u32; 16];
            // }
            // #[export_name = "the string that will be interned"]
            // #[link_section = ".counters.some_unique_identifier"]
            // static SYM: u8 = 0;
            // let index = &SYM as *const u8 as usize;
            // unsafe {
            //     let ctrs = &mut COUNTERS;
            //     ctrs[index] += 1;
            // }
            log_info!("idle");
            cortex_m::asm::delay(500_000);
        }
    }

    // #[task(shared = [], priority = 2)]
    // fn test_task2(_cx: test_task2::Context) {
    //     extern "C" {
    //         #[link_name = "_COUNTERS"]
    //         static mut COUNTERS: [u32; 16];
    //     }
    //     #[export_name = "the string that will be interned 2"]
    //     #[link_section = ".counters.some_unique_identifier 2"]
    //     static SYM: u8 = 0;
    //     let index = &SYM as *const u8 as usize;
    //     unsafe {
    //         let ctrs = &mut COUNTERS;
    //         ctrs[index] += 1;
    //     }
    //     test_task2::spawn_after(Milliseconds::new(110u32)).ok();
    // }
    //
    // #[task(shared = [blinker], priority = 2)]
    // fn test_task(_cx: test_task::Context) {
    //     // let x = unsafe { cx.shared.blinker.priority() };
    //     // let x = unsafe { *(x as *const _ as *const u8)};
    //     // log_info!("x: {}", x);
    //
    //     extern "C" {
    //         #[link_name = "_COUNTERS"]
    //         static mut COUNTERS: [u32; 16];
    //     }
    //     unsafe {
    //         let ctrs = &mut COUNTERS;
    //         log_debug!("ctrs[0]: {} [1]: {}", ctrs[0], ctrs[1]);
    //     }
    //     test_task::spawn_after(Milliseconds::new(500u32)).ok();
    // }

    extern "Rust" {
        #[task(shared = [blinker], capacity = 2)]
        fn blinker_task(cx: blinker_task::Context, e: crate::blinker::BlinkerEvent);
    }
}

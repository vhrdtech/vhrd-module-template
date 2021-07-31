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
mod module;

use stm32f0xx_hal as hal;
use stm32f0xx_hal::stm32 as pac;
use rtic::app;

#[app(device = stm32f0xx_hal::stm32, peripherals = true, dispatchers = [TSC, FLASH])]
mod app {
    use crate::config;
    use tim_systick_monotonic::TimSystickMonotonic;
    use stm32f0xx_hal::prelude::*;
    use crate::blinker::{blinker_task, BlinkerEvent, BlinkerState};
    use crate::canbus::{can_stm_task};
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
    use stm32f0xx_hal::exti::{GpioLine, ExtiLine, Exti, TriggerEdge};
    use stm32f0xx_hal::syscfg::SYSCFG;
    use stm32f0xx_hal::pac::Interrupt;

    #[shared]
    struct Shared {
        mr: crate::module::Resources,

        blinker: Blinker,

        mcp_irq: config::Mcp25625Irq,
        #[cfg(feature = "can-stm")]
        can_stm: config::CanStmInstance,
    }

    #[local]
    struct Local {

        #[cfg(feature = "can-mcp25625")]
        can_mcp25625: Option<config::Mcp25625Instance>,
    }

    #[monotonic(binds = SysTick, default = true)]
    type MyMono = TimSystickMonotonic<8_000_000>;

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        logging::init();
        log_info!("info");
        log_debug!("debug");
        log_error!("error");
        log_warn!("warn");
        log_trace!("trace");

        let cp = cx.core;
        let mut dp: super::pac::Peripherals = cx.device;
        let mono = TimSystickMonotonic::new(cp.SYST, dp.TIM15, dp.TIM17, 8_000_000);
        let mut rcc = dp.RCC.configure().sysclk(8.mhz()).freeze(&mut dp.FLASH);
        let mut exti = Exti::new(dp.EXTI);
        let mut syscfg = SYSCFG::new(dp.SYSCFG, &mut rcc);

        let gpioa = dp.GPIOA.split(&mut rcc);
        let gpiob = dp.GPIOB.split(&mut rcc);
        let gpioc = dp.GPIOC.split(&mut rcc);
        let (
            led,

            _can_rx,
            _can_tx,
            mut can_stby,

            _mcp25625_sck,
            _mcp25625_miso,
            _mcp25625_mosi,
            _mcp25625_cs,
            mcp_irq,

        ) = cortex_m::interrupt::free(|cs| {
            (
                gpioa.pa6,

                gpioa.pa11.into_alternate_af4(cs),
                gpioa.pa12.into_alternate_af4(cs),
                gpioa.pa15.into_push_pull_output(cs),

                gpiob.pb3.into_alternate_af0(cs),
                gpiob.pb4.into_alternate_af0(cs),
                gpiob.pb5.into_alternate_af0(cs),
                gpioc.pc14.into_push_pull_output(cs),
                gpioc.pc15.into_pull_up_input(cs),


            )
        });
        can_stby.set_low().ok();

        #[cfg(feature = "can-mcp25625")]
        let can_mcp25625 = match canbus::can_mcp25625_init(dp.SPI1, _mcp25625_sck, _mcp25625_miso, _mcp25625_mosi, _mcp25625_cs, &mut rcc) {
            Ok(mcp25625) => {
                log_info!("Mcp25625 init ok");
                let mcp_irq_line = GpioLine::from_raw_line(mcp_irq.pin_number()).unwrap();
                exti.listen_gpio(&mut syscfg, mcp_irq.port(), mcp_irq_line, TriggerEdge::Falling);
                Some(mcp25625)
            }
            Err(e) => {
                log_error!("Mcp25625 init error: {:?}", e);
                None
            }
        };

        #[cfg(feature = "can-stm")]
        let can_stm = canbus::can_stm_init(dp.CAN, _can_tx, _can_rx, &mut rcc);


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

        #[cfg(feature = "module-button")]
        let mr = crate::module::button::init();
        #[cfg(feature = "module-led")]
        let mr = crate::module::led::init();
        #[cfg(feature = "module-pi")]
        let mr = crate::module::pi::init();
        #[cfg(feature = "module-afe")]
        let mr = crate::module::afe::init();

        (
            Shared{
                mr,

                blinker,

                mcp_irq,
                #[cfg(feature = "can-stm")]
                can_stm,

            },
            Local{

                #[cfg(feature = "can-mcp25625")]
                can_mcp25625,
            },
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
            // log_info!("idle");
            cortex_m::asm::delay(500_000);
            rtic::pend(Interrupt::EXTI4_15);
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

    #[task(binds = EXTI4_15, shared = [mcp_irq], local = [can_mcp25625])]
    fn exti_4_15(mut cx: exti_4_15::Context) {
        let mcp_irq_line = GpioLine::from_raw_line(cx.shared.mcp_irq.lock(|pin| pin.pin_number())).unwrap();
        Exti::unpend(mcp_irq_line);
        #[cfg(feature = "can-mcp25625")]
        crate::canbus::can_mcp25625_irq(&mut cx);
    }

    extern "Rust" {
        #[task(shared = [blinker], capacity = 2)]
        fn blinker_task(cx: blinker_task::Context, e: crate::blinker::BlinkerEvent);

        #[task(binds = CEC_CAN, shared = [can_stm])]
        fn can_stm_task(cx: can_stm_task::Context);


    }
}
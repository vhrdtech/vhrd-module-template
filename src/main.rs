#![no_std]
#![no_main]
#![feature(const_option)]

use rtic::app;
use stm32f0xx_hal as hal;
use stm32f0xx_hal::stm32 as pac;

#[macro_use]
mod logging;
#[macro_use]
mod canbus;
mod error_handlers;
mod vt100;
mod config;
mod units;
mod module;
mod task;

#[app(device = stm32f0xx_hal::stm32, peripherals = true, dispatchers = [TSC, FLASH])]
mod app {
    #[allow(unused_imports)]
    use crate::hal;
    #[allow(unused_imports)]
    use crate::module;
    use cfg_if::cfg_if;
    use stm32f0xx_hal::exti::{Exti, };
    use stm32f0xx_hal::prelude::*;
    use stm32f0xx_hal::syscfg::SYSCFG;
    use tim_systick_monotonic::TimSystickMonotonic;

    use crate::canbus;
    use crate::config;
    use crate::log_info;
    use crate::task::blink::{blink_task, BlinkerEvent, BlinkerState};
    use crate::task::blink::Blinker;
    use crate::task::health_check::health_check_task;

    // use rtt_target::{rtt_init_default, rprintln, rtt_init_print};
    use super::logging;

    #[shared]
    struct Shared {
        can_tx: config::CanTxQueue,
        can_rx: config::CanRxQueue,

        blinker: Blinker,
        uptime: u32,
        health: crate::task::health_check::Health,


    }

    #[local]
    struct Local {
        #[cfg(feature = "can-mcp25625")]
        can_mcp25625: Option<config::Mcp25625Instance>,
        #[cfg(feature = "can-mcp25625")]
        mcp_irq: config::Mcp25625Irq,
        #[cfg(feature = "can-stm")]
        can_stm: config::CanStmInstance,

        #[cfg(feature = "module-afe-hx711")]
        hx711_rate: module::afe::Hx711Rate,
        #[cfg(feature = "module-afe-hx711")]
        hx711: module::afe::Hx711Instance,
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
        #[allow(unused_mut, unused_variables)]
        let mut exti = Exti::new(dp.EXTI);
        #[allow(unused_mut, unused_variables)]
        let mut syscfg = SYSCFG::new(dp.SYSCFG, &mut rcc);

        // let mut d = mono.new_handle();
        // loop {
            // lox  g_debug!("cnt: {}", unsafe { *(&mono.tim_msb.cnt as *const _ as *const u32) });
            // log_debug!("{}", mono.tim_now());
            // log_debug!("\n\n\n\n");
            // let delay_cycles = 8_000_000 as u64 * (10_000 * 1_000) as u64 / 1_000_000;
            // let mut elapsed = 0;
            // let mut count_prev = mono.tim_now();
            // while elapsed < delay_cycles {
            //     let count_now = mono.tim_now();
            //     let dt = count_now.wrapping_sub(count_prev);
            //     count_prev = count_now;
            //     elapsed += dt as u64;
            //
            //     log_debug!("dt:{} el:{} rem:{}", dt, elapsed, delay_cycles - elapsed);
            //     cortex_m::asm::delay(10_000);
            // }
        // }
        // loop {
        //     log_debug!("loop");
        //     d.delay_us(100_000_00);
        // }
        let gpioa = dp.GPIOA.split(&mut rcc);
        let gpiob = dp.GPIOB.split(&mut rcc);
        let gpioc = dp.GPIOC.split(&mut rcc);
        #[allow(unused_variables)]
        let (
            led,

            _pa8, _pa10,

            _can_rx,
            _can_tx,
            mut can_stby,

            _mcp25625_sck,
            _mcp25625_miso,
            _mcp25625_mosi,
            _pb6,
            _pb7,
            _pb8,
            _mcp25625_cs,
            mcp_irq,

        ) = cortex_m::interrupt::free(|cs| {
            (
                gpioa.pa6,

                gpioa.pa8, gpioa.pa10,

                gpioa.pa11.into_alternate_af4(cs),
                gpioa.pa12.into_alternate_af4(cs),
                gpioa.pa15.into_push_pull_output(cs),

                gpiob.pb3.into_alternate_af0(cs),
                gpiob.pb4.into_alternate_af0(cs),
                gpiob.pb5.into_alternate_af0(cs),
                gpiob.pb6,
                gpiob.pb7,
                gpiob.pb8,
                gpioc.pc14.into_push_pull_output(cs),
                gpioc.pc15.into_pull_up_input(cs),


            )
        });
        can_stby.set_low().ok();

        #[cfg(feature = "can-mcp25625")]
        let can_mcp25625 = match canbus::can_mcp25625_init(dp.SPI1, _mcp25625_sck, _mcp25625_miso, _mcp25625_mosi, _mcp25625_cs, &mut rcc) {
            Ok(mcp25625) => {
                log_info!("Mcp25625 init ok");
                use hal::exti::{GpioLine, TriggerEdge, ExtiLine};
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
        blink_task::spawn(BlinkerEvent::SetState(BlinkerState::Breath)).ok();

        health_check_task::spawn().ok();

        // #[used]
        // #[no_mangle]
        // #[export_name = "_COUNTERS"]
        // pub static mut COUNTERS: [u32; 16] = [0; 16];
        //
        // test_task::spawn().ok();
        // test_task2::spawn().ok();

        #[cfg(feature = "module-button")]
        let _mr = crate::module::button::init();
        #[cfg(feature = "module-led")]
        let mr = crate::module::led::init();
        #[cfg(feature = "module-pi")]
        let mr = crate::module::pi::init();
        #[cfg(feature = "module-afe-hx711")]
        let (hx711_rate, hx711) = crate::module::afe::init_hx711(mono.new_handle(),_pa8, _pb6, _pa10, _pb7, _pb8);

        (
            Shared{
                can_tx: heapless::BinaryHeap::new(),
                can_rx: heapless::BinaryHeap::new(),

                blinker,
                uptime: 0,
                health: crate::task::health_check::Health::Norminal,
            },
            Local{
                #[cfg(feature = "can-mcp25625")]
                can_mcp25625,
                #[cfg(feature = "can-mcp25625")]
                mcp_irq,
                #[cfg(feature = "can-stm")]
                can_stm,

                #[cfg(feature = "module-afe-hx711")]
                hx711_rate,
                #[cfg(feature = "module-afe-hx711")]
                hx711,
            },
            init::Monotonics(mono)
        )
    }

    #[idle(local = [hx711, hx711_rate])]
    fn idle(cx: idle::Context) -> ! {
        // loop {
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
            #[cfg(feature = "module-button")]
            crate::module::button::idle(cx);
            #[cfg(feature = "module-led")]
            crate::module::led::idle(cx);
            #[cfg(feature = "module-pi")]
            crate::module::pi::idle(cx);
            #[cfg(feature = "module-afe")]
            crate::module::afe::idle(cx);
        // }
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


    #[task(shared = [can_rx])]
    fn can_rx_router(_cx: can_rx_router::Context) {

    }

    #[task(binds = EXTI4_15, shared = [can_tx, can_rx], local = [can_mcp25625, mcp_irq])]
    #[allow(unused_mut)]
    fn exti_4_15(mut cx: exti_4_15::Context) {
        cfg_if! {
            if #[cfg(feature = "can-mcp25625")] {
                use hal::exti::{GpioLine, ExtiLine};
                let mcp_irq_line = GpioLine::from_raw_line(cx.local.mcp_irq.pin_number()).unwrap();
                Exti::unpend(mcp_irq_line);
                crate::canbus::can_mcp25625_irq(&mut cx);
            } else {
                let _cx = cx;
            }
        }
    }

    #[task(
        binds = CEC_CAN,
        shared = [can_tx, can_rx],
        local = [
            can_stm,

            state: crate::canbus::CanStmState = crate::canbus::CanStmState::new(),
        ]
    )]
    #[allow(unused_variables)]
    fn can_stm_task(cx: can_stm_task::Context) {
        #[cfg(feature = "can-stm")]
        crate::canbus::can_stm_task(cx);
    }

    extern "Rust" {
        #[task(shared = [blinker], capacity = 2)]
        fn blink_task(cx: blink_task::Context, e: crate::task::blink::BlinkerEvent);

        #[task(
            shared = [can_tx, uptime, health, ],
            local = [
                state: crate::task::health_check::State = crate::task::health_check::State::new()
            ]
        )]
        fn health_check_task(mut cx: health_check_task::Context);


    }
}

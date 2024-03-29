#![no_std]
#![no_main]
#![feature(const_option)]

use rtic::app;
use stm32f0xx_hal as hal;
use stm32f0xx_hal::stm32 as pac;

#[macro_use]
extern crate static_assertions;

#[macro_use]
mod logging;
#[macro_use]
mod canbus;
mod error_handlers;
mod vt100;
pub mod config;
mod units;
mod module;
mod task;
mod prelude;
// mod ramp_generator;
mod utils;
mod ramp_vesc;
// mod tf_vesc;
mod ramp_generator2;

#[cfg(feature = "module-led")]
pub const SYS_CLK_HZ: u32 = 48_000_000;
#[cfg(feature = "module-pi")]
pub const SYS_CLK_HZ: u32 = 8_000_000;
#[cfg(not(any(feature = "module-led", feature = "module-pi")))]
pub const SYS_CLK_HZ: u32 = 8_000_000;
pub type TimMono = tim_systick_monotonic::TimSystickMonotonic<SYS_CLK_HZ>;

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

    use crate::{canbus, SYS_CLK_HZ};
    use crate::config;
    use crate::log_info;
    use crate::task::blink::{blink_task, BlinkerEvent, BlinkerState};
    use crate::task::blink::Blinker;
    use crate::task::health_check::health_check_task;
    // use crate::module::can_rx_router;
    use crate::canbus::can_rx_router;

    // use rtt_target::{rtt_init_default, rprintln, rtt_init_print};
    use super::logging;
    // use stm32f0xx_hal::rcc::HSEBypassMode;

    #[shared]
    struct Shared {
        #[cfg(feature = "can-stm")]
        can_stm_tx: config::CanTxQueue,
        #[cfg(feature = "can-stm")]
        can_stm_rx: config::CanRxQueue,
        #[cfg(feature = "can-mcp25625")]
        can_mcp_tx: config::CanTxQueue,
        #[cfg(feature = "can-mcp25625")]
        can_mcp_rx: config::CanRxQueue,

        blinker: Blinker,
        uptime: u32,
        health: crate::task::health_check::Health,

        #[cfg(feature = "module-led")]
        drv8323: Option<module::led::Drv8323Instance>,
 //       #[cfg(feature = "module-led")]
//        stand_state: module::led::StandState,

        #[cfg(feature = "vesc-ctrl")]
        vesc_feedback: Option<crate::ramp_vesc::VescFeedback>,
        #[cfg(feature = "vesc-ctrl")]
        vesc_control_input: Option<crate::ramp_vesc::ControlInput>,
        #[cfg(feature = "vesc-ctrl")]
        vesc_watchdog_input: Option<i32>,
        #[cfg(feature = "vesc-ctrl")]
            vesc_watchdog_triggered: Option<()>,
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

        #[cfg(feature = "module-button")]
        mr: module::button::Resources,

        #[cfg(feature = "module-pi")]
        pi_en: module::pi::PiEn,
    }

    #[monotonic(binds = SysTick, default = true)]
    type TimMono = crate::TimMono;

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
        let mono = TimSystickMonotonic::new(cp.SYST, dp.TIM15, dp.TIM17, SYS_CLK_HZ);

        // #[cfg(not(feature = "module-pi"))]
        let mut rcc = dp.RCC.configure().sysclk(SYS_CLK_HZ.hz()).freeze(&mut dp.FLASH);
        // cfg_if! {
        //     if #[cfg(feature = "module-pi")] {
        //         let mut rcc = dp.RCC.configure().hse(2.mhz(), HSEBypassMode::Bypassed).sysclk(40.mhz()).freeze(&mut dp.FLASH);
        //         let dp = unsafe { crate::hal::pac::Peripherals::steal() };
        //         dp.RCC.cfgr.modify(|_, w| w.mco().sysclk().mcopre().div1());
        //     }
        // }


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
            pa4,
            pa5,
            led,
            pa7,

            pa8, pa9, pa10,

            can_rx,
            can_tx,
            mut can_stby,

            pb0, pb1, pb2,
            mcp25625_sck,
            mcp25625_miso,
            mcp25625_mosi,
            pb6,
            pb7,
            pb8,
            pb9, pb10, pb11, pb12, pb13, pb14, pb15,

            mcp25625_cs,
            mcp_irq,

        ) = cortex_m::interrupt::free(|cs| {
            (
                gpioa.pa4,
                gpioa.pa5,
                gpioa.pa6,
                gpioa.pa7,

                gpioa.pa8, gpioa.pa9, gpioa.pa10,

                gpioa.pa11.into_alternate_af4(cs),
                gpioa.pa12.into_alternate_af4(cs),
                gpioa.pa15.into_push_pull_output(cs),

                gpiob.pb0,
                gpiob.pb1,
                gpiob.pb2,
                gpiob.pb3.into_alternate_af0(cs),
                gpiob.pb4.into_alternate_af0(cs),
                gpiob.pb5.into_alternate_af0(cs),
                gpiob.pb6,
                gpiob.pb7,
                gpiob.pb8,
                gpiob.pb9,
                gpiob.pb10,
                gpiob.pb11,
                gpiob.pb12,
                gpiob.pb13,
                gpiob.pb14,
                gpiob.pb15,

                gpioc.pc14.into_push_pull_output(cs),
                gpioc.pc15.into_pull_up_input(cs),


            )
        });
        can_stby.set_low().ok();
        #[cfg(feature = "module-pi")] {
            let _ = cortex_m::interrupt::free(|cs| pa8.into_alternate_af0(cs));
        }


        #[cfg(feature = "can-mcp25625")]
        let can_mcp25625 = match canbus::can_mcp25625_init(dp.SPI1, mcp25625_sck, mcp25625_miso, mcp25625_mosi, mcp25625_cs, &mut rcc) {
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
        let can_stm = canbus::can_stm_init(dp.CAN, can_tx, can_rx, &mut rcc);


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
        let mr = crate::module::button::init(pb0, pb1, pb2, pb12, pa5, pa7, pa8, dp.TIM14, &rcc);
        #[cfg(feature = "module-button")]
        button_task::spawn().ok();

        #[cfg(feature = "module-led")]
        let drv8323 = crate::module::led::init(pb8, pb9, pb13, pb14, pb15, pb7, pb12,  pa8, pb2, pa9, pa4, pa10, pa7, pb0, pb1, dp.SPI2, &mut rcc);
        #[cfg(feature = "module-led")]
        animation_task::spawn().ok();

        #[cfg(feature = "module-pi")]
        let pi_en = crate::module::pi::init(pb0, pb2);
        #[cfg(feature = "module-afe-hx711")]
        let (hx711_rate, hx711) = crate::module::afe::init_hx711(mono.new_handle(),pa8, pb6, pa10, pb7, pb8);
        #[cfg(feature = "module-afe-lmp")]
        let _ = crate::module::afe::init_lmp();

        #[cfg(feature = "vesc-ctrl")]
        ramp_vesc::spawn().ok();
        #[cfg(feature = "vesc-ctrl")]
        watchdog_vesc::spawn().ok();

        log_info!("Init succeeded, sysclk={}", rcc.clocks.sysclk().0);

        (
            Shared {
                #[cfg(feature = "can-stm")]
                can_stm_tx: heapless::BinaryHeap::new(),
                #[cfg(feature = "can-stm")]
                can_stm_rx: heapless::BinaryHeap::new(),
                #[cfg(feature = "can-mcp25625")]
                can_mcp_tx: heapless::BinaryHeap::new(),
                #[cfg(feature = "can-mcp25625")]
                can_mcp_rx: heapless::BinaryHeap::new(),

                blinker,
                uptime: 0,
                health: crate::task::health_check::Health::Norminal,

                #[cfg(feature = "module-led")]
                drv8323,
                #[cfg(feature = "module-led")]
                stand_state: module::led::StandState::new(),

                #[cfg(feature = "vesc-ctrl")]
                vesc_feedback: None,
                #[cfg(feature = "vesc-ctrl")]
                vesc_control_input: None,
                #[cfg(feature = "vesc-ctrl")]
                vesc_watchdog_input: None,
                #[cfg(feature = "vesc-ctrl")]
                vesc_watchdog_triggered: None,
            },
            Local {
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

                #[cfg(feature = "module-button")]
                mr,

                #[cfg(feature = "module-pi")]
                pi_en,

            },
            init::Monotonics(mono)
        )
    }

    #[idle(
        shared = [
            can_stm_tx,
            can_mcp_tx,
        ],
        local = [
            hx711,
            hx711_rate,

            #[cfg(feature = "module-afe-hx711")]
            state: crate::module::afe::State = crate::module::afe::State::new()
        ]
    )]
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




    #[task(binds = EXTI4_15, shared = [can_mcp_tx, can_mcp_rx], local = [can_mcp25625, mcp_irq])]
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
        shared = [can_stm_tx, can_stm_rx],
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

    #[task(local = [mr], shared = [can_mcp_tx, can_stm_tx, ])]
    fn button_task(_cx: button_task::Context) {
        #[cfg(feature = "module-button")]
        module::button::button_task(_cx);
    }

    #[task(shared = [drv8323])]
    fn animation_task(_cx: animation_task::Context) {
        #[cfg(feature = "module-led")]
        module::led::animation_task(_cx);
    }

    // #[task(capacity = 2, shared = [], local = [
    //     state: crate::ramp_generator::State = crate::ramp_generator::State::new()
    // ])]
    // fn ramp_generator(_cx: ramp_generator::Context, _e: crate::ramp_generator::Event) {
    //     #[cfg(feature = "module-led")]
    //         crate::ramp_generator::ramp_generator(_cx, _e);
    // }

    #[task(capacity = 1, shared = [can_mcp_tx, can_stm_tx, vesc_feedback, vesc_control_input, vesc_watchdog_input, vesc_watchdog_triggered], local = [
        state: crate::ramp_vesc::State = crate::ramp_vesc::State::new()
    ])]
    fn ramp_vesc(_cx: ramp_vesc::Context) {
        #[cfg(feature = "vesc-ctrl")]
        crate::ramp_vesc::ramp_vesc(_cx);
    }

    #[task(capacity = 1, shared = [can_mcp_tx, can_stm_tx, vesc_watchdog_input, vesc_watchdog_triggered], local = [
        state: crate::ramp_vesc::WatchdogVescState = crate::ramp_vesc::WatchdogVescState::new()
    ])]
    fn watchdog_vesc(_cx: watchdog_vesc::Context) {
        #[cfg(feature = "vesc-ctrl")]
        crate::ramp_vesc::watchdog_vesc(_cx);
    }

    #[task(capacity = 1, local = [pi_en])]
    fn pi_task(_cx: pi_task::Context, _e: module::pi::Event) {
        #[cfg(feature = "module-pi")]
        crate::module::pi::pi_task(_cx, _e);
    }

//    #[task(capacity = 1, shared = [stand_state])]
 //   fn unpress_estop(mut cx: unpress_estop::Context) {
//        cx.shared.stand_state.lock(|s| s.is_estop_pressed = false);
  //      log_debug!("Estop UNpressed");
   // }

    extern "Rust" {
        #[task(shared = [blinker], capacity = 2)]
        fn blink_task(cx: blink_task::Context, e: crate::task::blink::BlinkerEvent);

        #[task(
            shared = [can_mcp_tx, can_stm_tx, uptime, health, ],
            local = [
                state: crate::task::health_check::State = crate::task::health_check::State::new()
            ]
        )]
        fn health_check_task(mut cx: health_check_task::Context);

        #[task(shared = [can_mcp_rx, can_stm_rx, vesc_feedback, vesc_control_input])]
        fn can_rx_router(_cx: can_rx_router::Context);

    }
}

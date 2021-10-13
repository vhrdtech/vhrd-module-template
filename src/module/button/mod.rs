use crate::prelude::*;

use stm32f0xx_hal::gpio::{Input, Floating, PullUp, Output, PushPull};
use stm32f0xx_hal::gpio::gpiob::{PB0, PB1, PB2, PB12};
use stm32f0xx_hal::gpio::gpioa::{PA8, PA5, PA7};
use embedded_hal::digital::v2::{InputPin, OutputPin};
use embedded_time::duration::Milliseconds;
use vhrdcan::{Frame, FrameId};
use crate::{hal, pac};

const BUTTON_CHECK_TIME: Milliseconds = Milliseconds(100);
const BUTTON_PRESS_TIME: Milliseconds = Milliseconds(1000);

const_assert!(BUTTON_PRESS_TIME.0 / BUTTON_CHECK_TIME.0 >= 1);

pub struct Resources {
    button0: PB0<Input<PullUp>>,
    button0_b: PB1<Input<PullUp>>,
    button1: PB2<Input<PullUp>>,
    button1_debounce: u8,
    button2: PB12<Input<PullUp>>,
    _led0: PA5<Output<PushPull>>,
    //led1: PA7<Output<PushPull>>,
    led2: PA8<Output<PushPull>>,
}

pub fn init(
    button0: PB0<Input<Floating>>,
    button0_b: PB1<Input<Floating>>,
    button1: PB2<Input<Floating>>,
    button2: PB12<Input<Floating>>,
    led0: PA5<Input<Floating>>,
    led1: PA7<Input<Floating>>,
    led2: PA8<Input<Floating>>,
    tim: pac::TIM14, rcc: &hal::rcc::Rcc,
) -> Resources {
    let (
        button0,
        button0_b,
        button1,
        button2,
        led0,
        _led1,
        led2
    ) = cortex_m::interrupt::free(|cs| (
        button0.into_pull_up_input(cs),
        button0_b.into_pull_up_input(cs),
        button1.into_pull_up_input(cs),
        button2.into_pull_up_input(cs),
        led0.into_push_pull_output(cs),
        led1.into_alternate_af4(cs),
        led2.into_push_pull_output(cs),
    ));

    init_tim14_ch1(tim, rcc);

    Resources {
        button0,
        button0_b,
        button1,
        button1_debounce: 0,
        button2,
        _led0: led0,
        //led1,
        led2
    }
}

pub fn button_task(mut cx: app::button_task::Context) {
    let mr: &mut Resources = cx.local.mr;
    let button0_is_pressed = mr.button0.is_high().unwrap();
    let button0_b_is_pressed = mr.button0_b.is_low().unwrap();
    let button1_is_pressed = mr.button1.is_low().unwrap();
    let button2_is_pressed = mr.button2.is_low().unwrap();
    log_info!("b0: {}, b0b: {}, b1: {}, b2: {}", button0_is_pressed, button0_b_is_pressed, button1_is_pressed, button2_is_pressed);

    if button1_is_pressed {
        mr.button1_debounce += 1;
        if mr.button1_debounce == (BUTTON_PRESS_TIME.0 / BUTTON_CHECK_TIME.0) as u8 {
            log_info!("Button1 pressed");
            let id = CanId::new_message_kind(config::UAVCAN_NODE_ID, config::POWER_BUTTON_SUBJECT, false, Priority::Nominal);
            let frame = Frame::new(id.into(), &[]).unwrap();
            can_send!(cx, frame);
        }
    } else {
        mr.button1_debounce = 0;
    }
    // mr.led1.set_state(button1_is_pressed.into()).ok();

    let estop_is_pressed = button0_is_pressed || button0_b_is_pressed;
    mr.led2.set_state(estop_is_pressed.into()).ok();

    const VESC_ID: u32 = 7;
    if !estop_is_pressed {
        const VESC_RESET_ESTOP_TIMEOUT: u32 = 46;
        let frame = Frame::new(FrameId::new_extended((VESC_RESET_ESTOP_TIMEOUT << 8) | VESC_ID).unwrap(), &[]).unwrap();
        can_send!(cx, frame);
    } else {
        const VESC_SET_CURRENT_BRAKE: u32 = 2;
        const BRAKE_CURRENT_MILLIAMPS: i32 = 3_000;
        let frame = Frame::new(FrameId::new_extended((VESC_SET_CURRENT_BRAKE << 8) | VESC_ID).unwrap(), &BRAKE_CURRENT_MILLIAMPS.to_be_bytes()).unwrap();
        can_send!(cx, frame);

        let id = CanId::new_message_kind(config::UAVCAN_NODE_ID, config::SAFETY_BUTTON_SUBJECT, false, Priority::Nominal);
        let frame = Frame::new(id.into(), &[]).unwrap();
        can_send!(cx, frame);
    }

    app::button_task::spawn_after(BUTTON_CHECK_TIME).ok();
}

fn init_tim14_ch1(tim: pac::TIM14, rcc: &hal::rcc::Rcc) {
    let dp = unsafe { pac::Peripherals::steal() };
    dp.RCC.apb1enr.modify(|_, w| w.tim14en().set_bit());

    tim.ccmr1_output_mut().modify(|_, w| unsafe { w.oc1m().bits(0b110) }); // PWM Mode 1
    tim.ccer.modify(|_, w| w.cc1e().set_bit()); // Output compare enable
    tim.ccr1.write(|w| unsafe { w.bits(0x0) });
    // tim.bdtr.modify(|_, w| w.moe().set_bit()); // Enable
    tim.ccer.write(|w| w.cc1e().set_bit());
    tim.egr.write(|w| w.ug().set_bit());
    tim.cr1.modify(|_, w| w.arpe().set_bit().cen().set_bit());


    let sysclk_hz = rcc.clocks.sysclk().0;
    let pwm_freq_hz = 20_000_u32;
    let max_duty = (sysclk_hz / pwm_freq_hz) as u16;
    tim.arr.write(|w| unsafe { w.arr().bits(max_duty) });
}

pub fn idle(_cx: app::idle::Context) -> ! {
    loop {

        cortex_m::asm::delay(1_000_000);
    }
}

pub fn handle_message(source: NodeId, message: Message, payload: &[u8]) {

}

pub fn handle_service_request(source: NodeId, service: Service, payload: &[u8]) {

}

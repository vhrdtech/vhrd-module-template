use crate::prelude::*;

use stm32f0xx_hal::gpio::{Input, Floating, PullUp, Output, PushPull};
use stm32f0xx_hal::gpio::gpiob::{PB0, PB1, PB2, PB12};
use stm32f0xx_hal::gpio::gpioa::{PA8, PA5, PA7};
use embedded_hal::digital::v2::{InputPin, OutputPin};
use embedded_time::duration::Milliseconds;
use vhrdcan::{Frame, FrameId};

pub struct Resources {
    button0: PB0<Input<PullUp>>,
    button0_b: PB1<Input<PullUp>>,
    button1: PB2<Input<PullUp>>,
    button2: PB12<Input<PullUp>>,
    _led0: PA5<Output<PushPull>>,
    led1: PA7<Output<PushPull>>,
    led2: PA8<Output<PushPull>>,
}

pub fn init(
    button0: PB0<Input<Floating>>,
    button0_b: PB1<Input<Floating>>,
    button1: PB2<Input<Floating>>,
    button2: PB12<Input<Floating>>,
    led0: PA5<Input<Floating>>,
    led1: PA7<Input<Floating>>,
    led2: PA8<Input<Floating>>
) -> Resources {
    let (
        button0,
        button0_b,
        button1,
        button2,
        led0,
        led1,
        led2
    ) = cortex_m::interrupt::free(|cs| (
        button0.into_pull_up_input(cs),
        button0_b.into_pull_up_input(cs),
        button1.into_pull_up_input(cs),
        button2.into_pull_up_input(cs),
        led0.into_push_pull_output(cs),
        led1.into_push_pull_output(cs),
        led2.into_push_pull_output(cs),
    ));
    Resources {
        button0,
        button0_b,
        button1,
        button2,
        _led0: led0,
        led1,
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

    mr.led1.set_state(button1_is_pressed.into()).ok();

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

        let id = CanId::new_message_kind(config::UAVCAN_NODE_ID, SubjectId::new(20).unwrap(), false, Priority::Nominal);
        let frame = Frame::new(id.into(), &[]).unwrap();
        can_send!(cx, frame);
    }

    app::button_task::spawn_after(Milliseconds(100_u32)).ok();
}

pub fn idle(_cx: app::idle::Context) -> ! {
    loop {

        cortex_m::asm::delay(1_000_000);
    }
}

pub fn can_rx_router(_cx: app::can_rx_router::Context) {

}
use stm32f0xx_hal::gpio::{Floating, Input, PushPull, Output};
use stm32f0xx_hal::gpio::gpiob::{PB0, PB2};
use embedded_hal::digital::v2::{OutputPin, StatefulOutputPin};
use crate::prelude::*;
use crate::app;

pub type PiEn = PB0<Output<PushPull>>;
const PI_SHUTDOWN_TIME: Seconds = Seconds(15);

pub struct Resources {
    pi_en: PB0<Output<PushPull>>,
}

pub fn init(pi_en: PB0<Input<Floating>>, pi_can_stby: PB2<Input<Floating>>) -> PiEn {
    let (pi_en, mut pi_can_stby, ) = cortex_m::interrupt::free(|cs| {
        (
            pi_en.into_push_pull_output(cs),
            pi_can_stby.into_push_pull_output(cs),
        )
    });
    pi_can_stby.set_low().ok();
    // pi_en.set_high().ok();

    pi_en
}

pub enum Event {
    PowerOff,
    //PowerOn,
    Toggle,
}

pub fn pi_task(cx: app::pi_task::Context, e: Event) {
    let pi_en: &mut PiEn = cx.local.pi_en;
    match e {
        Event::PowerOff => { pi_en.set_low().ok(); }
        //Event::PowerOn => { pi_en.set_high().ok(); }
        Event::Toggle => {
            if pi_en.is_set_low().unwrap() {
                log_info!("Enabling PI");
                pi_en.set_high();
            } else {
                log_info!("Scheduling PI off");
                app::pi_task::spawn_after(PI_SHUTDOWN_TIME, Event::PowerOff).ok();
            }
        }
    }
}

pub fn idle(_cx: app::idle::Context) -> ! {
    loop {
        cortex_m::asm::delay(1_000_000);
    }
}

use core::convert::AsMut;
use rtic::rtic_monotonic::Seconds;
// use crate::ramp_generator::Event;

fn clone_into_array<A, T>(slice: &[T]) -> A
    where A: Sized + Default + AsMut<[T]>,
          T: Clone
{
    let mut a = Default::default();
    <A as AsMut<[T]>>::as_mut(&mut a).clone_from_slice(slice);
    a
}

pub fn handle_message(source: NodeId, message: Message, payload: &[u8]) {
    // if source == config::PI_NODE_ID && message.subject_id == config::RMP_RAMP_TARGET_SUBJECT_ID {
    //     if payload.len() < 4 {
    //         return;
    //     }
    //     let rpm = i32::from_le_bytes(clone_into_array(&payload[0..=3]));
    //     count_result!(app::ramp_generator::spawn(Event::SetRpmTarget(rpm)));
    // } else
    if source == config::BUTTON_UAVCAN_NODE_ID && message.subject_id == config::POWER_BUTTON_SUBJECT {
        log_info!("Power button pressed");
        app::pi_task::spawn(Event::Toggle).ok();
    } else if source == config::PI_NODE_ID && message.subject_id == config::POWER_BUTTON_SUBJECT {
        log_info!("UI Power button pressed");
        app::pi_task::spawn_after(PI_SHUTDOWN_TIME, Event::PowerOff).ok();
    }
}

pub fn handle_service_request(_source: NodeId, service: Service, payload: &[u8]) {

}
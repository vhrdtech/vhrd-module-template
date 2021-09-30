use stm32f0xx_hal::gpio::{Floating, Input};
use stm32f0xx_hal::gpio::gpiob::PB0;
use embedded_hal::digital::v2::OutputPin;
use crate::prelude::*;
use crate::app;

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

use core::convert::AsMut;
use crate::ramp_generator::Event;

fn clone_into_array<A, T>(slice: &[T]) -> A
    where A: Sized + Default + AsMut<[T]>,
          T: Clone
{
    let mut a = Default::default();
    <A as AsMut<[T]>>::as_mut(&mut a).clone_from_slice(slice);
    a
}

pub fn handle_message(source: NodeId, message: Message, payload: &[u8]) {
    if source == config::PI_NODE_ID && message.subject_id == config::RMP_RAMP_TARGET_SUBJECT_ID {
        if payload.len() < 4 {
            return;
        }
        let rpm = i32::from_le_bytes(clone_into_array(&payload[0..=3]));
        count_result!(app::ramp_generator::spawn(Event::SetRpmTarget(rpm)));
    }
}

pub fn handle_service_request(_source: NodeId, service: Service, payload: &[u8]) {

}
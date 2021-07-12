use crate::pac;
use crate::hal;
use crate::app;
use crate::config;
use stm32f0xx_hal::gpio::gpioa::PA6;
use stm32f0xx_hal::gpio::{Input, Floating, Alternate, AF5};

type BlinkerTim = pac::TIM16;

pub enum BlinkerEvent {
    SetState(BlinkerState),
    Internal
}

#[derive(Copy, Clone)]
pub enum BlinkerState {
    Off,
    Breath,
}

pub struct Blinker {
    tim: BlinkerTim,
    _pin: PA6<Alternate<AF5>>,
    max_duty: u16,
    current_duty: u16,
    duty_direction_up: bool,
    duty_step: u16,
    state: BlinkerState,
    global_brightess: u8,
}

impl Blinker {
    pub fn new(tim: BlinkerTim, pin: PA6<Input<Floating>>, rcc: &hal::rcc::Rcc) -> Self {
        let dp = unsafe { pac::Peripherals::steal() };
        dp.RCC.apb2enr.modify(|_, w| w.tim16en().set_bit());

        tim.ccmr1_output_mut().modify(|_, w| unsafe { w.oc1m().bits(0b110) }); // PWM Mode 1
        tim.ccer.modify(|_, w| w.cc1e().set_bit()); // Output compare enable
        tim.ccr1.write(|w| unsafe { w.bits(0x0) });
        tim.bdtr.modify(|_, w| w.moe().set_bit()); // Enable
        tim.ccer.write(|w| w.cc1e().set_bit());
        tim.egr.write(|w| w.ug().set_bit());
        tim.cr1.modify(|_, w| w.arpe().set_bit().cen().set_bit());


        let sysclk_hz = rcc.clocks.sysclk().0;
        let pwm_freq_hz = 20_000_u32;
        let max_duty = (sysclk_hz / pwm_freq_hz) as u16;
        tim.arr.write(|w| unsafe { w.arr().bits(max_duty) });

        let pin = cortex_m::interrupt::free(|cs| pin.into_alternate_af5(cs));
        let breath_period = Milliseconds::new(config::BLINKER_BREATH_PERIOD.0 * 1000);
        let duty_step = breath_period.0 / config::BLINKER_UPDATE_PERIOD.0; // update count per breath
        let mut duty_step = max_duty / (duty_step as u16 / 2); // step to count up
        if duty_step == 0 {
            duty_step = 1;
        }
        Blinker {
            tim,
            _pin: pin,
            max_duty,
            current_duty: 0,
            duty_direction_up: true,
            duty_step,
            state: BlinkerState::Off,
            global_brightess: 100
        }
    }

    // fn set_duty_percent(&mut self, duty: u8) {
    //     let duty = if duty > 100 {
    //         100
    //     } else {
    //         duty
    //     };
    //     let duty = duty as u32 * self.max_duty as u32 / 100;
    //     self.set_duty_raw(duty as u16);
    // }

    fn set_duty_raw(&mut self, duty: u16) {
        let duty = self.global_brightess as u32 * duty as u32 / 100;
        self.tim.ccr1.write(|w| unsafe { w.ccr1().bits(duty as u16) });
    }

    pub fn set_global_brigthness_percent(&mut self, brightness: u8) {
        let brightness = if brightness > 100 {
            100
        } else {
            brightness
        };
        self.global_brightess = brightness;
    }
}

use rtic::Mutex;
use rtic::time::duration::{Milliseconds};

pub fn blinker_task(mut cx: app::blinker_task::Context, e: BlinkerEvent) {
    cx.shared.blinker.lock(|b: &mut Blinker| {
        match e {
            BlinkerEvent::SetState(state) => {
                b.state = state;
                match state {
                    BlinkerState::Off => {
                        b.set_duty_raw(0);
                    },
                    BlinkerState::Breath => {
                        b.current_duty = 0;
                        app::blinker_task::spawn_after(config::BLINKER_UPDATE_PERIOD, BlinkerEvent::Internal).ok();
                    }
                }
            }
            BlinkerEvent::Internal => {
                match b.state {
                    BlinkerState::Breath => {
                        b.set_duty_raw(b.current_duty);
                        if b.current_duty >= b.max_duty {
                            b.duty_direction_up = false;
                        } else if b.current_duty == 0 {
                            b.duty_direction_up = true;
                        }
                        if b.duty_direction_up {
                            b.current_duty += b.duty_step;
                        } else {
                            b.current_duty -= b.duty_step;
                        }
                        app::blinker_task::spawn_after(config::BLINKER_UPDATE_PERIOD, BlinkerEvent::Internal).ok();
                    },
                    _ => {}
                }
            }
        }
    });
}
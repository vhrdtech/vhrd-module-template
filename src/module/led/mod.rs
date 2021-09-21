use crate::hal;
use crate::prelude::*;
use crate::pac::SPI2;
use drv8323::DRV8323;
use hal::gpio::{
    gpiob::{PB12, PB13, PB14, PB15, PB7, PB8, PB9},
    Floating, Input,
};
use stm32f0xx_hal::gpio::gpioa::{PA10, PA8, PA9};
use stm32f0xx_hal::gpio::{Alternate, Output, PushPull, AF0};
use stm32f0xx_hal::spi::{SixteenBit, Spi};
use stm32f0xx_hal::time::U32Ext;
use drv8323::registers::DrvRegister;
use embedded_time::duration::Milliseconds;

pub type Drv8323Instance = DRV8323<
    Spi<SPI2, PB13<Alternate<AF0>>, PB14<Alternate<AF0>>, PB15<Alternate<AF0>>, SixteenBit>,
    PB7<Output<PushPull>>,
    PB8<Output<PushPull>>,
    PB9<Output<PushPull>>,
    PB12<Input<Floating>>,
    DummyDelay,
>;

pub fn init(
    drv_en: PB8<Input<Floating>>,
    drv_cal: PB9<Input<Floating>>,
    drv_sck: PB13<Input<Floating>>,
    drv_miso: PB14<Input<Floating>>,
    drv_mosi: PB15<Input<Floating>>,
    drv_cs: PB7<Input<Floating>>,
    drv_nfault: PB12<Input<Floating>>,

    ha: PA8<Input<Floating>>,
    hb: PA9<Input<Floating>>,
    hc: PA10<Input<Floating>>,

    spi2: hal::pac::SPI2,
    rcc: &mut hal::rcc::Rcc,
) -> (Option<Drv8323Instance>) {
    let (drv_sck, drv_miso, drv_mosi, drv_cs, drv_en, drv_cal, drv_nfault, ha, hb, hc) =
        cortex_m::interrupt::free(|cs| {
            (
                drv_sck.into_alternate_af0(cs),
                drv_miso.into_alternate_af0(cs),
                drv_mosi.into_alternate_af0(cs),
                drv_cs.into_push_pull_output(cs),
                drv_en.into_push_pull_output(cs),
                drv_cal.into_push_pull_output(cs),
                drv_nfault.into_floating_input(cs),
                ha.into_alternate_af2(cs),
                hb.into_alternate_af2(cs),
                hc.into_alternate_af2(cs),
            )
        });
    let drv_spi = hal::spi::Spi::spi2(
        spi2,
        (drv_sck, drv_miso, drv_mosi),
        embedded_hal::spi::MODE_1,
        10.khz(),
        rcc,
    )
    .into_16bit_width();
    let drv8323 = match DRV8323::new(drv_spi, drv_cs, drv_en, drv_cal, drv_nfault, DummyDelay {}) {
        Ok(mut drv8323) => {
            log_info!("DRV8323 create ok");
            let r = configure_drv8323(&mut drv8323);
            log_info!("DRV8323 configure: {:?}", r);
            Some(drv8323)
        }
        Err(e) => {
            log_error!("DRV8323 init fail: {:?}", e);
            None
        }
    };
    log_info!("Init TIM1");

    init_tim1(rcc.clocks.sysclk(), 20.khz().into());
    tim1_set_duty(50);

    (drv8323)
}

pub fn animation_task(mut cx: app::animation_task::Context) {
    cx.shared.drv8323.lock(|drv8323| {
        match drv8323 {
            Some(drv8323) => {
                let drv8323: &mut Drv8323Instance = drv8323;
                match drv8323.read_register(DrvRegister::FaultStatus1) {
                    Ok(fault_status_1) => {
                        log_debug!("fault_status_1: {:011b}", fault_status_1);
                    },
                    Err(e) => {
                        log_error!("drv8323 err: {:?}", e);
                    }
                }
                match drv8323.read_register(DrvRegister::FaultStatus2) {
                    Ok(fault_status_2) => {
                        log_debug!("fault_status_2: {:011b}", fault_status_2);
                    },
                    Err(e) => {
                        log_error!("drv8323 err: {:?}", e);
                    }
                }
            },
            None => {}
        }
    });

    app::animation_task::spawn_after(Milliseconds::new(100_u32)).ok();
}

fn configure_drv8323(drv8323: &mut Drv8323Instance) -> drv8323::DrvResult {
    use drv8323::registers::PwmMode;

    drv8323.set_pwm_mode(PwmMode::ThreePin)?;
    Ok(())
}

pub fn idle(_cx: app::idle::Context) -> ! {
    loop {
        cortex_m::asm::delay(1_000_000);
    }
}

pub struct DummyDelay {}
impl embedded_hal::blocking::delay::DelayUs<u32> for DummyDelay {
    fn delay_us(&mut self, us: u32) {
        cortex_m::asm::delay(us * 8);
    }
}

pub fn can_rx_router(_cx: app::can_rx_router::Context) {}

fn init_tim1(core_freq: stm32f0xx_hal::time::Hertz, pwm_freq: stm32f0xx_hal::time::Hertz) {
    let dp = unsafe { crate::hal::pac::Peripherals::steal() };
    dp.RCC.apb2enr.modify(|_, w| w.tim1en().enabled());
    dp.RCC.apb2rstr.modify(|_, w| w.tim1rst().set_bit());
    dp.RCC.apb2rstr.modify(|_, w| w.tim1rst().clear_bit());

    dp.TIM1
        .cr1
        .write(|w| w.cms().center_aligned1().ckd().div1());
    let arr_bits = (core_freq.0 / pwm_freq.0) as u16;
    dp.TIM1.arr.write(|w| w.arr().bits(arr_bits));
    dp.TIM1.psc.write(|w| w.psc().bits(0));
    dp.TIM1.rcr.write(|w| unsafe { w.rep().bits(0) });
    dp.TIM1.egr.write(|w| w.ug().update());

    // Disable output compare 1,2,3
    dp.TIM1.ccer.modify(|_, w| {
        w.cc1e()
            .clear_bit()
            .cc1ne()
            .clear_bit()
            .cc2e()
            .clear_bit()
            .cc2ne()
            .clear_bit()
            .cc3e()
            .clear_bit()
            .cc3ne()
            .clear_bit()
    });
    // Output idle and idle_n state
    dp.TIM1.cr2.modify(
        |_, w| {
            w.ois1()
                .set_bit() //.ois1n().set_bit()
                .ois2()
                .set_bit() //.ois2n().set_bit()
                .ois3()
                .set_bit()
        }, //.ois3n().set_bit()
    );
    // Select output mode
    dp.TIM1
        .ccmr1_output_mut()
        .modify(|_, w| w.oc1m().pwm_mode1().oc2m().pwm_mode1());
    dp.TIM1
        .ccmr2_output_mut()
        .modify(|_, w| w.oc3m().pwm_mode1());
    dp.TIM1.ccr1.write(|w| w.ccr().bits(arr_bits / 2));
    dp.TIM1.ccr2.write(|w| w.ccr().bits(arr_bits / 2));
    dp.TIM1.ccr3.write(|w| w.ccr().bits(arr_bits / 2));
    dp.TIM1.ccer.modify(
        |_, w| {
            w
                // polarity
                .cc1p()
                .set_bit()
                .cc2p()
                .set_bit()
                .cc3p()
                .set_bit()
                // enable outputs
                .cc1e()
                .set_bit() //.cc1ne().set_bit()
                .cc2e()
                .set_bit() //.cc2ne().set_bit()
                .cc3e()
                .set_bit()
        }, //.cc3ne().set_bit()
    );
    // Enable preload
    dp.TIM1
        .ccmr1_output_mut()
        .modify(|_, w| w.oc1pe().enabled().oc2pe().enabled());
    dp.TIM1
        .ccmr2_output_mut()
        .modify(|_, w| w.oc3pe().set_bit());
    // Dead time, break disable
    dp.TIM1.bdtr.write(|w| unsafe {
        w.ossr()
            .idle_level()
            .ossi()
            .idle_level()
            .lock()
            .bits(0)
            .dtg()
            .bits(127) // TODO: calculate proper dead time
            .aoe()
            .clear_bit()
            .bke()
            .clear_bit()
            .bkp()
            .set_bit()
    });
    // Preload enable on CCR and ARR
    dp.TIM1.cr2.modify(|_, w| w.ccpc().set_bit());
    dp.TIM1.cr1.modify(|_, w| w.arpe().set_bit());
    // Enable
    // dp.TIM1.cnt.write(0)
    dp.TIM1.cr1.modify(|_, w| w.cen().enabled());
    dp.TIM1.bdtr.modify(|_, w| w.moe().enabled());
}

fn tim1_set_duty(duty: u16) {
    let dp = unsafe { crate::hal::pac::Peripherals::steal() };
    let max_duty = dp.TIM1.arr.read().bits();
    dp.TIM1.cr1.modify(|_, w| w.udis().disabled());
    dp.TIM1.ccr1.write(|w| unsafe { w.bits(duty as u32) });
    dp.TIM1.ccr2.write(|w| unsafe { w.bits(0) });
    // dp.TIM1.ccr3.write(|w| unsafe { w.bits(self.duty_c) });
    dp.TIM1.cr1.modify(|_, w| w.udis().enabled());
}

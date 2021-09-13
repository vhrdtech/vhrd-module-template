use crate::app;
use crate::hal;
use hal::gpio::{Input, Floating, gpiob::{PB7, PB8, PB9, PB12, PB13, PB14, PB15}};
use drv8323::DRV8323;
use stm32f0xx_hal::time::U32Ext;

// pub type Drv8323Instance = DRV8323<>

pub fn init(
    drv_en: PB8<Input<Floating>>,
    drv_cal: PB9<Input<Floating>>,
    drv_sck: PB13<Input<Floating>>,
    drv_miso: PB14<Input<Floating>>,
    drv_mosi: PB15<Input<Floating>>,
    drv_cs: PB7<Input<Floating>>,
    drv_nfault: PB12<Input<Floating>>,

    spi2: hal::pac::SPI2,
    rcc: &mut hal::rcc::Rcc,
)  {
    let (drv_sck, drv_miso, drv_mosi, drv_cs, drv_en, drv_cal, drv_nfault) = cortex_m::interrupt::free(|cs| {
        (
            drv_sck.into_alternate_af0(cs),
            drv_miso.into_alternate_af0(cs),
            drv_mosi.into_alternate_af0(cs),
            drv_cs.into_push_pull_output(cs),
            drv_en.into_push_pull_output(cs),
            drv_cal.into_push_pull_output(cs),
            drv_nfault.into_floating_input(cs),
        )
    });
    let drv_spi = hal::spi::Spi::spi2(
        spi2,
        (drv_sck, drv_miso, drv_mosi),
        embedded_hal::spi::MODE_1,
        10.khz(),
        rcc
    ).into_16bit_width();
    let drv8323 = match DRV8323::new(drv_spi, drv_cs, drv_en, drv_cal, drv_nfault, DummyDelay {}) {
        Ok(drv8323) => {
            log_info!("DRV8323 init ok");
            Some(drv8323)
        },
        Err(e) => {
            log_error!("DRV8323 init fail: {:?}", e);
            None
        }
    };

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

pub fn can_rx_router(_cx: app::can_rx_router::Context) {

}
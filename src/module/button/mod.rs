use crate::app;

pub struct Resources {

}

pub fn init() -> Resources {

    Resources {

    }
}

pub fn idle(_cx: app::idle::Context) -> ! {
    loop {

        cortex_m::asm::delay(1_000_000);
    }
}
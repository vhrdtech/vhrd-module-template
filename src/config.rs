use embedded_time::duration::{Seconds, Milliseconds};

/// How often to update LED brightness during transitions (Breath, etc)
pub const BLINKER_UPDATE_PERIOD: Milliseconds = Milliseconds(20);
pub const BLINKER_BREATH_PERIOD: Seconds = Seconds(5);

/// MCP2515
#[cfg(feature = "can-mcp25625")]
pub mod mcp25625_config {
    use crate::{hal, pac};
    use hal::gpio::{Alternate, AF0, PushPull, Input, PullUp, Output, gpiob::{PB3, PB4, PB5}, gpioc::{PC14, PC15}};
    use pac::{Interrupt, SPI1};
    use hal::time::MegaHertz;

    pub type Mcp25625Sck = PB3<Alternate<AF0>>;
    pub const MCP25625SPI_FREQ: MegaHertz = MegaHertz(1);
    pub type Mcp25625Miso = PB4<Alternate<AF0>>;
    pub type Mcp25625Mosi = PB5<Alternate<AF0>>;
    pub type Mcp25625Cs = PC14<Output<PushPull>>;
    pub type Mcp25625Spi = SPI1;
    pub type Mcp25625Instance = mcp25625::MCP25625<hal::spi::Spi<Mcp25625Spi, Mcp25625Sck, Mcp25625Miso, Mcp25625Mosi>, Mcp25625Cs>;
    pub type Mcp25625Irq = PC15<Input<PullUp>>;
    pub const MCP25625_IRQ_HANDLER: Interrupt = Interrupt::EXTI4_15;
}
#[cfg(feature = "can-mcp25625")]
pub use mcp25625_config::*;
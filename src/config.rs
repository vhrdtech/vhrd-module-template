use embedded_time::duration::{Seconds, Milliseconds};

/// How often to update LED brightness during transitions (Breath, etc)
pub const BLINKER_UPDATE_PERIOD: Milliseconds = Milliseconds(20);
pub const BLINKER_BREATH_PERIOD: Seconds = Seconds(5);

pub const HEALTH_CHECK_PERIOD: Milliseconds = Milliseconds(1000);

pub const REBOOT_SERVICE_ID: ServiceId = ServiceId::new(4).unwrap();


#[cfg(feature = "module-pi")]
pub const UAVCAN_NODE_ID: NodeId = NodeId::new(5).unwrap();
#[cfg(feature = "module-led")]
pub const UAVCAN_NODE_ID: NodeId = NodeId::new(4).unwrap();
#[cfg(feature = "module-button")]
pub const UAVCAN_NODE_ID: NodeId = NodeId::new(3).unwrap();
#[cfg(feature = "module-afe")]
pub const UAVCAN_NODE_ID: NodeId = NodeId::new(2).unwrap();

// CAN Bus
use heapless::binary_heap::{BinaryHeap, Min};
use vhrdcan::frame::Frame;
pub type CanTxQueue = BinaryHeap<Frame<8>, Min, 32>;
pub type CanRxQueue = BinaryHeap<Frame<8>, Min, 32>;

/// CAN Bus: MCP2515
#[cfg(feature = "can-mcp25625")]
pub mod mcp25625_config {
    use crate::{hal, pac};
    use hal::gpio::{Alternate, AF0, PushPull, Input, PullUp, Output, gpiob::{PB3, PB4, PB5}, gpioc::{PC14, PC15}};
    use pac::{Interrupt, SPI1};
    use hal::time::MegaHertz;
    use hal::spi::{Spi};

    pub type Mcp25625Sck = PB3<Alternate<AF0>>;
    pub const MCP25625SPI_FREQ: MegaHertz = MegaHertz(1);
    pub type Mcp25625Miso = PB4<Alternate<AF0>>;
    pub type Mcp25625Mosi = PB5<Alternate<AF0>>;
    pub type Mcp25625Cs = PC14<Output<PushPull>>;
    pub type Mcp25625Spi = SPI1;
    pub type Mcp25625Instance = mcp25625::MCP25625<Spi<Mcp25625Spi, Mcp25625Sck, Mcp25625Miso, Mcp25625Mosi, hal::spi::EightBit>, Mcp25625Cs>;
    pub type Mcp25625Irq = PC15<Input<PullUp>>;
    pub const MCP25625_IRQ_HANDLER: Interrupt = Interrupt::EXTI4_15;
}
#[cfg(feature = "can-mcp25625")]
pub use mcp25625_config::*;
#[cfg(not(feature = "can-mcp25625"))]
pub type Mcp25625Instance = ();
#[cfg(not(feature = "can-mcp25625"))]
pub type Mcp25625Irq = ();

/// CAN Bus: STM
#[cfg(feature = "can-stm")]
pub mod can_stm_config {
    use crate::hal;
    use hal::gpio::{Alternate, AF4, gpioa::{PA11, PA12}};

    pub type CanTx = PA12<Alternate<AF4>>;
    pub type CanRx = PA11<Alternate<AF4>>;
    pub type CanStmInstance = hal::can::bxcan::Can<hal::can::CanInstance<CanTx, CanRx>>;
}
#[cfg(feature = "can-stm")]
pub use can_stm_config::*;
use uavcan_llr::types::{NodeId, ServiceId};

#[cfg(not(feature = "can-stm"))]
pub type CanStmInstance = ();
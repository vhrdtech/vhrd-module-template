use crate::{config, hal};

#[cfg(feature = "can-mcp25625")]
use crate::config::{Mcp25625Spi, Mcp25625Sck, Mcp25625Miso, Mcp25625Mosi, Mcp25625Cs, Mcp25625Instance};

#[cfg(feature = "can-mcp25625")]
pub fn mcp25625_init(
    spi: Mcp25625Spi,
    sck: Mcp25625Sck,
    miso: Mcp25625Miso,
    mosi: Mcp25625Mosi,
    cs: Mcp25625Cs,
    rcc: &mut hal::rcc::Rcc,
) -> Result<Mcp25625Instance, mcp25625::McpErrorKind> {
    let spi = hal::spi::Spi::spi1(
        spi,
        (sck, miso, mosi),
        embedded_hal::spi::MODE_0,
        config::MCP25625SPI_FREQ,
        rcc
    );
    let mut mcp25625 = mcp25625::MCP25625::new(
        spi,
        cs,
        config::MCP25625SPI_FREQ.0 * 1_000_000,
        rcc.clocks.sysclk().0
    );
    mcp25625_configure(&mut mcp25625)?;
    Ok(mcp25625)
}

#[cfg(feature = "can-mcp25625")]
use mcp25625::{McpErrorKind, FiltersConfigBuffer0, FiltersConfigBuffer1, FiltersMask, FiltersConfig, MCP25625Config, McpOperationMode};
#[cfg(feature = "can-mcp25625")]
fn mcp25625_configure(mcp25625: &mut config::Mcp25625Instance) -> Result<(), McpErrorKind> {
    // let filters_buffer0 = FiltersConfigBuffer0 {
    //     mask: FiltersMask::AllExtendedIdBits,
    //     filter0: config::,
    //     filter1: None
    // };
    // let filters_buffer1 = FiltersConfigBuffer1 {
    //     mask: FiltersMask::OnlyStandardIdBits,
    //     filter2: config::,
    //     filter3: None,
    //     filter4: None,
    //     filter5: None,
    // };
    // let filters_config = FiltersConfig::Filter(filters_buffer0, Some(filters_buffer1));
    let filters_config = FiltersConfig::ReceiveAll;
    let mcp_config = MCP25625Config {
        brp: 0, // Fosc=16MHz
        prop_seg: 3,
        ph_seg1: 2,
        ph_seg2: 2,
        sync_jump_width: 2,
        rollover_to_buffer1: true,
        filters_config,
        // filters_config: FiltersConfig::ReceiveAll,
        operation_mode: McpOperationMode::Normal
    };
    mcp25625.apply_config(mcp_config)?;
    mcp25625.enable_interrupts(0b0001_1111);
    Ok(())
}
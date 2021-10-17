use crate::{config, hal, app};
use rtic::Mutex;

macro_rules! can_send {
    ($cx:expr, $frame:expr) => {
        #[cfg(feature = "can-mcp25625")]
        $cx.shared.can_mcp_tx.lock(|tx| tx.push($frame)).ok();
        #[cfg(feature = "can-mcp25625")]
        rtic::pend(crate::config::MCP25625_IRQ_HANDLER);

        #[cfg(feature = "can-stm")]
        $cx.shared.can_stm_tx.lock(|tx| tx.push($frame)).ok();
        #[cfg(feature = "can-stm")]
        rtic::pend(crate::pac::Interrupt::CEC_CAN);
    };
}

pub fn can_rx_router(mut cx: app::can_rx_router::Context) {
    loop {
        #[cfg(feature = "can-mcp25625")]
        let frame: Option<Frame<8>> = cx.shared.can_mcp_rx.lock(|rx| rx.pop());
        #[cfg(feature = "can-stm")]
        let frame: Option<Frame<8>> = cx.shared.can_stm_rx.lock(|rx| rx.pop());

        match frame {
            Some(frame) => {
                match frame.id {
                    FrameId::Standard(_) => continue,
                    FrameId::Extended(eid) => {
                        if eid.inner() == 0x907 {
                            #[cfg(feature = "vesc-ctrl")]
                            if let Some(feedback) = crate::ramp_vesc::VescFeedback::new(frame) {
                                cx.shared.vesc_feedback.lock(|f| *f = Some(feedback));
                            }
                            continue;
                        }
                    }
                }
                match CanId::try_from(frame.id) {
                    Ok(uavcan_id) => {
                        match uavcan_id.transfer_kind {
                            TransferKind::Message(message) => {
                                #[cfg(feature = "vesc-ctrl")]
                                if let Some(input) = crate::ramp_vesc::ControlInput::new(uavcan_id.source_node_id, message, frame.data()) {
                                    cx.shared.vesc_control_input.lock(|i| *i = Some(input));
                                    continue;
                                }
                                if uavcan_id.source_node_id == config::BUTTON_UAVCAN_NODE_ID && message.subject_id == config::SAFETY_BUTTON_SUBJECT {
                                    log_debug!("Estop pressed");
                                    cx.shared.stand_state.lock(|s| s.is_estop_pressed = true);
                                    let _ = app::unpress_estop::spawn_after(Milliseconds::new(500u32));
                                } else if uavcan_id.source_node_id == config::BUTTON_UAVCAN_NODE_ID && message.subject_id == config::POWER_BUTTON_SUBJECT {
                                    log_debug!("Pwr pressed");
                                    cx.shared.stand_state.lock(|s| s.is_power_enabled = !s.is_power_enabled);
                                } else if uavcan_id.source_node_id == config::PI_NODE_ID && message.subject_id == SubjectId::new(77).unwrap() {
                                    log_debug!("Pwr pressed virt");
                                    cx.shared.stand_state.lock(|s| s.is_power_enabled = !s.is_power_enabled);
                                }
                                if false {

                                } else {
                                    crate::module::handle_message(uavcan_id.source_node_id, message, frame.data());
                                }
                            }
                            TransferKind::Service(service) => {
                                if service.destination_node_id != config::UAVCAN_NODE_ID {
                                    continue;
                                }
                                if service.service_id == config::REBOOT_SERVICE_ID {
                                    log_debug!("Reset requested from: {}", uavcan_id.source_node_id);
                                    cortex_m::asm::delay(10_000); // Minimum seems to be 3_000 @ 8MHz and JLink 255
                                    cortex_m::peripheral::SCB::sys_reset();
                                } else {
                                    crate::module::handle_service_request(uavcan_id.source_node_id, service, frame.data());
                                }
                            }
                        }
                    },
                    Err(_) => {}
                }
            },
            None => {
                return;
            }
        }
    }
}

#[cfg(feature = "can-mcp25625")]
use crate::config::{Mcp25625Spi, Mcp25625Sck, Mcp25625Miso, Mcp25625Mosi, Mcp25625Cs, Mcp25625Instance};

#[cfg(feature = "can-mcp25625")]
pub fn can_mcp25625_init(
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
use mcp25625::{McpErrorKind, FiltersConfig, MCP25625Config, McpOperationMode};
use vhrdcan::{FrameId, Frame};

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
    mcp25625.clkout_mode(mcp25625::ClkOutMode::SystemClockDiv8); // default is /8 as well = 2MHz
    Ok(())
}

macro_rules! log_debug_if_cps {
    ($($arg:tt)*) => {
        cfg_if::cfg_if! {
            if #[cfg(feature = "can-printstat")] {
                log_debug!(=>1, $($arg)*);
            }
        }
    };
}

#[cfg(feature = "can-mcp25625")]
pub fn can_mcp25625_irq(cx: &mut crate::app::exti_4_15::Context) {
    use mcp25625::{McpReceiveBuffer, McpPriority, };
    match cx.local.can_mcp25625 {
        Some(mcp25625) => {
            let mcp25625: &mut config::Mcp25625Instance = mcp25625;
            let intf = mcp25625.interrupt_flags();
            log_debug_if_cps!("INTF: {:?}", intf);
            let errf = mcp25625.error_flags();
            log_debug_if_cps!("{:?}", errf);

            let mut buffers = [None, None];
            buffers[0] = if intf.rx0if_is_set() {
                Some(McpReceiveBuffer::Buffer0)
            } else {
                None
            };
            buffers[1] = if intf.rx1if_is_set() {
                Some(McpReceiveBuffer::Buffer1)
            } else {
                None
            };
            let mut new_frames = false;
            for b in buffers.iter() {
                if b.is_none() {
                    continue;
                }
                let frame = mcp25625.receive(b.unwrap());
                match cx.shared.can_mcp_rx.lock(|rx| rx.push(frame)) {
                    Ok(_) => {
                        log_debug_if_cps!("RX: {:?}", frame);
                        new_frames = true;
                    },
                    Err(_) => {
                        log_debug_if_cps!("RX overflow");
                    }
                }
            }
            if new_frames {
                crate::app::can_rx_router::spawn().ok();
            }

            let _tec = mcp25625.tec();
            let _rec = mcp25625.rec();
            log_debug_if_cps!("TEC: {}, REC: {}", _tec, _rec);

            for _ in 0..3 {
                let maybe_frame = cx.shared.can_mcp_tx.lock(|tx| tx.peek().cloned());
                match maybe_frame {
                    Some(frame) => {
                        // Treat extended id frames as uavcan, use only one buffer for them to avoid priority inversion
                        // If standard id frame is placed after extended one it will have to wait with this implementation!
                        let buffer_choice = match frame.id {
                            FrameId::Standard(_) => { mcp25625::TxBufferChoice::Any }
                            FrameId::Extended(_) => { mcp25625::TxBufferChoice::OnlyOne(0) }
                        };
                        match mcp25625.send(frame.as_frame_ref(), buffer_choice, McpPriority::Highest) {
                            Ok(_) => {
                                let _ = cx.shared.can_mcp_tx.lock(|tx| tx.pop());
                                log_debug_if_cps!("TX: {:?}", frame);
                            }
                            Err(_e) => {
                                log_debug_if_cps!("TX error: {:?}", _e);
                                break;
                            }
                        }
                    },
                    None => {
                        break;
                    }
                }
            }

            if errf.is_err() {
                mcp25625.reset_error_flags();
            }
            mcp25625.reset_interrupt_flags(0xFF);
        },
        None => {}
    }
}

#[cfg(feature = "can-stm")]
pub fn can_stm_init(
    can_peripheral: hal::stm32::CAN,
    can_tx: config::CanTx,
    can_rx: config::CanRx,
    rcc: &mut hal::rcc::Rcc
) -> config::CanStmInstance {
    use stm32f0xx_hal::can::bxcan::filter::{BankConfig, Mask32};

    let can = hal::can::CanInstance::new(can_peripheral, can_tx, can_rx, rcc);
    let mut can = hal::can::bxcan::Can::new(can);
    #[cfg(feature = "module-button")]
    let bit_timing = 0x00050000;
    #[cfg(feature = "module-afe")]
    let bit_timing = 0x00050000;
    #[cfg(feature = "module-pi")]
    let bit_timing = 0x00050000;
    #[cfg(feature = "module-led")]
    let bit_timing = 0x001c0002;
    can.modify_config()
        .set_loopback(false)
        .set_silent(false)
        .set_bit_timing(bit_timing);
    {
        let mut filters = can.modify_filters();
        filters.enable_bank(0, BankConfig::Mask32(Mask32::accept_all()));
    }
    can.enable().ok();

    use hal::can::bxcan::Interrupt;
    // can.enable_interrupt(Interrupt::Fifo0MessagePending);
    // can.enable_interrupt(Interrupt::Fifo1MessagePending);
    can.enable_interrupt(Interrupt::Fifo0Full);
    // can.enable_interrupt(Interrupt::Fifo1Full); // endless interrupt
    can.enable_interrupt(Interrupt::TransmitMailboxEmpty);
    can
}

#[cfg(feature = "can-stm")]
use hal::can::bxcan::Frame as BxFrame;
use uavcan_llr::types::{CanId, TransferKind, SubjectId};
use core::convert::TryFrom;
use embedded_time::duration::Milliseconds;
// use vhrd_module_nvconfig::NVConfig;

pub struct CanStmState {
    #[cfg(feature = "can-stm")]
    pushed_out: Option<BxFrame>,
}
impl CanStmState {
    pub const fn new() -> Self {
        CanStmState {
            #[cfg(feature = "can-stm")]
            pushed_out: None
        }
    }
}

#[cfg(feature = "can-stm")]
pub fn can_stm_task(mut cx: crate::app::can_stm_task::Context) {
    // log_debug!("can_irq");
    use hal::can::bxcan::Data as BxData;

    let can: &mut config::CanStmInstance = cx.local.can_stm;
    can.clear_wakeup_interrupt();
    // unsafe {
    //     let dp = hal::pac::Peripherals::steal();
    //     log_debug!("msr:{:032b}", dp.CAN.msr.read().bits());
    // }
    let mut new_frames = false;
    for _ in 0..=1 {
        match can.receive() {
            Ok(frame) => {
                log_debug_if_cps!("R");
                if frame.is_data_frame() {
                    let frame = Frame::<8>::new(bxcanid2vhrdcanid(frame.id()), frame.data().unwrap()).unwrap();
                    log_debug_if_cps!("RX: {:?}", frame);
                    match cx.shared.can_stm_rx.lock(|rx| rx.push(frame)) {
                        Ok(_) => {
                            new_frames = true;
                        }
                        Err(_) => {

                        }
                    }
                }
            }
            Err(_) => {
                // log_debug_if_cps!("RX err");
            }
        }
    }
    if new_frames {
        crate::app::can_rx_router::spawn().ok();
    }

    cx.local.state.pushed_out = match &cx.local.state.pushed_out {
        Some(frame) => {
            match can.transmit(&frame) {
                Ok(maybe_frame) => {
                    log_debug_if_cps!("TXPu -> push");
                    maybe_frame
                }
                Err(_) => {
                    log_debug_if_cps!("TXPu -> none");
                    None
                }
            }
        }
        None => {
            None
        }
    };
    if cx.local.state.pushed_out.is_some() {
        return;
    }

    loop {
        match cx.shared.can_stm_tx.lock(|tx: &mut config::CanTxQueue| tx.pop()) {
            Some(frame) => {
                match can.transmit(&BxFrame::new_data(vhrdcanid2bxcanid(frame.id), BxData::new(frame.data()).unwrap())) {
                    Ok(maybe_frame) => {
                        match maybe_frame {
                            Some(frame) => {
                                cx.local.state.pushed_out = Some(frame);
                                log_debug_if_cps!("TX -> push");
                                break;
                            }
                            None => {
                                log_debug_if_cps!("TX -> none");
                            }
                        }
                    }
                    Err(_e) => {
                        log_debug_if_cps!("TX error: {:?}", _e);
                    }
                }
            }
            None => {
                break;
            }
        }
    }
}

#[cfg(feature = "can-stm")]
fn vhrdcanid2bxcanid(id: FrameId) -> crate::hal::can::bxcan::Id {
    use hal::can::bxcan::{Id, StandardId, ExtendedId};
    match id {
        FrameId::Standard(sid) => { Id::Standard(StandardId::new(sid.inner()).unwrap()) }
        FrameId::Extended(eid) => { Id::Extended(ExtendedId::new(eid.inner()).unwrap()) }
    }
}

#[cfg(feature = "can-stm")]
fn bxcanid2vhrdcanid(id: crate::hal::can::bxcan::Id) -> FrameId {
    use hal::can::bxcan::Id;
    use vhrdcan::id::{StandardId, ExtendedId};
    match id {
        Id::Standard(sid) => { FrameId::Standard( unsafe { StandardId::new_unchecked(sid.as_raw()) } )}
        Id::Extended(eid) => { FrameId::Extended( unsafe { ExtendedId::new_unchecked(eid.as_raw()) } )}
    }
}
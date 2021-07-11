use core::panic::PanicInfo;

#[inline(never)]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {

    cortex_m::asm::delay(6_000_000);
    cortex_m::peripheral::SCB::sys_reset(); // -> !
}
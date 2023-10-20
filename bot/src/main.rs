#![no_std]
#![no_main]

use core::{panic::PanicInfo, ptr};

#[panic_handler]
fn panic(_panic: &PanicInfo<'_>) -> ! {
    loop {}
}

#[export_name = "main"]
fn main() -> ! {
    let addr = 0x400 as *mut u32;
    let mut foo = unsafe {ptr::read(addr)};

    foo = foo +3;
    unsafe {ptr::write(addr, foo)}
    foo = foo*5;
    unsafe {ptr::write(addr, foo)}

    panic!();
    //loop {}
}

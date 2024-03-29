#![no_std]
#![no_main]

use core::{panic::PanicInfo, ptr};

#[panic_handler]
fn panic(_panic: &PanicInfo<'_>) -> ! {
    loop {}
}

pub const DRAM_SIZE: u32 = 1024 * 64;

pub enum Direction {
    NONE,
    LEFT,
    UP,
    RIGHT,
    DOWN,
}

#[inline(never)]
fn cmd(dir: Direction) {
    let dir = dir as u32;
    unsafe { ptr::write(4 as *mut u32, dir) }
}

#[export_name = "main"]
fn main() -> ! {
    loop {
        for _ in 0..5 {
            cmd(Direction::LEFT)
        }
        for _ in 0..5 {
            cmd(Direction::DOWN)
        }
        for _ in 0..5 {
            cmd(Direction::RIGHT)
        }
        for _ in 0..5 {
            cmd(Direction::UP)
        }
    }
}

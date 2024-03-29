use cpu::Cpu;
use dram::{Dram, DRAM_SIZE};
use std::env;
use std::fs;
use color_eyre::Result;

mod cpu;
mod dram;
mod tui;
fn main() -> Result<()> {
    //tracing_subscriber::FmtSubscriber::builder()
    //    .with_max_level(LevelFilter::DEBUG)
    //    .init();

    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        panic!("Usage: emulator <filename>");
    }
    let code = fs::read(&args[1])?;

    let (mut dram, entry) = Dram::new(code);

    dram.store(DRAM_SIZE - 4, 32, 4).unwrap();

    let cpu = Cpu::new(dram, entry);

    tui::run(cpu)
}

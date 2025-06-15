use color_eyre::Result;

use emulator_core as emulator;

use emulator::CpuBuilder;
use std::env;
use std::fs;

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
    let cpu = CpuBuilder::default().build(&code);

    //run_plain(cpu);

    tui::run(cpu)?;
    Ok(())
}

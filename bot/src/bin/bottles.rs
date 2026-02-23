#![no_std]
#![no_main]

use core::fmt::Write;

use botracers_bot_sdk::log;

#[unsafe(export_name = "main")]
fn main() -> ! {
    for i in (3..100).rev() {
        writeln!(
            log(),
            "{i} bottles of beer on the wall,\n\
             {i} bottles of beer.\n\
             Take one down and pass it around,\n\
             now there's {} more bottles of beer on the wall!\n",
            i - 1
        )
        .unwrap();
    }
    writeln!(
        log(),
        "2 bottles of beer on the wall,\n\
         2 bottles of beer.\n\
         Take one down and pass it around,\n\
         now there's 1 more bottle of beer on the wall!\n",
    )
    .unwrap();
    writeln!(
        log(),
        "1 bottle of beer on the wall,\n\
         1 bottle of beer.\n\
         Take one down and pass it around,\n\
         there's no more bottles of beer on the wall!"
    )
    .unwrap();
    panic!("Done singing!");
}

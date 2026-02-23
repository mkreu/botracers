#![no_std]
#![no_main]

use core::fmt::Write;

use botracers_bot_sdk::{
    driving::{CarControls, CarState},
    log, SLOT2, SLOT3,
};

#[unsafe(export_name = "main")]
fn main() -> ! {
    writeln!(log(), "Starter car bot running...").ok();

    let car_state = CarState::bind(SLOT2);
    let mut car_controls = CarControls::bind(SLOT3);

    loop {
        let speed = car_state.speed();
        let forward = car_state.forward();

        let accel = if speed < 18.0 { 0.35 } else { 0.1 };
        let brake = if speed > 24.0 { 0.15 } else { 0.0 };
        let steering = (-forward.x * 0.6).clamp(-0.5, 0.5);

        car_controls.set_accelerator(accel);
        car_controls.set_brake(brake);
        car_controls.set_steering(steering);
    }
}

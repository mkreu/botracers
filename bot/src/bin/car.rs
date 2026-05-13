#![no_std]
#![no_main]

use core::fmt::Write;

use botracers_bot_sdk::{
    driving::{CarControls, CarState, CarVision},
    log, SLOT2, SLOT3, SLOT4,
};

#[unsafe(export_name = "main")]
fn main() -> ! {
    writeln!(log(), "Car OS starting up...").ok();

    let state = CarState::bind(SLOT2);
    let mut controls = CarControls::bind(SLOT3);
    let mut vision = CarVision::bind(SLOT4);

    loop {
        let speed = state.speed();
        let offset = vision.offset();
        let angle = vision.angle();

        // Steering feed-forward: blend immediate curvature with a short fixed
        // lookahead (10 m) — close enough to be reactive, far enough to be smooth.
        let curv_now = vision.curvature_at(0.0);
        let curv_near = vision.curvature_at(10.0);

        let centering = offset * 0.04;
        let heading_correction = angle * 0.5;
        let feedforward = -(curv_now * 0.4 + curv_near * 0.6) * speed * 0.5;
        let steer = (centering + heading_correction + feedforward).clamp(-1.0, 1.0);
        controls.set_steering(steer);

        // Braking: speed-adaptive lookahead so we look further ahead the faster
        // we go (roughly reaction-distance scaling).  Clamped to [5, 35] m so
        // we don't overcorrect at low speed or look too far at high speed.
        let brake_lookahead = (speed * 1.2).clamp(5.0, 35.0);
        let curv_brake = vision.curvature_at(brake_lookahead);
        let brake = if curv_brake.abs() > 0.02 && speed > 20.0 {
            ((curv_brake.abs() - 0.02) * 50.0).clamp(0.0, 1.0)
        } else {
            0.0
        };
        controls.set_brake(brake);
        controls.set_accelerator(if brake < 0.5 { 1.0 } else { 0.0 });
    }
}

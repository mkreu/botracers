#![no_std]
#![no_main]

use core::fmt::Write;

use bevy_math::ops::{cos, sin};
use botracers_bot_sdk::{
    SLOT2, SLOT3, SLOT4, SLOT6,
    driving::{CarControls, CarDebug, CarState, CarVision},
    log,
};

#[unsafe(export_name = "main")]
fn main() -> ! {
    writeln!(log(), "Car OS starting up...").ok();

    let state = CarState::bind(SLOT2);
    let mut controls = CarControls::bind(SLOT3);
    let mut vision = CarVision::bind(SLOT4);
    let mut debug = CarDebug::bind(SLOT6);

    loop {
        let speed = state.speed();
        let offset = vision.offset();
        let angle = vision.angle();

        // Steering feed-forward: blend immediate curvature with a short fixed
        // lookahead (10 m) — close enough to be reactive, far enough to be smooth.
        let curv_now = vision.curvature_at(0.0);
        let curv_near = vision.curvature_at(10.0);

        let mut curv = [0.0; 20];
        let center_x = 0.0; //cos(angle) * offset;
        let center_y = 0.0; //sin(angle) * offset;
        let mut x = center_x;
        let mut y = center_y;
        debug.move_to_xy(center_x, center_y);
        for i in 0..20 {
            curv[i] = vision.curvature_at(i as f32);
            debug.line_to_xy(-y, x);
            x = x + cos(curv[i] * i as f32);
            y = y + sin(curv[i] * i as f32);
            //debug.line_to_xy(center_x + cos(angle + curv[i] * i as f32) * offset, center_y + sin(angle + curv[i] * i as f32) * offset);

        }
        debug.submit();

        let centering = offset * 0.03;
        let speed_factor = (speed / 40.0).clamp(0.0, 1.0);
        let heading_correction = angle * (1.0 - speed_factor);
        let feedforward = -(curv_now * (1.0 - speed_factor) + curv_near * speed_factor) * speed * 0.5;
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

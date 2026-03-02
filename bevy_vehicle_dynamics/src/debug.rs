use bevy::{color::palettes::css::*, prelude::*};

use crate::*;

pub struct VehicleDynamicsDebugPlugin;

impl Plugin for VehicleDynamicsDebugPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(Update, draw_wheel_gizmos);
    }
}

fn draw_wheel_gizmos(
    query: Query<
        (
            &GlobalTransform,
            &Wheel,
            &WheelForces,
            &WheelTelemetry,
        ),
        With<Wheel>,
    >,
    mut gizmos: Gizmos,
) {
    for (transform, wheel, forces, telemetry) in query.iter() {
        let wheel_forward = telemetry.wheel_forward;
        let wheel_left = wheel_forward.perp();
        let pos = transform.translation().xy();

        // Draw the wheel as a line in the forward direction
        gizmos.line_2d(
            pos - wheel_forward * wheel.radius,
            pos + wheel_forward * wheel.radius,
            WHITE,
        );

        // Draw wheel velocity
        gizmos.line_2d(
            pos,
            pos + telemetry.wheel_velocity * 0.1,
            GREEN,
        );

        // Draw force vector
        gizmos.line_2d(
            pos,
            pos + wheel_forward * forces.longitudinal * 0.1 + wheel_left * forces.lateral * 0.1,
            YELLOW,
        );

        // Draw a line representing slip ratio
        gizmos.line_2d(
            pos + wheel_left * wheel.radius + telemetry.slip_ratio * wheel_forward * wheel.radius,
            pos - wheel_left * wheel.radius + telemetry.slip_ratio * wheel_forward * wheel.radius,
            RED,
        );
    }
}
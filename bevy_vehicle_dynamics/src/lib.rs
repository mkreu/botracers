use avian2d::prelude::*;
use bevy::{
    ecs::query::QueryData,
    math::ops::{atan, sin},
    prelude::*,
};

mod debug;

pub use debug::VehicleDynamicsDebugPlugin;

pub struct VehicleDynamicsPlugin;

impl Plugin for VehicleDynamicsPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(
            FixedUpdate,
            (drivetrain_system, compute_wheel_forces, drivetrain_feedback).chain(),
        );
    }
}

#[derive(Component)]
#[require(VehicleState)]
pub struct Vehicle {
    throttle: f32,
    max_torque: f32,
}

#[derive(Component, Default)]
pub struct VehicleState {
    drive_axle_angular_velocity: f32,
}

#[derive(Component)]
#[require(WheelState, WheelForces)]
pub struct Wheel {
    is_driven: bool,
    radius: f32,
    //mass: f32,
}

#[derive(Component, Default)]
struct WheelState {
    global_rotation: f32,
    global_velocity: Vec2,
    wheel_load: f32,
    axle_angular_velocity: f32,
    drive_torque: f32,
}

#[derive(Component, Default)]
pub struct WheelForces {
    longitudinal: f32,
    lateral: f32,
}

#[derive(Component, Default)]
pub struct WheelTelemetry {
    slip_angle: f32,
    slip_ratio: f32,
    wheel_forward: Vec2,
    wheel_velocity: Vec2,
}

#[derive(QueryData)]
pub struct PhysicsComponents<'w> {
    mass: &'w ComputedMass,
    linear_velocity: &'w LinearVelocity,
    angular_velocity: &'w AngularVelocity,
}

fn drivetrain_system(
    query: Query<(
        &Vehicle,
        &VehicleState,
        &Transform,
        PhysicsComponents,
        &Children,
    )>,
    mut wheel_query: Query<(&Transform, &Wheel, &mut WheelState)>,
) {
    for (car, car_state, car_transform, physics, children) in query.iter() {
        for child in children.iter() {
            if let Ok((wheel_transform, wheel, mut state)) = wheel_query.get_mut(child) {
                state.global_rotation = car_transform.rotation.to_euler(EulerRot::XYZ).2
                    + wheel_transform.rotation.to_euler(EulerRot::XYZ).2;
                let global_position = car_transform.transform_point(wheel_transform.translation);

                state.global_velocity =
                    physics.linear_velocity.0 + physics.angular_velocity.0 * global_position.yx();

                state.wheel_load = physics.mass.value() * 9.81 / 4.0; // Simplified load distribution

                state.drive_torque = if wheel.is_driven {
                    car.throttle * car.max_torque / 2.0
                } else {
                    0.0
                };
                state.axle_angular_velocity = car_state.drive_axle_angular_velocity;
            }
        }
    }
}

fn compute_wheel_forces(
    mut query: Query<
        (
            &Wheel,
            &WheelState,
            &mut WheelForces,
            Option<&mut WheelTelemetry>,
        ),
        With<Wheel>,
    >,
) {
    for (wheel, state, mut forces, telemetry) in query.iter_mut() {
        let wheel_forward = Vec2::new(state.global_rotation.cos(), state.global_rotation.sin());

        let slip_angle = state.global_velocity.angle_to(wheel_forward);

        let slip_ratio = {
            let velocity_along_forward = state.global_velocity.dot(wheel_forward);
            let expected_angular_velocity = velocity_along_forward / wheel.radius;
            if expected_angular_velocity.abs() > 0.1 {
                (state.axle_angular_velocity * wheel.radius) / expected_angular_velocity - 1.0
            } else {
                (state.axle_angular_velocity * wheel.radius) / 0.1
                    * state.axle_angular_velocity.signum()
                    - 1.0
            }
        };

        let lat_force = {
            const B: f32 = 3.0;
            const C: f32 = 2.7;
            const D: f32 = 1.0;
            const E: f32 = 1.0;

            let pacejka =
                D * sin(C * atan(B * slip_angle - E * (B * slip_angle - atan(B * slip_angle))));

            pacejka * state.wheel_load
        };

        let lon_force = {
            const B: f32 = 3.0;
            const C: f32 = 2.7;
            const D: f32 = 1.0;
            const E: f32 = 1.0;

            let pacejka =
                D * sin(C * atan(B * slip_ratio - E * (B * slip_ratio - atan(B * slip_ratio))));

            let drive_force = state.drive_torque / wheel.radius;
            let traction_force = (drive_force * pacejka).clamp(-state.wheel_load, state.wheel_load);

            pacejka * traction_force
        };

        forces.lateral = lat_force;
        forces.longitudinal = lon_force;

        if let Some(mut telemetry) = telemetry {
            telemetry.slip_angle = slip_angle;
            telemetry.slip_ratio = slip_ratio;
            telemetry.wheel_forward = wheel_forward;
            telemetry.wheel_velocity = state.global_velocity;
        }
    }
}

fn drivetrain_feedback(
    mut query: Query<(&Vehicle, &mut VehicleState, &Children)>,
    wheel_query: Query<(&Wheel, &WheelForces), With<Wheel>>,
    time: Res<Time>,
) {
    for (car, mut car_state, children) in query.iter_mut() {
        let mut total_traction_torque = 0.0;
        let mut angular_inertia = 0.0;

        for child in children.iter() {
            if let Ok((wheel, wheel_forces)) = wheel_query.get(child) {
                total_traction_torque += wheel_forces.longitudinal * wheel.radius;
                let wheel_mass = 5.0; // Assume a mass for inertia
                angular_inertia += wheel.radius * wheel.radius * wheel_mass;
            }
        }
        let driving_torque = car.throttle * car.max_torque;
        car_state.drive_axle_angular_velocity +=
            (driving_torque - total_traction_torque) / angular_inertia * time.delta_secs();
    }
}

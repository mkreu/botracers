use std::f32::consts::PI;

use avian2d::dynamics::rigid_body::forces::ForcesItem;
use bevy::color::palettes::css::*;
use bevy::prelude::*;
use avian2d::prelude::*;

use crate::{Car, DebugGizmos};


pub const WHEEL_BASE: f32 = 1.18;
pub const WHEEL_TRACK: f32 = 0.95;

#[derive(Component, Default, Clone)]
pub struct LongitudinalDebugData {
    pub speed_mps: f32,
    pub engine_rpm: f32,
    pub wheel_rpm: f32,
    pub clutch_s: f32,
    pub t_eng: f32,
    pub t_drive_axle: f32,
    pub t_brake_axle: f32,
    pub f_drive: f32,
    pub f_brake: f32,
    pub f_rr: f32,
    pub f_drag: f32,
    pub f_raw: f32,
    pub f_clamped: f32,
    pub a_mps2: f32,
    pub traction_limit: f32,
    pub throttle: f32,
    pub brake: f32,
}

#[derive(Resource, Clone, Copy)]
struct KartLongitudinalParams {
    mass_kg: f32,
    wheel_radius_m: f32,
    gear_ratio: f32,
    drivetrain_efficiency: f32,
    tire_mu: f32,
    rolling_resistance: f32,
    air_density: f32,
    drag_area: f32,
    torque_peak_nm: f32,
    torque_peak_rpm: f32,
    redline_torque_fraction: f32,
    idle_rpm: f32,
    clutch_on_rpm: f32,
    clutch_lock_rpm: f32,
    redline_rpm: f32,
    engine_brake_nm: f32,
    brake_max_axle_nm: f32,
    sync_rate: f32,
    free_rev_rate: f32,
}

impl Default for KartLongitudinalParams {
    fn default() -> Self {
        Self {
            mass_kg: 165.0,
            wheel_radius_m: 0.13,
            gear_ratio: 5.0,
            drivetrain_efficiency: 0.9,
            tire_mu: 1.0,
            rolling_resistance: 0.015,
            air_density: 1.225,
            drag_area: 0.75,
            torque_peak_nm: 22.0,
            torque_peak_rpm: 2800.0,
            redline_torque_fraction: 0.6,
            idle_rpm: 1800.0,
            clutch_on_rpm: 2100.0,
            clutch_lock_rpm: 2600.0,
            redline_rpm: 6200.0,
            engine_brake_nm: 3.0,
            brake_max_axle_nm: 400.0,
            sync_rate: 40.0,
            free_rev_rate: 10.0,
        }
    }
}

#[derive(Component)]
struct EmulatorDriver;

#[derive(Component)]
pub struct FrontWheel;

fn apply_car_forces(
    mut car_query: Query<(
        Entity,
        &Transform,
        &mut Car,
        &mut LongitudinalDebugData,
        &Children,
        Forces,
        Has<DebugGizmos>,
    )>,
    mut wheel_query: Query<&mut Transform, (With<FrontWheel>, Without<Car>)>,
    mut gizmos: Gizmos,
    params: Res<KartLongitudinalParams>,
    time: Res<Time<Fixed>>,
) {
    let dt = time.delta_secs();
    let g = 9.81_f32;

    for (_entity, transform, mut car, mut debug_data, children, mut forces, show_gizmos) in
        &mut car_query
    {
        let position = transform.translation.xy();
        let forward = transform.up().xy().normalize();
        let left = forward.perp();
        let throttle = car.accelerator.clamp(0.0, 1.0);
        let brake = car.brake.clamp(0.0, 1.0);
        let v_long = forces.linear_velocity().dot(forward);

        car.wheel_omega = v_long / params.wheel_radius_m;
        let wheel_rpm = rad_per_sec_to_rpm(car.wheel_omega.abs());

        let engine_rpm_prev = car.engine_rpm.max(params.idle_rpm);
        let engine_torque_full = engine_torque_full(engine_rpm_prev, &params);
        let mut t_eng = throttle * engine_torque_full - (1.0 - throttle) * params.engine_brake_nm;
        t_eng *= governor_scale(engine_rpm_prev, &params);

        let clutch_s = smoothstep(
            params.clutch_on_rpm,
            params.clutch_lock_rpm,
            engine_rpm_prev,
        );
        let t_drive_axle =
            params.drivetrain_efficiency * params.gear_ratio * clutch_s * t_eng.max(0.0);
        let t_brake_axle = brake * params.brake_max_axle_nm;

        let f_drive = t_drive_axle / params.wheel_radius_m;
        let f_brake = t_brake_axle / params.wheel_radius_m;
        let f_rr = params.rolling_resistance * params.mass_kg * g;
        let f_drag_mag = 0.5 * params.air_density * params.drag_area * v_long * v_long;
        let v_sign = if v_long.abs() < 0.05 {
            0.0
        } else {
            v_long.signum()
        };
        // Rolling resistance should oppose motion, not create reverse acceleration from rest.
        let rr_sign = if v_long.abs() < 0.05 {
            0.0
        } else {
            v_long.signum()
        };
        let f_raw = f_drive - f_brake - rr_sign * f_rr - v_sign * f_drag_mag;
        let traction_limit = params.tire_mu * params.mass_kg * g;
        let mut f_clamped = f_raw.clamp(-traction_limit, traction_limit);

        // Prevent low-speed sign-flip jitter while braking/coasting to a stop.
        if v_long.abs() < 0.1 && f_clamped < 0.0 {
            f_clamped = 0.0;
        }

        let a_long = f_clamped / params.mass_kg;
        forces.apply_linear_acceleration(forward * a_long);

        let omega_lock = params.gear_ratio * car.wheel_omega;
        let omega_idle = rpm_to_rad_per_sec(params.idle_rpm);
        let omega_max = rpm_to_rad_per_sec(params.redline_rpm);
        let omega_target = omega_idle + throttle * (omega_max - omega_idle);
        let mut omega_engine = rpm_to_rad_per_sec(engine_rpm_prev);
        omega_engine += params.sync_rate * clutch_s * (omega_lock - omega_engine) * dt;
        omega_engine +=
            params.free_rev_rate * (1.0 - clutch_s) * (omega_target - omega_engine) * dt;
        let omega_ceiling = rpm_to_rad_per_sec(params.redline_rpm + 500.0);
        omega_engine = omega_engine.clamp(omega_idle, omega_ceiling);
        car.engine_rpm = rad_per_sec_to_rpm(omega_engine);

        debug_data.speed_mps = v_long;
        debug_data.engine_rpm = car.engine_rpm;
        debug_data.wheel_rpm = wheel_rpm;
        debug_data.clutch_s = clutch_s;
        debug_data.t_eng = t_eng;
        debug_data.t_drive_axle = t_drive_axle;
        debug_data.t_brake_axle = t_brake_axle;
        debug_data.f_drive = f_drive;
        debug_data.f_brake = f_brake;
        debug_data.f_rr = f_rr;
        debug_data.f_drag = f_drag_mag;
        debug_data.f_raw = f_raw;
        debug_data.f_clamped = f_clamped;
        debug_data.a_mps2 = a_long;
        debug_data.traction_limit = traction_limit;
        debug_data.throttle = throttle;
        debug_data.brake = brake;

        if show_gizmos {
            gizmos.arrow_2d(position, position + forward * a_long * 0.3, WHITE);
        }

        apply_wheel_force(
            position,
            forward * WHEEL_BASE + left * -WHEEL_TRACK / 2.0,
            Vec2::from_angle(-car.steer).rotate(forward),
            &mut forces,
            &mut gizmos,
            show_gizmos,
        );
        apply_wheel_force(
            position,
            forward * WHEEL_BASE + left * WHEEL_TRACK / 2.0,
            Vec2::from_angle(-car.steer).rotate(forward),
            &mut forces,
            &mut gizmos,
            show_gizmos,
        );
        apply_wheel_force(
            position,
            left * -WHEEL_TRACK / 2.0,
            forward,
            &mut forces,
            &mut gizmos,
            show_gizmos,
        );
        apply_wheel_force(
            position,
            left * WHEEL_TRACK / 2.0,
            forward,
            &mut forces,
            &mut gizmos,
            show_gizmos,
        );

        for child in children.iter() {
            if let Ok(mut wheel_transform) = wheel_query.get_mut(child) {
                wheel_transform.rotation = Quat::from_rotation_z(-car.steer);
            }
        }
    }
}

fn apply_wheel_force(
    car_position: Vec2,
    wheel_offset: Vec2,
    wheel_forward: Vec2,
    forces: &mut ForcesItem<'_, '_>,
    gizmos: &mut Gizmos,
    show_gizmos: bool,
) {
    let wheel_pos = car_position + wheel_offset;
    let wheel_left = wheel_forward.perp();

    if show_gizmos {
        gizmos.arrow_2d(wheel_pos, wheel_pos + wheel_forward * 1.0, YELLOW);
        gizmos.arrow_2d(wheel_pos, wheel_pos + wheel_left * 0.5, YELLOW);
    }

    let o = forces.angular_velocity();
    let l = forces.linear_velocity();
    let wow = wheel_pos - car_position;
    let wheel_velocity = l + Vec2::new(-o * wow.y, o * wow.x);

    if show_gizmos {
        gizmos.arrow_2d(wheel_pos, wheel_pos + wheel_velocity * 0.1, GREEN);
    }

    if wheel_velocity.length() > 0.1 {
        let force = -wheel_velocity.normalize().dot(wheel_left)
            * wheel_left
            * 10.0_f32.min(wheel_velocity.length() * 5.0);
        if show_gizmos {
            gizmos.arrow_2d(wheel_pos, wheel_pos + force, RED);
        }
        forces.apply_linear_acceleration_at_point(force, wheel_pos);
    }
}



fn rpm_to_rad_per_sec(rpm: f32) -> f32 {
    rpm * (2.0 * PI / 60.0)
}

fn rad_per_sec_to_rpm(rad_per_sec: f32) -> f32 {
    rad_per_sec * (60.0 / (2.0 * PI))
}

fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    if edge1 <= edge0 {
        return if value < edge0 { 0.0 } else { 1.0 };
    }
    let x = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

fn engine_torque_full(rpm: f32, params: &KartLongitudinalParams) -> f32 {
    let x = ((rpm - params.torque_peak_rpm) / (params.redline_rpm - params.torque_peak_rpm))
        .clamp(0.0, 1.0);
    params.torque_peak_nm * (1.0 - (1.0 - params.redline_torque_fraction) * x * x)
}

fn governor_scale(rpm: f32, params: &KartLongitudinalParams) -> f32 {
    if rpm <= params.redline_rpm {
        1.0
    } else {
        (1.0 - (rpm - params.redline_rpm) / 500.0).clamp(0.0, 1.0)
    }
}
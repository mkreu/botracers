use std::f32::consts::PI;

use avian2d::prelude::{forces::ForcesItem, *};
use bevy::{
    color::palettes::css::{BLUE, GREEN, RED, WHITE, YELLOW},
    prelude::*,
    ui::debug,
};

use crate::race_runtime::SimState;

pub const WHEEL_BASE: f32 = 1.18;
pub const WHEEL_TRACK: f32 = 0.95;

#[derive(Component)]
pub struct Car {
    pub steer: f32,
    pub throttle: f32,
    pub brake: f32,
    pub wheel_omega: f32,
}

#[derive(Bundle)]
pub struct CarBundle {
    car: Car,
    debug_gizmos: DebugGizmos,
    rigidbody: RigidBody,
    friction: Friction,
    restitution: Restitution,
    mass: Mass,
    com: CenterOfMass,
    car_forces: CarForces,
    telemetry: CarTelemetry,
}

impl Default for CarBundle {
    fn default() -> Self {
        Self {
            car: Car {
                steer: 0.0,
                throttle: 0.0,
                brake: 0.0,
                wheel_omega: 0.0,
            },
            debug_gizmos: DebugGizmos,
            rigidbody: RigidBody::Dynamic,
            friction: Friction::new(0.1),
            restitution: Restitution::new(0.2),
            mass: Mass(165.0),
            com: CenterOfMass::new(0.0, 0.66),
            car_forces: CarForces::default(),
            telemetry: CarTelemetry::default(),
        }
    }
}

pub struct CarDynamicsPlugin;

impl Plugin for CarDynamicsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PowertrainParams::default())
            .insert_resource(WheelParams::default())
            .insert_resource(WheelOffsets::default())
            .add_systems(FixedUpdate, engine_system)
            .add_systems(
                FixedUpdate,
                wheel_system
                    .run_if(in_state(SimState::Racing))
                    .after(engine_system),
            )
            .add_systems(
                FixedUpdate,
                apply_car_forces
                    .after(wheel_system)
                    .run_if(in_state(SimState::Racing)),
            )
            .add_systems(Update, debug_car_forces);
    }
}

#[derive(Resource)]
struct WheelOffsets {
    front_left: Vec2,
    front_right: Vec2,
    rear_left: Vec2,
    rear_right: Vec2,
}

impl Default for WheelOffsets {
    fn default() -> Self {
        Self {
            front_left: Vec2::new(WHEEL_BASE, -WHEEL_TRACK / 2.0),
            front_right: Vec2::new(WHEEL_BASE, WHEEL_TRACK / 2.0),
            rear_left: Vec2::new(0.0, -WHEEL_TRACK / 2.0),
            rear_right: Vec2::new(0.0, WHEEL_TRACK / 2.0),
        }
    }
}

impl WheelOffsets {
    fn world_positons(&self, transform: &Transform) -> WheelOffsets {
        // I feel like left and forward should be swapped in the way the wheel offsets are
        // defined vs how the car's transform is oriented, but this seems to work so ¯\_(ツ)_/¯
        let left = transform.up().xy().normalize();
        let forward = left.perp();
        WheelOffsets {
            front_left: transform.translation.xy()
                + forward * self.front_left.y
                + left * self.front_left.x,
            front_right: transform.translation.xy()
                + forward * self.front_right.y
                + left * self.front_right.x,
            rear_left: transform.translation.xy()
                + forward * self.rear_left.y
                + left * self.rear_left.x,
            rear_right: transform.translation.xy()
                + forward * self.rear_right.y
                + left * self.rear_right.x,
        }
    }
}

#[derive(Resource)]
struct PowertrainParams {
    idle_rpm: f32,
    redline_rpm: f32,
    torque_peak_nm: f32,
    gear_ratio: f32,
}

#[derive(Resource)]
struct WheelParams {
    radius_m: f32,
    mass_kg: f32,
    tire_mu: f32,
}

impl Default for WheelParams {
    fn default() -> Self {
        Self {
            radius_m: 0.13,
            mass_kg: 10.0,
            tire_mu: 1.0,
        }
    }
}

#[derive(Component, Default)]
struct CarForces {
    body: Vec2,
    front_left: Vec2,
    front_right: Vec2,
    rear_left: Vec2,
    rear_right: Vec2,
}

impl Default for PowertrainParams {
    fn default() -> Self {
        Self {
            idle_rpm: 0.0,
            redline_rpm: 6000.0,
            torque_peak_nm: 20.0,
            gear_ratio: 5.0,
        }
    }
}

#[derive(Component, Default)]
pub struct CarTelemetry {
    pub speed_mps: f32,
    pub wheel_rpm: f32,
    pub slip_ratio: f32,
    pub f_drive: f32,
    pub f_max: f32,
    pub f_traction: f32,
    pub throttle: f32,
    pub brake: f32,
}

fn engine_system(mut car_query: Query<&mut Car>, params: Res<PowertrainParams>, time: Res<Time>) {
    car_query.iter_mut().for_each(|mut car| {
        //Doing nothing atm, but will eventually handle torque and clutch
    });
}

fn wheel_system(
    mut car_query: Query<(
        &Transform,
        &mut Car,
        &mut CarForces,
        &mut CarTelemetry,
        Forces,
        &Mass,
    )>,
    wheel_offsets: Res<WheelOffsets>,
    power_params: Res<PowertrainParams>,
    wheel_params: Res<WheelParams>,
    time: Res<Time>,
) {
    for (transform, mut car, mut forces, mut telemetry, pyhsics, mass) in &mut car_query {
        let speed_ms = transform.up().xy().dot(pyhsics.linear_velocity());
        telemetry.speed_mps = speed_ms;
        let true_wheel_omega = speed_ms / wheel_params.radius_m;
        let slip_ratio =
            (true_wheel_omega - car.wheel_omega).abs() / true_wheel_omega.abs().max(1e-3);
        let slip_factor = (slip_ratio *5.0).clamp(0.0, 1.0);
        telemetry.slip_ratio = slip_ratio;

        // m*r² / 2 (x2 for 2 driven wheels)
        let angular_inertia = wheel_params.mass_kg * wheel_params.radius_m * wheel_params.radius_m;
        let drive_force = car.throttle * power_params.torque_peak_nm * power_params.gear_ratio
            / wheel_params.radius_m;
        let max_traction_force = wheel_params.tire_mu * mass.0 * 9.81 / 2.0; // half the load on rear wheels
        let traction_force =
            (drive_force * slip_factor).clamp(-max_traction_force, max_traction_force);
        telemetry.f_drive = drive_force;
        telemetry.f_max = max_traction_force;
        telemetry.f_traction = traction_force;

        // 1/s = 1/s + kg*m²/s² / kg*m² * s
        car.wheel_omega += (drive_force - traction_force) / angular_inertia * time.delta_secs();
        // Clamp max wheel speed to max rpm
        let redline_omega = rpm_to_rad_per_sec(power_params.redline_rpm / power_params.gear_ratio);
        car.wheel_omega -= (car.wheel_omega - redline_omega).max(0.0);
        telemetry.wheel_rpm = rad_per_sec_to_rpm(car.wheel_omega);

        let forward = transform.up().xy().normalize();
        let position = transform.translation.xy();
        let wheel_positions = wheel_offsets.world_positons(transform);

        forces.front_left = compute_tire_force(
            pyhsics.linear_velocity(),
            pyhsics.angular_velocity(),
            wheel_positions.front_left - position,
            Vec2::from_angle(-car.steer).rotate(forward),
        );
        forces.front_right = compute_tire_force(
            pyhsics.linear_velocity(),
            pyhsics.angular_velocity(),
            wheel_positions.front_right - position,
            Vec2::from_angle(-car.steer).rotate(forward),
        );
        forces.rear_left = compute_tire_force(
            pyhsics.linear_velocity(),
            pyhsics.angular_velocity(),
            wheel_positions.rear_left - position,
            forward,
        ) + forward * traction_force * 0.5;
        forces.rear_right = compute_tire_force(
            pyhsics.linear_velocity(),
            pyhsics.angular_velocity(),
            wheel_positions.rear_right - position,
            forward,
        ) + forward * traction_force * 0.5;
    }
}

fn debug_car_forces(
    car_query: Query<(&Transform, &CarForces), With<DebugGizmos>>,
    mut gizmos: Gizmos,
    wheel_offsets: Res<WheelOffsets>,
) {
    for (transform, forces) in &car_query {
        let wheel_positions = wheel_offsets.world_positons(transform);

        gizmos.cross_2d(transform.translation.xy(), 1.0, WHITE);

        gizmos.arrow_2d(
            wheel_positions.front_left,
            wheel_positions.front_left + forces.front_left,
            RED,
        );
        gizmos.arrow_2d(
            wheel_positions.front_right,
            wheel_positions.front_right + forces.front_right,
            YELLOW,
        );
        gizmos.arrow_2d(
            wheel_positions.rear_left,
            wheel_positions.rear_left + forces.rear_left,
            GREEN,
        );
        gizmos.arrow_2d(
            wheel_positions.rear_right,
            wheel_positions.rear_right + forces.rear_right,
            BLUE,
        );
    }
}

fn apply_car_forces(
    mut car_query: Query<(&Transform, &CarForces, Forces)>,
    wheel_offsets: Res<WheelOffsets>,
) {
    for (transform, forces, mut physics_forces) in &mut car_query {
        let wheel_positions = wheel_offsets.world_positons(transform);
        physics_forces.apply_force(forces.body);
        physics_forces.apply_force_at_point(forces.front_left, wheel_positions.front_left);
        physics_forces.apply_force_at_point(forces.front_right, wheel_positions.front_right);
        physics_forces.apply_force_at_point(forces.rear_left, wheel_positions.rear_left);
        physics_forces.apply_force_at_point(forces.rear_right, wheel_positions.rear_right);
    }
}

fn compute_tire_force(
    car_linear_velocity: Vec2,
    car_angular_velocity: f32,
    wheel_offset: Vec2,
    wheel_forward: Vec2,
) -> Vec2 {
    let wheel_left = wheel_forward.perp();

    let wheel_velocity = car_linear_velocity
        + Vec2::new(
            -car_angular_velocity * wheel_offset.y,
            car_angular_velocity * wheel_offset.x,
        );

    if wheel_velocity.length() > 0.1 {
        let force = -wheel_velocity.normalize().dot(wheel_left)
            * wheel_left
            * 10.0_f32.min(wheel_velocity.length() * 5.0);
        return force;
    } else {
        return Vec2::ZERO;
    }
}

#[derive(Component)]
pub struct DebugGizmos;

#[derive(Component)]
pub struct FrontWheel;

/* Here for future reference
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
}*/

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

/*fn engine_torque_full(rpm: f32, params: &KartLongitudinalParams) -> f32 {
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

pub fn apply_car_forces(
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



#[cfg(test)]
mod tests {
    use super::{KartLongitudinalParams, engine_torque_full, governor_scale, smoothstep};

    #[test]
    fn smoothstep_clamps_and_is_monotonic() {
        assert_eq!(smoothstep(2.0, 4.0, 1.0), 0.0);
        assert_eq!(smoothstep(2.0, 4.0, 5.0), 1.0);

        let mut prev = 0.0;
        for i in 0..=20 {
            let x = 2.0 + (i as f32) * 0.1;
            let y = smoothstep(2.0, 4.0, x);
            assert!((0.0..=1.0).contains(&y));
            assert!(y >= prev - 1e-6);
            prev = y;
        }
    }

    #[test]
    fn torque_curve_peaks_near_target_and_drops_off() {
        let params = KartLongitudinalParams::default();
        let near_peak = engine_torque_full(params.torque_peak_rpm, &params);
        let low = engine_torque_full(1200.0, &params);
        let high = engine_torque_full(5200.0, &params);
        assert!(near_peak > low);
        assert!(near_peak > high);
    }

    #[test]
    fn governor_reduces_torque_above_redline() {
        let params = KartLongitudinalParams::default();
        assert_eq!(governor_scale(params.redline_rpm, &params), 1.0);
        assert!(governor_scale(params.redline_rpm + 250.0, &params) < 1.0);
        assert_eq!(governor_scale(params.redline_rpm + 1000.0, &params), 0.0);
    }

    #[test]
    fn traction_clamp_enforces_limit() {
        let params = KartLongitudinalParams::default();
        let limit = params.tire_mu * params.mass_kg * 9.81;
        let clamped = (limit * 3.0).clamp(-limit, limit);
        assert!(clamped <= limit);
        assert!(clamped >= -limit);
    }
}*/

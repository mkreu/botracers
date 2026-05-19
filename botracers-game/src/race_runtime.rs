use std::f32::consts::PI;

use avian2d::prelude::{forces::ForcesItem, *};
use bevy::{
    color::palettes::css::{GREEN, RED, WHITE, YELLOW},
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
};
use emulator::bevy::{CpuComponent, cpu_system};
use emulator::cpu::LogDevice;

use botracers_game::devices::{
    self, CarControlsDevice, CarDebugDevice, CarRadarDevice, CarStateDevice,
};
use botracers_game::track;
use botracers_game::track_format::TrackFile;
use botracers_game::{Car, DebugGizmos, RaceClock, devices::CarVisionDevice};

use crate::game_api::{DriverType, SpawnResolvedCarRequest};

pub struct RaceRuntimePlugin;

impl Plugin for RaceRuntimePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<SimState>()
            .insert_resource(RaceManager::default())
            .insert_resource(FollowCar::default())
            .insert_resource(RaceClock::default())
            .insert_resource(KartLongitudinalParams::default())
            .add_systems(Startup, pause_physics)
            .add_systems(OnEnter(SimState::Racing), unpause_physics)
            .add_systems(OnEnter(SimState::Paused), pause_physics)
            .add_systems(
                OnEnter(SimState::PreRace),
                (pause_physics, reset_race_clock),
            )
            .add_systems(
                Update,
                (handle_spawn_resolved_event, apply_cpu_frequency_setting),
            )
            .add_systems(Update, handle_car_input)
            .configure_sets(
                FixedUpdate,
                (CpuSystems::PreCpu, CpuSystems::Cpu, CpuSystems::PostCpu).chain(),
            )
            .add_systems(
                FixedUpdate,
                (
                    devices::car_state_system.in_set(CpuSystems::PreCpu),
                    devices::car_radar_system.in_set(CpuSystems::PreCpu),
                    devices::car_vision_system.in_set(CpuSystems::PreCpu),
                    cpu_system::<RacingCpuConfig>.in_set(CpuSystems::Cpu),
                    devices::car_controls_system.in_set(CpuSystems::PostCpu),
                    tick_race_clock.in_set(CpuSystems::PostCpu),
                )
                    .run_if(in_state(SimState::Racing)),
            )
            .add_systems(
                FixedUpdate,
                apply_car_forces.run_if(in_state(SimState::Racing)),
            )
            .add_systems(
                Update,
                (
                    update_fps_counter,
                    update_camera,
                    draw_gizmos,
                    devices::car_debug_system,
                ),
            );
    }
}

#[derive(Resource, Default)]
pub struct FollowCar {
    pub target: Option<Entity>,
}

#[derive(Component)]
pub struct CarLabel {
    pub name: String,
}



#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
enum CpuSystems {
    PreCpu,
    Cpu,
    PostCpu,
}





fn pause_physics(mut physics_time: ResMut<Time<Physics>>) {
    physics_time.pause();
}

fn unpause_physics(mut physics_time: ResMut<Time<Physics>>) {
    physics_time.unpause();
}

fn reset_race_clock(mut race_clock: ResMut<RaceClock>) {
    race_clock.reset();
}

fn tick_race_clock(mut race_clock: ResMut<RaceClock>, time: Res<Time<Fixed>>) {
    race_clock.tick(time.delta_secs());
}

fn grid_world_position(start_point: Vec2, tangent: Vec2, index: usize) -> Vec2 {
    let local = grid_local_position(index) + Vec2::new(0.0, -1.5);
    let right = Vec2::new(tangent.y, -tangent.x);
    start_point + right * local.x + tangent * local.y
}

fn handle_spawn_resolved_event(
    mut events: MessageReader<SpawnResolvedCarRequest>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut manager: ResMut<RaceManager>,
    cpu_frequency: Res<CpuFrequencySetting>,
    state: Res<State<SimState>>,
) {
    for event in events.read() {
        if *state.get() != SimState::PreRace {
            continue;
        }

        spawn_car_entry(
            &mut commands,
            &asset_server,
            &mut manager,
            &cpu_frequency,
            event.driver.clone(),
            &event.elf_bytes,
        );
    }
}

fn spawn_car_entry(
    commands: &mut Commands,
    asset_server: &AssetServer,
    manager: &mut RaceManager,
    cpu_frequency: &CpuFrequencySetting,
    driver: DriverType,
    elf_bytes: &[u8],
) {
    let car_index = manager.cars.len();
    let track_file =
        TrackFile::load_builtin().unwrap_or_else(|_| panic!("Failed to load track file"));
    let control_points = track_file.control_points_vec2();
    let spline = track::build_spline(&control_points);
    let (start_point, tangent) =
        start_frame_from_spline(&spline, track::first_point_from_file(&track_file));

    let position = grid_world_position(start_point, tangent, car_index);
    let car_name = format!("Car {}", manager.next_car_id);
    let entity = spawn_car(
        commands,
        asset_server,
        position,
        start_frame_rotation(tangent),
        &car_name,
        kart_body_color(car_index),
        elf_bytes,
        cpu_frequency.instructions_per_update(),
    );
    manager.cars.push(CarEntry {
        entity,
        name: car_name,
        driver,
        console_output: String::new(),
    });
    manager.next_car_id += 1;
}

fn spawn_car(
    commands: &mut Commands,
    asset_server: &AssetServer,
    position: Vec2,
    rotation: f32,
    name: &str,
    body_color: Color,
    bot_elf: &[u8],
    instructions_per_update: u32,
) -> Entity {
    let sprite_scale = Vec3::splat(0.008);

    let mut entity = commands.spawn((
        Transform::from_xyz(position.x, position.y, 1.0)
            .with_rotation(Quat::from_axis_angle(Vec3::Z, rotation)),
        Visibility::default(),
        RigidBody::Dynamic,
        //LinearDamping(0.1),
        Friction::new(0.1),
        Restitution::new(0.2),
        Car {
            steer: 0.0,
            accelerator: 0.0,
            brake: 0.0,
            engine_rpm: 1800.0,
            wheel_omega: 0.0,
        },
        CarLabel {
            name: name.to_string(),
        },
        LongitudinalDebugData::default(),
    ));

    let cpu = CpuComponent::new(bot_elf, instructions_per_update);
    entity.insert((
        EmulatorDriver,
        cpu,
        LogDevice::default(),
        CarStateDevice::default(),
        CarControlsDevice::default(),
        CarVisionDevice::default(),
        CarRadarDevice::default(),
        CarDebugDevice::default(),
    ));

    let entity_id = entity.id();

    entity.with_children(|parent| {
        parent.spawn((
            Collider::rectangle(1.25, 2.0),
            Transform::from_xyz(0.0, 0.66, 0.0),
        ));

        let mut body_sprite = Sprite::from_image(asset_server.load("kart_body.png"));
        body_sprite.color = body_color;
        parent.spawn((
            body_sprite,
            Transform::from_xyz(0.0, 0.66, 0.09).with_scale(sprite_scale),
            KartBodySprite,
        ));

        parent.spawn((
            Sprite::from_image(asset_server.load("kart_details.png")),
            Transform::from_xyz(0.0, 0.66, 0.1).with_scale(sprite_scale),
        ));

        parent
            .spawn((
                Transform::from_xyz(-WHEEL_TRACK / 2.0, WHEEL_BASE, 0.1),
                Visibility::default(),
                FrontWheel,
            ))
            .with_children(|parent| {
                parent.spawn((
                    Sprite::from_image(asset_server.load("kart_wheel.png")),
                    Transform::default()
                        .with_scale(sprite_scale)
                        .with_rotation(Quat::from_rotation_z(0.0)),
                ));
            });

        parent
            .spawn((
                Transform::from_xyz(WHEEL_TRACK / 2.0, WHEEL_BASE, 0.1),
                Visibility::default(),
                FrontWheel,
            ))
            .with_children(|parent| {
                parent.spawn((
                    Sprite::from_image(asset_server.load("kart_wheel.png")),
                    Transform::default()
                        .with_scale(sprite_scale)
                        .with_rotation(Quat::from_rotation_z(PI)),
                ));
            });
    });

    entity_id
}

fn kart_body_color(car_index: usize) -> Color {
    match car_index % 6 {
        0 => Color::srgb(1.0, 1.0, 0.08),
        1 => Color::srgb(0.1, 0.45, 1.0),
        2 => Color::srgb(1.0, 0.12, 0.1),
        3 => Color::srgb(0.1, 0.85, 0.25),
        4 => Color::srgb(0.8, 0.2, 1.0),
        _ => Color::srgb(1.0, 0.45, 0.05),
    }
}

fn apply_cpu_frequency_setting(
    cpu_frequency: Res<CpuFrequencySetting>,
    mut cpu_query: Query<&mut CpuComponent>,
) {
    if !cpu_frequency.is_changed() {
        return;
    }

    let instructions_per_update = cpu_frequency.instructions_per_update();
    for mut cpu in &mut cpu_query {
        cpu.set_instructions_per_update(instructions_per_update);
    }
}



#[derive(Component)]
pub struct KartBodySprite;

emulator::define_cpu_config! {
    RacingCpuConfig {
        1 => LogDevice,
        2 => CarStateDevice,
        3 => CarControlsDevice,
        4 => CarVisionDevice,
        5 => CarRadarDevice,
        6 => CarDebugDevice,
    }
}

fn handle_car_input(
    mut car_query: Query<&mut Car, Without<EmulatorDriver>>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    for mut car in &mut car_query {
        car.accelerator = if keyboard.pressed(KeyCode::KeyW) {
            1.0
        } else {
            0.0
        };
        car.brake = if keyboard.pressed(KeyCode::KeyS) {
            1.0
        } else {
            0.0
        };

        let max_steer = PI / 6.0;
        let steer_rate = 0.05 * car.steer.abs().max(0.1);
        if keyboard.pressed(KeyCode::KeyA) {
            car.steer = (-max_steer).max(car.steer - steer_rate);
        } else if keyboard.pressed(KeyCode::KeyD) {
            car.steer = max_steer.min(car.steer + steer_rate);
        } else {
            car.steer = if car.steer > 0.0 {
                (car.steer - steer_rate).max(0.0)
            } else {
                (car.steer + steer_rate).min(0.0)
            };
        }
    }
}



fn draw_gizmos(car_query: Query<(&Transform, &Car), With<DebugGizmos>>, mut gizmos: Gizmos) {
    for (transform, _car) in &car_query {
        gizmos.cross(transform.to_isometry(), 0.2, RED);
        gizmos.cross(
            Isometry3d::new(
                transform.translation + transform.up() * WHEEL_BASE,
                transform.rotation,
            ),
            0.2,
            RED,
        );
    }
}


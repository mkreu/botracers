use std::f32::consts::PI;

use avian2d::prelude::*;
use bevy::prelude::*;

mod cpu;
pub mod track;
mod vehicle_dynamics;

pub struct RaceRuntimePlugin;
pub use track::Track;

use crate::{
    cpu::CarCpuBundle,
    vehicle_dynamics::{FrontWheel, WHEEL_BASE, WHEEL_TRACK},
};

pub const FIXED_TICK_HZ: u32 = 200;

impl Plugin for RaceRuntimePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            PhysicsPlugins::default(),
            cpu::CarCpuPlugin,
            vehicle_dynamics::VehicleDynamicsPlugin,
        ))
        .insert_resource(Gravity::ZERO)
        .insert_resource(Time::<Fixed>::from_duration(
            std::time::Duration::from_secs_f32(1.0 / FIXED_TICK_HZ as f32),
        ))
        .init_state::<RaceState>()
        .add_systems(Startup, track::setup_track)
        .add_systems(FixedUpdate, tick_race_clock)
        .add_observer(spawn_car);
    }
}

#[derive(Event)]
pub struct SpawnCarRequest {
    pub name: String,
    pub artifact_id: ArtifactId,
    pub elf_bytes: Vec<u8>,
}

fn spawn_car(
    event: On<SpawnCarRequest>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut manager: ResMut<RaceSetupManager>,
    state: Res<State<RaceState>>,
    track: Res<Track>,
) {
    if *state.get() != RaceState::PreRace {
        return;
    }
    let car_index = manager.cars.len();
    let (position, rotation) = track.grid_start_position(car_index);
    let car_name = format!(
        "[{}] {}#{}",
        manager.next_car_id, event.name, event.artifact_id.0
    );

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
    ));
    entity.insert(CarCpuBundle::new(&event.elf_bytes));

    let entity_id = entity.id();
    entity.with_children(|parent| {
        parent.spawn((
            Collider::rectangle(1.25, 2.0),
            Transform::from_xyz(0.0, 0.66, 0.0),
        ));

        let mut body_sprite = Sprite::from_image(asset_server.load("kart_body.png"));
        let body_color = kart_body_color(car_index);
        body_sprite.color = body_color;
        parent.spawn((
            body_sprite,
            Transform::from_xyz(0.0, 0.66, 0.09).with_scale(sprite_scale),
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

    manager.cars.push(CarEntry {
        entity: entity_id,
        name: car_name,
    });
    manager.next_car_id += 1;
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

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RaceState {
    #[default]
    PreRace,
    Racing,
    Paused,
}

/// Marker component — when present on a car entity, debug gizmos are drawn for that car.
#[derive(Component)]
struct DebugGizmos;

#[derive(Component)]
pub struct Car {
    pub steer: f32,
    pub accelerator: f32,
    pub brake: f32,
    pub engine_rpm: f32,
    pub wheel_omega: f32,
}

#[derive(Resource, Default)]
pub struct RaceClock {
    elapsed_secs: f32,
}

impl RaceClock {
    pub fn elapsed_secs(&self) -> f32 {
        self.elapsed_secs
    }

    pub fn reset(&mut self) {
        self.elapsed_secs = 0.0;
    }

    pub fn tick(&mut self, delta_secs: f32) {
        self.elapsed_secs += delta_secs;
    }
}

fn tick_race_clock(mut race_clock: ResMut<RaceClock>, time: Res<Time<Fixed>>) {
    race_clock.tick(time.delta_secs());
}

#[derive(Resource)]
pub struct RaceSetupManager {
    pub cars: Vec<CarEntry>,
    pub next_car_id: u32,
}

impl Default for RaceSetupManager {
    fn default() -> Self {
        Self {
            cars: Vec::new(),
            next_car_id: 1,
        }
    }
}

pub struct CarEntry {
    pub entity: Entity,
    pub name: String,
}

pub struct ArtifactId(i64);

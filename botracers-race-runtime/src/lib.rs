use avian2d::{PhysicsPlugins, dynamics::integrator::Gravity};
use bevy::prelude::*;

mod cpu;
mod track;
mod vehicle_dynamics;

pub struct RaceRuntimePlugin;
pub use track::Track;

impl Plugin for RaceRuntimePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((PhysicsPlugins::default(), cpu::CarCpuPlugin))
            .insert_resource(Gravity::ZERO)
            .init_state::<RaceState>()
            .add_systems(Startup, track::setup_track);
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
    pub driver: ArtifactId,
}

pub struct ArtifactId(i64);

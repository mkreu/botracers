use bevy::prelude::*;

pub mod devices;
pub mod track;
pub mod track_format;

/// Marker component — when present on a car entity, debug gizmos are drawn for that car.
#[derive(Component)]
pub struct DebugGizmos;

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

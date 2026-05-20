use bevy::prelude::*;
use emulator::bevy::{CpuClockSpeed, CpuComponent};

use crate::RaceState;

mod devices;

pub struct CarCpuPlugin;

impl Plugin for CarCpuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CpuFrequencySetting>()
            .add_systems(
                FixedUpdate,
                (
                    devices::car_state::system.in_set(CpuSystems::PreCpu),
                    devices::car_radar::system.in_set(CpuSystems::PreCpu),
                    devices::car_vision::system.in_set(CpuSystems::PreCpu),
                    sync_cpu_clock_speed.in_set(CpuSystems::PreCpu),
                    emulator::bevy::cpu_system::<RacingCpuConfig>.in_set(CpuSystems::Cpu),
                    devices::car_controls::system.in_set(CpuSystems::PostCpu),
                    //tick_race_clock.in_set(CpuSystems::PostCpu),
                )
                    .run_if(in_state(RaceState::Racing)),
            )
            .add_systems(Update, devices::car_debug::system);
    }
}

emulator::define_cpu_config! {
    RacingCpuConfig {
        1 => emulator::cpu::LogDevice,
        2 => devices::car_state::CarStateDevice,
        3 => devices::car_controls::CarControlsDevice,
        4 => devices::car_vision::CarVisionDevice,
        5 => devices::car_radar::CarRadarDevice,
        6 => devices::car_debug::CarDebugDevice,
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
enum CpuSystems {
    PreCpu,
    Cpu,
    PostCpu,
}

#[derive(Bundle)]
pub struct CarCpuBundle(
    CpuComponent,
    devices::car_debug::CarDebugDevice,
    devices::car_state::CarStateDevice,
    devices::car_controls::CarControlsDevice,
    devices::car_radar::CarRadarDevice,
    devices::car_vision::CarVisionDevice,
);

impl CarCpuBundle {
    pub fn new(elf_bytes: &[u8]) -> Self {
        Self(
            CpuComponent::new(elf_bytes),
            devices::car_debug::CarDebugDevice::default(),
            devices::car_state::CarStateDevice::default(),
            devices::car_controls::CarControlsDevice::default(),
            devices::car_radar::CarRadarDevice::default(),
            devices::car_vision::CarVisionDevice::default(),
        )
    }
}

const CPU_FREQUENCY_PRESETS_HZ: [u32; 10] = [
    1_000, 5_000, 10_000, 20_000, 50_000, 100_000, 200_000, 500_000, 1_000_000, 2_000_000,
];

#[derive(Resource, Clone, Copy)]
pub struct CpuFrequencySetting {
    preset_index: usize,
}

fn sync_cpu_clock_speed(mut clock_speed: ResMut<CpuClockSpeed>, setting: Res<CpuFrequencySetting>) {
    clock_speed.instructions_per_update = setting.instructions_per_update();
}

impl Default for CpuFrequencySetting {
    fn default() -> Self {
        Self {
            preset_index: CPU_FREQUENCY_PRESETS_HZ.len() - 1,
        }
    }
}

impl CpuFrequencySetting {
    pub fn hz(&self) -> u32 {
        CPU_FREQUENCY_PRESETS_HZ[self.preset_index]
    }

    pub fn instructions_per_update(&self) -> u32 {
        (self.hz() / super::FIXED_TICK_HZ).max(1)
    }

    pub fn step_up(&mut self) {
        if self.preset_index + 1 < CPU_FREQUENCY_PRESETS_HZ.len() {
            self.preset_index += 1;
        }
    }

    pub fn step_down(&mut self) {
        if self.preset_index > 0 {
            self.preset_index -= 1;
        }
    }

    pub fn format_hz_label(&self) -> String {
        let hz = self.hz();
        if hz >= 1_000_000 {
            format!("{:.1} MHz", hz as f32 / 1_000_000.0)
        } else {
            format!("{} kHz", hz / 1_000)
        }
    }
}

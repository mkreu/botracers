use bevy::prelude::*;

pub struct CarCpuPlugin;

impl Plugin for CarCpuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CpuFrequencySetting>();

    }
}

pub const FIXED_TICK_HZ: u32 = 200;
const CPU_FREQUENCY_PRESETS_HZ: [u32; 10] = [
    1_000, 5_000, 10_000, 20_000, 50_000, 100_000, 200_000, 500_000, 1_000_000, 2_000_000,
];

#[derive(Resource, Clone, Copy)]
pub struct CpuFrequencySetting {
    preset_index: usize,
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
        (self.hz() / FIXED_TICK_HZ).max(1)
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

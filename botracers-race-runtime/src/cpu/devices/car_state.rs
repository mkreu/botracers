use avian2d::prelude::LinearVelocity;
use bevy::prelude::*;
use emulator::cpu::Device;

use crate::RaceClock;

/// Memory-mapped device that provides car state to the RISC-V bot.
///
/// Layout (all f32, little-endian):
///   0x00: speed
///   0x04: elapsed race time in seconds
#[derive(Component)]
pub struct CarStateDevice {
    data: [u8; 8], // 2 × f32
}

impl Default for CarStateDevice {
    fn default() -> Self {
        Self { data: [0u8; 8] }
    }
}

impl CarStateDevice {
    fn write_f32(&mut self, offset: usize, value: f32) {
        let bytes = value.to_le_bytes();
        self.data[offset..offset + 4].copy_from_slice(&bytes);
    }

    /// Write the full car state from the simulation.
    pub fn update(&mut self, speed: f32, elapsed_secs: f32) {
        self.write_f32(0x00, speed);
        self.write_f32(0x04, elapsed_secs);
    }
}

impl Device for CarStateDevice {
    fn load(&self, addr: u32, size: u32) -> Result<u32, ()> {
        let addr = addr as usize;
        match size {
            8 => {
                if addr < self.data.len() {
                    Ok(self.data[addr] as u32)
                } else {
                    Ok(0)
                }
            }
            16 => {
                if addr + 1 < self.data.len() {
                    Ok((self.data[addr] as u32) | ((self.data[addr + 1] as u32) << 8))
                } else {
                    Ok(0)
                }
            }
            32 => {
                if addr + 3 < self.data.len() {
                    Ok((self.data[addr] as u32)
                        | ((self.data[addr + 1] as u32) << 8)
                        | ((self.data[addr + 2] as u32) << 16)
                        | ((self.data[addr + 3] as u32) << 24))
                } else {
                    Ok(0)
                }
            }
            _ => Err(()),
        }
    }

    fn store(&mut self, _addr: u32, _size: u32, _value: u32) -> Result<(), ()> {
        // CarState is read-only from the bot's perspective; silently ignore writes
        Ok(())
    }
}

/// Runs BEFORE cpu_system::<RacingCpuConfig>: writes host car state into CarStateDevice.
pub fn system(
    race_clock: Res<RaceClock>,
    mut emu_query: Query<(&LinearVelocity, &mut CarStateDevice)>,
) {
    for (velocity, mut state_dev) in &mut emu_query {
        state_dev.update(velocity.length(), race_clock.elapsed_secs());
    }
}

use bevy::color::palettes::css::ORANGE;
use bevy::prelude::*;
use emulator::cpu::Device;

use crate::DebugGizmos;

/// Memory-mapped device which allows the RISC-V bot to draw debug lines.
///
/// Layout (little-endian):
///   0x00: next line point x
///   0x04: next line point y
///   0x08: command (u32: 0 = move, 1 = line, 2 = submit)
#[derive(Component)]
pub struct CarDebugDevice {
    next_point: Vec2,
    draw_cache: Vec<Vec<Vec2>>,
    pending_cache: Vec<Vec<Vec2>>,
}

impl Default for CarDebugDevice {
    fn default() -> Self {
        Self {
            next_point: Vec2::ZERO,
            draw_cache: Vec::new(),
            pending_cache: Vec::new(),
        }
    }
}

impl CarDebugDevice {
    fn apply_command(&mut self, command: u32) {
        match command {
            0 => self.pending_cache.push(vec![self.next_point]),
            1 => {
                if let Some(line_strip) = self.pending_cache.last_mut() {
                    line_strip.push(self.next_point);
                } else {
                    self.pending_cache.push(vec![self.next_point]);
                }
            }
            2 => self.draw_cache = std::mem::take(&mut self.pending_cache),
            _ => {}
        }
    }
}

impl Device for CarDebugDevice {
    fn load(&self, _addr: u32, _size: u32) -> Result<u32, ()> {
        // This device is write-only
        Ok(0)
    }

    fn store(&mut self, addr: u32, size: u32, value: u32) -> Result<(), ()> {
        if size != 32 {
            return Err(());
        }

        match addr {
            0x00 => self.next_point.x = f32::from_bits(value),
            0x04 => self.next_point.y = f32::from_bits(value),
            0x08 => self.apply_command(value),
            _ => return Err(()),
        }
        Ok(())
    }
}

/// Draws bot-authored debug lines in car-local coordinates.
pub fn system(
    emu_query: Query<(&Transform, &CarDebugDevice), With<DebugGizmos>>,
    mut gizmos: Gizmos,
) {
    for (transform, debug_dev) in &emu_query {
        for line_strip in &debug_dev.draw_cache {
            if line_strip.len() < 2 {
                continue;
            }

            let points = line_strip.iter().map(|point| {
                transform
                    .transform_point(Vec3::new(point.x, point.y, 0.0))
                    .xy()
            });
            gizmos.linestrip_2d(points, ORANGE);
        }
    }
}

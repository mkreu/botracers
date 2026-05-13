use avian2d::prelude::*;
use bevy::color::palettes::css::LIGHT_CYAN;
use bevy::prelude::*;
use emulator::cpu::Device;

use crate::DebugGizmos;
use crate::track::TrackSpline;

/// Memory-mapped device that provides car / track vision to the RISC-V bot.
///
/// Layout (all f32, little-endian):
///   0x00: lateral offset to track centerline
///   0x04: angle to track centerline
///
#[derive(Component)]
pub struct CarVisionDevice {
    data: [u8; MEM_SIZE], // 5 × f32
}

const MEM_SIZE: usize = 4 * 2;

impl Default for CarVisionDevice {
    fn default() -> Self {
        Self {
            data: [0u8; MEM_SIZE],
        }
    }
}

impl CarVisionDevice {
    fn write_f32(&mut self, offset: usize, value: f32) {
        let bytes = value.to_le_bytes();
        self.data[offset..offset + 4].copy_from_slice(&bytes);
    }

    /// Write the full vision state from the simulation.
    pub fn update(&mut self, offset: f32, angle: f32) {
        self.write_f32(0x00, offset);
        self.write_f32(0x04, angle);
    }
}

impl Device for CarVisionDevice {
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
        // CarVision is read-only from the bot's perspective; silently ignore writes
        Ok(())
    }
}

/// Runs BEFORE cpu_system::<RacingCpuConfig>: writes host car kinematics into CarVisionDevice.
pub fn system(
    mut emu_query: Query<(
        &Transform,
        &LinearVelocity,
        &mut CarVisionDevice,
        Has<DebugGizmos>,
    )>,
    track_spline: Res<TrackSpline>,
    mut gizmos: Gizmos,
) {
    // Two-pass closest-point search.
    // Coarse pass identifies the rough segment; fine pass densely resamples a
    // ±1-coarse-step window around it (with cyclic wrapping).  The total sample
    // count is the same as a naive 256-point sweep but gives ~64× better t
    // resolution near the car.  Using the tangent at the refined t means the
    // offset vector is (geometrically) orthogonal to the spline at that point.
    const COARSE_SAMPLES: usize = 128;
    const FINE_SAMPLES: usize = 128;

    let spline = &track_spline.spline;
    let t_max = spline.domain().end();
    let coarse_step = t_max / COARSE_SAMPLES as f32;

    for (transform, _velocity, mut vision_dev, show_gizmos) in &mut emu_query {
        let car_pos = transform.translation.xy();
        let car_forward = transform.up().xy().normalize();

        // Coarse pass — find the nearest segment.
        let mut best_t = 0.0f32;
        let mut best_dist_sq = f32::INFINITY;
        for i in 0..COARSE_SAMPLES {
            let t = (i as f32 / COARSE_SAMPLES as f32) * t_max;
            let dist_sq = car_pos.distance_squared(spline.position(t));
            if dist_sq < best_dist_sq {
                best_dist_sq = dist_sq;
                best_t = t;
            }
        }

        // Fine pass — densely resample ±1 coarse step around best_t.
        // rem_euclid handles wrap-around near t=0 / t=t_max for cyclic splines.
        let t_start = best_t - coarse_step;
        let t_end = best_t + coarse_step;
        for i in 0..=FINE_SAMPLES {
            let t =
                (t_start + (i as f32 / FINE_SAMPLES as f32) * (t_end - t_start)).rem_euclid(t_max);
            let dist_sq = car_pos.distance_squared(spline.position(t));
            if dist_sq < best_dist_sq {
                best_dist_sq = dist_sq;
                best_t = t;
            }
        }

        // At the closest point on the curve, (car_pos - closest) is orthogonal
        // to the tangent by definition, so the offset is a true perpendicular distance.
        let closest = spline.position(best_t);
        let tangent = spline.velocity(best_t).normalize_or_zero();
        let normal = Vec2::new(-tangent.y, tangent.x); // left-hand normal

        // Positive = car is left of centreline; negative = right.
        let offset = (car_pos - closest).dot(normal);
        // Signed angle from track tangent to car heading (radians).
        let angle = tangent.angle_to(car_forward);

        if show_gizmos {
            // Draw the offset vector: from the closest spline point to the car position.
            gizmos.line_2d(closest, car_pos, LIGHT_CYAN);
        }

        vision_dev.update(offset, angle);
    }
}

use avian2d::prelude::*;
use bevy::color::palettes::css::LIGHT_CYAN;
use bevy::prelude::*;
use emulator::cpu::Device;

use crate::DebugGizmos;
use crate::track::TrackSpline;

/// Memory-mapped device that provides car / track vision to the RISC-V bot.
///
/// Layout (all f32, little-endian):
///   0x00: lateral offset to track centreline (m, positive = left of centre)  [read]
///   0x04: angle to track tangent (rad, positive = heading left)              [read]
///   0x08: lookahead request (m, write; rounded to nearest metre, clamped to [0, 50]) [write]
///   0x0C: curvature response (rad/m, positive = left turn)                   [read]
///
/// Protocol: bot writes desired lookahead in metres to 0x08; the device
/// immediately looks up the pre-cached curvature and populates 0x0C.
/// Curvature is pre-cached at 1-metre resolution for lookaheads 0..=50 m.
#[derive(Component)]
pub struct CarVisionDevice {
    /// 4 × f32 registers: [offset, angle, lookahead_req, curvature_resp]
    regs: [u8; 16],
    /// Pre-cached signed curvature at 1 m resolution; index = metres ahead (0..=50)
    curvature_cache: [f32; 51],
}

impl Default for CarVisionDevice {
    fn default() -> Self {
        Self {
            regs: [0u8; 16],
            curvature_cache: [0.0f32; 51],
        }
    }
}

impl CarVisionDevice {
    fn write_reg_f32(&mut self, offset: usize, value: f32) {
        self.regs[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn read_reg_f32(&self, offset: usize) -> f32 {
        f32::from_le_bytes(self.regs[offset..offset + 4].try_into().unwrap())
    }

    fn lookup_curvature(&self, lookahead_m: f32) -> f32 {
        if lookahead_m.is_nan() {
            return 0.0;
        }
        let idx = lookahead_m.round().clamp(0.0, 50.0) as usize;
        self.curvature_cache[idx]
    }

    /// Write the full vision state from the simulation.
    /// Also refreshes the curvature response for the current lookahead request.
    pub fn update(&mut self, offset: f32, angle: f32, curvature_cache: [f32; 51]) {
        self.write_reg_f32(0x00, offset);
        self.write_reg_f32(0x04, angle);
        self.curvature_cache = curvature_cache;
        // Re-evaluate the response in case the cache changed under the existing request.
        let req = self.read_reg_f32(0x08);
        let curv = self.lookup_curvature(req);
        self.write_reg_f32(0x0C, curv);
    }
}

impl Device for CarVisionDevice {
    fn load(&self, addr: u32, size: u32) -> Result<u32, ()> {
        let addr = addr as usize;
        match size {
            8 => Ok(self.regs.get(addr).copied().unwrap_or(0) as u32),
            16 => {
                if addr + 1 < self.regs.len() {
                    Ok((self.regs[addr] as u32) | ((self.regs[addr + 1] as u32) << 8))
                } else {
                    Ok(0)
                }
            }
            32 => {
                if addr + 3 < self.regs.len() {
                    Ok((self.regs[addr] as u32)
                        | ((self.regs[addr + 1] as u32) << 8)
                        | ((self.regs[addr + 2] as u32) << 16)
                        | ((self.regs[addr + 3] as u32) << 24))
                } else {
                    Ok(0)
                }
            }
            _ => Err(()),
        }
    }

    fn store(&mut self, addr: u32, size: u32, value: u32) -> Result<(), ()> {
        // Only 32-bit writes to the lookahead request register are meaningful.
        if size == 32 && addr == 0x08 {
            let lookahead_m = f32::from_bits(value);
            let curv = self.lookup_curvature(lookahead_m);
            self.write_reg_f32(0x08, lookahead_m);
            self.write_reg_f32(0x0C, curv);
        }
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

        // Pre-cache curvature at 1 m resolution for 0..=50 m ahead.
        // Walk forward incrementally from best_t; breaking at 50 m is always
        // fast (~50 iterations on a typical track with 2000 t-steps per lap).
        let walk_step = t_max / 2000.0;
        let mut walk_t = best_t;
        let mut walk_prev = spline.position(walk_t);
        let mut walk_arc = 0.0f32;

        let mut curvature_cache = [0.0f32; 51];
        curvature_cache[0] = signed_curvature(spline, walk_t);

        let mut next_m = 1usize;
        for _ in 0..2000 {
            if next_m > 50 {
                break;
            }
            walk_t = (walk_t + walk_step).rem_euclid(t_max);
            let pos = spline.position(walk_t);
            walk_arc += walk_prev.distance(pos);
            walk_prev = pos;
            while next_m <= 50 && walk_arc >= next_m as f32 {
                curvature_cache[next_m] = signed_curvature(spline, walk_t);
                next_m += 1;
            }
        }
        // Safety: fill any remaining entries with the last sampled value.
        let last = curvature_cache[next_m.saturating_sub(1)];
        for i in next_m..=50 {
            curvature_cache[i] = last;
        }

        if show_gizmos {
            // Draw the offset vector: from the closest spline point to the car position.
            gizmos.line_2d(closest, car_pos, LIGHT_CYAN);
        }

        vision_dev.update(offset, angle, curvature_cache);
    }
}

/// Signed curvature of `spline` at parameter `t`.
/// κ = (v × a) / |v|³  — positive = left turn, negative = right turn.
fn signed_curvature(spline: &CubicCurve<Vec2>, t: f32) -> f32 {
    let v = spline.velocity(t);
    let a = spline.acceleration(t);
    let cross = v.x * a.y - v.y * a.x;
    let speed_sq = v.length_squared();
    if speed_sq < 1e-10 {
        return 0.0;
    }
    cross / (speed_sq * speed_sq.sqrt())
}

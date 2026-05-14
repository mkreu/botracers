use core::ptr;

use bevy_math::Vec2;

pub struct CarControls {
    accelerator: *mut f32,
    brake: *mut f32,
    steering: *mut f32,
}

impl CarControls {
    pub const fn bind(slot: usize) -> Self {
        Self {
            accelerator: (slot + 0x00) as *mut f32,
            brake: (slot + 0x04) as *mut f32,
            steering: (slot + 0x08) as *mut f32,
        }
    }
    pub fn set_accelerator(&mut self, value: f32) {
        unsafe {
            ptr::write_volatile(self.accelerator, value);
        }
    }
    pub fn set_brake(&mut self, value: f32) {
        unsafe {
            ptr::write_volatile(self.brake, value);
        }
    }
    pub fn set_steering(&mut self, value: f32) {
        unsafe {
            ptr::write_volatile(self.steering, value);
        }
    }
    pub fn accelerator(&self) -> f32 {
        unsafe { ptr::read_volatile(self.accelerator) }
    }
    pub fn brake(&self) -> f32 {
        unsafe { ptr::read_volatile(self.brake) }
    }
    pub fn steering(&self) -> f32 {
        unsafe { ptr::read_volatile(self.steering) }
    }
}

/// SLOT2 (0x200) — car state, read-only.
///
/// Layout:
///   0x00: speed (m/s)
pub struct CarState {
    speed: *const f32,
}

impl CarState {
    pub const fn bind(slot: usize) -> Self {
        Self {
            speed: (slot + 0x00) as *const f32,
        }
    }
    pub fn speed(&self) -> f32 {
        unsafe { ptr::read_volatile(self.speed) }
    }
}

/// SLOT4 (0x400) — track vision.
///
/// Protocol: bot writes desired lookahead in metres (0..=50) to offset 0x08,
/// then reads the pre-cached curvature at that distance from offset 0x0C.
/// Fractional lookahead values are rounded to the nearest integer metre.
///
/// Layout:
///   0x00: lateral offset to track centreline (m, positive = left of centre)  [read]
///   0x04: heading angle relative to track tangent (rad, positive = heading left) [read]
///   0x08: lookahead request (m, write; rounded to nearest metre, clamped to [0, 50]) [write]
///   0x0C: curvature response (rad/m, positive = left turn) [read]
pub struct CarVision {
    offset: *const f32,
    angle: *const f32,
    lookahead_request: *mut f32,
    curvature_response: *const f32,
}

impl CarVision {
    pub const fn bind(slot: usize) -> Self {
        Self {
            offset: (slot + 0x00) as *const f32,
            angle: (slot + 0x04) as *const f32,
            lookahead_request: (slot + 0x08) as *mut f32,
            curvature_response: (slot + 0x0C) as *const f32,
        }
    }

    /// Lateral offset to track centreline in metres.
    /// Positive = car is left of centre, negative = right.
    pub fn offset(&self) -> f32 {
        unsafe { ptr::read_volatile(self.offset) }
    }

    /// Car heading angle relative to the track tangent in radians.
    /// Positive = heading left of tangent.
    pub fn angle(&self) -> f32 {
        unsafe { ptr::read_volatile(self.angle) }
    }

    /// Query the signed curvature (rad/m) at `lookahead_m` metres ahead along
    /// the track centreline. Positive = left turn.
    ///
    /// Writes `lookahead_m` to the device's request register, then reads back
    /// the pre-cached curvature. Values are rounded to the nearest integer metre
    /// and clamped to [0, 50].
    pub fn curvature_at(&mut self, lookahead_m: f32) -> f32 {
        unsafe {
            ptr::write_volatile(self.lookahead_request, lookahead_m);
            ptr::read_volatile(self.curvature_response)
        }
    }
}

pub struct CarRadar {
    car_x: [*const f32; 4],
    car_y: [*const f32; 4],
}

impl CarRadar {
    pub const fn bind(slot: usize) -> Self {
        Self {
            car_x: [
                (slot + 0x00) as *const f32,
                (slot + 0x08) as *const f32,
                (slot + 0x10) as *const f32,
                (slot + 0x18) as *const f32,
            ],
            car_y: [
                (slot + 0x04) as *const f32,
                (slot + 0x0C) as *const f32,
                (slot + 0x14) as *const f32,
                (slot + 0x1C) as *const f32,
            ],
        }
    }

    pub fn positions(&self) -> [Option<Vec2>; 4] {
        unsafe {
            let x0 = ptr::read_volatile(self.car_x[0]);
            let y0 = ptr::read_volatile(self.car_y[0]);
            let x1 = ptr::read_volatile(self.car_x[1]);
            let y1 = ptr::read_volatile(self.car_y[1]);
            let x2 = ptr::read_volatile(self.car_x[2]);
            let y2 = ptr::read_volatile(self.car_y[2]);
            let x3 = ptr::read_volatile(self.car_x[3]);
            let y3 = ptr::read_volatile(self.car_y[3]);

            [
                if x0.is_nan() || y0.is_nan() {
                    None
                } else {
                    Some(Vec2::new(x0, y0))
                },
                if x1.is_nan() || y1.is_nan() {
                    None
                } else {
                    Some(Vec2::new(x1, y1))
                },
                if x2.is_nan() || y2.is_nan() {
                    None
                } else {
                    Some(Vec2::new(x2, y2))
                },
                if x3.is_nan() || y3.is_nan() {
                    None
                } else {
                    Some(Vec2::new(x3, y3))
                },
            ]
        }
    }

    pub fn position(&self, index: usize) -> Option<Vec2> {
        if index >= self.car_x.len() {
            return None;
        }

        unsafe {
            let x = ptr::read_volatile(self.car_x[index]);
            let y = ptr::read_volatile(self.car_y[index]);
            if x.is_nan() || y.is_nan() {
                None
            } else {
                Some(Vec2::new(x, y))
            }
        }
    }
}

/// SLOT6 (0x600) — car-local debug line drawing.
///
/// Layout:
///   0x00: next line point x [write]
///   0x04: next line point y [write]
///   0x08: command (u32: 0 = move, 1 = line, 2 = submit) [write]
pub struct CarDebug {
    point_x: *mut f32,
    point_y: *mut f32,
    command: *mut u32,
}

impl CarDebug {
    pub const fn bind(slot: usize) -> Self {
        Self {
            point_x: (slot + 0x00) as *mut f32,
            point_y: (slot + 0x04) as *mut f32,
            command: (slot + 0x08) as *mut u32,
        }
    }

    pub fn submit(&mut self) {
        self.write_command(2);
    }

    pub fn move_to(&mut self, point: Vec2) {
        self.write_point_command(point, 0);
    }

    pub fn line_to(&mut self, point: Vec2) {
        self.write_point_command(point, 1);
    }

    pub fn move_to_xy(&mut self, x: f32, y: f32) {
        self.move_to(Vec2::new(x, y));
    }

    pub fn line_to_xy(&mut self, x: f32, y: f32) {
        self.line_to(Vec2::new(x, y));
    }

    fn write_point_command(&mut self, point: Vec2, command: u32) {
        unsafe {
            ptr::write_volatile(self.point_x, point.x);
            ptr::write_volatile(self.point_y, point.y);
            ptr::write_volatile(self.command, command);
        }
    }

    fn write_command(&mut self, command: u32) {
        unsafe {
            ptr::write_volatile(self.command, command);
        }
    }
}

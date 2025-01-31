use egui::{InputState, Modifiers};
use glam::{Mat4, Vec3};

/// An fps camera in 3d space, with up always being in the positive Y direction.
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    /// The position.
    pub eye: glam::Vec3,

    /// The euler angle defining rotation around the y axis.
    pub yaw: f32,
    /// The euler angle defining rotation around the x axis.
    pub pitch: f32,

    /// Has the camera moved since the last frame.
    pub moved: bool,
}

impl Camera {
    pub fn new_facing(position: Vec3, forward: Vec3) -> Self {
        let Vec3 { x, y, z } = forward.normalize();

        let pitch = y.asin().to_degrees();
        let yaw = f32::atan2(z, x).to_degrees();

        Self {
            eye: position,
            yaw,
            pitch,
            moved: false,
        }
    }

    pub fn forward(&self) -> Vec3 {
        Vec3::new(
            self.yaw.to_radians().cos() * self.pitch.to_radians().cos(),
            self.pitch.to_radians().sin(),
            self.yaw.to_radians().sin() * self.pitch.to_radians().cos(),
        )
        .normalize()
    }

    pub fn calculate_projection(&self, aspect_ratio: f32) -> Mat4 {
        Mat4::perspective_rh(45.0f32.to_radians(), aspect_ratio, 0.1, 1000.0)
    }

    pub fn calculate_view(&self) -> Mat4 {
        Mat4::look_to_rh(self.eye, self.forward().normalize(), Vec3::Y)
    }

    pub fn handle_keyboard(&mut self, input: &InputState, dt: f32) {
        use egui::Key;

        let forward = self.forward();
        let right = forward.cross(Vec3::Y);

        let mut delta_pos = Vec3::ZERO;

        if input.key_down(Key::W) {
            delta_pos += forward;
        }
        if input.key_down(Key::S) {
            delta_pos -= forward;
        }
        if input.key_down(Key::D) {
            delta_pos += right;
        }
        if input.key_down(Key::A) {
            delta_pos -= right;
        }

        if input.key_down(Key::Space) {
            delta_pos += Vec3::Y;
        }

        if input.modifiers.contains(Modifiers::SHIFT) {
            delta_pos -= Vec3::Y;
        }

        if delta_pos.length() != 0.0 {
            self.moved = true;
        }

        delta_pos = delta_pos.normalize_or_zero();

        let speed = if input.modifiers.contains(Modifiers::CTRL) {
            1.0
        } else {
            5.0
        };

        self.eye += speed * dt * delta_pos;
    }

    pub fn handle_mouse(&mut self, input: &InputState, delta: (f64, f64)) {
        if input.pointer.primary_down() {
            let mouse_sensitivity = 0.1;

            let dx = delta.0 as f32 * mouse_sensitivity;
            let dy = delta.1 as f32 * mouse_sensitivity;

            self.yaw += dx;
            self.pitch -= dy;

            self.pitch = self.pitch.clamp(-89.0, 89.0);

            if dx != 0.0 || dy != 0.0 {
                self.moved = true;
            }
        }
    }
}

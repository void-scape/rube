use crate::{ray::Ray, tree::VoxelTree};
use glam::{Mat4, Vec2, Vec3};
use rube_platform::winit::{event::ElementState, keyboard::KeyCode};
use std::f32::consts::FRAC_PI_2;

#[derive(Debug, Default)]
pub struct Camera {
    pub translation: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub fov: f32,
    pub znear: f32,
    pub zfar: f32,
    pub speed: f32,
    pub half_speed: bool,
    pub left: bool,
    pub right: bool,
    pub forward: bool,
    pub back: bool,
    pub up: bool,
    pub down: bool,
    pub disabled: bool,
    pub ortho: bool,
    pub exp_decay_translation: Vec3,
    pub exp_decay_rotations: Vec2,
    pub flying: bool,
}

impl Camera {
    pub fn handle_key(&mut self, key: KeyCode, state: ElementState) {
        match key {
            KeyCode::ControlLeft => {
                self.half_speed = !state.is_pressed();
            }
            KeyCode::KeyE if state.is_pressed() => {
                self.disabled = !self.disabled;
            }
            KeyCode::KeyO if state.is_pressed() => {
                self.ortho = !self.ortho;
            }
            KeyCode::KeyF if state.is_pressed() => {
                self.flying = !self.flying;
            }
            KeyCode::KeyA => {
                self.left = state.is_pressed();
            }
            KeyCode::KeyD => {
                self.right = state.is_pressed();
            }
            KeyCode::KeyW => {
                self.forward = state.is_pressed();
            }
            KeyCode::KeyS => {
                self.back = state.is_pressed();
            }
            KeyCode::Space => {
                self.up = state.is_pressed();
            }
            KeyCode::ShiftLeft => {
                self.down = state.is_pressed();
            }
            _ => {}
        }
    }

    pub fn handle_mouse(&mut self, dx: f32, dy: f32) {
        if !self.disabled {
            let sensitivity = 0.005;
            let mut frame_dt = Vec2::ZERO;
            frame_dt.x = dx * sensitivity;
            frame_dt.y = -dy * sensitivity;
            self.exp_decay_rotations = self.exp_decay_rotations * 0.8 + frame_dt * 0.2;
        }
    }

    pub fn update(&mut self, tree: &VoxelTree, dt: f32) {
        if !self.disabled {
            self.exp_decay_rotations *= 0.8;
            self.yaw += self.exp_decay_rotations.x;
            self.pitch += self.exp_decay_rotations.y;
            self.pitch = self.pitch.clamp(-FRAC_PI_2 + 0.001, FRAC_PI_2 - 0.001);

            let (yaw_sin, yaw_cos) = self.yaw.sin_cos();
            let forward = Vec3::new(yaw_cos, 0.0, yaw_sin).normalize();
            let right = Vec3::new(-yaw_sin, 0.0, yaw_cos).normalize();
            let mut dxz = Vec3::ZERO;
            dxz += forward * (self.forward as u32 as f32 - self.back as u32 as f32);
            dxz += right * (self.right as u32 as f32 - self.left as u32 as f32);
            let speed = if self.half_speed {
                self.speed / 8.0
            } else {
                self.speed
            };
            let mut frame_dt = Vec3::ZERO;
            frame_dt += dxz.normalize_or_zero() * speed * dt;

            if self.flying {
                if self.down {
                    frame_dt.y = speed * dt;
                }
                if self.up {
                    frame_dt.y = speed * dt;
                }
                self.exp_decay_translation = self.exp_decay_translation * 0.8 + frame_dt * 0.2;
                self.translation += self.exp_decay_translation;
            } else {
                let new_translation = self.translation + self.exp_decay_translation;

                let height = 0.004;
                let hit =
                    Ray::new(new_translation + Vec3::Y * height / 4.0, Vec3::NEG_Y).cast(tree);
                if hit.position != Vec3::ZERO {
                    frame_dt.y = ((hit.position.y + height) - new_translation.y) * 10.0 * dt;
                    self.exp_decay_translation = self.exp_decay_translation * 0.8 + frame_dt * 0.2;
                    self.translation += self.exp_decay_translation;
                }
            }
        }
    }

    pub fn view_matrix(&self) -> Mat4 {
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        Mat4::look_to_rh(
            self.translation,
            Vec3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize_or(Vec3::Z),
            Vec3::Y,
        )
    }

    pub fn projection_matrix(&self, width: usize, height: usize) -> Mat4 {
        let scale = 1.2;
        if self.ortho {
            Mat4::orthographic_rh(
                width as f32 / -scale,
                width as f32 / scale,
                height as f32 / -scale,
                height as f32 / scale,
                self.znear,
                self.zfar,
            )
        } else {
            Mat4::perspective_rh(
                self.fov,
                width as f32 / height as f32,
                self.znear,
                self.zfar,
            )
        }
    }
}

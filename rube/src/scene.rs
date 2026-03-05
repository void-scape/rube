use crate::{
    camera::Camera,
    indirect::{DirectionalLight, SKY_COLOR},
    tree::VoxelTree,
};
use glam::Vec3;
use std::path::Path;

pub struct Scene {
    pub camera: Camera,
    pub tree: VoxelTree,
    pub light: DirectionalLight,
}

impl Scene {
    pub fn from_tree<P: AsRef<Path>>(path: P) -> Self {
        Self {
            tree: VoxelTree::decompress(&std::fs::read(path).unwrap()),
            camera: Camera {
                translation: Vec3::new(1.383996, 1.0355718, 1.1922992),
                yaw: 9.500028,
                pitch: 0.039998103,
                fov: 90f32.to_radians(),
                znear: 0.01,
                zfar: 1000.0,
                speed: 0.1,
                half_speed: true,
                disabled: false,
                flying: false,
                ..Default::default()
            },
            light: DirectionalLight {
                direction: Vec3::new(0.3, 1.0, 0.3).normalize(),
                color: SKY_COLOR,
                intensity: 0.05,
            },
        }
    }

    pub fn castle() -> Self {
        Self {
            tree: VoxelTree::decompress(include_bytes!("../../assets/castle.bin.bz2")),
            camera: Camera {
                translation: Vec3::new(1.2385558, 1.0833066, 1.054556),
                yaw: 8.175014,
                pitch: -0.56000096,
                fov: 90f32.to_radians(),
                znear: 0.01,
                zfar: 1000.0,
                speed: 0.5,
                half_speed: true,
                disabled: true,
                flying: true,
                ..Default::default()
            },
            light: DirectionalLight {
                direction: Vec3::new(0.3, 1.0, 0.3).normalize(),
                color: SKY_COLOR,
                intensity: 0.05,
            },
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.camera.update(&self.tree, dt);
    }
}

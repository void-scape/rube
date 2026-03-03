use crate::{camera::Camera, tree::VoxelTree};
use glam::Vec3;
use std::path::Path;

pub struct Scene {
    pub camera: Camera,
    pub tree: VoxelTree,
}

impl Scene {
    pub fn from_tree<P: AsRef<Path>>(path: P) -> Self {
        Self {
            tree: VoxelTree::decompress(&std::fs::read(path).unwrap()),
            camera: Camera {
                translation: Vec3::new(1.1192523, 1.0224879, 1.0697857),
                yaw: 7.3650107,
                pitch: 0.20999885,
                fov: 90f32.to_radians(),
                znear: 0.01,
                zfar: 1000.0,
                speed: 0.5,
                half_speed: true,
                disabled: true,
                ..Default::default()
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
                ..Default::default()
            },
        }
    }
}

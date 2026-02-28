use ahash::{HashMap, HashMapExt};
use glam::IVec3;

pub extern crate ahash;
pub mod tree;

#[derive(Debug)]
pub struct VoxelMap {
    pub chunks: HashMap<IVec3, Brick>,
    pub palette: [[f32; 4]; 256],
}

impl Default for VoxelMap {
    fn default() -> Self {
        Self {
            chunks: HashMap::new(),
            palette: [[1.0; 4]; 256],
        }
    }
}

impl VoxelMap {
    pub fn brick(&self, voxel_pos: IVec3) -> Option<&Brick> {
        let brick_pos = voxel_pos >> 3;
        self.chunks.get(&brick_pos)
    }

    pub fn has_bricks_in_region(&self, pos: IVec3, range: u32) -> bool {
        if range == 0 {
            return false;
        }
        let min: IVec3 = pos >> 3;
        let max: IVec3 = (pos + IVec3::splat(range as i32 - 1)) >> 3;
        let diff = (max - min + IVec3::ONE).max(IVec3::ZERO);
        let volume = (diff.x as usize) * (diff.y as usize) * (diff.z as usize);
        if self.chunks.len() < volume {
            return self.chunks.keys().any(|k| {
                k.x >= min.x
                    && k.x <= max.x
                    && k.y >= min.y
                    && k.y <= max.y
                    && k.z >= min.z
                    && k.z <= max.z
            });
        }
        for x in min.x..=max.x {
            for y in min.y..=max.y {
                for z in min.z..=max.z {
                    if self.chunks.contains_key(&IVec3::new(x, y, z)) {
                        return true;
                    }
                }
            }
        }

        false
    }

    pub fn shift_to_positive(&mut self) {
        if self.chunks.is_empty() {
            return;
        }
        let min = self.chunks.keys().fold(IVec3::MAX, |acc, &k| acc.min(k));
        let offset = IVec3::new(
            if min.x < 0 { -min.x } else { 0 },
            if min.y < 0 { -min.y } else { 0 },
            if min.z < 0 { -min.z } else { 0 },
        );
        if offset == IVec3::ZERO {
            return;
        }
        self.chunks = self.chunks.drain().map(|(k, v)| (k + offset, v)).collect();
    }
}

#[derive(Debug, Clone)]
pub struct Brick {
    pub data: [u8; 512],
}

impl Default for Brick {
    fn default() -> Self {
        Self { data: [0; 512] }
    }
}

impl Brick {
    pub fn voxel_index(pos: IVec3) -> usize {
        (pos.x + (pos.z * 8) + (pos.y * 64)) as usize
    }
}

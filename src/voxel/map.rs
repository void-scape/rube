use glam::IVec3;
use std::collections::HashMap;

#[derive(Debug)]
pub struct VoxelMap {
    pub chunks: HashMap<IVec3, Brick>,
    pub palette: [u32; 256],
}

impl VoxelMap {
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
            palette: [u32::MAX; 256],
        }
    }

    pub fn brick(&self, voxel_pos: IVec3) -> Option<&Brick> {
        let brick_pos = voxel_pos >> 3;
        self.chunks.get(&brick_pos)
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

use glam::UVec3;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct VoxelMap {
    pub chunks: HashMap<UVec3, Brick>,
}

impl VoxelMap {
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
        }
    }

    pub fn brick(&self, voxel_pos: UVec3) -> Option<&Brick> {
        let brick_pos = voxel_pos >> 3;
        self.chunks.get(&brick_pos)
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
    pub fn voxel_index(pos: UVec3) -> usize {
        (pos.x + (pos.z * 8) + (pos.y * 64)) as usize
    }
}

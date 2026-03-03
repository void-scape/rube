use crate::map::{Brick, VoxelMap};
use fxhash::FxHashMap;
use glam::{IVec3, Vec3};
use std::io::{Read, Write};
use tint::Color;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct VoxelTree {
    pub nodes: Vec<Node>,
    pub leaves: Vec<u8>,
    pub palette: Vec<u32>,
    pub exp: u32,
}

impl VoxelTree {
    pub fn compress(&self) -> Vec<u8> {
        let bytes = postcard::to_allocvec(self).unwrap();
        let mut encoder = bzip2::write::BzEncoder::new(Vec::new(), bzip2::Compression::best());
        encoder.write_all(&bytes).unwrap();
        encoder.finish().unwrap()
    }

    pub fn decompress(bytes: &[u8]) -> Self {
        let mut encoder = bzip2::read::BzDecoder::new(bytes);
        let mut decompressed = Vec::with_capacity(bytes.len());
        encoder.read_to_end(&mut decompressed).unwrap();
        postcard::from_bytes(&decompressed).unwrap()
    }

    pub fn packed_srgb(&self, material_id: usize) -> u32 {
        self.palette[material_id]
    }

    pub fn pack_srgb(&self, srgb: tint::Srgb) -> u32 {
        ((srgb.r() as u32) << 16) | ((srgb.g() as u32) << 8) | (srgb.b() as u32)
    }

    pub fn pack_linear_rgb(&self, linear: Vec3) -> u32 {
        self.pack_srgb(tint::LinearRgb::from_rgb(linear.x, linear.y, linear.z).to_srgb())
    }

    pub fn unpack_srgb(&self, srgb: u32) -> tint::Srgb {
        tint::Srgb::from_rgb(
            ((srgb >> 16) & 0xff) as u8,
            ((srgb >> 8) & 0xff) as u8,
            (srgb & 0xff) as u8,
        )
    }

    pub fn srgb(&self, material_id: usize) -> tint::Srgb {
        let srgb = self.packed_srgb(material_id);
        self.unpack_srgb(srgb)
    }

    pub fn linear_rgb(&self, material_id: usize) -> Vec3 {
        let linear = self.srgb(material_id).to_linear();
        Vec3::new(linear.r(), linear.g(), linear.b())
    }
}

#[repr(C)]
#[derive(Default, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Node {
    // [     31      |    1    ]
    // [ child_index | is_leaf ]
    // is_leaf     // Indicates if this node is a leaf containing plain voxels.
    // child_index // Absolute offset to array of existing child nodes/voxels.
    child_index_is_leaf: u32,
    pub mask: u64,
}

impl Node {
    pub fn is_leaf(&self) -> bool {
        (self.child_index_is_leaf & 1) == 1
    }

    pub fn child_index(&self) -> usize {
        (self.child_index_is_leaf >> 1) as usize
    }
}

pub fn generate_tree(
    map: &VoxelMap,
    _node_hash: &mut FxHashMap<Vec<u8>, u32>,
    nodes: &mut Vec<Node>,
    leaves: &mut Vec<u8>,
    mut scale: u32,
    pos: IVec3,
    _saved_bytes: &mut usize,
) -> Node {
    debug_assert!(
        scale.is_multiple_of(2),
        "the tree is descended in increments of 2"
    );
    debug_assert_eq!((pos.x | pos.y | pos.z) % 4, 0);

    // Create leaf
    if scale == 2 {
        match map.brick(pos) {
            Some(brick) => {
                // Repack voxels into 4x4x4 tile
                // Cells are indexed by `x + z*4 + y*16`
                let mut temp = [0u8; 64];
                for i in (0..64).step_by(4) {
                    let offset = Brick::voxel_index(
                        IVec3::new(pos.x, pos.y + ((i >> 4) & 3), pos.z + ((i >> 2) & 3)) & 7,
                    );
                    temp[i as usize..i as usize + 4]
                        .copy_from_slice(&brick.data[offset..offset + 4]);
                }
                let mut mask = 0u64;
                let mut active_leaves = Vec::with_capacity(64);
                // generate bitmask of `temp[i] != 0`.
                for (i, &data) in temp.iter().enumerate() {
                    if data != 0 {
                        mask |= 1 << i;
                        active_leaves.push(data);
                    }
                }
                let leaf_index = leaves.len() as u32;
                leaves.extend_from_slice(&active_leaves);
                // TODO: This saves 10 MB on the castle scene! It just causes leaf_id
                // in the shaders to not be unique...
                // let leaf_index = if let Some(&existing_index) = node_hash.get(&active_leaves) {
                //     *saved_bytes += size_of_val(active_leaves.as_slice());
                //     existing_index
                // } else {
                //     let new_index = leaves.len() as u32;
                //     leaves.extend_from_slice(&active_leaves);
                //     node_hash.insert(active_leaves, new_index);
                //     new_index
                // };
                Node {
                    mask,
                    child_index_is_leaf: (leaf_index << 1) | 1,
                }
            }
            None => Node::default(),
        }
    } else {
        let region_size = 1 << scale;
        if !map.has_bricks_in_region(pos, region_size) {
            return Node::default();
        }

        // Descend
        scale -= 2;
        let mut children = [Node::default(); 64];
        let mut children_len = 0;
        let mut mask = 0u64;
        for i in 0..64 {
            let child_pos = IVec3::new(i & 3, (i >> 4) & 3, (i >> 2) & 3);
            let child = generate_tree(
                map,
                _node_hash,
                nodes,
                leaves,
                scale,
                pos + (child_pos << scale),
                _saved_bytes,
            );
            // Node contains voxel/children data
            if child.mask != 0 {
                mask |= 1 << i;
                children[children_len] = child;
                children_len += 1
            }
        }
        let len = nodes.len() as u32;
        nodes.extend(&children[..children_len]);
        Node {
            mask,
            child_index_is_leaf: len << 1,
        }
    }
}

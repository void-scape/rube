// Sparse-64 voxel tree ray marcher implementation adapted from:
// https://dubiousconst282.github.io/2024/10/03/voxel-ray-tracing/

use crate::tree::VoxelTree;
use glam::{IVec3, UVec3, Vec3};

#[derive(Default, Clone, Copy)]
pub struct PackedHitInfo {
    leaf_index_and_normal_and_escaped: u32,
    pub position: Vec3,
    // TODO: This needs to be better integrated. There is no point in storing leaf index
    // if the color data is already here.
    pub mip_map: u32,
    pub reads: u32,
}

impl PackedHitInfo {
    pub fn leaf_index(&self) -> usize {
        (self.leaf_index_and_normal_and_escaped >> 4) as usize
    }

    pub fn normal_index(&self) -> usize {
        ((self.leaf_index_and_normal_and_escaped >> 1) & 7) as usize
    }

    pub fn normal(&self) -> Vec3 {
        match self.normal_index() {
            0 => Vec3::new(1.0, 0.0, 0.0),
            1 => Vec3::new(-1.0, 0.0, 0.0),
            2 => Vec3::new(0.0, 1.0, 0.0),
            3 => Vec3::new(0.0, -1.0, 0.0),
            4 => Vec3::new(0.0, 0.0, 1.0),
            5 => Vec3::new(0.0, 0.0, -1.0),
            _ => unreachable!(),
        }
    }

    pub fn escaped(&self) -> bool {
        (self.leaf_index_and_normal_and_escaped & 1) == 1
    }
}

#[derive(Clone, Copy)]
pub struct Ray {
    origin: Vec3,
    direction: Vec3,
    lod: bool,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction,
            lod: false,
        }
    }

    pub fn lod(mut self) -> Self {
        self.lod = true;
        self
    }

    pub fn cast(self, tree: &VoxelTree) -> PackedHitInfo {
        cast_ray(tree, self)
    }
}

fn cast_ray(tree: &VoxelTree, mut ray: Ray) -> PackedHitInfo {
    let mut hit = PackedHitInfo {
        leaf_index_and_normal_and_escaped: 1,
        ..Default::default()
    };

    // Perform aabb intersection check before descending tree to prevent rays from
    // starting outside of the 1..2 bounding volume. Rays can only traverse in this
    // range so this check is required to support arbitrary camera positioning in
    // the world.
    let bbox_min = Vec3::splat(1.0);
    let bbox_max = Vec3::splat(2.0);
    let t0 = (bbox_min - ray.origin) / ray.direction;
    let t1 = (bbox_max - ray.origin) / ray.direction;
    let tmin = t0.min(t1);
    let tmax = t0.max(t1);
    let tnear = tmin.max_element();
    let tfar = tmax.min_element();

    if tnear > tfar || tfar < 0.0 {
        return hit;
    }

    if tnear > 0.0 {
        ray.origin += ray.direction * tnear;
    }

    let mut scale_exp = 21;
    let mut node_index = 0;
    let mut node = tree.nodes[node_index];
    hit.reads += 1;

    // Mirror coordinates to negative ray octant to simplify cell intersections
    let mut mirror_mask = 0;
    if ray.direction.x > 0.0 {
        mirror_mask |= 3;
    }
    if ray.direction.y > 0.0 {
        mirror_mask |= 3 << 4;
    }
    if ray.direction.z > 0.0 {
        mirror_mask |= 3 << 2;
    }

    ray.origin = mirrored_pos(ray.origin, ray.direction, true);
    // Clamp to prevent traversal from completely breaking for rays starting outside tree
    let mut pos = ray.origin.clamp(Vec3::splat(1.0), Vec3::splat(1.9999999));
    let inv_dir = 1.0 / -ray.direction.abs();

    let initial_ray_d = tnear.max(0.0);

    let mut gs_stack = [0; 11];

    let mut side_dist = Vec3::ZERO;
    for _ in 0..256 {
        let mut child_index = node_cell_index(pos, scale_exp) ^ mirror_mask;
        // Descend
        while bit(node.mask, child_index) && !node.is_leaf() {
            gs_stack[scale_exp >> 1] = node_index;
            node_index = node.child_index() + popcnt(node.mask, child_index);

            if ray.lod {
                // mipmap early exit check with a ray cone
                let t = initial_ray_d + (pos - ray.origin).length();
                // let factor = 0.008;
                let factor = 0.10;
                let cell_size = f32::from_bits((scale_exp as u32 + 127 - 23) << 23);
                let diff = t * factor - cell_size;
                if diff.is_sign_positive() {
                    let child_node = tree.nodes[node_index];
                    let linear_mip_map = VoxelTree::unpack_srgb_linear(node.mip_map);
                    let linear_child_mip_map = VoxelTree::unpack_srgb_linear(child_node.mip_map);
                    let linear_mip_map = linear_child_mip_map
                        .lerp(linear_mip_map, (diff / cell_size).clamp(0.0, 1.0));

                    hit.position = pos;
                    hit.mip_map = VoxelTree::pack_linear_rgb(linear_mip_map);
                    hit.leaf_index_and_normal_and_escaped = 0;
                    return hit;
                }
            }

            node = tree.nodes[node_index];
            hit.reads += 1;
            scale_exp -= 2;
            child_index = node_cell_index(pos, scale_exp) ^ mirror_mask;
        }

        if bit(node.mask, child_index) && node.is_leaf() {
            break;
        }

        let mut adv_scale_ecp = scale_exp;
        // wtf
        if ((node.mask >> (child_index & 42)) & 0x00330033) == 0 {
            adv_scale_ecp += 1;
        }

        // Compute next pos by intersecting with max cell sides
        let cell_min = floor_scale(pos, adv_scale_ecp);

        side_dist = (cell_min - ray.origin) * inv_dir;
        let tmax = side_dist.min_element();

        let f = IVec3::splat((1 << adv_scale_ecp) - 1);
        let t = IVec3::splat(-1);
        let mask = side_dist.cmpeq(Vec3::splat(tmax));
        let offset = IVec3::select(mask, t, f);
        let neighbor_max = IVec3::new(
            cell_min.x.to_bits() as i32,
            cell_min.y.to_bits() as i32,
            cell_min.z.to_bits() as i32,
        ) + offset;
        pos = (ray.origin - ray.direction.abs() * tmax).min(Vec3::new(
            f32::from_bits(neighbor_max.x as u32),
            f32::from_bits(neighbor_max.y as u32),
            f32::from_bits(neighbor_max.z as u32),
        ));

        // Find common ancestor based on left-most carry bit
        // We only care about changes in the exponent and high bits of
        // each cell position (10'10'10'...), so the odd bits are masked.
        let diff_pos = UVec3::new(pos.x.to_bits(), pos.y.to_bits(), pos.z.to_bits())
            ^ UVec3::new(
                cell_min.x.to_bits(),
                cell_min.y.to_bits(),
                cell_min.z.to_bits(),
            );
        let combined = (diff_pos.x | diff_pos.y | diff_pos.z) & 0xFFAAAAAA;
        let diff_exp: i32 = if combined == 0 {
            -1
        } else {
            31 - combined.leading_zeros() as i32
        };

        if diff_exp > scale_exp as i32 {
            // NOTE: scale_exp can never be negative
            scale_exp = diff_exp as usize;
            if diff_exp > 21 {
                break;
            }

            node_index = gs_stack[scale_exp >> 1];
            node = tree.nodes[node_index];
            hit.reads += 1;
        }
    }

    if node.is_leaf() && scale_exp <= 21 {
        pos = mirrored_pos(pos, ray.direction, false);
        let child_index = node_cell_index(pos, scale_exp);

        let leaf_index = node.child_index() + popcnt(node.mask, child_index);
        // hit.material_id = tree.leaves[leaf_index];
        hit.reads += 1;
        hit.leaf_index_and_normal_and_escaped |= (leaf_index as u32) << 4;
        hit.leaf_index_and_normal_and_escaped &= !1;
        // hit.leaf_index = leaf_index;
        // hit.escaped = false;
        hit.position = pos;

        // NOTE: This is currently hard coded in the lighting code so this must be valid here.
        assert_eq!(scale_exp, 11);

        let tmax = side_dist.min_element();
        let normal = if side_dist.x == tmax {
            Vec3::new(-ray.direction.x.signum(), 0.0, 0.0)
        } else if side_dist.y == tmax {
            Vec3::new(0.0, -ray.direction.y.signum(), 0.0)
        } else {
            Vec3::new(0.0, 0.0, -ray.direction.z.signum())
        };
        let normal_id = match normal.to_array() {
            [1.0, 0.0, 0.0] => 0,
            [-1.0, 0.0, 0.0] => 1,
            [0.0, 1.0, 0.0] => 2,
            [0.0, -1.0, 0.0] => 3,
            [0.0, 0.0, 1.0] => 4,
            [0.0, 0.0, -1.0] => 5,
            _ => unreachable!("{normal}"),
        };
        hit.leaf_index_and_normal_and_escaped |= normal_id << 1;
        // hit.normal = normal;
    }
    hit
}

fn bit(v: u64, i: usize) -> bool {
    ((v >> i) & 1) == 1
}

fn node_cell_index(pos: Vec3, scale_exp: usize) -> usize {
    let cell = UVec3::new(
        pos.x.to_bits() >> scale_exp,
        pos.y.to_bits() >> scale_exp,
        pos.z.to_bits() >> scale_exp,
    ) & UVec3::splat(3);
    (cell.x + (cell.z * 4) + (cell.y * 16)) as usize
}

// floor(pos / scale) * scale
pub fn floor_scale(pos: Vec3, scale_exp: usize) -> Vec3 {
    let mask = 0xFFFFFFFF << scale_exp;
    Vec3::new(
        f32::from_bits(pos.x.to_bits() & mask),
        f32::from_bits(pos.y.to_bits() & mask),
        f32::from_bits(pos.z.to_bits() & mask),
    )
}

// Reverses `pos` from range [1.0, 2.0) to (2.0, 1.0] if `dir > 0`.
fn mirrored_pos(pos: Vec3, dir: Vec3, range_check: bool) -> Vec3 {
    let mut mirrored = Vec3::new(
        f32::from_bits(pos.x.to_bits() ^ 0x7FFFFF),
        f32::from_bits(pos.y.to_bits() ^ 0x7FFFFF),
        f32::from_bits(pos.z.to_bits() ^ 0x7FFFFF),
    );
    // XOR-ing will only work for coords in range [1.0, 2.0),
    // fallback to subtractions if that's not the case.
    if range_check && (pos.cmplt(Vec3::splat(1.0)) | pos.cmpge(Vec3::splat(2.0))).any() {
        mirrored = Vec3::splat(3.0) - pos;
    }
    Vec3::select(dir.cmpgt(Vec3::ZERO), mirrored, pos)
}

// Count number of set bits in variable range [0..i].
fn popcnt(v: u64, i: usize) -> usize {
    (v & ((1 << i) - 1)).count_ones() as usize
}

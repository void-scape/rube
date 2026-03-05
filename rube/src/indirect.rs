// Monte carlo indirect diffuse global illumination implementation adapted from:
// https://www.scratchapixel.com/lessons/3d-basic-rendering/global-illumination-path-tracing/global-illumination-path-tracing-practical-implementation.html

use crate::{
    march::MarchPass,
    ray::{PackedHitInfo, Ray},
    scene::Scene,
    tree::VoxelTree,
};
use fxhash::FxHashMap;
use glam::Vec3;
use rand_core::{Rng, SeedableRng};
// use rayon::iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};
use std::f32::consts::TAU;

// pub const SKY_COLOR: Vec3 = Vec3::ZERO;
pub const SKY_COLOR: Vec3 = Vec3::new(0.246, 0.624, 0.838);

pub struct IndirectPass {
    last_visible_voxels: FxHashMap<usize, VoxelData>,
    visible_voxels: FxHashMap<usize, VoxelData>,
    color_buffer: Vec<PackedColorData>,
    frame: u32,
}

impl IndirectPass {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            last_visible_voxels: FxHashMap::default(),
            visible_voxels: FxHashMap::default(),
            color_buffer: vec![PackedColorData::default(); width * height],
            frame: 0,
        }
    }
}

#[derive(Clone, Copy)]
struct PackedColorData {
    leaf_index_and_escape: u32,
    color: Vec3,
}

impl Default for PackedColorData {
    fn default() -> Self {
        Self {
            leaf_index_and_escape: 1,
            color: Vec3::ZERO,
        }
    }
}

#[derive(Default)]
struct VoxelData {
    color: Vec3,
    accumulator: Vec3,
    center: Vec3,
    occluded: bool,
    samples: u32,
    frame: u16,
}

pub struct DirectionalLight {
    /// normalized, points towards the light
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
}

#[profiling::function]
pub fn indirect_pass(
    scene: &Scene,
    march_pass: &MarchPass,
    _indirect_pass: &mut IndirectPass,
    pixels: &mut [u32],
) {
    // let tree = &scene.tree;
    // let light = &scene.light;

    // indirect_pass.frame = indirect_pass.frame.wrapping_add(1);
    // indirect_pass.color_buffer.fill(Default::default());
    // std::mem::swap(
    //     &mut indirect_pass.visible_voxels,
    //     &mut indirect_pass.last_visible_voxels,
    // );

    // // NOTE: currently hardcoded based on the voxelization code
    // let scale_exp = 11;
    // let size_bits = 1u32 << scale_exp;
    // let size = f32::from_bits(0x3f800000 | size_bits) - 1.0;
    // let half_size = size * 0.5;
    //
    // {
    //     profiling::scope!("generate leaf map");
    //
    //     indirect_pass.visible_voxels.clear();
    //     for hit in march_pass.hits.iter().filter(|h| !h.escaped()) {
    //         indirect_pass
    //             .visible_voxels
    //             .entry(hit.leaf_index())
    //             .or_insert_with(|| VoxelData {
    //                 color: Vec3::ZERO,
    //                 accumulator: Vec3::ZERO,
    //                 occluded: false,
    //                 samples: 0,
    //                 frame: 1,
    //                 center: {
    //                     let cell_min = crate::ray::floor_scale(hit.position, scale_exp);
    //                     cell_min + Vec3::splat(half_size)
    //                 },
    //             });
    //     }
    // }

    // {
    //     profiling::scope!("shadow occlusion");
    //
    //     let visible_faces: [bool; 6] = [
    //         light.direction.x > 0.0,
    //         light.direction.x < 0.0,
    //         light.direction.y > 0.0,
    //         light.direction.y < 0.0,
    //         light.direction.z > 0.0,
    //         light.direction.z < 0.0,
    //     ];
    //     let normals = [
    //         Vec3::new(1.0, 0.0, 0.0),
    //         Vec3::new(-1.0, 0.0, 0.0),
    //         Vec3::new(0.0, 1.0, 0.0),
    //         Vec3::new(0.0, -1.0, 0.0),
    //         Vec3::new(0.0, 0.0, 1.0),
    //         Vec3::new(0.0, 0.0, -1.0),
    //     ];
    //
    //     indirect_pass
    //         .visible_voxels
    //         .par_iter_mut()
    //         .for_each(|(_, d)| {
    //             let mut occluded = true;
    //             for i in (0..6).filter(|i| visible_faces[*i]) {
    //                 let origin = d.center + normals[i] * (half_size + 1e-4);
    //                 if cast_ray(
    //                     tree,
    //                     Ray {
    //                         direction: light.direction,
    //                         origin,
    //                     },
    //                 )
    //                 .escaped()
    //                 {
    //                     occluded = false;
    //                     break;
    //                 }
    //             }
    //             d.occluded = occluded;
    //         });
    // }

    // {
    //     profiling::scope!("global illumination");
    //     indirect_pass
    //         .color_buffer
    //         .par_iter_mut()
    //         .zip(&march_pass.hits)
    //         .enumerate()
    //         .filter(|(_, (_, h))| !h.escaped())
    //         .for_each(|(i, (data, hit))| {
    //             data.color =
    //                 voxel_indirect(tree, hit, pcg(i as u32 ^ pcg(indirect_pass.frame)) as u64);
    //             data.leaf_index_and_escape = (hit.leaf_index() as u32) << 1;
    //         });
    // }

    // {
    //     profiling::scope!("accumulate samples");
    //     for data in indirect_pass
    //         .color_buffer
    //         .iter()
    //         .filter(|d| (d.leaf_index_and_escape & 1) == 0)
    //     {
    //         let voxel_data = indirect_pass
    //             .visible_voxels
    //             .get_mut(&((data.leaf_index_and_escape as usize) >> 1))
    //             .unwrap();
    //         voxel_data.accumulator += data.color;
    //         voxel_data.samples += 1;
    //     }
    // }

    // {
    //     profiling::scope!("temporal filter");
    //     for (key, data) in indirect_pass.visible_voxels.iter_mut() {
    //         data.color = data.accumulator / data.samples as f32;
    //         if let Some(last_data) = indirect_pass.last_visible_voxels.get(key) {
    //             let frame = last_data.frame.saturating_add(1);
    //             data.color = last_data.color + (data.color - last_data.color) / frame as f32;
    //             data.frame = frame;
    //         }
    //     }
    // }

    {
        profiling::scope!("write pixels");
        for (pixel, hit) in pixels.iter_mut().zip(march_pass.hits.iter()) {
            if !hit.escaped() {
                let albedo = Vec3::splat(hit.reads as f32) / 200.0;
                // let data = &indirect_pass.visible_voxels[&hit.leaf_index()];
                // let albedo = if hit.mip_map != 0 {
                //     VoxelTree::unpack_srgb_linear(hit.mip_map)
                // } else {
                //     tree.linear_rgb(tree.leaves[hit.leaf_index()] as usize)
                // };
                // let color = if data.occluded {
                //     albedo * 0.2
                // } else {
                let color = albedo;
                // };
                *pixel = VoxelTree::pack_linear_rgb(color);
            } else {
                // *pixel = VoxelTree::pack_linear_rgb(SKY_COLOR);
                *pixel = 0xff000000;
            }
        }
    }
}

fn voxel_indirect(tree: &VoxelTree, hit: &PackedHitInfo, seed: u64) -> Vec3 {
    let mut occlusion = 0.0;
    // compute the shaded point coordinate system using normal N
    let (nt, nb) = normal_coordinate_system(hit.normal());

    let samples = 2;
    let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(seed);
    for _ in 0..samples {
        let r1 = rng.next_u32() as f32 / u32::MAX as f32;
        let r2 = rng.next_u32() as f32 / u32::MAX as f32;
        let sample = uniform_sample_hemipshere(r1, r2);
        // transform the random samples to the shaded point’s local coordinate system
        let local_sample = Vec3::new(
            sample.x * nb.x + sample.y * hit.normal().x + sample.z * nt.x,
            sample.x * nb.y + sample.y * hit.normal().y + sample.z * nt.y,
            sample.x * nb.z + sample.y * hit.normal().z + sample.z * nt.z,
        );
        // Don't forget to apply the cosine law (i.e., multiply by cos(theta) = r1).
        // We should also divide the result by the PDF (1 / (2 * M_PI)), but we can do this after
        let origin = hit.position + hit.normal() * 1e-4;
        let sample_hit = Ray::new(origin, local_sample).cast(tree);
        if !sample_hit.escaped() {
            let dist = hit.position.distance(sample_hit.position);
            let attenuation = (1.0 - (dist / 0.01)).max(0.0);
            occlusion += attenuation * r1;
        }
    }

    let albedo = tree.linear_rgb(tree.leaves[hit.leaf_index()] as usize);
    let ao = 1.0 - (occlusion / samples as f32);
    albedo * ao
}

fn normal_coordinate_system(n: Vec3) -> (Vec3, Vec3) {
    let nt = if n.x.abs() > n.y.abs() {
        Vec3::new(n.z, 0.0, -n.x) / (n.x * n.x + n.z * n.z).sqrt()
    } else {
        Vec3::new(0.0, -n.z, n.y) / (n.y * n.y + n.z * n.z).sqrt()
    };
    let nb = n.cross(nt);
    (nt, nb)
}

fn uniform_sample_hemipshere(r1: f32, r2: f32) -> Vec3 {
    // cos(theta) = r1 = y
    // cos^2(theta) + sin^2(theta) = 1 -> sin(theta) = sqrtf(1 - cos^2(theta))
    let sin_theta = (1.0 - r1 * r1).sqrt();
    let phi = TAU * r2;
    let x = sin_theta * phi.cos();
    let z = sin_theta * phi.sin();
    Vec3::new(x, r1, z)
}

// https://gist.github.com/munrocket/236ed5ba7e409b8bdf1ff6eca5dcdc39
fn pcg(n: u32) -> u32 {
    let mut h = n.wrapping_mul(747796405).wrapping_add(2891336453);
    h = ((h >> ((h >> 28) + 4)) ^ h).wrapping_mul(277803737);
    (h >> 22) ^ h
}

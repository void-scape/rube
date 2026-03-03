// Monte carlo indirect diffuse global illumination implementation adapted from:
// https://www.scratchapixel.com/lessons/3d-basic-rendering/global-illumination-path-tracing/global-illumination-path-tracing-practical-implementation.html

use crate::{
    march::MarchPass,
    ray::{HitInfo, Ray, cast_ray},
    tree::VoxelTree,
};
use ahash::HashMap;
use glam::Vec3;
use rand_core::{Rng, SeedableRng};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};
use std::f32::consts::TAU;

const SKY_COLOR: Vec3 = Vec3::new(0.246, 0.624, 0.838);

pub struct IndirectPass {
    seeder: rand_xorshift::XorShiftRng,
    last_leaf_map: HashMap<usize, VoxelData>,
    leaf_map: HashMap<usize, VoxelData>,
    indirect_color: Vec<ColorData>,
}

impl IndirectPass {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            seeder: rand_xorshift::XorShiftRng::seed_from_u64(0),
            last_leaf_map: HashMap::default(),
            leaf_map: HashMap::default(),
            indirect_color: vec![ColorData::default(); width * height],
        }
    }
}

#[derive(Clone, Copy)]
struct ColorData {
    escaped: bool,
    leaf_index: usize,
    color: Vec3,
    seed: u64,
}

impl Default for ColorData {
    fn default() -> Self {
        Self {
            escaped: true,
            leaf_index: 0,
            color: Vec3::ZERO,
            seed: 0,
        }
    }
}

#[derive(Default)]
struct VoxelData {
    color: Vec3,
    accumulator: Vec3,
    faces_to_occlude: [(bool, HitInfo); 6],
    occluded: bool,
    samples: usize,
    frame: usize,
}

fn normal_to_face(normal: Vec3) -> usize {
    match normal.to_array() {
        [1.0, 0.0, 0.0] => 0,
        [-1.0, 0.0, 0.0] => 1,
        [0.0, 1.0, 0.0] => 2,
        [0.0, -1.0, 0.0] => 3,
        [0.0, 0.0, 1.0] => 4,
        [0.0, 0.0, -1.0] => 5,
        _ => unreachable!("{normal}"),
    }
}

// fn face_to_normal(face: usize) -> Vec3 {
//     match face {
//         0 => Vec3::new(1.0, 0.0, 0.0),
//         1 => Vec3::new(-1.0, 0.0, 0.0),
//         2 => Vec3::new(0.0, 1.0, 0.0),
//         3 => Vec3::new(0.0, -1.0, 0.0),
//         4 => Vec3::new(0.0, 0.0, 1.0),
//         5 => Vec3::new(0.0, 0.0, -1.0),
//         _ => unreachable!(),
//     }
// }

pub struct DirectionalLight {
    // normalized, points towards the light
    direction: Vec3,
    color: Vec3,
    intensity: f32,
}

#[profiling::function]
pub fn indirect_pass(
    tree: &VoxelTree,
    march_pass: &MarchPass,
    indirect_pass: &mut IndirectPass,
    pixels: &mut [u32],
) {
    let light = DirectionalLight {
        direction: Vec3::new(0.3, 1.0, 0.3).normalize(),
        color: SKY_COLOR,
        intensity: 0.05,
    };

    indirect_pass.indirect_color.fill(Default::default());
    std::mem::swap(
        &mut indirect_pass.leaf_map,
        &mut indirect_pass.last_leaf_map,
    );

    {
        profiling::scope!("generate leaf map");
        indirect_pass.leaf_map.clear();
        for hit in march_pass.hits.iter().filter(|h| !h.escaped) {
            let entry = indirect_pass
                .leaf_map
                .entry(hit.leaf_index)
                .or_insert_with(|| VoxelData {
                    color: Vec3::ZERO,
                    accumulator: Vec3::ZERO,
                    faces_to_occlude: [(false, Default::default()); 6],
                    occluded: false,
                    samples: 0,
                    frame: 1,
                });
            entry.faces_to_occlude[normal_to_face(hit.normal)] = (true, *hit);
        }
    }

    {
        profiling::scope!("shadow occlusion");
        // TODO: This is still technically incorrect, since faces that suddenly appear
        // may change the voxel from hidden to visible.
        indirect_pass.leaf_map.par_iter_mut().for_each(|(_, d)| {
            let mut occluded = true;
            for hit in d
                .faces_to_occlude
                .iter()
                .filter_map(|(b, hit)| b.then_some(hit))
            {
                let shadow_origin = hit.center + hit.normal * (hit.half_size + 1e-4);
                let shadow_hit = cast_ray(
                    tree,
                    Ray {
                        origin: shadow_origin,
                        direction: light.direction,
                    },
                );
                if shadow_hit.escaped {
                    occluded = false;
                }
            }
            d.occluded = occluded;
        });
    }

    {
        profiling::scope!("generate monte carlo seeds");
        for data in indirect_pass.indirect_color.iter_mut() {
            data.seed = indirect_pass.seeder.next_u64();
        }
    }

    {
        profiling::scope!("global illumination");
        indirect_pass
            .indirect_color
            .par_iter_mut()
            .zip(&march_pass.hits)
            .for_each(|(data, hit)| {
                if !hit.escaped {
                    data.color = voxel_indirect(tree, hit, data.seed);
                    data.leaf_index = hit.leaf_index;
                    data.escaped = false;
                }
            });
    }

    {
        profiling::scope!("accumulate samples");
        for data in indirect_pass.indirect_color.iter().filter(|d| !d.escaped) {
            let voxel_data = indirect_pass.leaf_map.get_mut(&data.leaf_index).unwrap();
            voxel_data.accumulator += data.color;
            voxel_data.samples += 1;
        }
    }

    {
        profiling::scope!("temporal filter");
        for (key, data) in indirect_pass.leaf_map.iter_mut() {
            data.color = data.accumulator / data.samples as f32;
            if let Some(last_data) = indirect_pass.last_leaf_map.get(key) {
                let frame = last_data.frame + 1;
                data.color = last_data.color + (data.color - last_data.color) / frame as f32;
                data.frame = frame;
            }
        }
    }

    {
        profiling::scope!("write pixels");
        for (pixel, hit) in pixels.iter_mut().zip(march_pass.hits.iter()) {
            if !hit.escaped {
                let data = &indirect_pass.leaf_map[&hit.leaf_index];
                let color = if data.occluded {
                    data.color * 0.2
                } else {
                    data.color + light.color * light.intensity
                };
                *pixel = tree.pack_linear_rgb(color);
            } else {
                *pixel = tree.pack_linear_rgb(SKY_COLOR);
            }
        }
    }
}

fn voxel_indirect(tree: &VoxelTree, hit: &HitInfo, seed: u64) -> Vec3 {
    let mut occlusion = 0.0;
    // compute the shaded point coordinate system using normal N
    let (nt, nb) = normal_coordinate_system(hit.normal);

    let samples = 1;
    let mut rng = rand_xorshift::XorShiftRng::seed_from_u64(seed);
    for _ in 0..samples {
        let r1 = rng.next_u32() as f32 / u32::MAX as f32;
        let r2 = rng.next_u32() as f32 / u32::MAX as f32;
        let sample = uniform_sample_hemipshere(r1, r2);
        // transform the random samples to the shaded point’s local coordinate system
        let local_sample = Vec3::new(
            sample.x * nb.x + sample.y * hit.normal.x + sample.z * nt.x,
            sample.x * nb.y + sample.y * hit.normal.y + sample.z * nt.y,
            sample.x * nb.z + sample.y * hit.normal.z + sample.z * nt.z,
        );
        // Don't forget to apply the cosine law (i.e., multiply by cos(theta) = r1).
        // We should also divide the result by the PDF (1 / (2 * M_PI)), but we can do this after
        let origin = hit.position + hit.normal * 1e-4;
        let sample_hit = cast_ray(
            tree,
            Ray {
                origin,
                direction: local_sample,
            },
        );
        if !sample_hit.escaped {
            let dist = hit.position.distance(sample_hit.position);
            let attenuation = (1.0 - (dist / 0.01)).max(0.0);
            occlusion += attenuation * r1;
        }
    }

    let albedo = tree.linear_rgb(hit.material_id as usize);
    let ao = 1.0 - (occlusion / (samples as f32 * 0.5));
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

// Implementation based on the paper found here:
//
// Dynamic Diffuse Global Illumination with Ray-Traced Irradiance Fields
// https://www.jcgt.org/published/0008/02/01/paper-lowres.pdf

use crate::{
    indirect::SKY_COLOR,
    march::MarchPass,
    ray::{PackedHitInfo, Ray},
    scene::Scene,
    tree::VoxelTree,
};
use glam::{IVec3, Mat3, Vec2, Vec2Swizzles, Vec3, Vec3Swizzles};
use rand::{Rng, RngExt};
use rayon::{
    iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator},
    slice::{ParallelSlice, ParallelSliceMut},
};
use std::f32::consts::{PI, TAU};

pub struct ProbePass {
    probes: Vec<Probe>,
    sample_data: Vec<SampleData>,
    probe_counts: IVec3,
    start_position: Vec3,
    probe_step: f32,
    normal_bias: f32,
}

impl ProbePass {
    pub fn new(tree: &VoxelTree) -> Self {
        let (min, max) = position_range(tree);
        let density = tree.exp * 8;
        let start_range = (density as f32 * min).floor().as_uvec3();
        let end_range = (density as f32 * max).ceil().as_uvec3();
        let mut probes = Vec::new();
        let d = density as f32 - 1.0;
        for z in start_range.z..=end_range.z {
            let zc = z as f32 / d;
            for y in start_range.y..=end_range.y {
                let yc = y as f32 / d;
                for x in start_range.x..=end_range.x {
                    let xc = x as f32 / d;
                    probes.push(Probe {
                        position: Vec3::new(1.0 + xc, 1.0 + yc, 1.0 + zc),
                        irradiance: [Vec3::ZERO; PROBE_IRRADIANCE_PIXELS],
                        moments: [Vec2::ONE; PROBE_IRRADIANCE_PIXELS],
                    })
                }
            }
        }
        Self {
            sample_data: vec![SampleData::default(); SAMPLES * probes.len()],
            probes,
            probe_counts: (end_range - start_range + 1).as_ivec3(),
            start_position: min + Vec3::ONE,
            probe_step: 1.0 / d,
            normal_bias: 1e-5,
        }
    }
}

// TODO: This is definitely a horrible way to do this...
fn position_range(tree: &VoxelTree) -> (Vec3, Vec3) {
    let mut positions = Vec::new();
    walk_tree(
        tree,
        0,
        IVec3::ZERO,
        tree.exp,
        (1 << tree.exp) as f32,
        &mut positions,
    );
    (
        positions.iter().fold(Vec3::MAX, |min, p| p.min(min)),
        positions.iter().fold(Vec3::MIN, |max, p| p.max(max)),
    )
}

fn walk_tree(
    tree: &VoxelTree,
    node_index: usize,
    pos: IVec3,
    scale: u32,
    world_scale: f32,
    positions: &mut Vec<Vec3>,
) {
    let node = &tree.nodes[node_index];
    for i in 0..64u64 {
        if node.mask & (1 << i) == 0 {
            continue;
        }
        let child_local = IVec3::new((i & 3) as i32, ((i >> 4) & 3) as i32, ((i >> 2) & 3) as i32);
        let child_scale = scale - 2;
        let child_pos = pos + (child_local << child_scale as i32);
        positions.push(child_pos.as_vec3() / world_scale);
        if !node.is_leaf() {
            let dense_index = (node.mask & ((1 << i) - 1)).count_ones() as usize;
            walk_tree(
                tree,
                node.child_index() + dense_index,
                child_pos,
                child_scale,
                world_scale,
                positions,
            );
        }
    }
}

const PROBE_IRRADIANCE_SIZE: usize = 16;
const PROBE_IRRADIANCE_PIXELS: usize = PROBE_IRRADIANCE_SIZE * PROBE_IRRADIANCE_SIZE;
// TODO: make this 16
const PROBE_MOMENTS_SIZE: usize = 16;
const PROBE_MOMENTS_PIXELS: usize = PROBE_MOMENTS_SIZE * PROBE_MOMENTS_SIZE;

pub struct Probe {
    position: Vec3,
    irradiance: [Vec3; PROBE_IRRADIANCE_PIXELS],
    moments: [Vec2; PROBE_MOMENTS_PIXELS],
}

// https://www.youtube.com/watch?v=1GqCmW_1BCw
#[track_caller]
fn octahedral_from_unit_vector(v: Vec3) -> Vec2 {
    debug_assert!(v.is_normalized(), "{v}");
    let one_norm = v.abs().element_sum();
    let mut out = (1.0 / one_norm) * v.xy();
    if v.z < 0.0 {
        out = out.signum() * (1.0 - out.abs().yx());
    }
    out
}

// https://www.youtube.com/watch?v=1GqCmW_1BCw
fn unit_vector_from_octahedral(v: Vec2) -> Vec3 {
    let sum = v.abs().element_sum();
    let mut out = v.extend(1.0 - sum);
    if sum > 1.0 {
        out.x = out.x.signum() * (1.0 - out.y.abs());
        out.y = out.y.signum() * (1.0 - out.x.abs());
        out.z = -out.z;
    }
    out.normalize()
}

fn random_orientation(rng: &mut impl Rng) -> Mat3 {
    Mat3::from_axis_angle(
        Vec3::new(
            rng.random_range(-1.0..=1.0),
            rng.random_range(-1.0..=1.0),
            rng.random_range(-1.0..=1.0),
        )
        .normalize_or(Vec3::Y),
        rng.random_range(0.0..TAU),
    )
}

fn spherical_fibonacci(i: f32, n: f32) -> Vec3 {
    let phi = 5f32.sqrt() * 0.5 + 0.5;
    fn madfrac(a: f32, b: f32) -> f32 {
        a * b - (a * b).floor()
    }
    let phi = 2.0 * PI * madfrac(i, phi - 1.0);
    let cos_theta = 1.0 - (2.0 * i + 1.0) * (1.0 / n);
    let sin_theta = (1.0 - cos_theta * cos_theta).clamp(0.0, 1.0).sqrt();
    Vec3::new(phi.cos() * sin_theta, phi.sin() * sin_theta, cos_theta)
}

fn base_grid_coord(probe_pass: &ProbePass, v: Vec3) -> IVec3 {
    ((v - probe_pass.start_position) / probe_pass.probe_step)
        .as_ivec3()
        .clamp(IVec3::ZERO, probe_pass.probe_counts - IVec3::ONE)
}

fn gridCoordToProbeIndex(probe_pass: &ProbePass, v: IVec3) -> usize {
    (v.x + v.y * probe_pass.probe_counts.x
        + v.z * probe_pass.probe_counts.x * probe_pass.probe_counts.y) as usize
}

fn grid_coord_to_position(probe_pass: &ProbePass, c: IVec3) -> Vec3 {
    probe_pass.probe_step * c.as_vec3() + probe_pass.start_position
}

fn sample_probe_irradiance(probe_pass: &ProbePass, uv: Vec2, probe_index: usize) -> Vec3 {
    let probe = &probe_pass.probes[probe_index];
    debug_assert!((-1.0..=1.0).contains(&uv.x));
    debug_assert!((-1.0..=1.0).contains(&uv.y));
    let nuv = ((uv / 2.0 + 0.5) * PROBE_IRRADIANCE_SIZE as f32 - 0.5)
        .clamp(Vec2::ZERO, Vec2::splat(PROBE_IRRADIANCE_SIZE as f32 - 1.0));

    let x0 = nuv.x as usize;
    let y0 = nuv.y as usize;
    let x1 = (x0 + 1).min(PROBE_IRRADIANCE_SIZE - 1);
    let y1 = (y0 + 1).min(PROBE_IRRADIANCE_SIZE - 1);

    let tx = nuv.x - x0 as f32;
    let ty = nuv.y - y0 as f32;

    let c00 = probe.irradiance[y0 * PROBE_IRRADIANCE_SIZE + x0];
    let c10 = probe.irradiance[y0 * PROBE_IRRADIANCE_SIZE + x1];
    let c01 = probe.irradiance[y1 * PROBE_IRRADIANCE_SIZE + x0];
    let c11 = probe.irradiance[y1 * PROBE_IRRADIANCE_SIZE + x1];

    c00 * (1.0 - tx) * (1.0 - ty) + c10 * tx * (1.0 - ty) + c01 * (1.0 - tx) * ty + c11 * tx * ty
}

fn sample_probe_moments(probe_pass: &ProbePass, uv: Vec2, probe_index: usize) -> Vec2 {
    let probe = &probe_pass.probes[probe_index];
    debug_assert!((-1.0..=1.0).contains(&uv.x));
    debug_assert!((-1.0..=1.0).contains(&uv.y));
    let nuv = ((uv / 2.0 + 0.5) * PROBE_MOMENTS_SIZE as f32 - 0.5)
        .clamp(Vec2::ZERO, Vec2::splat(PROBE_MOMENTS_SIZE as f32 - 1.0));

    let x0 = nuv.x as usize;
    let y0 = nuv.y as usize;
    let x1 = (x0 + 1).min(PROBE_MOMENTS_SIZE - 1);
    let y1 = (y0 + 1).min(PROBE_MOMENTS_SIZE - 1);

    let tx = nuv.x - x0 as f32;
    let ty = nuv.y - y0 as f32;

    let c00 = probe.moments[y0 * PROBE_MOMENTS_SIZE + x0];
    let c10 = probe.moments[y0 * PROBE_MOMENTS_SIZE + x1];
    let c01 = probe.moments[y1 * PROBE_MOMENTS_SIZE + x0];
    let c11 = probe.moments[y1 * PROBE_MOMENTS_SIZE + x1];

    c00 * (1.0 - tx) * (1.0 - ty) + c10 * tx * (1.0 - ty) + c01 * (1.0 - tx) * ty + c11 * tx * ty
}

fn sample_probes_for_hit(probe_pass: &ProbePass, origin: Vec3, hit: PackedHitInfo) -> Vec3 {
    if hit.escaped() {
        return Vec3::ZERO;
    }
    let wsn = hit.normal();
    let wsposition = hit.position;
    if origin.distance_squared(wsposition) < 1e-8 {
        return Vec3::ZERO;
    }
    // View vector
    let w_o = (origin - wsposition).normalize();
    let cos_o = wsn.dot(w_o);
    // Incoming reflection vector
    let w_mi = (wsn * (2.0 * cos_o) - w_o).normalize();

    let base_grid_coord = base_grid_coord(probe_pass, wsposition);
    let base_probe_pos = grid_coord_to_position(probe_pass, base_grid_coord);
    let mut sum_irradiance = Vec3::ZERO;
    let mut sum_weight = 0.0;

    // Alpha is how far from the floor(currentVertex) position. on [0, 1] for each axis.
    let alpha =
        ((wsposition - base_probe_pos) / probe_pass.probe_step).clamp(Vec3::ZERO, Vec3::ONE);

    // Iterate over adjacent probe cage
    for i in 0..8 {
        // Compute the offset grid coord and clamp to the probe grid boundary
        // Offset = 0 or 1 along each axis
        let offset = IVec3::new(i, i >> 1, i >> 2) & IVec3::ONE;
        let probe_grid_coord =
            (base_grid_coord + offset).clamp(IVec3::ZERO, probe_pass.probe_counts - 1);
        let p = gridCoordToProbeIndex(probe_pass, probe_grid_coord);

        // Make cosine falloff in tangent plane with respect to the angle from the surface to the probe so that we never
        // test a probe that is *behind* the surface.
        // It doesn't have to be cosine, but that is efficient to compute and we must clip to the tangent plane.
        let probePos = grid_coord_to_position(probe_pass, probe_grid_coord);

        // Bias the position at which visibility is computed; this
        // avoids performing a shadow test *at* a surface, which is a
        // dangerous location because that is exactly the line between
        // shadowed and unshadowed. If the normal bias is too small,
        // there will be light and dark leaks. If it is too large,
        // then samples can pass through thin occluders to the other
        // side (this can only happen if there are MULTIPLE occluders
        // near each other, a wall surface won't pass through itself.)
        let probeToPoint = wsposition - probePos + (wsn + 3.0 * w_o) * probe_pass.normal_bias;
        debug_assert!(
            probeToPoint.is_finite(),
            "{wsposition}, {probePos}, {wsn}, {w_o}, {}",
            probe_pass.normal_bias
        );
        let dir = (-probeToPoint).normalize();

        // Compute the trilinear weights based on the grid cell vertex to smoothly
        // transition between probes. Avoid ever going entirely to zero because that
        // will cause problems at the border probes. This isn't really a lerp.
        // We're using 1-a when offset = 0 and a when offset = 1.
        // Vector3 trilinear = lerp(1.0 - alpha, alpha, offset);
        let offset_f = offset.as_vec3();
        let trilinear = Vec3::new(
            if offset_f.x == 0.0 {
                1.0 - alpha.x
            } else {
                alpha.x
            },
            if offset_f.y == 0.0 {
                1.0 - alpha.y
            } else {
                alpha.y
            },
            if offset_f.z == 0.0 {
                1.0 - alpha.z
            } else {
                alpha.z
            },
        );
        let mut weight = 1.0;

        // Clamp all of the multiplies. We can't let the weight go to zero because then it would be
        // possible for *all* weights to be equally low and get normalized
        // up to 1/n. We want to distinguish between weights that are
        // low because of different factors.

        // Smooth backface test
        {
            // Computed without the biasing applied to the "dir" variable.
            // This test can cause reflection-map looking errors in the image
            // (stuff looks shiny) if the transition is poor.
            let trueDirectionToProbe = (probePos - wsposition).normalize();

            // The naive soft backface weight would ignore a probe when
            // it is behind the surface. That's good for walls. But for small details inside of a
            // room, the normals on the details might rule out all of the probes that have mutual
            // visibility to the point. So, we instead use a "wrap shading" test below inspired by
            // NPR work.
            // weight *= max(0.0001, dot(trueDirectionToProbe, wsN));

            // The small offset at the end reduces the "going to zero" impact
            // where this is really close to exactly opposite
            weight *= (0.0001f32.max((trueDirectionToProbe.dot(wsn) + 1.0) * 0.5)).powi(2) + 0.2;
        }

        // Moment visibility test
        {
            let texCoord = octahedral_from_unit_vector(-dir);
            let distToProbe = probeToPoint.length();

            let temp = sample_probe_moments(probe_pass, texCoord, p);
            let mean = temp.x;
            let variance = (temp.x.powi(2) - temp.y).abs();

            // http://www.punkuser.net/vsm/vsm_paper.pdf; equation 5
            // Need the max in the denominator because biasing can cause a negative displacement
            let mut chebyshevWeight =
                variance / (variance + ((distToProbe - mean).max(0.0)).powi(2));

            // Increase contrast in the weight
            chebyshevWeight = chebyshevWeight.powi(3).max(0.0);

            weight *= if distToProbe <= mean {
                chebyshevWeight
            } else {
                1.0
            };
        }

        // Avoid zero weight
        weight = 1e-5f32.max(weight);

        let irradianceDir = wsn;

        let texCoord = octahedral_from_unit_vector(irradianceDir);

        let probeIrradiance = sample_probe_irradiance(probe_pass, texCoord, p);
        // texture(irradianceFieldSurface.irradianceProbeGridbuffer, texCoord).rgb;

        // A tiny bit of light is really visible due to log perception, so
        // crush tiny weights but keep the curve continuous. This must be done
        // before the trilinear weights, because those should be preserved.
        let crushThreshold = 0.2;
        if weight < crushThreshold {
            weight *= weight * weight * (1.0 / (crushThreshold * crushThreshold));
        }

        // Trilinear weights
        weight *= trilinear.x * trilinear.y * trilinear.z;

        //         // Weight in a more-perceptual brightness space instead of radiance space.
        //         // This softens the transitions between probes with respect to translation.
        //         // It makes little difference most of the time, but when there are radical transitions
        //         // between probes this helps soften the ramp.
        // #       if LINEAR_BLENDING == 0
        //             probeIrradiance = sqrt(probeIrradiance);
        // #       endif

        sum_irradiance += weight * probeIrradiance;
        sum_weight += weight;
    }

    let mut netIrradiance = sum_irradiance / sum_weight;

    //     // Go back to linear irradiance
    // #   if LINEAR_BLENDING == 0
    //         netIrradiance = square(netIrradiance);
    // #   endif
    const energyPreservation: f32 = 0.85;
    netIrradiance *= energyPreservation;

    0.5 * PI * netIrradiance
}

const HYSTERESIS: f32 = 0.98;
const SAMPLES: usize = 32;
#[derive(Default, Clone, Copy)]
struct SampleData {
    ray_dir: Vec3,
    radiance: Vec3,
    distance: f32,
}

#[profiling::function]
pub fn probe_pass(
    scene: &Scene,
    march_pass: &MarchPass,
    probe_pass: &mut ProbePass,
    pixels: &mut [u32],
    width: usize,
    height: usize,
) {
    let tree = &scene.tree;

    let mut rng = rand::rng();
    let random_orientation = random_orientation(&mut rng);

    // generate irradiance data to update the probes
    let mut sample_data = std::mem::take(&mut probe_pass.sample_data);
    sample_data
        .par_chunks_mut(SAMPLES)
        .zip(&probe_pass.probes)
        .for_each(|(data, probe)| {
            for (i, data) in data.iter_mut().enumerate() {
                let ray_dir = random_orientation * spherical_fibonacci(i as f32, SAMPLES as f32);
                let sample_hit = Ray::new(probe.position, ray_dir).cast(tree);
                let distance = if !sample_hit.escaped() {
                    (sample_hit.position - probe.position).length()
                } else {
                    1.0
                };
                let albedo = if !sample_hit.escaped() {
                    tree.linear_rgb(tree.leaves[sample_hit.leaf_index()] as usize)
                } else {
                    SKY_COLOR
                };
                let indirect = if !sample_hit.escaped() {
                    sample_probes_for_hit(probe_pass, probe.position, sample_hit)
                } else {
                    Vec3::ZERO
                };
                data.ray_dir = ray_dir;
                data.radiance = albedo + indirect;
                data.distance = distance;
            }
        });
    _ = std::mem::replace(&mut probe_pass.sample_data, sample_data);

    // update the probes with the irradiance from last frame
    probe_pass
        .sample_data
        .par_chunks(SAMPLES)
        .zip(&mut probe_pass.probes)
        .for_each(|(sample_data, probe)| {
            for v in 0..PROBE_IRRADIANCE_SIZE {
                for u in 0..PROBE_IRRADIANCE_SIZE {
                    let uv = (Vec2::new(u as f32, v as f32) + 0.5) / PROBE_IRRADIANCE_SIZE as f32
                        * 2.0
                        - 1.0;
                    debug_assert!((-1.0..=1.0).contains(&uv.x));
                    debug_assert!((-1.0..=1.0).contains(&uv.y));
                    let texel_dir = unit_vector_from_octahedral(uv);
                    let (new_irradiance, new_moments) =
                        sample_data
                            .iter()
                            .fold((Vec3::ZERO, Vec2::ZERO), |(irr, mom), d| {
                                let weight = 0f32.max(texel_dir.dot(d.ray_dir));
                                let moment_weight = weight.powi(4);
                                (
                                    irr + weight * d.radiance,
                                    mom + moment_weight
                                        * Vec2::new(d.distance, d.distance * d.distance),
                                )
                            });
                    let index = v * PROBE_IRRADIANCE_SIZE + u;
                    probe.irradiance[index] = probe.irradiance[index]
                        .lerp(new_irradiance / SAMPLES as f32, 1.0 - HYSTERESIS);
                    probe.moments[index] =
                        probe.moments[index].lerp(new_moments / SAMPLES as f32, 1.0 - HYSTERESIS);
                }
            }
        });

    // final indirect + direct pass
    pixels
        .par_iter_mut()
        .zip(&march_pass.hits)
        .for_each(|(pixel, hit)| {
            if !hit.escaped() {
                let albedo = tree.linear_rgb(tree.leaves[hit.leaf_index()] as usize);
                let indirect = sample_probes_for_hit(probe_pass, scene.camera.translation, *hit);
                *pixel = VoxelTree::pack_linear_rgb(albedo + indirect);
            } else {
                *pixel = VoxelTree::pack_linear_rgb(SKY_COLOR);
            }
        });

    // debug render probe positions
    // let proj_matrix = scene
    //     .camera
    //     .projection_matrix(width, height)
    //     .mul_mat4(&scene.camera.view_matrix());
    // for probe in probe_pass.probes.iter() {
    //     let screen_position = proj_matrix * probe.position.extend(1.0);
    //     if screen_position.w <= 0.0 {
    //         continue;
    //     }
    //     let screen_position = screen_position.xyz() / screen_position.w;
    //     if !(-1.0..=1.0).contains(&screen_position.x) || !(-1.0..=1.0).contains(&screen_position.y)
    //     {
    //         continue;
    //     }
    //     let px = (((screen_position.x * 0.5 + 0.5) * width as f32) as usize).min(width - 1);
    //     let py = (((-screen_position.y * 0.5 + 0.5) * height as f32) as usize).min(height - 1);
    //     let index = py * width + px;
    //     if screen_position.z > march_pass.depth[index] {
    //         continue;
    //     }
    //     for dy in 0..4 {
    //         for dx in 0..4 {
    //             if py + dy < height && px + dx < width {
    //                 let index = (py + dy) * width + px + dx;
    //                 pixels[index] = u32::MAX;
    //             }
    //         }
    //     }
    // }

    // debug render probe maps
    // for (i, probe) in probe_pass.probes.iter_mut().enumerate() {
    //     let pcx = probe_pass.probe_counts.x as usize;
    //     let pcy = probe_pass.probe_counts.y as usize;
    //     let dz = i / (pcx * pcy);
    //     let dy = i / pcx;
    //     let dx = i % pcx;
    //     if dz < 1 {
    //         for py in 0..PROBE_IRRADIANCE_SIZE {
    //             for px in 0..PROBE_IRRADIANCE_SIZE {
    //                 pixels[(dz * width)
    //                     + (dy * PROBE_IRRADIANCE_SIZE + py) * width
    //                     + px
    //                     + dx * PROBE_IRRADIANCE_SIZE] =
    //                     VoxelTree::pack_linear_rgb(probe.pixels[py * PROBE_IRRADIANCE_SIZE + px]);
    //             }
    //         }
    //
    //         for py in 0..PROBE_IRRADIANCE_SIZE {
    //             for px in 0..PROBE_IRRADIANCE_SIZE {
    //                 pixels[(dz * width)
    //                     + (dy * PROBE_IRRADIANCE_SIZE + py) * width
    //                     + px
    //                     + dx * PROBE_IRRADIANCE_SIZE
    //                     + pcx * PROBE_IRRADIANCE_SIZE] = VoxelTree::pack_linear_rgb(Vec3::splat(
    //                     probe.moments[py * PROBE_IRRADIANCE_SIZE + px].x,
    //                 ));
    //             }
    //         }
    //     }
    // }
}

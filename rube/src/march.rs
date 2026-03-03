use crate::ray::PackedHitInfo;
use crate::ray::{Ray, cast_ray};
use crate::scene::Scene;
use glam::{Mat4, Vec2, Vec3, Vec4Swizzles};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};

pub struct MarchPass {
    pub hits: Vec<PackedHitInfo>,
}

impl MarchPass {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            hits: vec![PackedHitInfo::default(); width * height],
        }
    }
}

#[profiling::function]
pub fn march_pass(scene: &Scene, march_pass: &mut MarchPass, width: usize, height: usize) {
    let inv_proj_matrix = scene
        .camera
        .projection_matrix(width, height)
        .mul_mat4(&scene.camera.view_matrix())
        .inverse();
    march_pass
        .hits
        .par_iter_mut()
        .enumerate()
        .for_each(|(i, pixel)| {
            let py = i / width;
            let px = i % width;
            let ray = primary_ray(
                px,
                py,
                width,
                height,
                &inv_proj_matrix,
                scene.camera.translation,
            );
            *pixel = cast_ray(&scene.tree, ray);
        });
}

// // ghost reads
// if false {
//     use tint::Color;
//     let l = hit.reads as f32 / tree.nodes.len() as f32 * 1000.0;
//     let color = tint::LinearRgb::from_rgb(l, l, l).to_srgb();
//     *pixel =
//         (color.b() as u32) | ((color.g() as u32) << 8) | ((color.r() as u32) << 16);
//     return;
// }

fn primary_ray(
    px: usize,
    py: usize,
    width: usize,
    height: usize,
    inv_proj_matrix: &Mat4,
    origin: Vec3,
) -> Ray {
    let uv = (Vec2::new(px as f32, py as f32) + Vec2::splat(0.5))
        / Vec2::new(width as f32, height as f32);
    let ndc = Vec2::new(uv.x * 2.0 - 1.0, -(uv.y * 2.0 - 1.0));
    let far = inv_proj_matrix * ndc.extend(1.0).extend(1.0);
    Ray {
        direction: (far.xyz() / far.w).normalize(),
        origin,
    }
}

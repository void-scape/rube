use crate::ray::PackedHitInfo;
use crate::ray::Ray;
use crate::scene::Scene;
use glam::{Mat4, Vec2, Vec3, Vec4Swizzles};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};

pub struct MarchPass {
    pub hits: Vec<PackedHitInfo>,
    pub depth: Vec<f32>,
}

impl MarchPass {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            hits: vec![PackedHitInfo::default(); width * height],
            depth: vec![0.0; width * height],
        }
    }
}

#[profiling::function]
pub fn march_pass(scene: &Scene, march_pass: &mut MarchPass, width: usize, height: usize) {
    let proj_matrix = scene
        .camera
        .projection_matrix(width, height)
        .mul_mat4(&scene.camera.view_matrix());
    let inv_proj_matrix = proj_matrix.inverse();
    march_pass
        .hits
        .par_iter_mut()
        .zip(&mut march_pass.depth)
        .enumerate()
        .for_each(|(i, (pixel, depth))| {
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
            // *pixel = ray.lod().cast(&scene.tree);
            *pixel = ray.cast(&scene.tree);
            if pixel.escaped() {
                *depth = 1.0;
            } else {
                let screen_position = proj_matrix * pixel.position.extend(1.0);
                *depth = screen_position.z / screen_position.w;
            }
        });
}

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
    Ray::new(origin, (far.xyz() / far.w).normalize())
}

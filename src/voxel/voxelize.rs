// Stolen from https://github.com/DeadlockCode/voxel_ray_traversal/blob/main/src/voxelize.rs

use crate::voxel::map::{Brick, VoxelMap};
use dot_vox::{Frame, Model, SceneNode};
use glam::{IVec3, Mat3, UVec3, Vec2, Vec3, Vec4};
use std::path::Path;

pub fn vox_to_voxels(path: impl AsRef<Path>) -> VoxelMap {
    fn frame_translation(frames: &[Frame]) -> Option<IVec3> {
        let mut parts = frames.first()?.attributes.get("_t")?.split_whitespace();
        let x = parts.next()?.parse().ok()?;
        let y = parts.next()?.parse().ok()?;
        let z = parts.next()?.parse().ok()?;
        parts.next().is_none().then(|| IVec3::new(x, y, z))
    }

    fn frame_rotation(frames: &[Frame]) -> Option<Mat3> {
        let byte: u8 = frames.first()?.attributes.get("_r")?.trim().parse().ok()?;
        let index0 = (byte & 0b00000011) as usize;
        let index1 = ((byte & 0b00001100) >> 2) as usize;
        let index2 = 3 - index0 - index1;
        let sign0 = if byte & 0b00010000 != 0 { -1 } else { 1 };
        let sign1 = if byte & 0b00100000 != 0 { -1 } else { 1 };
        let sign2 = if byte & 0b01000000 != 0 { -1 } else { 1 };
        let mut cols = [Vec3::ZERO; 3];
        cols[index0][0] = sign0 as f32;
        cols[index1][1] = sign1 as f32;
        cols[index2][2] = sign2 as f32;
        Some(Mat3::from_cols(cols[0], cols[1], cols[2]))
    }

    fn descend_tree(
        map: &mut VoxelMap,
        scene_index: usize,
        scenes: &[SceneNode],
        vox_models: &[Model],
        translation: IVec3,
        rotation: Mat3,
    ) {
        match &scenes[scene_index] {
            dot_vox::SceneNode::Shape { models, .. } => {
                for shape_model in models.iter() {
                    let model = &vox_models[shape_model.model_id as usize];
                    let half = IVec3::new(
                        model.size.x as i32,
                        model.size.y as i32,
                        model.size.z as i32,
                    ) / 2;
                    for voxel in model.voxels.iter() {
                        let local =
                            IVec3::new(voxel.x as i32, voxel.y as i32, voxel.z as i32) - half;
                        let rotated = rotation * local.as_vec3();
                        let mut voxel_pos = rotated.as_ivec3() + translation;
                        std::mem::swap(&mut voxel_pos.y, &mut voxel_pos.z);
                        let brick_pos = voxel_pos >> 3;
                        let brick = map.chunks.entry(brick_pos).or_default();
                        let index = Brick::voxel_index(voxel_pos & 7);
                        brick.data[index] = voxel.i;
                    }
                }
            }
            dot_vox::SceneNode::Transform { frames, child, .. } => {
                let new_translation = if let Some(t) = frame_translation(frames) {
                    (translation.as_vec3() + rotation * t.as_vec3()).as_ivec3()
                } else {
                    translation
                };
                let new_rotation = if let Some(r) = frame_rotation(frames) {
                    rotation * r
                } else {
                    rotation
                };
                descend_tree(
                    map,
                    *child as usize,
                    scenes,
                    vox_models,
                    new_translation,
                    new_rotation,
                );
            }
            dot_vox::SceneNode::Group { children, .. } => {
                for child in children.iter() {
                    descend_tree(
                        map,
                        *child as usize,
                        scenes,
                        vox_models,
                        translation,
                        rotation,
                    );
                }
            }
        }
    }

    println!("Voxelizing...");
    let start = std::time::Instant::now();
    let vox = dot_vox::load(path.as_ref().to_str().unwrap()).unwrap();
    let mut map = VoxelMap::new();
    for (i, color) in vox.palette.iter().enumerate() {
        map.palette[i] = u32::from_le_bytes([color.r, color.g, color.b, color.a]);
    }
    descend_tree(
        &mut map,
        0,
        &vox.scenes,
        &vox.models,
        IVec3::ZERO,
        Mat3::IDENTITY,
    );
    map.shift_to_positive();
    println!("  [{:?}]", start.elapsed());
    map
}

pub fn obj_to_voxels(path: impl AsRef<Path>, resolution: u32) -> VoxelMap {
    println!("Voxelizing @ {resolution}...");
    let start = std::time::Instant::now();
    let mut mesh = parse_obj(path);
    transform_vertices(&mut mesh.vertices, resolution);
    let voxels = voxelize_mesh(&mesh);
    println!("  [{:?}]", start.elapsed());
    voxels
}

fn parse_obj(path: impl AsRef<Path>) -> Mesh {
    let (models, _) = tobj::load_obj(
        path.as_ref(),
        &tobj::LoadOptions {
            single_index: true,
            triangulate: true,
            ..Default::default()
        },
    )
    .expect("Failed to load OBJ file");
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    for model in models {
        let mesh = &model.mesh;
        for v in mesh.positions.chunks_exact(3) {
            vertices.push(Vec3::new(-v[0], v[2], v[1]));
        }
        for idx in mesh.indices.chunks_exact(3) {
            triangles.push([idx[0], idx[1], idx[2]]);
        }
    }
    Mesh {
        vertices,
        triangles,
    }
}

fn transform_vertices(vertices: &mut [Vec3], resolution: u32) {
    let mut min = Vec3::MAX;
    let mut max = Vec3::MIN;
    for vertex in vertices.iter() {
        min = min.min(*vertex);
        max = max.max(*vertex);
    }
    let range = max - min;
    let size = range.x.max(range.y).max(range.z);
    for vertex in vertices.iter_mut() {
        *vertex = ((*vertex - min) / size) * resolution as f32;
        *vertex = vertex.clamp(Vec3::ZERO, Vec3::splat((resolution as f32).next_down()));
    }
}

fn voxelize_mesh(mesh: &Mesh) -> VoxelMap {
    let mut map = VoxelMap::new();
    for triangle in &mesh.triangles {
        let a = mesh.vertices[triangle[0] as usize];
        let b = mesh.vertices[triangle[1] as usize];
        let c = mesh.vertices[triangle[2] as usize];
        let helper = Helper::new(a, b, c);
        helper.visit_intersecting_voxels(|x, y, z| {
            let voxel_pos = IVec3::new(x as i32, y as i32, z as i32);
            let brick_pos = voxel_pos >> 3;
            let brick = map.chunks.entry(brick_pos).or_default();
            let index = Brick::voxel_index(voxel_pos & 7);
            brick.data[index] = 1;
        });
    }
    map
}

struct Helper {
    // Bounds
    min: UVec3,
    max: UVec3,
    // Tests
    n: Vec3,
    lower: f32,
    upper: f32,
    tests: [Vec4; 9],
}

impl Helper {
    fn new(a: Vec3, b: Vec3, c: Vec3) -> Self {
        let n = (b - a).cross(c - a);
        let signum = n.signum();

        let min_f = a.min(b).min(c);
        let max_f = a.max(b).max(c);

        let nd1 = n.x + n.y + n.z;
        let nda = n.dot(a);
        let nds = n.dot(signum);

        let lower = nda - (nd1 + nds) * 0.5;
        let upper = nda - (nd1 - nds) * 0.5;

        let mut tests = [Vec4::ZERO; 9];

        let tri = [a, b, c];
        for i in 0..3 {
            let pos = tri[i];
            let edge = tri[(i + 1) % 3] - tri[i];

            for a_idx in 0..3 {
                let b_idx = (a_idx + 1) % 3;
                let c_idx = (a_idx + 2) % 3;

                let edge_arr = edge.to_array();
                let signum_arr = signum.to_array();
                let pos_arr = pos.to_array();

                let n_test = Vec2::new(-edge_arr[b_idx], edge_arr[a_idx]) * signum_arr[c_idx];
                let p_test = Vec2::new(pos_arr[a_idx], pos_arr[b_idx]);
                let d = n_test.dot(p_test) - n_test.x.max(0.0) - n_test.y.max(0.0);

                let mut test_arr = [0.0; 4];
                test_arr[a_idx] = n_test.x;
                test_arr[b_idx] = n_test.y;
                test_arr[3] = d;

                tests[a_idx * 3 + i] = Vec4::from_array(test_arr);
            }
        }

        Self {
            min: min_f.as_uvec3(),
            max: max_f.as_uvec3(),
            n,
            lower,
            upper,
            tests,
        }
    }

    fn visit_intersecting_voxels<F>(&self, mut f: F)
    where
        F: FnMut(usize, usize, usize),
    {
        for z in self.min.z..=self.max.z {
            let mut y_started = false;
            for y in self.min.y..=self.max.y {
                let mut x_started = false;
                for x in self.min.x..=self.max.x {
                    let coord = UVec3::new(x, y, z);
                    let intersects = self.intersect(coord);
                    if intersects {
                        f(x as usize, y as usize, z as usize);
                    }

                    if x_started && !intersects {
                        break;
                    }
                    x_started = intersects;
                }
                if y_started && !x_started {
                    break;
                }
                y_started = x_started;
            }
        }
    }

    fn intersect(&self, p: UVec3) -> bool {
        let p = p.as_vec3();
        let d = self.n.dot(p);
        if d < self.lower || d > self.upper {
            return false;
        }
        for test in &self.tests {
            if test.truncate().dot(p) < test.w {
                return false;
            }
        }
        true
    }
}

struct Mesh {
    vertices: Vec<Vec3>,
    triangles: Vec<[u32; 3]>,
}

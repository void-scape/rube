use dot_vox::{Frame, Model, SceneNode};
use glam::{IVec3, Mat3, Vec3};
use rube_voxel::{Brick, VoxelMap};
use std::path::Path;
use tint::Color;

pub fn voxelize(path: impl AsRef<Path>) -> VoxelMap {
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

    println!("Voxelizing {}...", path.as_ref().display());
    let start = std::time::Instant::now();
    let vox = dot_vox::load(path.as_ref().to_str().unwrap()).unwrap();
    let mut map = VoxelMap::default();
    for (i, color) in vox.palette.iter().enumerate() {
        map.palette[i] = tint::Srgb::new(color.r, color.g, color.b, color.a)
            .to_linear()
            .to_array();
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

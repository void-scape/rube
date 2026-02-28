@group(1) @binding(0) var input_texture: texture_storage_2d<r32uint, read>;
@group(1) @binding(1) var output_texture: texture_storage_2d<rgba8unorm, write>;
@group(1) @binding(2) var<storage, read_write> dispatch: OcclusionDispatch;

struct OcclusionDispatch {
    x: u32,
    y: u32,
    z: u32,
}

@compute @workgroup_size(1)
fn prepare_occlusion() {
    dispatch.x = (active_voxels.counter + 63u) / 64u;
    dispatch.y = 1u;
    dispatch.z = 1u;
}

@compute @workgroup_size(64)
fn occlusion(@builtin(global_invocation_id) id: vec3<u32>, @builtin(local_invocation_index) index: u32) {
    let voxel_index = id.x;
    if voxel_index >= active_voxels.counter {
        return;
    }
	let face = unpack_voxel_face(active_voxels.faces[voxel_index]);
    let pos = face.pos;
 	let light_pos = vec3(
        1.088904,
        1.0411127,
        1.045433,
	); 
	let light_dir = light_pos - pos;
 	let rd = normalize(light_dir);
	let ro = pos;
	let hit = ray_cast(ro, rd, index);
	if !hit.escaped && (distance(ro, hit.pos) < distance(ro, light_pos)) {
		occlude_leaf_face(face.leaf_id, face.face_id);
	}
}

@compute @workgroup_size(8, 8)
fn lighting(@builtin(global_invocation_id) id: vec3<u32>) {
    let sz = textureDimensions(output_texture);
    if id.x >= sz.x || id.y >= sz.y {
        return;
    }
    let r = textureLoad(input_texture, id.xy).r;
    if r == 0xFFFFFFFFu {
		textureStore(output_texture, id.xy, vec4(vec3(0.0), 1.0));
        return;
    }
	let leaf_id = r >> 3;
	let face_id = r & 7;
	let packed = voxel_state[leaf_id];
	let state = unpack_voxel_state(packed, face_id);
    var base_color = palette[state.material_id].rgb;
	if state.occluded {
		base_color *= 0.1;
	}
    textureStore(output_texture, id.xy, vec4(base_color, 1.0));
}

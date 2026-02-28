struct CameraData {
    inv_proj_matrix: mat4x4<f32>,
    origin: vec3<f32>,
	_pad: f32,
}

struct Ray {
    ro: vec3<f32>,
    rd: vec3<f32>,
}

fn get_primary_ray(screenPos: vec2<u32>, sz: vec2<u32>) -> Ray {
    var uv = (vec2<f32>(screenPos) + vec2(0.5)) / vec2<f32>(sz);
    let ndc = vec2(uv.x * 2.0 - 1.0, -(uv.y * 2.0 - 1.0));
    let far = camera.inv_proj_matrix * vec4(ndc, 1.0, 1.0);
    var ray: Ray;
    ray.rd = normalize(far.xyz / far.w);
    ray.ro = camera.origin;
    return ray;
}

@group(1) @binding(0) var<uniform> camera: CameraData;
@group(2) @binding(0) var output: texture_storage_2d<r32uint, write>;

@compute @workgroup_size(8, 8)
fn raymarch(@builtin(global_invocation_id) id: vec3<u32>, @builtin(local_invocation_index) index: u32) {
    let sz = textureDimensions(output);
    if id.x >= sz.x || id.y >= sz.y {
        return;
    }
	let ray = get_primary_ray(id.xy, sz);
	let hit = ray_cast(ray.ro, ray.rd, index);
	if !hit.escaped {
		let face = normal_to_face(hit.normal);
		textureStore(output, id.xy, vec4((hit.leaf_id << 3) | face, vec3(0u)));
		if try_queue_leaf_face(hit.leaf_id, face, hit.material_id) {
		 	let grid_pos = floor((hit.pos - hit.normal * (cell_size * 0.1)) / cell_size);
		 	let center = grid_pos * cell_size + vec3(cell_size * 0.5);
			let pos = center + hit.normal * cell_size * (1.0 + 1e-4);
			let packed = pack_voxel_face(hit.leaf_id, hit.normal, pos);
			let active_index = atomicAdd(&active_voxels.counter, 1u);
			active_voxels.faces[active_index] = packed;
		}
    } else {
		textureStore(output, id.xy, vec4(0xFFFFFFFFu, vec3(0u)));
	}
}

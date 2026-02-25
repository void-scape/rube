// Sparse-64 voxel tree ray marcher implementation adapted from:
// https://dubiousconst282.github.io/2024/10/03/voxel-ray-tracing/

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

struct Node {
    // [     31    |    1    ]
    // [ child_ptr | is_leaf ]
    // is_leaf   // Indicates if this node is a leaf containing plain voxels.
    // child_ptr // Absolute offset to array of existing child nodes/voxels.
    child_ptr_is_leaf: u32,
    // [   32  |   32  ]
    // [ maskh | maskl ]
    // Indicates which children/voxels are present in array.
    maskl: u32,
    maskh: u32,
    _pad: u32,
}

fn is_leaf(node: Node) -> bool {
	return (node.child_ptr_is_leaf & 1) == 1;
}

fn child_ptr(node: Node) -> u32 {
	return node.child_ptr_is_leaf >> 1;
}

@group(0) @binding(0) var output: texture_storage_2d<rgba32float, write>;
@group(1) @binding(0) var<uniform> camera: CameraData;
@group(2) @binding(0) var<storage, read> nodes: array<Node>;
@group(2) @binding(1) var<storage, read> leaves: array<u32>;

@compute @workgroup_size(8, 8)
fn raymarch(@builtin(global_invocation_id) id: vec3<u32>, @builtin(local_invocation_index) index: u32) {
    let sz = textureDimensions(output);
    if id.x >= sz.x || id.y >= sz.y {
        return;
    }
	let ray = get_primary_ray(id.xy, sz);
	let hit = ray_cast(ray.ro, ray.rd, false, index);
	// if hit.material_id == 69 {
    //     return textureStore(output, id.xy, vec4(vec3(0.02), 1.0));
	// } 
	if hit.material_id != 0 && hit.material_id != 69 {
        textureStore(output, id.xy, vec4((hit.normal + 1.0) / 2.0, 1.0));
    } else {
        textureStore(output, id.xy, vec4(0.0, 0.0, 0.0, 1.0));
    }
}

struct HitInfo {
    material_id: u32,
    pos: vec3<f32>,
    normal: vec3<f32>,
}

var<workgroup> gs_stack: array<array<u32, 11>, 64>;
fn ray_cast(origin_in: vec3<f32>, dir_in: vec3<f32>, coarse: bool, local_idx: u32) -> HitInfo {
    var origin = origin_in;
    var dir = dir_in;

	// Perform aabb intersection check before descending tree to prevent rays from
	// starting outside of the 1..2 bounding volume. Rays can only traverse in this
	// range so this check is required to support arbitrary camera positioning in 
	// the world.
	let bbox_min = vec3(1.0);
    let bbox_max = vec3(2.0);
    let t0 = (bbox_min - origin) / dir;
    let t1 = (bbox_max - origin) / dir;
    let tmin = min(t0, t1);
    let tmax = max(t0, t1);
    let tnear = max(max(tmin.x, tmin.y), tmin.z);
    let tfar = min(min(tmax.x, tmax.y), tmax.z);

    if tnear > tfar || tfar < 0.0 {
        var hit: HitInfo;
        hit.material_id = 69u;
        return hit;
    }
    if tnear > 0.0 {
        origin = origin + dir * tnear;
    }
    
    var scale_exp = 21;
    var node_index = 0u;
    var node = nodes[node_index];

	// Mirror coordinates to negative ray octant to simplify cell intersections
    var mirror_mask = 0u;
    if dir.x > 0.0 { mirror_mask |= 3u << 0u; }
    if dir.y > 0.0 { mirror_mask |= 3u << 4u; }
    if dir.z > 0.0 { mirror_mask |= 3u << 2u; }

    origin = mirrored_pos(origin, dir, true);
    // Clamp to prevent traversal from completely breaking for rays starting outside tree
    var pos = clamp(origin, vec3(1.0), vec3(1.9999999));
    let inv_dir = 1.0 / -abs(dir);
    
    var side_dist: vec3<f32>;
    for (var i = 0; i < 256; i++) {
        if coarse && i > 20 && is_leaf(node) { 
			break;
		}
        var child_index = node_cell_index(pos, scale_exp) ^ mirror_mask;
        // Descend
        while (bitu64(node.maskl, node.maskh, child_index) != 0u && !is_leaf(node)) {
            gs_stack[local_idx][u32(scale_exp) >> 1u] = node_index;
            node_index = child_ptr(node) + popcnt(node.maskl, node.maskh, child_index);
            node = nodes[node_index];
            scale_exp -= 2;
            child_index = node_cell_index(pos, scale_exp) ^ mirror_mask;
        }
        
        if bitu64(node.maskl, node.maskh, child_index) != 0u && is_leaf(node) {
			break;
		}

        var adv_scale_ecp = scale_exp;
		// wtf
        if (shru64(node.maskl, node.maskh, child_index & 42u) & 0x00330033u) == 0u {
            adv_scale_ecp++;
        }

        // Compute next pos by intersecting with max cell sides
        let cell_min = floor_scale(pos, adv_scale_ecp);
        
        side_dist = (cell_min - origin) * inv_dir;
        let tmax = min(min(side_dist.x, side_dist.y), side_dist.z);
        
        let f = vec3<i32>((1 << u32(adv_scale_ecp)) - 1);
        let t  = vec3(-1);
        let neighbor_max = bitcast<vec3<i32>>(cell_min) + select(f, t, side_dist == vec3<f32>(tmax));
        pos = min(origin - abs(dir) * tmax, bitcast<vec3<f32>>(neighbor_max));

		// Find common ancestor based on left-most carry bit
        // We only care about changes in the exponent and high bits of
        // each cell position (10'10'10'...), so the odd bits are masked.
        let diff_pos = bitcast<vec3<u32>>(pos) ^ bitcast<vec3<u32>>(cell_min);
        let diff_exp = i32(firstLeadingBit((diff_pos.x | diff_pos.y | diff_pos.z) & 0xFFAAAAAAu)); 

        if diff_exp > scale_exp {
            scale_exp = diff_exp;
            if diff_exp > 21 {
				break;
			}

            node_index = gs_stack[local_idx][u32(scale_exp) >> 1u];
            node = nodes[node_index];
        }
    }
    
    var hit: HitInfo;
    hit.material_id = 0u;
    
    if is_leaf(node) && scale_exp <= 21 {
        pos = mirrored_pos(pos, dir, false);
        let child_index = node_cell_index(pos, scale_exp);

		let leaf_index = child_ptr(node) + popcnt(node.maskl, node.maskh, child_index);
        hit.material_id = (leaves[leaf_index / 4u] >> ((leaf_index % 4u) * 8u)) & 0xffu;
        hit.pos = pos;

        let tmax: f32 = min(min(side_dist.x, side_dist.y), side_dist.z);
        let side_mask = vec3(tmax) >= side_dist;
        hit.normal = select(vec3<f32>(0.0), -sign(dir), side_mask);
    }
    
    return hit;
}

// Reverses `pos` from range [1.0, 2.0) to (2.0, 1.0] if `dir > 0`.
fn mirrored_pos(pos: vec3<f32>, dir: vec3<f32>, range_check: bool) -> vec3<f32> {
    var mirrored = bitcast<vec3<f32>>(bitcast<vec3<u32>>(pos) ^ vec3(0x7FFFFFu));
	// XOR-ing will only work for coords in range [1.0, 2.0),
    // fallback to subtractions if that's not the case.
    if range_check && any((pos < vec3(1.0)) | (pos >= vec3(2.0))) {
        mirrored = vec3(3.0) - pos;
    }
    return select(pos, mirrored, dir > vec3(0.0));
}

fn node_cell_index(pos: vec3<f32>, scale_exp: i32) -> u32 {
    let cell_pos = (bitcast<vec3<u32>>(pos) >> vec3(u32(scale_exp))) & vec3(3u);
    return cell_pos.x + (cell_pos.z * 4u) + (cell_pos.y * 16u);
}

// floor(pos / scale) * scale
fn floor_scale(pos: vec3<f32>, scale_exp: i32) -> vec3<f32> {
    let mask = 0xFFFFFFFFu << u32(scale_exp);
    return bitcast<vec3<f32>>(bitcast<vec3<u32>>(pos) & vec3(mask));
}

fn bitu64(l: u32, h: u32, bit_idx: u32) -> u32 {
    if bit_idx < 32u {
        return (l >> bit_idx) & 1u;
    } else {
        return (h >> (bit_idx - 32u)) & 1u;
    }
}

fn shru64(l: u32, h: u32, shift: u32) -> u32 {
    if shift >= 32u {
        return h >> (shift - 32u);
    }
    if shift == 0u {
        return l;
    }
    return (l >> shift) | (h << (32u - shift));
}

// Count number of set bits in variable range [0..width].
fn popcnt(maskl: u32, maskh: u32, width: u32) -> u32 {
    var mask: u32 = maskl;
    var count: u32 = 0u;
    if width >= 32u {
        count = countOneBits(mask);
        mask = maskh;
    }
    let m = 1u << (width & 31u);
    count += countOneBits(mask & (m - 1u));
    return count;
}

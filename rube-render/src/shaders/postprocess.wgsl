struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vertex(@builtin(vertex_index) id: u32) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vec2(f32((id << 1u) & 2u), f32(id & 2u));
    out.clip_position = vec4(out.uv * 2.0 + vec2(-1.0, -1.0), 0.0, 1.0);
	out.uv.y = 1.0 - out.uv.y;
    return out;
}

override GAMMA_CORRECT: bool = false;
@group(0) @binding(0) var texture: texture_2d<f32>;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
	let sz = textureDimensions(texture);
    let color = textureLoad(texture, vec2<i32>(in.uv * vec2<f32>(sz)), 0);
	return select(color, vec4(pow(color.rgb, vec3(1.0 / 2.2)), color.a), GAMMA_CORRECT);
}

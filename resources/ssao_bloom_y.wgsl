struct Uniforms {
    inverse_view_projection_mat: mat4x4<f32>,
    view_projection_mat: mat4x4<f32>,
    light_position: vec4<f32>,
    camera_position: vec4<f32>,
    size: vec2<f32>,
    ambient: f32,
    diffuse: f32,
    specular: f32,
};
@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertInput {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) v_uv: vec2<f32>,
};

@vertex
fn vs_main(in: VertInput) -> VertOutput {
    var out: VertOutput;
    out.position = vec4<f32>(in.pos, 0., 1.);
    out.v_uv = in.uv;
    return out;
}
@group(0) @binding(1)
var s: sampler;
@group(0) @binding(2)
var t_bloom_ao: texture_2d<f32>;

@fragment
fn fs_main(in: VertOutput) -> @location(0) vec4<f32> {
    let bloom_ao = textureSample(t_bloom_ao, s, in.v_uv);
    return bloom_ao; // TODO compute blurs
}

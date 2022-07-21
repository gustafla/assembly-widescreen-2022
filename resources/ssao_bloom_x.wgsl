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
    @location(1) v_pos: vec2<f32>,
};

@vertex
fn vs_main(in: VertInput) -> VertOutput {
    var out: VertOutput;
    out.position = vec4<f32>(in.pos, 0., 1.);
    out.v_uv = in.uv;
    // Expand texture coordinate to [-1, 1] range and flip y direction to get NDC
    out.v_pos = (in.uv - 0.5) * vec2<f32>(2., -2.);
    return out;
}
@group(0) @binding(1)
var s: sampler;
@group(0) @binding(2)
var t_lit_ao: texture_2d<f32>;
@group(0) @binding(3)
var t_normal_depth: texture_2d<f32>;

fn frag_world_pos(in: VertOutput, depth: f32) -> vec3<f32> {
    let ndc = vec4<f32>(in.v_pos, depth, 1.);
    let pos = uniforms.inverse_view_projection_mat * ndc;
    return pos.xyz / pos.w;
}

@fragment
fn fs_main(in: VertOutput) -> @location(0) vec4<f32> {
    let lit_ao = textureSample(t_lit_ao, s, in.v_uv);
    let normal_depth = textureSample(t_normal_depth, s, in.v_uv);
    let world_pos = frag_world_pos(in, normal_depth.a);
    
    // TODO compute SSAO and bloom

    return vec4<f32>(lit_ao.rgb, 1.);
}

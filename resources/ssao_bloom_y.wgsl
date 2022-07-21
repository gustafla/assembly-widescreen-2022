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
    var weight: array<f32, 5> = array<f32, 5>(0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216);
    let pixel = 1. / uniforms.size.y;

    var result: vec3<f32> = textureSample(t_bloom_ao, s, in.v_uv).rgb * weight[0];
    for (var i: i32 = 1; i < 5; i+=1) {
        let offset = vec2<f32>(0., pixel * f32(i));
        result += textureSample(t_bloom_ao, s, in.v_uv + offset).rgb * weight[i];
        result += textureSample(t_bloom_ao, s, in.v_uv - offset).rgb * weight[i];
    }

    return vec4<f32>(result, 1.); // TODO compute SSAO blur
}

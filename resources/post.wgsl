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

@group(0) @binding(0)
var t_color: texture_2d<f32>;
@group(0) @binding(1)
var t_depth: texture_2d<f32>;
@group(0) @binding(2)
var s: sampler;

@fragment
fn fs_main(in: VertOutput) -> @location(0) vec4<f32> {
    return textureSample(t_color, s, in.v_uv);
}

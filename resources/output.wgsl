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

@group(0) @binding(0)
var s: sampler;
@group(0) @binding(1)
var t_color: texture_2d<f32>;

@fragment
fn fs_main(in: VertOutput) -> @location(0) vec4<f32> {
    return textureSample(t_color, s, in.v_uv);
}

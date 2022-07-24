struct PostUniforms {
    screen_size: vec2<f32>,
    post_noise_size: vec2<f32>,
    bloom_offset: vec2<f32>,
    bloom_sample_exponent: f32,
};
@group(0) @binding(0)
var<uniform> uniforms: PostUniforms;

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
var t_in: texture_2d<f32>;

fn light(uv: vec2<f32>) -> vec3<f32> {
    return pow(textureSample(t_in, s, uv).rgb, vec3<f32>(uniforms.bloom_sample_exponent));
}

fn bloom(uv: vec2<f32>) -> vec3<f32> {
    var weight: array<f32, 5> = array<f32, 5>(0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216);
    let pixel = (1. / uniforms.screen_size) * uniforms.bloom_offset;

    var result: vec3<f32> = light(uv) * weight[0];
    for (var i: i32 = 1; i < 5; i+=1) {
        let offset = pixel * f32(i);
        result += light(uv + offset) * weight[i];
        result += light(uv - offset) * weight[i];
    }
    
    return result;
}

@fragment
fn fs_main(in: VertOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(bloom(in.v_uv), 1.);
}

struct PostUniforms {
    screen_size: vec2<f32>,
    post_noise_size: vec2<f32>,
    bloom_offset: vec2<f32>,
    bloom_sample_bias: f32,
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

fn luminance(v: vec3<f32>) -> f32 {
    return dot(v, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn light(uv: vec2<f32>) -> vec3<f32> {
    return clamp(textureSample(t_in, s, uv).rgb - uniforms.bloom_sample_bias, vec3<f32>(0.), vec3<f32> (10.));
}

fn bloom(uv: vec2<f32>) -> vec3<f32> {
    var weight: array<f32, 100> = array<f32, 100>(0.007979, 0.007977, 0.007972, 0.007964, 0.007953, 0.007939, 0.007922, 0.007901, 0.007877, 0.007851, 0.007821, 0.007788, 0.007752, 0.007714, 0.007672, 0.007628, 0.007581, 0.007531, 0.007478, 0.007423, 0.007365, 0.007305, 0.007243, 0.007178, 0.007111, 0.007041, 0.006970, 0.006896, 0.006821, 0.006744, 0.006664, 0.006584, 0.006501, 0.006417, 0.006332, 0.006245, 0.006157, 0.006068, 0.005977, 0.005886, 0.005794, 0.005701, 0.005607, 0.005512, 0.005417, 0.005322, 0.005226, 0.005129, 0.005033, 0.004936, 0.004839, 0.004743, 0.004646, 0.004549, 0.004453, 0.004357, 0.004261, 0.004166, 0.004071, 0.003977, 0.003884, 0.003791, 0.003699, 0.003607, 0.003517, 0.003427, 0.003339, 0.003251, 0.003164, 0.003079, 0.002995, 0.002911, 0.002829, 0.002748, 0.002669, 0.002590, 0.002513, 0.002438, 0.002363, 0.002290, 0.002218, 0.002148, 0.002079, 0.002012, 0.001946, 0.001881, 0.001818, 0.001756, 0.001696, 0.001637, 0.001579, 0.001523, 0.001468, 0.001415, 0.001363, 0.001312, 0.001263, 0.001215, 0.001169, 0.001124);

    let pixel = (1. / uniforms.screen_size) * uniforms.bloom_offset;

    var result: vec3<f32> = light(uv) * weight[0];
    for (var i: i32 = 1; i < 100; i+=1) {
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

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
var t_lit: texture_2d<f32>;
@group(0) @binding(2)
var t_bloom_ao: texture_2d<f32>;

fn rrt_and_odt_fit(v: vec3<f32>) -> vec3<f32> {
    let a = v * (v + 0.0245786) - 0.000090537;
    let b = v * (0.983729 * v + 0.4329510) + 0.238081;
    return a / b;
}

fn aces_fitted(color: vec3<f32>) -> vec3<f32> {
    let aces_input = mat3x3<f32>(
        vec3<f32>(0.59719, 0.35458, 0.04823),
        vec3<f32>(0.07600, 0.90834, 0.01566),
        vec3<f32>(0.02840, 0.13383, 0.83777)
    );

    let aces_output = mat3x3<f32>(
        vec3<f32>( 1.60475, -0.53108, -0.07367),
        vec3<f32>(-0.10208,  1.10813, -0.00605),
        vec3<f32>(-0.00327, -0.07276,  1.07602)
    );

    let color: vec3<f32> = aces_input * color;
    let color = rrt_and_odt_fit(color);
    return aces_output * color;
}

fn aces(x: vec3<f32>) -> vec3<f32> {
  let a = 2.51;
  let b = 0.03;
  let c = 2.43;
  let d = 0.59;
  let e = 0.14;
  return (x * (a * x + b)) / (x * (c * x + d) + e);
}

fn luminance(v: vec3<f32>) -> f32 {
    return dot(v, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn reinhard_jodie(v: vec3<f32>) -> vec3<f32> {
    let l = luminance(v);
    let tv = v / (1.0 + v);
    return mix(v / (1.0 + l), tv, tv);
}

@fragment
fn fs_main(in: VertOutput) -> @location(0) vec4<f32> {
    let bloom_ao = textureSample(t_bloom_ao, s, in.v_uv);
    let color = textureSample(t_lit, s, in.v_uv).rgb * bloom_ao.a + bloom_ao.rgb;
    return vec4<f32>(aces_fitted(color), 1.);
}

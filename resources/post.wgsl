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

struct FragUniforms {
    inverse_view_projection_mat: mat4x4<f32>,
    light_position: vec4<f32>,
    camera_position: vec4<f32>,
    ambient: f32,
    diffuse: f32,
    specular: f32,
};
@group(0) @binding(0)
var<uniform> fs_uniforms: FragUniforms;
@group(0) @binding(1)
var s: sampler;
@group(0) @binding(2)
var t_color: texture_2d<f32>;
@group(0) @binding(3)
var t_normal: texture_2d<f32>;
@group(0) @binding(4)
var t_depth: texture_depth_2d;

fn frag_world_pos(in: VertOutput) -> vec3<f32> {
    let ndc = vec4<f32>(in.v_pos, textureSample(t_depth, s, in.v_uv), 1.);
    let pos = fs_uniforms.inverse_view_projection_mat * ndc;
    return pos.xyz / pos.w;
}

fn scene(pos: vec3<f32>) -> f32 {
    return length(pos + vec3<f32>(0.5, 0.0, 0.0)) - 0.5;
}

fn grad(p: vec3<f32>) -> vec3<f32> {
    let e = vec2<f32>(0.001, 0.0);
    return (vec3<f32>(scene(p+e.xyy), scene(p+e.yxy), scene(p+e.yyx)) - scene(p)) / e.x;
}

fn march(origin: vec3<f32>, direction: vec3<f32>) -> f32 {
    var t: f32 = 0.;
    var dist: f32;
    for (var i: i32 = 0; i < 30; i += 1) {
        dist = scene(origin + direction * t);
        t += dist;
        if (t > 99.) {
            t = 99.;
            break;
        }
    }
    return t;
}

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
    let cam_pos = fs_uniforms.camera_position.xyz;
    let rast_pos = frag_world_pos(in);
    let rast_t = distance(rast_pos, cam_pos);
    let direction = (rast_pos - cam_pos) / rast_t;

    var pos: vec3<f32> = rast_pos;
    var normal: vec3<f32> = textureSample(t_normal, s, in.v_uv).rgb;
    var color: vec3<f32> = textureSample(t_color, s, in.v_uv).rgb;
    let march_t = march(cam_pos, direction);
    if (march_t < rast_t) {
        pos = cam_pos + direction * march_t;
        normal = grad(pos);
        color = vec3<f32>(1.0, 0.5, 0.1);
    }

    let diffuse = max(dot(normalize(normal), normalize(fs_uniforms.light_position.xyz - pos)), 0.);
    return vec4<f32>(aces_fitted(color * diffuse), 1.);
}

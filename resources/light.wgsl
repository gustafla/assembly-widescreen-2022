struct Light {
    coordinates: vec4<f32>,
    rgb_intensity: vec3<f32>,
};

struct RenderUniforms {
    view_projection_mat: mat4x4<f32>,
    inverse_view_projection_mat: mat4x4<f32>,
    camera_position: vec4<f32>,
    lights: array<Light, 8>,
};
@group(0) @binding(0)
var<uniform> uniforms: RenderUniforms;

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
var t_depth: texture_depth_2d;
@group(0) @binding(3)
var t_color_roughness: texture_2d<f32>;
@group(0) @binding(4)
var t_normal: texture_2d<f32>;

fn ndc_to_world_pos(in: VertOutput) -> vec3<f32> {
    let ndc = vec4<f32>(in.v_pos, textureSample(t_depth, s, in.v_uv), 1.);
    let pos = uniforms.inverse_view_projection_mat * ndc;
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

@fragment
fn fs_main(in: VertOutput) -> @location(0) vec4<f32> {
    let frag_pos = ndc_to_world_pos(in);
    let cam_pos = uniforms.camera_position.xyz;
    let to_frag = frag_pos - cam_pos;
    let rast_t = length(to_frag);
    let direction = to_frag / rast_t;

    var pos: vec3<f32> = frag_pos;
    var normal: vec3<f32> = textureSample(t_normal, s, in.v_uv).rgb;
    var color: vec3<f32> = textureSample(t_color_roughness, s, in.v_uv).rgb;
    let march_t = march(cam_pos, direction);
    if (march_t < rast_t) {
        pos = cam_pos + direction * march_t;
        normal = grad(pos);
        color = vec3<f32>(1.0, 0.5, 0.1);
    }
    
    let normal = normalize(normal);
    let diffuse = max(dot(normal, normalize(cam_pos - pos)), 0.);
    return vec4<f32>(color * diffuse, 1.);
}

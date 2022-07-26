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
        if (t > 199.) {
            t = 199.;
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

    // Compose sphere tracing surfaces on top of rasterized
    var pos: vec3<f32> = frag_pos;
    var normal: vec3<f32> = textureSample(t_normal, s, in.v_uv).rgb;
    var color_roughness: vec4<f32> = textureSample(t_color_roughness, s, in.v_uv);
    let march_t = march(cam_pos, direction);
    if (march_t < rast_t) {
        pos = cam_pos + direction * march_t;
        normal = grad(pos);
        color_roughness = vec4<f32>(1.0, 0.5, 0.1, 0.5);
    }
    
    // Compute lighting
    var diff_sum: vec3<f32> = vec3<f32>(0.);
    var spec_sum: vec3<f32> = vec3<f32>(0.);
    var ambient: vec3<f32> = vec3<f32>(0.);
    let normal = normalize(normal);
    for (var i: i32 = 0; i < 8; i+=1) {
        let light = uniforms.lights[i];
        var light_dir: vec3<f32> = -normalize(light.coordinates.xyz);
        var attenuation: f32 = 1.;

        // Check if light is a point light
        if (light.coordinates.w != 0.) {
            let to_light = light.coordinates.xyz - pos;
            let len = length(to_light);
            light_dir = to_light / len;
            let len = max(len, 0.01);
            attenuation = 1. / (len * len);
        }
        
        // Compute diffuse lighting based on this variant of Oren-Nayar glsl code
        // https://github.com/glslify/glsl-diffuse-oren-nayar
        let v = -direction;
        let l_dot_v = dot(light_dir, v);
        let l_dot_n = dot(light_dir, normal);
        let n_dot_v = dot(normal, v);
        let s = l_dot_v - l_dot_n * n_dot_v;
        let t = mix(1., max(l_dot_n, n_dot_v), step(0., s));
        let sigma2 = color_roughness.a * color_roughness.a;
        let a = 1. + sigma2 * (1. / (sigma2 + 0.13) + 0.5 / (sigma2 + 0.33));
        let b = 0.45 * sigma2 / (sigma2 + 0.09);
        let diffuse = max(0., l_dot_n) * (a + b * s / t) / 3.14159265;
        
        // Lol add Blinn-Phong -style specular too, this is not correct either but I want it
        let halfway = normalize(light_dir + v);
        let spec = pow(max(dot(normal, halfway), 0.), 32.);
        
        diff_sum += light.rgb_intensity * diffuse * attenuation;
        spec_sum += light.rgb_intensity * spec * (1. - color_roughness.a) * attenuation;
        ambient += light.rgb_intensity;
    }

    return vec4<f32>(color_roughness.rgb * (diff_sum + ambient * 0.01) + spec_sum, 1.);
}

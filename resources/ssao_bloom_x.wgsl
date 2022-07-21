struct Uniforms {
    view_mat: mat4x4<f32>,
    inverse_view_mat: mat4x4<f32>,
    projection_mat: mat4x4<f32>,
    inverse_projection_mat: mat4x4<f32>,
    light_position: vec4<f32>,
    camera_position: vec4<f32>,
    screen_size: vec2<f32>,
    ambient: f32,
    diffuse: f32,
    specular: f32,
    ssao_noise_size: f32,
    ssao_kernel: array<vec4<f32>, 64>,
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
@group(0) @binding(4)
var t_ssao_noise: texture_2d<f32>;

fn frag_view_pos(ndc: vec3<f32>) -> vec3<f32> {
    let ndc = vec4<f32>(ndc, 1.);
    let pos = uniforms.inverse_projection_mat * ndc;
    return pos.xyz / pos.w;
}

fn light(uv: vec2<f32>) -> vec3<f32> {
    return clamp(pow(textureSample(t_lit_ao, s, uv).rgb, vec3<f32>(16.)), vec3<f32>(0.), vec3<f32>(1.));
}

fn bloom(uv: vec2<f32>) -> vec3<f32> {
    var weight: array<f32, 5> = array<f32, 5>(0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216);
    let pixel = 1. / uniforms.screen_size.x;

    var result: vec3<f32> = light(uv) * weight[0];
    for (var i: i32 = 1; i < 5; i+=1) {
        let offset = vec2<f32>(pixel * f32(i), 0.);
        result += light(uv + offset) * weight[i];
        result += light(uv - offset) * weight[i];
    }
    
    return result;
}

@fragment
fn fs_main(in: VertOutput) -> @location(0) vec4<f32> {
    let normal_depth = textureSample(t_normal_depth, s, in.v_uv);
    let normal = normal_depth.rgb;
    let view_pos = frag_view_pos(vec3<f32>(in.v_pos, normal_depth.a));
    let noise_scale = uniforms.screen_size / uniforms.ssao_noise_size;
    let random = vec3<f32>(textureSample(t_ssao_noise, s, in.v_uv * noise_scale).rg, 0.);
    
    let tangent = normalize(random - normal * dot(random, normal));
    let bitangent = cross(normal, tangent);
    let tbn = mat3x3<f32>(tangent, bitangent, normal);
    
    let radius = 2.;
    var occlusion: f32 = 0.;
    for (var i: i32 = 0; i < 64; i+=1) {
        let sample_pos = tbn * uniforms.ssao_kernel[i].xyz;
        let sample_pos = view_pos + sample_pos * radius;
        
        let offset = vec4<f32>(sample_pos, 1.);
        let offset = uniforms.projection_mat * offset;
        let offset = offset.xyz / offset.w;
        let texcoord = offset.xy * vec2<f32>(0.5, -0.5) + 0.5;
        
        let sample_depth = frag_view_pos(vec3<f32>(offset.xy, textureSample(t_normal_depth, s, texcoord).a)).z;
        if (sample_depth >= sample_pos.z + 0.02) {
            occlusion += smoothstep(0., 1., radius / abs(view_pos.z - sample_depth));
        }
    }

    return vec4<f32>(bloom(in.v_uv), 1. - occlusion / 64.);
}

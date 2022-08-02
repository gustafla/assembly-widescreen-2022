struct Light {
    coordinates: vec4<f32>,
    rgb_intensity: vec3<f32>,
};

struct RenderUniforms {
    view_projection_mat: mat4x4<f32>,
    inverse_view_projection_mat: mat4x4<f32>,
    shadow_view_projection_mat: mat4x4<f32>,
    camera_position: vec4<f32>,
    ambient: f32,
    march_multiplier: f32,
    global_time: f32,
    beat: f32,
    lights: array<Light, 4>,
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
var t_shadow: texture_depth_2d;
@group(0) @binding(4)
var t_color_roughness: texture_2d<f32>;
@group(0) @binding(5)
var t_normal: texture_2d<f32>;

fn ndc_to_world_pos(in: VertOutput) -> vec3<f32> {
    let ndc = vec4<f32>(in.v_pos, textureSample(t_depth, s, in.v_uv), 1.);
    let pos = uniforms.inverse_view_projection_mat * ndc;
    return pos.xyz / pos.w;
}

fn rotation_x(a: f32) -> mat3x3<f32> {
    return mat3x3<f32>(
         1.0,  0.0,     0.0,
         0.0,  cos(a), -sin(a),
         0.0,  sin(a),  cos(a)
    );
}

fn tube(pos: vec3<f32>) -> f32 {
    let d = abs(pos.yz) - vec2<f32>(1.0);
    return min(max(d.x,d.y), 0.0) + length(max(d, vec2<f32>(0.0)));
}

fn scene(pos: vec3<f32>) -> f32 {
    let t = uniforms.global_time * 0.5;
    let b = uniforms.beat;
    let wave =
        sin(pos.x * 0.1 + t * 0.3) * 2.
      + sin(pos.x * 0.5 - t * 3.) * 0.9 * sin(pos.z * 0.1)
      + sin(pos.x + t * 0.12) * 0.1
      + sin(pos.z * 3. + t) * 0.6
      + sin(pos.z * 1. + t)
      + sin(pos.x / (1. - b) + b) * b
    ;
    // Repeat space over z
    let pos = vec3<f32>(pos.x, pos.y + wave - 8., (pos.z + 1000.) % 16.);
    let rotation = sin(pos.x * 0.15 + t * 0.11 + b * 3.33) * 2.13;
    return tube(rotation_x(rotation) * pos);
}

fn grad(p: vec3<f32>) -> vec3<f32> {
    let e = vec2<f32>(0.001, 0.0);
    return (vec3<f32>(scene(p+e.xyy), scene(p+e.yxy), scene(p+e.yyx)) - scene(p)) / e.x;
}

fn march(origin: vec3<f32>, direction: vec3<f32>, trange: vec2<f32>) -> f32 {
    var t: f32 = trange.x;
    var dist: f32;
    for (var i: i32 = 0; i < 100; i += 1) {
        dist = scene(origin + direction * t) * uniforms.march_multiplier;
        if (dist < 0.0001) {
            return t;
        }
        t += dist;
        if (t > trange.y) {
            t = trange.y;
            break;
        }
    }
    return t;
}

fn shadow(pos: vec3<f32>, bias: f32) -> f32 {
    let pos_shadow = uniforms.shadow_view_projection_mat * vec4<f32>(pos, 1.);
    let pos_shadow = pos_shadow.xyz / pos_shadow.w;
    let closest_depth = textureSample(t_shadow, s, pos_shadow.xy * vec2<f32>(0.5, -0.5) + 0.5);
    if (pos_shadow.z - bias > closest_depth) {
        return 0.;
    }
    return 1.;
}

fn volumetric_light(origin: vec3<f32>, direction: vec3<f32>, max_t: f32) -> f32 {
    let opacity = 0.001;
    let step = 0.1;
    var sum: f32 = 0.;
    for (var i: i32 = 0; i < 1000; i += 1) {
        let t = f32(i) * step;
        if (t > max_t) {
            break;
        }
        let pos = origin + direction * t;
        sum += shadow(pos, 0.) * opacity;
    }
    
    return sum;
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
    var distance_cam: f32 = rast_t;
    let march_t = march(cam_pos, direction, vec2<f32>(1., 999.));
    if (march_t < rast_t && uniforms.march_multiplier <= 1.) {
        pos = cam_pos + direction * march_t;
        normal = grad(pos);
        color_roughness = vec4<f32>(.1, 0.01, 0.2, 0.0);
        distance_cam = march_t;
    }
    
    // Compute lighting (Blinn-Phong)
    var diff_sum: vec3<f32> = vec3<f32>(0.);
    var spec_sum: vec3<f32> = vec3<f32>(0.);
    var ambient: vec3<f32> = vec3<f32>(0.);
    let normal = normalize(normal);
    for (var i: i32 = 0; i < 4; i+=1) {
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
        
        let diffuse = max(dot(normal, light_dir), 0.);
        let halfway = normalize(light_dir - direction);
        let spec = pow(max(dot(normal, halfway), 0.), 32.);
        
        // Account for shadow map if first light
        var s: f32 = 1.;
        if (i == 0) {
            s = shadow(pos, 0.0001);
        }

        diff_sum += light.rgb_intensity * diffuse * attenuation * s;
        spec_sum += light.rgb_intensity * spec * (1. - color_roughness.a) * attenuation * s;
        ambient += light.rgb_intensity;
    }

    // Add volumetric light
    var total_light: vec3<f32> = color_roughness.rgb * (diff_sum + ambient * uniforms.ambient) + spec_sum;
    total_light += volumetric_light(cam_pos, direction, distance_cam) * uniforms.lights[0].rgb_intensity;

    // Output with distance fog lit by primary light
    return vec4<f32>(mix(total_light, uniforms.lights[0].rgb_intensity, distance_cam / 1000.), 1.);
}

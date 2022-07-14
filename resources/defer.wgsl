struct VertUniforms {
    model_mat: mat4x4<f32>,
    view_projection_mat: mat4x4<f32>,
    normal_mat: mat4x4<f32>,
};
@group(0) @binding(0)
var<uniform> vs_uniforms: VertUniforms;

struct VertInput {
    @location(0) pos: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) color: vec4<f32>,
};

struct VertOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) v_normal: vec4<f32>,
    @location(1) v_color: vec4<f32>,
};

@vertex
fn vs_main(in: VertInput) -> VertOutput {
    var out: VertOutput;
    out.position = vs_uniforms.view_projection_mat * vs_uniforms.model_mat * in.pos;
    out.v_normal = vs_uniforms.normal_mat * in.normal;
    out.v_color = in.color;
    return out;
}

struct FragOutput {
    @location(0) color: vec4<f32>,
    @location(1) normal: vec4<f32>,
};

@fragment
fn fs_main(in: VertOutput) -> FragOutput {
    //let normal = normalize(in.v_normal.xyz);
    //let to_light = normalize(fs_uniforms.light_position.xyz - in.v_position.xyz);
    //let to_camera = normalize(fs_uniforms.camera_position.xyz - in.v_position.xyz);
    //let half = normalize(to_light + to_camera);

    //var diffuse = fs_uniforms.diffuse * max(dot(normal, to_light), 0.0);
    //var specular = fs_uniforms.specular * pow(max(dot(normal, half), 0.0), 32.0);

    //return in.v_color * (fs_uniforms.ambient + diffuse) + specular;
    var out: FragOutput;
    out.color = in.v_color;
    out.normal = in.v_normal;
    return out;
}

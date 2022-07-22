struct Uniforms {
    view_projection_mat: mat4x4<f32>,
    inverse_view_projection_mat: mat4x4<f32>,
};
@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertInput {
    @location(0) local_position: vec4<f32>,
    @location(1) color_roughness: vec4<f32>,
    @location(2) normal: vec4<f32>,
};

struct InstanceInput {
    @location(8)  model_0:  vec4<f32>,
    @location(9)  model_1:  vec4<f32>,
    @location(10) model_2:  vec4<f32>,
    @location(11) model_3:  vec4<f32>,
    @location(12) normal_0: vec4<f32>,
    @location(13) normal_1: vec4<f32>,
    @location(14) normal_2: vec4<f32>,
    @location(15) normal_3: vec4<f32>,
};

struct VertOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) v_color_roughness: vec4<f32>,
    @location(1) v_normal: vec3<f32>,
};

@vertex
fn vs_main(vert: VertInput, inst: InstanceInput) -> VertOutput {
    let model_mat = mat4x4<f32>(
        inst.model_0,
        inst.model_1,
        inst.model_2,
        inst.model_3,
    );
    let normal_mat = mat4x4<f32>(
        inst.normal_0,
        inst.normal_1,
        inst.normal_2,
        inst.normal_3,
    );

    var out: VertOutput;
    out.clip_position = uniforms.view_projection_mat * model_mat * vert.local_position;
    out.v_color_roughness = vert.color_roughness;
    out.v_normal = (normal_mat * vert.normal).xyz;
    return out;
}

struct FragOutput {
    @location(0) color_roughness: vec4<f32>,
    @location(1) normal: vec4<f32>,
};

@fragment
fn fs_main(in: VertOutput) -> FragOutput {
    var out: FragOutput;
    out.color_roughness = in.v_color_roughness;
    out.normal = vec4<f32>(in.v_normal, 0.);
    return out;
}

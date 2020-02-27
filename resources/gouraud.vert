#version 100
precision mediump float;

attribute vec3 a_Pos;
attribute vec3 a_Normal;

varying vec4 v_Color;

uniform mat4 u_Projection;
uniform mat4 u_View;
uniform vec3 u_LightPosition[128];

void main() {
    float l = 0.;

    for (int i = 0; i < 128; i++) {
        vec3 light_position = u_LightPosition[i];
        vec3 p = light_position - a_Pos.xyz;
        float len = length(p);
        l += max(dot(a_Normal, p/len), 0.) / (len * len);
    }

    v_Color = vec4(vec3(l), 1.);

    gl_Position = u_Projection * u_View * vec4(a_Pos, 1.0);
}

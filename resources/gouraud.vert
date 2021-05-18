#version 100
precision mediump float;

attribute vec3 a_Pos;
attribute vec3 a_Normal;

varying vec4 v_Color;

uniform mat4 u_Projection;
uniform mat4 u_View;
uniform mat4 u_Model;
uniform mat3 u_ModelNormal;
uniform vec3 u_SunDir;
uniform float u_AmbientLevel;
uniform float u_SunLevel;

void main() {
    float l = u_AmbientLevel;

    l += max(dot(u_ModelNormal * a_Normal, -normalize(u_SunDir)), 0.) *
        u_SunLevel;

    v_Color = vec4(vec3(l), 1.);

    gl_Position = u_Projection * u_View * u_Model * vec4(a_Pos, 1.0);
}

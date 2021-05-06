#version 100
precision mediump float;

attribute vec3 a_Pos;
attribute vec3 a_Normal;

varying vec4 v_Color;

uniform mat4 u_Projection;
uniform mat4 u_View;
uniform mat4 u_Model;
uniform mat3 u_ModelNormal;

void main() {
    vec3 sundir = normalize(vec3(2., -3., 0.4));

    float l = 0.01;

    l += max(dot(u_ModelNormal * a_Normal, -sundir), 0.) * 0.2;

    v_Color = vec4(vec3(l), 1.);

    gl_Position = u_Projection * u_View * u_Model * vec4(a_Pos, 1.0);
}

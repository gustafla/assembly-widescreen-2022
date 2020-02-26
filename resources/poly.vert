#version 100
precision mediump float;

attribute vec3 a_Pos;

varying vec4 v_Color;

uniform mat4 u_Projection;
uniform mat4 u_View;

void main() {
    vec4 pos = u_View * vec4(a_Pos, 1.0);

    v_Color = vec4(vec3(0.5), 1.);

    gl_Position = u_Projection * pos;
}

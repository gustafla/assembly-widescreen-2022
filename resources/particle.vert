#version 100
precision mediump float;

attribute vec3 a_Pos;

varying vec3 v_Pos;

uniform mat4 u_Projection;
uniform mat4 u_View;
uniform mat4 u_Model;

void main() {
    v_Pos = a_Pos;

    vec4 pos = u_View * u_Model * vec4(a_Pos, 1.0);
    gl_PointSize = 4.;
    gl_Position = u_Projection * pos;
}

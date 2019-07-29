#version 100
precision mediump float;

attribute vec3 a_Pos;
attribute vec2 a_TexCoord;

varying vec3 v_Pos;
varying vec2 v_TexCoord;

void main() {
    v_Pos = a_Pos;
    v_TexCoord = a_TexCoord;
    gl_Position = vec4(a_Pos, 1.0);
}

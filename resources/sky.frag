#version 100
precision mediump float;

varying vec2 v_TexCoord;

uniform vec2 u_Resolution;

void main() {
    // output
    gl_FragColor = vec4(0.5, 0.2, -0.4 + v_TexCoord.y, 1.);
}
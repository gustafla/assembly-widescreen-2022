#version 100
precision mediump float;

varying vec3 v_Pos;
varying vec2 v_TexCoord;

uniform sampler2D u_InputSampler;

void main() {
    gl_FragColor = vec4(texture2D(u_InputSampler, v_TexCoord).rgb, 1.);
}

#version 100
precision mediump float;

varying vec3 v_Pos;
varying vec2 v_TexCoord;

uniform sampler2D u_InputSampler;

void main() {
    vec3 color = texture2D(u_InputSampler, v_TexCoord).rgb;
    color -= length((v_TexCoord - vec2(0.5)) * 0.4); // vignette
    gl_FragColor = vec4(color, 1.);
}

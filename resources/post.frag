#version 100
precision mediump float;

varying vec3 v_Pos;
varying vec2 v_TexCoord;

uniform float u_NoiseAmount;
uniform float u_NoiseScale;
uniform sampler2D u_InputSampler0; // Bloom
uniform sampler2D u_InputSampler1; // Render
uniform sampler2D u_InputSampler2; // Noise
uniform vec2 u_Resolution;

void main() {
    // coordinates
    vec2 center = v_TexCoord - vec2(0.5);
    vec2 distor = center * vec2(
            1. + abs(center.y * center.y) * 0.1,
            1. + abs(center.x * center.x) * 0.2) + vec2(0.5);
    vec2 noisecoord = v_TexCoord * u_NoiseScale;

    // colors (clamp(bloom + original - vignette) + noise)
    vec3 color = max(
            texture2D(u_InputSampler0, distor).rgb + // bloom
            texture2D(u_InputSampler1, distor).rgb - // image
            length(center * 0.4), 0.) + // vignette
        texture2D(u_InputSampler2, noisecoord).r * u_NoiseAmount; // noise

    // output
    if (distor.x < 0. || distor.x > 1. || distor.y < 0. || distor.y > 1.) {
        gl_FragColor = vec4(vec3(0.), 1.); // Hide "overscan" at edges
    } else {
        gl_FragColor = vec4(color, 1.); // Actual output
    }
}

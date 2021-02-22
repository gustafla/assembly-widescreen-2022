#version 100
precision mediump float;

varying vec2 v_TexCoord;

uniform float u_NoiseAmount;
uniform float u_NoiseScale;
uniform float u_Beat;
uniform sampler2D u_InputSampler0; // Bloom
uniform sampler2D u_InputSampler1; // Render
uniform sampler2D u_InputSampler2; // Noise
uniform vec2 u_Resolution;

float bloom(vec2 coords) {
    return
    texture2D(u_InputSampler0, coords, 7.).r +
    texture2D(u_InputSampler0, coords, 6.).r +
    texture2D(u_InputSampler0, coords, 5.).r +
    texture2D(u_InputSampler0, coords, 4.).r +
    texture2D(u_InputSampler0, coords, 3.).r;
}

void main() {
    // coordinates
    vec2 center = v_TexCoord - vec2(0.5);
    vec2 distor = center * vec2(
            1. + abs(center.y * center.y) * 0.1,
            1. + abs(center.x * center.x) * 0.2) + vec2(0.5);

    if (distor.x < 0. || distor.x > 1. || distor.y < 0. || distor.y > 1.) {
        discard; // Hide "overscan" at edges
    }

    vec2 noisecoord = v_TexCoord * u_NoiseScale;

    // colors (clamp(bloom + original - vignette) + noise)
    vec3 color = max(
            vec3(bloom(distor)) + // bloom
            texture2D(u_InputSampler1, distor).rgb - // image
            length(center * 0.4), 0.) + // vignette
        texture2D(u_InputSampler2, noisecoord).r * u_NoiseAmount; // noise

    // output
    gl_FragColor = vec4(color + u_Beat * 0.01, 1.);
}

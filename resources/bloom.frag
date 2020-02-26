#version 100
precision mediump float;

varying vec2 v_TexCoord;

uniform sampler2D u_InputSampler0;

float intensity(vec3 color) {
    return 0.2126 * color.r + 0.7152 * color.g + 0.0722 * color.b;
}

void main() {
    vec3 color = texture2D(u_InputSampler0, v_TexCoord).rgb;
    gl_FragColor = vec4(color * intensity(color), 1.);
}

#version 100
precision mediump float;

varying vec3 v_Pos;
varying vec2 v_TexCoord;

uniform sampler2D u_InputSampler0;
uniform vec2 u_Resolution;

#define KERNEL_SIZE 35

void main() {
    float KERNEL[KERNEL_SIZE];
    KERNEL[0] = 0.010232;
    KERNEL[1] = 0.012066;
    KERNEL[2] = 0.014087;
    KERNEL[3] = 0.016283;
    KERNEL[4] = 0.018635;
    KERNEL[5] = 0.021114;
    KERNEL[6] = 0.023685;
    KERNEL[7] = 0.026305;
    KERNEL[8] = 0.028924;
    KERNEL[9] = 0.031488;
    KERNEL[10] = 0.033938;
    KERNEL[11] = 0.036215;
    KERNEL[12] = 0.038261;
    KERNEL[13] = 0.040021;
    KERNEL[14] = 0.041445;
    KERNEL[15] = 0.042493;
    KERNEL[16] = 0.043135;
    KERNEL[17] = 0.043351;
    KERNEL[18] = 0.043135;
    KERNEL[19] = 0.042493;
    KERNEL[20] = 0.041445;
    KERNEL[21] = 0.040021;
    KERNEL[22] = 0.038261;
    KERNEL[23] = 0.036215;
    KERNEL[24] = 0.033938;
    KERNEL[25] = 0.031488;
    KERNEL[26] = 0.028924;
    KERNEL[27] = 0.026305;
    KERNEL[28] = 0.023685;
    KERNEL[29] = 0.021114;
    KERNEL[30] = 0.018635;
    KERNEL[31] = 0.016283;
    KERNEL[32] = 0.014087;
    KERNEL[33] = 0.012066;
    KERNEL[34] = 0.010232;

    vec3 color = vec3(0.);

    for (int i=0; i < KERNEL_SIZE; i++) {
        float pix_offs = 1. / u_Resolution.y * float(i - KERNEL_SIZE/2);
        vec2 samplepos = vec2(v_TexCoord.x, v_TexCoord.y - pix_offs);
        color += texture2D(u_InputSampler0, samplepos).rgb * KERNEL[i];
    }

    gl_FragColor = vec4(color, 1.);
}

// TODO DRY or http://rastergrid.com/blog/2010/09/efficient-gaussian-blur-with-linear-sampling/
// https://software.intel.com/en-us/blogs/2014/07/15/an-investigation-of-fast-real-time-gpu-based-image-blur-algorithms
// http://dev.theomader.com/gaussian-kernel-calculator/

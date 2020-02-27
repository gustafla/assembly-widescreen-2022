#version 100
precision mediump float;

attribute vec3 a_Pos;
attribute vec3 a_Normal;

varying vec4 v_Color;

uniform mat4 u_Projection;
uniform mat4 u_View;
uniform int u_LightCountX;
uniform int u_LightCountY;
uniform int u_LightCountZ;
uniform vec3 u_LightGridScale;
uniform float u_LightIntensity[256];
uniform float u_LightIntensityScale;

void main() {
    float l = 0.;
    int i = 0;
    for (int iz = 0; iz < 256; iz++) {
        if (iz >= u_LightCountZ) {
            break;
        }
        for (int iy = 0; iy < 256; iy++) {
            if (iy >= u_LightCountY) {
                break;
            }
            for (int ix = 0; ix < 256; ix++) {
                if (ix >= u_LightCountX) {
                    break;
                }
                vec3 light_position = vec3(
                    float(ix) - float(u_LightCountX) / 2.,
                    float(iy) - float(u_LightCountY) / 2.,
                    float(iz) - float(u_LightCountZ) / 2.
                ) * u_LightGridScale;

                vec3 p = light_position - a_Pos.xyz;
                float len = length(p);
                l += (max(dot(a_Normal, p/len), 0.) * u_LightIntensity[i] * u_LightIntensityScale)
                    / (len * len);
                i++;
            }
        }
    }

    v_Color = vec4(vec3(l), 1.);

    gl_Position = u_Projection * u_View * vec4(a_Pos, 1.0);
}

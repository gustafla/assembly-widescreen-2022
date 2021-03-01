#version 100
precision mediump float;

attribute vec3 a_Pos;

varying vec4 v_Color;

uniform vec2 u_Resolution;
uniform mat4 u_Projection;
uniform mat4 u_View;

vec3 hsv2rgb(vec3 c) {
  vec4 K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
  vec3 p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
  return c.z * mix(K.xxx, clamp(p - K.xxx, 0.0, 1.0), c.y);
}

void main() {
    vec4 pos = u_View * vec4(a_Pos, 1.0);
    float dist = -pos.z;

    v_Color = vec4(hsv2rgb(vec3((a_Pos.x + a_Pos.y) / 110. + 0.5, 0.4, 1.3)),
            20. / dist);

    gl_PointSize = max(length(u_Resolution) / (dist * 10.), 1.);
    gl_Position = u_Projection * pos;
}

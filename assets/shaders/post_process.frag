varying vec2 vert;

uniform sampler2D render_target;

void main() {
    vec2 screen_uv = vert;
    vec3 color = texture2D(render_target, screen_uv).rgb;
    gl_FragColor = vec4(0.0, 0.0, 0.0, 1.0);
    gl_FragColor.rgb += texture2D(render_target, screen_uv + 0.005).rgb * vec3(0.0, 1.0, 1.0) * 0.25;
    gl_FragColor.rgb += texture2D(render_target, screen_uv).rgb * 0.5;
    gl_FragColor.rgb += texture2D(render_target, screen_uv - 0.005).rgb * vec3(0.0, 1.0, 0.0) * 0.25;
}

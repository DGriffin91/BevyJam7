attribute vec3 a_position;
attribute vec4 a_color;
varying vec4 color;

void main() {
    color = a_color;
    gl_Position = ub_clip_from_world * vec4(a_position, 1.0);
}

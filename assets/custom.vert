#version 450

precision highp float;

layout(std140, set = 0, binding = 0) uniform CustomUniformArgs {
    uniform float scale;
    uniform vec2 offset;
    uniform float aspect_ratio;
    uniform int max_iters;
};

layout(location = 0) in vec2 pos;

layout(location = 0) out vec2 coordinate;
layout(location = 1) out int max_iters;


void main() {
    coordinate = scale * mat2(2.0*aspect_ratio, 0.0, 0.0, -2.0) * pos - offset;


    gl_Position = vec4(pos, 0.0, 1.0);
}

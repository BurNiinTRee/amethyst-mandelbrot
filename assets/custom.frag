#version 450

layout(std140, set = 0, binding = 0) uniform CustomUniformArgs {
    uniform float scale;
    uniform vec2 offset;
    uniform float aspect_ratio;
    uniform int max_iters;
};

layout(location = 0) in vec2 coordinate;

layout(location = 0) out vec4 out_color;


float abs_squared(vec2 z) {
    return (z.x * z.x + z.y * z.y);
}

vec2 square(vec2 z) {
    float z_squared_r = z.x * z.x - z.y * z.y;
    float z_squared_i = 2.0 * z.x * z.y;

    return vec2(z_squared_r, z_squared_i);
}


int mandelbrot(vec2 c) {
    vec2 z = vec2(0, 0);
    for (int i = 1; i < max_iters; i++) {
        if (abs_squared(z) > 4.0) {
            return i;
        }
        z = square(z) + c;
    }
    return 0;
}


void main() {
    int iters = mandelbrot(coordinate);
    float color_value;
    if (iters == 0) {
        color_value = 0.0;
    } else {
        float v = iters;
        v = v / max_iters;
        color_value = v; 
    }
    out_color = vec4(color_value, color_value, color_value, 1.0);
}


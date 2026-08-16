// The one pipeline. Everything the engine draws — sprites now, rectangles,
// lines, circles and glyphs at R3 — arrives here as two triangles with a
// texture, a UV pair and a tint (renderer.md §7).
//
// Deliberately dull. Every decision about what is drawn, in what order, in how
// many batches was made in `jidousha-render-core` before this ran; a shader
// that decided anything would be a second place two backends could disagree
// (renderer.md §1).
//
// Colors arrive **already linear**. The engine's `Color` is sRGB-encoded
// (conventions), and the conversion happens on the CPU in `color.rs` so that
// the clear color and the vertex colors go through one implementation rather
// than one here and one there. The surface is an `-srgb` format, so what this
// returns is encoded on the way out.

struct Camera {
    view_projection: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> camera: Camera;

@group(1) @binding(0) var sprite_texture: texture_2d<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;

struct VertexIn {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct VertexOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}

@vertex
fn vertex_main(input: VertexIn) -> VertexOut {
    var output: VertexOut;
    // Z is zero for every quad: v1 is the painter's algorithm with no depth
    // buffer, so draw order alone decides what is on top (renderer.md §2).
    output.clip_position = camera.view_projection * vec4<f32>(input.position, 0.0, 1.0);
    output.uv = input.uv;
    output.color = input.color;
    return output;
}

@fragment
fn fragment_main(input: VertexOut) -> @location(0) vec4<f32> {
    // The texture is an `-srgb` format, so the sample is already linear too.
    // Straight (non-premultiplied) alpha, multiplied by the tint, blended by
    // the pipeline's blend state (conventions).
    return textureSample(sprite_texture, sprite_sampler, input.uv) * input.color;
}

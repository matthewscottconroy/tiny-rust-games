#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct RippleMaterial {
    time: f32,
}

@group(2) @binding(0)
var<uniform> material: RippleMaterial;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let center = vec2<f32>(0.5, 0.5);
    let dist = length(uv - center);
    let wave = sin(dist * 30.0 - material.time * 3.0) * 0.5 + 0.5;
    return vec4<f32>(wave * 0.15, wave * 0.5, wave * 0.9, 1.0);
}

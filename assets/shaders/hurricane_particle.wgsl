// Hurricane Particle Shader
// Renders particles as point sprites with color based on energy density

struct CameraUniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) size: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) point_coord: vec2<f32>,
};

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = vec4<f32>(vertex.position, 1.0);
    out.clip_position = camera.proj * camera.view * world_pos;
    out.color = vertex.color;
    out.point_coord = vec2<f32>(0.0, 0.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Soft circular particle
    let dist = length(in.point_coord);
    if dist > 1.0 {
        discard;
    }
    let alpha = (1.0 - dist * dist) * in.color.a;
    return vec4<f32>(in.color.rgb, alpha);
}

#import bevy_pbr::mesh_view_bindings

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) instance_transform_0: vec4<f32>,
    @location(4) instance_transform_1: vec4<f32>,
    @location(5) instance_transform_2: vec4<f32>,
    @location(6) instance_transform_3: vec4<f32>,
    @location(7) instance_color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    let instance_transform = mat4x4<f32>(
        vertex.instance_transform_0,
        vertex.instance_transform_1,
        vertex.instance_transform_2,
        vertex.instance_transform_3,
    );
    let world_position = instance_transform * vec4<f32>(vertex.position, 1.0);

    var out: VertexOutput;
    out.clip_position = mesh_view_bindings::view.view_proj * world_position;
    out.color = vertex.instance_color;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
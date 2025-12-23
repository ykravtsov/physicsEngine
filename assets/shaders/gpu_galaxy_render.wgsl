// GPU Galaxy Rendering Shader
// Reads particles directly from storage buffer for high-performance instancing

#import bevy_pbr::mesh_view_bindings

struct Particle {
    pos: vec4<f32>,
    vel: vec4<f32>,
    color: vec4<f32>,
};

@group(0) @binding(0) var<storage, read> particles: array<Particle>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vertex(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    // Simple quad vertices for each particle
    let quad_vertices = array<vec2<f32>, 6>(
        vec2<f32>(-0.5, -0.5), // bottom left
        vec2<f32>( 0.5, -0.5), // bottom right
        vec2<f32>( 0.5,  0.5), // top right
        vec2<f32>( 0.5,  0.5), // top right
        vec2<f32>(-0.5,  0.5), // top left
        vec2<f32>(-0.5, -0.5), // bottom left
    );

    let quad_vertex = quad_vertices[vertex_index % 6u];

    // Get particle data
    let particle = particles[instance_index];

    // Scale quad based on distance (farther = smaller)
    let distance = length(particle.pos.xyz);
    let scale = max(0.01, 0.1 / (1.0 + distance * 0.01));

    // Position quad in world space
    let world_pos = particle.pos.xyz + vec3<f32>(quad_vertex.x * scale, quad_vertex.y * scale, 0.0);

    var out: VertexOutput;
    out.clip_position = mesh_view_bindings::view.view_proj * vec4<f32>(world_pos, 1.0);
    out.color = particle.color;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
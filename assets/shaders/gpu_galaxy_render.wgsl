// GPU Galaxy Rendering Shader
// Reads particles directly from storage buffer for high-performance instancing

#import bevy_pbr::mesh_view_bindings

struct Particle {
    pos: vec4<f32>,
    vel: vec4<f32>,
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

    // Color based on velocity magnitude and position
    let speed = length(particle.vel.xyz);
    let hue = atan2(particle.pos.z, particle.pos.x) / (2.0 * 3.14159) + 0.5;
    let saturation = min(1.0, speed * 0.1);
    let brightness = 0.5 + 0.5 * sin(distance * 0.1);

    // Simple HSV to RGB conversion
    let c = saturation * brightness;
    let x = c * (1.0 - abs((hue * 6.0) % 2.0 - 1.0));
    let m = brightness - c;

    var color: vec3<f32>;
    if (hue < 1.0/6.0) {
        color = vec3<f32>(c, x, 0.0);
    } else if (hue < 2.0/6.0) {
        color = vec3<f32>(x, c, 0.0);
    } else if (hue < 3.0/6.0) {
        color = vec3<f32>(0.0, c, x);
    } else if (hue < 4.0/6.0) {
        color = vec3<f32>(0.0, x, c);
    } else if (hue < 5.0/6.0) {
        color = vec3<f32>(x, 0.0, c);
    } else {
        color = vec3<f32>(c, 0.0, x);
    }
    color += vec3<f32>(m);

    var out: VertexOutput;
    out.clip_position = mesh_view_bindings::view.view_proj * vec4<f32>(world_pos, 1.0);
    out.color = vec4<f32>(color, 1.0);
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
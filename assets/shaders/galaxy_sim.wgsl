// Galaxy Simulation Compute Shader
// Phi-Galaxy physics with Golden Spiral + Z-Pinch

struct Particle {
    pos: vec4<f32>,
    vel: vec4<f32>,
};

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> uniforms: GalaxyUniforms;

struct GalaxyUniforms {
    time: f32,
    dt: f32,
    pinch_strength: f32,
};

const PHI: f32 = 1.6180339887498948482;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let id = global_id.x;
    if (id >= arrayLength(&particles)) {
        return;
    }

    var particle = particles[id];
    let pos = particle.pos.xyz;
    let r = length(pos);

    // Quasar Recycling: If too close to center, respawn and shoot up/down
    if (r < 2.0) {
        particle.pos = vec4<f32>(0.0, 0.0, 0.0, 1.0);
        // Random Y direction (±80.0)
        let y_vel = select(-80.0, 80.0, (id % 2u) == 0u);
        particle.vel = vec4<f32>(0.0, y_vel, 0.0, 0.0);
    } else {
        // Phi Rotation: Calculate ideal spiral angle
        let ideal_angle = log(r) * PHI + uniforms.time * 0.1;

        // Ideal position on the spiral arm
        let ideal_x = r * cos(ideal_angle);
        let ideal_z = r * sin(ideal_angle);
        let ideal_pos = vec3<f32>(ideal_x, 0.0, ideal_z);

        // Z-Pinch: Force towards the ideal spiral arm
        let pinch_vector = ideal_pos - pos;
        particle.vel.x += pinch_vector.x * uniforms.pinch_strength * uniforms.dt;
        particle.vel.y += pinch_vector.y * uniforms.pinch_strength * uniforms.dt;
        particle.vel.z += pinch_vector.z * uniforms.pinch_strength * uniforms.dt;

        // Orbital Drive: Tangent velocity for rotation
        // Calculate tangent direction (perpendicular to radius vector)
        let tangent = normalize(vec3<f32>(-pos.z, 0.0, pos.x));
        particle.vel.x += tangent.x * 50.0 * uniforms.dt;
        particle.vel.y += tangent.y * 50.0 * uniforms.dt;
        particle.vel.z += tangent.z * 50.0 * uniforms.dt;
    }

    // Update position
    particle.pos.x += particle.vel.x * uniforms.dt;
    particle.pos.y += particle.vel.y * uniforms.dt;
    particle.pos.z += particle.vel.z * uniforms.dt;

    // Store updated particle
    particles[id] = particle;
}
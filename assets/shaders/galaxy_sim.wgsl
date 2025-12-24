// Galaxy Simulation Compute Shader
// Phi-Galaxy physics with Golden Spiral + Z-Pinch

struct Particle {
    pos: vec4<f32>,
    vel: vec4<f32>,
    color: vec4<f32>,
};

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> uniforms: GalaxyUniforms;

struct GalaxyUniforms {
    time: f32,
    dt: f32,
    pinch_strength: f32,
    phi_value: f32,
    arms: f32,
};


@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let id = global_id.x;
    if (id >= arrayLength(&particles)) {
        return;
    }

    var particle = particles[id];
    let pos = particle.pos.xyz;
    let r = length(pos.xz);

    // Quasar Recycling: If too close to center, respawn and shoot up/down
    if (r < 2.0) {
        particle.pos = vec4<f32>(0.0, 0.0, 0.0, 1.0);
        // Random Y direction (±80.0)
        let y_vel = select(-80.0, 80.0, (id % 2u) == 0u);
        particle.vel = vec4<f32>(0.0, y_vel, 0.0, 0.0);
    } else {
        // --- 1. GEOMETRY ---
        let r = length(pos.xz);
        let current_theta = atan2(pos.z, pos.x);

        // --- 2. THE GOLDEN FREQUENCY ---
        // This defines the "Root" of the spiral
        let spiral_phase = log(r) * uniforms.phi_value;

        // --- 3. THE DYNAMO EFFECT ---
        // The Pinch is caused by the Current (Flow), not a constant.
        // I = v * density (We use v as proxy for I)
        let speed = length(particle.vel.xyz);

        // Fast particles create strong fields (Biot-Savart Law)
        let dynamo_strength = speed * uniforms.pinch_strength;

        // --- 4. THE STANDING WAVE POTENTIAL ---
        // Calculate the phase difference between particle and root
        let phase = current_theta - spiral_phase;

        // Apply the dynamic strength to the wave function
        let wave_force = -sin(uniforms.arms * phase) * dynamo_strength;

        // --- 4. APPLY FORCE TANGENTIALLY ---
        // We push along the tangent to sweep them into the arm
        let tangent = normalize(vec3<f32>(-pos.z, 0.0, pos.x));
        let geometric_force = tangent * wave_force;

        // --- 5. ETHER DRAG (Stability) ---
        let phi_drag = pow(uniforms.phi_value, -4.0);
        let drag_force = -particle.vel.xyz * phi_drag;

        // --- 6. INTEGRATE ---
        let total_force = geometric_force + drag_force;
        let new_vel = particle.vel.xyz + total_force * uniforms.dt;
        particle.vel = vec4<f32>(new_vel, particle.vel.w);
    }

    // Update position
    particle.pos.x += particle.vel.x * uniforms.dt;
    particle.pos.y += particle.vel.y * uniforms.dt;
    particle.pos.z += particle.vel.z * uniforms.dt;

    // Calculate color based on speed
    let speed = length(particle.vel.xyz);
    var color: vec3<f32>;
    if (speed < 20.0) {
        let t = speed / 20.0;
        color = mix(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(1.0, 1.0, 0.0), t); // Red to Yellow
    } else {
        let t = min((speed - 20.0) / 20.0, 1.0);
        color = mix(vec3<f32>(1.0, 1.0, 0.0), vec3<f32>(0.0, 1.0, 1.0), t); // Yellow to Cyan
    }
    particle.color = vec4<f32>(color, 1.0);

    // Store updated particle
    particles[id] = particle;
}
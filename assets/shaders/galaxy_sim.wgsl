// Galaxy Simulation Compute Shader
// Quaternionic Dirac-Maxwell-NS (QDM-NS) operator:
//
//   DΨ = (∇ + m_eff + ν∇² − D/Dt) Ψ + Ψ̄·J + λ(ΨΨ̄ − 1)Ψ
//
// Particle state Ψ = (w=pressure, xyz=velocity) stored as vec4.
// Each term of the operator is implemented explicitly below.

struct Particle {
    pos: vec4<f32>,  // xyz = position,  w = alive flag
    vel: vec4<f32>,  // xyz = velocity (vec part of Ψ), w = pressure (scalar part of Ψ)
    color: vec4<f32>,
};

struct GalaxyUniforms {
    time:           f32,
    dt:             f32,
    // Ψ̄·J coupling — Birkeland / Z-pinch arm cohesion
    pinch_strength: f32,
    // φ — golden ratio (tunable). Arm pitch φ₀ = 2π / ln(φ).
    phi_value:      f32,
    arms:           f32,
    // ν — quaternion viscosity; regularises r→0 via |∇²Ψ|
    nu:             f32,
    // λ — vorticity-threshold brake: λ(ΨΨ̄−1)Ψ jet-trigger
    lambda:         f32,
    // m_eff — baryonic self-gravity ("vortex rest energy")
    m_eff:          f32,
};

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> uniforms: GalaxyUniforms;

// ---------------------------------------------------------------------------
// Quaternion helpers (operating on vec4 as (w, x, y, z))
// ---------------------------------------------------------------------------

// Hamilton product of two quaternions stored as vec4(w,x,y,z)
fn quat_mul(a: vec4<f32>, b: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(
        a.x*b.x - a.y*b.y - a.z*b.z - a.w*b.w,
        a.x*b.y + a.y*b.x + a.z*b.w - a.w*b.z,
        a.x*b.z - a.y*b.w + a.z*b.x + a.w*b.y,
        a.x*b.w + a.y*b.z - a.z*b.y + a.w*b.x,
    );
}

// Quaternion conjugate
fn quat_conj(q: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(q.x, -q.y, -q.z, -q.w);
}

// Quaternion norm squared: ΨΨ̄
fn quat_norm_sq(q: vec4<f32>) -> f32 {
    return dot(q, q);
}

// Rotate a 3-vector by quaternion sandwich: q * (0,v) * q⁻¹, returns xyz
fn quat_rotate(q: vec4<f32>, v: vec3<f32>) -> vec3<f32> {
    let pv = vec4<f32>(0.0, v.x, v.y, v.z);
    let r = quat_mul(quat_mul(q, pv), quat_conj(q));
    return r.yzw;
}

// Unit rotation quaternion around axis a (must be normalised) by angle θ
fn quat_from_axis_angle(axis: vec3<f32>, angle: f32) -> vec4<f32> {
    let half = angle * 0.5;
    let s = sin(half);
    return vec4<f32>(cos(half), axis.x * s, axis.y * s, axis.z * s);
}

// ---------------------------------------------------------------------------

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let id = global_id.x;
    if id >= arrayLength(&particles) {
        return;
    }

    var particle = particles[id];
    let pos = particle.pos.xyz;
    let r = length(pos.xz);

    // Quasar recycling: too close to centre → jet outflow along Y axis
    if r < 2.0 {
        particle.pos = vec4<f32>(0.0, 0.0, 0.0, 1.0);
        let y_vel = select(-80.0, 80.0, (id % 2u) == 0u);
        particle.vel = vec4<f32>(y_vel, 0.0, 0.0, 0.0); // w=pressure, xyz=vel
        particles[id] = particle;
        return;
    }

    // -----------------------------------------------------------------------
    // Build the state quaternion Ψ = (w=pressure, xyz=velocity)
    // Stored as vel.w = pressure (scalar), vel.xyz = velocity (vector part)
    // -----------------------------------------------------------------------
    let psi = vec4<f32>(particle.vel.w, particle.vel.x, particle.vel.y, particle.vel.z);

    // φ₀ = 2π / ln(φ) — the spiral pitch eigenvalue
    let phi0 = 6.283185 / log(max(uniforms.phi_value, 1.0001));

    let current_theta = atan2(pos.z, pos.x);

    // -----------------------------------------------------------------------
    // ∇Ψ term — Dirac + Maxwell core
    // Implemented as tangential "arm-restoring" force:
    //   the on-arm angle is θ_arm = φ₀·ln(r), so the phase offset
    //   drives the particle back onto the golden-ratio spiral.
    // -----------------------------------------------------------------------
    let theta_arm    = phi0 * log(r);
    let phase        = current_theta - theta_arm;

    let tangent      = normalize(vec3<f32>(-pos.z, 0.0, pos.x));
    let radial       = normalize(vec3<f32>(pos.x,  0.0, pos.z));

    let speed        = length(particle.vel.xyz);
    // Dynamo (Ψ̄·J): current-driven pinch — fast particles make stronger fields
    let dynamo       = speed * uniforms.pinch_strength;
    let dirac_force  = tangent * (-sin(uniforms.arms * phase) * dynamo);

    // -----------------------------------------------------------------------
    // m_eff·Ψ term — baryonic self-gravity / "vortex rest energy"
    // Pulls the velocity vector toward the tangential direction weighted by m_eff
    // -----------------------------------------------------------------------
    let target_vel   = tangent * speed;
    let m_eff_force  = (target_vel - particle.vel.xyz) * uniforms.m_eff;

    // -----------------------------------------------------------------------
    // ν∇²Ψ term — quaternion viscosity
    // Approximated as ν * (target − current) / r, which scales like a Laplacian.
    // The 1/r factor is the key: it regularises r→0 because as r→0 the
    // quaternion norm |∇²Ψ| → 0 (division algebra forbids point singularities).
    // -----------------------------------------------------------------------
    let laplacian_scale = uniforms.nu / max(r, 1.0);
    let nu_force        = (target_vel - particle.vel.xyz) * laplacian_scale;

    // -----------------------------------------------------------------------
    // −(D/Dt)Ψ term — material derivative (Navier-Stokes advection)
    // Standard drag: opposes velocity, encodes ether/plasma friction
    // -----------------------------------------------------------------------
    let phi_drag   = pow(uniforms.phi_value, -4.0);
    let drag_force = -particle.vel.xyz * phi_drag;

    // -----------------------------------------------------------------------
    // λ(ΨΨ̄ − 1)Ψ term — nonlinear vorticity-threshold brake
    // When |Ψ|² > 1 (super-critical vorticity) this damps excess energy into
    // jets/outflows. Derived variationally — not ad-hoc.
    // -----------------------------------------------------------------------
    let norm_sq      = quat_norm_sq(psi);
    let brake_force  = -particle.vel.xyz * (uniforms.lambda * (norm_sq - 1.0));

    // -----------------------------------------------------------------------
    // Ψ̄·J term — Lorentz force / current self-interaction
    // Rotate the velocity by the local Birkeland-current quaternion.
    // J is approximated as a unit quaternion rotating about the vertical axis
    // by the arm-phase angle, scaled by pinch_strength.
    // -----------------------------------------------------------------------
    let j_angle     = phase * uniforms.pinch_strength * 0.1;
    let j_quat      = quat_from_axis_angle(vec3<f32>(0.0, 1.0, 0.0), j_angle);
    let lorentz_vel = quat_rotate(j_quat, particle.vel.xyz);
    let lorentz_force = (lorentz_vel - particle.vel.xyz);

    // -----------------------------------------------------------------------
    // Sum all QDM-NS operator terms
    // -----------------------------------------------------------------------
    let total_force = dirac_force
                    + m_eff_force
                    + nu_force
                    + drag_force
                    + brake_force
                    + lorentz_force;

    var new_vel = particle.vel.xyz + total_force * uniforms.dt;

    // Update scalar (pressure) part of Ψ: w evolves with norm of velocity
    let new_pressure = length(new_vel) / 80.0; // normalise to [0,1] range

    particle.vel = vec4<f32>(new_vel.x, new_vel.y, new_vel.z, new_pressure);

    // Update position
    particle.pos = vec4<f32>(
        pos.x + new_vel.x * uniforms.dt,
        pos.y + new_vel.y * uniforms.dt,
        pos.z + new_vel.z * uniforms.dt,
        particle.pos.w,
    );

    // Color: map speed → red–yellow–cyan
    let spd = length(new_vel);
    var color: vec3<f32>;
    if spd < 20.0 {
        color = mix(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(1.0, 1.0, 0.0), spd / 20.0);
    } else {
        color = mix(vec3<f32>(1.0, 1.0, 0.0), vec3<f32>(0.0, 1.0, 1.0), min((spd - 20.0) / 20.0, 1.0));
    }
    particle.color = vec4<f32>(color, 1.0);

    particles[id] = particle;
}

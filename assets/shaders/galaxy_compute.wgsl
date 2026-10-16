// GPU port of src/galaxy.rs update_particles and update_particles_legacy.
// Constants and the order of terms match the CPU integrator. Do not retune
// them here without changing the CPU path; the parity test compares the two.

struct Star {
    pos: vec4<f32>, // xyz, dust flag (1 = dust lane sprite)
    vel: vec4<f32>, // xyz velocity, w pressure hint
}

struct Sim {
    dt: f32,
    pattern_angle: f32,
    a0: f32,
    bhx: f32,
    bhy: f32,
    bhz: f32,
    flags: u32,
    n: u32,
    seed: u32,
    frame: u32,
    dust: u32,
    _pad: u32,
}

const FLAG_MOND: u32 = 1u;
const FLAG_VORTEX: u32 = 2u;
const FLAG_LEGACY: u32 = 4u;

const GM_CORE: f32 = 4000.0;
const EPS_CORE: f32 = 8.0;
const GM_BH: f32 = 360.0;
const EPS_BH: f32 = 20.0;
const BH_ACCEL_CAP: f32 = 8.0;
const BH_ABSORB_R: f32 = 15.0;
const OUTER_RADIUS: f32 = 260.0;
const DISK_THICKNESS: f32 = 5.0;
const MAX_SPEED: f32 = 80.0;
const GOLDEN_ANGLE: f32 = 2.399963;
const PINCH: f32 = 0.05;
const PINCH_ARM: f32 = 0.001;
const NU: f32 = 0.015;
const VERTICAL_K: f32 = 0.8;
const VERTICAL_K_VORTEX: f32 = 0.10;
const DAMPING_LEGACY: f32 = 0.06;
const DAMPING_RATE: f32 = 0.001;
const INFLOW_FRAC: f32 = 0.035;
const UPDRAFT: f32 = 3.5;
const EYEWALL_R: f32 = 34.0;
const Y_WALL: f32 = 36.0;
const E: f32 = 2.718281828;

@group(0) @binding(0) var<storage, read_write> stars: array<Star>;
@group(0) @binding(1) var<uniform> sim: Sim;

fn plummer_factor(r_sq: f32, gm: f32, eps: f32) -> f32 {
    let d = r_sq + eps * eps;
    return gm / (d * sqrt(d));
}

fn mond_boost(a_n: f32, a0: f32) -> f32 {
    if (a_n <= 1e-8 || a0 <= 0.0) {
        return 1.0;
    }
    let y = a0 / a_n;
    return 0.5 + sqrt(0.25 + y);
}

fn force_circular_speed(r_in: f32, mond: bool, legacy: bool) -> f32 {
    let r = max(r_in, 1e-3);
    let a_n = plummer_factor(r * r, GM_CORE, EPS_CORE) * r;
    var a_r = a_n;
    if (!legacy && mond) {
        a_r = a_n * mond_boost(a_n, sim.a0);
    }
    return sqrt(a_r * r);
}

fn eyewall_updraft(r: f32, y: f32) -> f32 {
    let x = max(r / EYEWALL_R, 0.0);
    let radial = x * exp(-x) * E;
    let vert = clamp(1.0 - (y / Y_WALL) * (y / Y_WALL), -2.0, 1.0);
    return UPDRAFT * radial * vert;
}

fn hash_u(x_in: u32) -> u32 {
    var x = x_in;
    x = x ^ (x >> 16u);
    x = x * 0x7feb352du;
    x = x ^ (x >> 15u);
    x = x * 0x846ca68bu;
    x = x ^ (x >> 16u);
    return x;
}

fn rand01(i: u32, salt: u32) -> f32 {
    let h = hash_u(i ^ (sim.seed * 0x9e3779b9u) ^ (sim.frame * 0x85ebca6bu) ^ (salt * 0xc2b2ae35u));
    return f32(h >> 8u) / 16777216.0;
}

fn rotate_y(v: vec3<f32>, angle: f32) -> vec3<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return vec3<f32>(v.x * c + v.z * s, v.y, -v.x * s + v.z * c);
}

fn respawn(s: ptr<function, Star>, i: u32, vortex: bool, legacy: bool, mond: bool) {
    let u = rand01(i, 1u);
    let sr = 30.0 + sqrt(u) * (OUTER_RADIUS * 0.85 - 30.0);
    var ao = 0.0;
    if (rand01(i, 2u) > 0.5) {
        ao = 3.14159265;
    }
    var st = GOLDEN_ANGLE * log(sr) + ao + (rand01(i, 3u) - 0.5) * 0.8;
    if (!legacy) {
        st = st + sim.pattern_angle;
    }
    (*s).pos.x = sr * cos(st);
    (*s).pos.z = sr * sin(st);
    var py = (rand01(i, 4u) - 0.5) * DISK_THICKNESS;
    var vy = 0.0;
    if (vortex && !legacy) {
        let h = 8.0 + 18.0 * (sr / OUTER_RADIUS);
        py = (rand01(i, 4u) - 0.5) * 2.0 * h;
        vy = (rand01(i, 5u) - 0.5) * 2.0 * h * sqrt(VERTICAL_K_VORTEX);
    }
    (*s).pos.y = py;
    let sv = min(force_circular_speed(sr, mond, legacy), MAX_SPEED);
    let stx = -(*s).pos.z / sr;
    let stz = (*s).pos.x / sr;
    var svx = stx * sv;
    var svz = stz * sv;
    if (vortex && !legacy) {
        let vr = -sv * INFLOW_FRAC;
        svx = svx + ((*s).pos.x / sr) * vr;
        svz = svz + ((*s).pos.z / sr) * vr;
    }
    (*s).vel = vec4<f32>(svx, vy, svz, clamp(sv / MAX_SPEED, 0.0, 1.0));
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= sim.n) {
        return;
    }
    var s = stars[i];
    let legacy = (sim.flags & FLAG_LEGACY) != 0u;
    let mond = (sim.flags & FLAG_MOND) != 0u;
    let vortex = (sim.flags & FLAG_VORTEX) != 0u;
    let dt = sim.dt;

    let r_xz = max(sqrt(s.pos.x * s.pos.x + s.pos.z * s.pos.z), 0.5);
    let r3d = max(sqrt(s.pos.x * s.pos.x + s.pos.y * s.pos.y + s.pos.z * s.pos.z), 0.5);

    let g_core = plummer_factor(r3d * r3d, GM_CORE, EPS_CORE);
    var ax = -s.pos.x * g_core;
    var ay = -s.pos.y * g_core;
    var az = -s.pos.z * g_core;

    let dx = sim.bhx - s.pos.x;
    let dy = sim.bhy - s.pos.y;
    let dz = sim.bhz - s.pos.z;
    let r_bh_sq = dx * dx + dy * dy + dz * dz;
    let r_bh = sqrt(r_bh_sq);
    var g_bh = plummer_factor(r_bh_sq, GM_BH, EPS_BH);

    if (legacy) {
        g_bh = min(g_bh, 8.0);
        ax = ax + dx * g_bh;
        ay = ay + dy * g_bh;
        az = az + dz * g_bh;
        ay = ay - s.pos.y * VERTICAL_K;

        let vc = min(sqrt(plummer_factor(r_xz * r_xz, GM_CORE, EPS_CORE) * r_xz * r_xz), MAX_SPEED);
        let tx = -s.pos.z / r_xz;
        let tz = s.pos.x / r_xz;
        ax = ax + (tx * vc - s.vel.x) * NU;
        az = az + (tz * vc - s.vel.z) * NU;

        let theta_arm = GOLDEN_ANGLE * log(r_xz);
        let phase = atan2(s.pos.z, s.pos.x) - theta_arm;
        let spd_now = max(sqrt(s.vel.x * s.vel.x + s.vel.z * s.vel.z), 1.0);
        let pinch = sin(phase) * spd_now * PINCH;
        ax = ax - tx * pinch;
        az = az - tz * pinch;

        let damp = 1.0 - DAMPING_LEGACY * dt;
        s.vel.x = s.vel.x * damp + ax * dt;
        s.vel.y = s.vel.y * damp + ay * dt;
        s.vel.z = s.vel.z * damp + az * dt;

        let r_pl = sqrt(r3d * r3d + EPS_CORE * EPS_CORE);
        let omega = min(sqrt(GM_CORE / max(r_pl * r_pl * r_pl, 1.0)), 3.5);
        let spun = rotate_y(s.vel.xyz, omega * dt * 0.06);
        s.vel.x = spun.x;
        s.vel.y = spun.y;
        s.vel.z = spun.z;

        var spd = length(s.vel.xyz);
        // The CPU keeps this pre-cap speed for the excess test and the escape test.
        let spd_for_tests = spd;
        if (spd > MAX_SPEED) {
            let sc = MAX_SPEED / spd;
            s.vel.x = s.vel.x * sc;
            s.vel.y = s.vel.y * sc;
            s.vel.z = s.vel.z * sc;
        }
        let excess = spd_for_tests - vc * 1.5;
        if (excess > 0.0) {
            let extra = 1.0 - (excess / MAX_SPEED) * 0.3 * dt;
            s.vel.x = s.vel.x * extra;
            s.vel.y = s.vel.y * extra;
            s.vel.z = s.vel.z * extra;
        }
        s.vel.w = clamp(vc / MAX_SPEED, 0.0, 1.0);
        s.pos.x = s.pos.x + s.vel.x * dt;
        s.pos.y = s.pos.y + s.vel.y * dt;
        s.pos.z = s.pos.z + s.vel.z * dt;

        let ex = s.pos.x - sim.bhx;
        let ey = s.pos.y - sim.bhy;
        let ez = s.pos.z - sim.bhz;
        let r_bh_final = sqrt(ex * ex + ey * ey + ez * ez);
        let r_final = length(s.pos.xyz);
        let local_vc = sqrt(plummer_factor(min(r_final, OUTER_RADIUS) * min(r_final, OUTER_RADIUS), GM_CORE, EPS_CORE) * min(r_final, OUTER_RADIUS) * min(r_final, OUTER_RADIUS));
        let escaped = r_final > OUTER_RADIUS * 0.80 && spd_for_tests > local_vc * 1.4;
        if (r_bh_final < BH_ABSORB_R || r_final > OUTER_RADIUS * 1.05 || escaped) {
            respawn(&s, i, false, true, false);
        }
    } else {
        let a_bh = g_bh * r_bh;
        if (a_bh > BH_ACCEL_CAP) {
            g_bh = BH_ACCEL_CAP / max(r_bh, 1e-3);
        }
        ax = ax + dx * g_bh;
        ay = ay + dy * g_bh;
        az = az + dz * g_bh;

        if (mond) {
            let a_n = sqrt(ax * ax + ay * ay + az * az);
            let boost = mond_boost(a_n, sim.a0);
            ax = ax * boost;
            ay = ay * boost;
            az = az * boost;
        }

        let k_vert = select(VERTICAL_K, VERTICAL_K_VORTEX, vortex);
        let vy_damp_scale = select(1.0, 0.25, vortex);
        ay = ay - s.pos.y * k_vert;
        if (vortex) {
            ay = ay + eyewall_updraft(r_xz, s.pos.y);
        }

        let vc_ref = min(force_circular_speed(r_xz, mond, false), MAX_SPEED);
        let tx = -s.pos.z / r_xz;
        let tz = s.pos.x / r_xz;
        if (vortex) {
            let a_in = INFLOW_FRAC * vc_ref * vc_ref / r_xz;
            ax = ax - (s.pos.x / r_xz) * a_in;
            az = az - (s.pos.z / r_xz) * a_in;
        }

        let theta_arm = GOLDEN_ANGLE * log(r_xz) + sim.pattern_angle;
        let phase = atan2(s.pos.z, s.pos.x) - theta_arm;
        let spd_now = max(sqrt(s.vel.x * s.vel.x + s.vel.z * s.vel.z), 1.0);
        let pinch = sin(2.0 * phase) * spd_now * PINCH_ARM;
        ax = ax - tx * pinch;
        az = az - tz * pinch;

        let damp_h = 1.0 - DAMPING_RATE * dt;
        let damp_v = 1.0 - DAMPING_RATE * vy_damp_scale * dt;
        s.vel.x = s.vel.x * damp_h + ax * dt;
        s.vel.y = s.vel.y * damp_v + ay * dt;
        s.vel.z = s.vel.z * damp_h + az * dt;

        var spd = length(s.vel.xyz);
        if (spd > MAX_SPEED) {
            let sc = MAX_SPEED / spd;
            s.vel.x = s.vel.x * sc;
            s.vel.y = s.vel.y * sc;
            s.vel.z = s.vel.z * sc;
            spd = MAX_SPEED;
        }
        let excess = spd - vc_ref * 1.5;
        if (excess > 0.0) {
            let extra = 1.0 - (excess / MAX_SPEED) * 0.3 * dt;
            s.vel.x = s.vel.x * extra;
            s.vel.y = s.vel.y * extra;
            s.vel.z = s.vel.z * extra;
            spd = spd * extra;
        }
        s.vel.w = clamp(vc_ref / MAX_SPEED, 0.0, 1.0);
        s.pos.x = s.pos.x + s.vel.x * dt;
        s.pos.y = s.pos.y + s.vel.y * dt;
        s.pos.z = s.pos.z + s.vel.z * dt;

        let ex = s.pos.x - sim.bhx;
        let ey = s.pos.y - sim.bhy;
        let ez = s.pos.z - sim.bhz;
        let r_bh_final = sqrt(ex * ex + ey * ey + ez * ez);
        let r_cyl = sqrt(s.pos.x * s.pos.x + s.pos.z * s.pos.z);
        let v_plane = sqrt(s.vel.x * s.vel.x + s.vel.z * s.vel.z);
        let local_vc = force_circular_speed(min(r_cyl, OUTER_RADIUS), mond, false);
        let escaped = r_cyl > OUTER_RADIUS * 1.02 && v_plane > local_vc * 1.8;
        if (r_bh_final < BH_ABSORB_R || r_cyl > OUTER_RADIUS * 1.08 || escaped) {
            respawn(&s, i, vortex, false, mond);
        }
    }

    stars[i] = s;
}

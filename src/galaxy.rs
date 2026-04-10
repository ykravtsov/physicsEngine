//! Galaxy simulation — QDM-NS vortex field theory
//!
//! State Ψ = ψ + F + v ∈ H(C) stored per particle as FluxQuaternion:
//!   w = pressure / ether density (scalar part)
//!   xyz = velocity (vector part of Ψ)
//!
//! Operator applied each frame:
//!   DΨ = (∇ + m_eff + ν∇² − D/Dt) Ψ + Ψ̄·J + λ(ΨΨ̄ − 1)Ψ

use crate::hurricane_3d::Particle;
use crate::quaternion::math::FluxQuaternion;
use rand::Rng;

// ── physical constants ─────────────────────────────────────────────────────

/// φ₀ = 2π/ln(φ) ≈ 2.4 rad — spiral pitch eigenvalue
const PHI0: f32 = 2.399_963;

/// Gravitational constant of the galactic core (Plummer sphere)
const GM_CORE: f32 = 4_000.0;
/// Plummer softening radius ε for the core
const EPS_CORE: f32 = 8.0;

/// Gravitational constant of the wandering black hole
const GM_BH: f32 = 360.0;
/// Plummer softening radius ε for the BH (large → very gentle tides)
const EPS_BH: f32 = 20.0;

/// Ether (space fluid) drag on the BH — causes slow inspiral
const BH_DRAG: f32 = 0.004;

/// Arm-restoring (∇Ψ / Ψ̄·J) pinch strength relative to local speed
const PINCH: f32 = 0.05;
/// ν — viscosity strength pulling stars toward Keplerian orbit
const NU: f32 = 0.015;
/// Small vertical restoring force (keeps disk thin)
const VERTICAL_K: f32 = 0.8;

/// Radius at which a star is considered swallowed by the BH.
/// Must be large enough to consume stars that get gravitationally trapped
/// before they can pile up into a permanent cluster around the BH.
const BH_ABSORB_R: f32 = 15.0;
/// Radius of visible accretion heating around the BH
const BH_GLOW_R: f32 = 28.0;

const OUTER_RADIUS: f32 = 260.0;
const DISK_THICKNESS: f32 = 5.0;
const MAX_SPEED: f32 = 80.0;

// ── helpers ────────────────────────────────────────────────────────────────

/// Plummer circular speed at cylindrical radius r_xz.
/// v_c = sqrt( GM * r² / (r² + ε²)^(3/2) )
fn plummer_circular_speed(r: f32, gm: f32, eps: f32) -> f32 {
    let r2 = r * r;
    let eps2 = eps * eps;
    let denom = (r2 + eps2).powf(1.5);
    (gm * r2 / denom).sqrt()
}

/// Plummer acceleration magnitude: a = GM / (r² + ε²)
/// Returns the scalar factor; caller multiplies by displacement vector.
fn plummer_accel_factor(r_sq: f32, gm: f32, eps: f32) -> f32 {
    gm / (r_sq + eps * eps).powf(1.5)
}

// ── types ──────────────────────────────────────────────────────────────────

pub struct BlackHole {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub vx: f32,
    pub vy: f32,
    pub vz: f32,
}

pub struct GalaxySimulation {
    pub particles: Vec<Particle>,
    pub black_hole: BlackHole,
    frame: u64,
    absorbed_total: u64,
}

// ── initialisation ─────────────────────────────────────────────────────────

impl GalaxySimulation {
    pub fn new(count: usize) -> Self {
        let mut rng = rand::thread_rng();
        let mut particles = Vec::with_capacity(count);

        for i in 0..count {
            // Uniform-in-area distribution: r ∝ sqrt(u) gives flat disk
            let u: f32 = rng.r#gen::<f32>();
            let r = 8.0 + u.sqrt() * (OUTER_RADIUS * 0.88 - 8.0);

            // 80% of particles follow the golden-ratio arm; 20% are background disk
            let theta = if i % 5 != 0 {
                // On-arm: θ = φ₀·ln r ± fuzz
                let arm_offset = if i % 2 == 0 {
                    0.0_f32
                } else {
                    std::f32::consts::PI
                };
                let fuzz = (rng.r#gen::<f32>() - 0.5) * 0.7;
                PHI0 * r.ln() + arm_offset + fuzz
            } else {
                // Background disk: fully random angle fills inter-arm voids
                rng.r#gen::<f32>() * std::f32::consts::TAU
            };

            let px = r * theta.cos();
            let pz = r * theta.sin();
            let py = (rng.r#gen::<f32>() - 0.5) * DISK_THICKNESS;

            // Correct circular speed from Plummer potential
            let vc = plummer_circular_speed(r, GM_CORE, EPS_CORE).min(MAX_SPEED);
            let tx = -pz / r;
            let tz = px / r;

            // Velocity quaternion: w = ether pressure, xyz = velocity
            let pressure = (vc / MAX_SPEED).clamp(0.0, 1.0);
            let flux = FluxQuaternion::new(pressure, tx * vc, 0.0, tz * vc);

            // Color: core gold → arm cyan → outer deep blue
            let t = (r / OUTER_RADIUS).clamp(0.0, 1.0);
            let (cr, cg, cb, ca) = if r < 18.0 {
                (1.0, 0.95, 0.80, 1.0)
            } else if r < 60.0 {
                let s = (r - 18.0) / 42.0;
                (1.0 - s * 0.65, 0.85 + s * 0.10, 0.3 + s * 0.65, 0.95)
            } else {
                (
                    0.20 + (1.0 - t) * 0.25,
                    0.30 + (1.0 - t) * 0.30,
                    0.90 + t * 0.10,
                    (0.85 - t * 0.65).max(0.05),
                )
            };

            particles.push(Particle {
                x: px,
                y: py,
                z: pz,
                flux,
                r: cr,
                g: cg,
                b: cb,
                a: ca,
                size: if r < 14.0 { 2.0 } else { 1.0 },
            });
        }

        // BH starts BETWEEN arms (offset π/2 from arm angle) so it begins
        // in a low-density void and gradually sweeps into the arm — this
        // prevents the initial burst of thousands of interactions.
        let bh_r: f32 = 120.0;
        let bh_theta = PHI0 * bh_r.ln() + std::f32::consts::FRAC_PI_2; // mid inter-arm
        let bh_vc = plummer_circular_speed(bh_r, GM_CORE, EPS_CORE) * 0.90;
        let bh_cos = bh_theta.cos();
        let bh_sin = bh_theta.sin();
        // Tangential velocity (counter-clockwise)
        let black_hole = BlackHole {
            x: bh_r * bh_cos,
            y: 4.0,
            z: bh_r * bh_sin,
            vx: -bh_sin * bh_vc,
            vy: 0.0,
            vz: bh_cos * bh_vc,
        };

        Self {
            particles,
            black_hole,
            frame: 0,
            absorbed_total: 0,
        }
    }
}

// ── per-frame update ───────────────────────────────────────────────────────

impl GalaxySimulation {
    pub fn update(&mut self, dt: f32) {
        let dt = dt.min(0.033);
        let sub_dt = dt / 2.0;
        for _ in 0..2 {
            self.step(sub_dt);
        }
        self.frame += 1;

        // ── Diagnostic output every 120 frames (~2 s at 60 fps) ────────────
        if self.frame % 300 == 0 {
            self.print_diagnostics(dt);
        }
    }

    fn print_diagnostics(&self, dt: f32) {
        let bh = &self.black_hole;
        let bh_r = (bh.x * bh.x + bh.y * bh.y + bh.z * bh.z).sqrt();
        let bh_spd = (bh.vx * bh.vx + bh.vy * bh.vy + bh.vz * bh.vz).sqrt();

        // Per-particle stats
        let mut spd_sum = 0.0f32;
        let mut spd_max = 0.0f32;
        let mut nan_count = 0u32;
        let mut r_max = 0.0f32;
        let mut r_min = f32::MAX;
        let mut near_bh = 0u32;

        for p in &self.particles {
            let spd = (p.flux.x * p.flux.x + p.flux.y * p.flux.y + p.flux.z * p.flux.z).sqrt();
            let r = (p.x * p.x + p.y * p.y + p.z * p.z).sqrt();
            let dbh = {
                let ex = p.x - bh.x;
                let ey = p.y - bh.y;
                let ez = p.z - bh.z;
                (ex * ex + ey * ey + ez * ez).sqrt()
            };

            if spd.is_nan() || p.x.is_nan() {
                nan_count += 1;
                continue;
            }
            spd_sum += spd;
            if spd > spd_max {
                spd_max = spd;
            }
            if r > r_max {
                r_max = r;
            }
            if r < r_min {
                r_min = r;
            }
            if dbh < BH_GLOW_R {
                near_bh += 1;
            }
        }

        let n = self.particles.len() as f32;
        let avg_spd = spd_sum / n;

        println!(
            "[frame {:>6}  dt={:.4}]\n  \
             BH  pos=({:>7.1},{:>6.1},{:>7.1})  r={:>7.1}  spd={:>6.1}\n  \
             Stars  avg_spd={:>5.1}  max_spd={:>5.1}  r=[{:>5.1}..{:>6.1}]  \
             near_bh={:>4}  absorbed_total={}\n  \
             NaNs={}{}",
            self.frame,
            dt,
            bh.x,
            bh.y,
            bh.z,
            bh_r,
            bh_spd,
            avg_spd,
            spd_max,
            r_min,
            r_max,
            near_bh,
            self.absorbed_total,
            nan_count,
            if nan_count > 0 {
                "  *** NaN DETECTED ***"
            } else {
                ""
            },
        );
    }

    fn step(&mut self, dt: f32) {
        self.update_black_hole(dt);
        let (bhx, bhy, bhz) = (self.black_hole.x, self.black_hole.y, self.black_hole.z);
        let absorbed = self.update_particles(dt, bhx, bhy, bhz);
        self.absorbed_total += absorbed;
    }

    fn update_black_hole(&mut self, dt: f32) {
        let bh = &mut self.black_hole;

        // Core gravity on BH — Plummer
        let r2 = bh.x * bh.x + bh.y * bh.y + bh.z * bh.z;
        let g = plummer_accel_factor(r2, GM_CORE, EPS_CORE);
        bh.vx += -bh.x * g * dt;
        bh.vy += -bh.y * g * dt;
        bh.vz += -bh.z * g * dt;

        // Ether drag (space-fluid resistance → inspiral)
        let spd = (bh.vx * bh.vx + bh.vy * bh.vy + bh.vz * bh.vz)
            .sqrt()
            .max(1e-6);
        let drag = BH_DRAG * spd; // linear Stokes drag: F = -ν·v
        bh.vx -= bh.vx / spd * drag * dt;
        bh.vy -= bh.vy / spd * drag * dt;
        bh.vz -= bh.vz / spd * drag * dt;

        bh.x += bh.vx * dt;
        bh.y += bh.vy * dt;
        bh.z += bh.vz * dt;
    }

    fn update_particles(&mut self, dt: f32, bhx: f32, bhy: f32, bhz: f32) -> u64 {
        let mut rng = rand::thread_rng();
        let mut absorbed = 0u64;

        for p in &mut self.particles {
            let r_xz = (p.x * p.x + p.z * p.z).sqrt().max(0.5);
            let r3d = (p.x * p.x + p.y * p.y + p.z * p.z).sqrt().max(0.5);

            // ── 1. Core gravity (Plummer) ──────────────────────────────────
            let g_core = plummer_accel_factor(r3d * r3d, GM_CORE, EPS_CORE);
            let ax = -p.x * g_core;
            let ay = -p.y * g_core;
            let az = -p.z * g_core;

            // ── 2. BH gravity (Plummer) ────────────────────────────────────
            let dx = bhx - p.x;
            let dy = bhy - p.y;
            let dz = bhz - p.z;
            let r_bh_sq = dx * dx + dy * dy + dz * dz;
            let g_bh = plummer_accel_factor(r_bh_sq, GM_BH, EPS_BH);
            // Hard acceleration cap so no particle ever gets a huge kick
            let g_bh = g_bh.min(8.0);
            let ax = ax + dx * g_bh;
            let ay = ay + dy * g_bh;
            let az = az + dz * g_bh;

            // ── 3. Vertical restoring (thin disk) ─────────────────────────
            let ay = ay - p.y * VERTICAL_K;

            // ── 4. ν∇²Ψ viscosity — nudge toward local Keplerian orbit ────
            // Target = correct circular speed for this r (Plummer)
            let vc = plummer_circular_speed(r_xz, GM_CORE, EPS_CORE).min(MAX_SPEED);
            let tx = -p.z / r_xz;
            let tz = p.x / r_xz;
            let tvx = tx * vc;
            let tvz = tz * vc;
            let ax = ax + (tvx - p.flux.x) * NU;
            let az = az + (tvz - p.flux.z) * NU;

            // ── 5. ∇Ψ arm-restoring force (Ψ̄·J Lorentz pinch) ─────────────
            // Pushes particle tangentially toward the golden-spiral arm
            let theta_arm = PHI0 * r_xz.ln();
            let phase = p.z.atan2(p.x) - theta_arm;
            let spd_now = (p.flux.x * p.flux.x + p.flux.z * p.flux.z).sqrt().max(1.0);
            let pinch = phase.sin() * spd_now * PINCH;
            let ax = ax - tx * pinch;
            let az = az - tz * pinch;

            // ── 6. Integrate velocity ──────────────────────────────────────
            // Gentle system-wide damping (0.3 %/s) prevents slow energy drift
            // where particles gradually spiral outward from BH interactions.
            let damp = 1.0 - 0.06 * dt;
            p.flux.x = p.flux.x * damp + ax * dt;
            p.flux.y = p.flux.y * damp + ay * dt;
            p.flux.z = p.flux.z * damp + az * dt;

            // ── 7. Ψ̄·J Coriolis — quaternion sandwich ─────────────────────
            // Differential rotation: ω_Kepler from Plummer potential, capped
            let r_pl = (r3d * r3d + EPS_CORE * EPS_CORE).sqrt();
            let omega = (GM_CORE / (r_pl * r_pl * r_pl).max(1.0)).sqrt().min(3.5);
            let cq = FluxQuaternion::from_axis_angle(0.0, 1.0, 0.0, omega * dt * 0.06);
            let (rvx, rvy, rvz) = cq.rotate_vec3(p.flux.x, p.flux.y, p.flux.z);
            p.flux.x = rvx;
            p.flux.y = rvy;
            p.flux.z = rvz;

            // ── 8. Speed cap + progressive high-speed drag ────────────────
            let spd = (p.flux.x * p.flux.x + p.flux.y * p.flux.y + p.flux.z * p.flux.z).sqrt();
            // Hard cap
            if spd > MAX_SPEED {
                let s = MAX_SPEED / spd;
                p.flux.x *= s;
                p.flux.y *= s;
                p.flux.z *= s;
            }
            // Extra drag on fast outliers: kick in above 1.5× local Keplerian
            // This catches BH-slingshot stars before they escape to r_max drift.
            let excess = spd - vc * 1.5;
            if excess > 0.0 {
                let extra_damp = 1.0 - (excess / MAX_SPEED) * 0.3 * dt;
                p.flux.x *= extra_damp;
                p.flux.y *= extra_damp;
                p.flux.z *= extra_damp;
            }

            // Update scalar pressure w = vc / MAX_SPEED (ether density)
            p.flux.w = (vc / MAX_SPEED).clamp(0.0, 1.0);

            // ── 9. Integrate position ──────────────────────────────────────
            p.x += p.flux.x * dt;
            p.y += p.flux.y * dt;
            p.z += p.flux.z * dt;

            // ── 10. Respawn ────────────────────────────────────────────────
            let r_bh_final = {
                let ex = p.x - bhx;
                let ey = p.y - bhy;
                let ez = p.z - bhz;
                (ex * ex + ey * ey + ez * ez).sqrt()
            };
            let r_final = (p.x * p.x + p.y * p.y + p.z * p.z).sqrt();

            // Catch BH-slingshot escapees: any star beyond 80% of the disk
            // moving faster than 1.4× local Keplerian is on an escape trajectory.
            let local_vc = plummer_circular_speed(r_final.min(OUTER_RADIUS), GM_CORE, EPS_CORE);
            let escaped = r_final > OUTER_RADIUS * 0.80 && spd > local_vc * 1.4;

            if r_bh_final < BH_ABSORB_R || r_final > OUTER_RADIUS * 1.05 || escaped {
                absorbed += 1;
                let u: f32 = rng.r#gen::<f32>();
                let sr = 30.0 + u.sqrt() * (OUTER_RADIUS * 0.85 - 30.0);
                let ao = if rng.r#gen::<bool>() {
                    0.0_f32
                } else {
                    std::f32::consts::PI
                };
                let st = PHI0 * sr.ln() + ao + (rng.r#gen::<f32>() - 0.5) * 0.8;
                p.x = sr * st.cos();
                p.y = (rng.r#gen::<f32>() - 0.5) * DISK_THICKNESS;
                p.z = sr * st.sin();
                let sv = plummer_circular_speed(sr, GM_CORE, EPS_CORE).min(MAX_SPEED);
                let stx = -p.z / sr;
                let stz = p.x / sr;
                let pres = (sv / MAX_SPEED).clamp(0.0, 1.0);
                p.flux = FluxQuaternion::new(pres, stx * sv, 0.0, stz * sv);
            }

            // ── 11. Color ──────────────────────────────────────────────────
            let energy = (spd / MAX_SPEED).clamp(0.0, 1.0);
            let bh_prox = (1.0 - (r_bh_final / BH_GLOW_R).clamp(0.0, 1.0)).max(0.0);
            let r_new_xz = (p.x * p.x + p.z * p.z).sqrt();
            let t = (r_new_xz / OUTER_RADIUS).clamp(0.0, 1.0);

            if r_bh_final < BH_GLOW_R {
                // Accretion disk — orange→white heat
                p.r = 1.0;
                p.g = 0.50 + bh_prox * 0.50;
                p.b = bh_prox * 0.30;
                p.a = 0.55 + bh_prox * 0.45;
            } else if r_new_xz < 18.0 {
                // Galactic core — bright gold-white
                p.r = 1.0;
                p.g = 0.95;
                p.b = 0.80;
                p.a = 1.0;
            } else {
                // Spiral arms — blue-cyan, brighter where faster
                p.r = (0.15 + energy * 0.55 + (1.0 - t) * 0.20).clamp(0.0, 1.0);
                p.g = (0.30 + energy * 0.30 + (1.0 - t) * 0.20).clamp(0.0, 1.0);
                p.b = (0.88 + t * 0.12).clamp(0.0, 1.0);
                p.a = (0.88 - t * 0.65).max(0.04);
            }
        }

        absorbed
    }
}

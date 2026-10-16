//! Galaxy simulation — QDM-NS vortex field theory
//!
//! State Ψ = ψ + F + v ∈ H(C) stored per particle as FluxQuaternion:
//!   w = pressure / ether density (scalar part)
//!   xyz = velocity (vector part of Ψ)
//!
//! Operator applied each frame:
//!   DΨ = (∇ + m_eff + ν∇² − D/Dt) Ψ + Ψ̄·J + λ(ΨΨ̄ − 1)Ψ
//!
//! `GalaxyConfig::legacy` reproduces the force law from before the vortex/MOND
//! flags. The default config fixes the two-arm phase and the 6%/s damping,
//! and (flagged) applies a simple MOND boost to the total Newtonian
//! acceleration and gives the disk a vertical vortex. Tangential speed is
//! not relaxed toward a prescribed v_c(r); it comes from the accelerations.

use crate::quaternion::math::FluxQuaternion;

/// One star. `flux.w` is a pressure hint; `flux.xyz` is velocity.
/// Color is computed in the sprite shader, not stored on the particle.
#[derive(Clone, Copy)]
pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub flux: FluxQuaternion,
}
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

// ── physical constants ─────────────────────────────────────────────────────

/// Arm winding coefficient b in the logarithmic spiral θ = b·ln r  (r = r₀·e^(θ/b)).
/// The value is the golden angle 2π/φ² ≈ 2.39996 rad (137.5°), NOT 2π/ln φ (≈ 13.06 rad).
/// With b = 2π/φ² the radius grows by e^(φ²) ≈ 13.7× per full turn and the
/// pitch angle is atan(1/b) ≈ 22.6°. For comparison:
///   - growth by φ per full turn:    b = 2π/ln φ    ≈ 13.06, pitch = atan(ln φ/2π)  ≈ 4.38°
///   - growth by φ per quarter turn: b = π/(2 ln φ) ≈ 3.26,  pitch = atan(2 ln φ/π) ≈ 17.03°
///     (the classic "golden spiral")
const GOLDEN_ANGLE: f32 = 2.399_963;

/// Gravitational parameter GM of the galactic core (Plummer sphere), code units
const GM_CORE: f32 = 4_000.0;
/// Plummer softening radius ε for the core
const EPS_CORE: f32 = 8.0;

/// Gravitational parameter GM of the wandering black hole, code units
const GM_BH: f32 = 360.0;
/// Plummer softening radius ε for the BH (large → very gentle tides)
const EPS_BH: f32 = 20.0;

/// Ether (space fluid) drag on the BH — causes slow inspiral
const BH_DRAG: f32 = 0.004;

/// Arm-restoring (∇Ψ / Ψ̄·J) pinch strength relative to local speed.
/// Legacy value. Only the phase = 2πk arm is stable under sin(phase); see review.
const PINCH: f32 = 0.05;
/// Two-arm pinch (1/s). sin(2·phase) is restoring at both arms.
///
/// This accelerates a star toward the arm angle, which torques the disk
/// toward the pattern speed Ω_pattern·r (solid body). There is no velocity
/// slave pulling v_θ back onto the circular-speed curve, so the coefficient
/// has to stay small or that torque is the rotation curve. 0.02/s moved
/// v(195)/v(52.5) from 0.53 to 0.82 in 15 s on a Newtonian disk.
const PINCH_ARM: f32 = 0.001;
/// ν — legacy viscosity strength pulling stars toward Keplerian orbit (1/s).
const NU: f32 = 0.015;
/// Legacy vertical spring (1/s²). With 6%/s damping this collapses the disk.
const VERTICAL_K: f32 = 0.8;
/// Softer spring so a flared disk can keep its scale height (1/s²).
const VERTICAL_K_VORTEX: f32 = 0.10;
/// Legacy system-wide velocity damping (1/s). Four times NU, so v_eq ≈ 0.2 v_c.
const DAMPING_LEGACY: f32 = 0.06;
/// In-plane drag (1/s) on the non-legacy path. 0.1% per second, so over a
/// 40 s run it removes about 4% of the speed. It is not what sets v(r).
const DAMPING_RATE: f32 = 0.001;
/// Rigid pattern speed of the two-arm density wave (rad/s).
/// Near the circular frequency around r ≈ 100–130 for a flat v ≈ 9, so corotation
/// sits in the disk instead of torquing the whole curve toward solid-body rotation.
const PATTERN_OMEGA: f32 = 0.07;
/// Fraction of circular speed applied as inward radial flow (vortex streamline).
const INFLOW_FRAC: f32 = 0.035;
/// Peak eyewall updraft acceleration (code units).
const UPDRAFT: f32 = 3.5;
/// Cylindrical radius where the eyewall updraft peaks.
const EYEWALL_R: f32 = 34.0;
/// Vertical scale of the updraft fountain.
const Y_WALL: f32 = 36.0;
/// Cap on |a| from the black hole. The old code capped the Plummer *factor*
/// (max ≈ 0.045) by 8, so the cap never fired.
const BH_ACCEL_CAP: f32 = 8.0;

/// Radius at which a star is considered swallowed by the BH.
/// Must be large enough to consume stars that get gravitationally trapped
/// before they can pile up into a permanent cluster around the BH.
const BH_ABSORB_R: f32 = 15.0;
/// Radius of visible accretion heating around the BH
const BH_GLOW_R: f32 = 28.0;

const OUTER_RADIUS: f32 = 260.0;
const DISK_THICKNESS: f32 = 5.0;
const MAX_SPEED: f32 = 80.0;

/// Default MOND acceleration scale in code units.
/// Deep-MOND asymptote v_∞ = (GM · a0)^(1/4) ≈ 8.0 for GM = 4000, a0 = 1.
/// This is the code-unit analogue of Milgrom's a0, not the SI value 1.2e-10 m/s².
pub const DEFAULT_A0: f32 = 1.0;

// ── helpers ────────────────────────────────────────────────────────────────

/// Plummer circular speed at cylindrical radius r_xz.
/// v_c = sqrt( GM * r² / (r² + ε²)^(3/2) )
fn plummer_circular_speed(r: f32, gm: f32, eps: f32) -> f32 {
    let r2 = r * r;
    let eps2 = eps * eps;
    let denom = (r2 + eps2).powf(1.5);
    (gm * r2 / denom).sqrt()
}

/// Plummer acceleration factor GM / (r² + ε²)^(3/2).
/// Caller multiplies by the displacement vector, so |a| = GM·r / (r² + ε²)^(3/2).
fn plummer_accel_factor(r_sq: f32, gm: f32, eps: f32) -> f32 {
    gm / (r_sq + eps * eps).powf(1.5)
}

/// Simple MOND interpolating function.
/// μ(x) = x/(1+x) with x = a/a0 solves to
/// a = a_N · (1/2 + sqrt(1/4 + a0/a_N)).
/// Returns the boost a/a_N. Deep MOND (a_N ≪ a0) → sqrt(a0/a_N), so a → sqrt(a_N·a0)
/// and v⁴ → GM·a0. Newtonian limit (a_N ≫ a0) → 1.
fn mond_boost(a_n: f32, a0: f32) -> f32 {
    if a_n <= 1e-8 || a0 <= 0.0 {
        return 1.0;
    }
    let y = a0 / a_n;
    0.5 + (0.25 + y).sqrt()
}

fn newton_circular_speed(r: f32) -> f32 {
    plummer_circular_speed(r.max(0.0), GM_CORE, EPS_CORE)
}

fn mond_circular_speed(r: f32, a0: f32) -> f32 {
    let vc = newton_circular_speed(r);
    if r < 1e-3 {
        return vc;
    }
    let a_n = vc * vc / r;
    (vc * vc * mond_boost(a_n, a0)).sqrt()
}

/// Midplane circular speed `sqrt(a_r · r)` of the core Plummer field.
///
/// When MOND is on, `a_r` is the core's Newtonian acceleration times the same
/// simple-μ boost the integrator applies to `|a_core + a_BH|`. The black hole
/// is left out of this number: it is not centered, so it has no v_c(r).
/// Callers use it as an initial / respawn condition and as a reference for
/// the high-speed drag. The integrator does not relax `v` toward it.
fn force_circular_speed(r: f32, config: &GalaxyConfig) -> f32 {
    let r = r.max(1e-3);
    // |a_N| of the core at cylindrical r, in the midplane (3D radius = r).
    let a_n = plummer_accel_factor(r * r, GM_CORE, EPS_CORE) * r;
    let a_r = if !config.legacy && config.mond {
        a_n * mond_boost(a_n, config.a0)
    } else {
        a_n
    };
    (a_r * r).sqrt()
}

/// Eyewall fountain. Peaks at `EYEWALL_R` and pushes |y| out to about `Y_WALL`,
/// then the (1 - (y/Y)²) factor and the vertical spring turn the flow back.
fn eyewall_updraft(r: f32, y: f32) -> f32 {
    let x = (r / EYEWALL_R).max(0.0);
    let radial = x * (-x).exp() * std::f32::consts::E;
    let vert = (1.0 - (y / Y_WALL) * (y / Y_WALL)).clamp(-2.0, 1.0);
    UPDRAFT * radial * vert
}

// ── types ──────────────────────────────────────────────────────────────────

/// Switches for model changes. Bug fixes (two-arm phase, damping/viscosity
/// balance, BH acceleration cap applied to |a|) are on whenever `legacy` is false.
#[derive(Clone, Copy, Debug)]
pub struct GalaxyConfig {
    /// Reproduce the pre-fix force law and the razor-thin initial disk.
    pub legacy: bool,
    /// Multiply `|a_core + a_BH|` by the simple MOND μ boost.
    ///
    /// `a_core` is the Plummer acceleration at the star's 3D distance from the
    /// origin. `a_BH` is the Plummer acceleration at the 3D separation from
    /// the black hole. The spring and the updraft are applied after the boost
    /// and are not part of `a_N`. Ignored when `legacy` is set.
    pub mond: bool,
    /// MOND scale a0 in code units. Ignored when `mond` is false or `legacy` is true.
    pub a0: f32,
    /// Flared disk, log-spiral inflow, eyewall updraft, rotating two-arm pattern.
    pub vortex: bool,
}

impl Default for GalaxyConfig {
    fn default() -> Self {
        Self {
            legacy: false,
            mond: true,
            a0: DEFAULT_A0,
            vortex: true,
        }
    }
}

impl GalaxyConfig {
    /// Parse `--legacy`, `--mond`, `--no-mond`, `--vortex`, `--no-vortex`, `--a0 <float>`.
    /// Unknown flags are ignored so headless can share this parser.
    pub fn from_args(args: &[String]) -> Self {
        let mut c = Self::default();
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "--legacy" => c.legacy = true,
                "--mond" => c.mond = true,
                "--no-mond" => c.mond = false,
                "--vortex" => c.vortex = true,
                "--no-vortex" => c.vortex = false,
                "--a0" => {
                    if let Some(v) = args.get(i + 1).and_then(|s| s.parse::<f32>().ok()) {
                        i += 1;
                        c.a0 = v.max(0.0);
                    }
                }
                _ => {}
            }
            i += 1;
        }
        c
    }

    pub fn label(&self) -> String {
        if self.legacy {
            "legacy".to_string()
        } else {
            format!(
                "mond={} a0={:.4} vortex={}",
                self.mond as u8, self.a0, self.vortex as u8
            )
        }
    }
}

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
    pub config: GalaxyConfig,
    /// Print the every-300-frame text diagnostic (the interactive run).
    pub verbose: bool,
    frame: u64,
    absorbed_total: u64,
    time: f32,
    /// ∫ Ω_pattern dt. The arm locus is θ = b ln r + pattern_angle (+ π for arm 2).
    pattern_angle: f32,
    /// Initialisation and respawn. Seeded from `--seed` so a headless run
    /// repeats, including which stars are reborn.
    rng: StdRng,
}

#[derive(Clone, Debug)]
pub struct RadialBin {
    pub r_mid: f32,
    pub count: u32,
    pub v_theta: f32,
    pub v_rad: f32,
    pub rms_y: f32,
    pub v_newton: f32,
    pub v_mond: f32,
}

#[derive(Clone, Debug)]
pub struct SnapshotStats {
    pub t: f32,
    pub rms_y: f32,
    pub mean_abs_y: f32,
    pub p90_abs_y: f32,
    pub arm_contrast: f32,
    pub bh_r: f32,
    pub absorbed: u64,
    pub n: usize,
    pub bins: Vec<RadialBin>,
}

// ── initialisation ─────────────────────────────────────────────────────────

impl GalaxySimulation {
    pub fn with_config(count: usize, config: GalaxyConfig, seed: Option<u64>) -> Self {
        let rng = match seed {
            Some(s) => StdRng::seed_from_u64(s),
            None => StdRng::from_entropy(),
        };
        Self::from_rng(count, config, rng)
    }

    fn from_rng(count: usize, config: GalaxyConfig, mut rng: StdRng) -> Self {
        let mut particles = Vec::with_capacity(count);
        let vortex_init = config.vortex && !config.legacy;

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
                GOLDEN_ANGLE * r.ln() + arm_offset + fuzz
            } else {
                // Background disk: fully random angle fills inter-arm voids
                rng.r#gen::<f32>() * std::f32::consts::TAU
            };

            let px = r * theta.cos();
            let pz = r * theta.sin();
            let (py, vy) = if vortex_init {
                // Flare: the outer disk is thicker, like an HI layer / hurricane shield.
                let h = 8.0 + 18.0 * (r / OUTER_RADIUS);
                let py = (rng.r#gen::<f32>() - 0.5) * 2.0 * h;
                let vy = (rng.r#gen::<f32>() - 0.5) * 2.0 * h * VERTICAL_K_VORTEX.sqrt();
                (py, vy)
            } else {
                ((rng.r#gen::<f32>() - 0.5) * DISK_THICKNESS, 0.0)
            };

            let vc = force_circular_speed(r, &config).min(MAX_SPEED);
            let tx = -pz / r;
            let tz = px / r;
            let mut vx = tx * vc;
            let mut vz = tz * vc;
            if vortex_init {
                let vr = -vc * INFLOW_FRAC;
                vx += (px / r) * vr;
                vz += (pz / r) * vr;
            }

            // Velocity quaternion: w = ether pressure, xyz = velocity
            let pressure = (vc / MAX_SPEED).clamp(0.0, 1.0);
            let flux = FluxQuaternion::new(pressure, vx, vy, vz);

            particles.push(Particle {
                x: px,
                y: py,
                z: pz,
                flux,
            });
        }

        // BH starts BETWEEN arms (offset π/2 from arm angle) so it begins
        // in a low-density void and gradually sweeps into the arm — this
        // prevents the initial burst of thousands of interactions.
        let bh_r: f32 = 120.0;
        let bh_theta = GOLDEN_ANGLE * bh_r.ln() + std::f32::consts::FRAC_PI_2; // mid inter-arm
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
            config,
            verbose: true,
            frame: 0,
            absorbed_total: 0,
            time: 0.0,
            pattern_angle: 0.0,
            rng,
        }
    }
}

// ── per-frame update ───────────────────────────────────────────────────────

impl GalaxySimulation {
    pub fn time(&self) -> f32 {
        self.time
    }

    pub fn update(&mut self, dt: f32) {
        let dt = dt.min(0.033);
        let sub_dt = dt / 2.0;
        for _ in 0..2 {
            self.step(sub_dt);
        }
        self.time += dt;
        self.frame += 1;

        // ── Diagnostic output every 300 frames (~5 s at 60 fps) ────────────
        if self.verbose && self.frame.is_multiple_of(300) {
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

    /// Pattern speed and black-hole motion, without touching stars.
    /// The GPU integrator calls this once per substep so the perturber matches
    /// `update`, then integrates stars in the compute shader.
    pub fn advance_perturber(&mut self, dt: f32) {
        if !self.config.legacy {
            self.pattern_angle += PATTERN_OMEGA * dt;
        }
        self.update_black_hole(dt);
    }

    pub fn pattern_angle(&self) -> f32 {
        self.pattern_angle
    }

    pub fn black_hole_xyz(&self) -> [f32; 3] {
        [self.black_hole.x, self.black_hole.y, self.black_hole.z]
    }

    fn step(&mut self, dt: f32) {
        if !self.config.legacy {
            self.pattern_angle += PATTERN_OMEGA * dt;
        }
        self.update_black_hole(dt);
        let (bhx, bhy, bhz) = (self.black_hole.x, self.black_hole.y, self.black_hole.z);
        let absorbed = if self.config.legacy {
            self.update_particles_legacy(dt, bhx, bhy, bhz)
        } else {
            self.update_particles(dt, bhx, bhy, bhz)
        };
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

    /// Pre-fix integrator. Kept so `--legacy` / headless baselines match the
    /// force law this review measured, including the 6%/s damping and the
    /// one-arm `sin(phase)` pinch.
    fn update_particles_legacy(&mut self, dt: f32, bhx: f32, bhy: f32, bhz: f32) -> u64 {
        // Same seeded generator as init. Pulled out of `self` so the particle
        // loop can mutably borrow the buffer.
        let mut rng = std::mem::replace(&mut self.rng, StdRng::seed_from_u64(0));
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
            let theta_arm = GOLDEN_ANGLE * r_xz.ln();
            let phase = p.z.atan2(p.x) - theta_arm;
            let spd_now = (p.flux.x * p.flux.x + p.flux.z * p.flux.z).sqrt().max(1.0);
            let pinch = phase.sin() * spd_now * PINCH;
            let ax = ax - tx * pinch;
            let az = az - tz * pinch;

            // ── 6. Integrate velocity ──────────────────────────────────────
            // Gentle system-wide damping (6 %/s) prevents slow energy drift
            // where particles gradually spiral outward from BH interactions.
            let damp = 1.0 - DAMPING_LEGACY * dt;
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
                let st = GOLDEN_ANGLE * sr.ln() + ao + (rng.r#gen::<f32>() - 0.5) * 0.8;
                p.x = sr * st.cos();
                p.y = (rng.r#gen::<f32>() - 0.5) * DISK_THICKNESS;
                p.z = sr * st.sin();
                let sv = plummer_circular_speed(sr, GM_CORE, EPS_CORE).min(MAX_SPEED);
                let stx = -p.z / sr;
                let stz = p.x / sr;
                let pres = (sv / MAX_SPEED).clamp(0.0, 1.0);
                p.flux = FluxQuaternion::new(pres, stx * sv, 0.0, stz * sv);
            }
        }

        self.rng = rng;
        absorbed
    }

    fn update_particles(&mut self, dt: f32, bhx: f32, bhy: f32, bhz: f32) -> u64 {
        // Same seeded generator as init. Pulled out of `self` so the particle
        // loop can mutably borrow the buffer.
        let mut rng = std::mem::replace(&mut self.rng, StdRng::seed_from_u64(0));
        let mut absorbed = 0u64;
        let config = self.config;
        let pattern_angle = self.pattern_angle;
        let vortex = config.vortex;
        let k_vert = if vortex {
            VERTICAL_K_VORTEX
        } else {
            VERTICAL_K
        };
        let vy_damp_scale = if vortex { 0.25 } else { 1.0 };

        for p in &mut self.particles {
            let r_xz = (p.x * p.x + p.z * p.z).sqrt().max(0.5);
            let r3d = (p.x * p.x + p.y * p.y + p.z * p.z).sqrt().max(0.5);

            // ── 1. Newtonian core, 3D radius from the origin ───────────────
            let g_core = plummer_accel_factor(r3d * r3d, GM_CORE, EPS_CORE);
            let mut ax = -p.x * g_core;
            let mut ay = -p.y * g_core;
            let mut az = -p.z * g_core;

            // ── 2. Newtonian BH, 3D separation. Cap |a_BH|, not the factor. ─
            let dx = bhx - p.x;
            let dy = bhy - p.y;
            let dz = bhz - p.z;
            let r_bh_sq = dx * dx + dy * dy + dz * dz;
            let r_bh = r_bh_sq.sqrt();
            let mut g_bh = plummer_accel_factor(r_bh_sq, GM_BH, EPS_BH);
            let a_bh = g_bh * r_bh;
            if a_bh > BH_ACCEL_CAP {
                g_bh = BH_ACCEL_CAP / r_bh.max(1e-3);
            }
            ax += dx * g_bh;
            ay += dy * g_bh;
            az += dz * g_bh;

            // ── 3. MOND on the total Newtonian acceleration ────────────────
            // Direction stays Newtonian. Magnitude is boosted by the simple μ
            // function of |a_core + a_BH|. The spring and the updraft are
            // added afterwards and are not part of a_N. In the midplane, far
            // from the black hole, this reduces to boosting the core alone.
            if config.mond {
                let a_n = (ax * ax + ay * ay + az * az).sqrt();
                let boost = mond_boost(a_n, config.a0);
                ax *= boost;
                ay *= boost;
                az *= boost;
            }

            // ── 4. Vertical structure (not gravity) ────────────────────────
            ay -= p.y * k_vert;
            if vortex {
                ay += eyewall_updraft(r_xz, p.y);
            }

            // Reference circular speed of the core in the midplane. Used for
            // the inflow scale, the high-speed drag, and the pressure scalar.
            // It is not a velocity target: nothing below adds (v_target − v).
            let vc_ref = force_circular_speed(r_xz, &config).min(MAX_SPEED);
            let tx = -p.z / r_xz;
            let tz = p.x / r_xz;

            // ── 5. Inflow as a force, a few percent of v_c²/r, radially in ─
            if vortex {
                let a_in = INFLOW_FRAC * vc_ref * vc_ref / r_xz;
                ax -= (p.x / r_xz) * a_in;
                az -= (p.z / r_xz) * a_in;
            }

            // ── 6. Two-arm density wave ────────────────────────────────────
            // sin(phase) is restoring only at phase = 2πk, so the arm placed
            // at offset π was unstable and sheared away. sin(2·phase) is
            // restoring at both arms. The pattern rotates at PATTERN_OMEGA
            // instead of pinning stars to a fixed angle (which stops the orbit).
            // PINCH_ARM is small so the mean torque does not rewrite v(r).
            let theta_arm = GOLDEN_ANGLE * r_xz.ln() + pattern_angle;
            let phase = p.z.atan2(p.x) - theta_arm;
            let spd_now = (p.flux.x * p.flux.x + p.flux.z * p.flux.z).sqrt().max(1.0);
            let pinch = (2.0 * phase).sin() * spd_now * PINCH_ARM;
            let ax = ax - tx * pinch;
            let az = az - tz * pinch;

            // ── 7. Integrate velocity ──────────────────────────────────────
            // Horizontal damping is weak (see DAMPING_RATE). Vertical damping
            // is weaker still in vortex mode, otherwise the spring collapses
            // every orbit onto y = 0. The old 0.06·Ω_Kepler quaternion twist
            // is not applied here: it tipped v_θ into v_r and biased the
            // rotation curve. `--legacy` still has that twist.
            let damp_h = 1.0 - DAMPING_RATE * dt;
            let damp_v = 1.0 - DAMPING_RATE * vy_damp_scale * dt;
            p.flux.x = p.flux.x * damp_h + ax * dt;
            p.flux.y = p.flux.y * damp_v + ay * dt;
            p.flux.z = p.flux.z * damp_h + az * dt;

            // ── 8. Speed cap + progressive high-speed drag ────────────────
            let mut spd = (p.flux.x * p.flux.x + p.flux.y * p.flux.y + p.flux.z * p.flux.z).sqrt();
            if spd > MAX_SPEED {
                let s = MAX_SPEED / spd;
                p.flux.x *= s;
                p.flux.y *= s;
                p.flux.z *= s;
                spd = MAX_SPEED;
            }
            // Threshold is the midplane circular speed of the force law, so a
            // MOND-boosted outer disk is not flagged as a slingshot.
            let excess = spd - vc_ref * 1.5;
            if excess > 0.0 {
                let extra_damp = 1.0 - (excess / MAX_SPEED) * 0.3 * dt;
                p.flux.x *= extra_damp;
                p.flux.y *= extra_damp;
                p.flux.z *= extra_damp;
            }

            p.flux.w = (vc_ref / MAX_SPEED).clamp(0.0, 1.0);

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
            // Cylindrical radius, and in-plane speed only. Counting v_y as
            // "escape speed" deleted the eyewall as soon as the updraft started.
            let r_cyl = (p.x * p.x + p.z * p.z).sqrt();
            let v_plane = (p.flux.x * p.flux.x + p.flux.z * p.flux.z).sqrt();
            let local_vc = force_circular_speed(r_cyl.min(OUTER_RADIUS), &config);
            let escaped = r_cyl > OUTER_RADIUS * 1.02 && v_plane > local_vc * 1.8;

            if r_bh_final < BH_ABSORB_R || r_cyl > OUTER_RADIUS * 1.08 || escaped {
                absorbed += 1;
                let u: f32 = rng.r#gen::<f32>();
                let sr = 30.0 + u.sqrt() * (OUTER_RADIUS * 0.85 - 30.0);
                let ao = if rng.r#gen::<bool>() {
                    0.0_f32
                } else {
                    std::f32::consts::PI
                };
                let st =
                    GOLDEN_ANGLE * sr.ln() + pattern_angle + ao + (rng.r#gen::<f32>() - 0.5) * 0.8;
                p.x = sr * st.cos();
                p.z = sr * st.sin();
                let (py, vy) = if vortex {
                    let h = 8.0 + 18.0 * (sr / OUTER_RADIUS);
                    let py = (rng.r#gen::<f32>() - 0.5) * 2.0 * h;
                    let vy = (rng.r#gen::<f32>() - 0.5) * 2.0 * h * VERTICAL_K_VORTEX.sqrt();
                    (py, vy)
                } else {
                    ((rng.r#gen::<f32>() - 0.5) * DISK_THICKNESS, 0.0)
                };
                p.y = py;
                let sv = force_circular_speed(sr, &config).min(MAX_SPEED);
                let stx = -p.z / sr;
                let stz = p.x / sr;
                let mut svx = stx * sv;
                let mut svz = stz * sv;
                if vortex {
                    let vr = -sv * INFLOW_FRAC;
                    svx += (p.x / sr) * vr;
                    svz += (p.z / sr) * vr;
                }
                let pres = (sv / MAX_SPEED).clamp(0.0, 1.0);
                p.flux = FluxQuaternion::new(pres, svx, vy, svz);
            }
        }

        self.rng = rng;
        absorbed
    }

    /// Rotation curve and disk thickness. Bins are fixed so runs can be diffed.
    pub fn stats(&self) -> SnapshotStats {
        const EDGES: [f32; 11] = [
            0.0, 15.0, 30.0, 45.0, 60.0, 80.0, 100.0, 130.0, 170.0, 220.0, 280.0,
        ];
        let nb = EDGES.len() - 1;
        let mut count = vec![0u32; nb];
        let mut sum_vt = vec![0.0f32; nb];
        let mut sum_vr = vec![0.0f32; nb];
        let mut sum_y2 = vec![0.0f32; nb];

        let mut y2 = 0.0f32;
        let mut abs_y_sum = 0.0f32;
        let mut abs_y: Vec<f32> = Vec::with_capacity(self.particles.len());
        let mut hist = [0u32; 16];
        let mut hist_n = 0u32;

        for p in &self.particles {
            let r = (p.x * p.x + p.z * p.z).sqrt().max(1e-4);
            let rx = p.x / r;
            let rz = p.z / r;
            // +θ tangent matches the integrator: (-z, x) / r
            let tx = -rz;
            let tz = rx;
            let vt = p.flux.x * tx + p.flux.z * tz;
            let vr = p.flux.x * rx + p.flux.z * rz;
            y2 += p.y * p.y;
            abs_y_sum += p.y.abs();
            abs_y.push(p.y.abs());

            let bin = EDGES
                .partition_point(|e| *e <= r)
                .saturating_sub(1)
                .min(nb - 1);
            if r < EDGES[nb] {
                count[bin] += 1;
                sum_vt[bin] += vt;
                sum_vr[bin] += vr;
                sum_y2[bin] += p.y * p.y;
            }

            if (20.0..200.0).contains(&r) {
                let phase = p.z.atan2(p.x) - GOLDEN_ANGLE * r.ln() - self.pattern_angle;
                let mut d = phase.rem_euclid(std::f32::consts::PI);
                if d > std::f32::consts::FRAC_PI_2 {
                    d -= std::f32::consts::PI;
                }
                let u = (d + std::f32::consts::FRAC_PI_2) / std::f32::consts::PI;
                let k = ((u * hist.len() as f32) as usize).min(hist.len() - 1);
                hist[k] += 1;
                hist_n += 1;
            }
        }

        let n = self.particles.len().max(1) as f32;
        abs_y.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let p90_idx = ((abs_y.len() as f32) * 0.90) as usize;
        let p90 = abs_y
            .get(p90_idx.min(abs_y.len().saturating_sub(1)))
            .copied()
            .unwrap_or(0.0);

        let arm_contrast = if hist_n == 0 {
            0.0
        } else {
            let max_h = *hist.iter().max().unwrap_or(&0) as f32;
            let min_h = *hist.iter().min().unwrap_or(&0) as f32;
            if max_h + min_h > 0.0 {
                (max_h - min_h) / (max_h + min_h)
            } else {
                0.0
            }
        };

        let a0 = self.config.a0;
        let bins = (0..nb)
            .map(|i| {
                let c = count[i];
                let r_mid = 0.5 * (EDGES[i] + EDGES[i + 1]);
                let inv = if c > 0 { 1.0 / c as f32 } else { 0.0 };
                RadialBin {
                    r_mid,
                    count: c,
                    v_theta: sum_vt[i] * inv,
                    v_rad: sum_vr[i] * inv,
                    rms_y: (sum_y2[i] * inv).sqrt(),
                    v_newton: newton_circular_speed(r_mid),
                    v_mond: mond_circular_speed(r_mid, a0),
                }
            })
            .collect();

        let bh = &self.black_hole;
        SnapshotStats {
            t: self.time,
            rms_y: (y2 / n).sqrt(),
            mean_abs_y: abs_y_sum / n,
            p90_abs_y: p90,
            arm_contrast,
            bh_r: (bh.x * bh.x + bh.y * bh.y + bh.z * bh.z).sqrt(),
            absorbed: self.absorbed_total,
            n: self.particles.len(),
            bins,
        }
    }
}

/// Binned tangential speed at r ≈ 52.5 and r ≈ 195, plus disk rms height.
/// Shared by the CPU `stats` path's tests and the GPU parity check.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(test), allow(dead_code))]
pub struct CurveCheck {
    pub v_52: f32,
    pub v_195: f32,
    pub ratio: f32,
    pub rms_y: f32,
    pub n_52: u32,
    pub n_195: u32,
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn curve_check(samples: impl IntoIterator<Item = ([f32; 3], [f32; 3])>) -> CurveCheck {
    let mut n = 0u32;
    let mut y2 = 0.0f32;
    let mut n52 = 0u32;
    let mut n195 = 0u32;
    let mut vt52 = 0.0f32;
    let mut vt195 = 0.0f32;
    for (p, v) in samples {
        n += 1;
        y2 += p[1] * p[1];
        let r = (p[0] * p[0] + p[2] * p[2]).sqrt().max(1e-4);
        let tx = -p[2] / r;
        let tz = p[0] / r;
        let vt = v[0] * tx + v[2] * tz;
        if (45.0..60.0).contains(&r) {
            n52 += 1;
            vt52 += vt;
        } else if (170.0..220.0).contains(&r) {
            n195 += 1;
            vt195 += vt;
        }
    }
    let v_52 = if n52 > 0 { vt52 / n52 as f32 } else { 0.0 };
    let v_195 = if n195 > 0 { vt195 / n195 as f32 } else { 0.0 };
    let ratio = if v_52.abs() > 1e-3 { v_195 / v_52 } else { 0.0 };
    let rms_y = if n > 0 { (y2 / n as f32).sqrt() } else { 0.0 };
    CurveCheck {
        v_52,
        v_195,
        ratio,
        rms_y,
        n_52: n52,
        n_195: n195,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mond_newtonian_and_deep_limits() {
        let a0 = 1.0;
        let deep = mond_boost(1.0e-4, a0);
        let a = 1.0e-4 * deep;
        let expect = (1.0e-4_f32 * a0).sqrt();
        assert!((a - expect).abs() / expect < 0.02, "deep {a} vs {expect}");

        let newton = mond_boost(1.0e4, a0);
        assert!((newton - 1.0).abs() < 0.01, "newton boost {newton}");
    }

    #[test]
    fn mond_flat_asymptote_matches_gm_a0() {
        // Point-mass deep-MOND limit: v⁴ = GM a0.
        // r = 5000 is deep in the Plummer tail, where the simple μ function
        // has actually reached that asymptote. At r = 500 the approach is still ~3%.
        let a0 = DEFAULT_A0;
        let r = 5_000.0;
        let v = mond_circular_speed(r, a0);
        let v_flat = (GM_CORE * a0).powf(0.25);
        assert!((v - v_flat).abs() / v_flat < 0.02, "{v} vs {v_flat}");
        let ratio = v.powi(4) / (GM_CORE * a0);
        assert!((ratio - 1.0).abs() < 0.05, "v^4/(GM a0) = {ratio}");
    }

    fn v_at(s: &SnapshotStats, r_mid: f32) -> f32 {
        s.bins
            .iter()
            .find(|b| (b.r_mid - r_mid).abs() < 1.0 && b.count >= 15)
            .map(|b| b.v_theta)
            .unwrap_or(0.0)
    }

    /// v(195) / v(52.5). The analytic midplane ratios are about 0.83 (MOND,
    /// a0 = 1) and 0.53 (Plummer). This checks the integrated curve, not a
    /// velocity assignment.
    fn outer_inner_ratio(s: &SnapshotStats) -> f32 {
        let outer = v_at(s, 195.0);
        let inner = v_at(s, 52.5);
        assert!(inner > 1.0, "inner bin empty or stalled: {inner}");
        outer / inner
    }

    #[test]
    fn a_star_at_rest_is_not_snapped_to_circular_speed() {
        // NU_ORBIT = 1/s would close most of the gap to v_c in one second.
        // With that term gone, a star at rest only picks up a tangential
        // speed from the weak pinch. Radial speed should grow from gravity.
        let mut sim = GalaxySimulation::with_config(
            1,
            GalaxyConfig {
                vortex: false,
                mond: true,
                ..GalaxyConfig::default()
            },
            Some(1),
        );
        sim.verbose = false;
        sim.black_hole.x = 8_000.0;
        sim.black_hole.vx = 0.0;
        sim.black_hole.vy = 0.0;
        sim.black_hole.vz = 0.0;
        let p = &mut sim.particles[0];
        p.x = 100.0;
        p.y = 0.0;
        p.z = 0.0;
        p.flux = FluxQuaternion::new(0.0, 0.0, 0.0, 0.0);

        for _ in 0..60 {
            sim.update(1.0 / 60.0);
        }
        let p = &sim.particles[0];
        let v_theta = p.flux.z; // tangent at (x, 0) is +z
        let v_rad = p.flux.x; // radial at (x, 0) is +x; gravity pulls inward
        let vc = force_circular_speed(100.0, &sim.config);
        assert!(
            v_theta.abs() < 1.0,
            "tangential speed {v_theta} snapped toward v_c {vc}"
        );
        assert!(
            v_rad < -0.3,
            "expected an inward radial speed from gravity, got {v_rad}"
        );
    }

    #[test]
    fn vortex_disk_stays_thick_and_mond_ratio_stays_above_newtonian() {
        let n = 2000;
        let mut legacy = GalaxySimulation::with_config(
            n,
            GalaxyConfig {
                legacy: true,
                ..GalaxyConfig::default()
            },
            Some(7),
        );
        legacy.verbose = false;
        let mut mond = GalaxySimulation::with_config(n, GalaxyConfig::default(), Some(7));
        mond.verbose = false;
        let mut newton = GalaxySimulation::with_config(
            n,
            GalaxyConfig {
                mond: false,
                ..GalaxyConfig::default()
            },
            Some(7),
        );
        newton.verbose = false;

        let y0_l = legacy.stats().rms_y;
        let y0_m = mond.stats().rms_y;
        assert!(y0_m > y0_l * 3.0, "init thickness {y0_m} vs {y0_l}");

        // 10 s is long enough for a 1/s velocity slave to have locked, and
        // short of the eye-wall bounce emptying the r ≈ 52 bin. The outer
        // disk (r ≈ 90 and r ≈ 195) is where the force law shows.
        let steps = 10 * 60;
        for _ in 0..steps {
            legacy.update(1.0 / 60.0);
            mond.update(1.0 / 60.0);
            newton.update(1.0 / 60.0);
        }
        let sl = legacy.stats();
        let sm = mond.stats();
        let sn = newton.stats();
        assert!(
            sm.rms_y > sl.rms_y * 3.0,
            "t=10s thickness mond {} legacy {}",
            sm.rms_y,
            sl.rms_y
        );

        let v195_m = v_at(&sm, 195.0);
        let v195_n = v_at(&sn, 195.0);
        let v90_m = v_at(&sm, 90.0);
        let v90_n = v_at(&sn, 90.0);
        // Analytic midplane circular speeds: MOND 8.62 / 9.41, Plummer 4.52 / 6.63.
        assert!(
            (v195_m - 8.62).abs() < 1.5,
            "MOND v(195) = {v195_m} left the force's circular speed"
        );
        assert!(
            (v195_n - 4.52).abs() < 1.2,
            "Newtonian v(195) = {v195_n} left the Plummer circular speed"
        );
        let ratio_m = v195_m / v90_m;
        let ratio_n = v195_n / v90_n;
        assert!(
            ratio_m > 0.85,
            "MOND v(195)/v(90) = {ratio_m} fell like Kepler"
        );
        assert!(
            ratio_n < 0.80,
            "Newtonian v(195)/v(90) = {ratio_n} stayed flat"
        );
        // v(195)/v(52.5). The r ≈ 52 bin sits in the eye wall, so this ratio
        // moves when that bin bounces; it still must not invert the two laws.
        let wide_m = outer_inner_ratio(&sm);
        let wide_n = outer_inner_ratio(&sn);
        assert!(
            wide_m > wide_n,
            "MOND v(195)/v(52.5) = {wide_m} vs Newtonian {wide_n}"
        );
        assert!(
            wide_n < 0.80,
            "Newtonian v(195)/v(52.5) = {wide_n} did not fall"
        );
    }
}

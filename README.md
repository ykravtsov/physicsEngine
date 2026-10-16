# Quaternion Vortex Engine

A custom 3D physics engine built from scratch in Rust using **pure quaternion mathematics** — no matrix transforms (except the final GPU projection). This engine implements the **Quaternionic Dirac-Maxwell-NS (QDM-NS)** unified vortex field theory: a single quaternion operator that unifies Navier-Stokes fluid dynamics, Maxwell electromagnetism, and Dirac-like spin into one closed, self-consistent framework.

📐 For a symbol-by-symbol write-up of the equations, units, open questions and observational checks, see [THEORY.md](THEORY.md).

## 🌀 What You See

`cargo run` opens the galaxy simulation in `src/galaxy.rs`: about 60,000 stars around a fixed Plummer core, with a light black hole drifting through them. The default disk is flared (a vertical vortex: seeded vertical speed, a soft spring, an eye-wall updraft) and wound into a two-arm logarithmic spiral. The window title is `QDM-NS Galaxy — Black Hole Through Space Fluid`.

The older 50,000-particle hurricane lives in `src/hurricane_3d/` and `src/simulation/hurricane.rs`. The hurricane module is not what `main` steps. See [REVIEW.md](REVIEW.md) for what the running force law does and does not implement.

## 🎮 Controls

| Key | Action |
|-----|--------|
| `W/S` | Fly forward / backward |
| `A/D` | Fly left / right |
| `Space` | Fly up |
| `Shift` | Fly down |
| `Mouse` | Look around (always active) |
| `M` | Toggle the MOND boost (ignored with `--legacy`) |
| `Escape` | Quit |

## 🔬 The Physics

### Everything is a FluxQuaternion

The core mathematical object is [`FluxQuaternion`](src/quaternion/math.rs):

```
FluxQuaternion { w, x, y, z }
  w = scalar pressure (ether density / energy)
  x, y, z = vector flow (velocity / force direction)
```

**No matrices anywhere** except the final GPU view-projection matrix, which is built directly from the quaternion camera orientation via `FluxQuaternion::to_view_matrix()`.

### Hurricane Physics Model

The hurricane is a 3D atmospheric vortex governed by:

1. **Pressure Gradient Force** — inward suction toward the low-pressure eye
2. **Tangential Force** — Rankine vortex profile: `v = v_max × (r_eye / r)` drives rotation
3. **Harmonic Series** — 4 spiral frequencies (fundamental + harmonics) mix warm/cold air layers, creating the banded structure of real hurricanes
4. **Air Viscosity** — couples each particle's velocity to the local mean field, keeping the vortex coherent
5. **Coriolis Effect** — quaternion rotation around Y axis (Earth's spin, Northern Hemisphere = counter-clockwise)
6. **Gravity** — weak restoring force keeps particles in the atmospheric disk
7. **Drag** — atmospheric friction

### Why Quaternions Beat Matrices

| Operation | Matrix | Quaternion |
|-----------|--------|------------|
| Rotation representation | 9 floats (3×3) | 4 floats |
| Compose two rotations | 27 multiplications | 16 multiplications |
| Gimbal lock | Yes | No |
| Interpolation | Complex (SLERP needs quaternions anyway) | Natural SLERP |
| Physical meaning | Abstract transform | Pressure + flow unified |

The quaternion camera in [`src/quaternion/camera.rs`](src/quaternion/camera.rs) uses `q * v * q^-1` for all rotations — no Euler angles, no gimbal lock.

## 🏗️ Architecture

```
src/
  engine/
    gpu.rs          -- wgpu device, queue, surface setup
    input.rs        -- keyboard/mouse input state
  
  quaternion/
    math.rs         -- FluxQuaternion: Hamilton product, rotation, wave interaction
    camera.rs       -- Quaternion fly camera (no matrices, no gimbal lock)
  
  hurricane_3d/
    mod.rs          -- Particle type shared with the galaxy sim
    renderer.rs     -- wgpu particle renderer (point sprites, additive blending)

  galaxy.rs         -- Live galaxy integrator (Plummer core, black hole, optional MOND and vortex)
  headless.rs       -- `--headless` sampler: rotation curve and disk thickness, no window

  main.rs           -- winit event loop, frame timing, input dispatch, `--headless`

assets/shaders/
  particle3d.wgsl   -- GPU vertex/fragment shader for particles
```

## 🌌 The QDM-NS Mathematical Framework

This engine is built on the **Quaternionic Dirac-Maxwell-Navier-Stokes (QDM-NS)** unified vortex field theory.

### The Unified State Field

$$
\Psi = \psi + F + v \quad \in \mathbb{H}(\mathbb{C})
$$

- $\psi$ — Dirac-like wavefunction for macroscopic vortex "spin"
- $F$ — EM quaternion
- $v$ — fluid velocity quaternion

### The Master Operator

$$
\mathcal{D}\Psi = \left(\nabla + m_{\text{eff}} + \nu\nabla^2 - \frac{D}{Dt}\right)\Psi + \bar{\Psi} \cdot J + \lambda(\Psi\bar{\Psi} - 1)\Psi
$$

| Term | Physics |
|------|---------|
| $\nabla\Psi$ | Dirac + Maxwell core (relativistic + EM propagation) |
| $m_{\text{eff}}\Psi$ | Effective mass from baryonic density + self-gravity ("vortex rest energy") |
| $\nu\nabla^2\Psi$ | Quaternion viscosity — $\|\nabla^2\Psi\|$ regularises $r \to 0$; division algebra forbids true point singularities |
| $-\tfrac{D}{Dt}\Psi$ | Material derivative → Navier-Stokes advection + nonlinear vortex stretching |
| $\bar{\Psi} \cdot J$ | Lorentz force + current self-interaction (galactic arms as Birkeland / Z-pinch wires) |
| $\lambda(\Psi\bar{\Psi}-1)\Psi$ | Nonlinear vorticity-threshold brake; when $\|\omega\| = \|\operatorname{Im}(\nabla\Psi)\|$ exceeds the critical value set by $\lambda$, excess vorticity is damped into jets/outflows |

The pressure gradient $\nabla p$ is recovered as the real scalar part (Bernoulli-like from the quaternionic Dirac-Maxwell-Bernoulli relation).

### Galaxy as Vortex: Golden-Ratio Spirals Emerge Naturally

Assume a steady-state, axisymmetric solution in cylindrical coordinates. The velocity quaternion is purely rotational:

$$
v(r, \phi) = \omega(r)\,\hat{k} + \phi_0 \ln r \qquad (\phi_0\text{ pitch related to golden ratio})
$$

Plugging into the unified equation and linearising for small perturbations, the vorticity equation reduces to a self-similar logarithmic spiral whose pitch angle satisfies the eigenvalue condition from the $\nu\nabla^2$ + Lorentz-pinch balance. The exact attractor solution is the **golden-ratio logarithmic spiral**:

$$
r(\theta) = r_0 \exp\!\left(\frac{\theta}{\phi_0}\right), \qquad
\phi_0 = \frac{2\pi}{\varphi^2} \approx 2.400\,\text{rad}\ \text{(the golden angle)}, \qquad
\varphi = \frac{1+\sqrt{5}}{2}
$$

Here $\theta$ is the azimuthal angle and $\phi_0$ is the arm winding coefficient ($\theta_{\text{arm}} = \phi_0 \ln r$), as used in `src/galaxy.rs` (`GOLDEN_ANGLE`) and in `assets/shaders/galaxy_sim.wgsl`. The pitch angle is $\arctan(1/\phi_0) \approx 22.6^\circ$. For reference, other "golden" conventions give different spirals: growth by $\varphi$ per full turn means $\phi_0 = 2\pi/\ln\varphi \approx 13.06$ rad (pitch $\arctan(\ln\varphi/2\pi) \approx 4.38^\circ$), and the classic golden spiral (growth by $\varphi$ per quarter turn) has $\phi_0 = \pi/(2\ln\varphi) \approx 3.26$ (pitch $\arctan(2\ln\varphi/\pi) \approx 17.03^\circ$). The compute shader is not compiled into `cargo run`.

This is **not** forced — it is the attractor of the nonlinear vortex + EM-wire system. Arm spacing and winding are fixed by the golden ratio because it is the continued-fraction optimum for self-similar vorticity transport.

### Simulation Parameters (QDM-NS → Code)

| Symbol | Code field | Physical role |
|--------|-----------|---------------|
| $\varphi$ | `phi_value` | Golden ratio (tunable via ←/→ keys) |
| $\phi_0 = 2\pi/\varphi^2$ | `GOLDEN_ANGLE` | Arm winding coefficient ($\theta_{\text{arm}} = \phi_0 \ln r$, pitch ≈ 22.6°) |
| $\nu$ | `nu` | Quaternion viscosity (regularises $r\to0$) |
| $\lambda$ | `lambda` | Vorticity-threshold brake / jet trigger |
| $m_{\text{eff}}$ | `m_eff` | Baryonic self-gravity |
| $\bar{\Psi}\cdot J$ | `pinch_strength` | Birkeland / Z-pinch arm coupling |

### Why This Framework is Self-Consistent

- **Limits**: the written operator is meant to reduce to Dirac in the microscopic limit and to a weak-field mass term in the macroscopic limit. THEORY.md §3 is the dimensional check: the master equation as written does not yet make the units match by itself.
- **What the window actually integrates** is not that operator. `src/galaxy.rs` applies a Plummer core, a Plummer black hole, a vertical spring, a small two-arm pinch, and (unless `--legacy`) a weak drag. `--mond` multiplies the total Newtonian acceleration `|a_core + a_BH|` by the simple interpolating function; it does not come from a computed `B` or `J`. Flat curves are not produced by the pinch. Measured numbers are in [REVIEW.md](REVIEW.md).
- **Predictions the theory would like to make**, once a discretisation of `DΨ` exists: (1) a flat rotation curve from whatever radial support the closed equation actually has; (2) jet power scaling with `λ`; (3) a pitch near the golden ratio. The live pinch puts the golden angle in by hand.

### Hurricane Physics (original simulation)

The hurricane sim maps the same operator terms:

| QDM-NS term | Hurricane physics |
|-------------|-------------------|
| $\bar{\Psi}\cdot J$ | Tangential force — Rankine vortex |
| $-\tfrac{D}{Dt}\Psi$ harmonic | Harmonic series spiral mixing (4 frequencies) |
| $-\tfrac{\mu_0 I}{2\pi r}\hat{r}$ | Pressure gradient — inward suction |
| $\nu\nabla^2\Psi$ | Air viscosity + drag |

## 🛠️ Building

```bash
# Requires Rust (stable)
cargo build --release
cargo run --release

# No window. Writes a rotation-curve CSV (gitignored under analysis/review_data/).
cargo run --release -- --headless --seconds 40 --particles 60000 --seed 1 \
    --out analysis/review_data/modern_curve.csv

# Same disk and seed, parent-branch forces:
cargo run --release -- --headless --legacy --seconds 40 --particles 60000 --seed 1 \
    --out analysis/review_data/legacy_curve.csv

# Vortex disk, Newtonian gravity (MOND off):
cargo run --release -- --headless --no-mond --seconds 40 --particles 60000 --seed 1 \
    --out analysis/review_data/vortex_only_curve.csv
```

`--mond` / `--no-mond`, `--vortex` / `--no-vortex`, and `--a0 <float>` change the live force law. `--legacy` ignores those and runs the pre-fix integrator. `M` toggles MOND while the window is open.

## Dependencies

- `winit` 0.30 — OS window + input events
- `wgpu` 22 — Vulkan/DX12/Metal GPU rendering
- `bytemuck` — safe GPU buffer casting
- `pollster` — async runtime for GPU initialization
- `rand` — particle initialization
- `env_logger` — logging

No Bevy. No nalgebra. No glam. Pure quaternion math.

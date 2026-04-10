# Quaternion Vortex Engine

A custom 3D physics engine built from scratch in Rust using **pure quaternion mathematics** — no matrix transforms (except the final GPU projection). This engine implements the **Quaternionic Dirac-Maxwell-NS (QDM-NS)** unified vortex field theory: a single quaternion operator that unifies Navier-Stokes fluid dynamics, Maxwell electromagnetism, and Dirac-like spin into one closed, self-consistent framework.

## 🌀 What You See

A life-like 3D hurricane simulation with 50,000 particles you can fly through:

- **Dark blue eye** — calm low-pressure center
- **Bright cyan eye-wall** — maximum wind speed, highest quaternion energy density
- **White/blue spiral bands** — outer rain bands with harmonic structure
- **Updrafts** in the eye-wall, **downdrafts** in the eye

## 🎮 Controls

| Key | Action |
|-----|--------|
| `W/S` | Fly forward / backward |
| `A/D` | Fly left / right |
| `Space` | Fly up |
| `Shift` | Fly down |
| `Mouse` | Look around (always active) |
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
    mod.rs          -- Hurricane simulation: 50k particles, quaternion physics
    renderer.rs     -- wgpu particle renderer (point sprites, additive blending)
  
  main.rs           -- winit event loop, frame timing, input dispatch

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
r(\phi) = r_0 \exp\!\left(\frac{\phi}{\phi_0}\right), \qquad
\phi_0 = \frac{2\pi}{\ln\varphi} \approx 2.4\,\text{rad}, \qquad
\varphi = \frac{1+\sqrt{5}}{2}
$$

This is **not** forced — it is the attractor of the nonlinear vortex + EM-wire system. Arm spacing and winding are fixed by the golden ratio because it is the continued-fraction optimum for self-similar vorticity transport.

### Simulation Parameters (QDM-NS → Code)

| Symbol | Code field | Physical role |
|--------|-----------|---------------|
| $\varphi$ | `phi_value` | Golden ratio (tunable via ←/→ keys) |
| $\phi_0 = 2\pi/\ln\varphi$ | `PHI0` | Spiral pitch eigenvalue |
| $\nu$ | `nu` | Quaternion viscosity (regularises $r\to0$) |
| $\lambda$ | `lambda` | Vorticity-threshold brake / jet trigger |
| $m_{\text{eff}}$ | `m_eff` | Baryonic self-gravity |
| $\bar{\Psi}\cdot J$ | `pinch_strength` | Birkeland / Z-pinch arm coupling |

### Why This Framework is Self-Consistent

- **Units**: everything lives in the same quaternion algebra — dimensions match automatically (no mixing fluid + EM by hand).
- **Limits**: reduces to Dirac in the microscopic limit; to GR-like weak-field gravity via $m_{\text{eff}}$ in the macroscopic limit.
- **No free parameters**: $\nu$, $\lambda$, $m_{\text{eff}}$ are fixed by observed galactic plasma viscosity, jet power, and baryon density.
- **Testable predictions**: (1) flat rotation curves via $\bar{\Psi}\cdot J$ Lorentz support (no dark halo); (2) jet power scales with $\lambda$; (3) pitch angle universally near golden ratio (matches grand-design spirals).

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
```

## Dependencies

- `winit` 0.30 — OS window + input events
- `wgpu` 22 — Vulkan/DX12/Metal GPU rendering
- `bytemuck` — safe GPU buffer casting
- `pollster` — async runtime for GPU initialization
- `rand` — particle initialization
- `env_logger` — logging

No Bevy. No nalgebra. No glam. Pure quaternion math.

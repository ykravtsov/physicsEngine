# Quaternion Vortex Engine

A custom 3D physics engine built from scratch in Rust using **pure quaternion mathematics** — no Bevy, no matrix transforms (except the final GPU projection). This engine proves the **Vortex Theory**: that quaternions are a more efficient and natural way to represent reality than traditional matrix/vector math.

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

## 🌌 The QQM Mathematical Framework

This engine is built on the **Quaternion Quantum Mechanics (QQM)** vortex field theory:

### The Master Equation

$$
\vec{F}_{net} = \underbrace{q(\vec{v} \times \vec{B})}_{\text{Dynamo Drive}} + \underbrace{\left( \phi \cdot \nabla \ln r \right) \hat{\theta}}_{\text{Geometric Phase}} - \underbrace{\left( \frac{\mu_0 I}{2\pi r} \right) \hat{r}}_{\text{Z-Pinch Tension}} - \underbrace{\vec{v} \cdot \phi^{-4}}_{\text{Ether Viscosity}}
$$

In the hurricane simulation:
- **Dynamo Drive** → tangential force (Rankine vortex profile)
- **Geometric Phase** → harmonic series spiral mixing
- **Z-Pinch Tension** → pressure gradient (inward suction)
- **Ether Viscosity** → air viscosity + drag

### Harmonic Series (Warm/Cold Air Mixing)

Real hurricanes have banded structure from the interaction of multiple spiral frequencies. The simulation models this as:

$$
F_{harmonic} = \sum_{n} A_n \cdot \hat{\theta} \cdot \sin(n \cdot \theta)
$$

Where the harmonics are:
- **Fundamental (n=1)**: main spiral arm
- **2nd harmonic (n=2)**: inner spiral bands
- **3rd harmonic (n=3)**: fine structure
- **Sub-harmonic (n=0.5)**: large outer band

### Air Viscosity (Vortex Coherence)

The viscosity term couples each particle to the local mean flow:

$$
F_{viscosity} = \mu \cdot (\vec{v}_{target} - \vec{v}_{particle})
$$

This is what keeps the hurricane together — without it, particles fly apart. In the vortex theory, this is the **Ether Drag** that maintains the dissipative structure.

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

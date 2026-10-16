# QDM-NS Galaxy

`cargo run` opens a 60,000-particle galaxy. The integrator is the CPU loop in [`src/galaxy.rs`](src/galaxy.rs): a fixed Plummer core, a light black hole on a dragging orbit, and a flared disk wound into a two-arm logarithmic spiral. The window title is `QDM-NS Galaxy — Black Hole Through Space Fluid`.

The equations the model is aiming at are in [THEORY.md](THEORY.md). What this loop integrates, and the measurements behind the table below, are in [REVIEW.md](REVIEW.md).

## Forces

Each substep, for every star:

1. Newtonian Plummer acceleration of the core (`GM = 4000`, `ε = 8`) at the star's 3D distance from the origin.
2. Newtonian Plummer acceleration of the black hole (`GM = 360`, `ε = 20`) at the 3D separation. `|a_BH|` is capped at 8.
3. When MOND is on, those two accelerations are summed and the magnitude is multiplied by the simple interpolating function. The direction stays Newtonian.
4. A vertical spring. With the vortex flag, an eye-wall updraft is added afterwards. The spring and the updraft are not part of the MOND input.
5. With the vortex flag, an inward acceleration `0.035 · v_c² / r`.
6. A two-arm pinch, `sin(2 · phase)` at `0.001/s`, on a pattern rotating at `0.07 rad/s`. The arm locus is `θ = (2π/φ²) ln r` (pitch about 22.6°).
7. In-plane damping of `0.001/s`.

Tangential speed is whatever those accelerations produce. Stars are born on the midplane circular speed of the core, `sqrt(a_r · r)`. Nothing in the step relaxes `v` back onto that curve.

`--legacy` ignores the flags below and runs the earlier law: 6%/s damping, a one-arm `sin(phase)` pinch, a razor-thin disk (`|y| ≤ 2.5`, `vy = 0`), and a small extra twist of the velocity vector.

## MOND

```
a = a_N · (½ + √(¼ + a0 / a_N))
```

In the deep regime this is `a ≈ √(a_N · a0)`, so `v⁴ → GM · a0`. The default scale is `a0 = 1` in **simulation units** (`DEFAULT_A0` in `src/galaxy.rs`). With `GM = 4000` the asymptote is `v∞ = (GM · a0)^(1/4) ≈ 7.95`. At r = 195 the Plummer field is not fully in that regime, so the midplane circular speed is about 8.6. This `a0` is the code-unit knob, not Milgrom's SI value `1.2×10⁻¹⁰ m/s²`.

| Flag | Default | Effect |
|---|---|---|
| `--mond` / `--no-mond` | on | Boost on or off. |
| `--a0 <float>` | `1` | Simulation-unit scale. Negative values are clamped to 0. |
| `--vortex` / `--no-vortex` | on | Flared disk, seeded vertical speed, spring `k = 0.10`, eye-wall updraft, inflow, rotating two-arm pattern. |
| `--legacy` | off | Earlier integrator. Ignores `--mond`, `--no-mond`, `--a0`, and `--vortex`. |

`M` toggles MOND while the window is open. With `--legacy` the key prints a notice and leaves the force law alone.

## Vortex

The default disk is born with half-thickness `h = 8 + 18·(r/R)` and a vertical speed of that order. The updraft peaks at cylindrical radius 34 and turns over near `|y| = 36`. Vertical damping is a quarter of the in-plane rate, so the layer keeps an rms height of order 14 over a 40 s run. `--no-vortex` drops the flare, the updraft, and the inflow, and uses the stiffer spring.

## Measured rotation curve

Headless release run, 60,000 particles, seed 1, `dt = 1/60`, 40 s. `v_N` and `v_M` are midplane circular speeds of the core. They are labels on the bins, not values the integrator is steering toward.

| | legacy | vortex only (`--no-mond`) | MOND + vortex (default) |
|---|---:|---:|---:|
| rms height √⟨y²⟩ | 0.20 | 14.8 | 13.8 |
| v_θ at r ≈ 52 | 2.93 | 7.98 (v_N 8.58) | 8.64 (v_M 10.44) |
| v_θ at r ≈ 195 | 1.41 | 4.61 (v_N 4.52) | 9.51 (v_M 8.62) |
| v(195) / v(52.5) | 0.48 | 0.58 | 1.10 |

With MOND off, speed from r ≈ 70 outward stays on the falling Plummer curve. With MOND on, v_θ from r ≈ 90 to r ≈ 195 stays between 9.4 and 9.5. The ratio 1.10 is flatter than the analytic midplane ratio 0.83, because the r ≈ 52 bin sits in the eye wall and runs slow, and the outer disk runs about 10% above 8.62. Full bins and plots are in [REVIEW.md](REVIEW.md).

## Controls

| Key | Action |
|---|---|
| `W` / `S` | Fly forward / back |
| `A` / `D` | Fly left / right |
| `Space` / `Shift` | Fly up / down |
| Mouse | Look (cursor is grabbed) |
| `M` | Toggle MOND |
| `Escape` | Quit |

The camera starts at `(200, 480, 300)`, looking at `(0, 8, 0)`.

## Build and run

Rust **1.88** or newer. The crate is `edition = "2024"` and `Cargo.toml` sets `rust-version = "1.88"`. The window needs a GPU and a display. `--headless` does not.

```bash
cargo build --release
cargo run --release

cargo run --release -- --no-mond
cargo run --release -- --a0 0.3
cargo run --release -- --no-vortex
cargo run --release -- --legacy
```

### Headless

Prints the rotation curve and disk thickness, and writes a CSV. No window.

```bash
cargo run --release -- --headless --seconds 40 --particles 60000 --seed 1 \
    --out analysis/review_data/modern_curve.csv

cargo run --release -- --headless --legacy --seconds 40 --particles 60000 --seed 1 \
    --out analysis/review_data/legacy_curve.csv

cargo run --release -- --headless --no-mond --seconds 40 --particles 60000 --seed 1 \
    --out analysis/review_data/vortex_only_curve.csv
```

| Flag | Default |
|---|---|
| `--seconds` | 40 |
| `--particles` | 60000 |
| `--dt` | 1/60 |
| `--seed` | 1, used for the initial disk and for respawn |
| `--out` | `galaxy_curve.csv` |
| `--snapshot-dir` | unset. If set, writes `t0.csv` and `t_end.csv` with columns `x,y,z` |

Samples land on t = 0, 5, 10, 20, 40, … up to `--seconds`, plus the final time. `analysis/review_data/` is gitignored. `python3 analysis/plot_galaxy_review.py` redraws the figures in `docs/img/` from those CSVs.

```bash
cargo test --release
cargo clippy --release -- -D warnings
```

## What is compiled

```
src/main.rs            window loop, or --headless
src/galaxy.rs          force law
src/headless.rs        rotation-curve sampler
src/engine/            wgpu device and keyboard state
src/quaternion/        FluxQuaternion and the fly camera
src/hurricane_3d/      particle struct and the point-sprite renderer
assets/shaders/particle3d.wgsl
```

The camera is a quaternion (`q v q⁻¹`). The one matrix in the frame is the view-projection matrix built from that quaternion for wgpu. A star's velocity is the vector part of a `FluxQuaternion`; the force law writes `x`, `y`, and `z` directly, and stores a speed hint in `w` for color.

## Still unwired

`bevy` is commented out in `Cargo.toml`. `main.rs` does not declare `mod simulation` or `mod physics`, so none of the following is compiled or stepped:

- `src/simulation/` — an older Bevy galaxy, including the GPU compute update and the atmospheric hurricane
- `src/physics/` — a second `FluxQuaternion`
- `assets/shaders/galaxy_sim.wgsl`, `ether_compute.wgsl`, `gpu_galaxy_render.wgsl` — shaders for that path

The live view is 1-pixel points, additive blending, no depth buffer (`src/hurricane_3d/renderer.rs`, `assets/shaders/particle3d.wgsl`).

## Dependencies

- `winit` 0.30 — window and input
- `wgpu` 22 — GPU rendering
- `bytemuck` — buffer casts
- `pollster` — GPU init
- `rand` 0.8 — disk initialisation and respawn
- `env_logger` / `log`

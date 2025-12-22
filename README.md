# Physics Engine

A 3D physics simulation engine built with Rust and the Bevy game engine.

## Features

- Galaxy simulation
- Plasma effects
- Particle emitters
- 3D fly camera
- Custom rendering with WGSL shaders

## Installation

Ensure you have Rust installed. Then clone the repository and build:

```bash
git clone <repository-url>
cd physics-engine
cargo build --release
```

## Usage

Run the simulation:

```bash
cargo run
```

Use the mouse and keyboard to navigate the 3D space. Press Escape to exit.

## Dependencies

- Bevy 0.14
- Rand 0.8

## The QQM Mathematical Framework

The QQM Vortex Equation formalizes the Phi Spiral + Z-Pinch physics simulation:

$$\frac{\partial \vec{\psi}}{\partial t} = \underbrace{\nabla \times (\vec{v} \times \vec{B})}_{\text{Induction}} + \underbrace{\phi \cdot \ln(r)}_{\text{Golden Geometry}} - \underbrace{\frac{\mu_0 I}{2\pi r}}_{\text{Z-Pinch Tension}}$$

### Variable Definitions

- $\vec{\psi}$ (Psi): The Ether Field State (Position/Velocity).
- $\phi$ (Phi): The Golden Ratio ($1.618...$), representing the path of least resistance in the vortex.
- $\nabla \times$: The Curl operator (Vortex Spin).
- $Z_{pinch}$: The electromagnetic tension term ($1/r$) that replaces Dark Matter gravity ($1/r^2$).

## License

[Add license information here]

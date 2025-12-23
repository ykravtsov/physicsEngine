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

## 🌌 The QQM Mathematical Framework

This engine simulates the galaxy not as a collection of billiard balls, but as a **Non-Equilibrium Thermodynamic System** driven by Vortex Mechanics.

### 1. The Master Equation (Vector Form)

The motion of every particle is governed by the **QQM Vortex Field Equation**, which unifies the driving force (Dynamo) with the stabilizing force (Ether Drag):

$$
\vec{F}_{net} = \underbrace{q(\vec{v} \times \vec{B})}_{\text{Dynamo Drive}} + \underbrace{\left( \phi \cdot \nabla \ln r \right) \hat{\theta}}_{\text{Geometric Phase}} - \underbrace{\left( \frac{\mu_0 I}{2\pi r} \right) \hat{r}}_{\text{Z-Pinch Tension}} - \underbrace{\vec{v} \cdot \phi^{-4}}_{\text{Ether Viscosity}}
$$

- **Dynamo Drive:** The spinning core creates the rotational energy.
- **Geometric Phase:** The Golden Ratio ($\phi$) defines the path of least resistance (Logarithmic Spiral).
- **Z-Pinch Tension:** The electromagnetic "tether" that replaces Dark Matter gravity.
- **Ether Viscosity:** The "Golden Drag" ($\phi^{-4} \approx 0.146$) that imposes a cosmic speed limit, creating the observed **Flat Rotation Curves**.

### 2. Thermodynamics & Entropy

Why do galaxies form spirals?

- **Entropy Production:** Nature builds structures to maximize the dissipation of energy gradients.
- **The Logarithmic Spiral:** This geometry ($\theta \propto \ln r$) is the most efficient shape for mixing fluids across scales (from Quantum Foam to Galactic Arms).
- **Stability:** The interaction between the Drive (Energy In) and the Drag (Energy Out) creates a stable "Dissipative Structure" that can persist for billions of years.

---

Note: The CPU implementation (`FluxQuaternion`) has been deprecated in favor of the WGSL Compute Shader implementation for performance (1M+ particles).

## License

[Add license information here]

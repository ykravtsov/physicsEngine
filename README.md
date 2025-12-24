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

### 3. The Thermodynamic Efficiency Law ($\Omega$NF-BH v1.0)

While the vector equation describes motion, the **Unified Negentropic Spin-Curvature Equation** describes the _metabolic efficiency_ of the galaxy. It defines how effectively the vortex converts chaotic energy (Entropy) into structured flow (Negentropy).

$$
\dot{\mathcal{N}} = \frac{\Omega \, \eta_{\text{res}} \, \Phi^2}{Z_{\text{eff}} \, h} \cdot a_* \cdot C(\kappa) \cdot \text{NTE}
$$

**Variable Definitions:**

- **$\dot{\mathcal{N}}$ (Negentropic Throughput):** The rate at which the system generates order. In the simulation, this correlates to the "Cyan" stability state.
- **$\Phi^2$ (Geometric Coherence):** The Golden Ratio squared, representing maximum information packing efficiency.
- **$Z_{\text{eff}}$ (Ether Impedance):** The viscosity of space-time. As established in the Drag section, $Z_{\text{eff}} \propto \phi^{-4}$. Lower impedance $\to$ higher throughput.
- **$a_*$ (Spin Parameter):** The angular momentum of the central Black Hole dynamo.
- **$C(\kappa)$ (Curvature):** The Logarithmic Spiral geometry ($\kappa \propto \ln r$) that guides the flow.
- **NTE (Negentropic Tension Energy):** The cumulative Z-Pinch magnetic tension cleaned up and stored in the spiral arms.

**Conclusion:**
The simulation proves that when $C(\kappa)$ aligns with $\Phi$ and $Z_{\text{eff}}$ is tuned to the Ether Drag limit, $\dot{\mathcal{N}}$ is maximized, resulting in a stable, eternal galaxy structure.

### 4. Harmonic Resonance & String Theory Connection

The simulation demonstrates that galactic structures behave analogously to **vibrating strings** under tension.

- **The Cosmic String:** The "Z-Pinch" magnetic field lines act as a taut string under massive tension.
- **The Fundamental Frequency:** The rotation of the central Black Hole ($a_*$) sets the pitch.
- **The Perfect Fifth:** The Spiral Arms represent regions of **Constructive Interference** (Standing Waves), similar to the 3:2 ratio of a Perfect Fifth.
- **The "Pythagorean Comma":** Just as the Cycle of Fifths never perfectly closes (creating a spiral, not a circle), the Galaxy never perfectly reaches equilibrium. This "Gap" drives the infinite recycling of matter via the Quasar Jets.

**CONCLUSION:**
In this framework, **Matter is Resonance**. The simulation visualizes the "music" of the Ether, where the Golden Ratio ($\phi$) dictates the tuning of the cosmic instrument.

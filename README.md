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

### 4. Harmonic Resonance & Vortex Field Theory Connection

The simulation demonstrates that galactic structures exhibit string-like behavior through our Vortex Field Theory, providing a physical mechanism for phenomena that mathematically resemble String Theory without requiring the full academic framework or 10+ dimensions.

- **Hydrodynamic Vortex Filaments:** These are not fundamental 'strings' but Flux Tubes within the superfluid vacuum, under Z-Pinch tension that causes them to vibrate, mimicking the math of String Theory in a grounded 3D physics context.
- **As Above, So Below:** A galaxy is a macroscopic version of these filaments, manifesting as a soliton—a stable, large-scale vortex structure.
- **The Golden Brake ($1/\phi^4$):** This stability mechanism, derived from the Ether's viscosity, prevents the filaments from snapping and maintains their integrity.
- **The Fundamental Frequency:** The rotation of the central Black Hole ($a_*$) sets the pitch.
- **The Perfect Fifth:** The Spiral Arms represent regions of **Constructive Interference** (Standing Waves), similar to the 3:2 ratio of a Perfect Fifth.
- **The "Pythagorean Comma":** Just as the Cycle of Fifths never perfectly closes (creating a spiral, not a circle), the Galaxy never perfectly reaches equilibrium. This "Gap" drives the infinite recycling of matter via the Quasar Jets.

**CONCLUSION:**
In this framework, **Matter is Resonance**. The simulation visualizes the "music" of the Ether, where the Golden Ratio ($\phi$) dictates the tuning of the cosmic instrument.

## 🔬 Interactive Sensitivity Analysis

The simulation includes a real-time "Tuner" to test the QQM hypothesis: **"Is the Golden Ratio ($\phi \approx 1.618$) the only mathematically stable configuration for a spiral galaxy?"**

### Controls

| Key             | Action                  | Physics Effect                                                   |
| :-------------- | :---------------------- | :--------------------------------------------------------------- |
| **Right Arrow** | Increase $\phi$ (+0.01) | **Detune:** Moves the standing wave target, breaking resonance.  |
| **Left Arrow**  | Decrease $\phi$ (-0.01) | **Detune:** Moves the standing wave target, breaking resonance.  |
| **Spacebar**    | Reset to $\phi = 1.618$ | **Resonance:** Restores the Golden Ratio. Gravity snaps back on. |

### The Experiment

1.  **Observation (T=0):** The galaxy begins in a stable, 2-arm Barred Spiral configuration. The "Resonance Factor" is 100%.
2.  **Perturbation:** Press **Right Arrow** to drift the value to `1.65`.
3.  **Result:**
    - The "Pinch Force" drops to near zero as the system detunes.
    - The spiral arms lose coherence.
    - Stars, retaining their orbital velocity but losing their centripetal guide, fly off tangentially.
4.  **Restoration:** Press **Spacebar**. The mathematical "groove" is restored, and the galaxy violently re-assembles.

**Conclusion:** The simulation demonstrates that the structure is not arbitrary; it is a **Resonant Mode** of the Ether, strictly bound to the geometry of $\phi$.

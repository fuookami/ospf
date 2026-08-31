# chaotic

:us: English | :cn: [简体中文](README_ch.md)

## Overview

This module provides chaotic systems and iterative maps, including classic attractors like Lorenz, Chen, and Chua circuits, as well as various 2D maps and scalar maps. Each system consists of a system struct for step-by-step iteration and a generator struct for creating infinite sequences.

## Systems

### 3D Systems (Point3-based)

| System | Description |
|--------|-------------|
| `LorenzSystem` | Classic Lorenz attractor (weather convection model) |
| `ChenSystem` | Chen attractor (dual of Lorenz system) |
| `ChenCelikovskySystem` | Chen-Celikovsky system |
| `ChenLeeSystem` | Chen-Lee system |
| `CoulletSystem` | Coullet system |
| `BurkeShawSystem` | Burke-Shaw attractor |
| `BoualiSystem` | Bouali system |
| `AizawaSystem` | Aizawa attractor |
| `AnishchenkoAstakhovSystem` | Anishchenko-Astakhov system |
| `ArneodoSystem` | Arneodo system |
| `ChuaCircuitSystem` | Chua's circuit (3-segment piecewise-linear) |
| `ChuaAttractorSystem` | Chua attractor variant |
| `BrusselatorSystem` | Brusselator (chemical oscillation model) |
| `BiologyChaoticSystem` | Biological chaotic system |
| `CapacitanceEquationSystem` | Capacitance equation system |

### 2D Systems (Point2-based)

| System | Description |
|--------|-------------|
| `ArnoldsCatMapSystem` | Arnold's cat map (area-preserving) |
| `BakersMapSystem` | Baker's map (stretching and folding) |
| `BogdanovMapSystem` | Bogdanov map |
| `CircuitChaoticSystem` | 2D circuit chaotic system |
| `ComplexQuadraticSystem` | Complex quadratic map |
| `ComplexSquaringSystem` | Complex squaring map |

### Scalar Maps

| System | Description |
|--------|-------------|
| `ArnoldTongueSystem` | Arnold tongue (circle map synchronization) |
| `ChebyshevMapSystem` | Chebyshev map |
| `CircleMapSystem` | Circle map |
| `GaussMapSystem` | Gauss map |

### Special Systems

| System | Description |
|--------|-------------|
| `DoublePendulumSystem` | Double pendulum (chaotic mechanical system) |
| `CoupledLorenzSystem` | Coupled Lorenz systems |

## Usage Example

```rust
use ospf_rust_math::chaotic::{LorenzSystem, LorenzSystemGenerator, lorenz_system, lorenz_system_generator};
use ospf_rust_math::geometry::Point3;

// Create a Lorenz system with default parameters
let system = LorenzSystem::default();

// Create a system with custom parameters (a=10, b=28, c=8/3, h=0.01)
let system = LorenzSystem::new(10.0_f64, 28.0, 8.0 / 3.0, 0.01);

// Single step iteration
let next = system.step(Point3::new(1.0, 1.0, 1.0));

// Create an infinite sequence generator
let generator = system.generator(Point3::new(1.0, 1.0, 1.0));

// Or use the convenience function
let mut gen = lorenz_system_generator(10.0, 28.0, 8.0 / 3.0, 0.01, Point3::new(1.0, 1.0, 1.0));

// Iterate (infinite iterator)
for point in gen.take(1000) {
    println!("x={}, y={}, z={}", point.x(), point.y(), point.z());
}
```

## Common Pattern

All chaotic systems follow a consistent pattern:

1. **System struct** - Holds parameters and provides a `step()` method
2. **Generator struct** - Implements `Iterator` for infinite sequences
3. **Convenience functions** - Free functions for easy construction

```rust
// Pattern for each system
pub struct SomeSystem<S> { /* parameters */ }
impl<S> SomeSystem<S> {
    pub fn new(...) -> Self;
    pub fn step(&self, state: State) -> State;
    pub fn generator(self, initial: State) -> SomeSystemGenerator<S>;
}

pub struct SomeSystemGenerator<S> { /* system + state */ }
impl<S> Iterator for SomeSystemGenerator<S> {
    type Item = State;
    fn next(&mut self) -> Option<State>;
}

pub fn some_system(...) -> SomeSystem;
pub fn some_system_generator(...) -> SomeSystemGenerator;
```

## License

MIT License

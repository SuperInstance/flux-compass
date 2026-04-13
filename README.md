# flux-compass

> Heading and orientation engine with spring-damper physics, cardinal direction mapping, and distance calculations for FLUX agents.

## What This Is

`flux-compass` is a Rust crate providing a **2D compass** with smooth heading transitions via spring-damper physics, 8-direction cardinal mapping (N, NE, E, SE, S, SW, W, NW), bearing calculations, distance measurement, and angle-between-point calculations.

## Role in the FLUX Ecosystem

Knowing where you're pointing is fundamental to spatial reasoning:

- **`flux-navigate`** uses compass headings to orient pathfinding direction
- **`flux-perception`** fuses compass data with other sensor readings
- **`flux-simulator`** models vessel heading changes over time
- **`flux-evolve`** optimizes turn-rate and heading accuracy behaviors

## Key Features

| Feature | Description |
|---------|-------------|
| **Spring-Damper Physics** | Smooth heading transitions with configurable turn rate |
| **8 Cardinal Directions** | `direction()` maps heading to N/NE/E/SE/S/SW/W/NW |
| **Facing Check** | `facing(target, tolerance)` for orientation queries |
| **Offset Calculation** | `offset(distance)` returns 2D vector for given heading and range |
| **Angle Diff** | `diff(from, to)` returns shortest angular difference [-180, 180] |
| **Forward Vector** | `forward(heading)` returns unit vector (0°=N, 90°=E) |
| **Distance & Bearing** | Euclidean distance and angle-between-points utilities |

## Quick Start

```rust
use flux_compass::{Compass, Vec2, diff, forward, distance, angle_between};

// Create compass facing North
let mut compass = Compass::new(0.0);

// Turn to face East with smooth physics
compass.set_target(90.0);
for _ in 0..1000 {
    if compass.tick(0.016) { break; } // converged
}
println!("Heading: {:.1}°", compass.heading); // ≈ 90.0
println!("Direction: {:?}", compass.direction()); // E

// Spatial utilities
let origin = Vec2 { x: 0.0, y: 0.0 };
let target = Vec2 { x: 3.0, y: 4.0 };
println!("Distance: {}", distance(origin, target));     // 5.0
println!("Bearing: {:.0}°", angle_between(origin, target)); // 90.0 (East)

// Calculate forward offset
let offset = compass.offset(10.0); // 10 units East
```

## Building & Testing

```bash
cargo build
cargo test
```

## Related Fleet Repos

- [`flux-navigate`](https://github.com/SuperInstance/flux-navigate) — BFS pathfinding on grids
- [`flux-perception`](https://github.com/SuperInstance/flux-perception) — Multi-sensor fusion
- [`flux-simulator`](https://github.com/SuperInstance/flux-simulator) — Fleet simulation
- [`flux-evolve`](https://github.com/SuperInstance/flux-evolve) — Behavioral evolution
- [`flux-memory`](https://github.com/SuperInstance/flux-memory) — Cache heading state

## License

Part of the [SuperInstance](https://github.com/SuperInstance) FLUX fleet.

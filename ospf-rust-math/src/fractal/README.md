# fractal

:us: English | :cn: [简体中文](README_ch.md)

Fractal generation algorithms.

## Sub-modules

| Module | Description |
|--------|-------------|
| `mandelbrot` | Mandelbrot set iteration `z -> z^2 + c` |

## Key Types

| Type | Description |
|------|-------------|
| `MandelbrotSet<S>` | Mandelbrot set iteration function, parameterized by complex constant `c` |
| `MandelbrotSetGenerator<S>` | Infinite sequence generator implementing `Iterator` |

## Usage

```rust
use ospf_rust_math::fractal::MandelbrotSet;

// Create Mandelbrot set with c = -0.5 + 0.5i
let mandelbrot = MandelbrotSet::from_parts(-0.5, 0.5);

// Generate sequence from origin
let mut gen = mandelbrot.generator_from_origin();
let z0 = gen.next_point(); // (0, 0)
let z1 = gen.next_point(); // (-0.5, 0.5)
```

## License

MIT License

# fractal

:us: English | :cn: [简体中文](README_ch.md)

Fractal generation algorithms. All types are generic over `S: Field + Float`, defaulting to `f64`.

## Modules

| Module | Description |
|--------|-------------|
| `mandelbrot` | Mandelbrot set `z -> z^2 + c` |
| `julia` | Julia set `z -> z^2 + c` and Multi Julia set `z -> z^n + c` |

## Key Types

| Type | Description |
|------|-------------|
| `MandelbrotSet<S>` | Mandelbrot set iteration |
| `MandelbrotSetGenerator<S>` | Infinite sequence generator |
| `JuliaSet<S>` | Julia set iteration |
| `JuliaSetGenerator<S>` | Infinite sequence generator |
| `MultiJuliaSet<S>` | Multi Julia set with configurable exponent |
| `MultiJuliaSetGenerator<S>` | Infinite sequence generator |

## License

MIT License

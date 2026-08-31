# fractal

:us: [English](README.md) | :cn: 简体中文

分形生成算法。所有类型均基于泛型 `S: Field + Float`，默认为 `f64`。

## 模块

| 模块 | 说明 |
|------|------|
| `mandelbrot` | Mandelbrot 集 `z -> z^2 + c` |
| `julia` | Julia 集 `z -> z^2 + c` 和多重 Julia 集 `z -> z^n + c` |

## 核心类型

| 类型 | 说明 |
|------|------|
| `MandelbrotSet<S>` | Mandelbrot 集迭代 |
| `MandelbrotSetGenerator<S>` | 无限序列生成器 |
| `JuliaSet<S>` | Julia 集迭代 |
| `JuliaSetGenerator<S>` | 无限序列生成器 |
| `MultiJuliaSet<S>` | 可配指数的多重 Julia 集 |
| `MultiJuliaSetGenerator<S>` | 无限序列生成器 |

## 许可证

MIT License

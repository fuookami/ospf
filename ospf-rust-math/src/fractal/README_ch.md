# fractal

:us: [English](README.md) | :cn: 简体中文

分形生成算法。

## 子模块

| 模块 | 说明 |
|------|------|
| `mandelbrot` | Mandelbrot 集迭代 `z -> z^2 + c` |

## 核心类型

| 类型 | 说明 |
|------|------|
| `MandelbrotSet<S>` | Mandelbrot 集迭代函数，以复常数 `c` 参数化 |
| `MandelbrotSetGenerator<S>` | 实现 `Iterator` 的无限序列生成器 |

## 使用示例

```rust
use ospf_rust_math::fractal::MandelbrotSet;

// 创建 c = -0.5 + 0.5i 的 Mandelbrot 集
let mandelbrot = MandelbrotSet::from_parts(-0.5, 0.5);

// 从原点生成序列
let mut gen = mandelbrot.generator_from_origin();
let z0 = gen.next_point(); // (0, 0)
let z1 = gen.next_point(); // (-0.5, 0.5)
```

## 许可证

MIT License

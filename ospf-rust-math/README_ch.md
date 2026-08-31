# ospf-rust-math

:us: [English](README.md) | :cn: 简体中文

## 简介

`ospf-rust-math` 把 Kotlin `ospf-kotlin-math` 映射为 OSPF 的 Rust 数学基础。它提供代数结构、运算符、普通数学工具、几何、混沌系统、分形、组合数学、符号计算、表达式解析和三值逻辑。

## 作用范围

本 crate 拥有 core 建模、物理量、多维数组集成和领域 framework 使用的可复用数学抽象与算法。

明确非目标：

1. 优化模型装配和 solver 生命周期；这些属于 `ospf-rust-core` 与 `ospf-rust-framework`。
2. 物理单位所有权；这些属于 `ospf-rust-quantities`。
3. 领域专用业务公式，除非它们是可复用数学原语。

数学函数库，提供代数结构、几何实体、混沌系统、符号运算等功能。

## 模块结构

| Rust 模块 | Kotlin 边界 | 职责 |
| --- | --- | --- |
| [`algebra`](src/algebra/README_ch.md) | `math/algebra` | 代数结构和值域/区间。 |
| [`geometry`](src/geometry/README_ch.md) | `math/geometry` | 几何实体和 Delaunay 三角剖分。 |
| [`operator`](src/operator/README_ch.md) | `math/operator` | 绝对值、幂、三角函数、容差比较等数学 operator trait。 |
| [`chaotic`](src/chaotic/README_ch.md) | `math/chaotic` | 混沌系统与迭代映射。 |
| [`fractal`](src/fractal/README_ch.md) | `math/fractal` | 分形生成算法。 |
| [`combinatorics`](src/combinatorics/README_ch.md) | `math/combinatorics` | 组合、排列和笛卡尔积。 |
| [`ordinary`](src/ordinary/README_ch.md) | `math/ordinary` | GCD、LCM、素数、因式分解和常用工具。 |
| [`symbol`](src/symbol/README_ch.md) | `math/symbol` | 符号计算、多项式、不等式、表达式、解析器、宏和 operation trait。 |
| `trivalent` | `math/trivalent` | 三值逻辑。 |

## Public API

| API | 职责 | 稳定性 |
| --- | --- | --- |
| `algebra::*` | algebra concept、law 和 value range。 | stable within migration |
| `geometry::*` | point、vector、edge、triangle、circle、shape 和 triangulation helper。 | migration |
| `operator::*` | 可复用 operator trait。 | stable within migration |
| `ordinary::*` | 常用数学工具。 | stable within migration |
| `symbol::*` | 符号多项式、表达式、不等式、解析器和宏。 | migration |
| `chaotic::*` / `fractal::*` | dynamical-system 和 fractal helper。 | stable within migration |

## 代数结构层次

```
Semigroup (半群)
  └── Monoid (幺半群)
        └── Group (群)
              └── AbelianGroup (阿贝尔群)
                    └── Ring (环)
                          └── CommutativeRing (交换环)
                                └── Field (域)

VectorSpace (向量空间)
  └── NormedSpace (赋范空间)
        └── InnerProductSpace (内积空间)
```

## 混沌系统

支持 28 种混沌系统与迭代映射，包括：

- **三维系统**: Lorenz、Chen、Chen-Celikovsky、Chen-Lee、Coullet、Burke-Shaw、Bouali、Aizawa、Arneodo 等
- **二维系统**: Brusselator、Arnold's Cat Map、Baker's Map、Bogdanov Map 等
- **标量映射**: Circle Map、Arnold Tongue、Chebyshev Map、Gauss Map 等

## 使用示例

```rust
use ospf_rust_math::geometry::{Point2, Point3};
use ospf_rust_math::chaotic::LorenzSystem;

// 创建洛伦兹系统并生成轨迹
let system = LorenzSystem::new(10.0, 28.0, 8.0 / 3.0, 0.01);
let mut gen = system.generator(Point3::new(1.0, 1.0, 1.0));
let next = gen.next_point(); // 执行一步迭代
```

```rust
use ospf_rust_math::fractal::MandelbrotSet;

// 创建 Mandelbrot 集并生成序列
let mandelbrot = MandelbrotSet::from_parts(-0.5, 0.5);
let mut gen = mandelbrot.generator_from_origin();
let z0 = gen.next_point(); // 原点
let z1 = gen.next_point(); // 第一次迭代
```

```rust
use ospf_rust_math::combinatorics::{permutations, combinations_of_size, cross};

// 排列与组合
let perms = permutations(&[1, 2, 3]);                // 6 种排列
let combs = combinations_of_size(&[1, 2, 3, 4], 2);  // 6 种组合

// 笛卡尔积
let result = cross(&[vec![1, 2], vec![3, 4]]);        // 4 种组合
```

## 泛型数值边界

多数 math API 对数值类型和代数 trait 泛型化。具体数值转换应保留在 crate 边界显式完成，尤其是数据进入 solver adapter 或物理量 wrapper 时。

## Features

| Feature | 说明 |
|---------|------|
| `serde` | 序列化/反序列化支持 |
| `parser` | 符号表达式解析器 |

## 本地验证

```powershell
cargo check -p ospf-rust-math
cargo test -p ospf-rust-math
cargo check -p ospf-rust-math --features serde
cargo check -p ospf-rust-math --features parser
```

## 相关模块

- [根 README](../README_ch.md)
- [Quantities README](../ospf-rust-quantities/README_ch.md)
- [Kotlin math README](../../ospf-kotlin/ospf-kotlin-math/README_ch.md)

## 许可证

MIT License

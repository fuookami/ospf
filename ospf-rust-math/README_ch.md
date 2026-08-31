# ospf-rust-math

:us: English | :cn: [简体中文](README_ch.md)

数学函数库，提供代数结构、几何实体、混沌系统、符号运算等功能。

## 模块概览

| 模块 | 说明 |
|------|------|
| **algebra** | 代数结构（群、环、域、向量空间等）及值域/区间 |
| **geometry** | 几何实体（点、向量、边、三角形、圆等）及 Delaunay 三角剖分 |
| **operator** | 数学运算 traits（绝对值、幂、三角函数、容差比较等） |
| **chaotic** | 混沌系统与迭代映射（Lorenz、Chen、各类吸引子等） |
| **fractal** | 分形生成算法（Mandelbrot 集） |
| **combinatorics** | 组合数学（组合、排列、笛卡尔积） |
| **ordinary** | 常规数学函数（GCD、LCM、素数、因式分解） |
| **symbol** | 符号运算（线性规划、二次规划、不等式） |
| **trivalent** | 三值逻辑（真、假、未知） |

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

## Features

| Feature | 说明 |
|---------|------|
| `serde` | 序列化/反序列化支持 |
| `parser` | 符号表达式解析器 |

## 许可证

MIT License

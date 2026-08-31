# chaotic

:us: English | :cn: [简体中文](README_ch.md)

## 概述

本模块提供混沌系统与迭代映射，包括经典的洛伦兹、陈氏、蔡氏电路等吸引子，以及各种二维映射和标量映射。每个系统由系统结构体（用于逐步迭代）和生成器结构体（用于创建无限序列）组成。

## 系统

### 3D 系统（基于 Point3）

| 系统 | 描述 |
|------|------|
| `LorenzSystem` | 经典 Lorenz 吸引子（天气对流模型） |
| `ChenSystem` | Chen 吸引子（Lorenz 系统的对偶） |
| `ChenCelikovskySystem` | Chen-Celikovsky 系统 |
| `ChenLeeSystem` | Chen-Lee 系统 |
| `CoulletSystem` | Coullet 系统 |
| `BurkeShawSystem` | Burke-Shaw 吸引子 |
| `BoualiSystem` | Bouali 系统 |
| `AizawaSystem` | Aizawa 吸引子 |
| `AnishchenkoAstakhovSystem` | Anishchenko-Astakhov 系统 |
| `ArneodoSystem` | Arneodo 系统 |
| `ChuaCircuitSystem` | 蔡氏电路（三段分段线性） |
| `ChuaAttractorSystem` | 蔡氏吸引子变体 |
| `BrusselatorSystem` | 布鲁塞尔振子（化学振荡模型） |
| `BiologyChaoticSystem` | 生物混沌系统 |
| `CapacitanceEquationSystem` | 电容方程系统 |

### 2D 系统（基于 Point2）

| 系统 | 描述 |
|------|------|
| `ArnoldsCatMapSystem` | Arnold 猫映射（保面积映射） |
| `BakersMapSystem` | 面包师映射（拉伸与折叠） |
| `BogdanovMapSystem` | Bogdanov 映射 |
| `CircuitChaoticSystem` | 二维电路混沌系统 |
| `ComplexQuadraticSystem` | 复数二次映射 |
| `ComplexSquaringSystem` | 复数平方映射 |

### 标量映射

| 系统 | 描述 |
|------|------|
| `ArnoldTongueSystem` | Arnold 舌（圆映射同步化） |
| `ChebyshevMapSystem` | Chebyshev 映射 |
| `CircleMapSystem` | 圆映射 |
| `GaussMapSystem` | Gauss 映射 |

### 特殊系统

| 系统 | 描述 |
|------|------|
| `DoublePendulumSystem` | 双摆（混沌力学系统） |
| `CoupledLorenzSystem` | 耦合 Lorenz 系统 |

## 使用示例

```rust
use ospf_rust_math::chaotic::{LorenzSystem, LorenzSystemGenerator, lorenz_system, lorenz_system_generator};
use ospf_rust_math::geometry::Point3;

// 使用默认参数创建 Lorenz 系统
let system = LorenzSystem::default();

// 使用自定义参数创建系统 (a=10, b=28, c=8/3, h=0.01)
let system = LorenzSystem::new(10.0_f64, 28.0, 8.0 / 3.0, 0.01);

// 单步迭代
let next = system.step(Point3::new(1.0, 1.0, 1.0));

// 创建无限序列生成器
let generator = system.generator(Point3::new(1.0, 1.0, 1.0));

// 或使用便捷函数
let mut gen = lorenz_system_generator(10.0, 28.0, 8.0 / 3.0, 0.01, Point3::new(1.0, 1.0, 1.0));

// 迭代（无限迭代器）
for point in gen.take(1000) {
    println!("x={}, y={}, z={}", point.x(), point.y(), point.z());
}
```

## 通用模式

所有混沌系统遵循一致的模式：

1. **系统结构体** - 保存参数，提供 `step()` 方法
2. **生成器结构体** - 实现 `Iterator` 用于无限序列
3. **便捷函数** - 自由函数用于简化构造

```rust
// 每个系统的模式
pub struct SomeSystem<S> { /* 参数 */ }
impl<S> SomeSystem<S> {
    pub fn new(...) -> Self;
    pub fn step(&self, state: State) -> State;
    pub fn generator(self, initial: State) -> SomeSystemGenerator<S>;
}

pub struct SomeSystemGenerator<S> { /* 系统 + 状态 */ }
impl<S> Iterator for SomeSystemGenerator<S> {
    type Item = State;
    fn next(&mut self) -> Option<State>;
}

pub fn some_system(...) -> SomeSystem;
pub fn some_system_generator(...) -> SomeSystemGenerator;
```

## 许可证

MIT License
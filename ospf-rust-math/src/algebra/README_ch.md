# algebra

:us: [English](README.md) | :cn: 简体中文

## 概述

本模块提供完整的代数结构层次定义，从半群到域，以及向量空间、赋范空间、内积空间等。同时提供值空间与区间定义，用于数值计算。

## 子模块

| 模块 | 描述 |
|------|------|
| [`concept`] | 代数概念 traits（群、环、域、向量空间等） |
| [`law`] | 代数定律采样验证器（结合律、交换律、分配律等） |
| [`value_range`] | 值空间与区间定义 |

## 代数结构层次

```
加法结构:                       乘法结构:                       组合结构:
--------                       --------                       --------
Semigroup                      MultiplicativeSemigroup
    │                              │
    ▼                              ▼
Monoid                         MultiplicativeMonoid
    │                              │
    ▼                              ▼
Group                          MultiplicativeGroup
    │                              │
    ▼                              │
AbelianGroup                      │
    │                              │
    └────────────────┬─────────────┘
                       │
                       ▼
                     Ring
                       │
                       ▼
                CommutativeRing
                       │
                       ▼
                     Field

线性代数结构:
------------
VectorSpace
    │
    ▼
NormedSpace
    │
    ▼
InnerProductSpace
```

## 主要类型

### 代数概念 (`concept` 模块)

| Trait | 描述 |
|------|------|
| `Semigroup` | 半群（满足结合律的二元运算） |
| `Monoid` | 幺半群（半群 + 单位元） |
| `Group` | 群（幺半群 + 逆元） |
| `AbelianGroup` | 阿贝尔群/交换群 |
| `MultiplicativeSemigroup` | 乘法半群 |
| `MultiplicativeMonoid` | 乘法幺半群 |
| `MultiplicativeGroup` | 乘法群 |
| `Ring` | 环（加法群 + 乘法半群） |
| `CommutativeRing` | 交换环（环 + 乘法交换律） |
| `Field` | 域（交换环 + 乘法逆元） |
| `VectorSpace` | 向量空间（使用 GAT 定义标量域） |
| `NormedSpace` | 赋范空间（向量空间 + 范数） |
| `InnerProductSpace` | 内积空间（赋范空间 + 内积） |
| `TotallyOrdered` | 全序元素 |
| `Bounded` | 有界元素（具有最小/最大值） |
| `Epsilon` | 默认精度容差 |
| `Fixed` | 固定点/常量元素 |
| `Infinite` | 无穷大支持 |
| `Scalar` | 标量类型标记 |

### 定律验证器 (`law` 模块)

| 结构体 | 描述 |
|--------|------|
| `GroupLaw<T, Add, Neg, Eq>` | 验证结合律、单位元和逆元性质 |
| `RingLaw<T, Add, Mul, Neg, Eq>` | 验证加法群、加法交换律、乘法结合律、乘法单位元和分配律 |
| `FieldLaw<T, Add, Mul, Neg, Recip, IsZero, Eq>` | 验证所有环定律以及乘法交换律和非零元素的乘法逆元 |

### 值空间 (`value_range` 模块)

| 类型 | 描述 |
|------|------|
| `ValueWrapper<T>` | 值包装器，为任意数值类型添加无穷大支持 |
| `Bound<T, I>` | 区间边界，带开闭性质标记 |
| `ValueRange<T, IL, IU>` | 值空间/区间，支持编译时或运行时开闭性质 |
| `IntervalTrait` | 区间开闭性质抽象 trait |
| `Closed` | 闭区间标记类型（编译时） |
| `Open` | 开区间标记类型（编译时） |
| `Interval` | 运行时开闭性质枚举 |

## 使用示例

```rust
use ospf_rust_math::algebra::{Semigroup, Monoid, Group, Field};
use ospf_rust_math::algebra::value_range::{ValueRange, Bound, ValueWrapper, Closed, Open};

// 使用代数 traits
fn compute<T: Field>(a: T, b: T) -> T {
    a + b
}

// 使用值空间
let range: ValueRange<i64, Closed, Closed> = ValueRange::from_bounds(
    Bound::new(ValueWrapper::finite(1), Closed),
    Bound::new(ValueWrapper::finite(10), Closed),
);
```

## 许可证

MIT License

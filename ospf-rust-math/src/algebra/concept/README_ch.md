# concept

:us: [English](README.md) | :cn: 简体中文

本模块定义了完整的代数结构层次 trait，为抽象代数运算提供基础。

## 核心 Trait

### 基础代数结构

| Trait | 描述 | 要求 |
|-------|------|------|
| `Semigroup` | 半群（结合律） | `add` 运算 |
| `Monoid` | 幺半群（半群 + 单位元） | `Semigroup` + `zero` |
| `Group` | 群（幺半群 + 逆元） | `Monoid` + `neg` |
| `AbelianGroup` | 阿贝尔群（群 + 交换律） | `Group` + 交换律 |

### 乘法结构

| Trait | 描述 | 要求 |
|-------|------|------|
| `MultiplicativeSemigroup` | 乘法半群 | `mul` 运算 |
| `MultiplicativeMonoid` | 乘法幺半群 | `MultiplicativeSemigroup` + `one` |
| `MultiplicativeGroup` | 乘法群 | `MultiplicativeMonoid` + `recip` |

### 环与域

| Trait | 描述 | 要求 |
|-------|------|------|
| `Ring` | 环（加法群 + 乘法半群） | `AbelianGroup` + `MultiplicativeMonoid` |
| `CommutativeRing` | 交换环（环 + 乘法交换律） | `Ring` + 交换律 |
| `Field` | 域（交换环 + 乘法逆元） | `CommutativeRing` + `MultiplicativeGroup` |

### 线性代数结构

| Trait | 描述 | 要求 |
|-------|------|------|
| `VectorSpace` | 向量空间（使用 GAT 定义标量域） | `AbelianGroup` + 标量乘法 |
| `NormedSpace` | 赋范空间（向量空间 + 范数） | `VectorSpace` + `norm` |
| `InnerProductSpace` | 内积空间（赋范空间 + 内积） | `NormedSpace` + `dot` |

### 有序结构

| Trait | 描述 |
|-------|------|
| `TotallyOrdered` | 全序比较 |

### 其他性质

| Trait | 描述 |
|-------|------|
| `Bounded` | 有界性（最小/最大值） |
| `Epsilon` | 默认精度容差 |
| `Fixed` | 固定点性质 |
| `Infinite` | 无穷大支持 |
| `Scalar` | 标量类型标记 |

## 引用变体

每个代数 trait 都有对应的 `*Ref` 变体（如 `SemigroupRef`、`MonoidRef`），提供接受引用的运算，支持更灵活的借用模式。

## 使用示例

```rust
use ospf_rust_math::algebra::concept::{Semigroup, Monoid, Group};

// 为自定义类型实现 Semigroup
impl Semigroup for MyType {
    fn add(self, rhs: Self) -> Self {
        // 实现
    }
}
```

## 许可证

本项目采用 MIT 许可证。

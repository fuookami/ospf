# law

:us: English | :cn: [简体中文](README_ch.md)

本模块提供代数定律采样验证器，用于验证类型是否正确实现了其所声明的代数结构。

## 核心类型

| 类型 | 描述 | 验证内容 |
|------|------|----------|
| `GroupLaw<T>` | 群定律采样验证器 | 结合律、单位元、逆元 |
| `RingLaw<T>` | 环定律采样验证器 | 加法群、乘法半群、分配律 |
| `FieldLaw<T>` | 域定律采样验证器 | 环定律 + 乘法逆元 |

## GroupLaw

在有限采样上验证群公理：

- **结合律**：`(a + b) + c == a + (b + c)`
- **单位元**：`a + 0 == a` 且 `0 + a == a`
- **逆元**：`a + (-a) == 0` 且 `(-a) + a == 0`

```rust
use ospf_rust_math::algebra::law::GroupLaw;

let law = GroupLaw::new(
    vec![-2, -1, 0, 1, 2],
    |lhs: &i32, rhs: &i32| lhs + rhs,
    0,
    |value: &i32| -value,
    |lhs: &i32, rhs: &i32| lhs == rhs,
);

assert!(law.validate());
```

## RingLaw

在有限采样上验证环公理：

- **加法群**：结合律、单位元、逆元、交换律
- **乘法结合律**：`(a * b) * c == a * (b * c)`
- **乘法单位元**：`a * 1 == a` 且 `1 * a == a`
- **分配律**：`a * (b + c) == a * b + a * c`

```rust
use ospf_rust_math::algebra::law::RingLaw;

let law = RingLaw::new(
    vec![-2, -1, 0, 1, 2],
    |lhs: &i32, rhs: &i32| lhs + rhs,
    |lhs: &i32, rhs: &i32| lhs * rhs,
    0,
    1,
    |value: &i32| -value,
    |lhs: &i32, rhs: &i32| lhs == rhs,
);

assert!(law.validate());
```

## FieldLaw

在有限采样上验证域公理：

- **环定律**：所有环性质
- **乘法交换律**：`a * b == b * a`
- **乘法逆元**：非零元素 `a` 满足 `a * (1/a) == 1`

```rust
use ospf_rust_math::algebra::law::FieldLaw;

let law = FieldLaw::new(
    vec![-2.0, -1.0, 0.0, 1.0, 2.0],
    |lhs: &f64, rhs: &f64| lhs + rhs,
    |lhs: &f64, rhs: &f64| lhs * rhs,
    0.0,
    1.0,
    |value: &f64| -value,
    |value: &f64| 1.0 / value,
    |value: &f64| value.abs() <= f64::EPSILON,
    |lhs: &f64, rhs: &f64| (lhs - rhs).abs() <= 1e-10,
);

assert!(law.validate());
```

## 单独检查

每个验证器都提供单独的公理检查方法：

- `is_associative()` - 仅检查结合律
- `has_identity()` - 仅检查单位元
- `has_inverse()` - 仅检查逆元
- `validate()` - 检查所有公理

## 许可证

本项目采用 MIT 许可证。

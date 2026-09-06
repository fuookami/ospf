# value_range

:us: [English](README.md) | :cn: 简体中文

本模块提供值空间（区间）的实现，支持无穷大、编译时和运行时开闭性质，以及完整的区间代数运算。

## 核心类型

| 类型 | 描述 |
|------|------|
| `ValueWrapper<T>` | 值包装器，为任意数值类型添加无穷大支持 |
| `Bound<T, I>` | 带开闭标记的边界 |
| `ValueRange<T, IL, IU>` | 带上下界的完整区间 |
| `IntervalTrait` | 开闭性质抽象 trait |
| `Closed` | 闭区间标记类型（编译时） |
| `Open` | 开区间标记类型（编译时） |
| `Interval` | 运行时开闭性质枚举 |

## 类型别名

| 别名 | 描述 |
|------|------|
| `IntervalValue<T>` | 闭区间 `[lower, upper]` |
| `HalfOpenInterval<T>` | 左闭右开区间 `[lower, upper)` |
| `OpenInterval<T>` | 开区间 `(lower, upper)` |
| `DynamicInterval<T>` | 运行时确定开闭性质的区间 |

## 编译时开闭性质（零开销）

使用 `Closed` 或 `Open` 类型标记时，开闭性质在编译时确定：

```rust
use ospf_rust_math::algebra::value_range::{ValueRange, Bound, ValueWrapper, Closed, Open};

// 闭区间 [1, 10]
let range: ValueRange<i64, Closed, Closed> = ValueRange::from_bounds(
    Bound::new(ValueWrapper::finite(1), Closed),
    Bound::new(ValueWrapper::finite(10), Closed),
);

// 左闭右开区间 [1, 10)
let range: ValueRange<i64, Closed, Open> = ValueRange::from_bounds(
    Bound::new(ValueWrapper::finite(1), Closed),
    Bound::new(ValueWrapper::finite(10), Open),
);
```

## 运行时开闭性质（灵活）

使用 `Interval` 枚举时，开闭性质在运行时确定：

```rust
use ospf_rust_math::algebra::value_range::{ValueRange, Bound, ValueWrapper, Interval};

// 半无限区间 [0, +infinity)
let range: ValueRange<i64> = ValueRange::from_bounds(
    Bound::new(ValueWrapper::finite(0), Interval::Closed),
    Bound::new(ValueWrapper::positive_infinity(), Interval::Open),
);

// 根据条件动态决定开闭性质
let lower_is_closed = true;
let lower_interval = if lower_is_closed { Interval::Closed } else { Interval::Open };
let range: ValueRange<i64> = ValueRange::from_bounds(
    Bound::new(ValueWrapper::finite(1), lower_interval),
    Bound::new(ValueWrapper::finite(10), Interval::Closed),
);
```

## 无穷大支持

`ValueWrapper<T>` 为没有原生无穷大的类型添加无穷大支持：

```rust
use ospf_rust_math::algebra::value_range::ValueWrapper;

let finite = ValueWrapper::finite(42);
let pos_inf = ValueWrapper::positive_infinity::<i64>();
let neg_inf = ValueWrapper::negative_infinity::<i64>();
```

## 许可证

本项目采用 MIT 许可证。

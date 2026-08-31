# Value Range - 值空间/区间

[English](README.md)

## 概述

本模块提供了值空间（区间）的实现，主要特性包括：

- 为没有原生无穷大的类型（如 `i64`）添加无穷大概念
- 编译时和运行时开闭性质支持
- 完整的区间代数运算

## 核心组件

| 组件                      | 描述                  |
|-------------------------|---------------------|
| `ValueWrapper<T>`       | 值包装器，为任意数值类型添加无穷大支持 |
| `IntervalTrait`         | 开闭性质抽象 trait        |
| `Closed` / `Open`       | 编译时开闭性质标记类型（零大小类型）  |
| `Interval`              | 运行时开闭性质枚举           |
| `Bound<T, I>`           | 边界，包含值和开闭性质         |
| `ValueRange<T, IL, IU>` | 值空间/区间              |

### 类型别名

| 别名                    | 类型                                  | 描述              |
|-----------------------|-------------------------------------|-----------------|
| `IntervalValue<T>`    | `ValueRange<T, Closed, Closed>`     | 闭区间 `[a, b]`    |
| `HalfOpenInterval<T>` | `ValueRange<T, Closed, Open>`       | 左闭右开区间 `[a, b)` |
| `OpenInterval<T>`     | `ValueRange<T, Open, Open>`         | 开区间 `(a, b)`    |
| `DynamicInterval<T>`  | `ValueRange<T, Interval, Interval>` | 运行时确定开闭性质       |

## 使用示例

### 编译时开闭性质（零开销）

```rust
use ospf_rust_math::algebra::value_range::{ValueRange, Bound, ValueWrapper, Closed, Open};

// 闭区间 [1, 10]
let range: ValueRange<i64, Closed, Closed> = ValueRange::new(
    Bound::new(ValueWrapper::finite(1), Closed),
    Bound::new(ValueWrapper::finite(10), Closed),
);

// 左闭右开区间 [1, 10)
let range: ValueRange<i64, Closed, Open> = ValueRange::new(
    Bound::new(ValueWrapper::finite(1), Closed),
    Bound::new(ValueWrapper::finite(10), Open),
);
```

### 运行时开闭性质（灵活）

```rust
use ospf_rust_math::algebra::value_range::{ValueRange, Bound, ValueWrapper, Interval};

// [0, +∞) - 半无限区间
let range: ValueRange<i64> = ValueRange::new(
    Bound::new(ValueWrapper::finite(0), Interval::Closed),
    Bound::new(ValueWrapper::positive_infinity(), Interval::Open),
);
```

## 性能测试结果

### 测试环境

- CPU: 待补充
- 操作系统: Windows 10
- Rust 版本: edition 2024
- 编译模式: release (optimized)

### ValueWrapper 性能

| 操作      | 时间      |
|---------|---------|
| 创建有限值   | ~659 ps |
| 创建正无穷   | ~225 ps |
| 创建负无穷   | ~242 ps |
| 有限值相等比较 | ~432 ps |
| 有限值小于比较 | ~437 ps |
| 无穷大比较   | ~435 ps |

### IntervalKind 性能

| 操作             | 编译时 (Closed/Open) | 运行时 (Interval) |
|----------------|-------------------|----------------|
| `is_closed()`  | ~219 ps           | ~208 ps        |
| `is_open()`    | ~206 ps           | ~214 ps        |
| `lower_sign()` | ~429 ps           | -              |
| `union()`      | -                 | ~212 ps        |
| `intersect()`  | -                 | ~211 ps        |

**结论**: 编译时和运行时版本的性能基本相同，因为现代编译器对简单枚举判断有极好的优化。

### Bound 性能

| 操作           | 编译时版本       | 运行时版本    |
|--------------|-------------|----------|
| 创建 Bound     | ~637-709 ps | ~1.33 ns |
| `is_above()` | ~657-661 ps | -        |
| `is_below()` | ~592-591 ps | -        |

**结论**: 编译时版本的 Bound 创建比运行时版本快约 2 倍，因为运行时版本需要存储枚举值。

### ValueRange 性能

| 操作                        | 编译时版本    | 运行时版本    | 差异   |
|---------------------------|----------|----------|------|
| 创建 ValueRange             | ~1.24 ns | ~2.61 ns | 2.1x |
| `contains_value()` (闭区间)  | ~740 ps  | ~1.29 ns | 1.7x |
| `contains_value()` (混合开闭) | ~701 ps  | -        | -    |
| `contains_value()` (含无穷大) | ~690 ps  | ~867 ns  | 1.3x |
| 批量 contains (1000次)       | ~1.52 µs | -        | -    |

### 性能结论

1. **编译时版本性能优势明显**
    - 创建操作快约 2 倍
    - `contains` 操作快约 1.7 倍
    - 对于高频调用场景，推荐使用编译时版本

2. **零大小类型 (ZST) 优势**
    - `Closed` 和 `Open` 是零大小类型，不占用内存
    - 编译器可以完全内联优化相关方法

3. **运行时版本灵活性**
    - 性能差异在纳秒级别，对于大多数应用场景可以忽略
    - 在需要动态决定开闭性质的场景下，运行时版本是更好的选择

4. **推荐使用策略**
    - 性能关键路径：使用编译时版本 `ValueRange<T, Closed, Open>`
    - 动态配置场景：使用运行时版本 `ValueRange<T>`
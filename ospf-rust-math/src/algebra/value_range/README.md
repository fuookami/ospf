# Value Range - Value Range / Interval

[中文文档](README_ch.md)

## Overview

This module provides implementation of value ranges (intervals) with the following features:

- Adding infinity concept to types without native infinity (e.g., `i64`)
- Compile-time and runtime openness/closedness support
- Complete interval algebra operations

## Core Components

| Component | Description |
|-----------|-------------|
| `ValueWrapper<T>` | Value wrapper that adds infinity support to any numeric type |
| `IntervalKind` | Openness abstraction trait |
| `Closed` / `Open` | Compile-time openness marker types (zero-sized types) |
| `Interval` | Runtime openness enum |
| `Bound<T, I>` | Boundary containing value and openness |
| `ValueRange<T, IL, IU>` | Value range / interval |

## Usage Examples

### Compile-time Openness (Zero Overhead)

```rust
use ospf_rust_math::algebra::value_range::{ValueRange, Bound, ValueWrapper, Closed, Open};

// Closed interval [1, 10]
let range: ValueRange<i64, Closed, Closed> = ValueRange::new(
    Bound::new(ValueWrapper::finite(1), Closed),
    Bound::new(ValueWrapper::finite(10), Closed),
);

// Half-open interval [1, 10)
let range: ValueRange<i64, Closed, Open> = ValueRange::new(
    Bound::new(ValueWrapper::finite(1), Closed),
    Bound::new(ValueWrapper::finite(10), Open),
);
```

### Runtime Openness (Flexible)

```rust
use ospf_rust_math::algebra::value_range::{ValueRange, Bound, ValueWrapper, Interval};

// [0, +∞) - Half-infinite interval
let range: ValueRange<i64> = ValueRange::new(
    Bound::new(ValueWrapper::finite(0), Interval::Closed),
    Bound::new(ValueWrapper::positive_infinity(), Interval::Open),
);
```

## Performance Benchmark Results

### Test Environment
- CPU: TBD
- OS: Windows 10
- Rust Version: edition 2024
- Build Mode: release (optimized)

### ValueWrapper Performance

| Operation | Time |
|-----------|------|
| Create finite value | ~659 ps |
| Create positive infinity | ~225 ps |
| Create negative infinity | ~242 ps |
| Finite equality comparison | ~432 ps |
| Finite less-than comparison | ~437 ps |
| Infinity comparison | ~435 ps |

### IntervalKind Performance

| Operation | Compile-time (Closed/Open) | Runtime (Interval) |
|-----------|---------------------------|-------------------|
| `is_closed()` | ~219 ps | ~208 ps |
| `is_open()` | ~206 ps | ~214 ps |
| `lower_sign()` | ~429 ps | - |
| `union()` | - | ~212 ps |
| `intersect()` | - | ~211 ps |

**Conclusion**: Compile-time and runtime versions have similar performance because modern compilers excel at optimizing simple enum checks.

### Bound Performance

| Operation | Compile-time Version | Runtime Version |
|-----------|---------------------|-----------------|
| Create Bound | ~637-709 ps | ~1.33 ns |
| `is_above()` | ~657-661 ps | - |
| `is_below()` | ~592-591 ps | - |

**Conclusion**: Compile-time Bound creation is about 2x faster than runtime version because the latter needs to store an enum value.

### ValueRange Performance

| Operation | Compile-time Version | Runtime Version | Ratio |
|-----------|---------------------|-----------------|-------|
| Create ValueRange | ~1.24 ns | ~2.61 ns | 2.1x |
| `contains_value()` (closed) | ~740 ps | ~1.29 ns | 1.7x |
| `contains_value()` (mixed) | ~701 ps | - | - |
| `contains_value()` (with infinity) | ~690 ps | ~867 ns | 1.3x |
| Bulk contains (1000 times) | ~1.52 µs | - | - |

### Performance Conclusions

1. **Compile-time version has clear performance advantage**
   - Creation is ~2x faster
   - `contains` operation is ~1.7x faster
   - Recommended for performance-critical paths

2. **Zero-Sized Type (ZST) advantages**
   - `Closed` and `Open` are zero-sized types, no memory overhead
   - Compiler can fully inline optimize related methods

3. **Runtime version flexibility**
   - Performance difference is at nanosecond level, negligible for most applications
   - Better choice when openness needs to be determined dynamically

4. **Recommended usage strategy**
   - Performance-critical paths: Use compile-time version `ValueRange<T, Closed, Open>`
   - Dynamic configuration scenarios: Use runtime version `ValueRange<T>`
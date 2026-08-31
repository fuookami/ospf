# value_range

:us: English | :cn: [简体中文](README_ch.md)

This module provides value range/interval implementations with support for infinity, compile-time and runtime openness, and complete interval algebra operations.

## Key Types

| Type | Description |
|------|-------------|
| `ValueWrapper<T>` | Value wrapper adding infinity support to any numeric type |
| `Bound<T, I>` | Single boundary with openness marker |
| `ValueRange<T, IL, IU>` | Full interval with lower/upper bounds |
| `IntervalTrait` | Trait for openness abstraction |
| `Closed` | Closed interval marker (compile-time) |
| `Open` | Open interval marker (compile-time) |
| `Interval` | Runtime openness enum |

## Type Aliases

| Alias | Description |
|-------|-------------|
| `IntervalValue<T>` | Closed interval `[lower, upper]` |
| `HalfOpenInterval<T>` | Half-open interval `[lower, upper)` |
| `OpenInterval<T>` | Open interval `(lower, upper)` |
| `DynamicInterval<T>` | Runtime-determined openness |

## Compile-time Openness (Zero Overhead)

Using `Closed` or `Open` type markers, openness is determined at compile time:

```rust
use ospf_rust_math::algebra::value_range::{ValueRange, Bound, ValueWrapper, Closed, Open};

// Closed interval [1, 10]
let range: ValueRange<i64, Closed, Closed> = ValueRange::from_bounds(
    Bound::new(ValueWrapper::finite(1), Closed),
    Bound::new(ValueWrapper::finite(10), Closed),
);

// Half-open interval [1, 10)
let range: ValueRange<i64, Closed, Open> = ValueRange::from_bounds(
    Bound::new(ValueWrapper::finite(1), Closed),
    Bound::new(ValueWrapper::finite(10), Open),
);
```

## Runtime Openness (Flexible)

Using `Interval` enum, openness is determined at runtime:

```rust
use ospf_rust_math::algebra::value_range::{ValueRange, Bound, ValueWrapper, Interval};

// Half-infinite interval [0, +infinity)
let range: ValueRange<i64> = ValueRange::from_bounds(
    Bound::new(ValueWrapper::finite(0), Interval::Closed),
    Bound::new(ValueWrapper::positive_infinity(), Interval::Open),
);

// Dynamic openness based on condition
let lower_is_closed = true;
let lower_interval = if lower_is_closed { Interval::Closed } else { Interval::Open };
let range: ValueRange<i64> = ValueRange::from_bounds(
    Bound::new(ValueWrapper::finite(1), lower_interval),
    Bound::new(ValueWrapper::finite(10), Interval::Closed),
);
```

## Infinity Support

`ValueWrapper<T>` adds infinity support to types without native infinity:

```rust
use ospf_rust_math::algebra::value_range::ValueWrapper;

let finite = ValueWrapper::finite(42);
let pos_inf = ValueWrapper::positive_infinity::<i64>();
let neg_inf = ValueWrapper::negative_infinity::<i64>();
```

## License

This project is licensed under the MIT License.

//! Value Range 性能测试
//! Value Range Performance Benchmarks

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use ospf_rust_math::algebra::value_range::{
    Bound, Closed, Interval, IntervalTrait, Open, ValueRange, ValueWrapper,
};

// ============================================================================
// ValueWrapper 性能测试 / ValueWrapper benchmarks
// ============================================================================

fn bench_value_wrapper_creation(c: &mut Criterion) {
    c.bench_function("value_wrapper_finite_creation", |b| {
        b.iter(|| ValueWrapper::finite(black_box(42_i64)))
    });

    c.bench_function("value_wrapper_positive_infinity_creation", |b| {
        b.iter(|| ValueWrapper::<i64>::positive_infinity())
    });

    c.bench_function("value_wrapper_negative_infinity_creation", |b| {
        b.iter(|| ValueWrapper::<i64>::negative_infinity())
    });
}

fn bench_value_wrapper_comparison(c: &mut Criterion) {
    let val_a = ValueWrapper::finite(42_i64);
    let val_b = ValueWrapper::finite(43_i64);
    let pos_inf = ValueWrapper::<i64>::positive_infinity();
    let neg_inf = ValueWrapper::<i64>::negative_infinity();

    c.bench_function("value_wrapper_finite_eq", |b| {
        b.iter(|| black_box(val_a.clone() == val_a.clone()))
    });

    c.bench_function("value_wrapper_finite_lt", |b| {
        b.iter(|| black_box(val_a.clone() < val_b.clone()))
    });

    c.bench_function("value_wrapper_inf_comparison", |b| {
        b.iter(|| black_box(neg_inf.clone() < pos_inf.clone()))
    });
}

// ============================================================================
// IntervalKind 性能测试 / IntervalKind benchmarks
// ============================================================================

fn bench_interval_kind_compile_time(c: &mut Criterion) {
    c.bench_function("closed_is_closed", |b| b.iter(|| Closed.is_closed()));

    c.bench_function("open_is_open", |b| b.iter(|| Open.is_open()));

    c.bench_function("closed_lower_sign", |b| b.iter(|| Closed.lower_sign()));
}

fn bench_interval_kind_runtime(c: &mut Criterion) {
    let closed = Interval::Closed;
    let open = Interval::Open;

    c.bench_function("interval_closed_is_closed", |b| {
        b.iter(|| closed.is_closed())
    });

    c.bench_function("interval_open_is_open", |b| b.iter(|| open.is_open()));

    c.bench_function("interval_union", |b| b.iter(|| closed.union(&open)));

    c.bench_function("interval_intersect", |b| b.iter(|| closed.intersect(&open)));
}

// ============================================================================
// Bound 性能测试 / Bound benchmarks
// ============================================================================

fn bench_bound_creation(c: &mut Criterion) {
    c.bench_function("bound_closed_creation", |b| {
        b.iter(|| Bound::new(ValueWrapper::finite(black_box(10_i64)), Closed))
    });

    c.bench_function("bound_open_creation", |b| {
        b.iter(|| Bound::new(ValueWrapper::finite(black_box(10_i64)), Open))
    });

    c.bench_function("bound_runtime_creation", |b| {
        b.iter(|| Bound::new(ValueWrapper::finite(black_box(10_i64)), Interval::Closed))
    });
}

fn bench_bound_is_above(c: &mut Criterion) {
    let bound_closed = Bound::new(ValueWrapper::finite(10_i64), Closed);
    let bound_open = Bound::new(ValueWrapper::finite(10_i64), Open);
    let value = ValueWrapper::finite(15_i64);

    c.bench_function("bound_closed_is_above", |b| {
        b.iter(|| bound_closed.is_above(black_box(&value)))
    });

    c.bench_function("bound_open_is_above", |b| {
        b.iter(|| bound_open.is_above(black_box(&value)))
    });
}

fn bench_bound_is_below(c: &mut Criterion) {
    let bound_closed = Bound::new(ValueWrapper::finite(10_i64), Closed);
    let bound_open = Bound::new(ValueWrapper::finite(10_i64), Open);
    let value = ValueWrapper::finite(5_i64);

    c.bench_function("bound_closed_is_below", |b| {
        b.iter(|| bound_closed.is_below(black_box(&value)))
    });

    c.bench_function("bound_open_is_below", |b| {
        b.iter(|| bound_open.is_below(black_box(&value)))
    });
}

// ============================================================================
// ValueRange 性能测试 / ValueRange benchmarks
// ============================================================================

fn bench_value_range_creation(c: &mut Criterion) {
    c.bench_function("value_range_compile_time_creation", |b| {
        b.iter(|| {
            ValueRange::from_bounds(
                Bound::new(ValueWrapper::finite(black_box(1_i64)), Closed),
                Bound::new(ValueWrapper::finite(black_box(10_i64)), Closed),
            )
        })
    });

    c.bench_function("value_range_runtime_creation", |b| {
        b.iter(|| {
            ValueRange::from_bounds(
                Bound::new(ValueWrapper::finite(black_box(1_i64)), Interval::Closed),
                Bound::new(ValueWrapper::finite(black_box(10_i64)), Interval::Open),
            )
        })
    });
}

fn bench_value_range_contains(c: &mut Criterion) {
    // 编译时版本
    let range_closed: ValueRange<i64, Closed, Closed> = ValueRange::new(1, 100);
    let range_mixed: ValueRange<i64, Closed, Open> = ValueRange::new_half_open(1, 100);

    // 运行时版本
    let range_runtime: ValueRange<i64> = ValueRange::from_bounds(
        Bound::new(ValueWrapper::finite(1), Interval::Closed),
        Bound::new(ValueWrapper::finite(100), Interval::Open),
    );

    let value = ValueWrapper::finite(50_i64);

    c.bench_function("value_range_contains_compile_time_closed", |b| {
        b.iter(|| range_closed.contains_value(black_box(&value)))
    });

    c.bench_function("value_range_contains_compile_time_mixed", |b| {
        b.iter(|| range_mixed.contains_value(black_box(&value)))
    });

    c.bench_function("value_range_contains_runtime", |b| {
        b.iter(|| range_runtime.contains_value(black_box(&value)))
    });
}

fn bench_value_range_with_infinity(c: &mut Criterion) {
    // 半无限区间 [0, +∞)
    let range_compile: ValueRange<i64, Closed, Open> = ValueRange::from_bounds(
        Bound::new(ValueWrapper::finite(0), Closed),
        Bound::new(ValueWrapper::positive_infinity(), Open),
    );

    let range_runtime: ValueRange<i64> = ValueRange::from_bounds(
        Bound::new(ValueWrapper::finite(0), Interval::Closed),
        Bound::new(ValueWrapper::positive_infinity(), Interval::Open),
    );

    let value = ValueWrapper::finite(1000000_i64);

    c.bench_function("value_range_contains_infinity_compile_time", |b| {
        b.iter(|| range_compile.contains_value(black_box(&value)))
    });

    c.bench_function("value_range_contains_infinity_runtime", |b| {
        b.iter(|| range_runtime.contains_value(black_box(&value)))
    });
}

// ============================================================================
// 批量操作性能测试 / Bulk operation benchmarks
// ============================================================================

fn bench_value_range_bulk_contains(c: &mut Criterion) {
    let range: ValueRange<i64, Closed, Closed> = ValueRange::new(0, 1000);

    let values: Vec<ValueWrapper<i64>> = (0..1000).map(|v| ValueWrapper::finite(v)).collect();

    c.bench_function("value_range_bulk_1000_contains", |b| {
        b.iter(|| {
            values.iter().for_each(|v| {
                black_box(range.contains_value(v));
            })
        })
    });
}

// ============================================================================
// criterion_group! 宏定义测试组 / Define benchmark groups
// ============================================================================

criterion_group!(
    value_wrapper,
    bench_value_wrapper_creation,
    bench_value_wrapper_comparison,
);

criterion_group!(
    interval_kind,
    bench_interval_kind_compile_time,
    bench_interval_kind_runtime,
);

criterion_group!(
    bound,
    bench_bound_creation,
    bench_bound_is_above,
    bench_bound_is_below,
);

criterion_group!(
    value_range,
    bench_value_range_creation,
    bench_value_range_contains,
    bench_value_range_with_infinity,
    bench_value_range_bulk_contains,
);

criterion_main!(value_wrapper, interval_kind, bound, value_range);

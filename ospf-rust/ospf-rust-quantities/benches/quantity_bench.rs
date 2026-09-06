//! 性能基准测试 - 物理量
//! Performance Benchmark - Physical Quantities
//!
//! 比较编译时物理量（Quantity<V, U: CTUnit>）与运行时物理量（Quantity<V, Unit>）的性能差异
//! Comparing performance between compile-time (Quantity<V, U: CTUnit>) and runtime (Quantity<V, Unit>) physical quantities

use bigdecimal::BigDecimal;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::{Kilogram, Kilometer, Meter, Second};
use ospf_rust_quantities::unit::{CTUnit, Unit};

// ============================================================================
// 创建性能测试 / Creation performance tests
// ============================================================================

fn bench_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("creation");

    // 编译时物理量创建 / Compile-time quantity creation
    group.bench_function("ct_quantity", |b| {
        b.iter(|| {
            let _q: Quantity<BigDecimal, Meter> = Quantity::new_ct(black_box(BigDecimal::from(10)));
        });
    });

    // 运行时物理量创建 / Runtime quantity creation
    group.bench_function("quantity", |b| {
        b.iter(|| {
            let _q = Quantity::new(black_box(BigDecimal::from(10)), Meter::INSTANT.clone());
        });
    });

    group.finish();
}

// ============================================================================
// 加法性能测试 / Addition performance tests
// ============================================================================

fn bench_addition(c: &mut Criterion) {
    let mut group = c.benchmark_group("addition");

    // 编译时物理量加法（引用版本）/ Compile-time quantity addition (reference version)
    group.bench_function("ct_quantity_ref", |b| {
        let ct_q1: Quantity<BigDecimal, Meter> = Quantity::new_ct(BigDecimal::from(10));
        let ct_q2: Quantity<BigDecimal, Meter> = Quantity::new_ct(BigDecimal::from(5));

        b.iter(|| {
            let _sum = black_box(&ct_q1) + black_box(&ct_q2);
        });
    });

    // 运行时物理量加法（相同单位，引用版本）/ Runtime quantity addition (same unit, reference version)
    group.bench_function("quantity_ref_same_unit", |b| {
        let q1 = Quantity::new(BigDecimal::from(10), Meter::INSTANT.clone());
        let q2 = Quantity::new(BigDecimal::from(5), Meter::INSTANT.clone());

        b.iter(|| {
            let _sum = black_box(&q1) + black_box(&q2);
        });
    });

    // 运行时物理量加法（不同单位，引用版本）/ Runtime quantity addition (different units, reference version)
    group.bench_function("quantity_ref_different_units", |b| {
        let q3 = Quantity::new(BigDecimal::from(1), Kilometer::INSTANT.clone());
        let q4 = Quantity::new(BigDecimal::from(500), Meter::INSTANT.clone());

        b.iter(|| {
            let _sum = black_box(&q3) + black_box(&q4);
        });
    });

    group.finish();
}

// ============================================================================
// 减法性能测试 / Subtraction performance tests
// ============================================================================

fn bench_subtraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("subtraction");

    // 编译时物理量减法（引用版本）/ Compile-time quantity subtraction (reference version)
    group.bench_function("ct_quantity_ref", |b| {
        let ct_q1: Quantity<BigDecimal, Meter> = Quantity::new_ct(BigDecimal::from(10));
        let ct_q2: Quantity<BigDecimal, Meter> = Quantity::new_ct(BigDecimal::from(3));

        b.iter(|| {
            let _diff = black_box(&ct_q1) - black_box(&ct_q2);
        });
    });

    // 运行时物理量减法（引用版本）/ Runtime quantity subtraction (reference version)
    group.bench_function("quantity_ref", |b| {
        let q1 = Quantity::new(BigDecimal::from(10), Meter::INSTANT.clone());
        let q2 = Quantity::new(BigDecimal::from(3), Meter::INSTANT.clone());

        b.iter(|| {
            let _diff = black_box(&q1) - black_box(&q2);
        });
    });

    group.finish();
}

// ============================================================================
// 标量乘法性能测试 / Scalar multiplication performance tests
// ============================================================================

fn bench_scalar_multiplication(c: &mut Criterion) {
    let mut group = c.benchmark_group("scalar_multiplication");

    // 编译时物理量标量乘法（引用版本）/ Compile-time quantity scalar multiplication (reference version)
    group.bench_function("ct_quantity_ref", |b| {
        let ct_q: Quantity<BigDecimal, Meter> = Quantity::new_ct(BigDecimal::from(10));
        let scalar = BigDecimal::from(2);

        b.iter(|| {
            let _result = black_box(&ct_q) * black_box(&scalar);
        });
    });

    // 运行时物理量标量乘法（引用版本）/ Runtime quantity scalar multiplication (reference version)
    group.bench_function("quantity_ref", |b| {
        let q = Quantity::new(BigDecimal::from(10), Meter::INSTANT.clone());
        let scalar = BigDecimal::from(2);

        b.iter(|| {
            let _result = black_box(&q) * black_box(&scalar);
        });
    });

    group.finish();
}

// ============================================================================
// 标量除法性能测试 / Scalar division performance tests
// ============================================================================

fn bench_scalar_division(c: &mut Criterion) {
    let mut group = c.benchmark_group("scalar_division");

    // 编译时物理量标量除法（引用版本）/ Compile-time quantity scalar division (reference version)
    group.bench_function("ct_quantity_ref", |b| {
        let ct_q: Quantity<BigDecimal, Meter> = Quantity::new_ct(BigDecimal::from(10));
        let scalar = BigDecimal::from(2);

        b.iter(|| {
            let _result = black_box(&ct_q) / black_box(&scalar);
        });
    });

    // 运行时物理量标量除法（引用版本）/ Runtime quantity scalar division (reference version)
    group.bench_function("quantity_ref", |b| {
        let q = Quantity::new(BigDecimal::from(10), Meter::INSTANT.clone());
        let scalar = BigDecimal::from(2);

        b.iter(|| {
            let _result = black_box(&q) / black_box(&scalar);
        });
    });

    group.finish();
}

// ============================================================================
// 物理量乘法性能测试（产生新量纲）/ Quantity multiplication performance tests (producing new dimensions)
// ============================================================================

fn bench_quantity_multiplication(c: &mut Criterion) {
    let mut group = c.benchmark_group("quantity_multiplication");

    // 编译时物理量乘法（引用版本）/ Compile-time quantity multiplication (reference version)
    group.bench_function("ct_quantity_ref", |b| {
        let ct_length: Quantity<BigDecimal, Meter> = Quantity::new_ct(BigDecimal::from(10));
        let ct_mass: Quantity<BigDecimal, Kilogram> = Quantity::new_ct(BigDecimal::from(5));

        b.iter(|| {
            let _product = black_box(&ct_length) * black_box(&ct_mass);
        });
    });

    // 运行时物理量乘法（引用版本）/ Runtime quantity multiplication (reference version)
    group.bench_function("quantity_ref", |b| {
        let length = Quantity::new(BigDecimal::from(10), Meter::INSTANT.clone());
        let mass = Quantity::new(BigDecimal::from(5), Kilogram::INSTANT.clone());

        b.iter(|| {
            let _product = black_box(&length) * black_box(&mass);
        });
    });

    group.finish();
}

// ============================================================================
// 物理量除法性能测试（产生新量纲）/ Quantity division performance tests (producing new dimensions)
// ============================================================================

fn bench_quantity_division(c: &mut Criterion) {
    let mut group = c.benchmark_group("quantity_division");

    // 编译时物理量除法（引用版本）/ Compile-time quantity division (reference version)
    group.bench_function("ct_quantity_ref", |b| {
        let ct_length: Quantity<BigDecimal, Meter> = Quantity::new_ct(BigDecimal::from(100));
        let ct_time: Quantity<BigDecimal, Second> = Quantity::new_ct(BigDecimal::from(10));

        b.iter(|| {
            let _velocity = black_box(&ct_length) / black_box(&ct_time);
        });
    });

    // 运行时物理量除法（引用版本）/ Runtime quantity division (reference version)
    group.bench_function("quantity_ref", |b| {
        let length = Quantity::new(BigDecimal::from(100), Meter::INSTANT.clone());
        let time = Quantity::new(BigDecimal::from(10), Second::INSTANT.clone());

        b.iter(|| {
            let _velocity = black_box(&length) / black_box(&time);
        });
    });

    group.finish();
}

// ============================================================================
// 单位转换性能测试 / Unit conversion performance tests
// ============================================================================

fn bench_unit_conversion(c: &mut Criterion) {
    let mut group = c.benchmark_group("unit_conversion");

    // 编译时物理量单位转换（编译时检查）/ Compile-time quantity unit conversion (compile-time check)
    group.bench_function("ct_quantity_meter_to_kilometer", |b| {
        b.iter(|| {
            let ct_length_m: Quantity<BigDecimal, Meter> = Quantity::new_ct(BigDecimal::from(1000));
            let _km: Quantity<BigDecimal, Kilometer> = ct_length_m.to();
        });
    });

    // 编译时物理量反向单位转换 / Compile-time quantity reverse unit conversion
    group.bench_function("ct_quantity_kilometer_to_meter", |b| {
        b.iter(|| {
            let ct_length_km: Quantity<BigDecimal, Kilometer> =
                Quantity::new_ct(BigDecimal::from(1));
            let _m: Quantity<BigDecimal, Meter> = ct_length_km.to();
        });
    });

    // 运行时物理量单位转换（运行时检查）/ Runtime quantity unit conversion (runtime check)
    group.bench_function("quantity_meter_to_kilometer", |b| {
        let length_m = Quantity::new(BigDecimal::from(1000), Meter::INSTANT.clone());
        let km_unit = Kilometer::INSTANT.clone();

        b.iter(|| {
            let _km = black_box(&length_m).to_unit(black_box(&km_unit)).unwrap();
        });
    });

    // 运行时物理量反向单位转换 / Runtime quantity reverse unit conversion
    group.bench_function("quantity_kilometer_to_meter", |b| {
        let length_km = Quantity::new(BigDecimal::from(1), Kilometer::INSTANT.clone());
        let m_unit = Meter::INSTANT.clone();

        b.iter(|| {
            let _m = black_box(&length_km).to_unit(black_box(&m_unit)).unwrap();
        });
    });

    group.finish();
}

// ============================================================================
// 编译时转运行时性能测试 / Compile-time to runtime conversion tests
// ============================================================================

fn bench_ct_to_runtime(c: &mut Criterion) {
    let mut group = c.benchmark_group("ct_to_runtime");

    // 编译时转运行时 / Compile-time to runtime conversion
    group.bench_function("ct_to_runtime", |b| {
        let ct_q: Quantity<BigDecimal, Meter> = Quantity::new_ct(BigDecimal::from(10));

        b.iter(|| {
            let _rt = black_box(&ct_q).to_runtime();
        });
    });

    group.finish();
}

// ============================================================================
// 批量操作性能测试 / Batch operation performance tests
// ============================================================================

fn bench_batch_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_operations");

    // 测试不同规模的批量操作 / Test batch operations with different sizes
    for size in [100, 1000, 10000].iter() {
        // 编译时物理量批量加法（引用版本）/ Compile-time quantity batch addition (reference version)
        group.bench_with_input(
            BenchmarkId::new("ct_quantity_add_ref", size),
            size,
            |b, &size| {
                let ct_values: Vec<Quantity<BigDecimal, Meter>> = (0..size)
                    .map(|i| Quantity::new_ct(BigDecimal::from(i as i64)))
                    .collect();

                b.iter(|| {
                    let mut sum: Quantity<BigDecimal, Meter> =
                        Quantity::new_ct(BigDecimal::from(0));
                    for q in &ct_values {
                        sum = &sum + q;
                    }
                    black_box(sum)
                });
            },
        );

        // 运行时物理量批量加法（引用版本）/ Runtime quantity batch addition (reference version)
        group.bench_with_input(
            BenchmarkId::new("quantity_add_ref", size),
            size,
            |b, &size| {
                let values: Vec<Quantity<BigDecimal, Unit>> = (0..size)
                    .map(|i| Quantity::new(BigDecimal::from(i as i64), Meter::INSTANT.clone()))
                    .collect();

                b.iter(|| {
                    let mut sum = Quantity::new(BigDecimal::from(0), Meter::INSTANT.clone());
                    for q in &values {
                        sum = &sum + q;
                    }
                    black_box(sum)
                });
            },
        );

        // 编译时物理量批量乘法（引用版本）/ Compile-time quantity batch multiplication (reference version)
        group.bench_with_input(
            BenchmarkId::new("ct_quantity_mul_ref", size),
            size,
            |b, &size| {
                let ct_lengths: Vec<Quantity<BigDecimal, Meter>> = (0..size)
                    .map(|i| Quantity::new_ct(BigDecimal::from(i as i64 + 1)))
                    .collect();
                let ct_masses: Vec<Quantity<BigDecimal, Kilogram>> = (0..size)
                    .map(|i| Quantity::new_ct(BigDecimal::from(i as i64 + 1)))
                    .collect();

                b.iter(|| {
                    let mut products = Vec::with_capacity(size);
                    for (l, m) in ct_lengths.iter().zip(ct_masses.iter()) {
                        products.push(l * m);
                    }
                    black_box(products)
                });
            },
        );

        // 运行时物理量批量乘法（引用版本）/ Runtime quantity batch multiplication (reference version)
        group.bench_with_input(
            BenchmarkId::new("quantity_mul_ref", size),
            size,
            |b, &size| {
                let lengths: Vec<Quantity<BigDecimal, Unit>> = (0..size)
                    .map(|i| Quantity::new(BigDecimal::from(i as i64 + 1), Meter::INSTANT.clone()))
                    .collect();
                let masses: Vec<Quantity<BigDecimal, Unit>> = (0..size)
                    .map(|i| {
                        Quantity::new(BigDecimal::from(i as i64 + 1), Kilogram::INSTANT.clone())
                    })
                    .collect();

                b.iter(|| {
                    let mut products = Vec::with_capacity(size);
                    for (l, m) in lengths.iter().zip(masses.iter()) {
                        products.push(l * m);
                    }
                    black_box(products)
                });
            },
        );

        // 编译时物理量批量单位转换 / Compile-time quantity batch unit conversion
        group.bench_with_input(
            BenchmarkId::new("ct_quantity_convert", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let ct_lengths_m: Vec<Quantity<BigDecimal, Meter>> = (0..size)
                        .map(|i| Quantity::new_ct(BigDecimal::from(i as i64 * 1000)))
                        .collect();

                    let mut lengths_km = Vec::with_capacity(size);
                    for l in &ct_lengths_m {
                        lengths_km.push(l.clone().to::<Kilometer>());
                    }
                    black_box(lengths_km)
                });
            },
        );

        // 运行时物理量批量单位转换 / Runtime quantity batch unit conversion
        group.bench_with_input(
            BenchmarkId::new("quantity_convert", size),
            size,
            |b, &size| {
                let lengths_m: Vec<Quantity<BigDecimal, Unit>> = (0..size)
                    .map(|i| {
                        Quantity::new(BigDecimal::from(i as i64 * 1000), Meter::INSTANT.clone())
                    })
                    .collect();
                let km_unit = Kilometer::INSTANT.clone();

                b.iter(|| {
                    let mut lengths_km = Vec::with_capacity(size);
                    for l in &lengths_m {
                        lengths_km.push(l.to_unit(&km_unit).unwrap());
                    }
                    black_box(lengths_km)
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// 取负性能测试 / Negation performance tests
// ============================================================================

fn bench_negation(c: &mut Criterion) {
    let mut group = c.benchmark_group("negation");

    // 编译时物理量取负（引用版本）/ Compile-time quantity negation (reference version)
    group.bench_function("ct_quantity_ref", |b| {
        let ct_q: Quantity<BigDecimal, Meter> = Quantity::new_ct(BigDecimal::from(10));

        b.iter(|| {
            let _neg = -black_box(&ct_q);
        });
    });

    // 运行时物理量取负（引用版本）/ Runtime quantity negation (reference version)
    group.bench_function("quantity_ref", |b| {
        let q = Quantity::new(BigDecimal::from(10), Meter::INSTANT.clone());

        b.iter(|| {
            let _neg = -black_box(&q);
        });
    });

    group.finish();
}

// ============================================================================
// 复合运算性能测试 / Compound operation performance tests
// ============================================================================

fn bench_compound_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("compound_operations");

    // 编译时物理量复合运算（引用版本）: (&a + &b) * &c / &d
    // Compile-time quantity compound operation (reference version): (&a + &b) * &c / &d
    group.bench_function("ct_quantity_ref", |b| {
        let ct_a: Quantity<BigDecimal, Meter> = Quantity::new_ct(BigDecimal::from(10));
        let ct_b: Quantity<BigDecimal, Meter> = Quantity::new_ct(BigDecimal::from(5));
        let ct_c: Quantity<BigDecimal, Kilogram> = Quantity::new_ct(BigDecimal::from(2));
        let ct_d: Quantity<BigDecimal, Second> = Quantity::new_ct(BigDecimal::from(4));

        b.iter(|| {
            let sum = &ct_a + &ct_b;
            let product = &sum * &ct_c;
            let _result = &product / &ct_d;
        });
    });

    // 运行时物理量复合运算（引用版本）: (&a + &b) * &c / &d
    // Runtime quantity compound operation (reference version): (&a + &b) * &c / &d
    group.bench_function("quantity_ref", |bencher| {
        let a = Quantity::new(BigDecimal::from(10), Meter::INSTANT.clone());
        let b = Quantity::new(BigDecimal::from(5), Meter::INSTANT.clone());
        let c = Quantity::new(BigDecimal::from(2), Kilogram::INSTANT.clone());
        let d = Quantity::new(BigDecimal::from(4), Second::INSTANT.clone());

        bencher.iter(|| {
            let sum = &a + &b;
            let product = &sum * &c;
            let _result = &product / &d;
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_creation,
    bench_addition,
    bench_subtraction,
    bench_scalar_multiplication,
    bench_scalar_division,
    bench_quantity_multiplication,
    bench_quantity_division,
    bench_unit_conversion,
    bench_ct_to_runtime,
    bench_batch_operations,
    bench_negation,
    bench_compound_operations,
);

criterion_main!(benches);

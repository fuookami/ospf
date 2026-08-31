//! # Performance Benchmarks for ospf-rust-multiarray
//! 本基准测试套件用于 ospf-rust-multiarray 的性能测试
//!
//! This benchmark suite compares:
//! 1. Compile-time dimensions vs Runtime dimensions
//! 2. Row-major vs Column-major storage order
//!
//! 本基准测试套件比较：
//! 1. 编译时维度 vs 运行时维度
//! 2. 行优先 vs 列优先存储顺序

use cc_traits::Iter;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ospf_rust_multiarray::*;

// ============================================================================
// Helper functions for creating test arrays
// 辅助函数：创建测试数组
// ============================================================================

/// Create a compile-time 3D array with fixed dimensions (Row-Major)
/// 创建具有固定维度的编译时 3D 数组（行优先）
fn create_ct_array_3d_rm(size: usize) -> MultiArray<f64, Shape<3, RowMajor>> {
    let shape = Shape::<3, RowMajor>::new([size, size, size]);
    MultiArrayBuilder::new_by(shape, |idx, _| idx as f64)
}

/// Create a compile-time 3D array with fixed dimensions (Column-Major)
/// 创建具有固定维度的编译时 3D 数组（列优先）
fn create_ct_array_3d_cm(size: usize) -> MultiArray<f64, Shape<3, ColumnMajor>> {
    let shape = Shape::<3, ColumnMajor>::new([size, size, size]);
    MultiArrayBuilder::new_by(shape, |idx, _| idx as f64)
}

/// Create a runtime 3D array with dynamic dimensions and runtime storage order
/// 创建具有动态维度和运行时存储顺序的运行时 3D 数组
fn create_rt_array_3d_dyn(size: usize) -> MultiArray<f64, DynShape> {
    let shape = DynShape::new(vec![size, size, size]);
    MultiArrayBuilder::new_by(shape, |idx, _| idx as f64)
}

// ============================================================================
// Benchmark 1: Compile-time dimensions vs Runtime dimensions
// 基准测试 1：编译时维度 vs 运行时维度
// ============================================================================

fn bench_dimension_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("dimension_comparison");
    let size = 32; // 32^3 = 32768 elements
    group.throughput(Throughput::Elements((size * size * size) as u64));

    // Compile-time fixed dimension (Shape<3, RowMajor>)
    // 编译时固定维度 (Shape<3, RowMajor>)
    group.bench_with_input(
        BenchmarkId::new("CT fixed dim", "Shape<3, RowMajor>"),
        &size,
        |b, &size| {
            let array = create_ct_array_3d_rm(size);
            b.iter(|| {
                let mut sum = 0.0;
                for &val in array.iter() {
                    sum += black_box(val);
                }
                sum
            })
        },
    );

    // Runtime dimension with runtime storage order (DynShape)
    // 运行时维度，运行时存储顺序 (DynShape)
    group.bench_with_input(
        BenchmarkId::new("RT dim + RT storage", "DynShape"),
        &size,
        |b, &size| {
            let array = create_rt_array_3d_dyn(size);
            b.iter(|| {
                let mut sum = 0.0;
                for &val in array.iter() {
                    sum += black_box(val);
                }
                sum
            })
        },
    );

    group.finish();
}

// ============================================================================
// Benchmark 2: Storage order comparison
// 基准测试 2：存储顺序比较
// ============================================================================

fn bench_storage_order_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage_order_comparison");
    let size = 64;
    group.throughput(Throughput::Elements((size * size) as u64));

    // CT Row-Major storage iteration
    // CT 行优先存储迭代
    let array_ct_rm = create_ct_array_3d_rm(size);
    group.bench_function("CT Row-Major storage (3D)", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for &val in array_ct_rm.iter() {
                sum += black_box(val);
            }
            sum
        })
    });

    // CT Column-Major storage iteration
    // CT 列优先存储迭代
    let array_ct_cm = create_ct_array_3d_cm(size);
    group.bench_function("CT Column-Major storage (3D)", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for &val in array_ct_cm.iter() {
                sum += black_box(val);
            }
            sum
        })
    });

    group.finish();
}

// ============================================================================
// Benchmark 3: Index access performance
// 基准测试 3：索引访问性能
// ============================================================================

fn bench_index_access_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("index_access_comparison");
    let size = 100;
    group.throughput(Throughput::Elements((size * size) as u64));

    // CT Array with fixed dimension - array index access
    // CT 数组，固定维度 - 数组索引访问
    let shape_ct = Shape::<2, RowMajor>::new([size, size]);
    let array_ct: MultiArray<f64, _> = MultiArrayBuilder::new_by(shape_ct, |idx, _| idx as f64);

    group.bench_function("CT fixed dim - array index", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for i in 0..size {
                for j in 0..size {
                    sum += black_box(array_ct[&[i, j]]);
                }
            }
            sum
        })
    });

    // RT Array with dynamic dimension - vec index access
    // RT 数组，动态维度 - 向量索引访问
    let shape_rt = DynShape::new(vec![size, size]);
    let array_rt: MultiArray<f64, DynShape> = MultiArrayBuilder::new_by(shape_rt, |idx, _| idx as f64);

    group.bench_function("RT dynamic dim - vec index", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for i in 0..size {
                for j in 0..size {
                    sum += black_box(array_rt[&vec![i, j]]);
                }
            }
            sum
        })
    });

    // CT fixed dimension - flat index access
    // CT 固定维度 - 扁平索引访问
    group.bench_function("CT fixed dim - flat index", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for i in 0..(size * size) {
                sum += black_box(array_ct[i]);
            }
            sum
        })
    });

    // RT dynamic dimension - flat index access
    // RT 动态维度 - 扁平索引访问
    group.bench_function("RT dynamic dim - flat index", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for i in 0..(size * size) {
                sum += black_box(array_rt[i]);
            }
            sum
        })
    });

    group.finish();
}

// ============================================================================
// Benchmark 4: View creation and iteration
// 基准测试 4：视图创建和迭代
// ============================================================================

fn bench_view_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("view_comparison");
    let size = 50;
    group.throughput(Throughput::Elements((size * size) as u64));

    // Setup arrays with runtime storage order (required for view builders)
    // 设置运行时存储顺序的数组（视图构建器需要）
    let shape_rt = DynShape::new(vec![size * 2, size * 2]);
    let array_rt: MultiArray<f64, DynShape> = MultiArrayBuilder::new_by(shape_rt, |idx, _| idx as f64);

    // RT View - sliced iteration
    // RT 视图 - 切片迭代
    group.bench_function("RT View - sliced iteration", |b| {
        b.iter(|| {
            let dummy = dyn_dummy_expect![0..size, 0..size];
            let view = array_rt.view(&dummy).unwrap();
            let mut sum = 0.0;
            for &val in view.iter() {
                sum += black_box(val);
            }
            sum
        })
    });

    // RT View - map view (dimension reordering)
    // RT 视图 - 映射视图（维度重排）
    group.bench_function("RT View - map view (transpose)", |b| {
        b.iter(|| {
            let map_vector = dyn_map_expect![_1, _0];
            let view = array_rt.map_view(&map_vector).unwrap();
            let mut sum = 0.0;
            for &val in view.iter() {
                sum += black_box(val);
            }
            sum
        })
    });

    group.finish();
}

// ============================================================================
// Benchmark 5: Combined comparison - All factors
// 基准测试 5：综合比较 - 所有因素
// ============================================================================

fn bench_combined_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("combined_comparison");
    let size = 32;
    group.throughput(Throughput::Elements((size * size * size) as u64));

    // CT Array with fixed dimension + Row-Major
    // CT 数组：固定维度 + 行优先
    let array_ct_rm = create_ct_array_3d_rm(size);

    // CT Array with fixed dimension + Column-Major
    // CT 数组：固定维度 + 列优先
    let array_ct_cm = create_ct_array_3d_cm(size);

    // RT Array with dynamic dimension + runtime storage order
    // RT 数组：动态维度 + 运行时存储顺序
    let array_rt_dyn = create_rt_array_3d_dyn(size);

    // Full CT (fixed dim + Row-Major)
    // 完全 CT（固定维度 + 行优先）
    group.bench_function("Full CT (fixed dim + RM)", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for &val in array_ct_rm.iter() {
                sum += black_box(val);
            }
            sum
        })
    });

    // Full CT (fixed dim + Column-Major)
    // 完全 CT（固定维度 + 列优先）
    group.bench_function("Full CT (fixed dim + CM)", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for &val in array_ct_cm.iter() {
                sum += black_box(val);
            }
            sum
        })
    });

    // Full RT (dynamic dim + runtime storage)
    // 完全 RT（动态维度 + 运行时存储）
    group.bench_function("Full RT (dynamic dim + RT storage)", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for &val in array_rt_dyn.iter() {
                sum += black_box(val);
            }
            sum
        })
    });

    // Flat index access comparison
    // 扁平索引访问比较
    group.bench_function("RT flat index access", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for i in 0..array_rt_dyn.len() {
                sum += black_box(array_rt_dyn[i]);
            }
            sum
        })
    });

    group.finish();
}

// ============================================================================
// Benchmark 6: Index calculation performance
// 基准测试 6：索引计算性能
// ============================================================================

fn bench_index_calculation_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("index_calculation");
    let size = 100;
    let iterations = 10000;

    // CT Shape (fixed dimension)
    // CT 形状（固定维度）
    let shape_ct: Shape<3, RowMajor> = Shape::new([size, size, size]);

    // RT Shape (dynamic dimension, RT storage order)
    // RT 形状（动态维度，运行时存储顺序）
    let shape_rt: DynShape = DynShape::new(vec![size, size, size]);

    // CT Shape - index_of calculation (array index)
    // CT 形状 - index_of 计算（数组索引）
    group.bench_function("CT Shape - index_of (array)", |b| {
        b.iter(|| {
            let mut idx = 0usize;
            for _ in 0..iterations {
                let vector = [idx % size, (idx / size) % size, idx / (size * size)];
                idx = black_box(shape_ct.index_of(&vector).unwrap());
                idx = (idx + 1) % (size * size * size);
            }
            idx
        })
    });

    // RT Shape - index_of calculation (vec index)
    // RT 形状 - index_of 计算（向量索引）
    group.bench_function("RT Shape - index_of (vec)", |b| {
        b.iter(|| {
            let mut idx = 0usize;
            for _ in 0..iterations {
                let vector = vec![idx % size, (idx / size) % size, idx / (size * size)];
                idx = black_box(shape_rt.index_of(&vector).unwrap());
                idx = (idx + 1) % (size * size * size);
            }
            idx
        })
    });

    // CT Shape - vector_of calculation
    // CT 形状 - vector_of 计算
    group.bench_function("CT Shape - vector_of", |b| {
        b.iter(|| {
            let mut idx = 0usize;
            for _ in 0..iterations {
                let vector = black_box(shape_ct.vector_of(idx).unwrap());
                idx = (idx + 1) % (size * size * size);
                let _ = vector;
            }
            idx
        })
    });

    // RT Shape - vector_of calculation
    // RT 形状 - vector_of 计算
    group.bench_function("RT Shape - vector_of", |b| {
        b.iter(|| {
            let mut idx = 0usize;
            for _ in 0..iterations {
                let vector = black_box(shape_rt.vector_of(idx).unwrap());
                idx = (idx + 1) % (size * size * size);
                let _ = vector;
            }
            idx
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_dimension_benchmark,
    bench_storage_order_benchmark,
    bench_index_access_benchmark,
    bench_view_benchmark,
    bench_combined_benchmark,
    bench_index_calculation_benchmark,
);

criterion_main!(benches);
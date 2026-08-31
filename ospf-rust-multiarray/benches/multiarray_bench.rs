//! # Performance Benchmarks for ospf-rust-multiarray
//!
//! This benchmark suite compares:
//! 1. Compile-time dimensions vs Runtime dimensions
//! 2. Compile-time access order vs Runtime access order
//! 3. Compile-time storage order vs Runtime storage order
//!
//! 本基准测试套件比较：
//! 1. 编译时维度 vs 运行时维度
//! 2. 编译时访问顺序 vs 运行时访问顺序
//! 3. 编译时存储顺序 vs 运行时存储顺序

use cc_traits::Iter;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ospf_rust_multiarray::*;

// ============================================================================
// Helper functions for creating test arrays
// 辅助函数：创建测试数组
// ============================================================================

/// Create a compile-time 3D array with fixed dimensions
/// 创建具有固定维度的编译时 3D 数组
fn create_ct_array_3d(size: usize) -> MultiArrayRM<f64, ShapeRM<3>> {
    let shape = ShapeRM::<3>::new([size, size, size]);
    CTMultiArrayBuilder::new_by(shape, |idx, _| idx as f64)
}

/// Create a runtime 3D array with dynamic dimensions
/// 创建具有动态维度的运行时 3D 数组
fn create_rt_array_3d(size: usize) -> MultiArray<f64, DynShape> {
    let shape = DynShape::new(vec![size, size, size]);
    MultiArrayBuilder::new_by(shape, |idx, _| idx as f64)
}

/// Create a compile-time dynamic shape array (dimension at runtime, storage order at compile time)
/// 创建编译时动态形状数组（维度在运行时，存储顺序在编译时）
fn create_ct_dyn_array_3d(size: usize) -> MultiArrayRM<f64, DynShapeRM> {
    let shape = DynShapeRM::<Vec<usize>, Vec<DummyIndex>>::new(vec![size, size, size]);
    CTMultiArrayBuilder::new_by(shape, |idx, _| idx as f64)
}

// ============================================================================
// Benchmark 1: Compile-time dimensions vs Runtime dimensions
// 基准测试 1：编译时维度 vs 运行时维度
// ============================================================================

fn bench_dimension_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("dimension_comparison");
    let size = 32; // 32^3 = 32768 elements
    group.throughput(Throughput::Elements((size * size * size) as u64));

    // Compile-time fixed dimension (CTShape<3, RowMajor>)
    // 编译时固定维度 (CTShape<3, RowMajor>)
    group.bench_with_input(
        BenchmarkId::new("Compile-time fixed dimension", "CTShape<3>"),
        &size,
        |b, &size| {
            let array = create_ct_array_3d(size);
            b.iter(|| {
                let mut sum = 0.0;
                for &val in array.iter() {
                    sum += black_box(val);
                }
                sum
            })
        },
    );

    // Compile-time dynamic dimension (CTDynShape<RowMajor>)
    // 编译时动态维度 (CTDynShape<RowMajor>)
    group.bench_with_input(
        BenchmarkId::new("Compile-time dynamic dimension", "CTDynShape"),
        &size,
        |b, &size| {
            let array = create_ct_dyn_array_3d(size);
            b.iter(|| {
                let mut sum = 0.0;
                for &val in array.iter() {
                    sum += black_box(val);
                }
                sum
            })
        },
    );

    // Runtime dimension (DynShape)
    // 运行时维度 (DynShape)
    group.bench_with_input(
        BenchmarkId::new("Runtime dimension", "DynShape"),
        &size,
        |b, &size| {
            let array = create_rt_array_3d(size);
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
// Benchmark 2: Compile-time access order vs Runtime access order
// 基准测试 2：编译时访问顺序 vs 运行时访问顺序
// ============================================================================

fn bench_access_order_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("access_order_comparison");
    let size = 64; // 64^2 = 4096 elements
    group.throughput(Throughput::Elements((size * size) as u64));

    // Prepare arrays
    // 准备数组
    let shape_rm = ShapeRM::<2>::new([size, size]);
    let array_rm: MultiArrayRM<f64, _> = CTMultiArrayBuilder::new_by(shape_rm.clone(), |idx, _| idx as f64);

    let shape_cm = ShapeCM::<2>::new([size, size]);
    let array_cm: MultiArrayCM<f64, _> = CTMultiArrayBuilder::new_by(shape_cm.clone(), |idx, _| idx as f64);

    let rt_shape = DynShape::new(vec![size, size]);
    let array_rt: MultiArray<f64, DynShape> = MultiArrayBuilder::new_by(rt_shape, |idx, _| idx as f64);

    // Create views
    // 创建视图
    let dummy_full = dummy_expect![.., ..];

    // CT View with Row-Major access (compile-time)
    // CT 视图，行优先访问（编译时）
    let view_ct_rm: MultiArrayViewRM<'_, f64, ShapeRM<2>> =
        MultiArrayViewBuilderRM::new_by_dummy(&array_rm, &dummy_full);

    // CT View with Column-Major access (compile-time)
    // CT 视图，列优先访问（编译时）
    let view_ct_cm: MultiArrayViewCM<'_, f64, ShapeCM<2>> =
        MultiArrayViewBuilderCM::new_by_dummy(&array_cm, &dummy_full);

    // RT View with runtime access order
    // RT 视图，运行时访问顺序
    let view_rt = array_rt.view(&dyn_dummy_expect![.., ..]).unwrap();

    // Compile-time Row-Major access
    // 编译时行优先访问
    group.bench_function("CT Row-Major access", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for &val in view_ct_rm.iter() {
                sum += black_box(val);
            }
            sum
        })
    });

    // Compile-time Column-Major access
    // 编译时列优先访问
    group.bench_function("CT Column-Major access", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for &val in view_ct_cm.iter() {
                sum += black_box(val);
            }
            sum
        })
    });

    // Runtime Row-Major access
    // 运行时行优先访问
    group.bench_function("RT Row-Major access", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for val in view_rt.iter_with_order(AccessOrder::RowMajor) {
                sum += black_box(*val);
            }
            sum
        })
    });

    // Runtime Column-Major access
    // 运行时列优先访问
    group.bench_function("RT Column-Major access", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for val in view_rt.iter_with_order(AccessOrder::ColumnMajor) {
                sum += black_box(*val);
            }
            sum
        })
    });

    group.finish();
}

// ============================================================================
// Benchmark 3: Compile-time storage order vs Runtime storage order
// 基准测试 3：编译时存储顺序 vs 运行时存储顺序
// ============================================================================

fn bench_storage_order_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage_order_comparison");
    let size = 64;
    group.throughput(Throughput::Elements((size * size) as u64));

    // Compile-time Row-Major storage
    // 编译时行优先存储
    let shape_rm = ShapeRM::<2>::new([size, size]);
    let array_ct_rm: MultiArrayRM<f64, _> = CTMultiArrayBuilder::new_by(shape_rm, |idx, _| idx as f64);

    // Compile-time Column-Major storage
    // 编译时列优先存储
    let shape_cm = ShapeCM::<2>::new([size, size]);
    let array_ct_cm: MultiArrayCM<f64, _> = CTMultiArrayBuilder::new_by(shape_cm, |idx, _| idx as f64);

    // Runtime Row-Major storage
    // 运行时行优先存储
    let shape_rt_rm = Shape::new([size, size]);
    let array_rt_rm: MultiArray<f64, Shape<2>> = MultiArrayBuilder::new_by(shape_rt_rm, |idx, _| idx as f64);

    // Runtime Column-Major storage
    // 运行时列优先存储
    let shape_rt_cm = DynShape::new_with_order(vec![size, size], StorageOrder::ColumnMajor);
    let array_rt_cm: MultiArray<f64, DynShape> = MultiArrayBuilder::new_by(shape_rt_cm, |idx, _| idx as f64);

    // CT Row-Major storage iteration
    // CT 行优先存储迭代
    group.bench_function("CT Row-Major storage", |b| {
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
    group.bench_function("CT Column-Major storage", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for &val in array_ct_cm.iter() {
                sum += black_box(val);
            }
            sum
        })
    });

    // RT Row-Major storage iteration
    // RT 行优先存储迭代
    group.bench_function("RT Row-Major storage", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for &val in array_rt_rm.iter() {
                sum += black_box(val);
            }
            sum
        })
    });

    // RT Column-Major storage iteration
    // RT 列优先存储迭代
    group.bench_function("RT Column-Major storage", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for &val in array_rt_cm.iter() {
                sum += black_box(val);
            }
            sum
        })
    });

    group.finish();
}

// ============================================================================
// Benchmark 4: Index access performance
// 基准测试 4：索引访问性能
// ============================================================================

fn bench_index_access_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("index_access_comparison");
    let size = 100;
    group.throughput(Throughput::Elements((size * size) as u64));

    // CT Array with fixed dimension
    // CT 数组，固定维度
    let shape_ct = ShapeRM::<2>::new([size, size]);
    let array_ct: MultiArrayRM<f64, _> = CTMultiArrayBuilder::new_by(shape_ct.clone(), |idx, _| idx as f64);

    // CT Dyn Array
    // CT 动态数组
    let shape_ct_dyn = DynShapeRM::<Vec<usize>, Vec<DummyIndex>>::new(vec![size, size]);
    let array_ct_dyn: MultiArrayRM<f64, _> = CTMultiArrayBuilder::new_by(shape_ct_dyn, |idx, _| idx as f64);

    // RT Array
    // RT 数组
    let shape_rt = DynShape::new(vec![size, size]);
    let array_rt: MultiArray<f64, DynShape> = MultiArrayBuilder::new_by(shape_rt, |idx, _| idx as f64);

    // CT fixed dimension - vector index access
    // CT 固定维度 - 向量索引访问
    group.bench_function("CT fixed dim - vector index", |b| {
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

    // CT dynamic dimension - vector index access
    // CT 动态维度 - 向量索引访问
    group.bench_function("CT dynamic dim - vector index", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for i in 0..size {
                for j in 0..size {
                    sum += black_box(array_ct_dyn[&vec![i, j]]);
                }
            }
            sum
        })
    });

    // RT dynamic dimension - vector index access
    // RT 动态维度 - 向量索引访问
    group.bench_function("RT dynamic dim - vector index", |b| {
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
// Benchmark 5: View creation and iteration
// 基准测试 5：视图创建和迭代
// ============================================================================

fn bench_view_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("view_comparison");
    let size = 50;
    group.throughput(Throughput::Elements((size * size) as u64));

    // Setup arrays
    // 设置数组
    let shape_ct = ShapeRM::<2>::new([size * 2, size * 2]);
    let array_ct: MultiArrayRM<f64, _> = CTMultiArrayBuilder::new_by(shape_ct, |idx, _| idx as f64);

    let shape_rt = DynShape::new(vec![size * 2, size * 2]);
    let array_rt: MultiArray<f64, DynShape> = MultiArrayBuilder::new_by(shape_rt, |idx, _| idx as f64);

    // CT View - sliced iteration
    // CT 视图 - 切片迭代
    group.bench_function("CT View - sliced iteration", |b| {
        b.iter(|| {
            let dummy = dummy_expect![0..size, 0..size];
            let view = MultiArrayViewBuilderRM::new_by_dummy(&array_ct, &dummy);
            let mut sum = 0.0;
            for &val in view.iter() {
                sum += black_box(val);
            }
            sum
        })
    });

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

    // CT View - map view (dimension reordering)
    // CT 视图 - 映射视图（维度重排）
    group.bench_function("CT View - map view", |b| {
        b.iter(|| {
            let map_vector = map_expect![_1, _0];
            let view = MultiArrayViewBuilderRM::new_by_map(&array_ct, map_vector).unwrap();
            let mut sum = 0.0;
            for &val in view.iter() {
                sum += black_box(val);
            }
            sum
        })
    });

    // RT View - map view (dimension reordering)
    // RT 视图 - 映射视图（维度重排）
    group.bench_function("RT View - map view", |b| {
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
// Benchmark 6: Combined comparison - All factors
// 基准测试 6：综合比较 - 所有因素
// ============================================================================

fn bench_combined_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("combined_comparison");
    let size = 32;
    group.throughput(Throughput::Elements((size * size * size) as u64));

    // CT Array with fixed dimension + CT storage order + CT access order
    // CT 数组：固定维度 + CT 存储顺序 + CT 访问顺序
    let shape_ct_rm = ShapeRM::<3>::new([size, size, size]);
    let array_ct_rm: MultiArrayRM<f64, _> = CTMultiArrayBuilder::new_by(shape_ct_rm.clone(), |idx, _| idx as f64);

    // CT Array with fixed dimension + CT Column-Major storage + CT Column-Major access
    // CT 数组：固定维度 + CT 列优先存储 + CT 列优先访问
    let shape_ct_cm = ShapeCM::<3>::new([size, size, size]);
    let array_ct_cm: MultiArrayCM<f64, _> = CTMultiArrayBuilder::new_by(shape_ct_cm.clone(), |idx, _| idx as f64);

    // RT Array with runtime dimension + RT storage order + RT access order
    // RT 数组：运行时维度 + RT 存储顺序 + RT 访问顺序
    let shape_rt = DynShape::new(vec![size, size, size]);
    let array_rt: MultiArray<f64, DynShape> = MultiArrayBuilder::new_by(shape_rt, |idx, _| idx as f64);

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

    // Full RT (dynamic dim + runtime access)
    // 完全 RT（动态维度 + 运行时访问）
    group.bench_function("Full RT (dynamic dim + RT access)", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for &val in array_rt.iter() {
                sum += black_box(val);
            }
            sum
        })
    });

    // Mixed: CT dimension + RT storage order
    // 混合：CT 维度 + RT 存储顺序
    group.bench_function("Mixed (CT fixed dim as RT)", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for i in 0..array_rt.len() {
                sum += black_box(array_rt[i]);
            }
            sum
        })
    });

    group.finish();
}

// ============================================================================
// Benchmark 7: Index calculation performance
// 基准测试 7：索引计算性能
// ============================================================================

fn bench_index_calculation_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("index_calculation");
    let size = 100;
    let iterations = 10000;

    // CT Shape (fixed dimension)
    // CT 形状（固定维度）
    let shape_ct: ShapeRM<3> = ShapeRM::new([size, size, size]);

    // CTDyn Shape (dynamic dimension, CT storage order)
    // CT 动态形状（动态维度，CT 存储顺序）
    let shape_ct_dyn: DynShapeRM = DynShapeRM::new(vec![size, size, size]);

    // RT Shape (dynamic dimension, RT storage order)
    // RT 形状（动态维度，RT 存储顺序）
    let shape_rt: DynShape = DynShape::new(vec![size, size, size]);

    // CT Shape - index_of calculation
    // CT 形状 - index_of 计算
    group.bench_function("CT Shape - index_of", |b| {
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

    // CTDyn Shape - index_of calculation
    // CT 动态形状 - index_of 计算
    group.bench_function("CTDyn Shape - index_of", |b| {
        b.iter(|| {
            let mut idx = 0usize;
            for _ in 0..iterations {
                let vector = vec![idx % size, (idx / size) % size, idx / (size * size)];
                idx = black_box(shape_ct_dyn.index_of(&vector).unwrap());
                idx = (idx + 1) % (size * size * size);
            }
            idx
        })
    });

    // RT Shape - index_of calculation
    // RT 形状 - index_of 计算
    group.bench_function("RT Shape - index_of", |b| {
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
    bench_access_order_benchmark,
    bench_storage_order_benchmark,
    bench_index_access_benchmark,
    bench_view_benchmark,
    bench_combined_benchmark,
    bench_index_calculation_benchmark,
);

criterion_main!(benches);
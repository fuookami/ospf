# ospf-rust-multiarray

:us: [English](README.md) | :cn: 简体中文

## 简介

`ospf-rust-multiarray` 把 Kotlin `ospf-kotlin-multiarray` 映射为 Rust 多维数组 crate，支持编译期与运行期 shape、存储顺序感知索引、零拷贝 view、block array 和轻量 tabular data structure。

## 作用范围

本 crate 拥有泛型数组存储、shape/index 转换、view、切片、维度映射、block array 和 `DataFrame` helper。

明确非目标：

1. tensor algebra engine 或优化模型装配。
2. 领域专用数据协议。
3. 物理单位语义；这些属于 `ospf-rust-quantities`。

高性能、泛型的 Rust 多维数组库，支持编译期和运行期形状。

## 特性

### 泛型多维数组

支持编译期 (`Shape<N>`) 和运行期 (`DynShape`) 维度定义：

```rust
use ospf_rust_multiarray::{Shape, DynShape, MultiArray, MultiArrayBuilder};

// 编译期固定维度（3D 数组）
let shape: Shape<3> = Shape::new([4, 5, 6]);
let array: MultiArray<f64, Shape<3>> = MultiArrayBuilder::new(shape);

// 运行期动态维度
let dyn_shape: DynShape = DynShape::new(vec![4, 5, 6]);
let dyn_array: MultiArray<f64, DynShape> = MultiArrayBuilder::new(dyn_shape);
```

### 灵活的存储顺序

支持行优先和列优先存储顺序，以适应不同的访问模式：

```rust
use ospf_rust_multiarray::{Shape, RowMajor, ColumnMajor};

// 行优先存储（默认，适合按行遍历）
let rm_shape: Shape<2, RowMajor> = Shape::new([100, 100]);

// 列优先存储（适合按列遍历）
let cm_shape: Shape<2, ColumnMajor> = Shape::new([100, 100]);
```

### 零拷贝数组视图

创建视图时无需复制数据，支持切片和维度映射：

```rust
use ospf_rust_multiarray::{MultiArray, MultiArrayBuilder, Shape, dummy_expect, map_expect, _0, _1};

let shape: Shape<2> = Shape::new([10, 20]);
let array: MultiArray<f64, _> = MultiArrayBuilder::new_by(shape, |i, _| i as f64);

// 切片视图
let view = array.view(&dummy_expect![2..8, 5..15]).unwrap();

// 维度映射（转置）
let transposed = array.map_view(&map_expect![_1, _0]).unwrap();
```

### 块数组

分块存储，高效处理大型数组：

```rust
use ospf_rust_multiarray::{BlockMultiArray, BlockMultiArrayBuilder, DynShape};

let shape = DynShape::new(vec![1000, 1000]);
let block_array = BlockMultiArrayBuilder::new(shape);
```

### 数据框

带命名列的表格数据结构：

```rust
use ospf_rust_multiarray::{data_frame_of, DataFrameBuilder};

let df = data_frame_of([
    ("x", vec![Some(1.0), Some(2.0), Some(3.0)]),
    ("y", vec![Some(4.0), Some(5.0), Some(6.0)]),
]);

let rows = DataFrameBuilder::build_rows(["x", "y"], |rows| {
    rows.row([Some(1.0), Some(4.0)]);
    rows.row([Some(2.0), Some(5.0)]);
});
```

## 性能特性

### 基准测试结果

以下基准测试在 Windows 10 系统上运行，展示编译期与运行期配置的性能对比。

#### 维度对比 (32³ = 32,768 个元素)

| 配置 | 时间 | 吞吐量 |
|------|------|--------|
| CT 固定维度 (`Shape<3, RowMajor>`) | 23.24 µs | 1.41 Gelem/s |
| RT 维度 + RT 存储 (`DynShape`) | 23.49 µs | 1.40 Gelem/s |

**结论**：编译期固定维度在简单迭代中仅比运行期维度快约 1%。

#### 索引访问对比 (100² = 10,000 个元素)

| 访问类型 | 时间 | 吞吐量 |
|----------|------|--------|
| CT 固定维度 - 数组索引 `[&[i, j]]` | 15.34 µs | 652 Melem/s |
| RT 动态维度 - 向量索引 `[&vec![i, j]]` | 477.47 µs | 20.9 Melem/s |
| CT 固定维度 - 扁平索引 `[i]` | 7.15 µs | 1.40 Gelem/s |
| RT 动态维度 - 扁平索引 `[i]` | 7.14 µs | 1.40 Gelem/s |

**关键发现**：数组索引 `[&[i, j]]` 比向量索引 `[&vec![i, j]]` 快约 31 倍，但扁平索引访问无差异。

#### 存储顺序对比 (64³ = 262,144 个元素)

| 存储顺序 | 时间 | 吞吐量 |
|----------|------|--------|
| CT 行优先 | 186.77 µs | 21.93 Melem/s |
| CT 列优先 | 188.60 µs | 21.72 Melem/s |

**结论**：行优先和列优先存储的迭代性能相近（约 1% 差异）。

#### 索引计算性能 (10,000 次迭代)

| 操作 | CT Shape | RT Shape | 加速比 |
|------|----------|----------|--------|
| `index_of` (数组 vs 向量) | 158.67 µs | 507.26 µs | **3.2 倍** |
| `vector_of` | 143.71 µs | 615.31 µs | **4.3 倍** |

**关键发现**：编译期固定数组的索引计算比运行期向量快 3-4 倍。

### 编译期与运行期的权衡

| 特性 | 编译期 | 运行期 |
|------|--------|--------|
| **维度** | `Shape<N>` - 零成本抽象，索引计算快 3-4 倍 | `DynShape` - 灵活性高，迭代开销极小 |
| **存储顺序** | `RowMajor` / `ColumnMajor` 类型 - 分支消除 | `StorageOrder` 枚举 - 动态选择 |
| **索引类型** | 数组 `[usize; N]` - 最佳性能 | `Vec<usize>` - 重复访问慢约 30 倍 |

### 关键性能优势

1. **编译期维度 (`Shape<N>`)**
   - 维度数是 const 泛型参数
   - 循环展开和向量化优化机会
   - 固定大小数组的维度元数据无需堆分配
   - 编译期验证维度相关操作

2. **编译期存储顺序**
   - 存储顺序相关分支的无效代码消除
   - 更好的内联机会
   - 可预测的内存访问模式

3. **零拷贝视图**
   - 视图共享底层数据缓冲区
   - 创建视图时无需内存分配
   - 高效的切片和子数组操作

### 内存布局

- **行优先 (Row-Major)**：元素按 C 语言顺序存储，适合按行遍历
- **列优先 (Column-Major)**：元素按 Fortran 风格顺序存储，适合按列遍历

根据访问模式选择正确的存储顺序可以显著提高缓存利用率。

## API 概览

### 核心类型

| 类型 | 描述 |
|------|------|
| `MultiArray<T, S>` | 主要的多维数组类型 |
| `Shape<N, O>` | 编译期维度数的形状 |
| `DynShape` | 运行期维度数的形状 |
| `MultiArrayView` | 数组的非拥有视图 |
| `BlockMultiArray` | 分块存储的数组 |
| `DataFrame` | 带命名列的表格 |

### 构建器

| 构建器 | 描述 |
|--------|------|
| `MultiArrayBuilder` | 构建 `MultiArray` 实例 |
| `MultiArrayViewBuilderRM` | 创建行优先视图 |
| `MultiArrayViewBuilderCM` | 创建列优先视图 |
| `BlockMultiArrayBuilder` | 构建块数组 |
| `DataFrameBuilder` | 构建数据框 |

### 索引类型

| 类型 | 描述 |
|------|------|
| `DummyIndex` | 视图创建的索引占位符 |
| `MapIndex` | 维度重排的索引映射 |
| `_0`, `_1`, ... | 维度映射的占位符常量 |

## 使用示例

### 创建数组

```rust
use ospf_rust_multiarray::{MultiArray, MultiArrayBuilder, Shape};

// 创建未初始化的数组
let shape = Shape::new([3, 4]);
let array: MultiArray<f64, _> = MultiArrayBuilder::new(shape);

// 创建填充指定值的数组
let filled = MultiArrayBuilder::new_with(Shape::new([2, 3]), 0.0);

// 使用初始化函数创建
let initialized = MultiArrayBuilder::new_by(Shape::new([3, 3]), |idx, _| idx as f64);
```

### 索引访问

```rust
// 扁平索引
let value = array[0];

// 向量索引
let value = array[&[1, 2]];
let value = array[&vec![1, 2]];
```

### 迭代

```rust
use cc_traits::Iter;

// 遍历元素
for &value in array.iter() {
    println!("{}", value);
}

// 带索引枚举
for (view_idx, linear_idx, coords, value) in view.enumerate() {
    println!("[{}] = {}", view_idx, value);
}
```

### 视图与切片

```rust
use ospf_rust_multiarray::{dummy_expect, map_expect, _0, _1, _2};

// 完整切片
let view = array.view(&dummy_expect![.., ..]).unwrap();

// 范围切片
let slice = array.view(&dummy_expect![0..5, 2..8]).unwrap();

// 固定索引
let row = array.view(&dummy_expect![3, ..]).unwrap();

// 通过映射转置
let transposed = array.map_view(&map_expect![_1, _0]).unwrap();

// 链式视图
let sub_view = view.view_by_dummy(&dummy_expect![0, ..]).unwrap();
```

## 泛型数值边界

`MultiArray<T, S>` 对元素类型 `T` 泛型化；数值语义由调用方提供。本 crate 不应引入优化专用数值转换。

## 本地验证

```powershell
cargo check -p ospf-rust-multiarray
cargo test -p ospf-rust-multiarray
cargo bench --package ospf-rust-multiarray --bench multiarray_bench
```

## 相关模块

- [根 README](../README_ch.md)
- [Kotlin multiarray README](../../ospf-kotlin/ospf-kotlin-multiarray/README_ch.md)

## 安装

在 `Cargo.toml` 中添加：

```toml
[dependencies]
ospf-rust-multiarray = { path = "..." }
```

## 环境要求

- Rust nightly（使用 `generic_const_exprs`、`specialization` 特性）

## 许可证

基于 MIT 许可证发布。详情请参阅 [LICENSE](../LICENSE)。

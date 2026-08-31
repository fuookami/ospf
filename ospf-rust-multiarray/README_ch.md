# ospf-rust-multiarray

[![Crates.io](https://img.shields.io/crates/v/ospf-rust-multiarray)](https://crates.io/crates/ospf-rust-multiarray)
[![Documentation](https://docs.rs/ospf-rust-multiarray/badge.svg)](https://docs.rs/ospf-rust-multiarray)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

一个功能强大、灵活的多维数组库，支持高级切片和视图功能。

:us: [English](README.md) | :cn: 简体中文

## 概述

`ospf-rust-multiarray` 提供了一个全面的多维数组实现，支持：
- **静态和动态形状** - 具有编译时维度的固定大小数组或运行时定义的形状
- **存储顺序配置** - 行主序或列主序内存布局
- **访问顺序配置** - 独立控制迭代顺序
- **高级切片** - 支持复杂的索引模式，包括范围、负索引和索引数组
- **视图转换** - 创建零拷贝视图，支持维度重排、切片和过滤
- **类型安全操作** - 利用 Rust 的类型系统实现编译时安全性
- **迭代器支持** - 支持可配置访问模式的高效迭代

## 特性

- **多维数组**，支持可配置的存储后端
- **形状类型**，支持静态（`Shape<D>`）和动态（`DynShape`）维度
- **编译时形状**（`CTShape`、`CTDynShape`），具有类型级存储顺序保证
- **存储顺序** - 行主序（默认）或列主序内存布局
- **访问顺序** - 独立控制迭代顺序（与存储顺序分离）
- **虚拟索引**，支持范围、负索引和索引数组
- **映射索引**，用于维度重排和投影
- **零拷贝视图**，使用 `MultiArrayView` 进行高效数据操作
- **集合特征集成**，与 `cc-traits` 互操作
- **全面的错误处理**，提供详细的错误类型

## 安装

在 `Cargo.toml` 中添加：

```toml
[dependencies]
ospf-rust-multiarray = "0.1"
```

## 快速开始

### 基本用法

```rust
use ospf_rust_multiarray::*;

// 创建一个填充零的 2x3 数组
let shape = Shape::new([2, 3]);
let mut array = MultiArrayBuilder::new_with(shape, 0);

// 使用向量索引访问元素
array[&[0, 1]] = 42;
assert_eq!(array[&[0, 1]], 42);

// 使用扁平索引访问元素
array[3] = 100;
assert_eq!(array[3], 100);

// 迭代所有元素
for (i, &value) in array.iter().enumerate() {
    println!("索引 {}: {}", i, value);
}
```

### 存储顺序

```rust
use ospf_rust_multiarray::*;

// 创建一个行主序存储的数组（默认）
let shape_vec = vec![2, 3, 4];
let array_row: MultiArray<i32, DynShape> = 
    MultiArrayBuilder::new_with_order(&shape_vec, StorageOrder::RowMajor);

// 创建一个列主序存储的数组
let array_col: MultiArray<i32, DynShape> = 
    MultiArrayBuilder::new_with_order(&shape_vec, StorageOrder::ColumnMajor);

// 检查存储顺序
assert_eq!(array_row.storage_order(), StorageOrder::RowMajor);
assert_eq!(array_col.storage_order(), StorageOrder::ColumnMajor);

// 在存储顺序之间转换
let converted = array_row.to_storage_order(StorageOrder::ColumnMajor);
assert_eq!(converted.storage_order(), StorageOrder::ColumnMajor);
```

### 访问顺序

```rust
use ospf_rust_multiarray::*;

let shape = Shape::new([2, 3, 4]);
let mut array = MultiArrayBuilder::new_with(shape, 0);

// 用顺序值填充数组
for i in 0..array.len() {
    array[i] = i as i32;
}

// 创建一个具有行主序访问顺序的视图（默认 - 最后一个维度变化最快）
let dummy_vector = dummy_expect![0..2, 1..3, 0..2];
let view_row = array.view(&dummy_vector).unwrap();

// 使用显式的行主序迭代
for value in view_row.iter_with_order(AccessOrder::RowMajor) {
    println!("行主序访问：{}", value);
}

// 使用列主序迭代（第一个维度变化最快）
for value in view_row.iter_with_order(AccessOrder::ColumnMajor) {
    println!("列主序访问：{}", value);
}
```

### 高级切片

```rust
use ospf_rust_multiarray::*;

let shape = Shape::new([3, 4, 5]);
let mut array = MultiArrayBuilder::new_with(shape, 0);

// 用顺序值填充数组
for i in 0..array.len() {
    array[i] = i as i32;
}

// 使用虚拟索引创建视图
let dummy_vector = dummy_expect![0..2, 1, 2..4];
let view = array.view(&dummy_vector).unwrap();

// 视图包含选定的元素
assert_eq!(view.len(), 4); // 2 × 1 × 2 = 4 个元素

// 使用映射索引创建视图（维度重排）
let map_vector = map_expect![_2, _0, _1];
let mapped_view = array.map_view(&map_vector).unwrap();

// 维度被重排：原始 [3,4,5] -> 新的 [5,3,4]
assert_eq!(mapped_view.shape().dimension(), 3);
assert_eq!(mapped_view.shape().len_of_dimension(0).unwrap(), 5);
assert_eq!(mapped_view.shape().len_of_dimension(1).unwrap(), 3);
assert_eq!(mapped_view.shape().len_of_dimension(2).unwrap(), 4);
```

### 动态形状

```rust
use ospf_rust_multiarray::*;

// 创建动态形状
let dyn_shape = dyn_shape![2, 3, 4];
let mut array = MultiArrayBuilder::new_with(dyn_shape, 0);

// 动态形状与静态形状类似工作
assert_eq!(array.shape().dimension(), 3);
assert_eq!(array.len(), 24);

// 创建动态虚拟向量
let dyn_dummy = dyn_dummy_expect![0..2, 1, vec![0, 2, 3]];
let view = array.view(&dyn_dummy).unwrap();
```

### 重塑数组（Reshape）

```rust
use ospf_rust_multiarray::*;

let shape = Shape::new([2, 3]);
let mut array = MultiArrayBuilder::new_with(shape, 0);

// 用顺序值填充数组
for i in 0..array.len() {
    array[i] = (i + 1) as i32;
}

// 重塑为更大的数组，新元素使用默认值（0）填充
let new_shape = Shape::new([3, 3]);
let reshaped = array.reshape(new_shape);
assert_eq!(reshaped.len(), 9);
// 原数据保留：[1, 2, 3, 4, 5, 6, 0, 0, 0]

// 使用自定义值填充进行重塑
let shape2 = Shape::new([2, 2]);
let mut array2 = MultiArrayBuilder::new_with(shape2, 0);
array2[0] = 1;
array2[1] = 2;
array2[2] = 3;
array2[3] = 4;

let reshaped2 = array2.reshape_with(Shape::new([3, 3]), -1);
// 结果：[1, 2, 3, 4, -1, -1, -1, -1, -1]

// 使用生成器函数填充新元素进行重塑
let reshaped3 = array2.reshape_by(Shape::new([3, 3]), |index, _vec| -(index as i32));
// 结果：[1, 2, 3, 4, -4, -5, -6, -7, -8]
```

### 编译时数组（CT - Compile-Time）

为了获得最佳性能，可以使用编译时数组，其中存储顺序在编译时确定：

```rust
use ospf_rust_multiarray::*;

// 创建一个具有行主序存储的静态编译时数组
let shape = CTShape::<2, RowMajor>::new([2, 3]);
let array: MultiArrayRM<i32, _> = CTMultiArrayBuilder::new_with(shape, 42);

// 访问元素
assert_eq!(array[&[0, 1]], 42);
assert_eq!(array.len(), 6);

// 创建一个列主序编译时数组
let shape_col = CTShape::<2, ColumnMajor>::new([2, 3]);
let array_col: MultiArrayCM<i32, _> = CTMultiArrayBuilder::new(shape_col);

// 动态编译时数组
let dyn_shape = CTDynShape::<RowMajor>::new(vec![2, 3, 4]);
let array_dyn: MultiArrayRM<i32, _> = CTMultiArrayBuilder::new(dyn_shape);
assert_eq!(array_dyn.len(), 24);

// 使用生成器函数
let shape_gen = CTShape::<2, RowMajor>::new([2, 3]);
let array_gen: MultiArrayRM<usize, _> = 
    CTMultiArrayBuilder::new_by(shape_gen, |index, _vec| index * 10);
```

**编译时数组的优势：**
- 存储顺序在编译时确定（无运行时分支）
- 更好的内联和优化机会
- 类型安全的存储顺序保证

### 编译时视图（CT Views）

创建编译时数组的零拷贝视图，支持维度重排和切片：

```rust
use ospf_rust_multiarray::*;

let shape = CTShape::<3, RowMajor>::new([2, 3, 4]);
let mut array: MultiArrayRM<i32, _> = CTMultiArrayBuilder::new_with(shape, 0);

// 用顺序值填充数组
for i in 0..array.len() {
    array[i] = i as i32;
}

// 使用映射索引创建视图（维度重排）
let map_vector = map_expect![_2, _0, _1];
let view = MultiArrayViewBuilderRM::new_by_map(&array, map_vector)
    .expect("Should be able to create view");

// 维度被重排：原始 [2,3,4] -> 新的 [4,2,3]
assert_eq!(view.shape().dimension(), 3);
assert_eq!(view.shape().len_of_dimension(0).unwrap(), 4);
assert_eq!(view.shape().len_of_dimension(1).unwrap(), 2);
assert_eq!(view.shape().len_of_dimension(2).unwrap(), 3);
assert_eq!(view.len(), 24);

// 使用虚拟索引创建视图（切片）
let dummy_vector = dummy_expect![0..2, 1, 2..4];
let sliced_view = MultiArrayViewBuilderRM::new_by_dummy(&array, &dummy_vector)
    .expect("Should be able to create sliced view");

assert_eq!(sliced_view.len(), 4); // 2 × 1 × 2 = 4 个元素

// 视图链：从现有视图创建子视图
let sub_view = sliced_view
    .view_by_dummy(&dummy_expect![0..1])
    .expect("Should be able to create sub-view");

// 迭代视图元素
for value in view.iter() {
    println!("视图元素：{}", value);
}
```

**编译时视图的优势：**
- 零拷贝数据访问
- 编译时访问顺序优化
- 视图链支持
- 维度重排和切片

## 核心概念

### 存储顺序 vs 访问顺序

**存储顺序**（`StorageOrder`）定义数据在内存中的物理布局：
- **RowMajor（行主序）**：最后一个维度变化最快（C 风格顺序）
- **ColumnMajor（列主序）**：第一个维度变化最快（Fortran 风格顺序）

**访问顺序**（`AccessOrder`）定义迭代时元素的访问顺序：
- 独立于存储顺序
- 可以每个视图单独配置
- 不影响数据布局，只影响迭代顺序

### 形状

形状定义多维数组的维度：

- **`Shape<D>`**：具有编译时已知维度的静态形状
- **`DynShape`**：具有运行时定义维度的动态形状
- **`CTShape<D, SO>`**：具有类型级存储顺序的编译时形状
- **`CTDynShape<SO>`**：具有类型级存储顺序的编译时动态形状
- **`AbstractShape`**：定义常见形状操作的特征
- **`AbstractRTShape`**：具有存储顺序转换的运行时形状操作
- **`AbstractCTShape`**：类型系统中带有存储顺序的编译时形状

### 虚拟索引

虚拟索引允许选择数据的子集：

- **单个索引**：`0`、`-1`（从末尾开始的负索引）
- **范围**：`1..4`、`..3`、`2..`
- **索引数组**：`vec![0, 2, 4]`
- **完整范围**：`..`（选择全部）

### 映射索引

映射索引允许维度操作：

- **占位符**：`_0`、`_1`、`_2` 等用于维度重排
- **混合索引**：将占位符与虚拟索引结合
- **维度投影**：选择特定维度同时丢弃其他维度

### 视图

视图提供对数组子集的零拷贝访问：

- **`MultiArrayView`**：数组的不可变视图
- **`MultiArrayToView`**：用于将数组转换为视图的特征
- **视图链**：从其他视图创建视图
- **可配置的访问顺序**：使用 `iter_with_order()` 进行自定义迭代
- **高效迭代**：视图支持与数组相同的迭代器接口

### DataFrame

DataFrame 提供带有命名列和可选值的 2D 数组：

- **`DataFrame<T, C>`**：带有命名列的 2D 数组，包含 `Option<T>` 值
- **列名访问**：通过字符串名称访问列
- **行/列索引**：通过数字索引或列名访问
- **列视图**：将列数据作为 `MultiArrayView` 获取
- **灵活创建**：使用默认值、特定值或生成器函数创建

```rust
use ospf_rust_multiarray::data_frame::*;

// 创建一个带有列名的 DataFrame
let column_names = vec!["Name".to_string(), "Age".to_string()];
let mut df: DataFrameRM<String> = DataFrame::new(3, 2, column_names);

// 使用行和列名设置值
df.set_by_name(0, "Name", Some("Alice".to_string()));
df.set_by_name(0, "Age", Some("25".to_string()));

// 访问值
assert_eq!(df.get_by_name(0, "Name"), &Some(Some("Alice".to_string())));
assert_eq!(df[(0, "Age")], Some("25".to_string()));

// 将列作为视图获取
let name_column = df.get_column_by_name("Name").unwrap();
```

## 模块结构

```
ospf-rust-multiarray/
├── concept.rs      # 核心概念：StorageOrder、AccessOrder、Vector traits
├── error.rs        # 各种错误类型
├── shape.rs        # 运行时形状类型：Shape<D>、DynShape
├── ct_shape.rs     # 编译时形状类型：CTShape、CTDynShape
├── index_value.rs  # 索引值转换特征
├── dummy_index.rs  # 用于切片操作的虚拟索引类型
├── map_index.rs    # 用于维度重排的映射索引类型
├── multi_array.rs  # 运行时多维数组实现
├── multi_array_view.rs  # 运行时视图实现
├── ct_multi_array.rs    # 编译时数组实现
├── ct_multi_array_view.rs # 编译时视图实现
└── data_frame.rs   # DataFrame：带有命名列和可选值的 2D 数组
```

## API 参考

### 主要类型

- **`MultiArray<T, S, C>`**：主要的多维数组类型
- **`MultiArrayView<'a, T, S, C>`**：数组的不可变视图
- **`MultiArrayBuilder`**：创建数组的构建器模式
- **`CTMultiArrayBuilder`**：编译时数组构建器
- **`Shape<D>`**：静态形状类型
- **`DynShape`**：动态形状类型
- **`CTShape<D, SO>`**：带有存储顺序类型的编译时形状
- **`CTDynShape<SO>`**：编译时动态形状
- **`StorageOrder`**：存储顺序枚举（RowMajor, ColumnMajor）
- **`AccessOrder`**：访问顺序枚举（RowMajor, ColumnMajor）
- **`DummyIndex`**：虚拟索引操作的枚举
- **`MapIndex`**：映射索引操作的枚举

### 类型别名

#### 运行时数组
- **`MultiArrayRM<T, S>`**：行主序运行时数组
- **`MultiArrayCM<T, S>`**：列主序运行时数组

#### 编译时数组
- **`MultiArrayRM<T, S>`**：行主序编译时数组（通过 CTMultiArrayBuilder）
- **`MultiArrayCM<T, S>`**：列主序编译时数组（通过 CTMultiArrayBuilder）

#### 形状别名
- **`Shape0` 到 `Shape20`**：维度 0-20 的静态形状
- **`ShapeRM0` 到 `ShapeRM20`**：行主序编译时形状
- **`ShapeCM0` 到 `ShapeCM20`**：列主序编译时形状
- **`DynShapeRM`**：行主序动态编译时形状
- **`DynShapeCM`**：列主序动态编译时形状

### 宏

#### 形状创建
- `dyn_shape![...]`：创建动态形状

#### 虚拟索引
- `dummy![...]`：创建带错误处理的虚拟索引
- `dummy_expect![...]`：创建带 unwrap 的虚拟索引
- `dummy_with_err![...]`：创建带详细错误的虚拟索引
- `dyn_dummy![...]`：创建动态虚拟向量
- `dyn_dummy_expect![...]`：创建带 unwrap 的动态虚拟向量
- `dyn_dummy_with_err![...]`：创建带详细错误的动态虚拟向量

#### 映射索引
- `map![...]`：创建带错误处理的映射索引
- `map_expect![...]`：创建带 unwrap 的映射索引
- `map_with_err![...]`：创建带详细错误的映射索引
- `dyn_map![...]`：创建动态映射向量
- `dyn_map_expect![...]`：创建带 unwrap 的动态映射向量
- `dyn_map_with_err![...]`：创建带详细错误的动态映射向量

#### 数组比较
- `array_eq!(a, [x, y, z])`：将数组与切片比较
- `assert_array_eq!(a, [x, y, z])`：断言数组等于切片

### 常量

映射索引的占位符常量：
- `_0`、`_1`、`_2`、...、`_20`

## API 对比：MultiArray vs CTMultiArray

### 类型签名

| 特性 | MultiArray (运行时) | CTMultiArray (编译时) |
|------|---------------------|----------------------|
| **类型定义** | `MultiArray<T, S, C>` | `CTMultiArray<T, S, C, SO>` |
| **形状约束** | `S: AbstractRTShape` | `S: AbstractCTShape<SO>` |
| **存储顺序** | 形状中的运行时字段 | 类型参数 `SO: StorageOrderTrait` |
| **类型参数** | 3 个 (T, S, C) | 4 个 (T, S, C, SO) |

### 实现的特征

两种类型都实现相同的核心特征：

| 特征 | MultiArray | CTMultiArray |
|------|------------|--------------|
| `Clone` | ✅ | ✅ |
| `Deref` | ✅ | ✅ |
| `DerefMut` | ✅ | ✅ |
| `Collection` | ✅ | ✅ |
| `CollectionRef` | ✅ | ✅ |
| `CollectionMut` | ✅ | ✅ |
| `Len` | ✅ | ✅ |
| `Index<usize>` | ✅ | ✅ |
| `IndexMut<usize>` | ✅ | ✅ |
| `Index<&VectorType>` | ✅ | ✅ |
| `IndexMut<&VectorType>` | ✅ | ✅ |
| `Iter` | ✅ | ✅ |
| `IterMut` | ✅ | ✅ |
| `CTMultiArrayToView<S, SO, AO>` | ❌ | ✅ |
| `MultiArrayToView<S>` | ✅ | ❌ |

### 方法对比

| 方法 | MultiArray | CTMultiArray | 说明 |
|------|------------|--------------|------|
| `new(shape)` | ✅ | ✅ | 相同签名 |
| `new_with(shape, value)` | ✅ | ✅ | 相同签名 |
| `new_by(shape, generator)` | ✅ | ✅ | 相同签名 |
| `storage_order()` | ✅ (实例) | ✅ (静态) | RT: 实例方法，CT: 静态方法 |
| `to_storage_order(order)` | ✅ | ❌ | CT 无法在运行时更改存储顺序 |
| `reshape(new_shape)` | ✅ | ✅ | 相同签名 |
| `reshape_with(new_shape, fill)` | ✅ | ✅ | 相同签名 |
| `reshape_by(new_shape, gen)` | ✅ | ✅ | 相同签名 |
| `len()` | ✅ | ✅ | 相同签名 |
| `is_empty()` | ✅ | ✅ | 相同签名 |
| `shape()` | ✅ | ✅ | 相同签名 |
| `view(dummy_vector)` | ✅ | ✅ | CT: 通过 `CTMultiArrayToView::view()` 返回 `Result` |
| `map_view(map_vector)` | ✅ | ✅ | CT: 通过 `CTMultiArrayToView::map_view()` 返回 `Result` |

### 主要差异

1. **存储顺序处理**：
   - **MultiArray**：存储顺序是运行时字段，可通过 `to_storage_order()` 更改
   - **CTMultiArray**：存储顺序是类型参数，通过 `storage_order()` 静态方法在编译时确定

2. **视图创建**：
   - **MultiArray**：直接实现 `MultiArrayToView<S>` 特征，提供 `view()` 和 `map_view()` 方法
   - **CTMultiArray**：实现 `CTMultiArrayToView<S, SO, AO>` 特征，提供 `view()` 和 `map_view()` 方法
     - 该特征有三个类型参数：形状 `S`、存储顺序 `SO` 和访问顺序 `AO`
     - 两个方法都返回 `Result<ViewType<'a>, MappingIndexError>`

3. **类型复杂度**：
   - **MultiArray**：更简单的类型签名，3 个参数
   - **CTMultiArray**：更复杂，4 个参数包括存储顺序类型

### 构建器对比

| 构建器方法 | MultiArrayBuilder | CTMultiArrayBuilder |
|------------|-------------------|---------------------|
| `new(shape)` | ✅ | ✅ |
| `new_as(shape)` | ✅ | ✅ |
| `new_with(shape, value)` | ✅ | ✅ |
| `new_with_as(shape, value)` | ✅ | ✅ |
| `new_by(shape, gen)` | ✅ | ✅ |
| `new_by_as(shape, gen)` | ✅ | ✅ |
| `new_with_order(...)` | ✅ | ❌ (顺序是类型参数) |
| `new_with_order_and_value(...)` | ✅ | ❌ (顺序是类型参数) |
| `new_by_with_order(...)` | ✅ | ❌ (顺序是类型参数) |

## 错误处理

库提供全面的错误类型：

- `InvalidDummyIndexError`：虚拟索引转换失败
- `ExInvalidDummyIndexError<E>`：带有原始错误的虚拟索引转换失败
- `DimensionMismatchingError`：向量维度与形状不匹配
- `OutOfShapeError`：索引超出边界
- `IndexCalculationError`：索引计算失败（枚举）
- `RepeatMappingIndexError`：映射索引重复
- `MappingIndexError`：映射索引错误（枚举）

所有错误都实现了来自 `ospf-rust-base` 的 `Error` 特征。

## 性能基准测试

本库包含全面的基准测试，用于比较编译时与运行时配置的性能差异。

### 运行基准测试

```bash
cargo bench -p ospf-rust-multiarray --bench multiarray_bench
```

### 关键结果

所有基准测试均在 Windows 10 系统上运行。结果展示了典型的性能特征。

#### 1. 维度比较（32,768 个元素，3D 数组）

| 配置 | 时间 | 吞吐量 |
|------|------|--------|
| CT 固定维度 (`CTShape<3>`) | 23.312 µs | 1.406 Gelem/s |
| CT 动态维度 (`CTDynShape`) | 23.396 µs | 1.401 Gelem/s |
| RT 动态维度 (`DynShape`) | 23.307 µs | 1.406 Gelem/s |

**分析**：对于简单迭代，编译时和运行时维度显示出几乎相同的性能。Rust 编译器能够有效地优化运行时形状的顺序访问模式。

#### 2. 访问顺序比较（4,096 个元素，2D 视图）

| 配置 | 时间 | 吞吐量 |
|------|------|--------|
| CT 行主序访问 | 28.516 µs | 143.6 Melem/s |
| CT 列主序访问 | 28.831 µs | 142.1 Melem/s |
| RT 行主序访问 | 28.516 µs | 143.6 Melem/s |
| RT 列主序访问 | 29.702 µs | 137.1 Melem/s |

**分析**：编译时访问顺序比运行时访问顺序快约 4%。收益不大但表现一致。

#### 3. 存储顺序比较（4,096 个元素，2D 数组）

| 配置 | 时间 | 吞吐量 |
|------|------|--------|
| CT 行主序存储 | 2.900 µs | 1.412 Gelem/s |
| CT 列主序存储 | 2.899 µs | 1.413 Gelem/s |
| RT 行主序存储 | 2.899 µs | 1.413 Gelem/s |
| RT 列主序存储 | 2.900 µs | 1.412 Gelem/s |

**分析**：存储顺序（行主序与列主序）对顺序迭代性能的影响可以忽略不计。编译器为两种布局生成同样高效的代码。

#### 4. 索引访问性能（10,000 个元素，2D 数组）

| 访问类型 | 时间 | 吞吐量 |
|----------|------|--------|
| CT 固定维度 - 向量索引 | 13.817 µs | 723.8 Melem/s |
| CT 动态维度 - 向量索引 | 464.42 µs | 21.5 Melem/s |
| RT 动态维度 - 向量索引 | 476.15 µs | 21.0 Melem/s |
| CT 固定维度 - 扁平索引 | 7.135 µs | 1.402 Gelem/s |
| RT 动态维度 - 扁平索引 | 7.120 µs | 1.405 Gelem/s |

**分析**：
- **CT 固定维度的向量索引比动态维度快约 33 倍**
- 固定维度使用栈分配的数组（`[usize; N]`）避免了堆分配
- 扁平索引访问在 CT 和 RT 之间没有显著差异

#### 5. 视图操作（2,500 个元素，切片视图）

| 操作 | 时间 | 吞吐量 |
|------|------|--------|
| CT 视图 - 切片迭代 | 9.313 µs | 268.5 Melem/s |
| RT 视图 - 切片迭代 | 18.206 µs | 136.7 Melem/s |
| CT 视图 - 映射视图（重排） | 35.111 µs | 71.2 Melem/s |
| RT 视图 - 映射视图（重排） | 68.908 µs | 36.1 Melem/s |

**分析**：
- **CT 视图比 RT 视图快约 2 倍**（包括切片和映射操作）
- 编译时访问顺序消除了迭代器中的运行时分支

#### 6. 索引计算性能（10,000 次迭代）

| 操作 | 时间 |
|------|------|
| CT 形状 - `index_of` | 157.46 µs |
| CT 动态形状 - `index_of` | 492.48 µs |
| RT 形状 - `index_of` | 511.41 µs |
| CT 形状 - `vector_of` | 143.54 µs |
| RT 形状 - `vector_of` | 574.91 µs |

**分析**：
- **CT 固定维度索引计算比运行时快约 3.2 倍**
- **CT 固定维度向量计算比运行时快约 4 倍**
- 栈分配的偏移数组实现了更好的缓存局部性和 SIMD 优化

### 性能建议

1. **当维度在编译时已知时，使用 CT 固定维度（`CTShape<D, SO>`）**
   - 向量索引快达 33 倍
   - 索引计算快达 4 倍
   - 形状数据无堆分配

2. **对于视图密集型操作，使用 CT 视图**
   - 视图迭代快约 2 倍
   - 更好的内联机会

3. **扁平索引访问始终很快**
   - 尽可能使用扁平索引以获得最佳性能

4. **存储顺序的选择取决于访问模式，而非性能**
   - 根据算法的访问模式选择
   - 行主序适用于按行访问，列主序适用于按列访问

5. **动态维度仍然经过了良好优化**
   - 仅在索引计算时产生开销
   - 顺序迭代性能与编译时匹配

## 许可证

根据 Apache License, Version 2.0 许可。有关详细信息，请参阅 [LICENSE](./../LICENSE)。

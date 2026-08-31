# ospf-rust-multiarray

[![Crates.io](https://img.shields.io/crates/v/ospf-rust-multiarray)](https://crates.io/crates/ospf-rust-multiarray)
[![Documentation](https://docs.rs/ospf-rust-multiarray/badge.svg)](https://docs.rs/ospf-rust-multiarray)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

A powerful, flexible multi-dimensional array library for Rust with advanced slicing and view capabilities.

:us: English | :cn: [简体中文](README_ch.md)

## Overview

`ospf-rust-multiarray` provides a comprehensive multi-dimensional array implementation with support for:
- **Static and dynamic shapes** - Fixed-size arrays with compile-time dimensions or runtime-defined shapes
- **Storage order configuration** - Row-major or column-major memory layout
- **Access order configuration** - Independent control over iteration order
- **Advanced slicing** - Support for complex indexing patterns including ranges, negative indices, and index arrays
- **View transformations** - Create zero-copy views with dimension reordering, slicing, and filtering
- **Type-safe operations** - Leverages Rust's type system for compile-time safety
- **Iterator support** - Efficient iteration over arrays and views with configurable access patterns

## Features

- **Multi-dimensional arrays** with configurable storage backends
- **Shape types** for both static (`Shape<D>`) and dynamic (`DynShape`) dimensions
- **Compile-time shapes** (`CTShape`, `CTDynShape`) with type-level storage order guarantees
- **Storage order** - Row-major (default) or column-major memory layout
- **Access order** - Independent control over iteration order (separate from storage)
- **Dummy indexing** with support for ranges, negative indices, and index arrays
- **Map indexing** for dimension reordering and projection
- **Zero-copy views** with `MultiArrayView` for efficient data manipulation
- **Collection traits** integration with `cc-traits` for interoperability
- **Comprehensive error handling** with detailed error types

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
ospf-rust-multiarray = "0.1"
```

## Quick Start

### Basic Usage

```rust
use ospf_rust_multiarray::*;

// Create a 2x3 array filled with zeros
let shape = Shape::new([2, 3]);
let mut array = MultiArrayBuilder::new_with(shape, 0);

// Access elements using vector indices
array[&[0, 1]] = 42;
assert_eq!(array[&[0, 1]], 42);

// Access elements using flat indices
array[3] = 100;
assert_eq!(array[3], 100);

// Iterate over all elements
for (i, &value) in array.iter().enumerate() {
    println!("Index {}: {}", i, value);
}
```

### Storage Order

```rust
use ospf_rust_multiarray::*;

// Create an array with row-major storage (default)
let shape_vec = vec![2, 3, 4];
let array_row: MultiArray<i32, DynShape> = 
    MultiArrayBuilder::new_with_order(&shape_vec, StorageOrder::RowMajor);

// Create an array with column-major storage
let array_col: MultiArray<i32, DynShape> = 
    MultiArrayBuilder::new_with_order(&shape_vec, StorageOrder::ColumnMajor);

// Check the storage order
assert_eq!(array_row.storage_order(), StorageOrder::RowMajor);
assert_eq!(array_col.storage_order(), StorageOrder::ColumnMajor);

// Convert between storage orders
let converted = array_row.to_storage_order(StorageOrder::ColumnMajor);
assert_eq!(converted.storage_order(), StorageOrder::ColumnMajor);
```

### Access Order

```rust
use ospf_rust_multiarray::*;

let shape = Shape::new([2, 3, 4]);
let mut array = MultiArrayBuilder::new_with(shape, 0);

// Fill array with sequential values
for i in 0..array.len() {
    array[i] = i as i32;
}

// Create a view with row-major access order (default - last dimension changes fastest)
let dummy_vector = dummy_expect![0..2, 1..3, 0..2];
let view_row = array.view(&dummy_vector).unwrap();

// Iterate with explicit row-major order
for value in view_row.iter_with_order(AccessOrder::RowMajor) {
    println!("Row-major access: {}", value);
}

// Iterate with column-major order (first dimension changes fastest)
for value in view_row.iter_with_order(AccessOrder::ColumnMajor) {
    println!("Column-major access: {}", value);
}
```

### Advanced Slicing

```rust
use ospf_rust_multiarray::*;

let shape = Shape::new([3, 4, 5]);
let mut array = MultiArrayBuilder::new_with(shape, 0);

// Fill array with sequential values
for i in 0..array.len() {
    array[i] = i as i32;
}

// Create a view with dummy indexing
let dummy_vector = dummy_expect![0..2, 1, 2..4];
let view = array.view(&dummy_vector).unwrap();

// The view contains selected elements
assert_eq!(view.len(), 4); // 2 × 1 × 2 = 4 elements

// Create a view with map indexing (dimension reordering)
let map_vector = map_expect![_2, _0, _1];
let mapped_view = array.map_view(&map_vector).unwrap();

// Dimensions are reordered: original [3,4,5] -> new [5,3,4]
assert_eq!(mapped_view.shape().dimension(), 3);
assert_eq!(mapped_view.shape().len_of_dimension(0).unwrap(), 5);
assert_eq!(mapped_view.shape().len_of_dimension(1).unwrap(), 3);
assert_eq!(mapped_view.shape().len_of_dimension(2).unwrap(), 4);
```

### Dynamic Shapes

```rust
use ospf_rust_multiarray::*;

// Create a dynamic shape
let dyn_shape = dyn_shape![2, 3, 4];
let mut array = MultiArrayBuilder::new_with(dyn_shape, 0);

// Dynamic shapes work similarly to static shapes
assert_eq!(array.shape().dimension(), 3);
assert_eq!(array.len(), 24);

// Create a dynamic dummy vector
let dyn_dummy = dyn_dummy_expect![0..2, 1, vec![0, 2, 3]];
let view = array.view(&dyn_dummy).unwrap();
```

### Reshape Arrays

```rust
use ospf_rust_multiarray::*;

let shape = Shape::new([2, 3]);
let mut array = MultiArrayBuilder::new_with(shape, 0);

// Fill array with sequential values
for i in 0..array.len() {
    array[i] = (i + 1) as i32;
}

// Reshape to a larger array, filling new elements with default value (0)
let new_shape = Shape::new([3, 3]);
let reshaped = array.reshape(new_shape);
assert_eq!(reshaped.len(), 9);
// Original data preserved: [1, 2, 3, 4, 5, 6, 0, 0, 0]

// Reshape with a custom fill value
let shape2 = Shape::new([2, 2]);
let mut array2 = MultiArrayBuilder::new_with(shape2, 0);
array2[0] = 1;
array2[1] = 2;
array2[2] = 3;
array2[3] = 4;

let reshaped2 = array2.reshape_with(Shape::new([3, 3]), -1);
// Result: [1, 2, 3, 4, -1, -1, -1, -1, -1]

// Reshape with a generator function for new elements
let reshaped3 = array2.reshape_by(Shape::new([3, 3]), |index, _vec| -(index as i32));
// Result: [1, 2, 3, 4, -4, -5, -6, -7, -8]
```

### Compile-Time Arrays (CT - Compile-Time)

For maximum performance, use compile-time arrays where storage order is determined at compile time:

```rust
use ospf_rust_multiarray::*;

// Create a static compile-time array with row-major storage
let shape = CTShape::<2, RowMajor>::new([2, 3]);
let array: MultiArrayRM<i32, _> = CTMultiArrayBuilder::new_with(shape, 42);

// Access elements
assert_eq!(array[&[0, 1]], 42);
assert_eq!(array.len(), 6);

// Create a column-major compile-time array
let shape_col = CTShape::<2, ColumnMajor>::new([2, 3]);
let array_col: MultiArrayCM<i32, _> = CTMultiArrayBuilder::new(shape_col);

// Dynamic compile-time arrays
let dyn_shape = CTDynShape::<RowMajor>::new(vec![2, 3, 4]);
let array_dyn: MultiArrayRM<i32, _> = CTMultiArrayBuilder::new(dyn_shape);
assert_eq!(array_dyn.len(), 24);

// Use generator function
let shape_gen = CTShape::<2, RowMajor>::new([2, 3]);
let array_gen: MultiArrayRM<usize, _> = 
    CTMultiArrayBuilder::new_by(shape_gen, |index, _vec| index * 10);
```

**Benefits of Compile-Time Arrays:**
- Storage order determined at compile time (no runtime branching)
- Better inlining and optimization opportunities
- Type-safe storage order guarantees

### Compile-Time Views (CT Views)

Create zero-copy views of compile-time arrays with dimension reordering and slicing:

```rust
use ospf_rust_multiarray::*;

let shape = CTShape::<3, RowMajor>::new([2, 3, 4]);
let mut array: MultiArrayRM<i32, _> = CTMultiArrayBuilder::new_with(shape, 0);

// Fill array with sequential values
for i in 0..array.len() {
    array[i] = i as i32;
}

// Create a view with map indexing (dimension reordering)
let map_vector = map_expect![_2, _0, _1];
let view = MultiArrayViewBuilderRM::new_by_map(&array, map_vector)
    .expect("Should be able to create view");

// Dimensions are reordered: original [2,3,4] -> new [4,2,3]
assert_eq!(view.shape().dimension(), 3);
assert_eq!(view.shape().len_of_dimension(0).unwrap(), 4);
assert_eq!(view.shape().len_of_dimension(1).unwrap(), 2);
assert_eq!(view.shape().len_of_dimension(2).unwrap(), 3);
assert_eq!(view.len(), 24);

// Create a view with dummy indexing (slicing)
let dummy_vector = dummy_expect![0..2, 1, 2..4];
let sliced_view = MultiArrayViewBuilderRM::new_by_dummy(&array, &dummy_vector)
    .expect("Should be able to create sliced view");

assert_eq!(sliced_view.len(), 4); // 2 × 1 × 2 = 4 elements

// Chain views: create a sub-view from an existing view
let sub_view = sliced_view
    .view_by_dummy(&dummy_expect![0..1])
    .expect("Should be able to create sub-view");

// Iterate over view elements
for value in view.iter() {
    println!("View element: {}", value);
}
```

**Benefits of Compile-Time Views:**
- Zero-copy data access
- Compile-time access order optimization
- View chaining support
- Dimension reordering and slicing

## Core Concepts

### Storage Order vs Access Order

**Storage Order** (`StorageOrder`) defines how data is physically laid out in memory:
- **RowMajor**: Last dimension changes fastest (C-style order)
- **ColumnMajor**: First dimension changes fastest (Fortran-style order)

**Access Order** (`AccessOrder`) defines the order in which elements are visited during iteration:
- Independent from storage order
- Can be configured per-view
- Does not affect data layout, only iteration sequence

### Shapes

Shapes define the dimensions of multi-dimensional arrays:

- **`Shape<D>`**: Static shape with compile-time known dimensions
- **`DynShape`**: Dynamic shape with runtime-defined dimensions
- **`CTShape<D, SO>`**: Compile-time shape with type-level storage order
- **`CTDynShape<SO>`**: Compile-time dynamic shape with type-level storage order
- **`AbstractShape`**: Trait defining common shape operations
- **`AbstractRTShape`**: Runtime shape operations with storage order conversion
- **`AbstractCTShape`**: Compile-time shape with storage order in type system

### Dummy Indexing

Dummy indexing allows selecting subsets of data:

- **Single indices**: `0`, `-1` (negative indices from the end)
- **Ranges**: `1..4`, `..3`, `2..`
- **Index arrays**: `vec![0, 2, 4]`
- **Full ranges**: `..` (select all)

### Map Indexing

Map indexing allows dimension manipulation:

- **Placeholders**: `_0`, `_1`, `_2`, etc. for dimension reordering
- **Mixed indexing**: Combine placeholders with dummy indices
- **Dimension projection**: Select specific dimensions while dropping others

### Views

Views provide zero-copy access to array subsets:

- **`MultiArrayView`**: Immutable view into an array
- **`MultiArrayToView`**: Trait for converting arrays to views
- **View chaining**: Create views from other views
- **Configurable access order**: Use `iter_with_order()` for custom iteration
- **Efficient iteration**: Views support the same iterator interface as arrays

### DataFrame

DataFrame provides a 2D array with named columns and optional values:

- **`DataFrame<T, C>`**: 2D array with named columns containing `Option<T>` values
- **Column name access**: Access columns by string names
- **Row/column indexing**: Access by numeric indices or column names
- **Column views**: Get column data as `MultiArrayView`
- **Flexible creation**: Create with default values, specific values, or generator functions

```rust
use ospf_rust_multiarray::data_frame::*;

// Create a DataFrame with column names
let column_names = vec!["Name".to_string(), "Age".to_string()];
let mut df: DataFrameRM<String> = DataFrame::new(3, 2, column_names);

// Set values using row and column name
df.set_by_name(0, "Name", Some("Alice".to_string()));
df.set_by_name(0, "Age", Some("25".to_string()));

// Access values
assert_eq!(df.get_by_name(0, "Name"), &Some(Some("Alice".to_string())));
assert_eq!(df[(0, "Age")], Some("25".to_string()));

// Get column as a view
let name_column = df.get_column_by_name("Name").unwrap();
```

## Module Structure

```
ospf-rust-multiarray/
├── concept.rs      # Core concepts: StorageOrder, AccessOrder, Vector traits
├── error.rs        # Error types for various failure scenarios
├── shape.rs        # Runtime shape types: Shape<D>, DynShape
├── ct_shape.rs     # Compile-time shape types: CTShape, CTDynShape
├── index_value.rs  # Index value conversion trait
├── dummy_index.rs  # Virtual index types for slicing operations
├── map_index.rs    # Map index types for dimension reordering
├── multi_array.rs  # Runtime multi-dimensional array implementation
├── multi_array_view.rs  # Runtime view implementation
├── ct_multi_array.rs    # Compile-time array implementation
├── ct_multi_array_view.rs # Compile-time view implementation
└── data_frame.rs   # DataFrame: 2D array with column names and optional values
```

## API Reference

### Main Types

- **`MultiArray<T, S, C>`**: The main multi-dimensional array type
- **`MultiArrayView<'a, T, S, C>`**: Immutable view into an array
- **`MultiArrayBuilder`**: Builder pattern for creating arrays
- **`CTMultiArrayBuilder`**: Builder for compile-time arrays
- **`Shape<D>`**: Static shape type
- **`DynShape`**: Dynamic shape type
- **`CTShape<D, SO>`**: Compile-time shape with storage order type
- **`CTDynShape<SO>`**: Compile-time dynamic shape
- **`StorageOrder`**: Enum for storage order (RowMajor, ColumnMajor)
- **`AccessOrder`**: Enum for access order (RowMajor, ColumnMajor)
- **`DummyIndex`**: Enum for dummy indexing operations
- **`MapIndex`**: Enum for map indexing operations

### Type Aliases

#### Runtime Arrays
- **`MultiArrayRM<T, S>`**: Row-major runtime array
- **`MultiArrayCM<T, S>`**: Column-major runtime array

#### Compile-Time Arrays
- **`MultiArrayRM<T, S>`**: Row-major compile-time array (via CTMultiArrayBuilder)
- **`MultiArrayCM<T, S>`**: Column-major compile-time array (via CTMultiArrayBuilder)

#### Shape Aliases
- **`Shape0` to `Shape20`**: Static shapes with dimensions 0-20
- **`ShapeRM0` to `ShapeRM20`**: Row-major compile-time shapes
- **`ShapeCM0` to `ShapeCM20`**: Column-major compile-time shapes
- **`DynShapeRM`**: Row-major dynamic compile-time shape
- **`DynShapeCM`**: Column-major dynamic compile-time shape

### Macros

#### Shape Creation
- `dyn_shape![...]`: Create a dynamic shape

#### Dummy Indexing
- `dummy![...]`: Create dummy indices with error handling
- `dummy_expect![...]`: Create dummy indices with unwrap
- `dummy_with_err![...]`: Create dummy indices with detailed errors
- `dyn_dummy![...]`: Create dynamic dummy vectors
- `dyn_dummy_expect![...]`: Create dynamic dummy vectors with unwrap
- `dyn_dummy_with_err![...]`: Create dynamic dummy vectors with detailed errors

#### Map Indexing
- `map![...]`: Create map indices with error handling
- `map_expect![...]`: Create map indices with unwrap
- `map_with_err![...]`: Create map indices with detailed errors
- `dyn_map![...]`: Create dynamic map vectors
- `dyn_map_expect![...]`: Create dynamic map vectors with unwrap
- `dyn_map_with_err![...]`: Create dynamic map vectors with detailed errors

#### Array Comparison
- `array_eq!(a, [x, y, z])`: Compare array with slice
- `assert_array_eq!(a, [x, y, z])`: Assert array equals slice

### Constants

Placeholder constants for map indexing:
- `_0`, `_1`, `_2`, ..., `_20`

## API Comparison: MultiArray vs CTMultiArray

### Type Signatures

| Feature | MultiArray (Runtime) | CTMultiArray (Compile-Time) |
|---------|---------------------|----------------------------|
| **Type Definition** | `MultiArray<T, S, C>` | `CTMultiArray<T, S, C, SO>` |
| **Shape Constraint** | `S: AbstractRTShape` | `S: AbstractCTShape<SO>` |
| **Storage Order** | Runtime field in shape | Type parameter `SO: StorageOrderTrait` |
| **Type Parameters** | 3 (T, S, C) | 4 (T, S, C, SO) |

### Implemented Traits

Both types implement the same core traits:

| Trait | MultiArray | CTMultiArray |
|-------|------------|--------------|
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

### Method Comparison

| Method | MultiArray | CTMultiArray | Notes |
|--------|------------|--------------|-------|
| `new(shape)` | ✅ | ✅ | Same signature |
| `new_with(shape, value)` | ✅ | ✅ | Same signature |
| `new_by(shape, generator)` | ✅ | ✅ | Same signature |
| `storage_order()` | ✅ (instance) | ✅ (static) | RT: instance method, CT: static method |
| `to_storage_order(order)` | ✅ | ❌ | CT cannot change storage order at runtime |
| `reshape(new_shape)` | ✅ | ✅ | Same signature |
| `reshape_with(new_shape, fill)` | ✅ | ✅ | Same signature |
| `reshape_by(new_shape, gen)` | ✅ | ✅ | Same signature |
| `len()` | ✅ | ✅ | Same signature |
| `is_empty()` | ✅ | ✅ | Same signature |
| `shape()` | ✅ | ✅ | Same signature |
| `view(dummy_vector)` | ✅ | ✅ | CT: via `CTMultiArrayToView::view()` returning `Result` |
| `map_view(map_vector)` | ✅ | ✅ | CT: via `CTMultiArrayToView::map_view()` returning `Result` |

### Key Differences

1. **Storage Order Handling**:
   - **MultiArray**: Storage order is a runtime field, can be changed via `to_storage_order()`
   - **CTMultiArray**: Storage order is a type parameter, determined at compile time via `storage_order()` static method

2. **View Creation**:
   - **MultiArray**: Implements `MultiArrayToView` trait directly with `view()` and `map_view()` methods
   - **CTMultiArray**: Uses builder pattern with `MultiArrayViewBuilderRM` / `MultiArrayViewBuilderCM`

3. **Type Complexity**:
   - **MultiArray**: Simpler type signature with 3 parameters
   - **CTMultiArray**: More complex with 4 parameters including storage order type

### Builder Comparison

| Builder Method | MultiArrayBuilder | CTMultiArrayBuilder |
|----------------|-------------------|---------------------|
| `new(shape)` | ✅ | ✅ |
| `new_as(shape)` | ✅ | ✅ |
| `new_with(shape, value)` | ✅ | ✅ |
| `new_with_as(shape, value)` | ✅ | ✅ |
| `new_by(shape, gen)` | ✅ | ✅ |
| `new_by_as(shape, gen)` | ✅ | ✅ |
| `new_with_order(...)` | ✅ | ❌ (order is type param) |
| `new_with_order_and_value(...)` | ✅ | ❌ (order is type param) |
| `new_by_with_order(...)` | ✅ | ❌ (order is type param) |

## Error Handling

The library provides comprehensive error types:

- `InvalidDummyIndexError`: When dummy index conversion fails
- `ExInvalidDummyIndexError<E>`: Dummy index conversion failure with original error
- `DimensionMismatchingError`: When vector dimensions don't match shape
- `OutOfShapeError`: When indices are out of bounds
- `IndexCalculationError`: Index calculation failures (enum)
- `RepeatMappingIndexError`: When map indices repeat
- `MappingIndexError`: Mapping index errors (enum)

All errors implement the `Error` trait from `ospf-rust-base`.

## Performance Benchmarks

The library includes comprehensive benchmarks comparing compile-time vs runtime configurations.

### Running Benchmarks

```bash
cargo bench -p ospf-rust-multiarray --bench multiarray_bench
```

### Key Results

All benchmarks were run on a Windows 10 system. Results show typical performance characteristics.

#### 1. Dimension Comparison (32,768 elements, 3D array)

| Configuration | Time | Throughput |
|--------------|------|------------|
| CT Fixed Dimension (`CTShape<3>`) | 23.312 µs | 1.406 Gelem/s |
| CT Dynamic Dimension (`CTDynShape`) | 23.396 µs | 1.401 Gelem/s |
| RT Dynamic Dimension (`DynShape`) | 23.307 µs | 1.406 Gelem/s |

**Analysis**: For simple iteration, compile-time and runtime dimensions show nearly identical performance. The Rust compiler optimizes runtime shapes effectively for sequential access patterns.

#### 2. Access Order Comparison (4,096 elements, 2D view)

| Configuration | Time | Throughput |
|--------------|------|------------|
| CT Row-Major Access | 28.516 µs | 143.6 Melem/s |
| CT Column-Major Access | 28.831 µs | 142.1 Melem/s |
| RT Row-Major Access | 28.516 µs | 143.6 Melem/s |
| RT Column-Major Access | 29.702 µs | 137.1 Melem/s |

**Analysis**: Compile-time access order shows ~4% improvement over runtime access order. The benefit is modest but consistent.

#### 3. Storage Order Comparison (4,096 elements, 2D array)

| Configuration | Time | Throughput |
|--------------|------|------------|
| CT Row-Major Storage | 2.900 µs | 1.412 Gelem/s |
| CT Column-Major Storage | 2.899 µs | 1.413 Gelem/s |
| RT Row-Major Storage | 2.899 µs | 1.413 Gelem/s |
| RT Column-Major Storage | 2.900 µs | 1.412 Gelem/s |

**Analysis**: Storage order (row-major vs column-major) has negligible impact on sequential iteration performance. The compiler generates equally efficient code for both layouts.

#### 4. Index Access Performance (10,000 elements, 2D array)

| Access Type | Time | Throughput |
|-------------|------|------------|
| CT Fixed Dim - Vector Index | 13.817 µs | 723.8 Melem/s |
| CT Dynamic Dim - Vector Index | 464.42 µs | 21.5 Melem/s |
| RT Dynamic Dim - Vector Index | 476.15 µs | 21.0 Melem/s |
| CT Fixed Dim - Flat Index | 7.135 µs | 1.402 Gelem/s |
| RT Dynamic Dim - Flat Index | 7.120 µs | 1.405 Gelem/s |

**Analysis**: 
- **Vector indexing with CT fixed dimensions is ~33x faster** than dynamic dimensions
- Stack-allocated arrays (`[usize; N]`) for fixed dimensions avoid heap allocation
- Flat index access shows no significant difference between CT and RT

#### 5. View Operations (2,500 elements, sliced view)

| Operation | Time | Throughput |
|-----------|------|------------|
| CT View - Sliced Iteration | 9.313 µs | 268.5 Melem/s |
| RT View - Sliced Iteration | 18.206 µs | 136.7 Melem/s |
| CT View - Map View (reordering) | 35.111 µs | 71.2 Melem/s |
| RT View - Map View (reordering) | 68.908 µs | 36.1 Melem/s |

**Analysis**: 
- **CT views are ~2x faster** than RT views for both slicing and mapping operations
- Compile-time access order eliminates runtime branching in iterators

#### 6. Index Calculation Performance (10,000 iterations)

| Operation | Time |
|-----------|------|
| CT Shape - `index_of` | 157.46 µs |
| CT Dyn Shape - `index_of` | 492.48 µs |
| RT Shape - `index_of` | 511.41 µs |
| CT Shape - `vector_of` | 143.54 µs |
| RT Shape - `vector_of` | 574.91 µs |

**Analysis**:
- **CT fixed dimension index calculation is ~3.2x faster** than runtime
- **CT fixed dimension vector calculation is ~4x faster** than runtime
- Stack-allocated offset arrays enable better cache locality and SIMD optimization

### Performance Recommendations

1. **Use CT fixed dimensions (`CTShape<D, SO>`) when dimensions are known at compile time**
   - Up to 33x faster vector indexing
   - Up to 4x faster index calculations
   - No heap allocation for shape data

2. **Use CT views for view-heavy operations**
   - ~2x faster iteration over views
   - Better inlining opportunities

3. **Flat index access is always fast**
   - Use flat indices when possible for maximum performance

4. **Storage order choice depends on access patterns, not performance**
   - Choose based on your algorithm's access pattern
   - Row-major for row-wise access, column-major for column-wise access

5. **Dynamic dimensions are still well-optimized**
   - Only incur overhead for index calculations
   - Sequential iteration performance matches compile-time

## License

Licensed under Apache License, Version 2.0. See [LICENSE](./../LICENSE) for details.

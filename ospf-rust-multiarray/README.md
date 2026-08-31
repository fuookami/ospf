# ospf-rust-multiarray

🇺🇸 [English](README.md) | 🇨🇳 简体中文

A high-performance, generic multi-dimensional array library for Rust with compile-time and runtime shape support.

## Features

### Generic Multi-Dimensional Arrays

Support for both compile-time (`Shape<N>`) and runtime (`DynShape`) dimensionality:

```rust
use ospf_rust_multiarray::{Shape, DynShape, MultiArray, MultiArrayBuilder};

// Compile-time fixed dimensions (3D array)
let shape: Shape<3> = Shape::new([4, 5, 6]);
let array: MultiArray<f64, Shape<3>> = MultiArrayBuilder::new(shape);

// Runtime dynamic dimensions
let dyn_shape: DynShape = DynShape::new(vec![4, 5, 6]);
let dyn_array: MultiArray<f64, DynShape> = MultiArrayBuilder::new(dyn_shape);
```

### Flexible Storage Order

Row-major and column-major storage orders for optimal performance with different access patterns:

```rust
use ospf_rust_multiarray::{Shape, RowMajor, ColumnMajor};

// Row-major storage (default, optimal for row-wise iteration)
let rm_shape: Shape<2, RowMajor> = Shape::new([100, 100]);

// Column-major storage (optimal for column-wise iteration)
let cm_shape: Shape<2, ColumnMajor> = Shape::new([100, 100]);
```

### Zero-Copy Array Views

Create views without copying data, supporting slicing and dimension mapping:

```rust
use ospf_rust_multiarray::{MultiArray, MultiArrayBuilder, Shape, dummy_expect, map_expect, _0, _1};

let shape: Shape<2> = Shape::new([10, 20]);
let array: MultiArray<f64, _> = MultiArrayBuilder::new_by(shape, |i, _| i as f64);

// Slice view
let view = array.view(&dummy_expect![2..8, 5..15]).unwrap();

// Dimension mapping (transpose)
let transposed = array.map_view(&map_expect![_1, _0]).unwrap();
```

### Block Arrays

Chunked storage for efficient handling of large arrays:

```rust
use ospf_rust_multiarray::{BlockMultiArray, BlockMultiArrayBuilder, DynShape};

let shape = DynShape::new(vec![1000, 1000]);
let block_array = BlockMultiArrayBuilder::new(shape);
```

### DataFrame

Tabular data structure with named columns:

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

## Performance Characteristics

### Benchmark Results

The following benchmarks were run on a Windows 10 system. Results show the performance comparison between compile-time and runtime configurations.

#### Dimension Comparison (32³ = 32,768 elements)

| Configuration | Time | Throughput |
|---------------|------|------------|
| CT fixed dim (`Shape<3, RowMajor>`) | 23.24 µs | 1.41 Gelem/s |
| RT dim + RT storage (`DynShape`) | 23.49 µs | 1.40 Gelem/s |

**Result**: Compile-time fixed dimensions show only ~1% improvement over runtime dimensions for simple iteration.

#### Index Access Comparison (100² = 10,000 elements)

| Access Type | Time | Throughput |
|-------------|------|------------|
| CT fixed dim - array index `[&[i, j]]` | 15.34 µs | 652 Melem/s |
| RT dynamic dim - vec index `[&vec![i, j]]` | 477.47 µs | 20.9 Melem/s |
| CT fixed dim - flat index `[i]` | 7.15 µs | 1.40 Gelem/s |
| RT dynamic dim - flat index `[i]` | 7.14 µs | 1.40 Gelem/s |

**Key Finding**: Array index `[&[i, j]]` is ~31x faster than vec index `[&vec![i, j]]`, but flat index access shows no difference.

#### Storage Order Comparison (64³ = 262,144 elements)

| Storage Order | Time | Throughput |
|---------------|------|------------|
| CT Row-Major | 186.77 µs | 21.93 Melem/s |
| CT Column-Major | 188.60 µs | 21.72 Melem/s |

**Result**: Row-major and column-major storage show similar iteration performance (~1% difference).

#### Index Calculation Performance (10,000 iterations)

| Operation | CT Shape | RT Shape | Speedup |
|-----------|----------|----------|---------|
| `index_of` (array vs vec) | 158.67 µs | 507.26 µs | **3.2x** |
| `vector_of` | 143.71 µs | 615.31 µs | **4.3x** |

**Key Finding**: Index calculations with compile-time fixed arrays are 3-4x faster than with runtime vectors.

### Compile-Time vs Runtime Trade-offs

| Feature | Compile-Time | Runtime |
|---------|--------------|---------|
| **Dimensions** | `Shape<N>` - Zero-cost abstraction, 3-4x faster index calc | `DynShape` - Flexibility with minimal iteration overhead |
| **Storage Order** | `RowMajor` / `ColumnMajor` types - Branch elimination | `StorageOrder` enum - Dynamic selection |
| **Index Type** | Array `[usize; N]` - Optimal performance | `Vec<usize>` - ~30x slower for repeated access |

### Key Performance Benefits

1. **Compile-Time Dimensions (`Shape<N>`)**
   - Dimension count is a const generic parameter
   - Loop unrolling and vectorization opportunities
   - No heap allocation for dimension metadata in fixed-size arrays
   - Compile-time verification of dimension-related operations

2. **Compile-Time Storage Order**
   - Dead code elimination for storage-order-specific branches
   - Better inlining opportunities
   - Predictable memory access patterns

3. **Zero-Copy Views**
   - Views share the underlying data buffer
   - No memory allocation when creating views
   - Efficient slicing and sub-array operations

### Memory Layout

- **Row-Major**: Elements stored in C-style order, optimal for row-wise traversal
- **Column-Major**: Elements stored in Fortran-style order, optimal for column-wise traversal

Choosing the correct storage order for your access pattern can significantly improve cache utilization.

## API Overview

### Core Types

| Type | Description |
|------|-------------|
| `MultiArray<T, S>` | Main multi-dimensional array type |
| `Shape<N, O>` | Shape with compile-time dimension count |
| `DynShape` | Shape with runtime dimension count |
| `MultiArrayView` | Non-owning view into an array |
| `BlockMultiArray` | Array with chunked storage |
| `DataFrame` | Table with named columns |

### Builders

| Builder | Description |
|---------|-------------|
| `MultiArrayBuilder` | Construct `MultiArray` instances |
| `MultiArrayViewBuilderRM` | Create row-major views |
| `MultiArrayViewBuilderCM` | Create column-major views |
| `BlockMultiArrayBuilder` | Construct block arrays |
| `DataFrameBuilder` | Construct data frames |

### Index Types

| Type | Description |
|------|-------------|
| `DummyIndex` | Index placeholder for view creation |
| `MapIndex` | Index mapping for dimension reordering |
| `_0`, `_1`, ... | Placeholder constants for dimension mapping |

## Usage Examples

### Creating Arrays

```rust
use ospf_rust_multiarray::{MultiArray, MultiArrayBuilder, Shape};

// Create an uninitialized array
let shape = Shape::new([3, 4]);
let array: MultiArray<f64, _> = MultiArrayBuilder::new(shape);

// Create an array filled with a value
let filled = MultiArrayBuilder::new_with(Shape::new([2, 3]), 0.0);

// Create with initialization function
let initialized = MultiArrayBuilder::new_by(Shape::new([3, 3]), |idx, _| idx as f64);
```

### Indexing

```rust
// Flat index
let value = array[0];

// Vector index
let value = array[&[1, 2]];
let value = array[&vec![1, 2]];
```

### Iteration

```rust
use cc_traits::Iter;

// Iterate over elements
for &value in array.iter() {
    println!("{}", value);
}

// Enumerate with indices
for (view_idx, linear_idx, coords, value) in view.enumerate() {
    println!("[{}] = {}", view_idx, value);
}
```

### Views and Slicing

```rust
use ospf_rust_multiarray::{dummy_expect, map_expect, _0, _1, _2};

// Full slice
let view = array.view(&dummy_expect![.., ..]).unwrap();

// Range slice
let slice = array.view(&dummy_expect![0..5, 2..8]).unwrap();

// Fixed index
let row = array.view(&dummy_expect![3, ..]).unwrap();

// Transpose via mapping
let transposed = array.map_view(&map_expect![_1, _0]).unwrap();

// Chained views
let sub_view = view.view_by_dummy(&dummy_expect![0, ..]).unwrap();
```

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
ospf-rust-multiarray = { path = "..." }
```

## Requirements

- Rust nightly (uses `generic_const_exprs`, `specialization` features)

## License

Licensed under the MIT License. See [LICENSE](../LICENSE) for details.

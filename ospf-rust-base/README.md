# ospf-rust-base

🇺🇸 [English](README.md) | 🇨🇳 简体中文

Base utility library for the [ospf-rust](https://github.com/fuookami/ospf-rust) project.

## Overview

`ospf-rust-base` provides foundational utilities including error handling, type-safe indexing, collection abstractions, and specialized iterators. It serves as the core dependency for all other crates in the ospf-rust workspace.

## Modules

| Module | Description |
|--------|-------------|
| [`error`](#error-module) | Error handling with categorized error codes, extended result types, and position tracking |
| [`indexed_type`](#indexed_type-module) | Type-safe auto-incrementing and manual indexing for structs |
| [`collection`](#collection-module) | Collection abstractions with `Indices` trait |
| [`container`](#container-module) | Container type markers for generic programming |
| [`chunked_collection`](#chunked_collection-module) | Fixed-size chunk storage for cache locality and parallel processing |
| [`cloneable_function`](#cloneable_function-module) | Cloneable boxed closures via trait macro |
| [`generator_iterator`](#generator_iterator-module) | Coroutine-to-iterator bridge |
| [`iter`](#iter-module) | Iterator extensions |

## Error Module

Comprehensive error handling with categorized error codes, position tracking, and extended result states.

### Key Types

- **`ErrorCode`** - Enum with categorized error codes (authentication, file I/O, OR engine, application errors)
- **`ErrorPosition`** - Tracks file and line number for debugging
- **`Error`** / **`ExError<T>`** - Error traits for custom error types
- **`Ret<T>`** / **`Try`** - Type aliases for `Result<T, Box<dyn Error>>`
- **`ExResult<T, E>`** - Extended result with Ok/Failed/Warn/Fatal states

### ExResult States

```rust
pub enum ExResult<T, E> {
    Ok(T),                    // Success
    Failed(E),                // Single error
    Warn { value: T, warnings: Vec<E> },  // Success with warnings
    Fatal(Vec<E>),            // Multiple fatal errors
}
```

### Macros

| Macro | Purpose |
|-------|---------|
| `error_type!` | Define error structs with automatic position tracking |
| `error_enum!` | Define error enums with variant helpers |
| `error!` | Create error instances with file/line capture |

### Example

```rust
use ospf_rust_base::{error_type, error_enum, error, Error, ErrorCode, ExResult};

// Define an error type
error_type! {
    #[derive(Clone, Copy)]
    pub struct MyError {
        pub message: &'static str,
    }
}

impl Error for MyError {
    fn code(&self) -> ErrorCode { ErrorCode::Other }
    fn msg(&self) -> String { self.message.to_string() }
}

// Use ExResult for rich error states
fn process() -> ExResult<i32, MyError> {
    let value = 42;
    if value > 0 {
        ExResult::warning(value, error!(MyError { message: "minor issue" }))
    } else {
        ExResult::ok(value)
    }
}

// Check result states
let result = process();
assert!(result.is_warned());
assert_eq!(result.value(), Some(&42));
```

## Indexed Type Module

Type-safe auto-incrementing and manual indexing for structs.

### Key Types

- **`Index<T>`** - Auto-incrementing index (thread-safe global counter)
- **`ManualIndex<T>`** - Manually set index with deferred assignment
- **`Indexed`** / **`ManualIndexed`** - Traits for indexed types
- **`IndexedSliceExt`** - Extension for finding elements by index in slices

### Macros

| Macro | Purpose |
|-------|---------|
| `auto_indexed_type!` | Define struct with auto-incrementing index |
| `manual_indexed_type!` | Define struct with manually-set index |
| `indexed!` | Create indexed struct instances |

### Example

```rust
use ospf_rust_base::{auto_indexed_type, manual_indexed_type, indexed, Indexed, ManualIndexed, IndexedSliceExt};

// Auto-indexed type
auto_indexed_type! {
    pub struct User {
        pub name: String,
    }
}

let user1 = indexed!(User { name: "Alice".to_string() });
let user2 = indexed!(User { name: "Bob".to_string() });

assert_eq!(user1.index(), 0);
assert_eq!(user2.index(), 1);

// Manual-indexed type
manual_indexed_type! {
    pub struct Task {
        pub description: String,
    }
}

let task = indexed!(Task { description: "Example".to_string() });
assert!(!task.indexed());
task.set_indexed();  // Assign index when ready
assert!(task.indexed());

// Find by index in slice
let users = vec![user1, user2];
assert_eq!(users.find_or_get(1).map(|u| &u.name), Some(&"Bob".to_string()));
```

## Collection Module

Collection abstractions for index ranges.

### `Indices` Trait

```rust
pub trait Indices {
    fn indices(&self) -> Range<usize>;
}

// Implemented for: usize, [T], [T; N], Vec<T>, VecDeque<T>
```

### Example

```rust
use ospf_rust_base::Indices;

let vec = vec![10, 20, 30];
for i in vec.indices() {
    println!("Index {}: {}", i, vec[i]);
}
```

## Container Module

Type markers for generic container programming.

### Traits

- **`StaticContainer`** - Fixed-size containers (`Type<T, const D: usize>`)
- **`Container`** - Dynamic-size containers (`Type<T>`)
- **`Map`** - Key-value maps (`Type<K, V>`)

### Marker Types

| Marker | Container |
|--------|-----------|
| `Array` | `[T; D]` |
| `BoxArray` | `Box<[T; D]>` |
| `ArrayVec` | `arrayvec::ArrayVec<T, D>` (feature: `arrayvec`) |
| `Vec` | `std::vec::Vec<T>` |
| `VecDeque` | `std::collections::VecDeque<T>` |
| `LinkedList` | `std::collections::LinkedList<T>` |
| `HashSet` | `std::collections::HashSet<T>` |
| `BTreeSet` | `std::collections::BTreeSet<T>` |
| `HashMap` | `std::collections::HashMap<K, V>` |
| `BTreeMap` | `std::collections::BTreeMap<K, V>` |

### Example

```rust
use ospf_rust_base::{Container, Vec, HashMap, Map};

fn create_container<C: Container>() -> C::Type<i32> {
    Default::default()
}

let vec: <Vec as Container>::Type<i32> = create_container::<Vec>();

fn create_map<M: Map>() -> M::Type<String, i32> {
    Default::default()
}

let map: <HashMap as Map>::Type<String, i32> = create_map::<HashMap>();
```

## Chunked Collection Module

Fixed-size chunk storage for cache locality and parallel processing.

### `ChunkedVec<T>`

A vector-like container that stores elements in fixed-size chunks (default: 4096 elements).

**Advantages:**
- Better cache locality (each chunk fits in CPU cache)
- Parallel processing (chunks can be processed independently)
- Memory efficiency (avoids large contiguous allocations)
- Efficient growth without reallocation

### Example

```rust
use ospf_rust_base::ChunkedVec;

// Create with default chunk size (4096 elements)
let mut vec: ChunkedVec<i32> = ChunkedVec::new();

// Or specify custom chunk size
let mut vec: ChunkedVec<f64> = ChunkedVec::with_chunk_size(1024);

// Push elements
for i in 0..10000 {
    vec.push(i);
}

// Access by index
assert_eq!(vec[0], 0);
assert_eq!(vec[9999], 9999);

// Iterate over chunks for parallel processing
for chunk in vec.chunks() {
    // Process each chunk independently
    process_chunk(chunk);
}

// Convert to flat Vec
let flat: Vec<i32> = vec.into_vec();
```

### Parallel Processing Example

```rust
use ospf_rust_base::ChunkedVec;
use rayon::prelude::*;

let mut data: ChunkedVec<f64> = (0..100_000).map(|i| i as f64).collect();

// Process chunks in parallel
data.chunks_mut().for_each(|chunk| {
    for item in chunk {
        *item = item.sqrt();
    }
});
```

## Cloneable Function Module

Create cloneable boxed closures via trait macro.

### Example

```rust
use ospf_rust_base::cloneable_function;

cloneable_function!(type Handler = Fn(i32) -> String);

fn create_handler(prefix: String) -> Box<dyn Handler> {
    Box::new(move |x| format!("{}: {}", prefix, x))
}

let handler = create_handler("Value".to_string());
let handler_clone = handler.clone();  // Now cloneable!

assert_eq!(handler(42), "Value: 42");
assert_eq!(handler_clone(42), "Value: 42");
```

## Generator Iterator Module

Bridge Rust coroutines (unstable feature) to iterators.

### Example

```rust
#![feature(coroutines, coroutine_trait)]

use ospf_rust_base::GeneratorIterator;

let gen = GeneratorIterator(
    #[coroutine]
    || {
        yield 1;
        yield 2;
        yield 3;
    },
);

let values: Vec<i32> = gen.collect();
assert_eq!(values, vec![1, 2, 3]);
```

## Iter Module

Iterator extensions.

### `None` Trait

Opposite of `all()` - checks that no element matches a predicate.

```rust
use ospf_rust_base::iter::None;

let nums = vec![1, 2, 3, 4, 5];

// Check no element is negative
assert!(nums.iter().none(|&x| x < 0));

// Check no element equals 10
assert!(nums.iter().none(|&x| x == 10));
```

## Features

| Feature | Description |
|---------|-------------|
| `arrayvec` | Enables `ArrayVec` container type from the `arrayvec` crate |

## Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| `strum` | 0.28.0 | Enum derive macros for `ErrorCode` |
| `paste` | 1.0.15 | Macro helpers for `error_enum!` |
| `cc-traits` | git | Collection traits for `ChunkedVec` |
| `arrayvec` | 0.7.6 | Fixed-capacity vector (optional) |

## License

Licensed under the MIT License.

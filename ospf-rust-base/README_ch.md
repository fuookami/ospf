# ospf-rust-base

:us: [English](README.md) | :cn: 简体中文

## 简介

`ospf-rust-base` 是 `ospf-rust` workspace 的基础工具 crate。它把 Kotlin 支撑模块中的基础工具职责映射为 Rust 的错误处理、索引、集合、容器、迭代器和可克隆函数辅助。

## 作用范围

本 crate 拥有所有其他 crate 使用的小型、轻依赖基础工具。

明确非目标：

1. 优化建模、solver 抽象或领域 framework 逻辑。
2. 数学代数、物理量或多维数组。
3. application/runtime adapter。

[ospf-rust](https://github.com/fuookami/ospf-rust) 项目的基础工具库。

## 概述

`ospf-rust-base` 提供基础工具，包括错误处理、类型安全索引、集合抽象和专用迭代器。它是 ospf-rust 工作区中所有其他 crate 的核心依赖。

## 模块

| 模块 | 描述 |
|------|------|
| [`error`](#error-模块) | 错误处理：分类错误码、扩展结果类型和位置追踪 |
| [`indexed_type`](#indexed_type-模块) | 结构体的类型安全自增索引和手动索引 |
| [`collection`](#collection-模块) | 集合抽象与 `Indices` trait |
| [`container`](#container-模块) | 泛型编程的容器类型标记 |
| [`chunked_collection`](#chunked_collection-模块) | 固定大小分块存储，优化缓存局部性和并行处理 |
| [`cloneable_function`](#cloneable_function-模块) | 通过 trait 宏创建可克隆的闭包 |
| [`generator_iterator`](#generator_iterator-模块) | 协程到迭代器的桥接 |
| [`iter`](#iter-模块) | 迭代器扩展 |

## Error 模块

全面的错误处理，支持分类错误码、位置追踪和扩展结果状态。

### 核心类型

- **`ErrorCode`** - 分类错误码枚举（认证、文件 I/O、OR 引擎、应用错误）
- **`ErrorPosition`** - 追踪文件和行号以便调试
- **`Error`** / **`ExError<T>`** - 自定义错误类型的 trait
- **`Ret<T>`** / **`Try`** - `Result<T, Box<dyn Error>>` 的类型别名
- **`ExResult<T, E>`** - 具有 Ok/Failed/Warn/Fatal 状态的扩展结果

### ExResult 状态

```rust
pub enum ExResult<T, E> {
    Ok(T),                    // 成功
    Failed(E),                // 单个错误
    Warn { value: T, warnings: Vec<E> },  // 带警告的成功
    Fatal(Vec<E>),            // 多个致命错误
}
```

### 宏

| 宏 | 用途 |
|----|------|
| `error_type!` | 定义带自动位置追踪的错误结构体 |
| `error_enum!` | 定义带变体辅助函数的错误枚举 |
| `error!` | 创建错误实例并捕获文件/行号 |

### 示例

```rust
use ospf_rust_base::{error_type, error_enum, error, Error, ErrorCode, ExResult};

// 定义错误类型
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

// 使用 ExResult 处理丰富的错误状态
fn process() -> ExResult<i32, MyError> {
    let value = 42;
    if value > 0 {
        ExResult::warning(value, error!(MyError { message: "minor issue" }))
    } else {
        ExResult::ok(value)
    }
}

// 检查结果状态
let result = process();
assert!(result.is_warned());
assert_eq!(result.value(), Some(&42));
```

## Indexed Type 模块

结构体的类型安全自增索引和手动索引。

### 核心类型

- **`Index<T>`** - 自增索引（线程安全的全局计数器）
- **`ManualIndex<T>`** - 手动设置的索引，支持延迟分配
- **`Indexed`** / **`ManualIndexed`** - 可索引类型的 trait
- **`IndexedSliceExt`** - 切片中按索引查找元素的扩展

### 宏

| 宏 | 用途 |
|----|------|
| `auto_indexed_type!` | 定义带自增索引的结构体 |
| `manual_indexed_type!` | 定义带手动索引的结构体 |
| `indexed!` | 创建索引结构体实例 |

### 示例

```rust
use ospf_rust_base::{auto_indexed_type, manual_indexed_type, indexed, Indexed, ManualIndexed, IndexedSliceExt};

// 自增索引类型
auto_indexed_type! {
    pub struct User {
        pub name: String,
    }
}

let user1 = indexed!(User { name: "Alice".to_string() });
let user2 = indexed!(User { name: "Bob".to_string() });

assert_eq!(user1.index(), 0);
assert_eq!(user2.index(), 1);

// 手动索引类型
manual_indexed_type! {
    pub struct Task {
        pub description: String,
    }
}

let task = indexed!(Task { description: "Example".to_string() });
assert!(!task.indexed());
task.set_indexed();  // 准备好后分配索引
assert!(task.indexed());

// 在切片中按索引查找
let users = vec![user1, user2];
assert_eq!(users.find_or_get(1).map(|u| &u.name), Some(&"Bob".to_string()));
```

## Collection 模块

索引范围的集合抽象。

### `Indices` Trait

```rust
pub trait Indices {
    fn indices(&self) -> Range<usize>;
}

// 已为以下类型实现: usize, [T], [T; N], Vec<T>, VecDeque<T>
```

### 示例

```rust
use ospf_rust_base::Indices;

let vec = vec![10, 20, 30];
for i in vec.indices() {
    println!("索引 {}: {}", i, vec[i]);
}
```

## Container 模块

泛型容器编程的类型标记。

### Trait

- **`StaticContainer`** - 固定大小容器 (`Type<T, const D: usize>`)
- **`Container`** - 动态大小容器 (`Type<T>`)
- **`Map`** - 键值映射 (`Type<K, V>`)

### 类型标记

| 标记 | 容器 |
|------|------|
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

### 示例

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

## Chunked Collection 模块

固定大小分块存储，优化缓存局部性和并行处理。

### `ChunkedVec<T>`

类似向量的容器，将元素存储在固定大小的块中（默认：4096 个元素）。

**优势：**
- 更好的缓存局部性（每个块适合 CPU 缓存）
- 并行处理（块可以独立处理）
- 内存效率（避免大型连续分配）
- 无需重新分配的高效增长

### 示例

```rust
use ospf_rust_base::ChunkedVec;

// 使用默认块大小（4096 个元素）
let mut vec: ChunkedVec<i32> = ChunkedVec::new();

// 或指定自定义块大小
let mut vec: ChunkedVec<f64> = ChunkedVec::with_chunk_size(1024);

// 推入元素
for i in 0..10000 {
    vec.push(i);
}

// 按索引访问
assert_eq!(vec[0], 0);
assert_eq!(vec[9999], 9999);

// 迭代块进行并行处理
for chunk in vec.chunks() {
    // 独立处理每个块
    process_chunk(chunk);
}

// 转换为扁平 Vec
let flat: Vec<i32> = vec.into_vec();
```

### 并行处理示例

```rust
use ospf_rust_base::ChunkedVec;
use rayon::prelude::*;

let mut data: ChunkedVec<f64> = (0..100_000).map(|i| i as f64).collect();

// 并行处理块
data.chunks_mut().for_each(|chunk| {
    for item in chunk {
        *item = item.sqrt();
    }
});
```

## Cloneable Function 模块

通过 trait 宏创建可克隆的装箱闭包。

### 示例

```rust
use ospf_rust_base::cloneable_function;

cloneable_function!(type Handler = Fn(i32) -> String);

fn create_handler(prefix: String) -> Box<dyn Handler> {
    Box::new(move |x| format!("{}: {}", prefix, x))
}

let handler = create_handler("Value".to_string());
let handler_clone = handler.clone();  // 现在可克隆！

assert_eq!(handler(42), "Value: 42");
assert_eq!(handler_clone(42), "Value: 42");
```

## Generator Iterator 模块

将 Rust 协程（不稳定特性）桥接到迭代器。

### 示例

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

## Iter 模块

迭代器扩展。

### `None` Trait

`all()` 的相反操作 - 检查没有元素匹配谓词。

```rust
use ospf_rust_base::iter::None;

let nums = vec![1, 2, 3, 4, 5];

// 检查没有负数元素
assert!(nums.iter().none(|&x| x < 0));

// 检查没有元素等于 10
assert!(nums.iter().none(|&x| x == 10));
```

## Public API

| API | 职责 | 稳定性 |
| --- | --- | --- |
| `Ret<T>` / `Try` | 共享 fallible result alias。 | stable within migration |
| `ExResult<T, E>` | 带 warning 和 fatal variant 的扩展 result state。 | stable within migration |
| `error_type!`、`error_enum!`、`error!` | 带位置 metadata 的 error 构造宏。 | stable within migration |
| `Indexed`、`ManualIndexed`、`IndexedSliceExt` | 类型安全索引契约。 | stable within migration |
| `ChunkedVec` | 面向大向量的分块存储。 | stable within migration |
| `cloneable_function!` | 生成可克隆 boxed closure trait。 | stable within migration |

## 本地验证

```powershell
cargo check -p ospf-rust-base
cargo test -p ospf-rust-base
cargo check -p ospf-rust-base --features arrayvec
```

## 相关模块

- [根 README](../README_ch.md)
- [Kotlin workspace README](../../ospf-kotlin/README_ch.md)

## 特性

| 特性 | 描述 |
|------|------|
| `arrayvec` | 启用 `arrayvec` crate 的 `ArrayVec` 容器类型 |

## 依赖

| 依赖 | 版本 | 用途 |
|------|------|------|
| `strum` | 0.28.0 | `ErrorCode` 的枚举派生宏 |
| `paste` | 1.0.15 | `error_enum!` 的宏辅助工具 |
| `cc-traits` | git | `ChunkedVec` 的集合 trait |
| `arrayvec` | 0.7.6 | 固定容量向量（可选） |

## 许可证

采用 MIT 许可证授权。

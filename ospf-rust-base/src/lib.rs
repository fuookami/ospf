#![feature(specialization)]
#![feature(coroutines, coroutine_trait)]
#![feature(sync_unsafe_cell)]
#![feature(trait_alias)]
#![feature(associated_type_defaults)]
#![feature(allocator_api)]
#![cfg_attr(
    debug_assertions,
    allow(dead_code, unused, incomplete_features, static_mut_refs)
)]

//! 基础工具库 / Base utility library
//!
//! ospf-rust 的基础工具库，提供错误处理、索引类型、集合抽象等功能。
//! Base utility library for ospf-rust, providing error handling, indexed types, collection abstractions, and more.

#[macro_use]
extern crate strum;

// ============================================================================
// Lock helper macros - 安全的锁获取宏
// ============================================================================

/// 安全地获取 Mutex 锁 / Safely acquire a Mutex lock
///
/// 当锁被 poison 时 panic 并带上下文信息。
/// Panics with context when the lock is poisoned.
///
/// # 示例 / Example
///
/// ```ignore
/// let guard = lock_unwrap!(self.data);
/// ```
#[macro_export]
macro_rules! lock_unwrap {
    ($lock:expr) => {
        $lock
            .lock()
            .expect("lock poisoned: another thread panicked while holding the lock")
    };
}

/// 安全地获取 RwLock 读锁 / Safely acquire an RwLock read lock
///
/// 当锁被 poison 时 panic 并带上下文信息。
/// Panics with context when the lock is poisoned.
///
/// # 示例 / Example
///
/// ```ignore
/// let guard = read_unwrap!(self.cache);
/// ```
#[macro_export]
macro_rules! read_unwrap {
    ($lock:expr) => {
        $lock
            .read()
            .expect("rwlock poisoned: another thread panicked while holding the read lock")
    };
}

/// 安全地获取 RwLock 写锁 / Safely acquire an RwLock write lock
///
/// 当锁被 poison 时 panic 并带上下文信息。
/// Panics with context when the lock is poisoned.
///
/// # 示例 / Example
///
/// ```ignore
/// let guard = write_unwrap!(self.cache);
/// ```
#[macro_export]
macro_rules! write_unwrap {
    ($lock:expr) => {
        $lock
            .write()
            .expect("rwlock poisoned: another thread panicked while holding the write lock")
    };
}

pub use cloneable_function::*;
pub use collection::*;
pub use container::*;
pub use error::*;
pub use generator_iterator::*;
pub use indexed_type::{Indexed, IndexedSliceExt, ManualIndexed};
pub use iter::*;

#[macro_use]
pub mod error;
pub mod generator_iterator;
#[macro_use]
pub mod indexed_type;
pub mod iter;
#[macro_use]
pub mod cloneable_function;
pub mod chunked_collection;
pub mod collection;
pub mod container;

pub use chunked_collection::{ChunkedVec, ChunkedVecIter, ChunkedVecIterMut, DEFAULT_CHUNK_SIZE};

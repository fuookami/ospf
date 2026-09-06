//! 容器类型抽象。
//! Container type abstractions.

use std::alloc::{Allocator, Global};

/// 静态容量容器trait。
/// Trait for static-capacity containers.
pub trait StaticContainer {
    /// 关联类型，带编译期容量参数。
    /// Associated type with compile-time capacity parameter.
    type Type<T, const D: usize>;
}

/// 动态容器trait。
/// Trait for dynamic containers.
pub trait Container {
    /// 关联类型。
    /// Associated type.
    type Type<T>;
    /// 带分配器的关联类型。
    /// Associated type with custom allocator.
    type TypeWithAllocator<T, A: Allocator + Clone>;
}

/// 映射容器trait。
/// Trait for map containers.
pub trait Map {
    /// 关联类型。
    /// Associated type.
    type Type<K, V>;
    /// 带分配器的关联类型。
    /// Associated type with custom allocator.
    type TypeWithAllocator<K, V, A: Allocator + Clone>;
}

/// 数组容器标记类型。
/// Array container marker type.
pub struct Array;
impl StaticContainer for Array {
    type Type<T, const D: usize> = [T; D];
}

/// Box数组容器标记类型。
/// Boxed array container marker type.
pub struct BoxArray;
impl StaticContainer for BoxArray {
    type Type<T, const D: usize> = Box<[T; D]>;
}

/// ArrayVec容器标记类型。
/// ArrayVec container marker type.
#[cfg(feature = "arrayvec")]
pub struct ArrayVec;
#[cfg(feature = "arrayvec")]
impl StaticContainer for ArrayVec {
    type Type<T, const D: usize> = arrayvec::ArrayVec<T, D>;
}

/// Vec容器标记类型。
/// Vec container marker type.
pub struct Vec;
impl Container for Vec {
    type Type<T> = std::vec::Vec<T>;
    type TypeWithAllocator<T, A: Allocator + Clone> = std::vec::Vec<T, A>;
}

/// VecDeque容器标记类型。
/// VecDeque container marker type.
pub struct VecDeque;
impl Container for VecDeque {
    type Type<T> = std::collections::VecDeque<T>;
    type TypeWithAllocator<T, A: Allocator + Clone> = std::collections::VecDeque<T, A>;
}

/// LinkedList容器标记类型。
/// LinkedList container marker type.
pub struct LinkedList;
impl Container for LinkedList {
    type Type<T> = std::collections::LinkedList<T>;
    type TypeWithAllocator<T, A: Allocator + Clone> = std::collections::LinkedList<T, A>;
}

/// HashSet容器标记类型。
/// HashSet container marker type.
pub struct HashSet;
impl Container for HashSet {
    type Type<T> = std::collections::HashSet<T>;
    type TypeWithAllocator<T, A: Allocator + Clone> = std::collections::HashSet<T, A>;
}

/// BTreeSet容器标记类型。
/// BTreeSet container marker type.
pub struct BTreeSet;
impl Container for BTreeSet {
    type Type<T> = std::collections::BTreeSet<T>;
    type TypeWithAllocator<T, A: Allocator + Clone> = std::collections::BTreeSet<T, A>;
}

/// HashMap容器标记类型。
/// HashMap container marker type.
pub struct HashMap;
impl Map for HashMap {
    type Type<K, V> = std::collections::HashMap<K, V>;
    type TypeWithAllocator<K, V, A: Allocator + Clone> = std::collections::HashMap<K, V, A>;
}

/// BTreeMap容器标记类型。
/// BTreeMap container marker type.
pub struct BTreeMap;
impl Map for BTreeMap {
    type Type<K, V> = std::collections::BTreeMap<K, V>;
    type TypeWithAllocator<K, V, A: Allocator + Clone> = std::collections::BTreeMap<K, V, A>;
}

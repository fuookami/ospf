//! 集合抽象 trait / Collection abstraction traits
//!
//! 为多维数组和分块集合提供最小、稳定的集合接口。
//! Provides the minimal stable collection interface used by multi-arrays and chunked collections.

use std::ops::{Deref, DerefMut};

/// 集合及其元素类型 / Collection and its item type
pub trait Collection {
    /// 集合元素类型 / Collection item type
    type Item;
}

/// 可不可变引用的集合 / Collection with immutable item references
pub trait CollectionRef: Collection {
    /// 元素引用类型 / Item reference type
    type ItemRef<'a>: Clone + Deref<Target = Self::Item>
    where
        Self: 'a;

    /// 缩短元素引用生命周期 / Shorten an item reference lifetime
    fn upcast_item_ref<'short, 'long: 'short>(r: Self::ItemRef<'long>) -> Self::ItemRef<'short>
    where
        Self: 'long;
}

/// 可变引用的集合 / Collection with mutable item references
pub trait CollectionMut: Collection {
    /// 可变元素引用类型 / Mutable item reference type
    type ItemMut<'a>: DerefMut<Target = Self::Item>
    where
        Self: 'a;

    /// 缩短可变元素引用生命周期 / Shorten a mutable item reference lifetime
    fn upcast_item_mut<'short, 'long: 'short>(r: Self::ItemMut<'long>) -> Self::ItemMut<'short>
    where
        Self: 'long;
}

/// 提供长度信息的集合 / Collection exposing its length
pub trait Len {
    /// 返回元素数量 / Return the number of elements
    fn len(&self) -> usize;

    /// 判断集合是否为空 / Check whether the collection is empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// 可迭代集合 / Iterable collection
pub trait Iter: CollectionRef {
    /// 不可变迭代器类型 / Immutable iterator type
    type Iter<'a>: Iterator<Item = Self::ItemRef<'a>>
    where
        Self: 'a;

    /// 创建不可变迭代器 / Create an immutable iterator
    fn iter(&self) -> Self::Iter<'_>;
}

/// 可变迭代集合 / Mutably iterable collection
pub trait IterMut: CollectionMut {
    /// 可变迭代器类型 / Mutable iterator type
    type IterMut<'a>: Iterator<Item = Self::ItemMut<'a>>
    where
        Self: 'a;

    /// 创建可变迭代器 / Create a mutable iterator
    fn iter_mut(&mut self) -> Self::IterMut<'_>;
}

impl<T> Collection for Vec<T> {
    type Item = T;
}

impl<T> CollectionRef for Vec<T> {
    type ItemRef<'a>
        = &'a T
    where
        Self: 'a;

    fn upcast_item_ref<'short, 'long: 'short>(r: Self::ItemRef<'long>) -> Self::ItemRef<'short>
    where
        Self: 'long,
    {
        r
    }
}

impl<T> CollectionMut for Vec<T> {
    type ItemMut<'a>
        = &'a mut T
    where
        Self: 'a;

    fn upcast_item_mut<'short, 'long: 'short>(r: Self::ItemMut<'long>) -> Self::ItemMut<'short>
    where
        Self: 'long,
    {
        r
    }
}

impl<T> Len for Vec<T> {
    fn len(&self) -> usize {
        Vec::len(self)
    }

    fn is_empty(&self) -> bool {
        Vec::is_empty(self)
    }
}

impl<T> Iter for Vec<T> {
    type Iter<'a>
        = std::slice::Iter<'a, T>
    where
        Self: 'a;

    fn iter(&self) -> Self::Iter<'_> {
        self.as_slice().iter()
    }
}

impl<T> IterMut for Vec<T> {
    type IterMut<'a>
        = std::slice::IterMut<'a, T>
    where
        Self: 'a;

    fn iter_mut(&mut self) -> Self::IterMut<'_> {
        self.as_mut_slice().iter_mut()
    }
}

impl<T> Collection for [T] {
    type Item = T;
}

impl<T> CollectionRef for [T] {
    type ItemRef<'a>
        = &'a T
    where
        Self: 'a;

    fn upcast_item_ref<'short, 'long: 'short>(r: Self::ItemRef<'long>) -> Self::ItemRef<'short>
    where
        Self: 'long,
    {
        r
    }
}

impl<T> CollectionMut for [T] {
    type ItemMut<'a>
        = &'a mut T
    where
        Self: 'a;

    fn upcast_item_mut<'short, 'long: 'short>(r: Self::ItemMut<'long>) -> Self::ItemMut<'short>
    where
        Self: 'long,
    {
        r
    }
}

impl<T> Len for [T] {
    fn len(&self) -> usize {
        <[T]>::len(self)
    }

    fn is_empty(&self) -> bool {
        <[T]>::is_empty(self)
    }
}

impl<T> Iter for [T] {
    type Iter<'a>
        = std::slice::Iter<'a, T>
    where
        Self: 'a;

    fn iter(&self) -> Self::Iter<'_> {
        <[T]>::iter(self)
    }
}

impl<T> IterMut for [T] {
    type IterMut<'a>
        = std::slice::IterMut<'a, T>
    where
        Self: 'a;

    fn iter_mut(&mut self) -> Self::IterMut<'_> {
        <[T]>::iter_mut(self)
    }
}

impl<T, const N: usize> Collection for [T; N] {
    type Item = T;
}

impl<T, const N: usize> CollectionRef for [T; N] {
    type ItemRef<'a>
        = &'a T
    where
        Self: 'a;

    fn upcast_item_ref<'short, 'long: 'short>(r: Self::ItemRef<'long>) -> Self::ItemRef<'short>
    where
        Self: 'long,
    {
        r
    }
}

impl<T, const N: usize> CollectionMut for [T; N] {
    type ItemMut<'a>
        = &'a mut T
    where
        Self: 'a;

    fn upcast_item_mut<'short, 'long: 'short>(r: Self::ItemMut<'long>) -> Self::ItemMut<'short>
    where
        Self: 'long,
    {
        r
    }
}

impl<T, const N: usize> Len for [T; N] {
    fn len(&self) -> usize {
        N
    }

    fn is_empty(&self) -> bool {
        N == 0
    }
}

impl<T, const N: usize> Iter for [T; N] {
    type Iter<'a>
        = std::slice::Iter<'a, T>
    where
        Self: 'a;

    fn iter(&self) -> Self::Iter<'_> {
        self.as_slice().iter()
    }
}

impl<T, const N: usize> IterMut for [T; N] {
    type IterMut<'a>
        = std::slice::IterMut<'a, T>
    where
        Self: 'a;

    fn iter_mut(&mut self) -> Self::IterMut<'_> {
        self.as_mut_slice().iter_mut()
    }
}

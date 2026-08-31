//! 索引类型模块。
//! Indexed type module.

use std::any::TypeId;
use std::cell::Cell;
use std::collections::HashMap;
use std::fmt::Display;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::{Arc, Mutex, OnceLock};

/// 自动生成的索引类型。
/// Automatically generated index type.
#[derive(Debug, Clone)]
pub struct Index<T: 'static> {
    index: usize,
    _marker: PhantomData<T>,
}

impl<T: 'static> Deref for Index<T> {
    type Target = usize;

    fn deref(&self) -> &usize {
        &self.index
    }
}

impl<T: 'static> Default for Index<T> {
    fn default() -> Self {
        Self {
            index: (*IndexGenerator::instance::<T>().lock().unwrap()).next(),
            _marker: PhantomData::default(),
        }
    }
}

impl<T: 'static> Display for Index<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.index)
    }
}

/// 手动设置的索引类型。
/// Manually set index type.
#[derive(Debug, Clone)]
pub struct ManualIndex<T: 'static> {
    index: Cell<Option<usize>>,
    _marker: PhantomData<T>,
}

impl<T: 'static> ManualIndex<T> {
    /// 检查是否已设置索引。
    /// Checks if index has been set.
    pub fn indexed(&self) -> bool {
        self.index.get().is_some()
    }

    /// 设置索引值。
    /// Sets the index value.
    pub fn set_index(&self, index: usize) {
        self.index.set(Some(index))
    }
}

impl<T: 'static> Deref for ManualIndex<T> {
    type Target = usize;

    fn deref(&self) -> &usize {
        if self.indexed() {
            unsafe {
                let opt_ptr = self.index.as_ptr();
                let opt_ref = &*opt_ptr;
                opt_ref.as_ref().unwrap_unchecked()
            }
        } else {
            panic!("Attempted to deref ManualIndex that is not indexed");
        }
    }
}

impl<T: 'static> Default for ManualIndex<T> {
    fn default() -> Self {
        Self {
            index: Cell::new(None),
            _marker: PhantomData::default(),
        }
    }
}

impl<T: 'static> Display for ManualIndex<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self.index)
    }
}

/// 可索引trait。
/// Trait for indexable types.
pub trait Indexed<T: 'static = Self>: Sized {
    /// 返回索引值。
    /// Returns the index value.
    fn index(&self) -> usize;

    /// 重置索引生成器。
    /// Resets the index generator.
    fn flush() {
        Self::flush_with::<T>()
    }

    /// 重置指定类型的索引生成器。
    /// Resets the index generator for the specified type.
    fn flush_with<U: 'static>() {
        (*IndexGenerator::instance::<U>().lock().unwrap()).flush();
    }
}

/// 手动索引trait。
/// Trait for manually indexed types.
pub trait ManualIndexed<T: 'static = Self>: Indexed<T> {
    /// 检查是否已设置索引。
    /// Checks if index has been set.
    fn indexed(&self) -> bool;

    /// 设置索引值。
    /// Sets the index value.
    fn set_index(&self, index: usize);

    /// 使用自动生成的索引设置当前对象。
    /// Sets the current object with an auto-generated index.
    fn set_indexed(&self) {
        self.set_indexed_with::<T>()
    }

    /// 使用自动生成的索引设置当前对象（指定类型）。
    /// Sets the current object with an auto-generated index (specified type).
    fn set_indexed_with<U: 'static>(&self) {
        self.set_index((*IndexGenerator::instance::<U>().lock().unwrap()).next())
    }

    /// 刷新索引为新值。
    /// Refreshes the index to a new value.
    fn refresh_index(&self) {
        self.refresh_index_with::<T>()
    }

    /// 刷新索引为新值（指定类型）。
    /// Refreshes the index to a new value (specified type).
    fn refresh_index_with<U: 'static>(&self) {
        self.set_index((*IndexGenerator::instance::<U>().lock().unwrap()).next())
    }
}

/// 可索引切片查找扩展。
/// Indexed slice lookup extension.
pub trait IndexedSliceExt<T>
where
    T: Indexed + 'static,
{
    /// 先按元素 index 查找，找不到时按切片下标获取。
    /// Find by element index first, then fall back to slice position.
    fn find_or_get(&self, index: usize) -> Option<&T>;
}

impl<T> IndexedSliceExt<T> for [T]
where
    T: Indexed + 'static,
{
    fn find_or_get(&self, index: usize) -> Option<&T> {
        self.iter()
            .find(|item| item.index() == index)
            .or_else(|| self.get(index))
    }
}

/// 索引生成器实现。
/// Index generator implementation.
pub struct IndexGeneratorImpl {
    next_index: usize,
}

impl IndexGeneratorImpl {
    /// 创建新的索引生成器。
    /// Creates a new index generator.
    pub fn new() -> Self {
        Self { next_index: 0 }
    }

    /// 获取下一个索引。
    /// Gets the next index.
    pub fn next(&mut self) -> usize {
        let ret = self.next_index;
        self.next_index += 1;
        ret
    }

    /// 重置索引计数器。
    /// Resets the index counter.
    pub fn flush(&mut self) {
        self.next_index = 0
    }
}

struct IndexGenerator {
    inner: HashMap<TypeId, Arc<Mutex<IndexGeneratorImpl>>>,
}

impl IndexGenerator {
    fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    fn get_or_init() -> &'static Mutex<IndexGenerator> {
        static INSTANCE: OnceLock<Mutex<IndexGenerator>> = OnceLock::new();
        INSTANCE.get_or_init(|| Mutex::new(IndexGenerator::new()))
    }

    pub fn instance<T: 'static>() -> Arc<Mutex<IndexGeneratorImpl>> {
        let generator = Self::get_or_init();
        let mut guard = generator.lock().unwrap();

        guard
            .inner
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Arc::new(Mutex::new(IndexGeneratorImpl::new())))
            .clone()
    }
}

/// 为已定义的结构体实现`Indexed`trait。
/// Implements the `Indexed` trait for a defined struct.
#[macro_export]
macro_rules! indexed_type {
    ($vis:vis struct $name:ident { $($fieldVis:vis $field:ident: $type:ty),* }) => {
        impl Indexed for $name {
            fn index(&self) -> usize {
                *self.index
            }
        }

        impl From<$name> for usize {
            fn from(value: $name) -> usize {
                *value.index
            }
        }

        impl<'a> From<&'a $name> for usize {
            fn from(value: &'a $name) -> usize {
                *value.index
            }
        }

        impl From<$name> for isize {
            fn from(value: $name) -> isize {
                *value.index as isize
            }
        }

        impl<'a> From<&'a $name> for isize {
            fn from(value: &'a $name) -> isize {
                *value.index as isize
            }
        }
    };
}

/// 定义自动索引的结构体类型。
/// Defines an auto-indexed struct type.
///
/// 自动添加`index`字段并实现`Indexed`trait。
/// Automatically adds an `index` field and implements the `Indexed` trait.
#[macro_export]
macro_rules! auto_indexed_type {
    ($(#[$derive:meta])* $vis:vis struct $name:ident { $($fieldVis:vis $field:ident: $type:ty),* }) => {
        $(#[$derive])*
        $vis struct $name {
            $($fieldVis $field: $type),*,
            index: Index<$name>,
        }

        indexed_type!($vis struct $name { $($fieldVis $field: $type),* });
    };
}

/// 定义手动索引的结构体类型。
/// Defines a manually indexed struct type.
///
/// 自动添加`index`字段并实现`Indexed`和`ManualIndexed`trait。
/// Automatically adds an `index` field and implements `Indexed` and `ManualIndexed` traits.
#[macro_export]
macro_rules! manual_indexed_type {
    ($(#[$derive:meta])* $vis:vis struct $name:ident { $($fieldVis:vis $field:ident: $type:ty),* }) => {
        $(#[$derive])*
        $vis struct $name {
            $($fieldVis $field: $type),*,
            index: ManualIndex<$name>,
        }

        indexed_type!($vis struct $name { $($fieldVis $field: $type),* });

        impl ManualIndexed<$name> for $name {
           fn indexed(&self) -> bool {
               self.index.indexed()
           }

           fn set_index(&self, index: usize) {
               self.index.set_index(index)
           }
        }
    };
}

/// 创建带索引的结构体实例。
/// Creates an indexed struct instance.
///
/// 自动初始化`index`字段。
/// Automatically initializes the `index` field.
#[macro_export]
macro_rules! indexed {
    ($name:ident { $($field:ident: $val:expr),* }) => {
        $name {
            $($field: $val),*,
            index: Default::default(),
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    auto_indexed_type! {
        pub struct TestAutoIndexed {
            pub name: &'static str,
            pub value: u32
        }
    }

    #[test]
    fn test_auto_indexed_type() {
        let instance1 = indexed!(TestAutoIndexed {
            name: "test1",
            value: 1
        });
        let instance2 = indexed!(TestAutoIndexed {
            name: "test2",
            value: 2
        });

        assert_eq!(instance1.index(), 0);
        assert_eq!(instance2.index(), 1);

        let index1: usize = instance1.into();
        let index2: isize = (&instance2).into();
        assert_eq!(index1, 0);
        assert_eq!(index2, 1);
    }

    manual_indexed_type! {
        pub struct TestManualIndexed {
            pub name: &'static str,
            pub value: u32
        }
    }

    #[test]
    fn test_manual_indexed_type() {
        let instance1 = indexed!(TestManualIndexed {
            name: "test1",
            value: 1
        });
        let instance2 = indexed!(TestManualIndexed {
            name: "test2",
            value: 2
        });

        assert!(!instance1.indexed());
        assert!(!instance2.indexed());

        instance1.set_index(10);
        instance2.set_indexed();

        assert!(instance1.indexed());
        assert!(instance2.indexed());
        assert_eq!(instance1.index(), 10);
        assert_eq!(instance2.index(), 0);

        let index1: usize = instance1.into();
        let index2: isize = (&instance2).into();
        assert_eq!(index1, 10);
        assert_eq!(index2, 0);

        instance2.refresh_index();
        assert_eq!(instance2.index(), 1);
    }

    auto_indexed_type! {
        pub struct TestIndexFlush {
            pub name: &'static str,
            pub value: u32
        }
    }

    auto_indexed_type! {
        pub struct TestFindOrGetIndexed {
            pub name: &'static str,
            pub value: u32
        }
    }

    #[test]
    fn test_flush() {
        let instance1 = indexed!(TestIndexFlush {
            name: "test1",
            value: 1
        });
        assert_eq!(instance1.index(), 0);

        TestIndexFlush::flush();
        let instance2 = indexed!(TestIndexFlush {
            name: "test2",
            value: 2
        });
        assert_eq!(instance2.index(), 0);
    }

    #[test]
    fn test_find_or_get() {
        TestFindOrGetIndexed::flush();
        let instance0 = indexed!(TestFindOrGetIndexed {
            name: "test0",
            value: 0
        });
        let instance1 = indexed!(TestFindOrGetIndexed {
            name: "test1",
            value: 1
        });
        let items = vec![instance1, instance0];

        assert_eq!(items.find_or_get(0).map(|item| item.name), Some("test0"));
        assert_eq!(items.find_or_get(1).map(|item| item.name), Some("test1"));
        assert_eq!(items.find_or_get(2).map(|item| item.name), None);
    }
}

pub use crate::auto_indexed_type;
pub use crate::indexed;
pub use crate::manual_indexed_type;

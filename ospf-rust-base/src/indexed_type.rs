use std::any::TypeId;
use std::cell::Cell;
use std::collections::HashMap;
use std::fmt::Display;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::{Arc, Mutex, OnceLock};

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

#[derive(Debug, Clone)]
pub struct ManualIndex<T: 'static> {
    index: Cell<Option<usize>>,
    _marker: PhantomData<T>,
}

impl<T: 'static> ManualIndex<T> {
    pub fn indexed(&self) -> bool {
        self.index.get().is_some()
    }

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

pub trait Indexed<T: 'static = Self>: Sized {
    fn index(&self) -> usize;

    fn flush() {
        Self::flush_with::<T>()
    }

    fn flush_with<U: 'static>() {
        (*IndexGenerator::instance::<U>().lock().unwrap()).flush();
    }
}

pub trait ManualIndexed<T: 'static = Self>: Indexed<T> {
    fn indexed(&self) -> bool;

    fn set_index(&self, index: usize);

    fn set_indexed(&self) {
        self.set_indexed_with::<T>()
    }

    fn set_indexed_with<U: 'static>(&self) {
        self.set_index((*IndexGenerator::instance::<T>().lock().unwrap()).next())
    }
}

pub struct IndexGeneratorImpl {
    next_index: usize,
}

impl IndexGeneratorImpl {
    pub fn new() -> Self {
        Self { next_index: 0 }
    }

    pub fn next(&mut self) -> usize {
        let ret = self.next_index;
        self.next_index += 1;
        ret
    }

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
        let instance1 = indexed!(TestAutoIndexed { name: "test1", value: 1 });
        let instance2 = indexed!(TestAutoIndexed { name: "test2", value: 2 });

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
        let instance1 = indexed!(TestManualIndexed { name: "test1", value: 1 });
        let instance2 = indexed!(TestManualIndexed { name: "test2", value: 2 });

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
    }

    auto_indexed_type! {
        pub struct TestIndexFlush {
            pub name: &'static str,
            pub value: u32
        }
    }

    #[test]
    fn test_flush() {
        let instance1 = indexed!(TestIndexFlush { name: "test1", value: 1 });
        assert_eq!(instance1.index(), 0);

        TestIndexFlush::flush();
        let instance2 = indexed!(TestIndexFlush { name: "test2", value: 2 });
        assert_eq!(instance2.index(), 0);
    }
}

pub use crate::auto_indexed_type;
pub use crate::manual_indexed_type;
pub use crate::indexed;

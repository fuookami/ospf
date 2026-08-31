use std::any::TypeId;
use std::cell::{Cell, SyncUnsafeCell};
use std::collections::HashMap;
use std::fmt::Display;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::Mutex;

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
            _marker: PhantomData::default()
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
    _marker: PhantomData<T>
}

impl <T: 'static> ManualIndex<T> {
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
        unsafe { self.index.as_ptr().as_ref_unchecked().as_ref().unwrap() }
    }
}

impl<T: 'static> Default for ManualIndex<T> {
    fn default() -> Self {
        Self {
            index: Cell::new(None),
            _marker: PhantomData::default()
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
    inner: Option<HashMap<TypeId, Arc<Mutex<IndexGeneratorImpl>>>>,
}

static mut INDEX_GENERATOR: SyncUnsafeCell<IndexGenerator> = SyncUnsafeCell::new(IndexGenerator {
    inner: None
});

impl IndexGenerator {
    pub fn self_instance() -> &'static mut IndexGenerator {
        let mut instance = unsafe {
            INDEX_GENERATOR.get().as_mut_unchecked()
        };
        if instance.inner.is_none() {
            instance.inner = Some(HashMap::new());
        }
        instance
    }

    pub fn instance<T: 'static>() -> Arc<Mutex<IndexGeneratorImpl>> {
        let instance = Self::self_instance();
        instance.inner.as_mut().unwrap()
            .entry(TypeId::of::<T>())
            .insert_entry(Arc::new(Mutex::new(IndexGeneratorImpl::new())))
            .get()
            .clone()
    }
}

use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

pub trait Indexed: Sized {
    fn index(&self) -> usize;

    fn flush();
}

pub trait ManualIndexed: Indexed {
    fn indexed(&self) -> bool;
    fn set_indexed(&mut self);
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
        return ret;
    }

    pub fn flush(&mut self) {
        self.next_index = 0
    }
}

pub struct IndexGenerator {
    impls: HashMap<TypeId, Arc<Mutex<IndexGeneratorImpl>>>,
}

impl IndexGenerator {
    pub(self) fn self_instance() -> Arc<Mutex<IndexGenerator>> {
        static mut GENERATOR: Option<Arc<Mutex<IndexGenerator>>> = None;

        unsafe {
            GENERATOR
                .get_or_insert_with(|| {
                    Arc::new(Mutex::new(IndexGenerator {
                        impls: HashMap::new(),
                    }))
                })
                .clone()
        }
    }

    pub(crate) fn instance<T: 'static>() -> Arc<Mutex<IndexGeneratorImpl>> {
        let instance = Self::self_instance();
        let impls = &mut instance.lock().unwrap().impls;
        impls
            .entry(TypeId::of::<T>())
            .insert_entry(Arc::new(Mutex::new(IndexGeneratorImpl::new())))
            .get()
            .clone()
    }
}

#[macro_export]
macro_rules! next_index {
    () => {
        (*IndexGenerator::instance::<Self>().lock().unwrap()).next()
    };
}

#[macro_export]
macro_rules! flush {
    () => {
        (*IndexGenerator::instance::<Self>().lock().unwrap()).flush()
    };
}

#[macro_export]
macro_rules! auto_indexed {
    ($($type:ty)*) => {$(
        impl Indexed for $type {
            fn index(&self) -> usize {
                self.index
            }

            fn flush() {
                flush!();
            }
        }

        impl Deref for $type {
            type Target = isize;

            fn deref(&self) -> &Self::Target {
                let ptr = &self.index as *const usize;
                unsafe { &*(ptr as *const isize) }
            }
        }
    )*};
}

#[macro_export]
macro_rules! manual_indexed {
    ($($type:ty)*) => {$(
        impl Indexed for $type {
            fn index(&self) -> usize {
                self.index.unwrap()
            }

            fn flush() {
                flush!();
            }
        }

        impl ManualIndexed for $type {
            fn indexed(&self) -> bool {
                self.index.isSome()
            }

            fn set_indexed(&mut self) {
                self.index = Option::Some(next_index!());
            }
        }

        impl Deref for $type {
            type Target = isize;

            fn deref(&self) -> &Self::Target {
                let ptr = &self.index as *const usize;
                unsafe { &*(ptr as *const isize) }
            }
        }
    )*};
}

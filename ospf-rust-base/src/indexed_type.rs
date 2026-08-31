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
    fn set_indexed(&mut self)
    where
        Self: 'static,
    {
        self.set_indexed_with::<Self>()
    }
    fn set_indexed_with<T: 'static>(&mut self);
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
macro_rules! auto_indexed {
    (#[derive($($derive:meta),*)] $pub:vis struct $name:ident { $($fpub:vis $field:ident : $type:ty,)* }) => {
        #[derive($($derive),*)]
        $pub struct $name {
            index: usize,
            $($fpub $field : $type,)*
        }
        impl $name {
            $pub fn new<T: 'STATIC = Self>($($field:$type,)*) -> Self{
                Self {
                    index: (*IndexGenerator::instance::<T>().lock().unwrap()).next(),
                    $($field,)*
                }
            }
        }

        impl Indexed for $name {
            fn index(&self) -> usize {
                self.index
            }

            fn flush<T: 'STATIC = Self>() {
                (*IndexGenerator::instance::<T>().lock().unwrap()).flush();
            }
        }

        impl Deref for &$name {
            type Target = isize;

            fn deref(&self) -> &Self::Target {
                let ptr = &self.index as *const usize;
                unsafe { &*(ptr as *const isize) }
            }
        }
    }
}

#[macro_export]
macro_rules! manual_indexed {
    (#[derive($($derive:meta),*)] $pub:vis struct $name:ident { $($fpub:vis $field:ident : $type:ty,)* }) => {
        #[derive($($derive),*)]
        $pub struct $name {
            index: Option<usize>,
            $($fpub $field : $type,)*
        }
        impl $name {
            $pub fn new($($field:$type,)*) -> Self{
                Self {
                    index: None,
                    $($field,)*
                }
            }
        }

        impl Indexed for $name {
            fn index(&self) -> usize {
                self.index.unwrap()
            }

            fn flush<T: 'STATIC = Self>() {
                (*IndexGenerator::instance::<T>().lock().unwrap()).flush();
            }
        }

        impl ManualIndexed for $name {
            fn indexed(&self) -> bool {
                self.index.is_some()
            }

            fn set_indexed_with<T: 'static>(&mut self) {
                self.index = Some((*IndexGenerator::instance::<T>().lock().unwrap()).next());
            }
        }

        impl Deref for &$name {
            type Target = isize;

            fn deref(&self) -> &Self::Target {
                let ptr = self.index.as_ref().unwrap() as *const usize;
                unsafe { &*(ptr as *const isize) }
            }
        }
    }
}

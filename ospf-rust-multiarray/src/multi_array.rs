use crate::dummy_vector::*;
use crate::*;
use std::ops::{Index, IndexMut};

pub struct MultiArray<T: Sized, S: Shape> {
    pub(self) list: Vec<Option<T>>,
    pub(self) shape: S,
}

impl<T: Sized, S: Shape> MultiArray<T, S> {
    pub fn new(shape: S) -> Self {
        Self {
            list: (0..shape.len()).map(|_| Option::None).collect(),
            shape: shape,
        }
    }

    pub fn new_with(shape: S, value: T) -> Self
    where
        T: Clone,
    {
        Self {
            list: (0..shape.len())
                .map(|_| Option::Some(value.clone()))
                .collect(),
            shape: shape,
        }
    }

    pub fn new_by<G>(shape: S, generator: G) -> Self
    where
        G: Fn(usize) -> T,
    {
        Self {
            list: (0..shape.len())
                .map(|index| Option::Some(generator(index)))
                .collect(),
            shape: shape,
        }
    }

    pub fn get(&self, vector: S::DummyVectorType) -> Result<Vec<&Option<T>>, OutOfShapeError> {
        let mut ret = Vec::new();
        let policy = DummyAccessPolicy::new(&vector, &self.shape);
        let mut iter = policy.iter();
        loop {
            match iter.next() {
                Some(vector) => {
                    let index = self.shape.index(vector).unwrap();
                    ret.push(&self.list[index]);
                }
                None => {
                    break;
                }
            }
        }
        Ok(ret)
    }

    // fn map<'a>(&'a self, vector: &S::MapVectorType) -> MultiArrayView<'a, T, DynShape> {}
}

impl<T: Sized, S: Shape> Index<usize> for MultiArray<T, S> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        match &self.list[index] {
            Some(value) => value,
            None => {
                panic!(
                    "Element with index {} in the multi-array is not initialized",
                    index
                )
            }
        }
    }
}

impl<T: Sized, S: Shape> IndexMut<usize> for MultiArray<T, S> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match &mut self.list[index] {
            Some(value) => value,
            None => {
                panic!(
                    "Element with index {} in the multi-array is not initialized",
                    index
                )
            }
        }
    }
}

impl<T: Sized, S: Shape> Index<&S::VectorType> for MultiArray<T, S> {
    type Output = T;

    fn index(&self, vector: &S::VectorType) -> &Self::Output {
        match self.shape.index(vector) {
            Ok(index) => match &self.list[index] {
                Some(value) => value,
                None => {
                    panic!(
                        "Element with index {} in the multi-array is not initialized",
                        index
                    )
                }
            },
            Err(err) => panic!("{}", err),
        }
    }
}

impl<T: Sized, S: Shape> IndexMut<&S::VectorType> for MultiArray<T, S> {
    fn index_mut(&mut self, vector: &S::VectorType) -> &mut Self::Output {
        match self.shape.index(vector) {
            Ok(index) => match &mut self.list[index] {
                Some(value) => value,
                None => {
                    panic!(
                        "Element with index {} in the multi-array is not initialized",
                        index
                    )
                }
            },
            Err(err) => panic!("{}", err),
        }
    }
}

type MultiArray1<T> = MultiArray<T, Shape1>;
type MultiArray2<T> = MultiArray<T, Shape2>;
type MultiArray3<T> = MultiArray<T, Shape3>;
type MultiArray4<T> = MultiArray<T, Shape4>;
type MultiArray5<T> = MultiArray<T, Shape5>;
type MultiArray6<T> = MultiArray<T, Shape6>;
type MultiArray7<T> = MultiArray<T, Shape7>;
type MultiArray8<T> = MultiArray<T, Shape8>;
type MultiArray9<T> = MultiArray<T, Shape9>;
type MultiArray10<T> = MultiArray<T, Shape10>;
type MultiArray11<T> = MultiArray<T, Shape11>;
type MultiArray12<T> = MultiArray<T, Shape12>;
type MultiArray13<T> = MultiArray<T, Shape13>;
type MultiArray14<T> = MultiArray<T, Shape14>;
type MultiArray15<T> = MultiArray<T, Shape15>;
type MultiArray16<T> = MultiArray<T, Shape16>;
type MultiArray17<T> = MultiArray<T, Shape17>;
type MultiArray18<T> = MultiArray<T, Shape18>;
type MultiArray19<T> = MultiArray<T, Shape19>;
type MultiArray20<T> = MultiArray<T, Shape20>;
type DynMultiArray<T> = MultiArray<T, DynShape>;

macro_rules! vector_index {
    ($x:literal) => {
        $x as usize
    };
    ($x:expr) => {
        $x as usize
    };
}

#[macro_export]
macro_rules! vector {
    ($($x:expr),*) => {
        &[$(vector_index!($x),)*]
    };
}

#[macro_export]
macro_rules! dyn_vector {
    ($($x:expr),*) => {
        vec!($(vector_index!($x),)*)
    };
}

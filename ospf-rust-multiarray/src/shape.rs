use std::mem;

use super::dummy_index::DummyIndex;
use super::error::{DimensionMismatchingError, IndexCalculationError, OutOfShapeError};
use super::index_vector::IndexVector;

const DYN_DIMENSION: usize = usize::MAX;

pub trait AbstractShape {
    const DIMENSION: usize;
    type VectorType: IndexVector<usize> + Clone;
    type DummyVectorType: IndexVector<DummyIndex> + Clone;

    fn zero(&self) -> Self::VectorType;

    fn len(&self) -> usize;
    fn dimension(&self) -> usize {
        Self::DIMENSION
    }
    fn dimension_of(_: &Self::VectorType) -> usize {
        Self::DIMENSION
    }

    fn shape(&self) -> &[usize];
    fn offset(&self) -> &[usize];

    fn len_of_dimension(&self, dimension: usize) -> Result<usize, DimensionMismatchingError> {
        if dimension > Self::DIMENSION {
            Err(DimensionMismatchingError {
                dimension: Self::DIMENSION,
                vector_dimension: dimension,
            })
        } else {
            Ok(self.shape()[dimension])
        }
    }

    fn offset_of_dimension(&self, dimension: usize) -> Result<usize, DimensionMismatchingError> {
        if dimension > Self::DIMENSION {
            Err(DimensionMismatchingError {
                dimension: Self::DIMENSION,
                vector_dimension: dimension,
            })
        } else {
            Ok(self.offset()[dimension])
        }
    }

    fn index(&self, vector: &Self::VectorType) -> Result<usize, IndexCalculationError> {
        if Self::dimension_of(vector) > self.dimension() {
            Err(IndexCalculationError::DimensionMismatching(
                DimensionMismatchingError {
                    dimension: self.dimension(),
                    vector_dimension: Self::dimension_of(vector),
                },
            ))
        } else {
            let mut index = 0;
            for i in 0..self.dimension() {
                if vector[i] > self.len_of_dimension(i).unwrap() {
                    return Err(IndexCalculationError::OutOfShape(OutOfShapeError {
                        dimension: i,
                        len: self.len_of_dimension(i).unwrap(),
                        vector_index: vector[i] as isize,
                    }));
                }
                index += vector[i] * self.offset_of_dimension(i).unwrap();
            }
            Ok(index)
        }
    }

    fn vector(&self, mut index: usize) -> Self::VectorType {
        let mut vector = self.zero();
        for i in 0..self.dimension() {
            let offset = self.offset_of_dimension(i).unwrap();
            vector[i] = index / offset;
            index = index % offset;
        }
        vector
    }

    fn next_vector(&self, vector: &mut Self::VectorType) -> bool {
        let mut carry = false;
        vector[self.dimension() - 1] += 1;

        for i in (0..self.dimension()).rev() {
            if carry {
                vector[i] += 1;
                carry = false;
            }
            if vector[i] == self.len_of_dimension(i).unwrap() {
                vector[i] = 0;
                carry = true;
            }
        }
        !carry
    }

    fn actual_index(&self, dimension: usize, index: isize) -> Option<usize> {
        let len = self.len_of_dimension(dimension).unwrap();
        if index >= (len as isize) || index < -(len as isize) {
            None
        } else {
            Some((index % (len as isize)) as usize)
        }
    }
}

pub(self) fn offset<const DIMENSION: usize>(
    shape: &[usize; DIMENSION],
) -> ([usize; DIMENSION], usize) {
    let mut offset: [usize; DIMENSION] = unsafe { mem::zeroed() };

    offset[shape.len() - 1] = 1;
    let mut len = 1;
    for i in (0..(shape.len() - 1)).rev() {
        offset[i] = offset[i + 1] * shape[i + 1];
        len *= shape[i + 1];
    }
    len *= shape[0];
    (offset, len)
}

#[derive(Clone, Copy)]
pub struct Shape<const d: usize> {
    pub(self) shape: [usize; d],
    pub(self) offset: [usize; d],
    pub(self) len: usize,
}

impl<const d: usize> Shape<d> {
    pub fn new(shape: [usize; d]) -> Self {
        let (offset, len) = offset(&shape);
        Self { shape, offset, len }
    }
}

impl<const d: usize> AbstractShape for Shape<d> {
    const DIMENSION: usize = d;
    type VectorType = [usize; d];
    type DummyVectorType = [DummyIndex; d];

    fn zero(&self) -> Self::VectorType {
        unsafe { mem::zeroed() }
    }

    fn len(&self) -> usize {
        self.len
    }

    fn shape(&self) -> &[usize] {
        &self.shape
    }

    fn offset(&self) -> &[usize] {
        &self.offset
    }
}

macro_rules! shape {
    ($type:ident, $dim:expr) => {
        paste! {
            pub type [<Shape $dim>] = Shape<$dim>;
        }
    };
}

pub type Shape1 = Shape<1>;
pub type Shape2 = Shape<2>;
pub type Shape3 = Shape<3>;
pub type Shape4 = Shape<4>;
pub type Shape5 = Shape<5>;
pub type Shape6 = Shape<6>;
pub type Shape7 = Shape<7>;
pub type Shape8 = Shape<8>;
pub type Shape9 = Shape<9>;
pub type Shape10 = Shape<10>;
pub type Shape11 = Shape<11>;
pub type Shape12 = Shape<12>;
pub type Shape13 = Shape<13>;
pub type Shape14 = Shape<14>;
pub type Shape15 = Shape<15>;
pub type Shape16 = Shape<16>;
pub type Shape17 = Shape<17>;
pub type Shape18 = Shape<18>;
pub type Shape19 = Shape<19>;
pub type Shape20 = Shape<20>;

pub struct DynShape {
    pub(self) shape: Vec<usize>,
    pub(self) offset: Vec<usize>,
    pub(self) len: usize,
}

impl DynShape {
    pub fn new(shape: Vec<usize>) -> Self {
        let (offset, len) = Self::offset(&shape);
        Self { shape, offset, len }
    }

    pub(self) fn offset(shape: &Vec<usize>) -> (Vec<usize>, usize) {
        let mut offset: Vec<usize> = (0..shape.len()).map(|_| 0).collect();
        offset[shape.len() - 1] = 1;
        let mut len = 1;
        for i in (0..(shape.len() - 1)).rev() {
            offset[i] = offset[i + 1] * shape[i + 1];
            len *= shape[i + 1];
        }
        len *= shape[0];
        (offset, len)
    }
}

impl AbstractShape for DynShape {
    const DIMENSION: usize = DYN_DIMENSION;
    type VectorType = Vec<usize>;
    type DummyVectorType = Vec<DummyIndex>;

    fn zero(&self) -> Self::VectorType {
        (0..self.shape.len()).map(|_| 0).collect()
    }

    fn len(&self) -> usize {
        self.len
    }

    fn dimension(&self) -> usize {
        self.shape.len()
    }

    fn dimension_of(vector: &Self::VectorType) -> usize {
        vector.len()
    }

    fn shape(&self) -> &[usize] {
        &self.shape
    }

    fn offset(&self) -> &[usize] {
        &self.offset
    }
}

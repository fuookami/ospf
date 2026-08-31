use std::fmt::{Debug, Display, Formatter};

#[derive(Clone, Copy)]
pub struct DimensionMismatchingError {
    pub dimension: usize,
    pub vector_dimension: usize,
}

impl Display for DimensionMismatchingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Dimension should be {}, not {}.",
            self.dimension, self.vector_dimension
        )
    }
}

impl Debug for DimensionMismatchingError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "Dimension should be {}, not {}.",
            self.dimension, self.vector_dimension
        )
    }
}

#[derive(Clone, Copy)]
pub struct OutOfShapeError {
    pub dimension: usize,
    pub len: usize,
    pub vector_index: isize,
}

impl Display for OutOfShapeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Length of dimension {} is {}, but it get {}.",
            self.dimension, self.len, self.vector_index
        )
    }
}

impl Debug for OutOfShapeError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "Length of dimension {} is {}, but it get {}.",
            self.dimension, self.len, self.vector_index
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub enum IndexCalculationError {
    DimensionMismatching(DimensionMismatchingError),
    OutOfShape(OutOfShapeError),
}

impl Display for IndexCalculationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexCalculationError::DimensionMismatching(err) => {
                write!(f, "{}", err)
            }
            IndexCalculationError::OutOfShape(err) => {
                write!(f, "{}", err)
            }
        }
    }
}

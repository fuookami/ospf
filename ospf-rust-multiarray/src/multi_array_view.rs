use crate::{AbstractShape, MultiArray};

pub struct MultiArrayView<'a, T: Sized, S: AbstractShape> {
    pub(self) parent: &'a MultiArray<T, S>,
    pub(self) list: Vec<Vec<&'a T>>,
    pub(self) shape: S,
}

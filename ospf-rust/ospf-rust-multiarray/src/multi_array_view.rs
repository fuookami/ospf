use crate::{MultiArray, Shape};

pub struct MultiArrayView<'a, T: Sized, S: Shape> {
    pub(self) parent: &'a MultiArray<T, S>,
    pub(self) list: Vec<Vec<Option<&'a T>>>,
    pub(self) shape: S,
}

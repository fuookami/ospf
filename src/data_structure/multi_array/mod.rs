pub struct MultiArray<T, const D: usize> {
    _items: Vec<T>,
    _shape: Vec<usize>
}

impl<T, const D: usize> MultiArray<T, D> {
    pub fn new() -> Self {
        Self {
            _items: Vec::new(),
            _shape: Vec::new()
        }
    }

    pub fn shape(&self) -> &Vec<usize> {
        &self._shape
    }

    pub fn dimension(&self) -> usize {
        D
    }
}

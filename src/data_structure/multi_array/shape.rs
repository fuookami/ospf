trait Shape {
    type Index;
    const Dimension: usize;

    fn new() -> Self;
    fn new_from_array(shape: [Index; Dimension]) -> Self;
}

struct ShapeImpl<T, const D: usize> {
    _shape: [T; D]
}

impl<T, const D: usize> for ShapeImpl<T, D> {

}

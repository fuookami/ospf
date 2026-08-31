pub trait None: Iterator {
    fn none<F>(&mut self, f: F) -> bool
    where
        Self: Sized,
        F: FnMut(<Self as Iterator>::Item) -> bool;
}

impl<T: Sized + Iterator> None for T {
    fn none<F>(&mut self, mut f: F) -> bool
    where
        F: FnMut(<Self as Iterator>::Item) -> bool,
    {
        self.all(|x| !f(x))
    }
}

//! 迭代器扩展。
//! Iterator extensions.

/// 判断迭代器中没有元素满足条件的trait。
/// Trait for checking if no elements satisfy a condition.
pub trait None: Iterator {
    /// 检查是否没有元素满足指定条件。
    /// Checks if no elements satisfy the specified condition.
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

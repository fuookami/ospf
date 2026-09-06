//! 可克隆函数trait宏。
//! Macro for defining cloneable function traits.

#[macro_export]
/// 定义可克隆的函数trait。
/// Defines a cloneable function trait.
///
/// 生成的trait继承自`Fn`，并添加`clone_box`方法，使得`Box<dyn Trait>`可以克隆。
/// The generated trait inherits from `Fn` and adds a `clone_box` method, enabling `Box<dyn Trait>` to be cloneable.
macro_rules! cloneable_function {
    ($visible:vis type $type:ident = Fn($($arg:ty),*) -> $ret:ty) => {
        $visible trait $type: Fn($($arg),*) -> $ret {
            fn clone_box<'a>(&self) -> Box<dyn 'a + $type>
            where
                Self: 'a;
        }

        impl<F> $type for F
        where
            F: Fn($($arg),*) -> $ret + Clone,
        {
            fn clone_box<'a>(&self) -> Box<dyn 'a + $type>
            where
                Self: 'a,
            {
                Box::new(self.clone())
            }
        }

        impl<'a> Clone for Box<dyn 'a + $type> {
            fn clone(&self) -> Self {
                (**self).clone_box()
            }
        }
    };
}

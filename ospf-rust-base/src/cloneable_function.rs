#[macro_export]
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

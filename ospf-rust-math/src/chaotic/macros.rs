//! 混沌系统构造宏。
//! Chaotic system construction macros.

#![allow(unused_macros)]

/// 生成基于三维点的混沌系统结构体及生成器。
/// Generate a 3D-point-based chaotic system struct and its generator.
macro_rules! point3_system {
    ($(#[$attr:meta])* $name:ident, $(#[$gattr:meta])* $generator:ident, [$($field:ident),+], |$system:ident, $state:ident| $step:block) => {
        $(#[$attr])*
        #[derive(Clone, Debug, PartialEq)]
        pub struct $name<S: $crate::algebra::Field + num_traits::Float = f64> {
            $( $field: S, )+
        }

        impl<S: $crate::algebra::Field + num_traits::Float> $name<S> {
            pub fn new($($field: S),+) -> Self {
                Self { $($field),+ }
            }

            $(
                pub fn $field(&self) -> S {
                    self.$field
                }
            )+

            pub fn step(&self, x: $crate::geometry::Point3<S>) -> $crate::geometry::Point3<S> {
                let $system = self;
                let $state = x;
                $step
            }

            pub fn generator(self, initial: $crate::geometry::Point3<S>) -> $generator<S> {
                $generator::new(self, initial)
            }
        }

        $(#[$gattr])*
        #[derive(Clone, Debug, PartialEq)]
        pub struct $generator<S: $crate::algebra::Field + num_traits::Float = f64> {
            system: $name<S>,
            x: $crate::geometry::Point3<S>,
        }

        impl<S: $crate::algebra::Field + num_traits::Float> $generator<S> {
            pub fn new(system: $name<S>, x: $crate::geometry::Point3<S>) -> Self {
                Self { system, x }
            }

            pub fn system(&self) -> &$name<S> {
                &self.system
            }

            pub fn x(&self) -> &$crate::geometry::Point3<S> {
                &self.x
            }

            pub fn next_point(&mut self) -> $crate::geometry::Point3<S> {
                let x = self.x.clone();
                self.x = self.system.step(self.x.clone());
                x
            }
        }

        impl<S: $crate::algebra::Field + num_traits::Float> Iterator for $generator<S> {
            type Item = $crate::geometry::Point3<S>;

            fn next(&mut self) -> Option<Self::Item> {
                Some(self.next_point())
            }
        }
    };
}

/// 生成基于二维点的混沌系统结构体及生成器。
/// Generate a 2D-point-based chaotic system struct and its generator.
macro_rules! point2_system {
    ($(#[$attr:meta])* $name:ident, $(#[$gattr:meta])* $generator:ident, [$($field:ident),+], |$system:ident, $state:ident| $step:block) => {
        $(#[$attr])*
        #[derive(Clone, Debug, PartialEq)]
        pub struct $name<S: $crate::algebra::Field + num_traits::Float = f64> {
            $( $field: S, )+
        }

        impl<S: $crate::algebra::Field + num_traits::Float> $name<S> {
            pub fn new($($field: S),+) -> Self {
                Self { $($field),+ }
            }

            $(
                pub fn $field(&self) -> S {
                    self.$field
                }
            )+

            pub fn step(&self, x: $crate::geometry::Point2<S>) -> $crate::geometry::Point2<S> {
                let $system = self;
                let $state = x;
                $step
            }

            pub fn generator(self, initial: $crate::geometry::Point2<S>) -> $generator<S> {
                $generator::new(self, initial)
            }
        }

        $(#[$gattr])*
        #[derive(Clone, Debug, PartialEq)]
        pub struct $generator<S: $crate::algebra::Field + num_traits::Float = f64> {
            system: $name<S>,
            x: $crate::geometry::Point2<S>,
        }

        impl<S: $crate::algebra::Field + num_traits::Float> $generator<S> {
            pub fn new(system: $name<S>, x: $crate::geometry::Point2<S>) -> Self {
                Self { system, x }
            }

            pub fn system(&self) -> &$name<S> {
                &self.system
            }

            pub fn x(&self) -> &$crate::geometry::Point2<S> {
                &self.x
            }

            pub fn next_point(&mut self) -> $crate::geometry::Point2<S> {
                let x = self.x.clone();
                self.x = self.system.step(self.x.clone());
                x
            }
        }

        impl<S: $crate::algebra::Field + num_traits::Float> Iterator for $generator<S> {
            type Item = $crate::geometry::Point2<S>;

            fn next(&mut self) -> Option<Self::Item> {
                Some(self.next_point())
            }
        }
    };
}

macro_rules! point4_system {
    ($(#[$attr:meta])* $name:ident, $(#[$gattr:meta])* $generator:ident, [$($field:ident),+], |$system:ident, $state:ident| $step:block) => {
        $(#[$attr])*
        #[derive(Clone, Debug, PartialEq)]
        pub struct $name<S: $crate::algebra::Field + num_traits::Float = f64> {
            $( $field: S, )+
        }
        impl<S: $crate::algebra::Field + num_traits::Float> $name<S> {
            pub fn new($($field: S),+) -> Self { Self { $($field),+ } }
            $(pub fn $field(&self) -> S { self.$field })+
            pub fn step(&self, x: $crate::geometry::Point4<S>) -> $crate::geometry::Point4<S> {
                let $system = self; let $state = x; $step
            }
            pub fn generator(self, initial: $crate::geometry::Point4<S>) -> $generator<S> { $generator::new(self, initial) }
        }
        $(#[$gattr])*
        #[derive(Clone, Debug, PartialEq)]
        pub struct $generator<S: $crate::algebra::Field + num_traits::Float = f64> { system: $name<S>, x: $crate::geometry::Point4<S> }
        impl<S: $crate::algebra::Field + num_traits::Float> $generator<S> {
            pub fn new(system: $name<S>, x: $crate::geometry::Point4<S>) -> Self { Self { system, x } }
            pub fn system(&self) -> &$name<S> { &self.system }
            pub fn x(&self) -> &$crate::geometry::Point4<S> { &self.x }
            pub fn next_point(&mut self) -> $crate::geometry::Point4<S> { let x = self.x.clone(); self.x = self.system.step(self.x.clone()); x }
        }
        impl<S: $crate::algebra::Field + num_traits::Float> Iterator for $generator<S> {
            type Item = $crate::geometry::Point4<S>;
            fn next(&mut self) -> Option<Self::Item> { Some(self.next_point()) }
        }
    };
}

/// 生成基于标量的混沌映射结构体及生成器。
/// Generate a scalar-based chaotic map struct and its generator.
macro_rules! scalar_map {
    ($(#[$attr:meta])* $name:ident, $(#[$gattr:meta])* $generator:ident, [$($field:ident),+], |$system:ident, $state:ident| $step:block) => {
        $(#[$attr])*
        #[derive(Clone, Debug, PartialEq)]
        pub struct $name<S: $crate::algebra::Field + num_traits::Float = f64> {
            $( $field: S, )+
        }

        impl<S: $crate::algebra::Field + num_traits::Float> $name<S> {
            pub fn new($($field: S),+) -> Self {
                Self { $($field),+ }
            }

            $(
                pub fn $field(&self) -> S {
                    self.$field
                }
            )+

            pub fn step(&self, x: S) -> S {
                let $system = self;
                let $state = x;
                $step
            }

            pub fn generator(self, initial: S) -> $generator<S> {
                $generator::new(self, initial)
            }
        }

        $(#[$gattr])*
        #[derive(Clone, Debug, PartialEq)]
        pub struct $generator<S: $crate::algebra::Field + num_traits::Float = f64> {
            system: $name<S>,
            x: S,
        }

        impl<S: $crate::algebra::Field + num_traits::Float> $generator<S> {
            pub fn new(system: $name<S>, x: S) -> Self {
                Self { system, x }
            }

            pub fn system(&self) -> &$name<S> {
                &self.system
            }

            pub fn x(&self) -> S {
                self.x
            }

            pub fn next_value(&mut self) -> S {
                let x = self.x;
                self.x = self.system.step(self.x);
                x
            }
        }

        impl<S: $crate::algebra::Field + num_traits::Float> Iterator for $generator<S> {
            type Item = S;

            fn next(&mut self) -> Option<Self::Item> {
                Some(self.next_value())
            }
        }
    };
}

use paste::paste;

pub trait FundamentalDimension {}

pub trait FundamentalQuantity {
    type Dimension: FundamentalDimension;
    const INDEX: i32;
}

struct Multiply<
    Dimension: FundamentalDimension,
    Lhs: FundamentalQuantity<Dimension = Dimension>,
    Rhs: FundamentalQuantity<Dimension = Dimension>,
> {
    _marker: std::marker::PhantomData<(Dimension, Lhs, Rhs)>,
}

impl<
        Dimension: FundamentalDimension,
        Lhs: FundamentalQuantity<Dimension = Dimension>,
        Rhs: FundamentalQuantity<Dimension = Dimension>,
    > FundamentalQuantity for Multiply<Dimension, Lhs, Rhs>
{
    type Dimension = Dimension;
    const INDEX: i32 = Lhs::INDEX + Rhs::INDEX;
}

struct Divide<
    Dimension: FundamentalDimension,
    Lhs: FundamentalQuantity<Dimension = Dimension>,
    Rhs: FundamentalQuantity<Dimension = Dimension>,
> {
    _marker: std::marker::PhantomData<(Dimension, Lhs, Rhs)>,
}

impl<
        Dimension: FundamentalDimension,
        Lhs: FundamentalQuantity<Dimension = Dimension>,
        Rhs: FundamentalQuantity<Dimension = Dimension>,
    > FundamentalQuantity for Divide<Dimension, Lhs, Rhs>
{
    type Dimension = Lhs::Dimension;
    const INDEX: i32 = Lhs::INDEX - Rhs::INDEX;
}

struct Neg<Quantity: FundamentalQuantity> {
    _marker: std::marker::PhantomData<Quantity>,
}

impl<Quantity: FundamentalQuantity> FundamentalQuantity for Neg<Quantity> {
    type Dimension = Quantity::Dimension;
    const INDEX: i32 = -Quantity::INDEX;
}

struct Pow<Quantity: FundamentalQuantity, const INDEX: i32> {
    _marker: std::marker::PhantomData<Quantity>,
}

impl<Quantity: FundamentalQuantity, const INDEX: i32> FundamentalQuantity for Pow<Quantity, INDEX> {
    type Dimension = Quantity::Dimension;
    const INDEX: i32 = Quantity::INDEX * INDEX;
}

macro_rules! fundamental_quantity_template {
    ($type:ident, $dimension:literal) => {
        paste! {
            pub struct [<$type $dimension>] {}

            impl FundamentalQuantity for [<$type $dimension>] {
                type Dimension = $type;
                const INDEX: i32 = $dimension;
            }

            pub struct [<$type N $dimension>] {}

            impl FundamentalQuantity for [<$type N $dimension>] {
                type Dimension = $type;
                const INDEX: i32 = -$dimension;
            }
        }
    };
}

macro_rules! fundamental_dimension_template {
    ($($type:ident)*) => ($(
        pub struct $type {}

        impl FundamentalDimension for $type {}

        paste! {
            pub struct [<$type 0>] {}

            impl FundamentalQuantity for [<$type 0>] {
                type Dimension = $type;
                const INDEX: i32 = 0;
            }
        }

        fundamental_quantity_template!($type, 1);
        fundamental_quantity_template!($type, 2);
        fundamental_quantity_template!($type, 3);
        fundamental_quantity_template!($type, 4);
        fundamental_quantity_template!($type, 5);
        fundamental_quantity_template!($type, 6);
        fundamental_quantity_template!($type, 7);
        fundamental_quantity_template!($type, 8);
        fundamental_quantity_template!($type, 9);
        fundamental_quantity_template!($type, 10);
    )*)
}
// L: Length, M: Mass, T: Time, I: Current Intensity, O: Temperature, N: Substance Amount, J: Luminous Intensity, R: Rad, S: Sr, B: Information
fundamental_dimension_template! { M T I O N J R S B }

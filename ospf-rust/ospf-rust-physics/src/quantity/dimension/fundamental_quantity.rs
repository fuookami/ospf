use concat_idents::concat_idents;
use ospf_rust_meta_programming::*;

pub trait FundamentalDimension {}

pub trait FundamentalQuantity {
    type Dimension: FundamentalDimension;
    const INDEX: i32;
}

pub trait DimensionSame<Quantity: FundamentalQuantity> {}
impl<Lhs: FundamentalQuantity, Rhs: FundamentalQuantity> DimensionSame<Rhs> for Lhs where
    IsSameType<Lhs::Dimension, Rhs::Dimension>: SameType
{
}

struct Multiply<Lhs: FundamentalQuantity, Rhs: FundamentalQuantity>
where
    Lhs: DimensionSame<Rhs>,
{
    _marker: std::marker::PhantomData<(Lhs, Rhs)>,
}
impl<Lhs: FundamentalQuantity, Rhs: FundamentalQuantity> FundamentalQuantity for Multiply<Lhs, Rhs>
where
    Lhs: DimensionSame<Rhs>,
{
    type Dimension = Lhs::Dimension;
    const INDEX: i32 = Lhs::INDEX + Rhs::INDEX;
}

struct Divide<Lhs: FundamentalQuantity, Rhs: FundamentalQuantity>
where
    Lhs: DimensionSame<Rhs>,
{
    _marker: std::marker::PhantomData<(Lhs, Rhs)>,
}
impl<Lhs: FundamentalQuantity, Rhs: FundamentalQuantity> FundamentalQuantity for Divide<Lhs, Rhs>
where
    Lhs: DimensionSame<Rhs>,
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

macro_rules! fundamental_dimension_template {
    ($($type:ident)*) => ($(
        pub struct $type {}

        impl FundamentalDimension for $type {}

        concat_idents!(quantity_name = $type, 0 {
            pub struct quantity_name {}

            impl FundamentalQuantity for quantity_name {
                type Dimension = $type;
                const INDEX: i32 = 0;
            }
        });

        concat_idents!(quantity_name = $type, 1 {
            pub struct quantity_name {}

            impl FundamentalQuantity for quantity_name {
                type Dimension = $type;
                const INDEX: i32 = 1;
            }
        });

        concat_idents!(quantity_name = $type, N1 {
            pub struct quantity_name {}

            impl FundamentalQuantity for quantity_name {
                type Dimension = $type;
                const INDEX: i32 = -1;
            }
        });

        concat_idents!(quantity_name = $type, 2 {
            pub struct quantity_name {}

            impl FundamentalQuantity for quantity_name {
                type Dimension = $type;
                const INDEX: i32 = 2;
            }
        });

        concat_idents!(quantity_name = $type, N2 {
            pub struct quantity_name {}

            impl FundamentalQuantity for quantity_name {
                type Dimension = $type;
                const INDEX: i32 = -2;
            }
        });


        concat_idents!(quantity_name = $type, 3 {
            pub struct quantity_name {}

            impl FundamentalQuantity for quantity_name {
                type Dimension = $type;
                const INDEX: i32 = 3;
            }
        });

        concat_idents!(quantity_name = $type, N3 {
            pub struct quantity_name {}

            impl FundamentalQuantity for quantity_name {
                type Dimension = $type;
                const INDEX: i32 = -3;
            }
        });

        concat_idents!(quantity_name = $type, 4 {
            pub struct quantity_name {}

            impl FundamentalQuantity for quantity_name {
                type Dimension = $type;
                const INDEX: i32 = 4;
            }
        });

        concat_idents!(quantity_name = $type, N4 {
            pub struct quantity_name {}

            impl FundamentalQuantity for quantity_name {
                type Dimension = $type;
                const INDEX: i32 = -4;
            }
        });

        concat_idents!(quantity_name = $type, 5 {
            pub struct quantity_name {}

            impl FundamentalQuantity for quantity_name {
                type Dimension = $type;
                const INDEX: i32 = 5;
            }
        });

        concat_idents!(quantity_name = $type, N5 {
            pub struct quantity_name {}

            impl FundamentalQuantity for quantity_name {
                type Dimension = $type;
                const INDEX: i32 = -5;
            }
        });

        concat_idents!(quantity_name = $type, 6 {
            pub struct quantity_name {}

            impl FundamentalQuantity for quantity_name {
                type Dimension = $type;
                const INDEX: i32 = 6;
            }
        });

        concat_idents!(quantity_name = $type, N6 {
            pub struct quantity_name {}

            impl FundamentalQuantity for quantity_name {
                type Dimension = $type;
                const INDEX: i32 = -6;
            }
        });

        concat_idents!(quantity_name = $type, 7 {
            pub struct quantity_name {}

            impl FundamentalQuantity for quantity_name {
                type Dimension = $type;
                const INDEX: i32 = 7;
            }
        });

        concat_idents!(quantity_name = $type, N7 {
            pub struct quantity_name {}

            impl FundamentalQuantity for quantity_name {
                type Dimension = $type;
                const INDEX: i32 = -7;
            }
        });

        concat_idents!(quantity_name = $type, 8 {
            pub struct quantity_name {}

            impl FundamentalQuantity for quantity_name {
                type Dimension = $type;
                const INDEX: i32 = 8;
            }
        });

        concat_idents!(quantity_name = $type, N8 {
            pub struct quantity_name {}

            impl FundamentalQuantity for quantity_name {
                type Dimension = $type;
                const INDEX: i32 = -8;
            }
        });

        concat_idents!(quantity_name = $type, 9 {
            pub struct quantity_name {}

            impl FundamentalQuantity for quantity_name {
                type Dimension = $type;
                const INDEX: i32 = 9;
            }
        });

        concat_idents!(quantity_name = $type, N9 {
            pub struct quantity_name {}

            impl FundamentalQuantity for quantity_name {
                type Dimension = $type;
                const INDEX: i32 = -9;
            }
        });

        concat_idents!(quantity_name = $type, 10 {
            pub struct quantity_name {}

            impl FundamentalQuantity for quantity_name {
                type Dimension = $type;
                const INDEX: i32 = 10;
            }
        });

        concat_idents!(quantity_name = $type, N10 {
            pub struct quantity_name {}

            impl FundamentalQuantity for quantity_name {
                type Dimension = $type;
                const INDEX: i32 = -10;
            }
        });
    )*)
}
// L: Length, M: Mass, T: Time, I: Current Intensity, O: Temperature, N: Substance Amount, J: Luminous Intensity, R: Rad, S: Sr, B: Information
fundamental_dimension_template! { M T I O N J R S B }

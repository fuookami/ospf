use paste::paste;

use ospf_rust_multiarray::Shape;

pub use item::{VariableItem, VariableItemTag};
pub use range::VariableRangeType;
pub use variable_type::*;

pub(super) mod combination_item;
pub(super) mod independent_item;
pub mod item;
pub mod range;
pub mod variable_type;

macro_rules! variable_type_exporter_template {
    ($id:ident, $type:ident) => {
        paste! {
            pub type [<$id Var>] = independent_item::IndependentVariableItem<$type>;
            pub type [<$id Variable 1>] = combination_item::Variable1<$type>;
            pub type [<$id Variable 2>] = combination_item::Variable2<$type>;
            pub type [<$id Variable 3>] = combination_item::Variable3<$type>;
            pub type [<$id Variable 4>] = combination_item::Variable4<$type>;
            pub type [<Dyn $id Variable>] = combination_item::DynVariable<$type>;
            pub type [<$id Variable>]<const D: usize> = combination_item::VariableCombination<$type, Shape<D>>;
        }
    };
}

variable_type_exporter_template!(Bin, Binary);
variable_type_exporter_template!(Ter, Ternary);
variable_type_exporter_template!(BTer, BalancedTernary);
variable_type_exporter_template!(Pct, Percentage);
variable_type_exporter_template!(Int, Integer);
variable_type_exporter_template!(UInt, UInteger);
variable_type_exporter_template!(Real, Continuous);
variable_type_exporter_template!(UReal, UContinuous);

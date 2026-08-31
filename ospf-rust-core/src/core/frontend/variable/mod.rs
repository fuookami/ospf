use paste::paste;

pub use item::VariableItem;
pub use range::VariableRange;
pub use variable_type::*;

pub mod item;
pub mod range;
pub mod variable_type;

macro_rules! variable_type_exporter_template {
    ($id:ident, $type:ident) => {
        paste!{
            pub type [<$id Var>] = item::IndependentVariableItem<$type>;
            pub type [<$id Variable>]<const D: usize> = item::VariableItemCombination<$type, D>;
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

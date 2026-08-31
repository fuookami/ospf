//! 模型视图 trait（Kotlin 对齐）
//! Model view traits (Kotlin-aligned)

use std::fmt::Debug;
use crate::model::{BasicModel, MechanismModel};

/// 基础模型视图 / Basic model view
pub trait BasicModelView<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn name(&self) -> &str;
    fn num_tokens(&self) -> usize;
    fn num_constraints(&self) -> usize;
}

/// 含变量语义的模型视图 / Model view with variable semantics
pub trait ModelView<V>: BasicModelView<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn num_variables(&self) -> usize;
}

impl<V> BasicModelView<V> for BasicModel<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn num_tokens(&self) -> usize {
        self.num_tokens()
    }

    fn num_constraints(&self) -> usize {
        self.num_constraints()
    }
}

impl<V> ModelView<V> for BasicModel<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn num_variables(&self) -> usize {
        self.num_tokens()
    }
}

impl<V> BasicModelView<V> for MechanismModel<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn name(&self) -> &str {
        &self.basic.name
    }

    fn num_tokens(&self) -> usize {
        self.basic.num_variables()
    }

    fn num_constraints(&self) -> usize {
        self.basic.num_constraints()
    }
}

impl<V> ModelView<V> for MechanismModel<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn num_variables(&self) -> usize {
        self.basic.num_variables()
    }
}

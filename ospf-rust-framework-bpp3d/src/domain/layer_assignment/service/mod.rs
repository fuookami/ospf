//! 层分配服务 / Layer assignment services
//!
//! RMP/final MILP 赋值模型、约束和目标族。
//! RMP/final MILP assignment models, constraints, and objective families.

pub mod limits;

use ospf_rust_core::model::meta_model::MetaModel;
use ospf_rust_core::variable::{Binary, UContinuous, VariableRange};
use ospf_rust_framework::model::{
    DynamicColumnContext, DynamicModelLifecycle, IndexedLinearExpressionSymbols1,
    IndexedVariableCombination1, IndexedVariableCombination2, Pipeline,
};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;

use super::model::Bpp3dModelComponent;
use crate::domain::item::{BinLayer, BinType, Bpp3dDemandKey, Bpp3dDemandMode};

include!("symbol_builder.rs");
include!("value_adapter.rs");
include!("assignment.rs");
include!("load.rs");
include!("capacity.rs");
include!("layer_aggregation.rs");
include!("iterative_context.rs");
include!("solution_analyzer.rs");
include!("aggregation.rs");
include!("context.rs");
include!("tests.rs");

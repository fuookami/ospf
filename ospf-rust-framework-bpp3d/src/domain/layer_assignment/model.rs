//! BPP3D 建模适配层 / BPP3D modeling adapter
//!
//! 补齐 Kotlin DSL 到 Rust `MetaModel` 的承接层。
//! Bridges Kotlin DSL to Rust `MetaModel` for BPP3D domain modeling.
//!
//! # 核心组件 / Core Components
//!
//! - `VariableArray1`: 一维变量集合 / 1D variable array
//! - `VariableArray2`: 二维变量集合 / 2D variable array
//! - `ExpressionArray1`: 一维线性表达式集合 / 1D linear expression array
//! - `SolutionExtractor`: 结果提取辅助 / Solution extraction helper

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use ospf_rust_core::model::meta_model::MetaModel;
use ospf_rust_core::variable::VariableRange;
use ospf_rust_core::variable::variable_item::{
    BinaryVariableItem, ContinuousVariableItem, UContinuousVariableItem,
};


include!("model/variable_array1.rs");
include!("model/variable_array2.rs");
include!("model/expression_array.rs");
include!("model/solution_extractor.rs");
include!("model/component.rs");
include!("model/tests.rs");

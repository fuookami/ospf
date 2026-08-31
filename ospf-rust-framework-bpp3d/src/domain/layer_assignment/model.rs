//! BPP3D 建模适配层 / BPP3D modeling adapter
//!
//! 补齐 Kotlin DSL 到 Rust `MetaModel` 的承接层。
//! Bridges Kotlin DSL to Rust `MetaModel` for BPP3D domain modeling.
//!
//! # 核心组件 / Core Components
//!
//! - `VariableArray1`: 一维变量集合 / 1D variable array
//! - `VariableArray2`: 二维变量集合 / 2D variable array
//! - `ExpressionArray1`: 一维线性表达式集合（已弃用，保留向后兼容） / 1D linear expression array (deprecated, kept for backward compatibility)
//! - `SolutionExtractor`: 结果提取辅助 / Solution extraction helper
//!
//! # Phase J: 线性表达式符号 / Linear Expression Symbols
//!
//! 中间表达式现在通过 `LinearExpressionSymbol` 注册到模型，而非使用 `ExpressionArray1`。
//! 参见 `build_linear_expression_symbol` 和 `ImpreciseAssignment::build_symbols`。
//!
//! Intermediate expressions are now registered to the model via `LinearExpressionSymbol`,
//! replacing `ExpressionArray1`. See `build_linear_expression_symbol` and
//! `ImpreciseAssignment::build_symbols`.

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

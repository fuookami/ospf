#![allow(dead_code)]

//! 持久化骨架（Kotlin 对齐）
//! Persistence scaffold (Kotlin-aligned)

pub mod backend;
pub mod expression;
pub mod log_record;
pub mod persistence_api_controller;
pub mod query;
pub mod request;
pub mod request_record;
mod response;
mod response_record;
pub mod sql_type;

pub use backend::*;
pub use expression::*;
pub use log_record::*;
pub use persistence_api_controller::*;
pub use query::{
    ColumnRef, JoinCardinality, JoinSpec, JoinType, NullsOrder as QueryNullsOrder, OrderSpec,
    PageSpec, ProjectionSpec, QueryAuditSummary, QueryExecutionErrorCategory, QueryExecutionResult,
    QueryExecutionStats, QuerySource, RelationalQueryFailure, RelationalQueryLimits,
    RelationalQueryPlan, RelationalQueryValidationError, SortDirection as QuerySortDirection,
    contains_exact_column_correlation,
};
pub use request::*;
pub use request_record::*;
pub use response::*;
pub use response_record::*;
pub use sql_type::*;

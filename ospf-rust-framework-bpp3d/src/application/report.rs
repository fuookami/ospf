//! 结构化运行报告 / Structured run report
//!
//! 为 BPP3D solver backend 提供统一、可序列化的运行报告格式。
//! Provides unified, serializable run report format for BPP3D solver backends.
//!
//! 覆盖 fake / no-run / 真实 solver backend，统一 objective、coverage、diagnostics、
//! render、selected layer/bin assignment 字段的报告格式。
//! Covers fake / no-run / real solver backends, unifying objective, coverage,
//! diagnostics, render, selected layer/bin assignment fields.

use std::collections::{BTreeMap, BTreeSet};
#[cfg(feature = "serde")]
use std::path::Path;
use std::time::Duration;

include!("report/status.rs");
include!("report/fixture.rs");
include!("report/suite_summary.rs");
include!("report/comparison.rs");
include!("report/run_report.rs");
include!("report/fixture_filter.rs");
include!("report/tests.rs");

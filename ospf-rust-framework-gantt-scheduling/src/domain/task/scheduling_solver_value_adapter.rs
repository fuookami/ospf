//! 调度求解器值适配器 / Scheduling solver value adapter
//!
//! 桥接泛型数值类型 V 与求解器 f64 边界。
//! Bridges generic numeric type V with solver f64 boundary.

use ospf_rust_core::solver::value::SolveValue;
use ospf_rust_core::solver::value::SolveValueConversionPolicy;

/// 调度求解器值适配器 trait / Scheduling solver value adapter trait
///
/// 提供求解器数值和泛型数值之间的转换能力。
/// Provides conversion between solver numeric values and generic numeric values.
pub trait SchedulingSolverValueAdapter<V: SolveValue>: Send + Sync + std::fmt::Debug + 'static {
    /// 从 f64 转换为 V / Convert from f64 to V
    fn into_value(&self, value: f64) -> V;

    /// 从 V 转换为 f64 / Convert from V to f64
    fn from_value(&self, value: &V) -> f64;

    /// 舍入解值 / Round solution value
    ///
    /// 将 f64 转换为最近的合法求解值。
    /// Converts f64 to nearest legal solver value.
    fn round_solution(&self, value: f64) -> V;

    /// 向下取整为 u64 / Floor to u64
    fn floor_to_u64(&self, value: f64) -> u64;

    /// 向下取整 f64 值 / Floor f64 value
    fn floor_value(&self, value: f64) -> f64;

    /// 零值 / Zero value
    fn zero(&self) -> V;

    /// 一值 / One value
    fn one(&self) -> V;
}

/// f64 适配器 / f64 adapter
///
/// f64 到 f64 的恒等转换。
/// Identity conversion from f64 to f64.
#[derive(Debug, Clone, Copy)]
pub struct F64SolverValueAdapter;

impl SchedulingSolverValueAdapter<f64> for F64SolverValueAdapter {
    fn into_value(&self, value: f64) -> f64 {
        value
    }

    fn from_value(&self, value: &f64) -> f64 {
        *value
    }

    fn round_solution(&self, value: f64) -> f64 {
        value.round()
    }

    fn floor_to_u64(&self, value: f64) -> u64 {
        value.round() as u64
    }

    fn floor_value(&self, value: f64) -> f64 {
        value.floor()
    }

    fn zero(&self) -> f64 {
        0.0
    }

    fn one(&self) -> f64 {
        1.0
    }
}

/// 泛型适配器 / Generic adapter
///
/// 使用 SolveValue trait 进行泛型数值转换。
/// Uses SolveValue trait for generic numeric conversion.
#[derive(Debug)]
pub struct GenericSolverValueAdapter<V: SolveValue> {
    _marker: std::marker::PhantomData<V>,
}

impl<V: SolveValue> GenericSolverValueAdapter<V> {
    /// 创建新的泛型适配器 / Create new generic adapter
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<V: SolveValue> Default for GenericSolverValueAdapter<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: SolveValue> SchedulingSolverValueAdapter<V> for GenericSolverValueAdapter<V> {
    fn into_value(&self, value: f64) -> V {
        V::from_f64_with_policy(value, SolveValueConversionPolicy::AllowRounding)
            .expect("GenericSolverValueAdapter::into_value failed")
    }

    fn from_value(&self, value: &V) -> f64 {
        value
            .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
            .expect("GenericSolverValueAdapter::from_value failed")
    }

    fn round_solution(&self, value: f64) -> V {
        self.into_value(value.round())
    }

    fn floor_to_u64(&self, value: f64) -> u64 {
        value.round() as u64
    }

    fn floor_value(&self, value: f64) -> f64 {
        value.floor()
    }

    fn zero(&self) -> V {
        self.into_value(0.0)
    }

    fn one(&self) -> V {
        self.into_value(1.0)
    }
}

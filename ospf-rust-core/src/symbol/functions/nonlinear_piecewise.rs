//! 非线性函数的分段线性近似 / Piecewise-linear approximations of nonlinear functions

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
};
use super::univariate_linear_piecewise::{Point2, UnivariateLinearPiecewiseFunction};
use crate::error::{ModelError, Result};
use crate::model::LinearConstraint;
use crate::symbol::flatten::{Linear, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::ContinuousVariableItem;
use num_traits::{FromPrimitive, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use std::any::Any;
use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul};
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
enum NonlinearOperation {
    Exp,
    Log,
    Reciprocal,
    Power(f64),
    Logistic { steepness: f64, midpoint: f64 },
}

impl NonlinearOperation {
    fn evaluate(self, x: f64) -> f64 {
        match self {
            Self::Exp => x.exp(),
            Self::Log => x.ln(),
            Self::Reciprocal => 1.0 / x,
            Self::Power(exponent) => x.powf(exponent),
            Self::Logistic {
                steepness,
                midpoint,
            } => {
                let scaled = steepness * (x - midpoint);
                if scaled >= 0.0 {
                    1.0 / (1.0 + (-scaled).exp())
                } else {
                    let exponential = scaled.exp();
                    exponential / (1.0 + exponential)
                }
            }
        }
    }

    fn validate_domain(self, lower: f64, upper: f64) -> Result<()> {
        let valid = match self {
            Self::Log => lower > 0.0,
            Self::Reciprocal => lower > 0.0 || upper < 0.0,
            Self::Power(exponent) => {
                exponent.is_finite()
                    && (lower >= 0.0 || exponent.fract() == 0.0)
                    && (exponent >= 0.0 || upper < 0.0 || lower > 0.0)
            }
            Self::Logistic {
                steepness,
                midpoint,
            } => steepness.is_finite() && steepness > 0.0 && midpoint.is_finite(),
            Self::Exp => true,
        };
        if valid {
            Ok(())
        } else {
            Err(ModelError::InvalidConstraint(format!(
                "invalid finite domain [{lower}, {upper}] for nonlinear piecewise function"
            ))
            .into())
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Exp => "exp",
            Self::Log => "log",
            Self::Reciprocal => "reciprocal",
            Self::Power(_) => "power",
            Self::Logistic { .. } => "logistic_approximation",
        }
    }
}

/// 公共非线性近似配置的内部表示 / Internal representation for nonlinear approximations.
#[derive(Debug, Clone)]
struct NonlinearPiecewise<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    piecewise: UnivariateLinearPiecewiseFunction<V>,
    operation: NonlinearOperation,
    declared_dependency_ids: Vec<u64>,
}

impl<V> NonlinearPiecewise<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive + ToPrimitive,
{
    fn with_segments(
        id: u64,
        name: &str,
        input: Linear<V>,
        lower: V,
        upper: V,
        segments: usize,
        operation: NonlinearOperation,
    ) -> Result<Self> {
        let lower = finite_f64(&lower, "lower bound")?;
        let upper = finite_f64(&upper, "upper bound")?;
        if lower >= upper {
            return Err(invalid("nonlinear approximation requires lower < upper"));
        }
        let width = upper - lower;
        if !width.is_finite() || width <= 0.0 {
            return Err(invalid(
                "nonlinear approximation domain width must be finite and positive",
            ));
        }
        if segments == 0 || segments > 100_000 {
            return Err(invalid(
                "nonlinear approximation segment count must be in 1..=100000",
            ));
        }
        let step = width / segments as f64;
        if !step.is_finite() || step <= 0.0 {
            return Err(invalid(
                "nonlinear approximation segment width must be finite and positive",
            ));
        }
        operation.validate_domain(lower, upper)?;
        let breakpoints = (0..=segments)
            .map(|index| {
                if index == segments {
                    upper
                } else {
                    lower + step * index as f64
                }
            })
            .collect::<Vec<_>>();
        Self::from_f64_breakpoints(id, name, input, breakpoints, operation)
    }

    fn with_breakpoints(
        id: u64,
        name: &str,
        input: Linear<V>,
        breakpoints: Vec<V>,
        operation: NonlinearOperation,
    ) -> Result<Self> {
        let breakpoints = breakpoints
            .iter()
            .map(|value| finite_f64(value, "breakpoint"))
            .collect::<Result<Vec<_>>>()?;
        Self::from_f64_breakpoints(id, name, input, breakpoints, operation)
    }

    fn from_f64_breakpoints(
        id: u64,
        name: &str,
        input: Linear<V>,
        breakpoints: Vec<f64>,
        operation: NonlinearOperation,
    ) -> Result<Self> {
        if !finite_f64(input.constant_term(), "input constant")?.is_finite()
            || input.monomials().iter().any(|monomial| {
                monomial
                    .coefficient()
                    .to_f64()
                    .is_none_or(|value| !value.is_finite())
            })
        {
            return Err(invalid(
                "nonlinear approximation input coefficients must be finite",
            ));
        }
        if breakpoints.len() < 2 {
            return Err(invalid(
                "nonlinear approximation requires at least two breakpoints",
            ));
        }
        if breakpoints.iter().any(|value| !value.is_finite()) {
            return Err(invalid(
                "nonlinear approximation breakpoints must be finite",
            ));
        }
        if breakpoints.len() > 100_001 {
            return Err(invalid(
                "nonlinear approximation supports at most 100000 segments",
            ));
        }
        if breakpoints.windows(2).any(|pair| {
            let width = pair[1] - pair[0];
            !width.is_finite() || width <= 0.0
        }) {
            return Err(invalid(
                "nonlinear approximation breakpoints must have finite positive widths",
            ));
        }
        let lower = breakpoints[0];
        let upper = breakpoints[breakpoints.len() - 1];
        operation.validate_domain(lower, upper)?;
        let points: Vec<Point2<V>> = breakpoints
            .into_iter()
            .map(|x| {
                let y = operation.evaluate(x);
                if !y.is_finite() {
                    return Err(invalid(format!(
                        "{} is not finite at breakpoint {x}",
                        operation.name()
                    )));
                }
                Ok(Point2::new(
                    from_f64(x, "breakpoint")?,
                    from_f64(y, "function value")?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        if points.windows(2).any(|pair| {
            match (
                pair[0].x.to_f64(),
                pair[1].x.to_f64(),
                pair[0].y.to_f64(),
                pair[1].y.to_f64(),
            ) {
                (Some(x0), Some(x1), Some(y0), Some(y1)) => {
                    let width = x1 - x0;
                    let height = y1 - y0;
                    !width.is_finite() || width <= 0.0 || !height.is_finite()
                }
                _ => true,
            }
        }) || points
            .iter()
            .any(|point| point.x.to_f64().is_none_or(|value| !value.is_finite()))
            || points
                .iter()
                .any(|point| point.y.to_f64().is_none_or(|value| !value.is_finite()))
        {
            return Err(invalid(
                "nonlinear breakpoints or values are not representable in the model value type",
            ));
        }
        let piecewise = UnivariateLinearPiecewiseFunction::new(id, name, input, points);
        Ok(Self {
            piecewise,
            operation,
            declared_dependency_ids: Vec::new(),
        })
    }

    fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    fn evaluate(&self, x: &V) -> Option<V> {
        let x = x.to_f64()?;
        interpolate(self.piecewise.points(), x)
    }

    fn original_value(&self, x: &V) -> Option<V> {
        let x = x.to_f64()?;
        if !x.is_finite() {
            return None;
        }
        let points = self.piecewise.points();
        let lower = points.first()?.x.to_f64()?;
        let upper = points.last()?.x.to_f64()?;
        if x < lower || x > upper {
            return None;
        }
        let value = self.operation.evaluate(x);
        value.is_finite().then(|| V::from_f64(value)).flatten()
    }

    fn result_variable(&self) -> &ContinuousVariableItem {
        self.piecewise.result_variable()
    }

    fn points(&self) -> &[Point2<V>] {
        self.piecewise.points()
    }
}

fn invalid(message: impl Into<String>) -> crate::error::CoreError {
    ModelError::InvalidConstraint(message.into()).into()
}

fn finite_f64<V: ToPrimitive>(value: &V, label: &str) -> Result<f64> {
    let value = value.to_f64().ok_or_else(|| {
        ModelError::InvalidConstraint(format!(
            "nonlinear approximation {label} cannot be represented as f64"
        ))
    })?;
    if !value.is_finite() {
        return Err(invalid(format!(
            "nonlinear approximation {label} must be finite"
        )));
    }
    Ok(value)
}

fn from_f64<V: FromPrimitive>(value: f64, label: &str) -> Result<V> {
    V::from_f64(value).ok_or_else(|| {
        ModelError::InvalidConstraint(format!(
            "nonlinear approximation {label} cannot be converted from f64"
        ))
        .into()
    })
}

fn interpolate<V>(points: &[Point2<V>], x: f64) -> Option<V>
where
    V: FromPrimitive + ToPrimitive,
{
    let first = points.first()?.x.to_f64()?;
    let last = points.last()?.x.to_f64()?;
    if !x.is_finite() || x < first || x > last {
        return None;
    }
    for pair in points.windows(2) {
        let x0 = pair[0].x.to_f64()?;
        let x1 = pair[1].x.to_f64()?;
        if x <= x1 {
            let y0 = pair[0].y.to_f64()?;
            let y1 = pair[1].y.to_f64()?;
            return V::from_f64(y0 + (x - x0) * (y1 - y0) / (x1 - x0));
        }
    }
    None
}

macro_rules! nonlinear_builders {
    ($name:ident, $operation:expr) => {
        /// 单变量非线性函数的分段线性近似 / Piecewise-linear approximation of a univariate nonlinear function.
        #[derive(Debug, Clone)]
        pub struct $name<V = f64>
        where
            V: Clone + Debug + Send + Sync + 'static,
        {
            inner: NonlinearPiecewise<V>,
        }

        impl<V> $name<V>
        where
            V: Clone + Debug + Send + Sync + 'static + FromPrimitive + ToPrimitive,
        {
            /// 使用均匀分段构造近似 / Create an approximation with uniform segments.
            pub fn with_segments(
                id: u64,
                name: &str,
                input: Linear<V>,
                lower: V,
                upper: V,
                segments: usize,
            ) -> Result<Self> {
                Ok(Self {
                    inner: NonlinearPiecewise::with_segments(
                        id, name, input, lower, upper, segments, $operation,
                    )?,
                })
            }

            /// 使用用户断点构造近似 / Create an approximation from user breakpoints.
            pub fn with_breakpoints(
                id: u64,
                name: &str,
                input: Linear<V>,
                breakpoints: Vec<V>,
            ) -> Result<Self> {
                Ok(Self {
                    inner: NonlinearPiecewise::with_breakpoints(
                        id,
                        name,
                        input,
                        breakpoints,
                        $operation,
                    )?,
                })
            }

            /// 返回模型中的线性插值值 / Return the piecewise-linear model value.
            pub fn evaluate(&self, x: &V) -> Option<V> {
                self.inner.evaluate(x)
            }

            /// 返回原始数学函数值；不会给出插值误差保证 / Return the original function value; no interpolation-error guarantee is provided.
            pub fn original_value(&self, x: &V) -> Option<V> {
                self.inner.original_value(x)
            }

            /// 返回结果变量 / Return the result variable.
            pub fn result_variable(&self) -> &ContinuousVariableItem {
                self.inner.result_variable()
            }

            /// 返回采样断点 / Return the sampled breakpoints.
            pub fn points(&self) -> &[Point2<V>] {
                self.inner.points()
            }

            /// 设置声明的依赖符号 ID / Set declared dependency symbol IDs.
            pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
                self.inner = self.inner.with_declared_dependencies(dependency_ids);
                self
            }
        }
    };
}

macro_rules! nonlinear_symbol_traits {
    ($name:ident) => {
        impl<V> Display for $name<V>
        where
            V: Clone + Debug + Send + Sync + 'static + FromPrimitive + ToPrimitive,
        {
            fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
                write!(
                    formatter,
                    "{}({})",
                    self.inner.operation.name(),
                    self.inner.piecewise.id().name
                )
            }
        }

        impl<V> DynSymbol for $name<V>
        where
            V: Clone + Debug + Send + Sync + 'static + FromPrimitive + ToPrimitive,
        {
            fn name(&self) -> &str {
                self.inner.piecewise.name()
            }

            fn display_name(&self) -> &str {
                self.inner.piecewise.name()
            }

            fn dyn_id(&self) -> SymbolDynId<'_> {
                SymbolDynId::standalone(self.inner.piecewise.id().id as usize)
            }

            fn as_any(&self) -> &dyn Any {
                self
            }
        }

        impl<V> Symbol for $name<V>
        where
            V: Clone + Debug + Send + Sync + 'static + FromPrimitive + ToPrimitive,
        {
            type Id = IntermediateSymbolId;

            fn id(&self) -> Self::Id {
                self.inner.piecewise.id()
            }
        }

        impl<V> IntermediateSymbol<V> for $name<V>
        where
            V: Clone
                + Debug
                + Send
                + Sync
                + 'static
                + Add<Output = V>
                + Mul<Output = V>
                + Zero
                + ToPrimitive
                + FromPrimitive,
            f64: IntoValue<V>,
        {
            fn category(&self) -> Category {
                Category::Linear
            }

            fn cached(&self) -> bool {
                false
            }

            fn dependencies(&self) -> HashSet<Arc<dyn IntermediateSymbol<V>>> {
                HashSet::new()
            }

            fn declared_dependency_ids(&self) -> Vec<u64> {
                self.inner.declared_dependency_ids.clone()
            }

            fn flush(&self, force: bool) {
                self.inner.piecewise.flush(force)
            }

            fn register_auxiliary_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
                <Self as FunctionSymbol<V>>::register_tokens(self, tokens)
            }

            fn mechanism_constraints(
                &self,
                symbol_to_index: &std::collections::HashMap<usize, usize>,
            ) -> Result<Vec<LinearConstraint<V>>> {
                self.inner.piecewise.mechanism_constraints(symbol_to_index)
            }

            fn evaluate_from_tokens(
                &self,
                token_table: &dyn TokenList<V>,
                zero_if_none: bool,
            ) -> Option<V> {
                <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
            }

            fn prepare(&self, values: &std::collections::HashMap<usize, V>) -> Option<V> {
                values
                    .get(&self.inner.piecewise.result_variable().index())
                    .cloned()
            }

            fn to_raw_string(&self, _unfold: u64) -> String {
                format!(
                    "{}({})",
                    self.inner.operation.name(),
                    self.inner.piecewise.id().name
                )
            }
        }

        impl<V> FunctionSymbol<V> for $name<V>
        where
            V: Clone
                + Debug
                + Send
                + Sync
                + 'static
                + Add<Output = V>
                + Mul<Output = V>
                + Zero
                + ToPrimitive
                + FromPrimitive,
            f64: IntoValue<V>,
        {
            fn register_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
                self.inner.piecewise.register_tokens(tokens)
            }

            fn calculate_value(
                &self,
                token_table: &dyn TokenList<V>,
                zero_if_none: bool,
            ) -> Option<V> {
                self.inner
                    .piecewise
                    .calculate_value(token_table, zero_if_none)
            }
        }

        impl<V> LinearIntermediateSymbol<V> for $name<V>
        where
            V: Clone
                + Debug
                + Send
                + Sync
                + 'static
                + Add<Output = V>
                + Mul<Output = V>
                + Zero
                + ToPrimitive
                + FromPrimitive,
            f64: IntoValue<V>,
        {
            fn to_linear_polynomial(&self) -> Linear<V> {
                self.inner.piecewise.to_linear_polynomial()
            }

            fn to_quadratic_polynomial(&self) -> Quadratic<V> {
                self.inner.piecewise.to_quadratic_polynomial()
            }
        }
    };
}

nonlinear_builders!(ExpFunction, NonlinearOperation::Exp);
nonlinear_builders!(LogFunction, NonlinearOperation::Log);
nonlinear_builders!(ReciprocalFunction, NonlinearOperation::Reciprocal);
nonlinear_symbol_traits!(ExpFunction);
nonlinear_symbol_traits!(LogFunction);
nonlinear_symbol_traits!(ReciprocalFunction);

/// `x^p` 的分段线性近似 / Piecewise-linear approximation of `x^p`.
#[derive(Debug, Clone)]
pub struct PowerFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    inner: NonlinearPiecewise<V>,
}

impl<V> PowerFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive + ToPrimitive,
{
    /// 使用均匀分段和指数构造近似 / Create an approximation with uniform segments and an exponent.
    pub fn with_exponent_segments(
        id: u64,
        name: &str,
        input: Linear<V>,
        exponent: f64,
        lower: V,
        upper: V,
        segments: usize,
    ) -> Result<Self> {
        Ok(Self {
            inner: NonlinearPiecewise::with_segments(
                id,
                name,
                input,
                lower,
                upper,
                segments,
                NonlinearOperation::Power(exponent),
            )?,
        })
    }

    /// 使用用户断点和指数构造近似 / Create an approximation from user breakpoints and an exponent.
    pub fn with_exponent_breakpoints(
        id: u64,
        name: &str,
        input: Linear<V>,
        exponent: f64,
        breakpoints: Vec<V>,
    ) -> Result<Self> {
        Ok(Self {
            inner: NonlinearPiecewise::with_breakpoints(
                id,
                name,
                input,
                breakpoints,
                NonlinearOperation::Power(exponent),
            )?,
        })
    }

    /// 返回模型中的线性插值值 / Return the piecewise-linear model value.
    pub fn evaluate(&self, x: &V) -> Option<V> {
        self.inner.evaluate(x)
    }

    /// 返回原函数值；不会给出插值误差保证 / Return the original function value; no interpolation-error guarantee is provided.
    pub fn original_value(&self, x: &V) -> Option<V> {
        self.inner.original_value(x)
    }

    /// 返回结果变量 / Return the result variable.
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }

    /// 返回采样断点 / Return the sampled breakpoints.
    pub fn points(&self) -> &[Point2<V>] {
        self.inner.points()
    }

    /// 设置声明的依赖符号 ID / Set declared dependency symbol IDs.
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.inner = self.inner.with_declared_dependencies(dependency_ids);
        self
    }
}

nonlinear_symbol_traits!(PowerFunction);

/// 平滑 logistic 的分段线性近似 / Piecewise-linear approximation of a smooth logistic function.
#[derive(Debug, Clone)]
pub struct LogisticApproximationFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    inner: NonlinearPiecewise<V>,
}

impl<V> LogisticApproximationFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive + ToPrimitive,
{
    /// 使用标准 logistic `1 / (1 + exp(-x))` 构造均匀分段近似。
    /// Create uniform segments for the standard logistic `1 / (1 + exp(-x))`.
    pub fn with_segments(
        id: u64,
        name: &str,
        input: Linear<V>,
        lower: V,
        upper: V,
        segments: usize,
    ) -> Result<Self> {
        Self::with_parameters_and_segments(id, name, input, lower, upper, segments, 1.0, 0.0)
    }

    /// 使用标准 logistic 和用户断点构造近似 / Create an approximation from breakpoints for the standard logistic.
    pub fn with_breakpoints(
        id: u64,
        name: &str,
        input: Linear<V>,
        breakpoints: Vec<V>,
    ) -> Result<Self> {
        Self::with_parameters_and_breakpoints(id, name, input, breakpoints, 1.0, 0.0)
    }

    /// 使用可配置陡峭度与中点构造均匀分段近似 / Create uniform segments with configurable steepness and midpoint.
    #[expect(clippy::too_many_arguments, reason = "保留既有公开分段函数构造接口 / Preserve the existing public piecewise-function constructor")]
    pub fn with_parameters_and_segments(
        id: u64,
        name: &str,
        input: Linear<V>,
        lower: V,
        upper: V,
        segments: usize,
        steepness: f64,
        midpoint: f64,
    ) -> Result<Self> {
        Ok(Self {
            inner: NonlinearPiecewise::with_segments(
                id,
                name,
                input,
                lower,
                upper,
                segments,
                NonlinearOperation::Logistic {
                    steepness,
                    midpoint,
                },
            )?,
        })
    }

    /// 使用可配置陡峭度、中点和用户断点构造近似。
    /// Create an approximation from breakpoints with configurable steepness and midpoint.
    pub fn with_parameters_and_breakpoints(
        id: u64,
        name: &str,
        input: Linear<V>,
        breakpoints: Vec<V>,
        steepness: f64,
        midpoint: f64,
    ) -> Result<Self> {
        Ok(Self {
            inner: NonlinearPiecewise::with_breakpoints(
                id,
                name,
                input,
                breakpoints,
                NonlinearOperation::Logistic {
                    steepness,
                    midpoint,
                },
            )?,
        })
    }

    /// 返回模型中的线性插值值 / Return the piecewise-linear model value.
    pub fn evaluate(&self, x: &V) -> Option<V> {
        self.inner.evaluate(x)
    }

    /// 返回原函数值；不会给出插值误差保证 / Return the original function value; no interpolation-error guarantee is provided.
    pub fn original_value(&self, x: &V) -> Option<V> {
        self.inner.original_value(x)
    }

    /// 返回结果变量 / Return the result variable.
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }

    /// 返回采样断点 / Return the sampled breakpoints.
    pub fn points(&self) -> &[Point2<V>] {
        self.inner.points()
    }

    /// 设置声明的依赖符号 ID / Set declared dependency symbol IDs.
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.inner = self.inner.with_declared_dependencies(dependency_ids);
        self
    }
}

nonlinear_symbol_traits!(LogisticApproximationFunction);

/// 平滑 logistic 近似的语义别名 / Semantic alias for the smooth logistic approximation.
pub type SmoothLogisticFunction<V = f64> = LogisticApproximationFunction<V>;

#[cfg(test)]
mod tests {
    use super::{
        ExpFunction, LogFunction, LogisticApproximationFunction, PowerFunction, ReciprocalFunction,
    };
    use crate::model::MetaModel;
    use crate::symbol::flatten::{Linear, LinearMonomial};
    use crate::symbol::{Category, IntermediateSymbol};
    use crate::variable::{ContinuousVariableItem, VariableId, VariableRange, new_group_id};
    use std::sync::Arc;

    #[test]
    fn exp_interpolation_and_original_evaluation_are_separate() {
        let function = ExpFunction::with_segments(1, "exp", Linear::constant(0.0), 0.0, 5.0, 2)
            .expect("valid exponential domain");
        assert_eq!(function.points().len(), 3);
        assert_eq!(
            function
                .points()
                .iter()
                .map(|point| point.x)
                .collect::<Vec<_>>(),
            vec![0.0, 2.5, 5.0]
        );
        assert!((function.evaluate(&2.5).expect("sampled node") - 2.5_f64.exp()).abs() < 1e-10);
        assert!(
            (function.evaluate(&1.25).expect("interpolation") - (1.0 + 2.5_f64.exp()) / 2.0).abs()
                < 1e-10
        );
        let original = function.original_value(&1.25).expect("original value");
        assert!((original - 1.25_f64.exp()).abs() < 1e-10);
        assert!((original - function.evaluate(&1.25).expect("interpolation")).abs() > 0.1);
        assert_eq!(function.evaluate(&-0.1), None);
        assert_eq!(function.original_value(&5.1), None);
    }

    #[test]
    fn nonlinear_domains_and_breakpoints_are_validated() {
        assert!(LogFunction::with_segments(2, "log", Linear::constant(0.0), 0.0, 2.0, 4).is_err());
        assert!(
            ReciprocalFunction::with_segments(3, "reciprocal", Linear::constant(0.0), -1.0, 1.0, 4)
                .is_err()
        );
        assert!(
            ExpFunction::with_breakpoints(4, "bad", Linear::constant(0.0), vec![0.0, 1.0, 1.0])
                .is_err()
        );
        assert!(
            PowerFunction::with_exponent_segments(
                5,
                "power",
                Linear::constant(0.0),
                0.5,
                -1.0,
                1.0,
                4
            )
            .is_err()
        );
        assert!(
            ExpFunction::with_segments(
                13,
                "exp-subnormal-step",
                Linear::constant(0.0),
                0.0,
                f64::from_bits(1),
                2,
            )
            .is_err()
        );
        assert!(
            PowerFunction::with_exponent_breakpoints(
                12,
                "power-one-wide-segment",
                Linear::constant(0.0),
                1.0,
                vec![-1e308, 1e308],
            )
            .is_err()
        );
        assert!(
            PowerFunction::with_exponent_segments(
                10,
                "power-zero",
                Linear::constant(0.0),
                0.0,
                -1e308,
                1e308,
                1,
            )
            .is_err()
        );
        assert!(
            PowerFunction::with_exponent_segments(
                6,
                "power",
                Linear::constant(0.0),
                -1.0,
                -1.0,
                1.0,
                4
            )
            .is_err()
        );
    }

    #[test]
    fn reciprocal_near_zero_and_logistic_parameters_are_supported_when_finite() {
        let reciprocal =
            ReciprocalFunction::with_segments(7, "reciprocal", Linear::constant(0.0), 0.1, 1.0, 4)
                .expect("reciprocal interval excludes zero");
        assert_eq!(reciprocal.original_value(&0.1), Some(10.0));
        let logistic = LogisticApproximationFunction::with_parameters_and_segments(
            8,
            "logistic",
            Linear::constant(0.0),
            -4.0,
            4.0,
            16,
            2.0,
            0.5,
        )
        .expect("valid logistic parameters");
        let midpoint_value: f64 = logistic.original_value(&0.5).expect("midpoint");
        assert!((midpoint_value - 0.5_f64).abs() < 1e-12);
        let power_zero = PowerFunction::with_exponent_breakpoints(
            11,
            "power-zero",
            Linear::constant(0.0),
            0.0,
            vec![0.0, 1.0],
        )
        .expect("zero exponent on finite domain");
        assert_eq!(power_zero.original_value(&f64::NAN), None);
    }

    #[test]
    fn nonlinear_approximation_registers_in_the_linear_mechanism() {
        let mut model = MetaModel::<f64>::new("exp-linear-mechanism");
        let input = ContinuousVariableItem::with_range(
            VariableId::new(new_group_id(), 0),
            "input",
            VariableRange::bounded(0.0, 2.0),
        );
        let input_index = model.register_variable(input).expect("register input");
        let function = Arc::new(
            ExpFunction::with_segments(
                9,
                "exp_approx",
                Linear::new(vec![LinearMonomial::new(1.0, input_index)], 0.0),
                0.0,
                2.0,
                2,
            )
            .expect("valid exponential approximation"),
        );
        assert_eq!(function.category(), Category::Linear);
        model
            .add_symbols(vec![function as Arc<dyn IntermediateSymbol<f64>>])
            .expect("register approximation");

        let mechanism = model.try_to_mechanism_model().expect("build mechanism");
        assert!(!mechanism.constraints().is_empty());
        assert!(mechanism.quadratic_constraints().is_empty());
    }
}

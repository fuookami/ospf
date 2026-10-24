//! 分段计费和固定启用费组合 / Tiered tariffs and fixed-charge compositions

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
};
use super::univariate_linear_piecewise::{Point2, UnivariateLinearPiecewiseFunction};
use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{
    BinaryVariableItem, ContinuousVariableItem, VariableId, VariableRange, new_group_id,
};
use num_traits::{FromPrimitive, One, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul, Neg};
use std::sync::Arc;

fn invalid(message: impl Into<String>) -> crate::error::CoreError {
    ModelError::InvalidConstraint(message.into()).into()
}

fn to_f64<V: ToPrimitive>(value: &V, field: &str) -> Result<f64> {
    let value = value.to_f64().ok_or_else(|| {
        ModelError::InvalidConstraint(format!("tariff {field} cannot be represented as f64"))
    })?;
    if !value.is_finite() {
        return Err(invalid(format!("tariff {field} must be finite")));
    }
    Ok(value)
}

fn from_f64<V: FromPrimitive>(value: f64, field: &str) -> Result<V> {
    V::from_f64(value).ok_or_else(|| {
        ModelError::InvalidConstraint(format!("tariff {field} cannot be converted from f64")).into()
    })
}

fn evaluate_linear<V>(
    polynomial: &Linear<V>,
    token_table: &dyn TokenList<V>,
    zero_if_none: bool,
) -> Option<V>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V> + Zero,
{
    let mut value = polynomial.constant_term().clone();
    for monomial in polynomial.monomials() {
        let term_value = match token_table
            .find_by_index(monomial.var_index())
            .and_then(|token| token.get_result())
        {
            Some(value) => value,
            None if zero_if_none => V::zero(),
            None => return None,
        };
        value = value + monomial.coefficient().clone() * term_value;
    }
    Some(value)
}

/// 提供分段计费函数工厂 / Factories for piecewise tariff functions.
pub struct TariffFunctions;

impl TariffFunctions {
    /// 创建边际费率逐段累加的增量计费函数。
    ///
    /// 断点至少含两个值且首点必须为 0；费率数量必须比断点少一个。输入范围为首末断点，
    /// 首点费用为 0，每段按自己的边际费率累加。
    /// Create an incremental tariff that accumulates each tier's marginal rate. The first
    /// breakpoint must be zero; the rate count is one less than the breakpoint count.
    pub fn incremental<V>(
        id: u64,
        name: &str,
        input: Linear<V>,
        breakpoints: Vec<V>,
        marginal_rates: Vec<V>,
    ) -> Result<UnivariateLinearPiecewiseFunction<V>>
    where
        V: Clone + Debug + Send + Sync + 'static + FromPrimitive + ToPrimitive,
    {
        if breakpoints.len() < 2 || marginal_rates.len() + 1 != breakpoints.len() {
            return Err(invalid(
                "incremental tariff requires at least two breakpoints and one rate per tier",
            ));
        }
        let points_x = breakpoints
            .iter()
            .map(|value| to_f64(value, "breakpoint"))
            .collect::<Result<Vec<_>>>()?;
        if input.monomials().iter().any(|monomial| {
            monomial
                .coefficient()
                .to_f64()
                .is_none_or(|value| !value.is_finite())
        }) {
            return Err(invalid(
                "incremental tariff input must contain finite values",
            ));
        }
        to_f64(input.constant_term(), "input constant")?;
        if points_x[0] != 0.0 {
            return Err(invalid("incremental tariff must start at breakpoint zero"));
        }
        if points_x.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(invalid(
                "incremental tariff breakpoints must be strictly increasing",
            ));
        }
        let rates = marginal_rates
            .iter()
            .map(|value| to_f64(value, "marginal rate"))
            .collect::<Result<Vec<_>>>()?;
        if rates.iter().any(|rate| *rate < 0.0) {
            return Err(invalid(
                "incremental tariff marginal rates must be non-negative",
            ));
        }
        let mut cost = 0.0;
        let mut points = Vec::with_capacity(points_x.len());
        points.push(Point2::new(
            breakpoints[0].clone(),
            from_f64(0.0, "initial cost")?,
        ));
        for index in 0..rates.len() {
            cost += (points_x[index + 1] - points_x[index]) * rates[index];
            if !cost.is_finite() {
                return Err(invalid("incremental tariff cumulative cost is not finite"));
            }
            points.push(Point2::new(
                breakpoints[index + 1].clone(),
                from_f64(cost, "cumulative cost")?,
            ));
        }
        if points.windows(2).any(|pair| {
            pair[0]
                .x
                .to_f64()
                .is_none_or(|left| pair[1].x.to_f64().is_none_or(|right| left >= right))
        }) || points
            .iter()
            .any(|point| point.y.to_f64().is_none_or(|value| !value.is_finite()))
        {
            return Err(invalid(
                "incremental tariff points are not representable in the model value type",
            ));
        }
        Ok(UnivariateLinearPiecewiseFunction::new(
            id, name, input, points,
        ))
    }

    /// 创建具有分段跳变的全量折扣费用函数 / Create an all-units discount tariff with tier jumps.
    pub fn all_units_discount<V>(
        id: u64,
        name: &str,
        input: Linear<V>,
        breakpoints: Vec<V>,
        rates: Vec<V>,
        boundary_gap: V,
    ) -> Result<AllUnitsDiscountFunction<V>>
    where
        V: Clone + Debug + Send + Sync + 'static + FromPrimitive + ToPrimitive,
    {
        AllUnitsDiscountFunction::with_breakpoints(
            id,
            name,
            input,
            breakpoints,
            rates,
            boundary_gap,
        )
    }
}

/// 全量折扣费率档 / A rate tier in an all-units discount tariff.
#[derive(Debug, Clone)]
pub struct TariffTier<V> {
    /// 档位允许的最小数量 / Inclusive lower quantity bound.
    pub lower: V,
    /// 档位允许的最大数量 / Inclusive upper quantity bound.
    pub upper: V,
    /// 对该档全部数量应用的单位费率 / Unit rate applied to the entire quantity in this tier.
    pub rate: V,
}

/// 全量折扣费用符号，档位之间的 gap 不可行，断点归入右侧档。
/// Exact all-units tariff symbol. Gaps between tiers are infeasible and breakpoints belong to the
/// tier on their right.
#[derive(Debug, Clone)]
pub struct AllUnitsDiscountFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Linear<V>,
    tiers: Vec<TariffTier<V>>,
    quantity_vars: Vec<ContinuousVariableItem>,
    selector_vars: Vec<BinaryVariableItem>,
    result_var: ContinuousVariableItem,
    declared_dependency_ids: Vec<u64>,
}

impl<V> AllUnitsDiscountFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive + ToPrimitive,
{
    /// 按断点、费率和严格正 gap 构造全量折扣费用函数。
    ///
    /// `rates.len() + 1 == breakpoints.len()`，断点首值必须为 0，末值是数量上界。档位 `i`
    /// 对应 `[breakpoints[i], breakpoints[i + 1] - boundary_gap]`；最后一档闭合至末断点。
    /// The boundary value is assigned to the right-hand tier; each excluded gap makes its interior
    /// infeasible. Strict positive gaps are required for a continuous-input MILP formulation.
    pub fn with_breakpoints(
        id: u64,
        name: &str,
        input: Linear<V>,
        breakpoints: Vec<V>,
        rates: Vec<V>,
        boundary_gap: V,
    ) -> Result<Self> {
        if breakpoints.len() < 2 || rates.len() + 1 != breakpoints.len() {
            return Err(invalid(
                "all-units tariff requires at least two breakpoints and one rate per tier",
            ));
        }
        let values = breakpoints
            .iter()
            .map(|value| to_f64(value, "breakpoint"))
            .collect::<Result<Vec<_>>>()?;
        if values[0] != 0.0 {
            return Err(invalid("all-units tariff must start at quantity zero"));
        }
        if values.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(invalid(
                "all-units tariff breakpoints must be strictly increasing",
            ));
        }
        let gap = to_f64(&boundary_gap, "boundary gap")?;
        if gap <= 0.0 {
            return Err(invalid(
                "all-units tariff boundary gap must be strictly positive",
            ));
        }
        let rate_values = rates
            .iter()
            .map(|rate| to_f64(rate, "unit rate"))
            .collect::<Result<Vec<_>>>()?;
        if rate_values.iter().any(|rate| *rate < 0.0)
            || rate_values.windows(2).any(|pair| pair[0] < pair[1])
        {
            return Err(invalid(
                "all-units tariff rates must be non-negative and non-increasing",
            ));
        }
        let mut tiers = Vec::with_capacity(rates.len());
        for index in 0..rates.len() {
            let lower = values[index];
            let upper = if index + 1 == rates.len() {
                values[index + 1]
            } else {
                values[index + 1] - gap
            };
            if !upper.is_finite() || lower > upper {
                return Err(invalid(format!(
                    "all-units tariff gap leaves tier {index} empty"
                )));
            }
            tiers.push(TariffTier {
                lower: breakpoints[index].clone(),
                upper: from_f64(upper, "tier upper bound")?,
                rate: rates[index].clone(),
            });
        }
        Self::with_tiers(id, name, input, tiers)
    }

    /// 使用明確且互不重疊的閉合檔位構造全量折扣函數 / Create a tariff from explicit disjoint closed tiers.
    pub fn with_tiers(
        id: u64,
        name: &str,
        input: Linear<V>,
        tiers: Vec<TariffTier<V>>,
    ) -> Result<Self> {
        if tiers.is_empty() {
            return Err(invalid("all-units tariff requires at least one tier"));
        }
        let tier_bounds = tiers
            .iter()
            .map(|tier| {
                Ok((
                    to_f64(&tier.lower, "tier lower bound")?,
                    to_f64(&tier.upper, "tier upper bound")?,
                    to_f64(&tier.rate, "tier rate")?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        if tier_bounds[0].0 != 0.0
            || tier_bounds
                .iter()
                .any(|(lower, upper, rate)| lower < &0.0 || lower > upper || rate < &0.0)
            || tier_bounds.windows(2).any(|pair| pair[0].1 >= pair[1].0)
            || tier_bounds.windows(2).any(|pair| pair[0].2 < pair[1].2)
        {
            return Err(invalid(
                "all-units tariff tiers must start at zero, be ordered and disjoint, and have non-increasing non-negative rates",
            ));
        }
        let input_constant = to_f64(input.constant_term(), "input constant")?;
        if !input
            .monomials()
            .iter()
            .all(|monomial| monomial.coefficient().to_f64().is_some_and(f64::is_finite))
            || !input_constant.is_finite()
        {
            return Err(invalid("all-units tariff input must contain finite values"));
        }
        let group_id = new_group_id();
        let mut result_lower = f64::INFINITY;
        let mut result_upper = f64::NEG_INFINITY;
        for (lower, upper, rate) in &tier_bounds {
            result_lower = result_lower.min(lower * rate);
            result_upper = result_upper.max(upper * rate);
        }
        if !result_lower.is_finite() || !result_upper.is_finite() {
            return Err(invalid("all-units tariff cost range is not finite"));
        }
        let result_var = ContinuousVariableItem::with_range(
            VariableId::new(group_id, 0),
            &format!("{name}_all_units"),
            VariableRange::bounded(result_lower, result_upper),
        );
        let quantity_vars = tier_bounds
            .iter()
            .enumerate()
            .map(|(index, (_, upper, _))| {
                ContinuousVariableItem::with_range(
                    VariableId::new(group_id, index + 1),
                    &format!("{name}_all_units_q{index}"),
                    VariableRange::bounded(0.0, upper - tier_bounds[index].0),
                )
            })
            .collect();
        let selector_vars = (0..tiers.len())
            .map(|index| {
                BinaryVariableItem::create(
                    VariableId::new(group_id, tiers.len() + index + 1),
                    &format!("{name}_all_units_t{index}"),
                )
            })
            .collect();
        Ok(Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            tiers,
            quantity_vars,
            selector_vars,
            result_var,
            declared_dependency_ids: Vec::new(),
        })
    }

    /// 設置聲明的依賴 ID / Set declared dependency IDs.
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 返回結果變量 / Return the result variable.
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    /// 返回檔位選擇變量 / Return the tier selector variables.
    pub fn selector_variables(&self) -> &[BinaryVariableItem] {
        &self.selector_vars
    }

    /// 返回檔位數量變量 / Return the tier quantity variables.
    pub fn tier_quantity_variables(&self) -> &[ContinuousVariableItem] {
        &self.quantity_vars
    }

    /// 返回指定數量的精確檔位費用；gap 內輸入不允許 / Return the exact tier cost; inputs in gaps are unavailable.
    pub fn evaluate(&self, quantity: &V) -> Option<V> {
        let quantity = quantity.to_f64()?;
        for tier in &self.tiers {
            let lower = tier.lower.to_f64()?;
            let upper = tier.upper.to_f64()?;
            if quantity >= lower && quantity <= upper {
                return V::from_f64(quantity * tier.rate.to_f64()?);
            }
        }
        None
    }
}

impl<V> Display for AllUnitsDiscountFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "all_units_discount({})", self.id.name)
    }
}

impl<V> DynSymbol for AllUnitsDiscountFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn name(&self) -> &str {
        &self.id.name
    }

    fn display_name(&self) -> &str {
        &self.id.name
    }

    fn dyn_id(&self) -> SymbolDynId<'_> {
        SymbolDynId::standalone(self.id.id as usize)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<V> Symbol for AllUnitsDiscountFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for AllUnitsDiscountFunction<V>
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
        self.declared_dependency_ids.clone()
    }

    fn flush(&self, _force: bool) {}

    fn register_auxiliary_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
        <Self as FunctionSymbol<V>>::register_tokens(self, tokens)
    }

    fn mechanism_constraints(
        &self,
        symbol_to_index: &std::collections::HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered("all-units result variable".to_owned())
            })?;
        let quantity_indices = self
            .quantity_vars
            .iter()
            .map(|variable| {
                symbol_to_index
                    .get(&(variable.id().unique_id() as usize))
                    .copied()
                    .ok_or_else(|| {
                        ModelError::SymbolNotRegistered("all-units quantity variable".to_owned())
                    })
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let selector_indices = self
            .selector_vars
            .iter()
            .map(|variable| {
                symbol_to_index
                    .get(&(variable.id().unique_id() as usize))
                    .copied()
                    .ok_or_else(|| {
                        ModelError::SymbolNotRegistered("all-units selector variable".to_owned())
                    })
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut rows = Vec::with_capacity(3 + self.tiers.len());
        let mut selector_sum = Vec::with_capacity(selector_indices.len());
        for index in &selector_indices {
            selector_sum.push(LinearMonomial::new(
                from_f64(1.0, "selector coefficient")?,
                *index,
            ));
        }
        rows.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(selector_sum, from_f64(0.0, "selector constant")?),
                ConstraintRelation::Equal,
                from_f64(1.0, "selector rhs")?,
            ),
            &format!("{}_all_units_selector_sum", self.id.name),
            Arc::new(self.clone()),
        ));

        let mut input_monomials = self
            .input
            .monomials()
            .iter()
            .map(|monomial| {
                Ok(LinearMonomial::new(
                    monomial.coefficient().clone(),
                    monomial.var_index(),
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        for (tier, (selector_index, quantity_index)) in self
            .tiers
            .iter()
            .zip(selector_indices.iter().zip(quantity_indices.iter()))
        {
            input_monomials.push(LinearMonomial::new(
                from_f64(
                    -to_f64(&tier.lower, "tier lower bound")?,
                    "tier lower coefficient",
                )?,
                *selector_index,
            ));
            input_monomials.push(LinearMonomial::new(
                from_f64(-1.0, "tier quantity coefficient")?,
                *quantity_index,
            ));
        }
        rows.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(input_monomials, self.input.constant_term().clone()),
                ConstraintRelation::Equal,
                from_f64(0.0, "input rhs")?,
            ),
            &format!("{}_all_units_input", self.id.name),
            Arc::new(self.clone()),
        ));

        let mut result_monomials = vec![LinearMonomial::new(
            from_f64(1.0, "result coefficient")?,
            result_index,
        )];
        for ((tier, selector_index), quantity_index) in self
            .tiers
            .iter()
            .zip(selector_indices.iter())
            .zip(quantity_indices.iter())
        {
            let rate = to_f64(&tier.rate, "tier rate")?;
            let lower = to_f64(&tier.lower, "tier lower bound")?;
            result_monomials.push(LinearMonomial::new(
                from_f64(-rate * lower, "tier cost intercept")?,
                *selector_index,
            ));
            result_monomials.push(LinearMonomial::new(
                from_f64(-rate, "tier marginal cost")?,
                *quantity_index,
            ));
        }
        rows.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(result_monomials, from_f64(0.0, "result constant")?),
                ConstraintRelation::Equal,
                from_f64(0.0, "result rhs")?,
            ),
            &format!("{}_all_units_result", self.id.name),
            Arc::new(self.clone()),
        ));

        for (index, (tier, (selector_index, quantity_index))) in self
            .tiers
            .iter()
            .zip(selector_indices.iter().zip(quantity_indices.iter()))
            .enumerate()
        {
            let width =
                to_f64(&tier.upper, "tier upper bound")? - to_f64(&tier.lower, "tier lower bound")?;
            rows.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![
                            LinearMonomial::new(
                                from_f64(1.0, "tier quantity coefficient")?,
                                *quantity_index,
                            ),
                            LinearMonomial::new(
                                from_f64(-width, "tier selector width")?,
                                *selector_index,
                            ),
                        ],
                        from_f64(0.0, "tier quantity constant")?,
                    ),
                    ConstraintRelation::LessEqual,
                    from_f64(0.0, "tier quantity rhs")?,
                ),
                &format!("{}_all_units_tier_{index}", self.id.name),
                Arc::new(self.clone()),
            ));
        }
        Ok(rows)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &std::collections::HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_var.index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("all_units_discount({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for AllUnitsDiscountFunction<V>
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
        tokens.push(Token::from_generic(
            self.result_var.clone(),
            self.result_var.index(),
        ));
        for variable in &self.quantity_vars {
            tokens.push(Token::from_generic(variable.clone(), variable.index()));
        }
        for variable in &self.selector_vars {
            tokens.push(Token::from_generic(variable.clone(), variable.index()));
        }
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let quantity = evaluate_linear(&self.input, token_table, zero_if_none)?;
        self.evaluate(&quantity)
    }
}

impl<V> LinearIntermediateSymbol<V> for AllUnitsDiscountFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + One
        + ToPrimitive
        + FromPrimitive,
    f64: IntoValue<V>,
{
    fn to_linear_polynomial(&self) -> Linear<V> {
        Linear::new(
            vec![LinearMonomial::new(V::one(), self.result_var.index())],
            V::zero(),
        )
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

/// 固定启用费与有界产量的线性组合 / Linear fixed-charge composition with bounded activity.
#[derive(Debug, Clone)]
pub struct FixedChargeFunction<V> {
    activation: BinaryVariableItem,
    quantity: Linear<V>,
    unit_rate: V,
    fixed_fee: V,
    upper_bound: V,
    minimum_active_quantity: Option<V>,
}

impl<V> FixedChargeFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + FromPrimitive
        + ToPrimitive
        + Mul<Output = V>
        + Neg<Output = V>
        + Zero,
{
    /// 创建固定启用费组合。默认激活变量可以在零产量时为 1。
    /// Create a fixed-charge composition. By default the activation may be 1 at zero quantity.
    pub fn new(
        activation: BinaryVariableItem,
        quantity: Linear<V>,
        unit_rate: V,
        fixed_fee: V,
        upper_bound: V,
    ) -> Result<Self> {
        let unit_rate_f64 = to_f64(&unit_rate, "unit rate")?;
        let fixed_fee_f64 = to_f64(&fixed_fee, "fixed fee")?;
        let upper_f64 = to_f64(&upper_bound, "activity upper bound")?;
        if unit_rate_f64 < 0.0 || fixed_fee_f64 < 0.0 || upper_f64 <= 0.0 {
            return Err(invalid(
                "fixed-charge rate and fee must be non-negative and activity upper bound must be positive",
            ));
        }
        to_f64(quantity.constant_term(), "quantity constant")?;
        if quantity.monomials().iter().any(|monomial| {
            monomial
                .coefficient()
                .to_f64()
                .is_none_or(|value| !value.is_finite())
        }) {
            return Err(invalid("fixed-charge quantity must contain finite values"));
        }
        Ok(Self {
            activation,
            quantity,
            unit_rate,
            fixed_fee,
            upper_bound,
            minimum_active_quantity: None,
        })
    }

    /// 设置激活时的最小产量；值必须严格为正 / Require a strictly positive minimum quantity when activated.
    pub fn with_minimum_active_quantity(mut self, minimum: V) -> Result<Self> {
        let minimum_f64 = to_f64(&minimum, "minimum active quantity")?;
        let upper_f64 = to_f64(&self.upper_bound, "activity upper bound")?;
        if minimum_f64 <= 0.0 || minimum_f64 > upper_f64 {
            return Err(invalid(
                "minimum active quantity must be positive and no greater than the upper bound",
            ));
        }
        self.minimum_active_quantity = Some(minimum);
        Ok(self)
    }

    fn activation_solver_index(&self, symbol_to_index: &HashMap<usize, usize>) -> Result<usize> {
        let unique_id = self.activation.id().unique_id() as usize;
        symbol_to_index.get(&unique_id).copied().ok_or_else(|| {
            ModelError::SymbolNotRegistered(format!(
                "fixed-charge activation variable id {unique_id}"
            ))
            .into()
        })
    }

    /// 返回固定费加单位费率乘数量的线性成本表达式。
    /// Return `fixed_fee * activation + unit_rate * quantity`, resolving activation by symbol ID.
    pub fn cost_expression(&self, symbol_to_index: &HashMap<usize, usize>) -> Result<Linear<V>> {
        let activation_index = self.activation_solver_index(symbol_to_index)?;
        let mut monomials = self
            .quantity
            .monomials()
            .iter()
            .map(|monomial| {
                LinearMonomial::new(
                    monomial.coefficient().clone() * self.unit_rate.clone(),
                    monomial.var_index(),
                )
            })
            .collect::<Vec<_>>();
        monomials.push(LinearMonomial::new(
            self.fixed_fee.clone(),
            activation_index,
        ));
        Ok(Linear::new(
            monomials,
            self.quantity.constant_term().clone() * self.unit_rate.clone(),
        ))
    }

    /// 返回数量与激活变量之间的线性联动约束 / Return linear linking rows between activity and activation.
    pub fn linking_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearInequality<V>>> {
        let activation = self.activation_solver_index(symbol_to_index)?;
        let mut upper_monomials = self
            .quantity
            .monomials()
            .iter()
            .map(|monomial| {
                LinearMonomial::new(monomial.coefficient().clone(), monomial.var_index())
            })
            .collect::<Vec<_>>();
        upper_monomials.push(LinearMonomial::new(-self.upper_bound.clone(), activation));
        let lower_bound = self
            .minimum_active_quantity
            .as_ref()
            .cloned()
            .unwrap_or_else(V::zero);
        let lower_monomials = self
            .quantity
            .monomials()
            .iter()
            .map(|monomial| {
                LinearMonomial::new(-monomial.coefficient().clone(), monomial.var_index())
            })
            .chain(std::iter::once(LinearMonomial::new(
                lower_bound,
                activation,
            )))
            .collect::<Vec<_>>();
        Ok(vec![
            LinearInequality::new(
                Linear::new(upper_monomials, self.quantity.constant_term().clone()),
                ConstraintRelation::LessEqual,
                from_f64(0.0, "upper linking rhs")?,
            ),
            LinearInequality::new(
                Linear::new(lower_monomials, -self.quantity.constant_term().clone()),
                ConstraintRelation::LessEqual,
                from_f64(0.0, "lower linking rhs")?,
            ),
        ])
    }

    /// 默认情况下允许激活但零产量 / Whether activation at zero quantity is allowed by this formulation.
    pub fn allows_zero_quantity_activation(&self) -> bool {
        self.minimum_active_quantity.is_none()
    }
}

/// 与旧 tariff 类型名兼容 / Compatibility alias for the all-units tariff.
pub type AllUnitsDiscountTariff<V = f64> = AllUnitsDiscountFunction<V>;

#[cfg(test)]
mod tests {
    use super::{AllUnitsDiscountFunction, FixedChargeFunction, TariffFunctions};
    use crate::model::{ConstraintRelation, LinearInequality, MetaModel};
    use crate::symbol::flatten::{Linear, LinearMonomial};
    use crate::symbol::{Category, IntermediateSymbol};
    use crate::variable::{
        BinaryVariableItem, ContinuousVariableItem, VariableId, VariableRange, new_group_id,
    };
    use std::collections::HashMap;
    use std::sync::Arc;

    #[test]
    fn incremental_tariff_accumulates_marginal_tiers() {
        let tariff = TariffFunctions::incremental(
            1,
            "incremental",
            Linear::constant(0.0),
            vec![0.0, 10.0, 20.0],
            vec![2.0, 3.0],
        )
        .expect("valid incremental tariff");
        assert_eq!(tariff.points().len(), 3);
        assert_eq!(tariff.points()[2].y, 50.0);
        assert!(
            TariffFunctions::incremental(
                2,
                "invalid",
                Linear::constant(0.0),
                vec![0.0, 10.0],
                vec![],
            )
            .is_err()
        );
    }

    #[test]
    fn all_units_tariff_assigns_breakpoints_to_right_tier_and_rejects_gap() {
        let tariff = AllUnitsDiscountFunction::with_breakpoints(
            3,
            "all-units",
            Linear::constant(0.0),
            vec![0.0, 10.0, 20.0],
            vec![5.0, 4.0],
            1.0,
        )
        .expect("valid discount tariff");
        assert_eq!(tariff.evaluate(&9.0), Some(45.0));
        assert_eq!(tariff.evaluate(&10.0), Some(40.0));
        assert_eq!(tariff.evaluate(&9.5), None);
        assert_eq!(tariff.evaluate(&20.0), Some(80.0));
        assert!(
            AllUnitsDiscountFunction::with_breakpoints(
                4,
                "zero-gap",
                Linear::constant(0.0),
                vec![0.0, 10.0],
                vec![5.0],
                0.0,
            )
            .is_err()
        );
    }

    #[test]
    fn fixed_charge_exposes_activation_link_and_zero_output_policy() {
        let activation = BinaryVariableItem::create(VariableId::new(new_group_id(), 0), "active");
        let activation_id = activation.id().unique_id() as usize;
        let fixed = FixedChargeFunction::new(activation, Linear::constant(0.0), 2.0, 10.0, 100.0)
            .expect("valid fixed charge");
        let symbol_to_index = HashMap::from([(activation_id, 0)]);
        assert!(fixed.allows_zero_quantity_activation());
        assert_eq!(
            fixed
                .linking_constraints(&symbol_to_index)
                .expect("rows")
                .len(),
            2
        );
        assert!(fixed.clone().with_minimum_active_quantity(1.0).is_ok());
        assert!(fixed.with_minimum_active_quantity(101.0).is_err());
    }

    #[test]
    fn fixed_charge_resolves_activation_to_registered_solver_column() {
        let activation = BinaryVariableItem::create(VariableId::new(new_group_id(), 0), "active");
        let activation_id = activation.id().unique_id() as usize;
        let fixed = FixedChargeFunction::new(
            activation,
            Linear::new(vec![LinearMonomial::new(1.0, 2)], 0.0),
            2.0,
            10.0,
            100.0,
        )
        .expect("valid fixed charge")
        .with_minimum_active_quantity(1.0)
        .expect("valid minimum activity");
        let symbol_to_index = HashMap::from([(activation_id, 3)]);
        let values = [7.0, 0.0, 5.0, 1.0];

        let cost = fixed
            .cost_expression(&symbol_to_index)
            .expect("cost expression");
        let cost_value = *cost.constant_term()
            + cost
                .monomials()
                .iter()
                .map(|monomial| monomial.coefficient() * values[monomial.var_index()])
                .sum::<f64>();
        assert_eq!(cost_value, 20.0);

        let rows = fixed
            .linking_constraints(&symbol_to_index)
            .expect("linking rows");
        assert_eq!(rows.len(), 2);
        for row in rows {
            let lhs = *row.polynomial.constant_term()
                + row
                    .polynomial
                    .monomials()
                    .iter()
                    .map(|monomial| monomial.coefficient() * values[monomial.var_index()])
                    .sum::<f64>();
            assert!(lhs <= row.rhs + 1e-9);
        }

        assert!(fixed.cost_expression(&HashMap::new()).is_err());
        assert!(fixed.linking_constraints(&HashMap::new()).is_err());
    }

    #[test]
    fn all_units_highest_tier_is_feasible_with_lower_tier_inactive() {
        let mut model = MetaModel::<f64>::new("all-units-high-tier");
        let input = ContinuousVariableItem::with_range(
            VariableId::new(new_group_id(), 0),
            "quantity",
            VariableRange::bounded(0.0, 20.0),
        );
        let input_id = input.id();
        let input_index = model.register_variable(input).expect("register quantity");
        model
            .add_inequality(
                LinearInequality::new(
                    Linear::new(vec![LinearMonomial::new(1.0, input_index)], 0.0),
                    ConstraintRelation::Equal,
                    15.0,
                ),
                "fix_quantity",
            )
            .expect("fix quantity");

        let tariff = AllUnitsDiscountFunction::with_breakpoints(
            10,
            "discount",
            Linear::new(vec![LinearMonomial::new(1.0, input_index)], 0.0),
            vec![0.0, 10.0, 20.0],
            vec![5.0, 4.0],
            1.0,
        )
        .expect("valid all-units tariff");
        assert_eq!(tariff.category(), Category::Linear);
        let result_id = tariff.result_variable().id();
        let quantity_ids = tariff
            .tier_quantity_variables()
            .iter()
            .map(ContinuousVariableItem::id)
            .collect::<Vec<_>>();
        let selector_ids = tariff
            .selector_variables()
            .iter()
            .map(|variable| variable.id())
            .collect::<Vec<_>>();
        model
            .add_symbols(vec![Arc::new(tariff) as Arc<dyn IntermediateSymbol<f64>>])
            .expect("register tariff");

        let mechanism = model.try_to_mechanism_model().expect("build mechanism");
        let mut values = vec![0.0; mechanism.tokens().len()];
        for (id, value) in [
            (input_id, 15.0),
            (result_id, 60.0),
            (quantity_ids[0], 0.0),
            (quantity_ids[1], 5.0),
            (selector_ids[0], 0.0),
            (selector_ids[1], 1.0),
        ] {
            let token = mechanism
                .find_token(id)
                .expect("registered candidate variable");
            values[token.solver_index] = value;
        }

        for constraint in mechanism.constraints() {
            let polynomial = &constraint.inequality.polynomial;
            let lhs = *polynomial.constant_term()
                + polynomial
                    .monomials()
                    .iter()
                    .map(|monomial| monomial.coefficient() * values[monomial.var_index()])
                    .sum::<f64>();
            let rhs = constraint.inequality.rhs;
            let feasible = match constraint.inequality.relation {
                ConstraintRelation::LessEqual => lhs <= rhs + 1e-9,
                ConstraintRelation::Equal => (lhs - rhs).abs() <= 1e-9,
                ConstraintRelation::GreaterEqual => lhs + 1e-9 >= rhs,
            };
            assert!(
                feasible,
                "candidate violates {}: {lhs} {:?} {rhs}",
                constraint.name, constraint.inequality.relation
            );
        }
    }
}

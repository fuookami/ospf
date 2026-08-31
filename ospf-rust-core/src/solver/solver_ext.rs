//! 求解器扩展入口
//! Solver Extension Entry Points

use super::{SolveValue, SolveValueConversionPolicy, Solver, SolverOutput, SolvingStatusCallback};
use crate::error::Result;
use crate::model::flatten::{Linear, LinearMonomial, Quadratic, QuadraticMonomial};
use crate::model::intermediate::{LinearTriadModel, QuadraticTetradModel};
use crate::model::mechanism::{
    BasicMechanismModel, LinearConstraint, LinearInequality, MechanismModel, QuadraticConstraint,
    QuadraticInequality,
};
use crate::model::{MetaModel, ModelBuildingStatusCallback};
use crate::model::{Objective, SubObjective};
use crate::solver::iis::{IISConfig, LinearIISModel, compute_iis};
use crate::token::{AnyVariable, Token, TokenVariableData};

/// 统一求解参数 / Unified solve options
#[derive(Clone, Copy, Default)]
pub struct SolveOptions<'a> {
    /// 期望的解数量（多解接口）/ Expected solution amount (for multi-solution APIs)
    pub solution_amount: usize,
    /// 建模阶段回调 / Model-building status callback
    pub model_building_status_callback: Option<&'a ModelBuildingStatusCallback>,
    /// 求解阶段回调 / Solving status callback
    pub solving_status_callback: Option<&'a SolvingStatusCallback>,
    /// 数值转换策略 / Numeric conversion policy
    pub value_conversion_policy: SolveValueConversionPolicy,
}

impl<'a> SolveOptions<'a> {
    /// 创建默认配置 / Create default options
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置解数量 / Set solution amount
    pub fn with_solution_amount(mut self, solution_amount: usize) -> Self {
        self.solution_amount = solution_amount.max(1);
        self
    }

    /// 设置建模回调 / Set model-building callback
    pub fn with_building_callback(
        mut self,
        callback: Option<&'a ModelBuildingStatusCallback>,
    ) -> Self {
        self.model_building_status_callback = callback;
        self
    }

    /// 设置求解回调 / Set solving callback
    pub fn with_solving_callback(mut self, callback: Option<&'a SolvingStatusCallback>) -> Self {
        self.solving_status_callback = callback;
        self
    }

    /// 设置数值转换策略 / Set numeric conversion policy
    pub fn with_value_conversion_policy(
        mut self,
        value_conversion_policy: SolveValueConversionPolicy,
    ) -> Self {
        self.value_conversion_policy = value_conversion_policy;
        self
    }
}

fn convert_linear_to_f64<V>(
    polynomial: &Linear<V>,
    policy: SolveValueConversionPolicy,
) -> Result<Linear<f64>>
where
    V: SolveValue,
{
    let mut monomials: Vec<LinearMonomial<f64>> = Vec::with_capacity(polynomial.monomials().len());
    for monomial in polynomial.monomials() {
        monomials.push(LinearMonomial::new(
            monomial.coefficient().to_f64_with_policy(policy)?,
            monomial.var_index(),
        ));
    }
    Ok(Linear::new(
        monomials,
        polynomial.constant_term().to_f64_with_policy(policy)?,
    ))
}

fn convert_quadratic_to_f64<V>(
    polynomial: &Quadratic<V>,
    policy: SolveValueConversionPolicy,
) -> Result<Quadratic<f64>>
where
    V: SolveValue,
{
    let mut monomials: Vec<QuadraticMonomial<f64>> =
        Vec::with_capacity(polynomial.monomials().len());
    for monomial in polynomial.monomials() {
        let coefficient = monomial.coefficient().to_f64_with_policy(policy)?;
        let converted = if let Some(var_index2) = monomial.var_index2() {
            QuadraticMonomial::new_quadratic(coefficient, monomial.var_index1(), var_index2)
        } else {
            QuadraticMonomial::new_linear(coefficient, monomial.var_index1())
        };
        monomials.push(converted);
    }
    Ok(Quadratic::new(
        monomials,
        polynomial.constant().to_f64_with_policy(policy)?,
    ))
}

/// 将泛型机理模型按策略转换为 `f64` 机理模型 / Convert generic mechanism model into `f64` mechanism model with policy
pub fn convert_mechanism_model_to_f64<V>(
    mechanism_model: &MechanismModel<V>,
    policy: SolveValueConversionPolicy,
) -> Result<MechanismModel<f64>>
where
    V: SolveValue,
{
    let mut basic_mechanism_model = BasicMechanismModel::new(&mechanism_model.basic.name);
    for token in mechanism_model.basic.tokens() {
        let variable_data = token.variable.data();
        let converted_data = TokenVariableData {
            id: variable_data.id,
            index: variable_data.index,
            name: variable_data.name.clone(),
            display_name: variable_data.display_name.clone(),
            var_type: variable_data.var_type,
            lower_bound: variable_data
                .lower_bound
                .as_ref()
                .map(|value| value.to_f64_with_policy(policy))
                .transpose()?,
            upper_bound: variable_data
                .upper_bound
                .as_ref()
                .map(|value| value.to_f64_with_policy(policy))
                .transpose()?,
        };
        let converted_token = Token::new(AnyVariable::new(converted_data), token.solver_index);
        if let Some(result) = token.get_result() {
            converted_token.set_result(result.to_f64_with_policy(policy)?);
        }
        basic_mechanism_model.add_token(converted_token);
    }

    for constraint in mechanism_model.basic.constraints() {
        let inequality = &constraint.inequality;
        let converted_inequality = LinearInequality::new(
            convert_linear_to_f64(&inequality.polynomial, policy)?,
            inequality.relation,
            inequality.rhs.to_f64_with_policy(policy)?,
        );
        let mut converted_constraint =
            LinearConstraint::new(converted_inequality, &constraint.name);
        converted_constraint.group = constraint.group.clone();
        converted_constraint.lazy = constraint.lazy;
        converted_constraint.priority = constraint.priority;
        converted_constraint.args = constraint.args.clone();
        basic_mechanism_model.add_constraint(converted_constraint);
    }

    for constraint in mechanism_model.basic.quadratic_constraints() {
        let inequality = &constraint.inequality;
        let converted_inequality = QuadraticInequality::new(
            convert_quadratic_to_f64(&inequality.polynomial, policy)?,
            inequality.relation,
            inequality.rhs.to_f64_with_policy(policy)?,
        );
        let mut converted_constraint =
            QuadraticConstraint::new(converted_inequality, &constraint.name);
        converted_constraint.group = constraint.group.clone();
        converted_constraint.lazy = constraint.lazy;
        converted_constraint.priority = constraint.priority;
        converted_constraint.args = constraint.args.clone();
        basic_mechanism_model.add_quadratic_constraint(converted_constraint);
    }

    let objective = mechanism_model.objective();
    let mut converted_objective = Objective::new(objective.category);
    for sub_objective in &objective.sub_objectives {
        converted_objective.add_sub_objective(SubObjective::new_with_weight(
            sub_objective.category,
            convert_linear_to_f64(&sub_objective.polynomial, policy)?,
            &sub_objective.name,
            sub_objective.weight.to_f64_with_policy(policy)?,
        ));
    }

    let mut converted_model = MechanismModel::from_basic(basic_mechanism_model);
    converted_model.set_objective(converted_objective);
    Ok(converted_model)
}

/// Nightly: 可调用求解器包装器 / Nightly: callable solver wrapper
#[cfg(feature = "nightly")]
#[derive(Clone, Copy)]
pub struct SolveFn<'a, S>
where
    S: Solver + ?Sized,
{
    solver: &'a S,
    options: SolveOptions<'a>,
}

#[cfg(feature = "nightly")]
impl<'a, S> SolveFn<'a, S>
where
    S: Solver + ?Sized,
{
    /// 创建可调用包装器 / Create callable wrapper
    pub fn new(solver: &'a S) -> Self {
        Self {
            solver,
            options: SolveOptions::default(),
        }
    }

    /// 设置参数 / Set options
    pub fn with_options(mut self, options: SolveOptions<'a>) -> Self {
        self.options = options;
        self
    }
}

#[cfg(feature = "nightly")]
impl<'a, S> FnOnce<(&MetaModel<f64>,)> for SolveFn<'a, S>
where
    S: Solver + ?Sized,
{
    type Output = Result<SolverOutput>;

    extern "rust-call" fn call_once(self, args: (&MetaModel<f64>,)) -> Self::Output {
        self.solver.solve_with_options(args.0, &self.options)
    }
}

#[cfg(feature = "nightly")]
impl<'a, S> FnMut<(&MetaModel<f64>,)> for SolveFn<'a, S>
where
    S: Solver + ?Sized,
{
    extern "rust-call" fn call_mut(&mut self, args: (&MetaModel<f64>,)) -> Self::Output {
        self.solver.solve_with_options(args.0, &self.options)
    }
}

#[cfg(feature = "nightly")]
impl<'a, S> Fn<(&MetaModel<f64>,)> for SolveFn<'a, S>
where
    S: Solver + ?Sized,
{
    extern "rust-call" fn call(&self, args: (&MetaModel<f64>,)) -> Self::Output {
        self.solver.solve_with_options(args.0, &self.options)
    }
}

/// 带 IIS 的求解输出 / Solver output with IIS
#[derive(Debug, Clone)]
pub struct SolverOutputWithIIS {
    /// 原始求解输出 / Raw solver output
    pub output: SolverOutput,
    /// IIS 结果（仅不可行时）/ IIS result (only when infeasible)
    pub iis: Option<LinearIISModel>,
}

/// 多解输出（兼容接口）/ Multi-solution output (compatibility interface)
#[derive(Debug, Clone)]
pub struct MultiSolutionOutput {
    /// 主求解输出 / Primary solver output
    pub output: SolverOutput,
    /// 解池 / Solution pool
    pub solutions: Vec<Vec<f64>>,
}

/// 求解器扩展 trait / Solver extension trait
pub trait SolverExt: Solver {
    /// Nightly: 获取可调用包装器 / Nightly: get callable wrapper
    #[cfg(feature = "nightly")]
    fn as_fn<'a>(&'a self) -> SolveFn<'a, Self>
    where
        Self: Sized,
    {
        SolveFn::new(self)
    }

    /// Nightly: 获取带参数的可调用包装器 / Nightly: get callable wrapper with options
    #[cfg(feature = "nightly")]
    fn as_fn_with_options<'a>(&'a self, options: SolveOptions<'a>) -> SolveFn<'a, Self>
    where
        Self: Sized,
    {
        SolveFn::new(self).with_options(options)
    }

    /// 统一 MetaModel 入口 / Unified MetaModel entry
    fn solve<V>(&self, model: &MetaModel<V>) -> Result<SolverOutput>
    where
        V: SolveValue,
    {
        self.solve_with_options(model, &SolveOptions::default())
    }

    /// 统一 MetaModel 入口（参数对象）/ Unified MetaModel entry (options object)
    fn solve_with_options<V>(
        &self,
        model: &MetaModel<V>,
        options: &SolveOptions<'_>,
    ) -> Result<SolverOutput>
    where
        V: SolveValue,
    {
        let mechanism_model = model
            .try_to_mechanism_model_with_status_callback(options.model_building_status_callback)?;
        let mechanism_model =
            convert_mechanism_model_to_f64(&mechanism_model, options.value_conversion_policy)?;
        if mechanism_model.num_quadratic_constraints() > 0 {
            let quadratic_model = mechanism_model
                .try_into_quadratic_tetrad_model_with_status_callback(
                    options.model_building_status_callback,
                )?;
            return self.solve_quadratic_with_options(&quadratic_model, options);
        }

        let linear_model = mechanism_model.try_into_linear_triad_model_with_status_callback(
            options.model_building_status_callback,
        )?;
        self.solve_linear_with_options(&linear_model, options)
    }

    /// 线性模型 + IIS fallback / Linear solve with IIS fallback
    fn solve_linear_with_iis(
        &self,
        model: &LinearTriadModel,
        iis_config: &IISConfig,
    ) -> Result<SolverOutputWithIIS> {
        let output = self.solve_linear(model)?;
        let iis = if output.status.is_infeasible() {
            Some(compute_iis(model.as_basic(), iis_config)?)
        } else {
            None
        };
        Ok(SolverOutputWithIIS { output, iis })
    }

    /// 二次模型 + IIS fallback（针对线性约束部分）/ Quadratic solve with IIS fallback (linear constraints only)
    fn solve_quadratic_with_iis(
        &self,
        model: &QuadraticTetradModel,
        iis_config: &IISConfig,
    ) -> Result<SolverOutputWithIIS> {
        let output = self.solve_quadratic(model)?;
        let iis = if output.status.is_infeasible() {
            Some(compute_iis(&model.basic.linear, iis_config)?)
        } else {
            None
        };
        Ok(SolverOutputWithIIS { output, iis })
    }

    /// 线性模型多解接口（默认返回主解）/ Multi-solution API for linear model (returns primary solution by default)
    fn solve_linear_multi_with_options(
        &self,
        model: &LinearTriadModel,
        options: &SolveOptions<'_>,
    ) -> Result<MultiSolutionOutput> {
        if options.solution_amount > 1 {
            if let Some((output, solutions)) =
                self.solve_linear_with_solution_pool(model, options.solution_amount)?
            {
                return Ok(MultiSolutionOutput { output, solutions });
            }
        }

        let output = self.solve_linear(model)?;
        let mut solutions = Vec::new();
        if options.solution_amount > 0 {
            if let Some(solution) = output.solution.clone() {
                solutions.push(solution);
            }
        }
        Ok(MultiSolutionOutput { output, solutions })
    }

    /// 二次模型多解接口（默认返回主解）/ Multi-solution API for quadratic model (returns primary solution by default)
    fn solve_quadratic_multi_with_options(
        &self,
        model: &QuadraticTetradModel,
        options: &SolveOptions<'_>,
    ) -> Result<MultiSolutionOutput> {
        if options.solution_amount > 1 {
            if let Some((output, solutions)) =
                self.solve_quadratic_with_solution_pool(model, options.solution_amount)?
            {
                return Ok(MultiSolutionOutput { output, solutions });
            }
        }

        let output = self.solve_quadratic(model)?;
        let mut solutions = Vec::new();
        if options.solution_amount > 0 {
            if let Some(solution) = output.solution.clone() {
                solutions.push(solution);
            }
        }
        Ok(MultiSolutionOutput { output, solutions })
    }

    /// 统一异步入口（线性）/ Unified async entry (linear)
    #[cfg(feature = "async")]
    async fn solve_linear_async(&self, model: &LinearTriadModel) -> Result<SolverOutput>
    where
        Self: Sync,
    {
        self.solve_linear(model)
    }

    /// 统一异步入口（二次）/ Unified async entry (quadratic)
    #[cfg(feature = "async")]
    async fn solve_quadratic_async(&self, model: &QuadraticTetradModel) -> Result<SolverOutput>
    where
        Self: Sync,
    {
        self.solve_quadratic(model)
    }

    /// 统一异步入口（MetaModel）/ Unified async entry (MetaModel)
    #[cfg(feature = "async")]
    async fn solve_async<V>(&self, model: &MetaModel<V>) -> Result<SolverOutput>
    where
        Self: Sync,
        V: SolveValue,
    {
        self.solve(model)
    }
}

impl<T> SolverExt for T where T: Solver + ?Sized {}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::error::{CoreError, SolverError};
    use crate::model::flatten::{Linear, LinearMonomial};
    use crate::model::intermediate::{BasicLinearTriadModel, BasicQuadraticTetradModel};
    use crate::model::{
        ConstraintRelation, MetaModel, ModelBuildingStage, ModelBuildingStatusCallback,
    };
    use crate::solver::{
        LinearSolver, QuadraticSolver, SolverCapability, SolverInfo, SolverStatus,
    };
    use crate::variable::ContinuousVariableItem;
    use bigdecimal::BigDecimal;
    use num_rational::BigRational;

    #[derive(Debug)]
    struct DummySolver;

    impl SolverInfo for DummySolver {
        fn name(&self) -> &str {
            "dummy"
        }

        fn capabilities(&self) -> Vec<SolverCapability> {
            vec![SolverCapability::Linear, SolverCapability::Quadratic]
        }
    }

    impl LinearSolver for DummySolver {
        fn solve_linear(&self, _model: &LinearTriadModel) -> Result<SolverOutput> {
            Ok(SolverOutput::optimal(1.0, vec![2.0, 3.0]))
        }
    }

    impl QuadraticSolver for DummySolver {
        fn solve_quadratic(&self, _model: &QuadraticTetradModel) -> Result<SolverOutput> {
            Ok(SolverOutput::new(SolverStatus::Optimal).with_solution(vec![4.0]))
        }
    }

    #[derive(Debug)]
    struct NativePoolSolver;

    impl SolverInfo for NativePoolSolver {
        fn name(&self) -> &str {
            "native_pool_solver"
        }

        fn capabilities(&self) -> Vec<SolverCapability> {
            vec![SolverCapability::Linear, SolverCapability::Quadratic]
        }
    }

    impl LinearSolver for NativePoolSolver {
        fn solve_linear(&self, _model: &LinearTriadModel) -> Result<SolverOutput> {
            Ok(SolverOutput::optimal(1.0, vec![10.0]))
        }

        fn solve_linear_with_solution_pool(
            &self,
            _model: &LinearTriadModel,
            _solution_amount: usize,
        ) -> Result<Option<(SolverOutput, Vec<Vec<f64>>)>> {
            Ok(Some((
                SolverOutput::optimal(1.0, vec![10.0]),
                vec![vec![10.0], vec![11.0], vec![12.0]],
            )))
        }
    }

    impl QuadraticSolver for NativePoolSolver {
        fn solve_quadratic(&self, _model: &QuadraticTetradModel) -> Result<SolverOutput> {
            Ok(SolverOutput::optimal(2.0, vec![20.0]))
        }
    }

    #[test]
    fn multi_solution_output_returns_primary_solution_for_linear() {
        let solver = DummySolver;
        let model = LinearTriadModel::from_basic(BasicLinearTriadModel::new("dummy_linear"));
        let output = solver
            .solve_linear_multi_with_options(&model, &SolveOptions::new().with_solution_amount(5))
            .expect("multi solution should succeed");
        assert_eq!(output.solutions.len(), 1);
        assert_eq!(output.solutions[0], vec![2.0, 3.0]);
    }

    #[test]
    fn multi_solution_output_returns_primary_solution_for_quadratic() {
        let solver = DummySolver;
        let model = QuadraticTetradModel::from_basic(BasicQuadraticTetradModel::new("dummy_q"));
        let output = solver
            .solve_quadratic_multi_with_options(
                &model,
                &SolveOptions::new().with_solution_amount(3),
            )
            .expect("quadratic multi solution should succeed");
        assert_eq!(output.solutions.len(), 1);
        assert_eq!(output.solutions[0], vec![4.0]);
    }

    #[test]
    fn multi_solution_prefers_native_solution_pool_when_available() {
        let solver = NativePoolSolver;
        let model = LinearTriadModel::from_basic(BasicLinearTriadModel::new("native_pool_linear"));
        let output = solver
            .solve_linear_multi_with_options(&model, &SolveOptions::new().with_solution_amount(3))
            .expect("native pool multi solution should succeed");
        assert_eq!(output.solutions.len(), 3);
        assert_eq!(output.solutions[0], vec![10.0]);
        assert_eq!(output.solutions[1], vec![11.0]);
        assert_eq!(output.solutions[2], vec![12.0]);
    }

    #[test]
    fn solve_with_options_supports_meta_model_entry_with_callbacks() {
        let mut model = MetaModel::<f64>::new("dummy_meta");
        let x = ContinuousVariableItem::auto("dummy_meta_x");
        let x_index = model.register_variable(x).unwrap();
        model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                2.0,
                "dummy_meta_c",
            )
            .unwrap();

        let model_stages = Arc::new(Mutex::new(Vec::new()));
        let model_stages_for_callback = model_stages.clone();
        let model_callback: ModelBuildingStatusCallback = Arc::new(move |status| {
            model_stages_for_callback.lock().unwrap().push(status.stage);
            Ok(())
        });

        let solving_statuses = Arc::new(Mutex::new(Vec::new()));
        let solving_statuses_for_callback = solving_statuses.clone();
        let solving_callback: SolvingStatusCallback = Arc::new(move |status| {
            solving_statuses_for_callback
                .lock()
                .unwrap()
                .push(status.status);
            Ok(())
        });

        let solver = DummySolver;
        let options = SolveOptions::new()
            .with_building_callback(Some(&model_callback))
            .with_solving_callback(Some(&solving_callback));
        let output = solver
            .solve_with_options(&model, &options)
            .expect("meta model solve with callbacks should succeed");
        assert!(output.status.is_optimal());

        let model_stages = model_stages.lock().unwrap();
        assert!(
            model_stages
                .iter()
                .any(|stage| *stage == ModelBuildingStage::RegisterTokens)
        );
        assert!(
            model_stages
                .iter()
                .any(|stage| *stage == ModelBuildingStage::FlattenLinearModel)
        );

        let solving_statuses = solving_statuses.lock().unwrap();
        assert_eq!(solving_statuses.len(), 2);
        assert_eq!(solving_statuses[0], SolverStatus::Solving);
        assert_eq!(solving_statuses[1], SolverStatus::Optimal);
    }

    #[test]
    fn solve_with_options_supports_meta_model_entry() {
        let mut model = MetaModel::<f64>::new("dummy_meta_options");
        let x = ContinuousVariableItem::auto("dummy_meta_options_x");
        let x_index = model.register_variable(x).unwrap();
        model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                2.0,
                "dummy_meta_options_c",
            )
            .unwrap();

        let solver = DummySolver;
        let options = SolveOptions::new();
        let output = solver
            .solve_with_options(&model, &options)
            .expect("meta model solve with options should succeed");
        assert!(output.status.is_optimal());
    }

    #[test]
    fn solve_with_options_supports_big_rational_meta_model_entry() {
        let mut model = MetaModel::<BigRational>::new("dummy_meta_big_rational");
        let x = ContinuousVariableItem::auto("dummy_meta_big_rational_x");
        let x_index = model.register_variable(x).unwrap();
        let polynomial = Linear::new(
            vec![LinearMonomial::new(
                BigRational::from_integer(1.into()),
                x_index,
            )],
            BigRational::from_integer(0.into()),
        );
        model
            .add_linear_polynomial_constraint(
                polynomial,
                ConstraintRelation::LessEqual,
                BigRational::from_integer(2.into()),
                "dummy_meta_big_rational_c",
            )
            .unwrap();

        let solver = DummySolver;
        let options =
            SolveOptions::new().with_value_conversion_policy(SolveValueConversionPolicy::Strict);
        let output = solver
            .solve_with_options(&model, &options)
            .expect("big rational meta model solve with options should succeed");
        assert!(output.status.is_optimal());
    }

    #[test]
    fn solve_with_options_supports_big_decimal_meta_model_entry() {
        let mut model = MetaModel::<BigDecimal>::new("dummy_meta_big_decimal");
        let x = ContinuousVariableItem::auto("dummy_meta_big_decimal_x");
        let x_index = model.register_variable(x).unwrap();
        let polynomial = Linear::new(
            vec![LinearMonomial::new(
                BigDecimal::from_str("1.0").expect("create decimal coefficient"),
                x_index,
            )],
            BigDecimal::from_str("0.0").expect("create decimal constant"),
        );
        model
            .add_linear_polynomial_constraint(
                polynomial,
                ConstraintRelation::LessEqual,
                BigDecimal::from_str("2.0").expect("create decimal rhs"),
                "dummy_meta_big_decimal_c",
            )
            .unwrap();

        let solver = DummySolver;
        let options =
            SolveOptions::new().with_value_conversion_policy(SolveValueConversionPolicy::Strict);
        let output = solver
            .solve_with_options(&model, &options)
            .expect("big decimal meta model solve with options should succeed");
        assert!(output.status.is_optimal());
    }

    #[test]
    fn solve_with_options_rejects_big_rational_precision_loss_in_strict_mode() {
        let mut model =
            MetaModel::<BigRational>::new("dummy_meta_big_rational_precision_loss_strict");
        let x = ContinuousVariableItem::auto("dummy_meta_big_rational_precision_loss_strict_x");
        let x_index = model.register_variable(x).unwrap();
        let polynomial = Linear::new(
            vec![LinearMonomial::new(
                BigRational::new(1.into(), 3.into()),
                x_index,
            )],
            BigRational::from_integer(0.into()),
        );
        model
            .add_linear_polynomial_constraint(
                polynomial,
                ConstraintRelation::LessEqual,
                BigRational::from_integer(2.into()),
                "dummy_meta_big_rational_precision_loss_strict_c",
            )
            .unwrap();

        let solver = DummySolver;
        let options =
            SolveOptions::new().with_value_conversion_policy(SolveValueConversionPolicy::Strict);
        let error = solver
            .solve_with_options(&model, &options)
            .expect_err("strict mode should reject precision-loss big rational conversion");
        assert!(matches!(
            error,
            CoreError::Solver(SolverError::PrecisionLoss(_))
        ));
    }

    #[test]
    fn solve_with_options_rejects_big_decimal_precision_loss_in_strict_mode() {
        let mut model =
            MetaModel::<BigDecimal>::new("dummy_meta_big_decimal_precision_loss_strict");
        let x = ContinuousVariableItem::auto("dummy_meta_big_decimal_precision_loss_strict_x");
        let x_index = model.register_variable(x).unwrap();
        let polynomial = Linear::new(
            vec![LinearMonomial::new(
                BigDecimal::from_str("0.12345678901234567890123456789")
                    .expect("create high precision decimal coefficient"),
                x_index,
            )],
            BigDecimal::from_str("0.0").expect("create decimal constant"),
        );
        model
            .add_linear_polynomial_constraint(
                polynomial,
                ConstraintRelation::LessEqual,
                BigDecimal::from_str("2.0").expect("create decimal rhs"),
                "dummy_meta_big_decimal_precision_loss_strict_c",
            )
            .unwrap();

        let solver = DummySolver;
        let options =
            SolveOptions::new().with_value_conversion_policy(SolveValueConversionPolicy::Strict);
        let error = solver
            .solve_with_options(&model, &options)
            .expect_err("strict mode should reject precision-loss big decimal conversion");
        assert!(matches!(
            error,
            CoreError::Solver(SolverError::PrecisionLoss(_))
        ));
    }

    #[cfg(feature = "nightly")]
    #[test]
    fn solve_fn_shortcut_supports_meta_model_call() {
        let mut model = MetaModel::<f64>::new("dummy_meta_fn");
        let x = ContinuousVariableItem::auto("dummy_meta_fn_x");
        let x_index = model.register_variable(x).unwrap();
        model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                2.0,
                "dummy_meta_fn_c",
            )
            .unwrap();

        let solver = DummySolver;
        let callable = solver.as_fn();
        let output = callable(&model).expect("nightly callable solver should succeed");
        assert!(output.status.is_optimal());
    }

    #[cfg(feature = "nightly")]
    #[test]
    fn solve_fn_with_options_forwards_solving_callback() {
        let mut model = MetaModel::<f64>::new("dummy_meta_fn_options");
        let x = ContinuousVariableItem::auto("dummy_meta_fn_options_x");
        let x_index = model.register_variable(x).unwrap();
        model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                2.0,
                "dummy_meta_fn_options_c",
            )
            .unwrap();

        let statuses = Arc::new(Mutex::new(Vec::new()));
        let statuses_for_callback = statuses.clone();
        let callback: SolvingStatusCallback = Arc::new(move |status| {
            statuses_for_callback.lock().unwrap().push(status.status);
            Ok(())
        });

        let solver = DummySolver;
        let options = SolveOptions::new().with_solving_callback(Some(&callback));
        let callable = solver.as_fn_with_options(options);
        let output = callable(&model).expect("nightly callable solver with options should succeed");
        assert!(output.status.is_optimal());

        let statuses = statuses.lock().unwrap();
        assert_eq!(statuses.len(), 2);
        assert_eq!(statuses[0], SolverStatus::Solving);
        assert_eq!(statuses[1], SolverStatus::Optimal);
    }
}

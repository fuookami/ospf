//! Meta model.

use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

use super::flatten::{Linear, LinearMonomial};
use super::intermediate::{LinearTriadModel, QuadraticTetradModel};
use super::mechanism::{
    BasicMechanismModel, Constraint, ConstraintGroup, ConstraintRelation, LinearInequality,
    MechanismModel, MetaConstraint, SymbolicLinearConstraint, SymbolicLinearInequality,
    SymbolicQuadraticConstraint, SymbolicQuadraticInequality,
};
use super::{
    BasicModel, MetaModelConfiguration, ModelBuildingStage, ModelBuildingStatus,
    ModelBuildingStatusCallback, Objective, ObjectiveCategory, SubObjective,
};
use crate::error::{ModelError, Result};
use crate::symbol::IntermediateSymbol;
use crate::variable::VariableRange;
use ospf_rust_math::symbol::{Comparison, LinearInequality as MathLinearInequality};

/// User-facing model with objective and constraints.
pub struct MetaModel<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    basic: BasicModel<V>,
    objective: Objective<V>,
    config: MetaModelConfiguration,
    symbolic_constraints: Vec<SymbolicLinearConstraint<V>>,
    symbolic_quadratic_constraints: Vec<SymbolicQuadraticConstraint<V>>,
}

impl<V> MetaModel<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn emit_model_building_status(
        callback: Option<&ModelBuildingStatusCallback>,
        status: ModelBuildingStatus,
    ) -> Result<()> {
        if let Some(callback) = callback {
            callback(&status)?;
        }
        Ok(())
    }

    pub fn new(name: &str) -> Self {
        Self {
            basic: BasicModel::new(name),
            objective: Objective::default(),
            config: MetaModelConfiguration::default(),
            symbolic_constraints: Vec::new(),
            symbolic_quadratic_constraints: Vec::new(),
        }
    }

    pub fn from_basic(basic: BasicModel<V>) -> Self {
        Self {
            basic,
            objective: Objective::default(),
            config: MetaModelConfiguration::default(),
            symbolic_constraints: Vec::new(),
            symbolic_quadratic_constraints: Vec::new(),
        }
    }

    pub fn ensure_flatten_context(&mut self) {
        self.basic.ensure_flatten_context();
    }

    pub fn ensure_value_cache_context(&mut self) {
        self.basic.ensure_value_cache_context();
    }

    pub fn ensure_range_cache_context(&mut self) {
        self.basic.ensure_range_cache_context();
    }

    /// 鑾峰彇妯″瀷閰嶇疆 / Get model configuration
    pub fn config(&self) -> &MetaModelConfiguration {
        &self.config
    }

    /// 鑾峰彇鍙彉妯″瀷閰嶇疆 / Get mutable model configuration
    pub fn config_mut(&mut self) -> &mut MetaModelConfiguration {
        &mut self.config
    }

    /// 璁剧疆妯″瀷閰嶇疆 / Set model configuration
    pub fn set_config(&mut self, config: MetaModelConfiguration) {
        self.config = config;
    }

    pub fn maximize(&mut self) {
        self.objective.category = ObjectiveCategory::Maximum;
    }

    pub fn minimize(&mut self) {
        self.objective.category = ObjectiveCategory::Minimum;
    }

    pub fn set_objective_category(&mut self, category: ObjectiveCategory) {
        self.objective.category = category;
    }

    pub fn add_sub_objective(&mut self, sub_objective: SubObjective<V>) {
        self.objective.add_sub_objective(sub_objective);
    }

    pub fn objective(&self) -> &Objective<V> {
        &self.objective
    }

    pub fn objective_mut(&mut self) -> &mut Objective<V> {
        &mut self.objective
    }

    pub fn add_inequality(&mut self, inequality: LinearInequality<V>, name: &str) -> Result<()> {
        let constraint = MetaConstraint::new(inequality, name);
        self.basic.add_constraint(constraint)
    }

    pub fn add_inequality_with_metadata(
        &mut self,
        inequality: LinearInequality<V>,
        name: &str,
        group: Option<Arc<ConstraintGroup>>,
        lazy: bool,
        priority: u32,
        args: Option<String>,
    ) -> Result<()> {
        let mut constraint = MetaConstraint::new(inequality, name).with_priority(priority);
        if let Some(group) = group {
            constraint = constraint.with_group(group);
        }
        if let Some(args) = args {
            constraint = constraint.with_args(args);
        }
        constraint.set_lazy(lazy);
        self.basic.add_constraint(constraint)
    }

    pub fn add_linear_polynomial_constraint(
        &mut self,
        polynomial: Linear<V>,
        relation: ConstraintRelation,
        rhs: V,
        name: &str,
    ) -> Result<()> {
        let inequality = LinearInequality::new(polynomial, relation, rhs);
        self.add_inequality(inequality, name)
    }

    pub fn add_symbolic_inequality(&mut self, inequality: SymbolicLinearInequality<V>, name: &str) {
        let constraint = SymbolicLinearConstraint::new(inequality, name);
        self.symbolic_constraints.push(constraint);
    }

    pub fn add_symbolic_constraint(&mut self, constraint: SymbolicLinearConstraint<V>) {
        self.symbolic_constraints.push(constraint);
    }

    pub fn add_symbolic_inequalities<I, N>(&mut self, inequalities: I)
    where
        I: IntoIterator<Item = (SymbolicLinearInequality<V>, N)>,
        N: AsRef<str>,
    {
        for (inequality, name) in inequalities {
            self.add_symbolic_inequality(inequality, name.as_ref());
        }
    }

    pub fn add_symbolic_quadratic_inequality(
        &mut self,
        inequality: SymbolicQuadraticInequality<V>,
        name: &str,
    ) {
        let constraint = SymbolicQuadraticConstraint::new(inequality, name);
        self.symbolic_quadratic_constraints.push(constraint);
    }

    pub fn add_symbolic_quadratic_constraint(
        &mut self,
        constraint: SymbolicQuadraticConstraint<V>,
    ) {
        self.symbolic_quadratic_constraints.push(constraint);
    }

    pub fn add_symbolic_quadratic_inequalities<I, N>(&mut self, inequalities: I)
    where
        I: IntoIterator<Item = (SymbolicQuadraticInequality<V>, N)>,
        N: AsRef<str>,
    {
        for (inequality, name) in inequalities {
            self.add_symbolic_quadratic_inequality(inequality, name.as_ref());
        }
    }

    pub fn add_math_inequality(&mut self, inequality: MathLinearInequality<V>, name: &str) {
        let relation = match inequality.comparison {
            Comparison::Less => ConstraintRelation::LessEqual,
            Comparison::LessEqual => ConstraintRelation::LessEqual,
            Comparison::Greater => ConstraintRelation::GreaterEqual,
            Comparison::GreaterEqual => ConstraintRelation::GreaterEqual,
            Comparison::Equal => ConstraintRelation::Equal,
        };

        let symbolic_inequality =
            SymbolicLinearInequality::new(inequality.lhs, relation, inequality.rhs);
        let constraint = SymbolicLinearConstraint::new(symbolic_inequality, name);
        self.symbolic_constraints.push(constraint);
    }

    pub fn add_symbols<I>(&mut self, symbols: I) -> Result<()>
    where
        I: IntoIterator<Item = std::sync::Arc<dyn IntermediateSymbol<V>>>,
    {
        for symbol in symbols {
            self.basic.add_symbol(symbol)?;
        }
        Ok(())
    }

    pub fn try_to_mechanism_model(&self) -> Result<MechanismModel<V>> {
        self.try_to_mechanism_model_with_status_callback(None)
    }

    pub fn try_to_mechanism_model_with_status_callback(
        &self,
        callback: Option<&ModelBuildingStatusCallback>,
    ) -> Result<MechanismModel<V>> {
        let mut basic_mechanism = BasicMechanismModel::new(&self.basic.name);
        let model_name = self.basic.name.clone();

        let mut symbol_to_index: HashMap<usize, usize> = HashMap::new();
        let token_total = self.basic.tokens().len();
        Self::emit_model_building_status(
            callback,
            ModelBuildingStatus::new(
                model_name.clone(),
                ModelBuildingStage::RegisterTokens,
                0,
                token_total,
            ),
        )?;

        for (idx, token) in self.basic.tokens().iter().enumerate() {
            symbol_to_index.insert(token.id().unique_id() as usize, idx);
            basic_mechanism.add_token(token.clone());
            Self::emit_model_building_status(
                callback,
                ModelBuildingStatus::new(
                    model_name.clone(),
                    ModelBuildingStage::RegisterTokens,
                    idx + 1,
                    token_total,
                ),
            )?;
        }

        let linear_constraint_total =
            self.basic.constraints().len() + self.symbolic_constraints.len();
        let mut linear_ready = 0usize;
        Self::emit_model_building_status(
            callback,
            ModelBuildingStatus::new(
                model_name.clone(),
                ModelBuildingStage::RegisterLinearConstraints,
                0,
                linear_constraint_total,
            ),
        )?;

        for meta_constraint in self.basic.constraints() {
            let inequality = LinearInequality::new(
                meta_constraint.inequality.polynomial.clone(),
                meta_constraint.inequality.relation,
                meta_constraint.inequality.rhs.clone(),
            );
            let mut constraint = Constraint::new(inequality, &meta_constraint.name);
            constraint.group = meta_constraint.group.clone();
            constraint.lazy = meta_constraint.lazy;
            constraint.priority = meta_constraint.priority;
            constraint.args = meta_constraint.args.clone();
            basic_mechanism.add_constraint(constraint);
            linear_ready += 1;
            Self::emit_model_building_status(
                callback,
                ModelBuildingStatus::new(
                    model_name.clone(),
                    ModelBuildingStage::RegisterLinearConstraints,
                    linear_ready,
                    linear_constraint_total,
                ),
            )?;
        }

        for symbolic_constraint in self.symbolic_constraints.iter() {
            let inequality = symbolic_constraint
                .inequality
                .clone()
                .try_into_linear_inequality(&symbol_to_index)
                .map_err(|err| {
                    ModelError::InvalidConstraint(format!(
                        "symbolic constraint `{}` conversion failed: {}",
                        symbolic_constraint.name, err
                    ))
                })?;
            let constraint = Constraint::new(inequality, &symbolic_constraint.name);
            basic_mechanism.add_constraint(constraint);
            linear_ready += 1;
            Self::emit_model_building_status(
                callback,
                ModelBuildingStatus::new(
                    model_name.clone(),
                    ModelBuildingStage::RegisterLinearConstraints,
                    linear_ready,
                    linear_constraint_total,
                ),
            )?;
        }

        let quadratic_constraint_total = self.symbolic_quadratic_constraints.len();
        Self::emit_model_building_status(
            callback,
            ModelBuildingStatus::new(
                model_name.clone(),
                ModelBuildingStage::RegisterQuadraticConstraints,
                0,
                quadratic_constraint_total,
            ),
        )?;

        for (idx, symbolic_constraint) in self.symbolic_quadratic_constraints.iter().enumerate() {
            let inequality = symbolic_constraint
                .inequality
                .clone()
                .try_into_quadratic_inequality(&symbol_to_index)
                .map_err(|err| {
                    ModelError::InvalidConstraint(format!(
                        "symbolic quadratic constraint `{}` conversion failed: {}",
                        symbolic_constraint.name, err
                    ))
                })?;
            let constraint = Constraint::new(inequality, &symbolic_constraint.name);
            basic_mechanism.add_quadratic_constraint(constraint);
            Self::emit_model_building_status(
                callback,
                ModelBuildingStatus::new(
                    model_name.clone(),
                    ModelBuildingStage::RegisterQuadraticConstraints,
                    idx + 1,
                    quadratic_constraint_total,
                ),
            )?;
        }

        let symbol_total = self.basic.symbols().len();
        Self::emit_model_building_status(
            callback,
            ModelBuildingStatus::new(
                model_name.clone(),
                ModelBuildingStage::RegisterSymbols,
                0,
                symbol_total,
            ),
        )?;

        for (symbol_index, symbol) in self.basic.symbols().iter().enumerate() {
            let mut auxiliary_tokens = Vec::new();
            symbol.register_auxiliary_tokens(&mut auxiliary_tokens)?;

            let mut generated_constraints =
                symbol.mechanism_constraints_with_tokens(&symbol_to_index, self.basic.tokens())?;
            let mut generated_quadratic_constraints = symbol
                .quadratic_mechanism_constraints_with_tokens(
                    &symbol_to_index,
                    self.basic.tokens(),
                )?;
            if !auxiliary_tokens.is_empty()
                && generated_constraints.is_empty()
                && generated_quadratic_constraints.is_empty()
            {
                return Err(ModelError::InvalidConstraint(format!(
                    "symbol `{}` registers auxiliary tokens but emits no mechanism constraints (linear/quadratic)",
                    symbol.id().name
                ))
                .into());
            }

            for mut generated_constraint in generated_constraints.drain(..) {
                if generated_constraint.from.is_none() {
                    generated_constraint.set_from(symbol.clone());
                }
                basic_mechanism.add_constraint(generated_constraint);
            }
            for mut generated_constraint in generated_quadratic_constraints.drain(..) {
                if generated_constraint.from.is_none() {
                    generated_constraint.set_from(symbol.clone());
                }
                basic_mechanism.add_quadratic_constraint(generated_constraint);
            }
            Self::emit_model_building_status(
                callback,
                ModelBuildingStatus::new(
                    model_name.clone(),
                    ModelBuildingStage::RegisterSymbols,
                    symbol_index + 1,
                    symbol_total,
                ),
            )?;
        }

        let mut mechanism = MechanismModel::from_basic(basic_mechanism);
        Self::emit_model_building_status(
            callback,
            ModelBuildingStatus::new(model_name.clone(), ModelBuildingStage::BuildObjective, 0, 1),
        )?;
        mechanism.set_objective(self.objective.clone());
        Self::emit_model_building_status(
            callback,
            ModelBuildingStatus::new(model_name, ModelBuildingStage::BuildObjective, 1, 1),
        )?;
        Ok(mechanism)
    }

    pub fn try_into_mechanism_model(self) -> Result<MechanismModel<V>> {
        self.try_to_mechanism_model()
    }

    pub fn try_into_mechanism_model_with_status_callback(
        self,
        callback: Option<&ModelBuildingStatusCallback>,
    ) -> Result<MechanismModel<V>> {
        self.try_to_mechanism_model_with_status_callback(callback)
    }

    pub fn into_mechanism_model(self) -> MechanismModel<V> {
        self.try_into_mechanism_model().unwrap_or_else(|err| {
            panic!("failed to convert MetaModel into MechanismModel: {}", err)
        })
    }

    pub fn evaluate_symbol(&mut self, symbol: &dyn IntermediateSymbol<V>) -> Option<V> {
        self.basic.evaluate_symbol(symbol)
    }

    pub fn symbol_range(&mut self, symbol: &dyn IntermediateSymbol<V>) -> Option<VariableRange<V>> {
        self.basic.symbol_range(symbol)
    }

    pub fn evaluate_registered_symbol(&mut self, symbol_id: u64) -> Option<V> {
        self.basic.evaluate_registered_symbol(symbol_id)
    }

    pub fn registered_symbol_range(&mut self, symbol_id: u64) -> Option<VariableRange<V>> {
        self.basic.registered_symbol_range(symbol_id)
    }

    pub fn add_symbol_dependency(&mut self, symbol_id: u64, dependency_id: u64) -> Result<()> {
        self.basic.add_symbol_dependency(symbol_id, dependency_id)
    }

    pub fn add_symbol_dependencies<I>(&mut self, symbol_id: u64, dependency_ids: I) -> Result<()>
    where
        I: IntoIterator<Item = u64>,
    {
        self.basic
            .add_symbol_dependencies(symbol_id, dependency_ids)
    }

    pub fn symbol_dependency_ids(&self, symbol_id: u64) -> Vec<u64> {
        self.basic.symbol_dependency_ids(symbol_id)
    }

    pub fn add_symbol_with_dependencies<I>(
        &mut self,
        symbol: std::sync::Arc<dyn IntermediateSymbol<V>>,
        dependency_ids: I,
    ) -> Result<()>
    where
        I: IntoIterator<Item = u64>,
    {
        self.basic
            .add_symbol_with_dependencies(symbol, dependency_ids)
    }

    pub fn as_basic(&self) -> &BasicModel<V> {
        &self.basic
    }

    pub fn as_basic_mut(&mut self) -> &mut BasicModel<V> {
        &mut self.basic
    }

    pub fn into_basic(self) -> BasicModel<V> {
        self.basic
    }
}

impl MetaModel<f64> {
    /// 转换为线性三元组模型 / Convert into linear triad model
    pub fn try_to_linear_triad_model(&self) -> Result<LinearTriadModel> {
        self.try_to_linear_triad_model_with_status_callback(None)
    }

    /// 转换为线性三元组模型（带构建回调）/
    /// Convert into linear triad model with model-building callback
    pub fn try_to_linear_triad_model_with_status_callback(
        &self,
        callback: Option<&ModelBuildingStatusCallback>,
    ) -> Result<LinearTriadModel> {
        let mechanism = self.try_to_mechanism_model_with_status_callback(callback)?;
        mechanism.try_into_linear_triad_model_with_status_callback(callback)
    }

    /// 转换为线性三元组模型（失败即 panic）/
    /// Convert into linear triad model (panic on failure)
    pub fn to_linear_triad_model(&self) -> LinearTriadModel {
        self.try_to_linear_triad_model().unwrap_or_else(|err| {
            panic!("failed to convert MetaModel into LinearTriadModel: {}", err)
        })
    }

    /// 转换为线性三元组模型 / Convert into linear triad model
    pub fn try_into_linear_triad_model(self) -> Result<LinearTriadModel> {
        self.try_to_linear_triad_model()
    }

    /// 转换为线性三元组模型（带构建回调）/
    /// Convert into linear triad model with model-building callback
    pub fn try_into_linear_triad_model_with_status_callback(
        self,
        callback: Option<&ModelBuildingStatusCallback>,
    ) -> Result<LinearTriadModel> {
        self.try_to_linear_triad_model_with_status_callback(callback)
    }

    /// 转换为线性三元组模型（失败即 panic）/
    /// Convert into linear triad model (panic on failure)
    pub fn into_linear_triad_model(self) -> LinearTriadModel {
        self.to_linear_triad_model()
    }

    /// 一步完成“MetaModel -> 自动判型（线性/二次）-> 求解”/
    /// One-step "MetaModel -> auto select (linear/quadratic) -> solve"
    pub fn solve<S>(&self, solver: &S) -> Result<crate::solver::SolverOutput>
    where
        S: crate::solver::Solver + ?Sized,
    {
        self.solve_with_options(solver, &crate::solver::SolveOptions::default())
    }

    /// 一步完成“MetaModel -> 自动判型（线性/二次）-> 求解（参数对象）”/
    /// One-step "MetaModel -> auto select (linear/quadratic) -> solve (options object)"
    pub fn solve_with_options<S>(
        &self,
        solver: &S,
        options: &crate::solver::SolveOptions<'_>,
    ) -> Result<crate::solver::SolverOutput>
    where
        S: crate::solver::Solver + ?Sized,
    {
        use crate::solver::SolverExt;
        solver.solve_with_options(self, options)
    }

    /// 一步完成“MetaModel -> 线性模型 -> 求解”/
    /// One-step "MetaModel -> linear model -> solve"
    pub fn solve_linear_with<S>(self, solver: &S) -> Result<crate::solver::SolverOutput>
    where
        S: crate::solver::LinearSolver,
    {
        let linear_model = self.try_into_linear_triad_model()?;
        solver.solve_linear(&linear_model)
    }

    /// 一步完成“MetaModel -> 线性模型 -> 求解（参数对象）”/
    /// One-step "MetaModel -> linear model -> solve (options object)"
    pub fn solve_linear_with_options<S>(
        self,
        solver: &S,
        options: &crate::solver::SolveOptions<'_>,
    ) -> Result<crate::solver::SolverOutput>
    where
        S: crate::solver::LinearSolver,
    {
        let linear_model = self.try_into_linear_triad_model_with_status_callback(
            options.model_building_status_callback,
        )?;
        solver.solve_linear_with_options(&linear_model, options)
    }

    pub fn try_to_quadratic_tetrad_model(&self) -> Result<QuadraticTetradModel> {
        self.try_to_quadratic_tetrad_model_with_status_callback(None)
    }

    pub fn try_to_quadratic_tetrad_model_with_status_callback(
        &self,
        callback: Option<&ModelBuildingStatusCallback>,
    ) -> Result<QuadraticTetradModel> {
        let mechanism = self.try_to_mechanism_model_with_status_callback(callback)?;
        mechanism.try_into_quadratic_tetrad_model_with_status_callback(callback)
    }

    pub fn to_quadratic_tetrad_model(&self) -> QuadraticTetradModel {
        self.try_to_quadratic_tetrad_model().unwrap_or_else(|err| {
            panic!(
                "failed to convert MetaModel into QuadraticTetradModel: {}",
                err
            )
        })
    }

    pub fn try_into_quadratic_tetrad_model(self) -> Result<QuadraticTetradModel> {
        self.try_to_quadratic_tetrad_model()
    }

    pub fn try_into_quadratic_tetrad_model_with_status_callback(
        self,
        callback: Option<&ModelBuildingStatusCallback>,
    ) -> Result<QuadraticTetradModel> {
        self.try_to_quadratic_tetrad_model_with_status_callback(callback)
    }

    pub fn into_quadratic_tetrad_model(self) -> QuadraticTetradModel {
        self.to_quadratic_tetrad_model()
    }

    /// 一步完成“MetaModel -> 二次模型 -> 求解”/
    /// One-step "MetaModel -> quadratic model -> solve"
    pub fn solve_quadratic_with<S>(self, solver: &S) -> Result<crate::solver::SolverOutput>
    where
        S: crate::solver::QuadraticSolver,
    {
        let quadratic_model = self.try_into_quadratic_tetrad_model()?;
        solver.solve_quadratic(&quadratic_model)
    }

    /// 一步完成“MetaModel -> 二次模型 -> 求解（参数对象）”/
    /// One-step "MetaModel -> quadratic model -> solve (options object)"
    pub fn solve_quadratic_with_options<S>(
        self,
        solver: &S,
        options: &crate::solver::SolveOptions<'_>,
    ) -> Result<crate::solver::SolverOutput>
    where
        S: crate::solver::QuadraticSolver,
    {
        let quadratic_model = self.try_into_quadratic_tetrad_model_with_status_callback(
            options.model_building_status_callback,
        )?;
        solver.solve_quadratic_with_options(&quadratic_model, options)
    }

    pub fn add_linear_objective(&mut self, coefficients: &[(usize, f64)], name: &str) {
        let monomials: Vec<LinearMonomial<f64>> = coefficients
            .iter()
            .map(|(idx, coef)| LinearMonomial::new(*coef, *idx))
            .collect();
        let polynomial = Linear::new(monomials, 0.0);
        let sub_objective = SubObjective::new(self.objective.category, polynomial, name);
        self.objective.add_sub_objective(sub_objective);
    }

    pub fn set_linear_objective(&mut self, coefficients: Vec<f64>, category: ObjectiveCategory) {
        self.objective.category = category;
        let monomials: Vec<LinearMonomial<f64>> = coefficients
            .iter()
            .enumerate()
            .filter(|(_, c)| **c != 0.0)
            .map(|(idx, coef)| LinearMonomial::new(*coef, idx))
            .collect();
        let polynomial = Linear::new(monomials, 0.0);
        self.objective.sub_objectives.clear();
        let sub_objective = SubObjective::new(category, polynomial, "main_objective");
        self.objective.add_sub_objective(sub_objective);
    }

    pub fn add_linear_constraint(
        &mut self,
        coefficients: &[(usize, f64)],
        relation: ConstraintRelation,
        rhs: f64,
        name: &str,
    ) -> Result<()> {
        let monomials: Vec<LinearMonomial<f64>> = coefficients
            .iter()
            .map(|(idx, coef)| LinearMonomial::new(*coef, *idx))
            .collect();
        let polynomial = Linear::new(monomials, 0.0);
        self.add_linear_polynomial_constraint(polynomial, relation, rhs, name)
    }

    pub fn add_linear_constraint_with_metadata(
        &mut self,
        coefficients: &[(usize, f64)],
        relation: ConstraintRelation,
        rhs: f64,
        name: &str,
        group: Option<Arc<ConstraintGroup>>,
        lazy: bool,
        priority: u32,
        args: Option<String>,
    ) -> Result<()> {
        let monomials: Vec<LinearMonomial<f64>> = coefficients
            .iter()
            .map(|(idx, coef)| LinearMonomial::new(*coef, *idx))
            .collect();
        let polynomial = Linear::new(monomials, 0.0);
        let inequality = LinearInequality::new(polynomial, relation, rhs);
        self.add_inequality_with_metadata(inequality, name, group, lazy, priority, args)
    }

    pub fn add_le_constraint(
        &mut self,
        coefficients: &[(usize, f64)],
        rhs: f64,
        name: &str,
    ) -> Result<()> {
        self.add_linear_constraint(coefficients, ConstraintRelation::LessEqual, rhs, name)
    }

    pub fn add_ge_constraint(
        &mut self,
        coefficients: &[(usize, f64)],
        rhs: f64,
        name: &str,
    ) -> Result<()> {
        self.add_linear_constraint(coefficients, ConstraintRelation::GreaterEqual, rhs, name)
    }

    pub fn add_eq_constraint(
        &mut self,
        coefficients: &[(usize, f64)],
        rhs: f64,
        name: &str,
    ) -> Result<()> {
        self.add_linear_constraint(coefficients, ConstraintRelation::Equal, rhs, name)
    }

    pub fn add_le_constraint_with_metadata(
        &mut self,
        coefficients: &[(usize, f64)],
        rhs: f64,
        name: &str,
        group: Option<Arc<ConstraintGroup>>,
        lazy: bool,
        priority: u32,
        args: Option<String>,
    ) -> Result<()> {
        self.add_linear_constraint_with_metadata(
            coefficients,
            ConstraintRelation::LessEqual,
            rhs,
            name,
            group,
            lazy,
            priority,
            args,
        )
    }

    pub fn add_ge_constraint_with_metadata(
        &mut self,
        coefficients: &[(usize, f64)],
        rhs: f64,
        name: &str,
        group: Option<Arc<ConstraintGroup>>,
        lazy: bool,
        priority: u32,
        args: Option<String>,
    ) -> Result<()> {
        self.add_linear_constraint_with_metadata(
            coefficients,
            ConstraintRelation::GreaterEqual,
            rhs,
            name,
            group,
            lazy,
            priority,
            args,
        )
    }

    pub fn add_eq_constraint_with_metadata(
        &mut self,
        coefficients: &[(usize, f64)],
        rhs: f64,
        name: &str,
        group: Option<Arc<ConstraintGroup>>,
        lazy: bool,
        priority: u32,
        args: Option<String>,
    ) -> Result<()> {
        self.add_linear_constraint_with_metadata(
            coefficients,
            ConstraintRelation::Equal,
            rhs,
            name,
            group,
            lazy,
            priority,
            args,
        )
    }

    pub fn partition_linear_coefficients(
        &mut self,
        coefficients: &[(usize, f64)],
        name: &str,
    ) -> Result<()> {
        self.add_eq_constraint(coefficients, 1.0, name)
    }

    pub fn partition_linear_coefficients_with_metadata(
        &mut self,
        coefficients: &[(usize, f64)],
        name: &str,
        group: Option<Arc<ConstraintGroup>>,
        lazy: bool,
        priority: u32,
        args: Option<String>,
    ) -> Result<()> {
        self.add_eq_constraint_with_metadata(coefficients, 1.0, name, group, lazy, priority, args)
    }

    pub fn partition_linear_indices(&mut self, indices: &[usize], name: &str) -> Result<()> {
        let coefficients: Vec<(usize, f64)> =
            indices.iter().copied().map(|idx| (idx, 1.0)).collect();
        self.partition_linear_coefficients(&coefficients, name)
    }

    pub fn partition_linear_indices_with_metadata(
        &mut self,
        indices: &[usize],
        name: &str,
        group: Option<Arc<ConstraintGroup>>,
        lazy: bool,
        priority: u32,
        args: Option<String>,
    ) -> Result<()> {
        let coefficients: Vec<(usize, f64)> =
            indices.iter().copied().map(|idx| (idx, 1.0)).collect();
        self.partition_linear_coefficients_with_metadata(
            &coefficients,
            name,
            group,
            lazy,
            priority,
            args,
        )
    }
}

impl<V> std::ops::Deref for MetaModel<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Target = BasicModel<V>;

    fn deref(&self) -> &Self::Target {
        &self.basic
    }
}

impl<V> std::ops::DerefMut for MetaModel<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.basic
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use crate::error::Result;
    use crate::flatten::{Linear, LinearMonomial};
    use crate::model::intermediate::{LinearTriadModel, QuadraticTetradModel};
    use crate::model::{
        ConstraintRelation, LinearConstraint, LinearInequality, MetaConstraint, ModelBuildingStage,
        ModelBuildingStatusCallback,
    };
    use crate::solver::{
        LinearSolver, QuadraticSolver, SolverCapability, SolverInfo, SolverOutput, SolverStatus,
        SolvingStatusCallback,
    };
    use crate::symbol::functions::{
        AbsFunction, AndFunction, BalanceTernaryzationFunction, BinaryzationFunction, CosFunction,
        FirstFunction, IfElseFunction, InStepRangeFunction, InequalityFunction, InequalityKind,
        MaxFunction, MinFunction, ModFunction, NotFunction, OneOfFunction, OrFunction,
        RoundingFunction, SameAsFunction, SatisfiedAmountFunction, SemiFunction, SigmoidFunction,
        SinFunction, XorFunction,
    };
    use crate::variable::{BinaryVariableItem, ContinuousVariableItem, VariableId, VariableRange};

    use super::MetaModel;

    fn lhs_value(constraint: &LinearConstraint<f64>, values: &HashMap<usize, f64>) -> f64 {
        let mut lhs = *constraint.inequality.polynomial.constant_term();
        for monomial in constraint.inequality.polynomial.monomials() {
            let value = values.get(&monomial.var_index()).copied().unwrap_or(0.0);
            lhs += *monomial.coefficient() * value;
        }
        lhs
    }

    fn satisfies_constraint(
        constraint: &LinearConstraint<f64>,
        values: &HashMap<usize, f64>,
    ) -> bool {
        const EPS: f64 = 1e-9;
        let lhs = lhs_value(constraint, values);
        let rhs = constraint.inequality.rhs;
        match constraint.inequality.relation {
            ConstraintRelation::LessEqual => lhs <= rhs + EPS,
            ConstraintRelation::Equal => (lhs - rhs).abs() <= EPS,
            ConstraintRelation::GreaterEqual => lhs + EPS >= rhs,
        }
    }

    #[test]
    fn binaryzation_constraints_are_injected_into_mechanism_model() {
        let mut model = MetaModel::<f64>::new("binaryzation_injection");
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(1),
            "x",
            VariableRange::bounded(0.0, 2.0),
        );
        let x_index = model.register_variable(x).unwrap();

        let input = Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0);
        let binary = BinaryzationFunction::with_big_m(100, "bin", input, 10.0);
        let y_id = binary.result_variable().id();
        model.add_symbol(Arc::new(binary)).unwrap();

        let mechanism = model.try_into_mechanism_model().unwrap();
        let generated_constraints = mechanism
            .constraints()
            .iter()
            .filter(|constraint| {
                constraint
                    .from
                    .as_ref()
                    .map(|symbol| symbol.id().id == 100)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        assert_eq!(generated_constraints.len(), 2);

        let x_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "x")
            .map(|token| token.solver_index)
            .unwrap();
        let y_solver_index = mechanism.find_token(y_id).unwrap().solver_index;

        let feasible_zero = HashMap::from([(x_solver_index, 0.0), (y_solver_index, 0.0)]);
        assert!(
            generated_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &feasible_zero))
        );

        let feasible_positive = HashMap::from([(x_solver_index, 1.0), (y_solver_index, 1.0)]);
        assert!(
            generated_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &feasible_positive))
        );

        let infeasible_positive_zero =
            HashMap::from([(x_solver_index, 1.0), (y_solver_index, 0.0)]);
        assert!(
            generated_constraints
                .iter()
                .any(|constraint| !satisfies_constraint(constraint, &infeasible_positive_zero))
        );
    }

    #[test]
    fn inequality_constraints_are_injected_into_mechanism_model() {
        let mut model = MetaModel::<f64>::new("inequality_injection");
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(2),
            "x",
            VariableRange::bounded(0.0, 2.0),
        );
        let x_index = model.register_variable(x).unwrap();

        let left = Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0);
        let inequality = InequalityFunction::less_equal(200, "le", left, 1.0, 10.0);
        let y_id = inequality.result_variable().id();
        model.add_symbol(Arc::new(inequality)).unwrap();

        let mechanism = model.try_into_mechanism_model().unwrap();
        let generated_constraints = mechanism
            .constraints()
            .iter()
            .filter(|constraint| {
                constraint
                    .from
                    .as_ref()
                    .map(|symbol| symbol.id().id == 200)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        assert_eq!(generated_constraints.len(), 2);

        let x_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "x")
            .map(|token| token.solver_index)
            .unwrap();
        let y_solver_index = mechanism.find_token(y_id).unwrap().solver_index;

        let feasible_less_equal = HashMap::from([(x_solver_index, 0.5), (y_solver_index, 1.0)]);
        assert!(
            generated_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &feasible_less_equal))
        );

        let feasible_greater = HashMap::from([(x_solver_index, 1.5), (y_solver_index, 0.0)]);
        assert!(
            generated_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &feasible_greater))
        );

        let infeasible_flag = HashMap::from([(x_solver_index, 1.5), (y_solver_index, 1.0)]);
        assert!(
            generated_constraints
                .iter()
                .any(|constraint| !satisfies_constraint(constraint, &infeasible_flag))
        );
    }

    #[test]
    fn inequality_equal_constraints_are_injected_into_mechanism_model() {
        let mut model = MetaModel::<f64>::new("inequality_equal_injection");
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(21),
            "x",
            VariableRange::bounded(0.0, 2.0),
        );
        let x_index = model.register_variable(x).unwrap();

        let left = Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0);
        let inequality = InequalityFunction::new(201, "eq", left, 1.0, InequalityKind::Equal, 10.0);
        let y_id = inequality.result_variable().id();
        model.add_symbol(Arc::new(inequality)).unwrap();

        let mechanism = model.try_into_mechanism_model().unwrap();
        let generated_constraints = mechanism
            .constraints()
            .iter()
            .filter(|constraint| {
                constraint
                    .from
                    .as_ref()
                    .map(|symbol| symbol.id().id == 201)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        assert_eq!(generated_constraints.len(), 4);

        let x_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "x")
            .map(|token| token.solver_index)
            .unwrap();
        let y_solver_index = mechanism.find_token(y_id).unwrap().solver_index;
        let side_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "eq_side")
            .map(|token| token.solver_index)
            .unwrap();

        let feasible_equal = HashMap::from([
            (x_solver_index, 1.0),
            (y_solver_index, 1.0),
            (side_solver_index, 0.0),
        ]);
        assert!(
            generated_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &feasible_equal))
        );

        let feasible_not_equal = HashMap::from([
            (x_solver_index, 1.2),
            (y_solver_index, 0.0),
            (side_solver_index, 1.0),
        ]);
        assert!(
            generated_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &feasible_not_equal))
        );

        let infeasible_wrong_flag = HashMap::from([
            (x_solver_index, 1.2),
            (y_solver_index, 1.0),
            (side_solver_index, 0.0),
        ]);
        assert!(
            generated_constraints
                .iter()
                .any(|constraint| !satisfies_constraint(constraint, &infeasible_wrong_flag))
        );
    }

    #[test]
    fn inequality_not_equal_constraints_are_injected_into_mechanism_model() {
        let mut model = MetaModel::<f64>::new("inequality_not_equal_injection");
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(22),
            "x",
            VariableRange::bounded(0.0, 2.0),
        );
        let x_index = model.register_variable(x).unwrap();

        let left = Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0);
        let inequality =
            InequalityFunction::new(202, "neq", left, 1.0, InequalityKind::NotEqual, 10.0);
        let y_id = inequality.result_variable().id();
        model.add_symbol(Arc::new(inequality)).unwrap();

        let mechanism = model.try_into_mechanism_model().unwrap();
        let generated_constraints = mechanism
            .constraints()
            .iter()
            .filter(|constraint| {
                constraint
                    .from
                    .as_ref()
                    .map(|symbol| symbol.id().id == 202)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        assert_eq!(generated_constraints.len(), 4);

        let x_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "x")
            .map(|token| token.solver_index)
            .unwrap();
        let y_solver_index = mechanism.find_token(y_id).unwrap().solver_index;
        let side_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "neq_side")
            .map(|token| token.solver_index)
            .unwrap();

        let feasible_equal = HashMap::from([
            (x_solver_index, 1.0),
            (y_solver_index, 0.0),
            (side_solver_index, 0.0),
        ]);
        assert!(
            generated_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &feasible_equal))
        );

        let feasible_not_equal = HashMap::from([
            (x_solver_index, 1.2),
            (y_solver_index, 1.0),
            (side_solver_index, 1.0),
        ]);
        assert!(
            generated_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &feasible_not_equal))
        );

        let infeasible_wrong_flag = HashMap::from([
            (x_solver_index, 0.5),
            (y_solver_index, 1.0),
            (side_solver_index, 1.0),
        ]);
        assert!(
            generated_constraints
                .iter()
                .any(|constraint| !satisfies_constraint(constraint, &infeasible_wrong_flag))
        );
    }

    #[test]
    fn same_as_constraints_are_injected_into_mechanism_model() {
        let mut model = MetaModel::<f64>::new("same_as_injection");
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(23),
            "x",
            VariableRange::bounded(0.0, 2.0),
        );
        let x_index = model.register_variable(x).unwrap();

        let same_as = SameAsFunction::new(
            203,
            "same",
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            Linear::new(vec![], 1.0),
            0.01,
        );
        let y_id = same_as.result_variable().id();
        model.add_symbol(Arc::new(same_as)).unwrap();

        let mechanism = model.try_into_mechanism_model().unwrap();
        let generated_constraints = mechanism
            .constraints()
            .iter()
            .filter(|constraint| {
                constraint
                    .from
                    .as_ref()
                    .map(|symbol| symbol.id().id == 203)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        assert_eq!(generated_constraints.len(), 4);

        let x_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "x")
            .map(|token| token.solver_index)
            .unwrap();
        let y_solver_index = mechanism.find_token(y_id).unwrap().solver_index;
        let side_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "same_side")
            .map(|token| token.solver_index)
            .unwrap();

        let feasible_equal = HashMap::from([
            (x_solver_index, 1.005),
            (y_solver_index, 1.0),
            (side_solver_index, 0.0),
        ]);
        assert!(
            generated_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &feasible_equal))
        );

        let feasible_not_equal = HashMap::from([
            (x_solver_index, 1.1),
            (y_solver_index, 0.0),
            (side_solver_index, 1.0),
        ]);
        assert!(
            generated_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &feasible_not_equal))
        );

        let infeasible_wrong_flag = HashMap::from([
            (x_solver_index, 1.1),
            (y_solver_index, 1.0),
            (side_solver_index, 0.0),
        ]);
        assert!(
            generated_constraints
                .iter()
                .any(|constraint| !satisfies_constraint(constraint, &infeasible_wrong_flag))
        );
    }

    #[test]
    fn one_of_constraints_are_injected_into_mechanism_model() {
        let mut model = MetaModel::<f64>::new("one_of_injection");
        let one_of = OneOfFunction::new(
            300,
            "one_of",
            vec![Linear::new(vec![], 5.0), Linear::new(vec![], 8.0)],
        );
        let y_id = one_of.result_variable().id();
        model.add_symbol(Arc::new(one_of)).unwrap();

        let mechanism = model.try_into_mechanism_model().unwrap();
        let generated_constraints = mechanism
            .constraints()
            .iter()
            .filter(|constraint| {
                constraint
                    .from
                    .as_ref()
                    .map(|symbol| symbol.id().id == 300)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        assert_eq!(generated_constraints.len(), 7);

        let y_solver_index = mechanism.find_token(y_id).unwrap().solver_index;
        let sel0_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "one_of_sel0")
            .map(|token| token.solver_index)
            .unwrap();
        let sel1_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "one_of_sel1")
            .map(|token| token.solver_index)
            .unwrap();

        let feasible_none = HashMap::from([
            (y_solver_index, 0.0),
            (sel0_solver_index, 0.0),
            (sel1_solver_index, 0.0),
        ]);
        assert!(
            generated_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &feasible_none))
        );

        let feasible_select_first = HashMap::from([
            (y_solver_index, 5.0),
            (sel0_solver_index, 1.0),
            (sel1_solver_index, 0.0),
        ]);
        assert!(
            generated_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &feasible_select_first))
        );

        let feasible_select_second = HashMap::from([
            (y_solver_index, 8.0),
            (sel0_solver_index, 0.0),
            (sel1_solver_index, 1.0),
        ]);
        assert!(
            generated_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &feasible_select_second))
        );

        let infeasible_wrong_result = HashMap::from([
            (y_solver_index, 8.0),
            (sel0_solver_index, 1.0),
            (sel1_solver_index, 0.0),
        ]);
        assert!(
            generated_constraints
                .iter()
                .any(|constraint| !satisfies_constraint(constraint, &infeasible_wrong_result))
        );
    }

    #[test]
    fn if_else_constraints_are_injected_into_mechanism_model() {
        let mut model = MetaModel::<f64>::new("if_else_injection");
        let condition = BinaryVariableItem::create(VariableId::standalone(401), "if_else_cond");
        model.register_variable(condition.clone()).unwrap();

        let if_else = IfElseFunction::new(
            400,
            "if_else",
            condition.clone(),
            Linear::new(vec![], 10.0),
            Linear::new(vec![], 20.0),
        );
        let y_id = if_else.result_variable().id();
        model.add_symbol(Arc::new(if_else)).unwrap();

        let mechanism = model.try_into_mechanism_model().unwrap();
        let generated_constraints = mechanism
            .constraints()
            .iter()
            .filter(|constraint| {
                constraint
                    .from
                    .as_ref()
                    .map(|symbol| symbol.id().id == 400)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        assert_eq!(generated_constraints.len(), 4);

        let y_solver_index = mechanism.find_token(y_id).unwrap().solver_index;
        let condition_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "if_else_cond")
            .map(|token| token.solver_index)
            .unwrap();

        let feasible_then = HashMap::from([(y_solver_index, 10.0), (condition_solver_index, 1.0)]);
        assert!(
            generated_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &feasible_then))
        );

        let feasible_else = HashMap::from([(y_solver_index, 20.0), (condition_solver_index, 0.0)]);
        assert!(
            generated_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &feasible_else))
        );

        let infeasible_branch =
            HashMap::from([(y_solver_index, 20.0), (condition_solver_index, 1.0)]);
        assert!(
            generated_constraints
                .iter()
                .any(|constraint| !satisfies_constraint(constraint, &infeasible_branch))
        );
    }

    #[test]
    fn first_constraints_are_injected_into_mechanism_model() {
        let mut model = MetaModel::<f64>::new("first_injection");
        let cond0 = BinaryVariableItem::create(VariableId::standalone(502), "first_cond0");
        let cond1 = BinaryVariableItem::create(VariableId::standalone(503), "first_cond1");
        let first = FirstFunction::new(
            500,
            "first_case",
            vec![Linear::new(vec![], 3.0), Linear::new(vec![], 7.0)],
            vec![cond0, cond1],
        );
        let y_id = first.result_variable().id();
        model.add_symbol(Arc::new(first)).unwrap();

        let mechanism = model.try_into_mechanism_model().unwrap();
        let generated_constraints = mechanism
            .constraints()
            .iter()
            .filter(|constraint| {
                constraint
                    .from
                    .as_ref()
                    .map(|symbol| symbol.id().id == 500)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        assert_eq!(generated_constraints.len(), 6);

        let y_solver_index = mechanism.find_token(y_id).unwrap().solver_index;
        let cond0_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "first_cond0")
            .map(|token| token.solver_index)
            .unwrap();
        let cond1_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "first_cond1")
            .map(|token| token.solver_index)
            .unwrap();

        let feasible_none = HashMap::from([
            (y_solver_index, 0.0),
            (cond0_solver_index, 0.0),
            (cond1_solver_index, 0.0),
        ]);
        assert!(
            generated_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &feasible_none))
        );

        let feasible_first = HashMap::from([
            (y_solver_index, 3.0),
            (cond0_solver_index, 1.0),
            (cond1_solver_index, 0.0),
        ]);
        assert!(
            generated_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &feasible_first))
        );

        let feasible_second = HashMap::from([
            (y_solver_index, 7.0),
            (cond0_solver_index, 0.0),
            (cond1_solver_index, 1.0),
        ]);
        assert!(
            generated_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &feasible_second))
        );

        let feasible_first_wins = HashMap::from([
            (y_solver_index, 3.0),
            (cond0_solver_index, 1.0),
            (cond1_solver_index, 1.0),
        ]);
        assert!(
            generated_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &feasible_first_wins))
        );

        let infeasible_wrong_first = HashMap::from([
            (y_solver_index, 7.0),
            (cond0_solver_index, 1.0),
            (cond1_solver_index, 1.0),
        ]);
        assert!(
            generated_constraints
                .iter()
                .any(|constraint| !satisfies_constraint(constraint, &infeasible_wrong_first))
        );
    }

    #[test]
    fn balance_ternary_constraints_are_injected_into_mechanism_model() {
        let mut model = MetaModel::<f64>::new("balance_ternary_injection");
        let bal = BalanceTernaryzationFunction::new(700, "bal");
        let y_id = bal.result_variable().id();
        model.add_symbol(Arc::new(bal)).unwrap();

        let mechanism = model.try_into_mechanism_model().unwrap();
        let generated_constraints = mechanism
            .constraints()
            .iter()
            .filter(|constraint| {
                constraint
                    .from
                    .as_ref()
                    .map(|symbol| symbol.id().id == 700)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        assert_eq!(generated_constraints.len(), 2);

        let y_solver_index = mechanism.find_token(y_id).unwrap().solver_index;
        let pos_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "bal_pos")
            .map(|token| token.solver_index)
            .unwrap();
        let neg_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "bal_neg")
            .map(|token| token.solver_index)
            .unwrap();

        let feasible_zero = HashMap::from([
            (y_solver_index, 0.0),
            (pos_solver_index, 0.0),
            (neg_solver_index, 0.0),
        ]);
        assert!(
            generated_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &feasible_zero))
        );

        let feasible_positive = HashMap::from([
            (y_solver_index, 1.0),
            (pos_solver_index, 1.0),
            (neg_solver_index, 0.0),
        ]);
        assert!(
            generated_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &feasible_positive))
        );

        let feasible_negative = HashMap::from([
            (y_solver_index, -1.0),
            (pos_solver_index, 0.0),
            (neg_solver_index, 1.0),
        ]);
        assert!(
            generated_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &feasible_negative))
        );

        let infeasible_conflict = HashMap::from([
            (y_solver_index, 1.0),
            (pos_solver_index, 1.0),
            (neg_solver_index, 1.0),
        ]);
        assert!(
            generated_constraints
                .iter()
                .any(|constraint| !satisfies_constraint(constraint, &infeasible_conflict))
        );
    }

    #[test]
    fn semi_constraints_are_injected_into_mechanism_model() {
        let mut model = MetaModel::<f64>::new("semi_injection");
        let semi = SemiFunction::new(710, "semi", 2.0, 5.0);
        let y_id = semi.result_variable().id();
        model.add_symbol(Arc::new(semi)).unwrap();

        let mechanism = model.try_into_mechanism_model().unwrap();
        let generated_constraints = mechanism
            .constraints()
            .iter()
            .filter(|constraint| {
                constraint
                    .from
                    .as_ref()
                    .map(|symbol| symbol.id().id == 710)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        assert_eq!(generated_constraints.len(), 2);

        let y_solver_index = mechanism.find_token(y_id).unwrap().solver_index;
        let ind_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "semi_ind")
            .map(|token| token.solver_index)
            .unwrap();

        let feasible_off = HashMap::from([(y_solver_index, 0.0), (ind_solver_index, 0.0)]);
        assert!(
            generated_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &feasible_off))
        );

        let feasible_on = HashMap::from([(y_solver_index, 3.0), (ind_solver_index, 1.0)]);
        assert!(
            generated_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &feasible_on))
        );

        let infeasible_below_lower =
            HashMap::from([(y_solver_index, 1.0), (ind_solver_index, 1.0)]);
        assert!(
            generated_constraints
                .iter()
                .any(|constraint| !satisfies_constraint(constraint, &infeasible_below_lower))
        );
    }

    #[test]
    fn satisfied_amount_constraints_are_injected_into_mechanism_model() {
        let mut model = MetaModel::<f64>::new("satisfied_amount_injection");
        let ind0 = BinaryVariableItem::create(VariableId::standalone(801), "sat_ind0");
        let ind1 = BinaryVariableItem::create(VariableId::standalone(802), "sat_ind1");
        let sat = SatisfiedAmountFunction::new(800, "sat", vec![ind0, ind1]);
        let y_id = sat.result_variable().id();
        model.add_symbol(Arc::new(sat)).unwrap();

        let mechanism = model.try_into_mechanism_model().unwrap();
        let generated_constraints = mechanism
            .constraints()
            .iter()
            .filter(|constraint| {
                constraint
                    .from
                    .as_ref()
                    .map(|symbol| symbol.id().id == 800)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        assert_eq!(generated_constraints.len(), 1);

        let y_solver_index = mechanism.find_token(y_id).unwrap().solver_index;
        let ind0_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "sat_ind0")
            .map(|token| token.solver_index)
            .unwrap();
        let ind1_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "sat_ind1")
            .map(|token| token.solver_index)
            .unwrap();

        let feasible_none = HashMap::from([
            (y_solver_index, 0.0),
            (ind0_solver_index, 0.0),
            (ind1_solver_index, 0.0),
        ]);
        assert!(
            generated_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &feasible_none))
        );

        let feasible_two = HashMap::from([
            (y_solver_index, 2.0),
            (ind0_solver_index, 1.0),
            (ind1_solver_index, 1.0),
        ]);
        assert!(
            generated_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &feasible_two))
        );

        let infeasible_count = HashMap::from([
            (y_solver_index, 1.0),
            (ind0_solver_index, 1.0),
            (ind1_solver_index, 1.0),
        ]);
        assert!(
            generated_constraints
                .iter()
                .any(|constraint| !satisfies_constraint(constraint, &infeasible_count))
        );
    }

    #[test]
    fn in_step_range_constraints_are_injected_into_mechanism_model() {
        let mut model = MetaModel::<f64>::new("in_step_range_injection");
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(860),
            "x_step",
            VariableRange::bounded(-1.0, 3.0),
        );
        let x_index = model.register_variable(x).unwrap();
        let in_step = InStepRangeFunction::new(
            861,
            "in_step",
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            0.0,
            2.0,
            1.0,
        );
        let y_id = in_step.result_variable().id();
        model.add_symbol(Arc::new(in_step)).unwrap();

        let mechanism = model.try_into_mechanism_model().unwrap();
        let generated_constraints = mechanism
            .constraints()
            .iter()
            .filter(|constraint| {
                constraint
                    .from
                    .as_ref()
                    .map(|symbol| symbol.id().id == 861)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        assert_eq!(generated_constraints.len(), 16);

        let x_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "x_step")
            .map(|token| token.solver_index)
            .unwrap();
        let y_solver_index = mechanism.find_token(y_id).unwrap().solver_index;
        let pt0_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "in_step_step_pt0")
            .map(|token| token.solver_index)
            .unwrap();
        let pt1_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "in_step_step_pt1")
            .map(|token| token.solver_index)
            .unwrap();
        let pt2_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "in_step_step_pt2")
            .map(|token| token.solver_index)
            .unwrap();
        let side0_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "in_step_step_side0")
            .map(|token| token.solver_index)
            .unwrap();
        let side1_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "in_step_step_side1")
            .map(|token| token.solver_index)
            .unwrap();
        let side2_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "in_step_step_side2")
            .map(|token| token.solver_index)
            .unwrap();

        let feasible_on_step = HashMap::from([
            (x_solver_index, 1.0),
            (y_solver_index, 1.0),
            (pt0_solver_index, 0.0),
            (pt1_solver_index, 1.0),
            (pt2_solver_index, 0.0),
            (side0_solver_index, 1.0),
            (side1_solver_index, 0.0),
            (side2_solver_index, 0.0),
        ]);
        assert!(
            generated_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &feasible_on_step))
        );

        let feasible_off_step = HashMap::from([
            (x_solver_index, 0.5),
            (y_solver_index, 0.0),
            (pt0_solver_index, 0.0),
            (pt1_solver_index, 0.0),
            (pt2_solver_index, 0.0),
            (side0_solver_index, 1.0),
            (side1_solver_index, 0.0),
            (side2_solver_index, 0.0),
        ]);
        assert!(
            generated_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &feasible_off_step))
        );

        let infeasible_wrong_activation = HashMap::from([
            (x_solver_index, 0.5),
            (y_solver_index, 1.0),
            (pt0_solver_index, 0.0),
            (pt1_solver_index, 0.0),
            (pt2_solver_index, 0.0),
            (side0_solver_index, 1.0),
            (side1_solver_index, 0.0),
            (side2_solver_index, 0.0),
        ]);
        assert!(
            generated_constraints
                .iter()
                .any(|constraint| !satisfies_constraint(constraint, &infeasible_wrong_activation))
        );
    }

    #[test]
    fn min_and_max_constraints_are_injected_into_mechanism_model() {
        let mut model = MetaModel::<f64>::new("min_max_injection");
        let min_fn = MinFunction::new(
            900,
            "min_exact",
            vec![Linear::new(vec![], 3.0), Linear::new(vec![], 7.0)],
            true,
        );
        let min_y_id = min_fn.result_variable().id();
        model.add_symbol(Arc::new(min_fn)).unwrap();

        let max_fn = MaxFunction::new(
            910,
            "max_exact",
            vec![Linear::new(vec![], 3.0), Linear::new(vec![], 7.0)],
            true,
        );
        let max_y_id = max_fn.result_variable().id();
        model.add_symbol(Arc::new(max_fn)).unwrap();

        let mechanism = model.try_into_mechanism_model().unwrap();

        let min_constraints = mechanism
            .constraints()
            .iter()
            .filter(|constraint| {
                constraint
                    .from
                    .as_ref()
                    .map(|symbol| symbol.id().id == 900)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        assert_eq!(min_constraints.len(), 5);

        let min_y_solver_index = mechanism.find_token(min_y_id).unwrap().solver_index;
        let min_u0_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "min_exact_u0")
            .map(|token| token.solver_index)
            .unwrap();
        let min_u1_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "min_exact_u1")
            .map(|token| token.solver_index)
            .unwrap();

        let min_feasible_first = HashMap::from([
            (min_y_solver_index, 3.0),
            (min_u0_solver_index, 1.0),
            (min_u1_solver_index, 0.0),
        ]);
        assert!(
            min_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &min_feasible_first))
        );

        let min_infeasible_wrong = HashMap::from([
            (min_y_solver_index, 5.0),
            (min_u0_solver_index, 1.0),
            (min_u1_solver_index, 0.0),
        ]);
        assert!(
            min_constraints
                .iter()
                .any(|constraint| !satisfies_constraint(constraint, &min_infeasible_wrong))
        );

        let max_constraints = mechanism
            .constraints()
            .iter()
            .filter(|constraint| {
                constraint
                    .from
                    .as_ref()
                    .map(|symbol| symbol.id().id == 910)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        assert_eq!(max_constraints.len(), 5);

        let max_y_solver_index = mechanism.find_token(max_y_id).unwrap().solver_index;
        let max_u0_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "max_exact_u0")
            .map(|token| token.solver_index)
            .unwrap();
        let max_u1_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "max_exact_u1")
            .map(|token| token.solver_index)
            .unwrap();

        let max_feasible_second = HashMap::from([
            (max_y_solver_index, 7.0),
            (max_u0_solver_index, 0.0),
            (max_u1_solver_index, 1.0),
        ]);
        assert!(
            max_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &max_feasible_second))
        );

        let max_infeasible_wrong = HashMap::from([
            (max_y_solver_index, 3.0),
            (max_u0_solver_index, 0.0),
            (max_u1_solver_index, 1.0),
        ]);
        assert!(
            max_constraints
                .iter()
                .any(|constraint| !satisfies_constraint(constraint, &max_infeasible_wrong))
        );
    }

    #[test]
    fn meta_constraint_metadata_is_propagated_to_mechanism_and_intermediate_models() {
        let mut model = MetaModel::<f64>::new("meta_constraint_metadata");
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(599),
            "x_meta",
            VariableRange::bounded(-10.0, 10.0),
        );
        let x_index = model.register_variable(x).unwrap();

        let group = model.create_constraint_group(700, "meta_group").unwrap();
        let mut constraint = MetaConstraint::new(
            LinearInequality::new(
                Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
                ConstraintRelation::LessEqual,
                2.0,
            ),
            "meta_with_fields",
        )
        .with_group(group)
        .with_priority(9)
        .with_args("{\"tag\":\"p0\"}");
        constraint.set_lazy(true);
        model.add_constraint(constraint).unwrap();

        let mechanism = model.try_into_mechanism_model().unwrap();
        let constraint = mechanism
            .constraints()
            .iter()
            .find(|item| item.name == "meta_with_fields")
            .unwrap();
        assert_eq!(constraint.group.as_ref().map(|group| group.id), Some(700));
        assert!(constraint.lazy);
        assert_eq!(constraint.priority, 9);
        assert_eq!(constraint.args.as_deref(), Some("{\"tag\":\"p0\"}"));

        let linear = mechanism.into_linear_triad_model();
        assert_eq!(linear.basic.constraint_names.len(), 1);
        assert_eq!(linear.basic.constraint_group_ids[0], Some(700));
        assert!(linear.basic.constraint_lazy_flags[0]);
        assert_eq!(linear.basic.constraint_priorities[0], 9);
        assert_eq!(
            linear.basic.constraint_args[0].as_deref(),
            Some("{\"tag\":\"p0\"}")
        );
        assert_eq!(linear.basic.constraint_source_symbol_ids[0], None);
    }

    #[test]
    fn meta_constraint_metadata_defaults_are_optional() {
        let mut model = MetaModel::<f64>::new("meta_constraint_defaults");
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(598),
            "x_meta_default",
            VariableRange::bounded(-10.0, 10.0),
        );
        let x_index = model.register_variable(x).unwrap();

        let constraint = MetaConstraint::new(
            LinearInequality::new(
                Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
                ConstraintRelation::LessEqual,
                3.0,
            ),
            "meta_defaults",
        );
        model.add_constraint(constraint).unwrap();

        let mechanism = model.try_into_mechanism_model().unwrap();
        let constraint = mechanism
            .constraints()
            .iter()
            .find(|item| item.name == "meta_defaults")
            .unwrap();
        assert!(constraint.group.is_none());
        assert!(!constraint.lazy);
        assert_eq!(constraint.priority, 0);
        assert!(constraint.args.is_none());
    }

    #[test]
    fn partition_shortcut_with_metadata_is_propagated() {
        let mut model = MetaModel::<f64>::new("meta_partition_shortcut");
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(597),
            "x_partition",
            VariableRange::bounded(0.0, 1.0),
        );
        let y = ContinuousVariableItem::with_range(
            VariableId::standalone(596),
            "y_partition",
            VariableRange::bounded(0.0, 1.0),
        );
        let x_index = model.register_variable(x).unwrap();
        let y_index = model.register_variable(y).unwrap();

        let group = model
            .create_constraint_group(701, "partition_group")
            .unwrap();
        model
            .partition_linear_indices_with_metadata(
                &[x_index, y_index],
                "partition_xy",
                Some(group),
                true,
                3,
                Some("{\"kind\":\"partition\"}".to_string()),
            )
            .unwrap();

        let mechanism = model.try_into_mechanism_model().unwrap();
        let constraint = mechanism
            .constraints()
            .iter()
            .find(|item| item.name == "partition_xy")
            .unwrap();
        assert_eq!(constraint.group.as_ref().map(|group| group.id), Some(701));
        assert!(constraint.lazy);
        assert_eq!(constraint.priority, 3);
        assert_eq!(constraint.args.as_deref(), Some("{\"kind\":\"partition\"}"));
        assert_eq!(constraint.inequality.relation, ConstraintRelation::Equal);
        assert_eq!(constraint.inequality.rhs, 1.0);

        let lhs_at_x1_y0 = lhs_value(constraint, &HashMap::from([(x_index, 1.0), (y_index, 0.0)]));
        assert!((lhs_at_x1_y0 - 1.0).abs() <= 1e-9);
    }

    #[test]
    fn add_symbols_shortcut_registers_batch_function_symbols() {
        let mut model = MetaModel::<f64>::new("meta_add_symbols_batch");
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(595),
            "x_batch",
            VariableRange::bounded(-3.0, 4.0),
        );
        let x_index = model.register_variable(x).unwrap();

        let abs = Arc::new(AbsFunction::new(
            606,
            "abs_batch",
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
        ));
        let abs_result_id = abs.result_variable().id();
        let abs_side_id = abs.side_variable().id();
        model
            .add_symbols(vec![abs as Arc<dyn crate::symbol::IntermediateSymbol<f64>>])
            .unwrap();

        let mechanism = model.try_into_mechanism_model().unwrap();
        assert!(mechanism.find_token(abs_result_id).is_some());
        assert!(mechanism.find_token(abs_side_id).is_some());

        let generated_constraints = mechanism
            .constraints()
            .iter()
            .filter(|constraint| {
                constraint
                    .from
                    .as_ref()
                    .map(|symbol| symbol.id().id == 606)
                    .unwrap_or(false)
            })
            .count();
        assert_eq!(generated_constraints, 4);
    }

    #[test]
    fn trigonometric_constraints_are_injected_into_mechanism_model() {
        let mut model = MetaModel::<f64>::new("trigonometric_injection");
        let sin_fn = SinFunction::new(600, "sin_piecewise", Linear::new(vec![], 0.0));
        let sin_id = sin_fn.result_variable().id();
        model.add_symbol(Arc::new(sin_fn)).unwrap();

        let cos_fn = CosFunction::new(601, "cos_piecewise", Linear::new(vec![], 0.0));
        let cos_id = cos_fn.result_variable().id();
        model.add_symbol(Arc::new(cos_fn)).unwrap();

        let mechanism = model.try_into_mechanism_model().unwrap();

        let sin_constraints = mechanism
            .constraints()
            .iter()
            .filter(|constraint| {
                constraint
                    .from
                    .as_ref()
                    .map(|symbol| symbol.id().id == 600)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        let cos_constraints = mechanism
            .constraints()
            .iter()
            .filter(|constraint| {
                constraint
                    .from
                    .as_ref()
                    .map(|symbol| symbol.id().id == 601)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();

        assert_eq!(sin_constraints.len(), 71);
        assert_eq!(cos_constraints.len(), 71);
        assert!(mechanism.find_token(sin_id).is_some());
        assert!(mechanism.find_token(cos_id).is_some());
        let sin_token = mechanism.find_token(sin_id).unwrap();
        assert_eq!(sin_token.variable.lower_bound(), Some(-1.0));
        assert_eq!(sin_token.variable.upper_bound(), Some(1.0));
        let cos_token = mechanism.find_token(cos_id).unwrap();
        assert_eq!(cos_token.variable.lower_bound(), Some(-1.0));
        assert_eq!(cos_token.variable.upper_bound(), Some(1.0));
        assert!(
            mechanism
                .tokens()
                .iter()
                .any(|token| token.name() == "sin_piecewise_sin_z0")
        );
        assert!(
            mechanism
                .tokens()
                .iter()
                .any(|token| token.name() == "cos_piecewise_cos_z0")
        );
    }

    #[test]
    fn sigmoid_constraints_are_tightened_with_segment_binaries() {
        let mut model = MetaModel::<f64>::new("sigmoid_injection");
        let sigmoid_fn = SigmoidFunction::new(602, "sigmoid_piece", Linear::new(vec![], 0.0));
        let sigmoid_id = sigmoid_fn.result_variable().id();
        model.add_symbol(Arc::new(sigmoid_fn)).unwrap();

        let mechanism = model.try_into_mechanism_model().unwrap();
        let sigmoid_constraints = mechanism
            .constraints()
            .iter()
            .filter(|constraint| {
                constraint
                    .from
                    .as_ref()
                    .map(|symbol| symbol.id().id == 602)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        assert!(!sigmoid_constraints.is_empty());
        assert!(
            sigmoid_constraints
                .iter()
                .any(|constraint| constraint.name.contains("_sigmoid_seg_sum"))
        );
        assert!(
            sigmoid_constraints
                .iter()
                .any(|constraint| constraint.name.contains("_sigmoid_lambda_link_"))
        );
        assert!(
            sigmoid_constraints
                .iter()
                .any(|constraint| constraint.name.contains("_sigmoid_y_ub"))
        );
        assert!(
            sigmoid_constraints
                .iter()
                .any(|constraint| constraint.name.contains("_sigmoid_y_lb"))
        );

        let sigmoid_token = mechanism.find_token(sigmoid_id).unwrap();
        assert!(sigmoid_token.variable.lower_bound().is_some());
        assert!(sigmoid_token.variable.upper_bound().is_some());
        assert!(
            mechanism
                .tokens()
                .iter()
                .any(|token| token.name() == "sigmoid_piece_sigmoid_b0")
        );
    }

    #[test]
    fn rounding_constraints_are_injected_into_mechanism_model() {
        let mut model = MetaModel::<f64>::new("rounding_injection");
        let floor_fn = RoundingFunction::floor(606, "round_floor", Linear::new(vec![], 1.2));
        let floor_y_id = floor_fn.result_variable().id();
        model.add_symbol(Arc::new(floor_fn)).unwrap();

        let round_fn = RoundingFunction::round(607, "round_nearest", Linear::new(vec![], -1.5));
        let round_y_id = round_fn.result_variable().id();
        model.add_symbol(Arc::new(round_fn)).unwrap();

        let trunc_fn = RoundingFunction::trunc(608, "round_trunc", Linear::new(vec![], -1.8));
        let trunc_y_id = trunc_fn.result_variable().id();
        model.add_symbol(Arc::new(trunc_fn)).unwrap();

        let mechanism = model.try_into_mechanism_model().unwrap();

        let floor_constraints = mechanism
            .constraints()
            .iter()
            .filter(|constraint| {
                constraint
                    .from
                    .as_ref()
                    .map(|symbol| symbol.id().id == 606)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        assert_eq!(floor_constraints.len(), 3);

        let floor_y_solver_index = mechanism.find_token(floor_y_id).unwrap().solver_index;
        let floor_int_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "round_floor_int")
            .map(|token| token.solver_index)
            .unwrap();

        let floor_feasible =
            HashMap::from([(floor_y_solver_index, 1.0), (floor_int_solver_index, 1.0)]);
        assert!(
            floor_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &floor_feasible))
        );

        let floor_infeasible =
            HashMap::from([(floor_y_solver_index, 2.0), (floor_int_solver_index, 2.0)]);
        assert!(
            floor_constraints
                .iter()
                .any(|constraint| !satisfies_constraint(constraint, &floor_infeasible))
        );

        let round_constraints = mechanism
            .constraints()
            .iter()
            .filter(|constraint| {
                constraint
                    .from
                    .as_ref()
                    .map(|symbol| symbol.id().id == 607)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        assert_eq!(round_constraints.len(), 7);

        let round_y_solver_index = mechanism.find_token(round_y_id).unwrap().solver_index;
        let round_int_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "round_nearest_int")
            .map(|token| token.solver_index)
            .unwrap();
        let round_sign_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "round_nearest_sign")
            .map(|token| token.solver_index)
            .unwrap();

        let round_feasible = HashMap::from([
            (round_y_solver_index, -2.0),
            (round_int_solver_index, -2.0),
            (round_sign_solver_index, 0.0),
        ]);
        assert!(
            round_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &round_feasible))
        );

        let round_infeasible = HashMap::from([
            (round_y_solver_index, -1.0),
            (round_int_solver_index, -1.0),
            (round_sign_solver_index, 0.0),
        ]);
        assert!(
            round_constraints
                .iter()
                .any(|constraint| !satisfies_constraint(constraint, &round_infeasible))
        );

        let trunc_constraints = mechanism
            .constraints()
            .iter()
            .filter(|constraint| {
                constraint
                    .from
                    .as_ref()
                    .map(|symbol| symbol.id().id == 608)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        assert_eq!(trunc_constraints.len(), 7);

        let trunc_y_solver_index = mechanism.find_token(trunc_y_id).unwrap().solver_index;
        let trunc_int_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "round_trunc_int")
            .map(|token| token.solver_index)
            .unwrap();
        let trunc_sign_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "round_trunc_sign")
            .map(|token| token.solver_index)
            .unwrap();

        let trunc_feasible = HashMap::from([
            (trunc_y_solver_index, -1.0),
            (trunc_int_solver_index, -1.0),
            (trunc_sign_solver_index, 0.0),
        ]);
        assert!(
            trunc_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &trunc_feasible))
        );

        let trunc_infeasible = HashMap::from([
            (trunc_y_solver_index, -2.0),
            (trunc_int_solver_index, -2.0),
            (trunc_sign_solver_index, 0.0),
        ]);
        assert!(
            trunc_constraints
                .iter()
                .any(|constraint| !satisfies_constraint(constraint, &trunc_infeasible))
        );
    }

    #[test]
    fn mod_constraints_are_injected_into_mechanism_model() {
        let mut model = MetaModel::<f64>::new("mod_injection");
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(608),
            "x_mod",
            VariableRange::bounded(-10.0, 10.0),
        );
        let x_index = model.register_variable(x).unwrap();
        let mod_fn = ModFunction::new(
            609,
            "mod_value",
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            2.0,
        );
        let y_id = mod_fn.result_variable().id();
        model.add_symbol(Arc::new(mod_fn)).unwrap();

        let mod_neg_fn = ModFunction::new(
            610,
            "mod_value_neg",
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            -2.0,
        );
        let y_neg_id = mod_neg_fn.result_variable().id();
        model.add_symbol(Arc::new(mod_neg_fn)).unwrap();

        let mechanism = model.try_into_mechanism_model().unwrap();
        let mod_constraints = mechanism
            .constraints()
            .iter()
            .filter(|constraint| {
                constraint
                    .from
                    .as_ref()
                    .map(|symbol| symbol.id().id == 609)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        assert_eq!(mod_constraints.len(), 3);

        let x_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "x_mod")
            .map(|token| token.solver_index)
            .unwrap();
        let y_solver_index = mechanism.find_token(y_id).unwrap().solver_index;
        let q_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "mod_value_q")
            .map(|token| token.solver_index)
            .unwrap();

        let feasible = HashMap::from([
            (x_solver_index, 5.0),
            (y_solver_index, 1.0),
            (q_solver_index, 2.0),
        ]);
        assert!(
            mod_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &feasible))
        );

        let infeasible = HashMap::from([
            (x_solver_index, 5.0),
            (y_solver_index, 0.0),
            (q_solver_index, 2.0),
        ]);
        assert!(
            mod_constraints
                .iter()
                .any(|constraint| !satisfies_constraint(constraint, &infeasible))
        );

        let mod_neg_constraints = mechanism
            .constraints()
            .iter()
            .filter(|constraint| {
                constraint
                    .from
                    .as_ref()
                    .map(|symbol| symbol.id().id == 610)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        assert_eq!(mod_neg_constraints.len(), 3);

        let y_neg_solver_index = mechanism.find_token(y_neg_id).unwrap().solver_index;
        let q_neg_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "mod_value_neg_q")
            .map(|token| token.solver_index)
            .unwrap();

        let feasible_neg = HashMap::from([
            (x_solver_index, 5.0),
            (y_neg_solver_index, -1.0),
            (q_neg_solver_index, -3.0),
        ]);
        assert!(
            mod_neg_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &feasible_neg))
        );

        let infeasible_neg = HashMap::from([
            (x_solver_index, 5.0),
            (y_neg_solver_index, 1.0),
            (q_neg_solver_index, -2.0),
        ]);
        assert!(
            mod_neg_constraints
                .iter()
                .any(|constraint| !satisfies_constraint(constraint, &infeasible_neg))
        );
    }

    #[test]
    fn abs_constraints_are_injected_into_mechanism_model() {
        let mut model = MetaModel::<f64>::new("abs_injection");
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(61),
            "x",
            VariableRange::bounded(-2.0, 2.0),
        );
        let x_index = model.register_variable(x).unwrap();
        let abs = AbsFunction::new(
            601,
            "abs_value",
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
        );
        let y_id = abs.result_variable().id();
        model.add_symbol(Arc::new(abs)).unwrap();

        let mechanism = model.try_into_mechanism_model().unwrap();
        let generated_constraints = mechanism
            .constraints()
            .iter()
            .filter(|constraint| {
                constraint
                    .from
                    .as_ref()
                    .map(|symbol| symbol.id().id == 601)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        assert_eq!(generated_constraints.len(), 4);

        let x_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "x")
            .map(|token| token.solver_index)
            .unwrap();
        let y_solver_index = mechanism.find_token(y_id).unwrap().solver_index;
        let side_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "abs_value_side")
            .map(|token| token.solver_index)
            .unwrap();

        let feasible_positive = HashMap::from([
            (x_solver_index, 2.0),
            (y_solver_index, 2.0),
            (side_solver_index, 1.0),
        ]);
        assert!(
            generated_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &feasible_positive))
        );

        let feasible_negative = HashMap::from([
            (x_solver_index, -2.0),
            (y_solver_index, 2.0),
            (side_solver_index, 0.0),
        ]);
        assert!(
            generated_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &feasible_negative))
        );

        let infeasible_wrong_abs = HashMap::from([
            (x_solver_index, -2.0),
            (y_solver_index, -2.0),
            (side_solver_index, 0.0),
        ]);
        assert!(
            generated_constraints
                .iter()
                .any(|constraint| !satisfies_constraint(constraint, &infeasible_wrong_abs))
        );
    }

    #[test]
    fn logic_constraints_are_injected_into_mechanism_model() {
        let mut model = MetaModel::<f64>::new("logic_injection");
        let and_fn = AndFunction::new(
            602,
            "and_logic",
            vec![Linear::new(vec![], 1.0), Linear::new(vec![], 0.0)],
        );
        let and_id = and_fn.result_variable().id();
        model.add_symbol(Arc::new(and_fn)).unwrap();

        let xor_fn = XorFunction::new(
            603,
            "xor_logic",
            vec![Linear::new(vec![], 1.0), Linear::new(vec![], 0.0)],
        );
        let xor_id = xor_fn.result_variable().id();
        model.add_symbol(Arc::new(xor_fn)).unwrap();

        let or_fn = OrFunction::new(
            604,
            "or_logic",
            vec![Linear::new(vec![], 1.0), Linear::new(vec![], 0.0)],
        );
        let or_id = or_fn.result_variable().id();
        model.add_symbol(Arc::new(or_fn)).unwrap();

        let not_fn = NotFunction::new(605, "not_logic", Linear::new(vec![], 0.0));
        let not_id = not_fn.result_variable().id();
        model.add_symbol(Arc::new(not_fn)).unwrap();

        let mechanism = model.try_into_mechanism_model().unwrap();

        let and_constraints = mechanism
            .constraints()
            .iter()
            .filter(|constraint| {
                constraint
                    .from
                    .as_ref()
                    .map(|symbol| symbol.id().id == 602)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        assert!(!and_constraints.is_empty());

        let and_solver_index = mechanism.find_token(and_id).unwrap().solver_index;
        let and_nz0_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "and_logic_and_nz0")
            .map(|token| token.solver_index)
            .unwrap();
        let and_nz1_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "and_logic_and_nz1")
            .map(|token| token.solver_index)
            .unwrap();
        let and_side0_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "and_logic_and_side0")
            .map(|token| token.solver_index)
            .unwrap();
        let and_side1_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "and_logic_and_side1")
            .map(|token| token.solver_index)
            .unwrap();

        let and_feasible = HashMap::from([
            (and_solver_index, 0.0),
            (and_nz0_solver_index, 1.0),
            (and_nz1_solver_index, 0.0),
            (and_side0_solver_index, 1.0),
            (and_side1_solver_index, 0.0),
        ]);
        assert!(
            and_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &and_feasible))
        );

        let xor_constraints = mechanism
            .constraints()
            .iter()
            .filter(|constraint| {
                constraint
                    .from
                    .as_ref()
                    .map(|symbol| symbol.id().id == 603)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        assert!(!xor_constraints.is_empty());

        let xor_solver_index = mechanism.find_token(xor_id).unwrap().solver_index;
        let xor_nz0_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "xor_logic_xor_nz0")
            .map(|token| token.solver_index)
            .unwrap();
        let xor_nz1_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "xor_logic_xor_nz1")
            .map(|token| token.solver_index)
            .unwrap();
        let xor_side0_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "xor_logic_xor_side0")
            .map(|token| token.solver_index)
            .unwrap();
        let xor_side1_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "xor_logic_xor_side1")
            .map(|token| token.solver_index)
            .unwrap();

        let xor_feasible = HashMap::from([
            (xor_solver_index, 1.0),
            (xor_nz0_solver_index, 1.0),
            (xor_nz1_solver_index, 0.0),
            (xor_side0_solver_index, 1.0),
            (xor_side1_solver_index, 0.0),
        ]);
        assert!(
            xor_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &xor_feasible))
        );

        let or_constraints = mechanism
            .constraints()
            .iter()
            .filter(|constraint| {
                constraint
                    .from
                    .as_ref()
                    .map(|symbol| symbol.id().id == 604)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        assert!(!or_constraints.is_empty());

        let or_solver_index = mechanism.find_token(or_id).unwrap().solver_index;
        let or_nz0_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "or_logic_or_nz0")
            .map(|token| token.solver_index)
            .unwrap();
        let or_nz1_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "or_logic_or_nz1")
            .map(|token| token.solver_index)
            .unwrap();
        let or_side0_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "or_logic_or_side0")
            .map(|token| token.solver_index)
            .unwrap();
        let or_side1_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "or_logic_or_side1")
            .map(|token| token.solver_index)
            .unwrap();

        let or_feasible = HashMap::from([
            (or_solver_index, 1.0),
            (or_nz0_solver_index, 1.0),
            (or_nz1_solver_index, 0.0),
            (or_side0_solver_index, 1.0),
            (or_side1_solver_index, 0.0),
        ]);
        assert!(
            or_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &or_feasible))
        );

        let not_constraints = mechanism
            .constraints()
            .iter()
            .filter(|constraint| {
                constraint
                    .from
                    .as_ref()
                    .map(|symbol| symbol.id().id == 605)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        assert!(!not_constraints.is_empty());

        let not_solver_index = mechanism.find_token(not_id).unwrap().solver_index;
        let not_nz_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "not_logic_not_nz")
            .map(|token| token.solver_index)
            .unwrap();
        let not_side_solver_index = mechanism
            .tokens()
            .iter()
            .find(|token| token.name() == "not_logic_not_side")
            .map(|token| token.solver_index)
            .unwrap();

        let not_feasible = HashMap::from([
            (not_solver_index, 1.0),
            (not_nz_solver_index, 0.0),
            (not_side_solver_index, 0.0),
        ]);
        assert!(
            not_constraints
                .iter()
                .all(|constraint| satisfies_constraint(constraint, &not_feasible))
        );
    }

    #[test]
    fn meta_model_building_status_callback_covers_registration_and_objective() {
        let mut model = MetaModel::<f64>::new("meta_status");
        let x = ContinuousVariableItem::auto("meta_status_x");
        let x_index = model.register_variable(x).unwrap();
        model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                2.0,
                "meta_status_c",
            )
            .unwrap();

        let statuses = Arc::new(Mutex::new(Vec::new()));
        let statuses_for_callback = statuses.clone();
        let callback: ModelBuildingStatusCallback = Arc::new(move |status| {
            statuses_for_callback.lock().unwrap().push(status.clone());
            Ok(())
        });

        let mechanism = model
            .try_into_mechanism_model_with_status_callback(Some(&callback))
            .expect("meta model conversion with status callback should succeed");
        assert_eq!(mechanism.num_variables(), 1);

        let statuses = statuses.lock().unwrap();
        assert!(!statuses.is_empty());
        assert!(
            statuses
                .iter()
                .any(|status| status.stage == ModelBuildingStage::RegisterTokens)
        );
        assert!(
            statuses
                .iter()
                .any(|status| status.stage == ModelBuildingStage::RegisterLinearConstraints)
        );
        assert!(
            statuses
                .iter()
                .any(|status| status.stage == ModelBuildingStage::BuildObjective)
        );
    }

    #[test]
    fn meta_to_quadratic_status_callback_reaches_flatten_stage() {
        let mut model = MetaModel::<f64>::new("meta_to_q_status");
        let x = ContinuousVariableItem::auto("meta_to_q_x");
        let x_index = model.register_variable(x).unwrap();
        model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                3.0,
                "meta_to_q_c",
            )
            .unwrap();

        let statuses = Arc::new(Mutex::new(Vec::new()));
        let statuses_for_callback = statuses.clone();
        let callback: ModelBuildingStatusCallback = Arc::new(move |status| {
            statuses_for_callback.lock().unwrap().push(status.clone());
            Ok(())
        });

        let quadratic = model
            .try_into_quadratic_tetrad_model_with_status_callback(Some(&callback))
            .expect("meta to quadratic conversion with status callback should succeed");
        assert_eq!(quadratic.num_variables(), 1);

        let statuses = statuses.lock().unwrap();
        assert!(
            statuses
                .iter()
                .any(|status| status.stage == ModelBuildingStage::RegisterTokens)
        );
        assert!(
            statuses
                .iter()
                .any(|status| status.stage == ModelBuildingStage::FlattenQuadraticModel)
        );
    }

    #[derive(Debug)]
    struct MetaModelShortcutDummySolver;

    impl SolverInfo for MetaModelShortcutDummySolver {
        fn name(&self) -> &str {
            "meta_shortcut_dummy"
        }

        fn capabilities(&self) -> Vec<SolverCapability> {
            vec![SolverCapability::Linear, SolverCapability::Quadratic]
        }
    }

    impl LinearSolver for MetaModelShortcutDummySolver {
        fn solve_linear(&self, _model: &LinearTriadModel) -> Result<crate::solver::SolverOutput> {
            Ok(SolverOutput::optimal(7.0, vec![1.0]))
        }
    }

    impl QuadraticSolver for MetaModelShortcutDummySolver {
        fn solve_quadratic(
            &self,
            _model: &QuadraticTetradModel,
        ) -> Result<crate::solver::SolverOutput> {
            Ok(SolverOutput::new(SolverStatus::Optimal).with_solution(vec![2.0]))
        }
    }

    #[test]
    fn solve_linear_with_options_shortcuts_model_and_solver_chain() {
        let mut model = MetaModel::<f64>::new("meta_shortcut_linear");
        let x = ContinuousVariableItem::auto("meta_shortcut_x");
        let x_index = model.register_variable(x).unwrap();
        model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                1.0,
                "meta_shortcut_c",
            )
            .unwrap();

        let model_statuses = Arc::new(Mutex::new(Vec::new()));
        let model_statuses_for_callback = model_statuses.clone();
        let model_callback: ModelBuildingStatusCallback = Arc::new(move |status| {
            model_statuses_for_callback
                .lock()
                .unwrap()
                .push(status.stage);
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

        let solver = MetaModelShortcutDummySolver;
        let options = crate::solver::SolveOptions::new()
            .with_building_callback(Some(&model_callback))
            .with_solving_callback(Some(&solving_callback));
        let output = model
            .solve_linear_with_options(&solver, &options)
            .expect("linear shortcut solving should succeed");

        assert!(output.status.is_optimal());
        let model_statuses = model_statuses.lock().unwrap();
        assert!(
            model_statuses
                .iter()
                .any(|stage| *stage == ModelBuildingStage::RegisterTokens)
        );
        assert!(
            model_statuses
                .iter()
                .any(|stage| *stage == ModelBuildingStage::FlattenLinearModel)
        );

        let solving_statuses = solving_statuses.lock().unwrap();
        assert_eq!(solving_statuses.len(), 2);
        assert_eq!(solving_statuses[0], SolverStatus::Solving);
        assert_eq!(solving_statuses[1], SolverStatus::Optimal);
    }

    #[test]
    fn solve_quadratic_with_shortcut_converts_then_solves() {
        let mut model = MetaModel::<f64>::new("meta_shortcut_quadratic");
        let x = ContinuousVariableItem::auto("meta_shortcut_q_x");
        let x_index = model.register_variable(x).unwrap();
        model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                3.0,
                "meta_shortcut_q_c",
            )
            .unwrap();

        let solver = MetaModelShortcutDummySolver;
        let output = model
            .solve_quadratic_with(&solver)
            .expect("quadratic shortcut solving should succeed");
        assert!(output.status.is_optimal());
        assert_eq!(output.solution, Some(vec![2.0]));
    }

    #[test]
    fn solve_with_shortcut_auto_selects_and_reports_callbacks() {
        let mut model = MetaModel::<f64>::new("meta_shortcut_auto");
        let x = ContinuousVariableItem::auto("meta_shortcut_auto_x");
        let x_index = model.register_variable(x).unwrap();
        model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                1.0,
                "meta_shortcut_auto_c",
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

        let solver = MetaModelShortcutDummySolver;
        let options = crate::solver::SolveOptions::new()
            .with_building_callback(Some(&model_callback))
            .with_solving_callback(Some(&solving_callback));
        let output = model
            .solve_with_options(&solver, &options)
            .expect("meta-model auto shortcut solving should succeed");
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
}

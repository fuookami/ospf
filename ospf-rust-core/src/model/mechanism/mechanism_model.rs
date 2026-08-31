//! 机理模型
//! Mechanism Model

use super::super::flatten::{Linear, LinearMonomial, Quadratic, QuadraticMonomial};
use super::super::intermediate::{
    BasicLinearTriadModel, BasicQuadraticTetradModel, LinearTriadModel, QuadraticTetradModel,
    SparseMatrix, SparseVector,
};
use super::super::object::{Objective, ObjectiveCategory};
use super::super::{ModelBuildingStage, ModelBuildingStatus, ModelBuildingStatusCallback};
use super::{BasicMechanismModel, ConstraintRelation, LinearInequality, QuadraticInequality};
use crate::error::{ModelError, Result};
use crate::solver::SolverOutput;
use crate::variable::VariableId;
use std::collections::HashMap;
use std::fmt::Debug;

/// Benders 割类型 / Benders cut kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BendersCutKind {
    /// 可行性割 / Feasibility cut
    Feasibility,
    /// 最优性割 / Optimality cut
    Optimality,
}

/// Benders 割请求 / Benders cut request
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BendersCutRequest {
    /// 生成可行性割（无需目标变量）/ Generate feasibility cut (objective variable not required)
    Feasibility,
    /// 生成最优性割（需要目标变量）/ Generate optimality cut (objective variable required)
    Optimality { objective_variable: VariableId },
}

/// 机理模型 / Mechanism Model
///
/// 继承基本机理模型，添加目标函数支持。
/// Inherits basic mechanism model, adding objective function support.
#[derive(Debug, Clone)]
pub struct MechanismModel<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 基本机理模型 / Basic mechanism model
    pub basic: BasicMechanismModel<V>,
    /// 目标函数 / Objective function
    objective: Objective<V>,
}

impl<V> MechanismModel<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建空模型 / Create empty model
    pub fn new(name: &str) -> Self {
        Self {
            basic: BasicMechanismModel::new(name),
            objective: Objective::default(),
        }
    }

    /// 从基本机理模型创建 / Create from basic mechanism model
    pub fn from_basic(basic: BasicMechanismModel<V>) -> Self {
        Self {
            basic,
            objective: Objective::default(),
        }
    }

    /// 设置目标函数 / Set objective
    pub fn set_objective(&mut self, objective: Objective<V>) {
        self.objective = objective;
    }

    /// 获取目标函数 / Get objective
    pub fn objective(&self) -> &Objective<V> {
        &self.objective
    }

    /// 获取基本模型引用 / Get basic model reference
    pub fn as_basic(&self) -> &BasicMechanismModel<V> {
        &self.basic
    }

    /// 提取基本模型 / Extract basic model
    pub fn into_basic(self) -> BasicMechanismModel<V> {
        self.basic
    }
}

impl MechanismModel<f64> {
    fn emit_model_building_status(
        callback: Option<&ModelBuildingStatusCallback>,
        status: ModelBuildingStatus,
    ) -> Result<()> {
        if let Some(callback) = callback {
            callback(&status)?;
        }
        Ok(())
    }

    fn is_zero(value: f64) -> bool {
        value.abs() <= f64::EPSILON
    }

    fn expanded_linear_constraint_count(&self) -> usize {
        self.basic
            .constraints()
            .iter()
            .map(|constraint| match constraint.inequality.relation {
                ConstraintRelation::Equal => 2,
                ConstraintRelation::LessEqual | ConstraintRelation::GreaterEqual => 1,
            })
            .sum()
    }

    fn normalize_fixed_variables_by_solver_index(
        &self,
        fixed_variables: &HashMap<VariableId, f64>,
    ) -> Result<HashMap<usize, f64>> {
        let mut normalized = HashMap::with_capacity(fixed_variables.len());
        for (var_id, value) in fixed_variables {
            let token = self.find_token(*var_id).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "fixed variable {} not found in mechanism model `{}`",
                    var_id, self.basic.name
                ))
            })?;
            normalized.insert(token.solver_index, *value);
        }
        Ok(normalized)
    }

    fn push_feasibility_cut_row(
        &self,
        coefficients: impl Iterator<Item = (usize, f64)>,
        rhs: f64,
        dual: f64,
        fixed_variables: &HashMap<usize, f64>,
        cut_coefficients: &mut HashMap<usize, f64>,
        cut_constant: &mut f64,
        fixed_point_value: &mut f64,
    ) -> Result<()> {
        if Self::is_zero(dual) {
            return Ok(());
        }

        *cut_constant += dual * rhs;
        *fixed_point_value += dual * rhs;

        for (var_index, row_coeff) in coefficients {
            if var_index >= self.basic.num_variables() {
                return Err(ModelError::InvalidConstraint(format!(
                    "constraint references variable index {} but model `{}` has only {} variables",
                    var_index,
                    self.basic.name,
                    self.basic.num_variables()
                ))
                .into());
            }
            if let Some(fixed_value) = fixed_variables.get(&var_index) {
                let projected = dual * row_coeff;
                if !Self::is_zero(projected) {
                    let entry = cut_coefficients.entry(var_index).or_insert(0.0);
                    *entry -= projected;
                    *fixed_point_value -= projected * *fixed_value;
                }
            }
        }

        Ok(())
    }

    fn push_optimal_cut_row(
        &self,
        coefficients: impl Iterator<Item = (usize, f64)>,
        rhs: f64,
        dual: f64,
        fixed_variables: &HashMap<usize, f64>,
        rhs_coefficients: &mut HashMap<usize, f64>,
        rhs_constant: &mut f64,
    ) -> Result<()> {
        if Self::is_zero(dual) {
            return Ok(());
        }

        *rhs_constant += dual * rhs;
        for (var_index, row_coeff) in coefficients {
            if var_index >= self.basic.num_variables() {
                return Err(ModelError::InvalidConstraint(format!(
                    "constraint references variable index {} but model `{}` has only {} variables",
                    var_index,
                    self.basic.name,
                    self.basic.num_variables()
                ))
                .into());
            }
            if fixed_variables.contains_key(&var_index) {
                let projected = -dual * row_coeff;
                if !Self::is_zero(projected) {
                    let entry = rhs_coefficients.entry(var_index).or_insert(0.0);
                    *entry += projected;
                }
            }
        }

        Ok(())
    }

    fn push_optimal_quadratic_cut_row(
        &self,
        coefficients: impl Iterator<Item = (usize, Option<usize>, f64)>,
        rhs: f64,
        dual: f64,
        fixed_variables: &HashMap<usize, f64>,
        rhs_linear_coefficients: &mut HashMap<usize, f64>,
        rhs_quadratic_coefficients: &mut HashMap<(usize, usize), f64>,
        rhs_constant: &mut f64,
    ) -> Result<()> {
        if Self::is_zero(dual) {
            return Ok(());
        }

        *rhs_constant += dual * rhs;
        for (var_index1, var_index2, row_coeff) in coefficients {
            if var_index1 >= self.basic.num_variables() {
                return Err(ModelError::InvalidConstraint(format!(
                    "constraint references variable index {} but model `{}` has only {} variables",
                    var_index1,
                    self.basic.name,
                    self.basic.num_variables()
                ))
                .into());
            }
            if let Some(var_index2) = var_index2 {
                if var_index2 >= self.basic.num_variables() {
                    return Err(ModelError::InvalidConstraint(format!(
                        "constraint references variable index {} but model `{}` has only {} variables",
                        var_index2,
                        self.basic.name,
                        self.basic.num_variables()
                    ))
                    .into());
                }
                if fixed_variables.contains_key(&var_index1)
                    && fixed_variables.contains_key(&var_index2)
                {
                    let projected = -dual * row_coeff;
                    if !Self::is_zero(projected) {
                        let key = if var_index1 <= var_index2 {
                            (var_index1, var_index2)
                        } else {
                            (var_index2, var_index1)
                        };
                        let entry = rhs_quadratic_coefficients.entry(key).or_insert(0.0);
                        *entry += projected;
                    }
                }
            } else if fixed_variables.contains_key(&var_index1) {
                let projected = -dual * row_coeff;
                if !Self::is_zero(projected) {
                    let entry = rhs_linear_coefficients.entry(var_index1).or_insert(0.0);
                    *entry += projected;
                }
            }
        }

        Ok(())
    }

    /// 生成可行性割（按展开后的线性约束对偶值）/ Generate feasibility cut from expanded-row dual values
    ///
    /// 输入的 `row_duals` 对应机理模型线性约束展开为 `Ax <= b` 后的每一行：
    /// - `<=` 对应 1 行
    /// - `>=` 对应 1 行（符号翻转后）
    /// - `=` 对应 2 行（`<=` 与 `>=` 两行）
    ///
    /// The input `row_duals` must align with expanded `Ax <= b` rows:
    /// - `<=` contributes one row
    /// - `>=` contributes one sign-flipped row
    /// - `=` contributes two rows (`<=` and `>=`)
    pub fn generate_feasibility_cut_from_row_duals(
        &self,
        fixed_variables: &HashMap<usize, f64>,
        row_duals: &[f64],
    ) -> Result<LinearInequality<f64>> {
        let expected_rows = self.expanded_linear_constraint_count();
        if row_duals.len() < expected_rows {
            return Err(ModelError::InvalidConstraint(format!(
                "row_duals length {} is smaller than expected expanded constraint rows {}",
                row_duals.len(),
                expected_rows
            ))
            .into());
        }
        if let Some((invalid_index, _)) = fixed_variables
            .iter()
            .find(|(var_index, _)| **var_index >= self.basic.num_variables())
        {
            return Err(ModelError::InvalidConstraint(format!(
                "fixed variable index {} out of range for mechanism model `{}` (variables={})",
                invalid_index,
                self.basic.name,
                self.basic.num_variables()
            ))
            .into());
        }

        let mut dual_offset = 0usize;
        let mut cut_constant = 0.0;
        let mut fixed_point_value = 0.0;
        let mut cut_coefficients: HashMap<usize, f64> = HashMap::new();

        for constraint in self.basic.constraints() {
            let inequality = &constraint.inequality;
            let constant = *inequality.polynomial.constant_term();
            match inequality.relation {
                ConstraintRelation::LessEqual => {
                    let rhs = inequality.rhs - constant;
                    let dual = row_duals[dual_offset];
                    dual_offset += 1;
                    self.push_feasibility_cut_row(
                        inequality
                            .polynomial
                            .monomials()
                            .iter()
                            .map(|mono| (mono.var_index(), *mono.coefficient())),
                        rhs,
                        dual,
                        fixed_variables,
                        &mut cut_coefficients,
                        &mut cut_constant,
                        &mut fixed_point_value,
                    )?;
                }
                ConstraintRelation::GreaterEqual => {
                    let rhs = constant - inequality.rhs;
                    let dual = row_duals[dual_offset];
                    dual_offset += 1;
                    self.push_feasibility_cut_row(
                        inequality
                            .polynomial
                            .monomials()
                            .iter()
                            .map(|mono| (mono.var_index(), -*mono.coefficient())),
                        rhs,
                        dual,
                        fixed_variables,
                        &mut cut_coefficients,
                        &mut cut_constant,
                        &mut fixed_point_value,
                    )?;
                }
                ConstraintRelation::Equal => {
                    let rhs_le = inequality.rhs - constant;
                    let dual_le = row_duals[dual_offset];
                    dual_offset += 1;
                    self.push_feasibility_cut_row(
                        inequality
                            .polynomial
                            .monomials()
                            .iter()
                            .map(|mono| (mono.var_index(), *mono.coefficient())),
                        rhs_le,
                        dual_le,
                        fixed_variables,
                        &mut cut_coefficients,
                        &mut cut_constant,
                        &mut fixed_point_value,
                    )?;

                    let rhs_ge = constant - inequality.rhs;
                    let dual_ge = row_duals[dual_offset];
                    dual_offset += 1;
                    self.push_feasibility_cut_row(
                        inequality
                            .polynomial
                            .monomials()
                            .iter()
                            .map(|mono| (mono.var_index(), -*mono.coefficient())),
                        rhs_ge,
                        dual_ge,
                        fixed_variables,
                        &mut cut_coefficients,
                        &mut cut_constant,
                        &mut fixed_point_value,
                    )?;
                }
            }
        }

        if fixed_point_value < 0.0 {
            cut_constant = -cut_constant;
            for coefficient in cut_coefficients.values_mut() {
                *coefficient = -*coefficient;
            }
        }

        let mut sorted_indexes: Vec<usize> = cut_coefficients.keys().copied().collect();
        sorted_indexes.sort_unstable();
        let monomials: Vec<LinearMonomial<f64>> = sorted_indexes
            .into_iter()
            .filter_map(|var_index| {
                let coefficient = cut_coefficients.get(&var_index).copied().unwrap_or(0.0);
                if Self::is_zero(coefficient) {
                    None
                } else {
                    Some(LinearMonomial::new(coefficient, var_index))
                }
            })
            .collect();

        Ok(LinearInequality::less_equal(
            Linear::new(monomials, cut_constant),
            0.0,
        ))
    }

    /// 生成可行性割（按变量 ID 固定值 + 展开行对偶值）/ Generate feasibility cut from fixed variable IDs and expanded-row duals
    pub fn generate_feasibility_cut_from_row_duals_by_id(
        &self,
        fixed_variables: &HashMap<VariableId, f64>,
        row_duals: &[f64],
    ) -> Result<LinearInequality<f64>> {
        let normalized = self.normalize_fixed_variables_by_solver_index(fixed_variables)?;
        self.generate_feasibility_cut_from_row_duals(&normalized, row_duals)
    }

    /// 生成可行性割（直接消费 Farkas 对偶模型解向量）/ Generate feasibility cut directly from Farkas-dual solution vector
    ///
    /// `farkas_solution` 需要是 `LinearTriadModel::to_farkas_dual()` 求解后得到的变量解向量，
    /// 其前 `expanded_linear_constraint_count()` 项为约束乘子。
    ///
    /// `farkas_solution` must be the variable solution vector of `LinearTriadModel::to_farkas_dual()`.
    /// The first `expanded_linear_constraint_count()` entries are row multipliers.
    pub fn generate_feasibility_cut_from_farkas_solution(
        &self,
        fixed_variables: &HashMap<VariableId, f64>,
        farkas_solution: &[f64],
    ) -> Result<LinearInequality<f64>> {
        let expected_rows = self.expanded_linear_constraint_count();
        if farkas_solution.len() < expected_rows {
            return Err(ModelError::InvalidConstraint(format!(
                "farkas solution length {} is smaller than required row multipliers {}",
                farkas_solution.len(),
                expected_rows
            ))
            .into());
        }
        self.generate_feasibility_cut_from_row_duals_by_id(
            fixed_variables,
            &farkas_solution[..expected_rows],
        )
    }

    /// 生成可行性割列表（Kotlin 链路对齐）/ Generate feasibility cut list (aligned with Kotlin chain)
    ///
    /// 当前实现返回单条 cut；保留列表返回是为了后续扩展到多割场景。
    /// Current implementation returns one cut; list form keeps room for multi-cut extension.
    pub fn generate_feasibility_cuts_from_farkas_solution(
        &self,
        fixed_variables: &HashMap<VariableId, f64>,
        farkas_solution: &[f64],
    ) -> Result<Vec<LinearInequality<f64>>> {
        Ok(vec![self.generate_feasibility_cut_from_farkas_solution(
            fixed_variables,
            farkas_solution,
        )?])
    }

    /// 生成最优性割（按展开后的线性约束对偶值）/ Generate optimal cut from expanded-row dual values
    ///
    /// 输入的 `row_duals` 对应机理模型线性约束展开为 `Ax <= b` 后的每一行：
    /// - `<=` 对应 1 行
    /// - `>=` 对应 1 行（符号翻转后）
    /// - `=` 对应 2 行（`<=` 与 `>=` 两行）
    ///
    /// 返回的割形式与 Kotlin `generateOptimalCut` 对齐：
    /// - 目标为 `Maximum`：`theta <= rhs(x_fixed)`
    /// - 目标为 `Minimum`：`theta >= rhs(x_fixed)`
    ///
    /// The returned cut aligns with Kotlin `generateOptimalCut`:
    /// - `Maximum`: `theta <= rhs(x_fixed)`
    /// - `Minimum`: `theta >= rhs(x_fixed)`
    pub fn generate_optimal_cut_from_row_duals(
        &self,
        objective_variable_index: usize,
        fixed_variables: &HashMap<usize, f64>,
        row_duals: &[f64],
    ) -> Result<LinearInequality<f64>> {
        let expected_rows = self.expanded_linear_constraint_count();
        if row_duals.len() < expected_rows {
            return Err(ModelError::InvalidConstraint(format!(
                "row_duals length {} is smaller than expected expanded constraint rows {}",
                row_duals.len(),
                expected_rows
            ))
            .into());
        }
        if objective_variable_index >= self.basic.num_variables() {
            return Err(ModelError::InvalidConstraint(format!(
                "objective variable index {} out of range for mechanism model `{}` (variables={})",
                objective_variable_index,
                self.basic.name,
                self.basic.num_variables()
            ))
            .into());
        }
        if let Some((invalid_index, _)) = fixed_variables
            .iter()
            .find(|(var_index, _)| **var_index >= self.basic.num_variables())
        {
            return Err(ModelError::InvalidConstraint(format!(
                "fixed variable index {} out of range for mechanism model `{}` (variables={})",
                invalid_index,
                self.basic.name,
                self.basic.num_variables()
            ))
            .into());
        }

        let mut dual_offset = 0usize;
        let mut rhs_constant = 0.0;
        let mut rhs_coefficients: HashMap<usize, f64> = HashMap::new();

        for constraint in self.basic.constraints() {
            let inequality = &constraint.inequality;
            let constant = *inequality.polynomial.constant_term();
            match inequality.relation {
                ConstraintRelation::LessEqual => {
                    let rhs = inequality.rhs - constant;
                    let dual = row_duals[dual_offset];
                    dual_offset += 1;
                    self.push_optimal_cut_row(
                        inequality
                            .polynomial
                            .monomials()
                            .iter()
                            .map(|mono| (mono.var_index(), *mono.coefficient())),
                        rhs,
                        dual,
                        fixed_variables,
                        &mut rhs_coefficients,
                        &mut rhs_constant,
                    )?;
                }
                ConstraintRelation::GreaterEqual => {
                    let rhs = constant - inequality.rhs;
                    let dual = row_duals[dual_offset];
                    dual_offset += 1;
                    self.push_optimal_cut_row(
                        inequality
                            .polynomial
                            .monomials()
                            .iter()
                            .map(|mono| (mono.var_index(), -*mono.coefficient())),
                        rhs,
                        dual,
                        fixed_variables,
                        &mut rhs_coefficients,
                        &mut rhs_constant,
                    )?;
                }
                ConstraintRelation::Equal => {
                    let rhs_le = inequality.rhs - constant;
                    let dual_le = row_duals[dual_offset];
                    dual_offset += 1;
                    self.push_optimal_cut_row(
                        inequality
                            .polynomial
                            .monomials()
                            .iter()
                            .map(|mono| (mono.var_index(), *mono.coefficient())),
                        rhs_le,
                        dual_le,
                        fixed_variables,
                        &mut rhs_coefficients,
                        &mut rhs_constant,
                    )?;

                    let rhs_ge = constant - inequality.rhs;
                    let dual_ge = row_duals[dual_offset];
                    dual_offset += 1;
                    self.push_optimal_cut_row(
                        inequality
                            .polynomial
                            .monomials()
                            .iter()
                            .map(|mono| (mono.var_index(), -*mono.coefficient())),
                        rhs_ge,
                        dual_ge,
                        fixed_variables,
                        &mut rhs_coefficients,
                        &mut rhs_constant,
                    )?;
                }
            }
        }

        let mut lhs_coefficients: HashMap<usize, f64> = HashMap::new();
        lhs_coefficients.insert(objective_variable_index, 1.0);
        for (var_index, rhs_coeff) in rhs_coefficients {
            let lhs_coeff = -rhs_coeff;
            if !Self::is_zero(lhs_coeff) {
                let entry = lhs_coefficients.entry(var_index).or_insert(0.0);
                *entry += lhs_coeff;
            }
        }

        let mut sorted_indexes: Vec<usize> = lhs_coefficients.keys().copied().collect();
        sorted_indexes.sort_unstable();
        let monomials: Vec<LinearMonomial<f64>> = sorted_indexes
            .into_iter()
            .filter_map(|var_index| {
                let coefficient = lhs_coefficients.get(&var_index).copied().unwrap_or(0.0);
                if Self::is_zero(coefficient) {
                    None
                } else {
                    Some(LinearMonomial::new(coefficient, var_index))
                }
            })
            .collect();

        let relation = match self.objective.category {
            ObjectiveCategory::Maximum => ConstraintRelation::LessEqual,
            ObjectiveCategory::Minimum => ConstraintRelation::GreaterEqual,
        };
        Ok(LinearInequality::new(
            Linear::new(monomials, 0.0),
            relation,
            rhs_constant,
        ))
    }

    /// 生成最优性割（按变量 ID 固定值 + 展开行对偶值）/ Generate optimal cut from fixed variable IDs and expanded-row duals
    pub fn generate_optimal_cut_from_row_duals_by_id(
        &self,
        objective_variable: VariableId,
        fixed_variables: &HashMap<VariableId, f64>,
        row_duals: &[f64],
    ) -> Result<LinearInequality<f64>> {
        let objective_var_index = self
            .find_token(objective_variable)
            .ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "objective variable {} not found in mechanism model `{}`",
                    objective_variable, self.basic.name
                ))
            })?
            .solver_index;
        let normalized = self.normalize_fixed_variables_by_solver_index(fixed_variables)?;
        self.generate_optimal_cut_from_row_duals(objective_var_index, &normalized, row_duals)
    }

    /// 生成最优性割（直接消费对偶模型解向量）/ Generate optimal cut directly from dual-solution vector
    ///
    /// `dual_solution` 需要是 `LinearTriadModel::to_dual()` 求解后得到的变量解向量，
    /// 其前 `expanded_linear_constraint_count()` 项为约束乘子。
    ///
    /// `dual_solution` must be the variable solution vector of `LinearTriadModel::to_dual()`.
    /// The first `expanded_linear_constraint_count()` entries are row multipliers.
    pub fn generate_optimal_cut_from_dual_solution(
        &self,
        objective_variable: VariableId,
        fixed_variables: &HashMap<VariableId, f64>,
        dual_solution: &[f64],
    ) -> Result<LinearInequality<f64>> {
        let expected_rows = self.expanded_linear_constraint_count();
        if dual_solution.len() < expected_rows {
            return Err(ModelError::InvalidConstraint(format!(
                "dual solution length {} is smaller than required row multipliers {}",
                dual_solution.len(),
                expected_rows
            ))
            .into());
        }
        self.generate_optimal_cut_from_row_duals_by_id(
            objective_variable,
            fixed_variables,
            &dual_solution[..expected_rows],
        )
    }

    /// 生成最优性割列表（Kotlin 链路对齐）/ Generate optimal cut list (aligned with Kotlin chain)
    ///
    /// 当前实现返回单条 cut；保留列表返回是为了后续扩展到多割场景。
    /// Current implementation returns one cut; list form keeps room for multi-cut extension.
    pub fn generate_optimal_cuts_from_dual_solution(
        &self,
        objective_variable: VariableId,
        fixed_variables: &HashMap<VariableId, f64>,
        dual_solution: &[f64],
    ) -> Result<Vec<LinearInequality<f64>>> {
        Ok(vec![self.generate_optimal_cut_from_dual_solution(
            objective_variable,
            fixed_variables,
            dual_solution,
        )?])
    }

    /// 统一生成 Benders 割列表 / Unified Benders cut generation
    ///
    /// 该接口用于上层统一调度：
    /// - `Feasibility`: 输入 Farkas 对偶解向量；
    /// - `Optimality`: 输入普通对偶解向量，并指定目标变量（如 `theta`）。
    ///
    /// This interface provides a single dispatch point:
    /// - `Feasibility`: expects Farkas-dual solution vector;
    /// - `Optimality`: expects dual solution vector and objective variable (e.g. `theta`).
    pub fn generate_benders_cuts_from_solution(
        &self,
        request: BendersCutRequest,
        fixed_variables: &HashMap<VariableId, f64>,
        solution: &[f64],
    ) -> Result<Vec<LinearInequality<f64>>> {
        match request {
            BendersCutRequest::Feasibility => {
                self.generate_feasibility_cuts_from_farkas_solution(fixed_variables, solution)
            }
            BendersCutRequest::Optimality { objective_variable } => self
                .generate_optimal_cuts_from_dual_solution(
                    objective_variable,
                    fixed_variables,
                    solution,
                ),
        }
    }

    /// 统一生成 Benders 割列表（直接消费求解器输出）/ Unified Benders cut generation from solver output
    ///
    /// 该接口会校验输出状态，并按兼容策略提取对偶/解向量后分派：
    /// - 优先使用 `output.dual_solution`（原问题直接对偶乘子）；
    /// - 若缺失则回退使用 `output.solution`（对偶模型变量解向量）。
    ///
    /// This interface validates solver status, then dispatches with a compatible dual extraction strategy:
    /// - prefer `output.dual_solution` (row multipliers from primal solve),
    /// - fallback to `output.solution` (variable vector from explicit dual solve).
    pub fn generate_benders_cuts_from_output(
        &self,
        request: BendersCutRequest,
        fixed_variables: &HashMap<VariableId, f64>,
        output: &SolverOutput,
    ) -> Result<Vec<LinearInequality<f64>>> {
        if !output.status.is_feasible() {
            return Err(ModelError::InvalidConstraint(format!(
                "solver output status {:?} is not feasible for benders cut generation",
                output.status
            ))
            .into());
        }
        let solution = output
            .dual_solution
            .as_deref()
            .or(output.solution.as_deref())
            .ok_or_else(|| {
                ModelError::InvalidConstraint(
                    "solver output has feasible status but missing dual/solution vector"
                        .to_string(),
                )
            })?;
        self.generate_benders_cuts_from_solution(request, fixed_variables, solution)
    }

    /// 生成可行性割列表（直接消费 Farkas 对偶求解输出）/ Generate feasibility cuts directly from Farkas solver output
    pub fn generate_feasibility_cuts_from_farkas_output(
        &self,
        fixed_variables: &HashMap<VariableId, f64>,
        output: &SolverOutput,
    ) -> Result<Vec<LinearInequality<f64>>> {
        if !output.status.is_feasible() {
            return Err(ModelError::InvalidConstraint(format!(
                "solver output status {:?} is not feasible for benders cut generation",
                output.status
            ))
            .into());
        }
        // 显式 Farkas 对偶模型输出时，`solution` 是对偶变量向量，`dual_solution` 是该对偶模型本身的行乘子。
        // 对齐 Kotlin 链路：此入口优先消费 `solution`，缺失时再回退 `dual_solution`。
        // For explicit Farkas-dual solves, `solution` stores dual-variable values while
        // `dual_solution` are row multipliers of the dual model itself. Align with Kotlin flow:
        // prefer `solution` first, then fall back to `dual_solution`.
        let solution = output
            .solution
            .as_deref()
            .or(output.dual_solution.as_deref())
            .ok_or_else(|| {
                ModelError::InvalidConstraint(
                    "solver output has feasible status but missing dual/solution vector"
                        .to_string(),
                )
            })?;
        self.generate_benders_cuts_from_solution(
            BendersCutRequest::Feasibility,
            fixed_variables,
            solution,
        )
    }

    /// 生成最优性割列表（直接消费对偶求解输出）/ Generate optimal cuts directly from dual solver output
    pub fn generate_optimal_cuts_from_dual_output(
        &self,
        objective_variable: VariableId,
        fixed_variables: &HashMap<VariableId, f64>,
        output: &SolverOutput,
    ) -> Result<Vec<LinearInequality<f64>>> {
        if !output.status.is_feasible() {
            return Err(ModelError::InvalidConstraint(format!(
                "solver output status {:?} is not feasible for benders cut generation",
                output.status
            ))
            .into());
        }
        // 显式对偶模型输出时，`solution` 是对偶变量向量，`dual_solution` 是该对偶模型的行乘子。
        // 对齐 Kotlin 链路：此入口优先消费 `solution`，缺失时再回退 `dual_solution`。
        // For explicit dual solves, `solution` stores dual-variable values while
        // `dual_solution` are row multipliers of the dual model. Align with Kotlin flow:
        // prefer `solution` first, then fall back to `dual_solution`.
        let solution = output
            .solution
            .as_deref()
            .or(output.dual_solution.as_deref())
            .ok_or_else(|| {
                ModelError::InvalidConstraint(
                    "solver output has feasible status but missing dual/solution vector"
                        .to_string(),
                )
            })?;
        self.generate_benders_cuts_from_solution(
            BendersCutRequest::Optimality { objective_variable },
            fixed_variables,
            solution,
        )
    }

    /// 生成二次最优性割（消费线性/二次约束对偶向量）/
    /// Generate quadratic optimality cut from linear/quadratic row dual vectors
    pub fn generate_optimal_quadratic_cut_from_row_duals(
        &self,
        objective_variable_index: usize,
        fixed_variables: &HashMap<usize, f64>,
        linear_row_duals: &[f64],
        quadratic_row_duals: &[f64],
    ) -> Result<QuadraticInequality<f64>> {
        let expected_linear_rows = self.expanded_linear_constraint_count();
        if linear_row_duals.len() < expected_linear_rows {
            return Err(ModelError::InvalidConstraint(format!(
                "linear row_duals length {} is smaller than expected expanded constraint rows {}",
                linear_row_duals.len(),
                expected_linear_rows
            ))
            .into());
        }
        let expected_quadratic_rows = self.basic.quadratic_constraints().len();
        if quadratic_row_duals.len() < expected_quadratic_rows {
            return Err(ModelError::InvalidConstraint(format!(
                "quadratic row_duals length {} is smaller than quadratic constraint rows {}",
                quadratic_row_duals.len(),
                expected_quadratic_rows
            ))
            .into());
        }
        if objective_variable_index >= self.basic.num_variables() {
            return Err(ModelError::InvalidConstraint(format!(
                "objective variable index {} out of range for mechanism model `{}` (variables={})",
                objective_variable_index,
                self.basic.name,
                self.basic.num_variables()
            ))
            .into());
        }
        if let Some((invalid_index, _)) = fixed_variables
            .iter()
            .find(|(var_index, _)| **var_index >= self.basic.num_variables())
        {
            return Err(ModelError::InvalidConstraint(format!(
                "fixed variable index {} out of range for mechanism model `{}` (variables={})",
                invalid_index,
                self.basic.name,
                self.basic.num_variables()
            ))
            .into());
        }

        let mut linear_dual_offset = 0usize;
        let mut rhs_constant = 0.0;
        let mut rhs_linear_coefficients: HashMap<usize, f64> = HashMap::new();
        let mut rhs_quadratic_coefficients: HashMap<(usize, usize), f64> = HashMap::new();

        for constraint in self.basic.constraints() {
            let inequality = &constraint.inequality;
            let constant = *inequality.polynomial.constant_term();
            match inequality.relation {
                ConstraintRelation::LessEqual => {
                    let rhs = inequality.rhs - constant;
                    let dual = linear_row_duals[linear_dual_offset];
                    linear_dual_offset += 1;
                    self.push_optimal_cut_row(
                        inequality
                            .polynomial
                            .monomials()
                            .iter()
                            .map(|mono| (mono.var_index(), *mono.coefficient())),
                        rhs,
                        dual,
                        fixed_variables,
                        &mut rhs_linear_coefficients,
                        &mut rhs_constant,
                    )?;
                }
                ConstraintRelation::GreaterEqual => {
                    let rhs = constant - inequality.rhs;
                    let dual = linear_row_duals[linear_dual_offset];
                    linear_dual_offset += 1;
                    self.push_optimal_cut_row(
                        inequality
                            .polynomial
                            .monomials()
                            .iter()
                            .map(|mono| (mono.var_index(), -*mono.coefficient())),
                        rhs,
                        dual,
                        fixed_variables,
                        &mut rhs_linear_coefficients,
                        &mut rhs_constant,
                    )?;
                }
                ConstraintRelation::Equal => {
                    let rhs_le = inequality.rhs - constant;
                    let dual_le = linear_row_duals[linear_dual_offset];
                    linear_dual_offset += 1;
                    self.push_optimal_cut_row(
                        inequality
                            .polynomial
                            .monomials()
                            .iter()
                            .map(|mono| (mono.var_index(), *mono.coefficient())),
                        rhs_le,
                        dual_le,
                        fixed_variables,
                        &mut rhs_linear_coefficients,
                        &mut rhs_constant,
                    )?;

                    let rhs_ge = constant - inequality.rhs;
                    let dual_ge = linear_row_duals[linear_dual_offset];
                    linear_dual_offset += 1;
                    self.push_optimal_cut_row(
                        inequality
                            .polynomial
                            .monomials()
                            .iter()
                            .map(|mono| (mono.var_index(), -*mono.coefficient())),
                        rhs_ge,
                        dual_ge,
                        fixed_variables,
                        &mut rhs_linear_coefficients,
                        &mut rhs_constant,
                    )?;
                }
            }
        }

        for (constraint_index, constraint) in self.basic.quadratic_constraints().iter().enumerate()
        {
            let inequality = &constraint.inequality;
            let dual = quadratic_row_duals[constraint_index];
            let constant = *inequality.polynomial.constant();
            match inequality.relation {
                ConstraintRelation::LessEqual => {
                    let rhs = inequality.rhs - constant;
                    self.push_optimal_quadratic_cut_row(
                        inequality.polynomial.monomials().iter().map(|mono| {
                            (mono.var_index1(), mono.var_index2(), *mono.coefficient())
                        }),
                        rhs,
                        dual,
                        fixed_variables,
                        &mut rhs_linear_coefficients,
                        &mut rhs_quadratic_coefficients,
                        &mut rhs_constant,
                    )?;
                }
                ConstraintRelation::GreaterEqual => {
                    let rhs = constant - inequality.rhs;
                    self.push_optimal_quadratic_cut_row(
                        inequality.polynomial.monomials().iter().map(|mono| {
                            (mono.var_index1(), mono.var_index2(), -*mono.coefficient())
                        }),
                        rhs,
                        dual,
                        fixed_variables,
                        &mut rhs_linear_coefficients,
                        &mut rhs_quadratic_coefficients,
                        &mut rhs_constant,
                    )?;
                }
                ConstraintRelation::Equal => {
                    let rhs_le = inequality.rhs - constant;
                    self.push_optimal_quadratic_cut_row(
                        inequality.polynomial.monomials().iter().map(|mono| {
                            (mono.var_index1(), mono.var_index2(), *mono.coefficient())
                        }),
                        rhs_le,
                        dual,
                        fixed_variables,
                        &mut rhs_linear_coefficients,
                        &mut rhs_quadratic_coefficients,
                        &mut rhs_constant,
                    )?;

                    let rhs_ge = constant - inequality.rhs;
                    self.push_optimal_quadratic_cut_row(
                        inequality.polynomial.monomials().iter().map(|mono| {
                            (mono.var_index1(), mono.var_index2(), -*mono.coefficient())
                        }),
                        rhs_ge,
                        dual,
                        fixed_variables,
                        &mut rhs_linear_coefficients,
                        &mut rhs_quadratic_coefficients,
                        &mut rhs_constant,
                    )?;
                }
            }
        }

        let mut lhs_linear_coefficients: HashMap<usize, f64> = HashMap::new();
        lhs_linear_coefficients.insert(objective_variable_index, 1.0);
        for (var_index, rhs_coeff) in rhs_linear_coefficients {
            let lhs_coeff = -rhs_coeff;
            if !Self::is_zero(lhs_coeff) {
                let entry = lhs_linear_coefficients.entry(var_index).or_insert(0.0);
                *entry += lhs_coeff;
            }
        }

        let mut lhs_quadratic_coefficients: HashMap<(usize, usize), f64> = HashMap::new();
        for (key, rhs_coeff) in rhs_quadratic_coefficients {
            let lhs_coeff = -rhs_coeff;
            if !Self::is_zero(lhs_coeff) {
                let entry = lhs_quadratic_coefficients.entry(key).or_insert(0.0);
                *entry += lhs_coeff;
            }
        }

        let mut monomials: Vec<QuadraticMonomial<f64>> = Vec::new();
        let mut sorted_linear_indexes: Vec<usize> =
            lhs_linear_coefficients.keys().copied().collect();
        sorted_linear_indexes.sort_unstable();
        for var_index in sorted_linear_indexes {
            let coefficient = lhs_linear_coefficients
                .get(&var_index)
                .copied()
                .unwrap_or(0.0);
            if !Self::is_zero(coefficient) {
                monomials.push(QuadraticMonomial::new_linear(coefficient, var_index));
            }
        }

        let mut sorted_quadratic_indexes: Vec<(usize, usize)> =
            lhs_quadratic_coefficients.keys().copied().collect();
        sorted_quadratic_indexes.sort_unstable();
        for (var_index1, var_index2) in sorted_quadratic_indexes {
            let coefficient = lhs_quadratic_coefficients
                .get(&(var_index1, var_index2))
                .copied()
                .unwrap_or(0.0);
            if !Self::is_zero(coefficient) {
                monomials.push(QuadraticMonomial::new_quadratic(
                    coefficient,
                    var_index1,
                    var_index2,
                ));
            }
        }

        let relation = match self.objective.category {
            ObjectiveCategory::Maximum => ConstraintRelation::LessEqual,
            ObjectiveCategory::Minimum => ConstraintRelation::GreaterEqual,
        };
        Ok(QuadraticInequality::new(
            Quadratic::new(monomials, 0.0),
            relation,
            rhs_constant,
        ))
    }

    /// 生成二次最优性割列表（按展开行对偶）/
    /// Generate quadratic optimal cut list from expanded row duals
    pub fn generate_optimal_quadratic_cuts_from_row_duals(
        &self,
        objective_variable_index: usize,
        fixed_variables: &HashMap<usize, f64>,
        linear_row_duals: &[f64],
        quadratic_row_duals: &[f64],
    ) -> Result<Vec<QuadraticInequality<f64>>> {
        Ok(vec![self.generate_optimal_quadratic_cut_from_row_duals(
            objective_variable_index,
            fixed_variables,
            linear_row_duals,
            quadratic_row_duals,
        )?])
    }

    /// 生成二次最优性割（按变量 ID 固定值 + 线性/二次对偶）/
    /// Generate quadratic optimality cut from fixed variable IDs and linear/quadratic row duals
    pub fn generate_optimal_quadratic_cut_from_row_duals_by_id(
        &self,
        objective_variable: VariableId,
        fixed_variables: &HashMap<VariableId, f64>,
        linear_row_duals: &[f64],
        quadratic_row_duals: &[f64],
    ) -> Result<QuadraticInequality<f64>> {
        let objective_var_index = self
            .find_token(objective_variable)
            .ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "objective variable {} not found in mechanism model `{}`",
                    objective_variable, self.basic.name
                ))
            })?
            .solver_index;
        let normalized = self.normalize_fixed_variables_by_solver_index(fixed_variables)?;
        self.generate_optimal_quadratic_cut_from_row_duals(
            objective_var_index,
            &normalized,
            linear_row_duals,
            quadratic_row_duals,
        )
    }

    /// 生成二次最优性割列表（按变量 ID 固定值 + 线性/二次对偶）/
    /// Generate quadratic optimal cut list from fixed variable IDs and linear/quadratic row duals
    pub fn generate_optimal_quadratic_cuts_from_row_duals_by_id(
        &self,
        objective_variable: VariableId,
        fixed_variables: &HashMap<VariableId, f64>,
        linear_row_duals: &[f64],
        quadratic_row_duals: &[f64],
    ) -> Result<Vec<QuadraticInequality<f64>>> {
        Ok(vec![
            self.generate_optimal_quadratic_cut_from_row_duals_by_id(
                objective_variable,
                fixed_variables,
                linear_row_duals,
                quadratic_row_duals,
            )?,
        ])
    }

    /// 生成二次最优性割（直接消费求解器输出）/
    /// Generate quadratic optimality cut directly from solver output
    pub fn generate_optimal_quadratic_cut_from_output(
        &self,
        objective_variable: VariableId,
        fixed_variables: &HashMap<VariableId, f64>,
        output: &SolverOutput,
    ) -> Result<QuadraticInequality<f64>> {
        if !output.status.is_feasible() {
            return Err(ModelError::InvalidConstraint(format!(
                "solver output status {:?} is not feasible for quadratic benders cut generation",
                output.status
            ))
            .into());
        }
        let linear_duals = output.dual_solution.as_deref().ok_or_else(|| {
            ModelError::InvalidConstraint(
                "solver output has feasible status but missing linear dual solution vector"
                    .to_string(),
            )
        })?;
        let quadratic_duals = output.quadratic_dual_solution.as_deref().ok_or_else(|| {
            ModelError::InvalidConstraint(
                "solver output has feasible status but missing quadratic dual solution vector"
                    .to_string(),
            )
        })?;
        self.generate_optimal_quadratic_cut_from_row_duals_by_id(
            objective_variable,
            fixed_variables,
            linear_duals,
            quadratic_duals,
        )
    }

    /// 生成二次最优性割列表（直接消费求解器输出）/
    /// Generate quadratic optimal cut list directly from solver output
    pub fn generate_optimal_quadratic_cuts_from_output(
        &self,
        objective_variable: VariableId,
        fixed_variables: &HashMap<VariableId, f64>,
        output: &SolverOutput,
    ) -> Result<Vec<QuadraticInequality<f64>>> {
        Ok(vec![self.generate_optimal_quadratic_cut_from_output(
            objective_variable,
            fixed_variables,
            output,
        )?])
    }

    // ========================================================================
    // 模型转换 / Model Transformation
    // ========================================================================

    /// 转换为线性三角模型 / Convert to linear triad model
    ///
    /// 将机理模型转换为可求解的线性三角模型。
    /// Converts mechanism model to solvable linear triad model.
    ///
    /// # 转换规则 / Transformation Rules
    /// 1. 所有变量转换为 Token，保持索引映射
    /// 2. 约束统一转换为 <= 形式
    /// 3. 目标函数转换为系数向量
    ///
    /// # 约束转换 / Constraint Conversion
    /// - LessEqual: 直接使用
    /// - GreaterEqual: 乘以 -1 转换为 LessEqual
    /// - Equal: 拆分为两个约束（<= 和 >=）
    pub fn into_linear_triad_model(self) -> LinearTriadModel {
        self.try_into_linear_triad_model_with_status_callback(None)
            .unwrap_or_else(|err| {
                panic!(
                    "failed to convert MechanismModel into LinearTriadModel: {}",
                    err
                )
            })
    }

    pub fn try_into_linear_triad_model_with_status_callback(
        self,
        callback: Option<&ModelBuildingStatusCallback>,
    ) -> Result<LinearTriadModel> {
        let mut basic_linear = BasicLinearTriadModel::new(&self.basic.name);
        let model_name = self.basic.name.clone();
        let token_total = self.basic.tokens().len();
        Self::emit_model_building_status(
            callback,
            ModelBuildingStatus::new(
                model_name.clone(),
                ModelBuildingStage::FlattenLinearModel,
                0,
                token_total,
            ),
        )?;

        // 转换变量 / Convert variables
        for (token_index, token) in self.basic.tokens().iter().enumerate() {
            basic_linear.add_variable(token.clone());
            Self::emit_model_building_status(
                callback,
                ModelBuildingStatus::new(
                    model_name.clone(),
                    ModelBuildingStage::FlattenLinearModel,
                    token_index + 1,
                    token_total,
                ),
            )?;
        }

        // 转换约束 / Convert constraints
        let linear_constraint_total = self.basic.constraints().len();
        let mut linear_constraint_ready = 0usize;
        Self::emit_model_building_status(
            callback,
            ModelBuildingStatus::new(
                model_name.clone(),
                ModelBuildingStage::RegisterLinearConstraints,
                0,
                linear_constraint_total,
            ),
        )?;
        for constraint in self.basic.constraints() {
            let inequality = &constraint.inequality;
            let group_id = constraint.group.as_ref().map(|group| group.id);
            let lazy = constraint.lazy;
            let priority = constraint.priority;
            let args = constraint.args.clone();
            let source_symbol_id = constraint.from.as_ref().map(|symbol| symbol.id().id);
            let constant = *inequality.polynomial.constant_term();

            match inequality.relation {
                ConstraintRelation::LessEqual => {
                    // 直接添加 <= 约束
                    let mut row = SparseVector::new();
                    for mono in inequality.polynomial.monomials() {
                        row.add(mono.var_index(), *mono.coefficient());
                    }
                    basic_linear.add_constraint_with_metadata(
                        row,
                        inequality.rhs - constant,
                        constraint.name.clone(),
                        group_id,
                        lazy,
                        priority,
                        args.clone(),
                        source_symbol_id,
                    );
                }
                ConstraintRelation::GreaterEqual => {
                    // 转换 >= 为 <=：乘以 -1
                    let mut row = SparseVector::new();
                    for mono in inequality.polynomial.monomials() {
                        row.add(mono.var_index(), -*mono.coefficient());
                    }
                    basic_linear.add_constraint_with_metadata(
                        row,
                        constant - inequality.rhs,
                        constraint.name.clone(),
                        group_id,
                        lazy,
                        priority,
                        args.clone(),
                        source_symbol_id,
                    );
                }
                ConstraintRelation::Equal => {
                    // 拆分 == 为 <= 和 >=
                    // 第一个约束: <= rhs
                    let mut row1 = SparseVector::new();
                    for mono in inequality.polynomial.monomials() {
                        row1.add(mono.var_index(), *mono.coefficient());
                    }
                    basic_linear.add_constraint_with_metadata(
                        row1,
                        inequality.rhs - constant,
                        format!("{}_eq_le", constraint.name),
                        group_id,
                        lazy,
                        priority,
                        args.clone(),
                        source_symbol_id,
                    );

                    // 第二个约束: >= rhs (转换为 <= -rhs)
                    let mut row2 = SparseVector::new();
                    for mono in inequality.polynomial.monomials() {
                        row2.add(mono.var_index(), -*mono.coefficient());
                    }
                    basic_linear.add_constraint_with_metadata(
                        row2,
                        constant - inequality.rhs,
                        format!("{}_eq_ge", constraint.name),
                        group_id,
                        lazy,
                        priority,
                        args.clone(),
                        source_symbol_id,
                    );
                }
            }
            linear_constraint_ready += 1;
            Self::emit_model_building_status(
                callback,
                ModelBuildingStatus::new(
                    model_name.clone(),
                    ModelBuildingStage::RegisterLinearConstraints,
                    linear_constraint_ready,
                    linear_constraint_total,
                ),
            )?;
        }

        // 构建线性三角模型 / Build linear triad model
        let mut linear_model = LinearTriadModel::from_basic(basic_linear);
        Self::emit_model_building_status(
            callback,
            ModelBuildingStatus::new(model_name.clone(), ModelBuildingStage::BuildObjective, 0, 1),
        )?;

        // 转换目标函数 / Convert objective
        let obj = &self.objective;
        linear_model.objective_category = obj.category;

        if !obj.sub_objectives.is_empty() {
            let n = linear_model.basic.num_variables();
            let mut c = vec![0.0; n];

            for sub_obj in &obj.sub_objectives {
                let direction = if sub_obj.category == linear_model.objective_category {
                    1.0
                } else {
                    -1.0
                };
                let scale = direction * sub_obj.weight;

                for mono in sub_obj.polynomial.monomials() {
                    let idx = mono.var_index();
                    if idx < n {
                        c[idx] += *mono.coefficient() * scale;
                    }
                }
            }

            linear_model.c = c;
        }
        Self::emit_model_building_status(
            callback,
            ModelBuildingStatus::new(model_name, ModelBuildingStage::BuildObjective, 1, 1),
        )?;

        Ok(linear_model)
    }

    pub fn into_quadratic_tetrad_model(self) -> QuadraticTetradModel {
        self.try_into_quadratic_tetrad_model_with_status_callback(None)
            .unwrap_or_else(|err| {
                panic!(
                    "failed to convert MechanismModel into QuadraticTetradModel: {}",
                    err
                )
            })
    }

    pub fn try_into_quadratic_tetrad_model_with_status_callback(
        self,
        callback: Option<&ModelBuildingStatusCallback>,
    ) -> Result<QuadraticTetradModel> {
        let model_name = self.basic.name.clone();
        let linear = self
            .clone()
            .try_into_linear_triad_model_with_status_callback(callback)?;
        let mut quadratic = QuadraticTetradModel::from_basic(
            BasicQuadraticTetradModel::from_linear(linear.basic.clone()),
        );
        quadratic.c = linear.c;
        quadratic.objective_category = linear.objective_category;

        let mut q_objective = SparseMatrix::new();
        for _ in 0..quadratic.basic.num_variables() {
            q_objective.add_row(SparseVector::new());
        }
        quadratic.Q = q_objective;

        let quadratic_constraint_total = self.basic.quadratic_constraints().len();
        Self::emit_model_building_status(
            callback,
            ModelBuildingStatus::new(
                model_name.clone(),
                ModelBuildingStage::FlattenQuadraticModel,
                0,
                quadratic_constraint_total,
            ),
        )?;
        for (constraint_index, constraint) in self.basic.quadratic_constraints().iter().enumerate()
        {
            let group_id = constraint.group.as_ref().map(|group| group.id);
            let source_symbol_id = constraint.from.as_ref().map(|symbol| symbol.id().id);
            quadratic.add_quadratic_constraint_with_metadata(
                constraint.inequality.clone(),
                constraint.name.clone(),
                group_id,
                constraint.lazy,
                constraint.priority,
                constraint.args.clone(),
                source_symbol_id,
            );
            Self::emit_model_building_status(
                callback,
                ModelBuildingStatus::new(
                    model_name.clone(),
                    ModelBuildingStage::FlattenQuadraticModel,
                    constraint_index + 1,
                    quadratic_constraint_total,
                ),
            )?;
        }

        Ok(quadratic)
    }
}

impl<V> std::ops::Deref for MechanismModel<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Target = BasicMechanismModel<V>;
    fn deref(&self) -> &Self::Target {
        &self.basic
    }
}

impl<V> std::ops::DerefMut for MechanismModel<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.basic
    }
}

impl Default for MechanismModel<f64> {
    fn default() -> Self {
        Self::new("default")
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::model::{
        LinearConstraint, LinearInequality, ModelBuildingStage, ModelBuildingStatusCallback,
        QuadraticConstraint, QuadraticInequality,
    };
    use crate::solver::{SolverOutput, SolverStatus};
    use crate::token::Token;
    use crate::variable::ContinuousVariableItem;

    fn lhs_value(inequality: &LinearInequality<f64>, values: &HashMap<usize, f64>) -> f64 {
        let mut value = *inequality.polynomial.constant_term();
        for monomial in inequality.polynomial.monomials() {
            value +=
                *monomial.coefficient() * values.get(&monomial.var_index()).copied().unwrap_or(0.0);
        }
        value
    }

    fn satisfies(inequality: &LinearInequality<f64>, values: &HashMap<usize, f64>) -> bool {
        let lhs = lhs_value(inequality, values);
        match inequality.relation {
            ConstraintRelation::LessEqual => lhs <= inequality.rhs + 1e-9,
            ConstraintRelation::GreaterEqual => lhs + 1e-9 >= inequality.rhs,
            ConstraintRelation::Equal => (lhs - inequality.rhs).abs() <= 1e-9,
        }
    }

    fn quadratic_lhs_value(
        inequality: &QuadraticInequality<f64>,
        values: &HashMap<usize, f64>,
    ) -> f64 {
        let mut value = *inequality.polynomial.constant();
        for monomial in inequality.polynomial.monomials() {
            let coefficient = *monomial.coefficient();
            let left = values.get(&monomial.var_index1()).copied().unwrap_or(0.0);
            if let Some(var_index2) = monomial.var_index2() {
                let right = values.get(&var_index2).copied().unwrap_or(0.0);
                value += coefficient * left * right;
            } else {
                value += coefficient * left;
            }
        }
        value
    }

    fn quadratic_satisfies(
        inequality: &QuadraticInequality<f64>,
        values: &HashMap<usize, f64>,
    ) -> bool {
        let lhs = quadratic_lhs_value(inequality, values);
        match inequality.relation {
            ConstraintRelation::LessEqual => lhs <= inequality.rhs + 1e-9,
            ConstraintRelation::GreaterEqual => lhs + 1e-9 >= inequality.rhs,
            ConstraintRelation::Equal => (lhs - inequality.rhs).abs() <= 1e-9,
        }
    }

    #[test]
    fn feasibility_cut_is_generated_on_mechanism_layer_from_row_duals() {
        let x = ContinuousVariableItem::auto("x");
        let x_id = x.id();
        let y = ContinuousVariableItem::auto("y");

        let mut basic = BasicMechanismModel::new("cut_template");
        basic.add_token(Token::from_generic(x, 0));
        basic.add_token(Token::from_generic(y, 1));

        let c1 = LinearConstraint::new(
            LinearInequality::less_equal(
                Linear::new(
                    vec![LinearMonomial::new(1.0, 0), LinearMonomial::new(-1.0, 1)],
                    0.0,
                ),
                0.0,
            ),
            "recourse_link",
        );
        basic.add_constraint(c1);
        let c2 = LinearConstraint::new(
            LinearInequality::less_equal(Linear::new(vec![LinearMonomial::new(1.0, 1)], 0.0), 1.0),
            "recourse_cap",
        );
        basic.add_constraint(c2);

        let mechanism = MechanismModel::from_basic(basic);
        let fixed = HashMap::from([(x_id, 2.0)]);
        let row_duals = vec![1.0, 1.0];

        let cut = mechanism
            .generate_feasibility_cut_from_row_duals_by_id(&fixed, &row_duals)
            .expect("mechanism should generate feasibility cut from row duals");
        assert_eq!(cut.relation, ConstraintRelation::LessEqual);

        let violated = lhs_value(&cut, &HashMap::from([(0usize, 2.0)]));
        assert!(
            violated > 1e-6,
            "cut should violate current fixed point: {}",
            violated
        );

        let feasible = lhs_value(&cut, &HashMap::from([(0usize, 1.0)]));
        assert!(
            feasible <= 1e-6,
            "cut should keep feasible point: {}",
            feasible
        );
    }

    #[test]
    fn optimal_cut_is_generated_on_mechanism_layer_from_row_duals() {
        let x = ContinuousVariableItem::auto("x");
        let x_id = x.id();
        let y = ContinuousVariableItem::auto("y");
        let theta = ContinuousVariableItem::auto("theta");
        let theta_id = theta.id();

        let mut basic = BasicMechanismModel::new("optimal_cut_template");
        basic.add_token(Token::from_generic(x, 0));
        basic.add_token(Token::from_generic(y, 1));
        basic.add_token(Token::from_generic(theta, 2));
        basic.add_constraint(LinearConstraint::new(
            LinearInequality::less_equal(
                Linear::new(
                    vec![LinearMonomial::new(1.0, 0), LinearMonomial::new(-1.0, 1)],
                    0.0,
                ),
                0.0,
            ),
            "link_xy",
        ));

        let mechanism = MechanismModel::from_basic(basic);
        let cut = mechanism
            .generate_optimal_cut_from_row_duals_by_id(
                theta_id,
                &HashMap::from([(x_id, 2.0)]),
                &[-1.0],
            )
            .expect("mechanism should generate optimal cut from row duals");
        assert_eq!(cut.relation, ConstraintRelation::GreaterEqual);

        assert!(
            !satisfies(&cut, &HashMap::from([(0usize, 2.0), (2usize, 1.0)])),
            "theta=1 should violate cut at x=2"
        );
        assert!(
            satisfies(&cut, &HashMap::from([(0usize, 2.0), (2usize, 2.0)])),
            "theta=2 should satisfy cut at x=2"
        );
    }

    #[test]
    fn benders_unified_api_dispatches_both_cut_kinds() {
        let x = ContinuousVariableItem::auto("x");
        let x_id = x.id();
        let y = ContinuousVariableItem::auto("y");
        let theta = ContinuousVariableItem::auto("theta");
        let theta_id = theta.id();

        let mut basic = BasicMechanismModel::new("benders_unified_template");
        basic.add_token(Token::from_generic(x, 0));
        basic.add_token(Token::from_generic(y, 1));
        basic.add_token(Token::from_generic(theta, 2));
        basic.add_constraint(LinearConstraint::new(
            LinearInequality::less_equal(
                Linear::new(
                    vec![LinearMonomial::new(1.0, 0), LinearMonomial::new(-1.0, 1)],
                    0.0,
                ),
                0.0,
            ),
            "link_xy",
        ));
        basic.add_constraint(LinearConstraint::new(
            LinearInequality::less_equal(Linear::new(vec![LinearMonomial::new(1.0, 1)], 0.0), 1.0),
            "cap_y",
        ));

        let mechanism = MechanismModel::from_basic(basic);

        let feasibility_cuts = mechanism
            .generate_benders_cuts_from_solution(
                BendersCutRequest::Feasibility,
                &HashMap::from([(x_id, 2.0)]),
                &[1.0, 1.0],
            )
            .expect("unified API should generate feasibility cut");
        assert_eq!(feasibility_cuts.len(), 1);
        assert_eq!(feasibility_cuts[0].relation, ConstraintRelation::LessEqual);

        let optimal_cuts = mechanism
            .generate_benders_cuts_from_solution(
                BendersCutRequest::Optimality {
                    objective_variable: theta_id,
                },
                &HashMap::from([(x_id, 2.0)]),
                &[-1.0, 0.0],
            )
            .expect("unified API should generate optimal cut");
        assert_eq!(optimal_cuts.len(), 1);
        assert_eq!(optimal_cuts[0].relation, ConstraintRelation::GreaterEqual);
    }

    #[test]
    fn benders_output_api_validates_status_and_solution() {
        let x = ContinuousVariableItem::auto("x");
        let x_id = x.id();
        let y = ContinuousVariableItem::auto("y");
        let theta = ContinuousVariableItem::auto("theta");
        let theta_id = theta.id();

        let mut basic = BasicMechanismModel::new("benders_output_template");
        basic.add_token(Token::from_generic(x, 0));
        basic.add_token(Token::from_generic(y, 1));
        basic.add_token(Token::from_generic(theta, 2));
        basic.add_constraint(LinearConstraint::new(
            LinearInequality::less_equal(
                Linear::new(
                    vec![LinearMonomial::new(1.0, 0), LinearMonomial::new(-1.0, 1)],
                    0.0,
                ),
                0.0,
            ),
            "link_xy",
        ));
        basic.add_constraint(LinearConstraint::new(
            LinearInequality::less_equal(Linear::new(vec![LinearMonomial::new(1.0, 1)], 0.0), 1.0),
            "cap_y",
        ));
        let mechanism = MechanismModel::from_basic(basic);
        let fixed = HashMap::from([(x_id, 2.0)]);

        let infeasible_output = SolverOutput::new(SolverStatus::Infeasible);
        let err = mechanism
            .generate_feasibility_cuts_from_farkas_output(&fixed, &infeasible_output)
            .expect_err("infeasible solver output should be rejected");
        assert!(
            err.to_string().contains("not feasible"),
            "unexpected error: {}",
            err
        );

        let missing_dual_and_solution_output = SolverOutput::new(SolverStatus::Optimal);
        let err = mechanism
            .generate_optimal_cuts_from_dual_output(
                theta_id,
                &fixed,
                &missing_dual_and_solution_output,
            )
            .expect_err("missing dual/solution should be rejected");
        assert!(
            err.to_string().contains("missing dual/solution"),
            "unexpected error: {}",
            err
        );

        let dual_only_output = SolverOutput::new(SolverStatus::Optimal).with_dual(vec![-1.0, 0.0]);
        let cuts = mechanism
            .generate_optimal_cuts_from_dual_output(theta_id, &fixed, &dual_only_output)
            .expect("dual-only output should generate optimal cut");
        assert_eq!(cuts.len(), 1);

        let usable_output = SolverOutput::optimal(0.0, vec![1.0, 1.0]);
        let cuts = mechanism
            .generate_feasibility_cuts_from_farkas_output(&fixed, &usable_output)
            .expect("valid solver output should generate feasibility cut");
        assert_eq!(cuts.len(), 1);
    }

    #[test]
    fn quadratic_optimal_cut_can_be_generated_from_linear_and_quadratic_duals() {
        let x = ContinuousVariableItem::auto("qcut_x");
        let x_id = x.id();
        let y = ContinuousVariableItem::auto("qcut_y");
        let theta = ContinuousVariableItem::auto("qcut_theta");
        let theta_id = theta.id();

        let mut basic = BasicMechanismModel::new("quadratic_optimal_cut_template");
        basic.add_token(Token::from_generic(x, 0));
        basic.add_token(Token::from_generic(y, 1));
        basic.add_token(Token::from_generic(theta, 2));
        basic.add_constraint(LinearConstraint::new(
            LinearInequality::less_equal(
                Linear::new(
                    vec![LinearMonomial::new(1.0, 0), LinearMonomial::new(-1.0, 1)],
                    0.0,
                ),
                0.0,
            ),
            "link_xy",
        ));
        basic.add_quadratic_constraint(QuadraticConstraint::new(
            QuadraticInequality::new(
                crate::symbol::flatten::Quadratic::new(
                    vec![crate::symbol::flatten::QuadraticMonomial::new_quadratic(
                        1.0, 0, 0,
                    )],
                    0.0,
                ),
                ConstraintRelation::LessEqual,
                4.0,
            ),
            "quad_cap",
        ));

        let mechanism = MechanismModel::from_basic(basic);
        let cut = mechanism
            .generate_optimal_quadratic_cut_from_row_duals_by_id(
                theta_id,
                &HashMap::from([(x_id, 2.0)]),
                &[-1.0],
                &[0.5],
            )
            .expect("mechanism should generate quadratic optimal cut from duals");
        assert_eq!(cut.relation, ConstraintRelation::GreaterEqual);

        assert!(
            !quadratic_satisfies(&cut, &HashMap::from([(0usize, 2.0), (2usize, 1.0)])),
            "theta=1 should violate quadratic cut at x=2"
        );
        assert!(
            quadratic_satisfies(&cut, &HashMap::from([(0usize, 2.0), (2usize, 2.0)])),
            "theta=2 should satisfy quadratic cut at x=2"
        );

        let cuts = mechanism
            .generate_optimal_quadratic_cuts_from_row_duals_by_id(
                theta_id,
                &HashMap::from([(x_id, 2.0)]),
                &[-1.0],
                &[0.5],
            )
            .expect("quadratic cut list generation should succeed");
        assert_eq!(cuts.len(), 1);
        assert_eq!(cuts[0].relation, ConstraintRelation::GreaterEqual);
    }

    #[test]
    fn quadratic_benders_output_api_validates_dual_vectors() {
        let x = ContinuousVariableItem::auto("qout_x");
        let x_id = x.id();
        let y = ContinuousVariableItem::auto("qout_y");
        let theta = ContinuousVariableItem::auto("qout_theta");
        let theta_id = theta.id();

        let mut basic = BasicMechanismModel::new("quadratic_benders_output_template");
        basic.add_token(Token::from_generic(x, 0));
        basic.add_token(Token::from_generic(y, 1));
        basic.add_token(Token::from_generic(theta, 2));
        basic.add_constraint(LinearConstraint::new(
            LinearInequality::less_equal(
                Linear::new(
                    vec![LinearMonomial::new(1.0, 0), LinearMonomial::new(-1.0, 1)],
                    0.0,
                ),
                0.0,
            ),
            "link_xy",
        ));
        basic.add_quadratic_constraint(QuadraticConstraint::new(
            QuadraticInequality::new(
                crate::symbol::flatten::Quadratic::new(
                    vec![crate::symbol::flatten::QuadraticMonomial::new_quadratic(
                        1.0, 0, 0,
                    )],
                    0.0,
                ),
                ConstraintRelation::LessEqual,
                4.0,
            ),
            "quad_cap",
        ));
        let mechanism = MechanismModel::from_basic(basic);
        let fixed = HashMap::from([(x_id, 2.0)]);

        let missing_linear_dual_output = SolverOutput::new(SolverStatus::Optimal)
            .with_solution(vec![0.0])
            .with_quadratic_dual(vec![0.5]);
        let err = mechanism
            .generate_optimal_quadratic_cut_from_output(
                theta_id,
                &fixed,
                &missing_linear_dual_output,
            )
            .expect_err("missing linear dual vector should be rejected");
        assert!(
            err.to_string().contains("missing linear dual solution"),
            "unexpected error: {}",
            err
        );

        let missing_quadratic_dual_output = SolverOutput::new(SolverStatus::Optimal)
            .with_solution(vec![0.0])
            .with_dual(vec![-1.0]);
        let err = mechanism
            .generate_optimal_quadratic_cut_from_output(
                theta_id,
                &fixed,
                &missing_quadratic_dual_output,
            )
            .expect_err("missing quadratic dual vector should be rejected");
        assert!(
            err.to_string().contains("missing quadratic dual solution"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn mechanism_linear_flatten_status_callback_is_invoked() {
        let x = ContinuousVariableItem::auto("mx");
        let mut basic = BasicMechanismModel::new("mechanism_linear_status");
        basic.add_token(Token::from_generic(x, 0));
        basic.add_constraint(LinearConstraint::new(
            LinearInequality::less_equal(Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0), 1.0),
            "mx_c",
        ));
        let mechanism = MechanismModel::from_basic(basic);

        let statuses = Arc::new(Mutex::new(Vec::new()));
        let statuses_for_callback = statuses.clone();
        let callback: ModelBuildingStatusCallback = Arc::new(move |status| {
            statuses_for_callback.lock().unwrap().push(status.clone());
            Ok(())
        });

        let linear = mechanism
            .try_into_linear_triad_model_with_status_callback(Some(&callback))
            .expect("linear flatten with status callback should succeed");
        assert_eq!(linear.num_variables(), 1);
        let statuses = statuses.lock().unwrap();
        assert!(
            statuses
                .iter()
                .any(|status| status.stage == ModelBuildingStage::FlattenLinearModel)
        );
        assert!(
            statuses
                .iter()
                .any(|status| status.stage == ModelBuildingStage::BuildObjective)
        );
    }

    #[test]
    fn mechanism_quadratic_flatten_status_callback_is_invoked() {
        let x = ContinuousVariableItem::auto("qx");
        let mut basic = BasicMechanismModel::new("mechanism_quadratic_status");
        basic.add_token(Token::from_generic(x, 0));
        basic.add_constraint(LinearConstraint::new(
            LinearInequality::less_equal(Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0), 2.0),
            "qx_c",
        ));
        let mechanism = MechanismModel::from_basic(basic);

        let statuses = Arc::new(Mutex::new(Vec::new()));
        let statuses_for_callback = statuses.clone();
        let callback: ModelBuildingStatusCallback = Arc::new(move |status| {
            statuses_for_callback.lock().unwrap().push(status.clone());
            Ok(())
        });

        let quadratic = mechanism
            .try_into_quadratic_tetrad_model_with_status_callback(Some(&callback))
            .expect("quadratic flatten with status callback should succeed");
        assert_eq!(quadratic.num_variables(), 1);
        let statuses = statuses.lock().unwrap();
        assert!(
            statuses
                .iter()
                .any(|status| status.stage == ModelBuildingStage::FlattenQuadraticModel)
        );
    }
}

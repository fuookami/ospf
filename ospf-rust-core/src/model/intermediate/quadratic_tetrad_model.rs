//! 二次四角模型
//! Quadratic Tetrad Model

use super::super::mechanism::QuadraticInequality;
use super::{BasicQuadraticTetradModel, SparseMatrix};
use crate::model::object::ObjectiveCategory;
use crate::variable::VariableType;

/// 二次四角模型 / Quadratic Tetrad Model
///
/// 继承基本二次四角模型，添加目标函数支持。
/// Inherits basic quadratic tetrad model, adding objective function support.
///
/// 标准形式: min/max x^T Q x + c^T x s.t. Ax ≤ b, x ∈ [lb, ub]
/// Standard form: min/max x^T Q x + c^T x s.t. Ax ≤ b, x ∈ [lb, ub]
#[allow(non_snake_case)]
#[derive(Debug, Clone)]
pub struct QuadraticTetradModel {
    /// 基本二次四角模型 / Basic quadratic tetrad model
    pub basic: BasicQuadraticTetradModel,
    /// 线性目标系数 / Linear objective coefficients
    pub c: Vec<f64>,
    /// 二次目标矩阵 / Quadratic objective matrix
    pub Q: SparseMatrix<f64>,
    /// 二次约束 / Quadratic constraints
    pub quadratic_constraints: Vec<QuadraticInequality<f64>>,
    pub quadratic_constraint_names: Vec<String>,
    pub quadratic_constraint_group_ids: Vec<Option<u64>>,
    pub quadratic_constraint_lazy_flags: Vec<bool>,
    pub quadratic_constraint_priorities: Vec<u32>,
    pub quadratic_constraint_args: Vec<Option<String>>,
    pub quadratic_constraint_source_symbol_ids: Vec<Option<u64>>,
    /// 目标方向 / Objective direction
    pub objective_category: ObjectiveCategory,
}

impl QuadraticTetradModel {
    fn relaxed_variable_type(variable_type: VariableType) -> VariableType {
        match variable_type {
            VariableType::Binary => VariableType::Percentage,
            VariableType::Ternary | VariableType::UInteger => VariableType::UContinuous,
            VariableType::BalancedTernary | VariableType::Integer => VariableType::Continuous,
            variable_type => variable_type,
        }
    }

    /// 创建空模型 / Create empty model
    pub fn new(name: &str) -> Self {
        Self {
            basic: BasicQuadraticTetradModel::new(name),
            c: Vec::new(),
            Q: SparseMatrix::new(),
            quadratic_constraints: Vec::new(),
            quadratic_constraint_names: Vec::new(),
            quadratic_constraint_group_ids: Vec::new(),
            quadratic_constraint_lazy_flags: Vec::new(),
            quadratic_constraint_priorities: Vec::new(),
            quadratic_constraint_args: Vec::new(),
            quadratic_constraint_source_symbol_ids: Vec::new(),
            objective_category: ObjectiveCategory::Minimum,
        }
    }

    /// 从基本二次四角模型创建 / Create from basic quadratic tetrad model
    pub fn from_basic(basic: BasicQuadraticTetradModel) -> Self {
        let n = basic.num_variables();
        Self {
            basic,
            c: vec![0.0; n],
            Q: SparseMatrix::new(),
            quadratic_constraints: Vec::new(),
            quadratic_constraint_names: Vec::new(),
            quadratic_constraint_group_ids: Vec::new(),
            quadratic_constraint_lazy_flags: Vec::new(),
            quadratic_constraint_priorities: Vec::new(),
            quadratic_constraint_args: Vec::new(),
            quadratic_constraint_source_symbol_ids: Vec::new(),
            objective_category: ObjectiveCategory::Minimum,
        }
    }

    /// 设置目标函数 / Set objective
    #[allow(non_snake_case)]
    pub fn set_objective(
        &mut self,
        c: Vec<f64>,
        Q: SparseMatrix<f64>,
        category: ObjectiveCategory,
    ) {
        self.c = c;
        self.Q = Q;
        self.objective_category = category;
    }

    /// 设置线性目标系数 / Set linear objective coefficients
    pub fn set_linear_objective(&mut self, c: Vec<f64>) {
        self.c = c;
    }

    /// 设置二次目标矩阵 / Set quadratic objective matrix
    #[allow(non_snake_case)]
    pub fn set_quadratic_objective(&mut self, Q: SparseMatrix<f64>) {
        self.Q = Q;
    }

    pub fn add_quadratic_constraint_with_metadata(
        &mut self,
        inequality: QuadraticInequality<f64>,
        name: String,
        group_id: Option<u64>,
        lazy: bool,
        priority: u32,
        args: Option<String>,
        source_symbol_id: Option<u64>,
    ) -> usize {
        let idx = self.quadratic_constraints.len();
        self.quadratic_constraints.push(inequality);
        self.quadratic_constraint_names.push(name);
        self.quadratic_constraint_group_ids.push(group_id);
        self.quadratic_constraint_lazy_flags.push(lazy);
        self.quadratic_constraint_priorities.push(priority);
        self.quadratic_constraint_args.push(args);
        self.quadratic_constraint_source_symbol_ids
            .push(source_symbol_id);
        idx
    }

    /// 就地线性松弛 / In-place linear relaxation
    pub fn linear_relax(&mut self) {
        for variable_type in &mut self.basic.linear.var_types {
            *variable_type = Self::relaxed_variable_type(*variable_type);
        }
    }

    /// 返回松弛副本 / Return relaxed copy
    pub fn linear_relaxed(&self) -> Self {
        let mut relaxed = self.clone();
        relaxed.linear_relax();
        relaxed
    }

    /// 可行性模型（去目标）/ Feasibility model (objective removed)
    #[allow(non_snake_case)]
    pub fn feasibility(&self) -> Self {
        let mut model = self.clone();
        model.c = vec![0.0; model.num_variables()];
        model.Q = SparseMatrix::new();
        model.objective_category = ObjectiveCategory::Minimum;
        model
    }

    /// 设置目标方向 / Set objective direction
    pub fn set_objective_category(&mut self, category: ObjectiveCategory) {
        self.objective_category = category;
    }

    /// 获取线性目标系数 / Get linear objective coefficients
    pub fn linear_objective(&self) -> &[f64] {
        &self.c
    }

    /// 获取二次目标矩阵 / Get quadratic objective matrix
    pub fn quadratic_objective(&self) -> &SparseMatrix<f64> {
        &self.Q
    }

    pub fn num_quadratic_constraints(&self) -> usize {
        self.quadratic_constraints.len()
    }

    /// 获取基本模型引用 / Get basic model reference
    pub fn as_basic(&self) -> &BasicQuadraticTetradModel {
        &self.basic
    }

    /// 提取基本模型（消耗 self）/ Extract basic model (consumes self)
    pub fn into_basic(self) -> BasicQuadraticTetradModel {
        self.basic
    }
}

impl std::ops::Deref for QuadraticTetradModel {
    type Target = BasicQuadraticTetradModel;
    fn deref(&self) -> &Self::Target {
        &self.basic
    }
}

impl std::ops::DerefMut for QuadraticTetradModel {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.basic
    }
}

impl Default for QuadraticTetradModel {
    fn default() -> Self {
        Self::new("default")
    }
}

/// f64 精度的二次四角模型 / Quadratic tetrad model with f64 precision
pub type QuadraticTetradModelF64 = QuadraticTetradModel;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::Token;
    use crate::variable::{BinaryVariableItem, UContinuousVariableItem};

    #[test]
    fn linear_relaxed_converts_integer_types() {
        let mut basic = BasicQuadraticTetradModel::new("quadratic_relax");
        basic.linear.add_variable_with_bounds(
            Token::from_generic(BinaryVariableItem::auto("b"), 0),
            0.0,
            1.0,
            VariableType::Binary,
        );
        basic.linear.add_variable_with_bounds(
            Token::from_generic(UContinuousVariableItem::auto("x"), 1),
            0.0,
            10.0,
            VariableType::UContinuous,
        );

        let model = QuadraticTetradModel::from_basic(basic);
        let relaxed = model.linear_relaxed();
        assert_eq!(relaxed.basic.linear.var_types[0], VariableType::Percentage);
        assert_eq!(relaxed.basic.linear.var_types[1], VariableType::UContinuous);
    }

    #[test]
    fn feasibility_model_clears_linear_and_quadratic_objective() {
        let mut basic = BasicQuadraticTetradModel::new("quadratic_feasibility");
        basic.linear.add_variable_with_bounds(
            Token::from_generic(UContinuousVariableItem::auto("x"), 0),
            0.0,
            10.0,
            VariableType::UContinuous,
        );
        let mut model = QuadraticTetradModel::from_basic(basic);
        model.set_objective(vec![4.0], SparseMatrix::new(), ObjectiveCategory::Maximum);

        let feasibility = model.feasibility();
        assert_eq!(feasibility.objective_category, ObjectiveCategory::Minimum);
        assert_eq!(feasibility.c, vec![0.0]);
        assert_eq!(feasibility.Q.rows(), 0);
    }
}

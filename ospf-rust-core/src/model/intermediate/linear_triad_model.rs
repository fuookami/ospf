//! 线性三角模型
//! Linear Triad Model

use super::super::object::ObjectiveCategory;
use super::{BasicLinearTriadModel, SparseVector};
use crate::token::Token;
use crate::variable::{ContinuousVariableItem, VariableId, VariableType};

/// 线性三角模型 / Linear Triad Model
///
/// 继承基本线性三角模型，添加目标函数支持。
/// Inherits basic linear triad model, adding objective function support.
///
/// 标准形式: min/max c^T x s.t. Ax ≤ b, x ∈ [lb, ub]
/// Standard form: min/max c^T x s.t. Ax ≤ b, x ∈ [lb, ub]
#[derive(Debug, Clone)]
pub struct LinearTriadModel {
    /// 基本线性三角模型 / Basic linear triad model
    pub basic: BasicLinearTriadModel,
    /// 目标函数系数 / Objective coefficients
    pub c: Vec<f64>,
    /// 目标方向 / Objective direction
    pub objective_category: ObjectiveCategory,
}

impl LinearTriadModel {
    fn relaxed_variable_type(variable_type: VariableType) -> VariableType {
        match variable_type {
            VariableType::Binary => VariableType::Percentage,
            VariableType::Ternary | VariableType::UInteger => VariableType::UContinuous,
            VariableType::BalancedTernary | VariableType::Integer => VariableType::Continuous,
            variable_type => variable_type,
        }
    }

    /// 是否包含整数变量 / Whether model contains integer variables
    pub fn has_integer_variables(&self) -> bool {
        self.var_types.iter().any(VariableType::is_integer)
    }

    /// 就地线性松弛 / In-place linear relaxation
    pub fn linear_relax(&mut self) {
        for variable_type in &mut self.basic.var_types {
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
    pub fn feasibility(&self) -> Self {
        let mut model = self.clone();
        model.c = vec![0.0; model.num_variables()];
        model.objective_category = ObjectiveCategory::Minimum;
        model
    }

    /// 弹性模型（每条约束增加一个非负松弛变量）/ Elastic model (add one non-negative slack per constraint)
    pub fn elastic(&self, penalty: f64) -> Self {
        let mut model = self.clone();
        let original_constraints = model.num_constraints();
        let mut objective = model.c.clone();
        objective.resize(model.num_variables(), 0.0);

        let mut next_group_id = model
            .variables
            .iter()
            .map(|token| token.id().group_id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);

        for constraint_index in 0..original_constraints {
            let slack = ContinuousVariableItem::create(
                VariableId::standalone(next_group_id),
                &format!("{}_elastic_s{}", model.basic.name, constraint_index),
            );
            next_group_id = next_group_id.saturating_add(1);
            let solver_index = model.num_variables();

            let slack_index = model.add_variable_with_bounds(
                Token::from_generic(slack, solver_index),
                0.0,
                f64::INFINITY,
                VariableType::Continuous,
            );
            objective.push(penalty);

            if let Some(row) = model.A.rows.get_mut(constraint_index) {
                row.add(slack_index, -1.0);
            }
        }

        model.c = objective;
        model.objective_category = ObjectiveCategory::Minimum;
        model
    }

    /// 整理对偶解（过滤近零值）/ Tidy dual solution (filter near-zero entries)
    pub fn tidy_dual_solution(&self, solution: &[f64]) -> Vec<(usize, f64)> {
        solution
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, value)| value.abs() > f64::EPSILON)
            .collect()
    }

    /// 按来源符号整理对偶解 / Tidy dual solution by source symbol id
    pub fn tidy_dual_solution_by_source_symbol(&self, solution: &[f64]) -> Vec<(u64, f64)> {
        let mut compact = Vec::new();
        for (index, value) in solution.iter().copied().enumerate() {
            if value.abs() <= f64::EPSILON {
                continue;
            }
            if let Some(source_symbol_id) = self
                .basic
                .constraint_source_symbol_ids
                .get(index)
                .and_then(|item| *item)
            {
                compact.push((source_symbol_id, value));
            }
        }
        compact
    }

    fn is_positive_infinite(value: f64) -> bool {
        value.is_infinite() && value.is_sign_positive()
    }

    fn is_negative_infinite(value: f64) -> bool {
        value.is_infinite() && value.is_sign_negative()
    }

    fn is_zero(value: f64) -> bool {
        value.abs() <= f64::EPSILON
    }

    fn negate_row(row: &SparseVector<f64>) -> SparseVector<f64> {
        let mut neg = SparseVector::new();
        for (index, value) in &row.entries {
            neg.add(*index, -*value);
        }
        neg
    }

    /// 创建空模型 / Create empty model
    pub fn new(name: &str) -> Self {
        Self {
            basic: BasicLinearTriadModel::new(name),
            c: Vec::new(),
            objective_category: ObjectiveCategory::Minimum,
        }
    }

    /// 从基本线性三角模型创建 / Create from basic linear triad model
    pub fn from_basic(basic: BasicLinearTriadModel) -> Self {
        let n = basic.num_variables();
        Self {
            basic,
            c: vec![0.0; n],
            objective_category: ObjectiveCategory::Minimum,
        }
    }

    /// 设置目标函数 / Set objective
    pub fn set_objective(&mut self, c: Vec<f64>, category: ObjectiveCategory) {
        self.c = c;
        self.objective_category = category;
    }

    /// 设置目标系数 / Set objective coefficients
    pub fn set_objective_coeffs(&mut self, c: Vec<f64>) {
        self.c = c;
    }

    /// 设置目标方向 / Set objective direction
    pub fn set_objective_category(&mut self, category: ObjectiveCategory) {
        self.objective_category = category;
    }

    /// 获取目标函数系数 / Get objective coefficients
    pub fn objective(&self) -> &[f64] {
        &self.c
    }

    /// 获取基本模型引用（用于对偶转换）/ Get basic model reference (for dual transformation)
    pub fn as_basic(&self) -> &BasicLinearTriadModel {
        &self.basic
    }

    /// 提取基本模型（消耗 self）/ Extract basic model (consumes self)
    pub fn into_basic(self) -> BasicLinearTriadModel {
        self.basic
    }

    /// 生成对偶模型 / Generate dual model
    ///
    /// 参考 Kotlin 版本实现，支持：
    /// 1) 约束对偶变量（按目标方向决定符号）
    /// 2) 变量上下界的对偶变量
    /// 3) 变量归一化符号（x>=0/x<=0/free）映射到对偶约束方向
    pub fn to_dual(&self) -> Self {
        if self.has_integer_variables() {
            return self.linear_relaxed().to_dual();
        }

        #[derive(Clone, Copy, PartialEq, Eq)]
        enum VariableSignKind {
            Free,
            PositiveNormalized,
            NegativeNormalized,
            NonNormalized,
        }

        #[derive(Clone, Copy)]
        enum DualConstraintRelation {
            LessEqual,
            GreaterEqual,
            Equal,
        }

        let classify_variable_sign = |lb: f64, ub: f64| -> VariableSignKind {
            if Self::is_negative_infinite(lb) && Self::is_positive_infinite(ub) {
                VariableSignKind::Free
            } else if Self::is_zero(lb) && Self::is_positive_infinite(ub) {
                VariableSignKind::PositiveNormalized
            } else if Self::is_negative_infinite(lb) && Self::is_zero(ub) {
                VariableSignKind::NegativeNormalized
            } else {
                VariableSignKind::NonNormalized
            }
        };

        let relation_for_variable =
            |kind: VariableSignKind, objective: ObjectiveCategory| -> DualConstraintRelation {
                match kind {
                    VariableSignKind::NonNormalized | VariableSignKind::Free => {
                        DualConstraintRelation::Equal
                    }
                    VariableSignKind::NegativeNormalized => match objective {
                        ObjectiveCategory::Maximum => DualConstraintRelation::LessEqual,
                        ObjectiveCategory::Minimum => DualConstraintRelation::GreaterEqual,
                    },
                    VariableSignKind::PositiveNormalized => match objective {
                        ObjectiveCategory::Maximum => DualConstraintRelation::GreaterEqual,
                        ObjectiveCategory::Minimum => DualConstraintRelation::LessEqual,
                    },
                }
            };

        let m = self.basic.num_constraints();
        let n = self.basic.num_variables();

        let mut dual_basic = BasicLinearTriadModel::new(&format!("{}_dual", self.basic.name));
        let mut dual_c = Vec::new();
        let mut next_group_id = self
            .basic
            .variables
            .iter()
            .map(|token| token.id().group_id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);

        let add_dual_variable = |dual_basic: &mut BasicLinearTriadModel,
                                 dual_c: &mut Vec<f64>,
                                 next_group_id: &mut usize,
                                 name: String,
                                 lb: f64,
                                 ub: f64,
                                 objective_coeff: f64| {
            let variable =
                ContinuousVariableItem::create(VariableId::standalone(*next_group_id), &name);
            *next_group_id = next_group_id.saturating_add(1);
            let solver_index = dual_basic.num_variables();
            dual_basic.add_variable_with_bounds(
                Token::from_generic(variable, solver_index),
                lb,
                ub,
                VariableType::Continuous,
            );
            dual_c.push(objective_coeff);
            solver_index
        };

        for i in 0..m {
            let (lb, ub) = match self.objective_category {
                ObjectiveCategory::Maximum => (0.0, f64::INFINITY),
                ObjectiveCategory::Minimum => (f64::NEG_INFINITY, 0.0),
            };
            let objective_coeff = self.basic.b.get(i).copied().unwrap_or(0.0);
            add_dual_variable(
                &mut dual_basic,
                &mut dual_c,
                &mut next_group_id,
                format!("{}_dual_y{}", self.basic.name, i),
                lb,
                ub,
                objective_coeff,
            );
        }

        let mut stationarity_rows: Vec<SparseVector<f64>> =
            (0..n).map(|_| SparseVector::new()).collect();
        for (row_index, row) in self.basic.A.rows.iter().enumerate() {
            for (col_index, coefficient) in &row.entries {
                if *col_index < n {
                    stationarity_rows[*col_index].add(row_index, *coefficient);
                }
            }
        }

        for col_index in 0..n {
            let lower = self
                .basic
                .lb
                .get(col_index)
                .copied()
                .unwrap_or(f64::NEG_INFINITY);
            let upper = self
                .basic
                .ub
                .get(col_index)
                .copied()
                .unwrap_or(f64::INFINITY);
            let sign_kind = classify_variable_sign(lower, upper);

            if sign_kind == VariableSignKind::NonNormalized {
                match self.objective_category {
                    ObjectiveCategory::Maximum => {
                        if lower.is_finite() {
                            let lb_dual_index = add_dual_variable(
                                &mut dual_basic,
                                &mut dual_c,
                                &mut next_group_id,
                                format!("{}_lb_dual", self.basic.variables[col_index].name()),
                                f64::NEG_INFINITY,
                                0.0,
                                lower,
                            );
                            stationarity_rows[col_index].add(lb_dual_index, 1.0);
                        }
                        if upper.is_finite() {
                            let ub_dual_index = add_dual_variable(
                                &mut dual_basic,
                                &mut dual_c,
                                &mut next_group_id,
                                format!("{}_ub_dual", self.basic.variables[col_index].name()),
                                0.0,
                                f64::INFINITY,
                                upper,
                            );
                            stationarity_rows[col_index].add(ub_dual_index, 1.0);
                        }
                    }
                    ObjectiveCategory::Minimum => {
                        if lower.is_finite() {
                            let lb_dual_index = add_dual_variable(
                                &mut dual_basic,
                                &mut dual_c,
                                &mut next_group_id,
                                format!("{}_lb_dual", self.basic.variables[col_index].name()),
                                0.0,
                                f64::INFINITY,
                                lower,
                            );
                            stationarity_rows[col_index].add(lb_dual_index, 1.0);
                        }
                        if upper.is_finite() {
                            let ub_dual_index = add_dual_variable(
                                &mut dual_basic,
                                &mut dual_c,
                                &mut next_group_id,
                                format!("{}_ub_dual", self.basic.variables[col_index].name()),
                                f64::NEG_INFINITY,
                                0.0,
                                upper,
                            );
                            stationarity_rows[col_index].add(ub_dual_index, 1.0);
                        }
                    }
                }
            }
        }

        for (col_index, row) in stationarity_rows.into_iter().enumerate() {
            let lower = self
                .basic
                .lb
                .get(col_index)
                .copied()
                .unwrap_or(f64::NEG_INFINITY);
            let upper = self
                .basic
                .ub
                .get(col_index)
                .copied()
                .unwrap_or(f64::INFINITY);
            let sign_kind = classify_variable_sign(lower, upper);
            let primal_c = self.c.get(col_index).copied().unwrap_or(0.0);

            match relation_for_variable(sign_kind, self.objective_category) {
                DualConstraintRelation::LessEqual => {
                    dual_basic.add_constraint(row, primal_c);
                }
                DualConstraintRelation::GreaterEqual => {
                    dual_basic.add_constraint(Self::negate_row(&row), -primal_c);
                }
                DualConstraintRelation::Equal => {
                    dual_basic.add_constraint(row.clone(), primal_c);
                    dual_basic.add_constraint(Self::negate_row(&row), -primal_c);
                }
            }
        }

        let mut dual = LinearTriadModel::from_basic(dual_basic);
        dual.c = dual_c;
        dual.objective_category = match self.objective_category {
            ObjectiveCategory::Minimum => ObjectiveCategory::Maximum,
            ObjectiveCategory::Maximum => ObjectiveCategory::Minimum,
        };
        dual
    }

    /// 生成 Farkas 对偶模型 / Generate Farkas Dual Model
    ///
    /// 用于原问题不可行时构造可行性证据模型，便于 Benders 等算法生成可行性割。
    /// Used to construct infeasibility certificate model for decomposition algorithms such as Benders.
    ///
    /// 当前实现基于 `Ax <= b` 约束格式，变量边界通过额外对偶变量注入。
    /// Current implementation assumes `Ax <= b` constraints and injects variable-bound dual variables.
    pub fn to_farkas_dual(&self) -> Self {
        if self.has_integer_variables() {
            return self.linear_relaxed().to_farkas_dual();
        }

        #[derive(Clone, Copy, PartialEq, Eq)]
        enum BoundKind {
            Free,
            PositiveNormalized,
            NegativeNormalized,
            PositiveFree,
            NegativeFree,
            Bounded,
        }

        let classify_bound_kind = |lb: f64, ub: f64| -> BoundKind {
            if Self::is_negative_infinite(lb) && Self::is_positive_infinite(ub) {
                BoundKind::Free
            } else if Self::is_zero(lb) && Self::is_positive_infinite(ub) {
                BoundKind::PositiveNormalized
            } else if Self::is_negative_infinite(lb) && Self::is_zero(ub) {
                BoundKind::NegativeNormalized
            } else if lb.is_finite() && Self::is_positive_infinite(ub) {
                BoundKind::PositiveFree
            } else if Self::is_negative_infinite(lb) && ub.is_finite() {
                BoundKind::NegativeFree
            } else {
                BoundKind::Bounded
            }
        };

        let m = self.basic.num_constraints();
        let n = self.basic.num_variables();

        let mut dual_basic =
            BasicLinearTriadModel::new(&format!("{}_farkas_dual", self.basic.name));
        let mut dual_c = Vec::new();
        let mut next_group_id = self
            .basic
            .variables
            .iter()
            .map(|token| token.id().group_id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);

        let add_variable = |dual_basic: &mut BasicLinearTriadModel,
                            dual_c: &mut Vec<f64>,
                            next_group_id: &mut usize,
                            name: String,
                            lb: f64,
                            ub: f64,
                            objective_coeff: f64| {
            let variable =
                ContinuousVariableItem::create(VariableId::standalone(*next_group_id), &name);
            *next_group_id = next_group_id.saturating_add(1);
            let solver_index = dual_basic.num_variables();
            dual_basic.add_variable_with_bounds(
                Token::from_generic(variable, solver_index),
                lb,
                ub,
                VariableType::Continuous,
            );
            dual_c.push(objective_coeff);
            solver_index
        };

        // 对每条 primal 约束构造 Farkas 乘子：Ax <= b => y >= 0
        // Build Farkas multipliers for each primal constraint: Ax <= b => y >= 0.
        for i in 0..m {
            add_variable(
                &mut dual_basic,
                &mut dual_c,
                &mut next_group_id,
                format!("{}_cons{}_farkas", self.basic.name, i),
                0.0,
                f64::INFINITY,
                1.0,
            );
        }

        let mut stationarity_rows: Vec<SparseVector<f64>> =
            (0..n).map(|_| SparseVector::new()).collect();
        for (row_index, row) in self.basic.A.rows.iter().enumerate() {
            for (col_index, coefficient) in &row.entries {
                if *col_index < n {
                    stationarity_rows[*col_index].add(row_index, *coefficient);
                }
            }
        }

        let mut normalization_row = SparseVector::new();
        for (constraint_index, rhs) in self.basic.b.iter().copied().enumerate() {
            if constraint_index < m && rhs != 0.0 {
                normalization_row.add(constraint_index, rhs);
            }
        }

        for var_index in 0..n {
            let lower = self
                .basic
                .lb
                .get(var_index)
                .copied()
                .unwrap_or(f64::NEG_INFINITY);
            let upper = self
                .basic
                .ub
                .get(var_index)
                .copied()
                .unwrap_or(f64::INFINITY);
            let kind = classify_bound_kind(lower, upper);
            let name = self.basic.variables[var_index].name().to_string();

            match kind {
                BoundKind::Free | BoundKind::PositiveNormalized | BoundKind::NegativeNormalized => {
                }
                BoundKind::PositiveFree => {
                    let lb_dual_index = add_variable(
                        &mut dual_basic,
                        &mut dual_c,
                        &mut next_group_id,
                        format!("{}_lb_farkas_dual", name),
                        f64::NEG_INFINITY,
                        0.0,
                        0.0,
                    );
                    stationarity_rows[var_index].add(lb_dual_index, 1.0);
                    normalization_row.add(lb_dual_index, lower);
                }
                BoundKind::NegativeFree => {
                    let ub_dual_index = add_variable(
                        &mut dual_basic,
                        &mut dual_c,
                        &mut next_group_id,
                        format!("{}_ub_farkas_dual", name),
                        0.0,
                        f64::INFINITY,
                        0.0,
                    );
                    stationarity_rows[var_index].add(ub_dual_index, 1.0);
                    normalization_row.add(ub_dual_index, upper);
                }
                BoundKind::Bounded => {
                    if lower.is_finite() {
                        let lb_dual_index = add_variable(
                            &mut dual_basic,
                            &mut dual_c,
                            &mut next_group_id,
                            format!("{}_lb_farkas_dual", name),
                            f64::NEG_INFINITY,
                            0.0,
                            0.0,
                        );
                        stationarity_rows[var_index].add(lb_dual_index, 1.0);
                        normalization_row.add(lb_dual_index, lower);
                    }
                    if upper.is_finite() {
                        let ub_dual_index = add_variable(
                            &mut dual_basic,
                            &mut dual_c,
                            &mut next_group_id,
                            format!("{}_ub_farkas_dual", name),
                            0.0,
                            f64::INFINITY,
                            0.0,
                        );
                        stationarity_rows[var_index].add(ub_dual_index, 1.0);
                        normalization_row.add(ub_dual_index, upper);
                    }
                }
            }
        }

        // Stationarity equalities: A^T y + λ_lb + λ_ub = 0
        // 使用双不等式表达等式。 Represent equalities with paired inequalities.
        for row in stationarity_rows {
            dual_basic.add_constraint(row.clone(), 0.0);
            dual_basic.add_constraint(Self::negate_row(&row), 0.0);
        }

        // Normalization equality: b^T y + lb^T λ_lb + ub^T λ_ub = -1
        // 使用双不等式表达等式。 Represent equality with paired inequalities.
        dual_basic.add_constraint(normalization_row.clone(), -1.0);
        dual_basic.add_constraint(Self::negate_row(&normalization_row), 1.0);

        let mut dual = LinearTriadModel::from_basic(dual_basic);
        dual.c = dual_c;
        dual.objective_category = ObjectiveCategory::Minimum;
        dual
    }
}

impl std::ops::Deref for LinearTriadModel {
    type Target = BasicLinearTriadModel;
    fn deref(&self) -> &Self::Target {
        &self.basic
    }
}

impl std::ops::DerefMut for LinearTriadModel {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.basic
    }
}

impl Default for LinearTriadModel {
    fn default() -> Self {
        Self::new("default")
    }
}

/// f64 精度的线性三角模型 / Linear triad model with f64 precision
pub type LinearTriadModelF64 = LinearTriadModel;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::variable::UContinuousVariableItem;

    fn sparse_row(entries: &[(usize, f64)]) -> SparseVector<f64> {
        let mut row = SparseVector::new();
        for (index, value) in entries {
            row.add(*index, *value);
        }
        row
    }

    fn dense_rows(model: &LinearTriadModel) -> Vec<Vec<f64>> {
        let cols = model.num_variables();
        model
            .A
            .rows
            .iter()
            .map(|row| {
                let mut dense = vec![0.0; cols];
                for (index, value) in &row.entries {
                    dense[*index] += *value;
                }
                dense
            })
            .collect()
    }

    #[test]
    fn to_dual_converts_max_primal_to_min_dual() {
        let mut basic = BasicLinearTriadModel::new("primal");
        let x1 = UContinuousVariableItem::auto("x1");
        let x2 = UContinuousVariableItem::auto("x2");
        basic.add_variable(Token::from_generic(x1, 0));
        basic.add_variable(Token::from_generic(x2, 1));

        basic.add_constraint(sparse_row(&[(0, 1.0), (1, 1.0)]), 4.0);
        basic.add_constraint(sparse_row(&[(0, 1.0)]), 2.0);
        basic.add_constraint(sparse_row(&[(1, 1.0)]), 3.0);

        let mut primal = LinearTriadModel::from_basic(basic);
        primal.set_objective(vec![3.0, 2.0], ObjectiveCategory::Maximum);

        let dual = primal.to_dual();
        assert_eq!(dual.objective_category, ObjectiveCategory::Minimum);
        assert_eq!(dual.num_variables(), 3);
        assert_eq!(dual.num_constraints(), 2);
        assert_eq!(dual.c, vec![4.0, 2.0, 3.0]);
        assert_eq!(dual.b, vec![-3.0, -2.0]);
        assert_eq!(
            dense_rows(&dual),
            vec![vec![-1.0, -1.0, 0.0], vec![-1.0, 0.0, -1.0]]
        );
    }

    #[test]
    fn to_dual_converts_min_primal_to_max_dual() {
        let mut basic = BasicLinearTriadModel::new("primal_min");
        let x1 = UContinuousVariableItem::auto("x1");
        let x2 = UContinuousVariableItem::auto("x2");
        basic.add_variable(Token::from_generic(x1, 0));
        basic.add_variable(Token::from_generic(x2, 1));

        basic.add_constraint(sparse_row(&[(0, 2.0), (1, 1.0)]), 5.0);
        basic.add_constraint(sparse_row(&[(0, 1.0), (1, 3.0)]), 6.0);

        let mut primal = LinearTriadModel::from_basic(basic);
        primal.set_objective(vec![4.0, 7.0], ObjectiveCategory::Minimum);

        let dual = primal.to_dual();
        assert_eq!(dual.objective_category, ObjectiveCategory::Maximum);
        assert_eq!(dual.num_variables(), 2);
        assert_eq!(dual.num_constraints(), 2);
        assert_eq!(dual.c, vec![5.0, 6.0]);
        assert_eq!(dual.b, vec![4.0, 7.0]);
        assert_eq!(dense_rows(&dual), vec![vec![2.0, 1.0], vec![1.0, 3.0]]);
        assert!(
            dual.lb
                .iter()
                .all(|value| value.is_infinite() && value.is_sign_negative())
        );
        assert!(dual.ub.iter().all(|value| *value == 0.0));
    }

    #[test]
    fn to_dual_introduces_bound_dual_variables_for_bounded_primal_variable() {
        let mut basic = BasicLinearTriadModel::new("bounded_primal");
        let x = UContinuousVariableItem::auto("x");
        basic.add_variable_with_bounds(
            Token::from_generic(x, 0),
            1.0,
            3.0,
            VariableType::Continuous,
        );

        let mut primal = LinearTriadModel::from_basic(basic);
        primal.set_objective(vec![2.0], ObjectiveCategory::Maximum);

        let dual = primal.to_dual();
        assert_eq!(dual.objective_category, ObjectiveCategory::Minimum);
        assert_eq!(dual.num_variables(), 2);
        assert_eq!(dual.num_constraints(), 2);
        assert_eq!(dual.c, vec![1.0, 3.0]);
        assert_eq!(dual.b, vec![2.0, -2.0]);
        assert_eq!(dense_rows(&dual), vec![vec![1.0, 1.0], vec![-1.0, -1.0]]);
        assert!(dual.lb[0].is_infinite() && dual.lb[0].is_sign_negative());
        assert_eq!(dual.ub[0], 0.0);
        assert_eq!(dual.lb[1], 0.0);
        assert!(dual.ub[1].is_infinite() && dual.ub[1].is_sign_positive());
    }

    #[test]
    fn to_farkas_dual_adds_normalization_and_stationarity_for_bound_conflict() {
        let mut basic = BasicLinearTriadModel::new("farkas_bound_conflict");
        let x = UContinuousVariableItem::auto("x");
        basic.add_variable_with_bounds(
            Token::from_generic(x, 0),
            1.0,
            0.0,
            VariableType::Continuous,
        );

        let primal = LinearTriadModel::from_basic(basic);
        let farkas = primal.to_farkas_dual();
        assert_eq!(farkas.objective_category, ObjectiveCategory::Minimum);
        assert_eq!(farkas.num_variables(), 2);
        assert_eq!(farkas.num_constraints(), 4);
        assert_eq!(farkas.c, vec![0.0, 0.0]);
        assert_eq!(farkas.b, vec![0.0, 0.0, -1.0, 1.0]);
        assert_eq!(
            dense_rows(&farkas),
            vec![
                vec![1.0, 1.0],
                vec![-1.0, -1.0],
                vec![1.0, 0.0],
                vec![-1.0, 0.0]
            ]
        );
        assert!(farkas.lb[0].is_infinite() && farkas.lb[0].is_sign_negative());
        assert_eq!(farkas.ub[0], 0.0);
        assert_eq!(farkas.lb[1], 0.0);
        assert!(farkas.ub[1].is_infinite() && farkas.ub[1].is_sign_positive());
    }

    #[test]
    fn linear_relaxed_converts_integer_types_to_continuous_family() {
        let mut basic = BasicLinearTriadModel::new("relax_types");
        basic.add_variable_with_bounds(
            Token::from_generic(crate::variable::BinaryVariableItem::auto("b"), 0),
            0.0,
            1.0,
            VariableType::Binary,
        );
        basic.add_variable_with_bounds(
            Token::from_generic(crate::variable::IntegerVariableItem::auto("i"), 1),
            -5.0,
            8.0,
            VariableType::Integer,
        );
        let model = LinearTriadModel::from_basic(basic);
        let relaxed = model.linear_relaxed();
        assert_eq!(relaxed.var_types[0], VariableType::Percentage);
        assert_eq!(relaxed.var_types[1], VariableType::Continuous);
    }

    #[test]
    fn feasibility_model_removes_objective_coefficients() {
        let mut basic = BasicLinearTriadModel::new("feasibility");
        let x = UContinuousVariableItem::auto("x");
        basic.add_variable(Token::from_generic(x, 0));
        let mut model = LinearTriadModel::from_basic(basic);
        model.set_objective(vec![3.0], ObjectiveCategory::Maximum);

        let feasibility = model.feasibility();
        assert_eq!(feasibility.objective_category, ObjectiveCategory::Minimum);
        assert_eq!(feasibility.c, vec![0.0]);
    }

    #[test]
    fn elastic_model_adds_slack_variables_and_penalty_objective() {
        let mut basic = BasicLinearTriadModel::new("elastic");
        let x = UContinuousVariableItem::auto("x");
        basic.add_variable(Token::from_generic(x, 0));
        basic.add_constraint(sparse_row(&[(0, 1.0)]), 4.0);
        basic.add_constraint(sparse_row(&[(0, -1.0)]), -1.0);
        let model = LinearTriadModel::from_basic(basic);

        let elastic = model.elastic(100.0);
        assert_eq!(elastic.num_variables(), 3);
        assert_eq!(elastic.num_constraints(), 2);
        assert_eq!(elastic.c, vec![0.0, 100.0, 100.0]);
        assert_eq!(elastic.objective_category, ObjectiveCategory::Minimum);
    }

    #[test]
    fn to_dual_relaxes_integer_model_before_conversion() {
        let mut basic = BasicLinearTriadModel::new("int_to_dual");
        basic.add_variable_with_bounds(
            Token::from_generic(crate::variable::IntegerVariableItem::auto("x"), 0),
            0.0,
            3.0,
            VariableType::Integer,
        );
        basic.add_constraint(sparse_row(&[(0, 1.0)]), 2.0);

        let mut model = LinearTriadModel::from_basic(basic);
        model.set_objective(vec![1.0], ObjectiveCategory::Maximum);
        let dual = model.to_dual();
        assert_eq!(dual.objective_category, ObjectiveCategory::Minimum);
        assert_eq!(dual.num_constraints(), 2);
    }

    #[test]
    fn to_farkas_dual_relaxes_integer_model_before_conversion() {
        let mut basic = BasicLinearTriadModel::new("int_to_farkas");
        basic.add_variable_with_bounds(
            Token::from_generic(crate::variable::IntegerVariableItem::auto("x"), 0),
            0.0,
            1.0,
            VariableType::Integer,
        );
        basic.add_constraint(sparse_row(&[(0, 1.0)]), 0.0);
        basic.add_constraint(sparse_row(&[(0, -1.0)]), -1.0);

        let mut model = LinearTriadModel::from_basic(basic);
        model.set_objective(vec![1.0], ObjectiveCategory::Maximum);

        let farkas_from_integer = model.to_farkas_dual();
        let farkas_from_relaxed = model.linear_relaxed().to_farkas_dual();

        assert_eq!(
            farkas_from_integer.objective_category,
            ObjectiveCategory::Minimum
        );
        assert_eq!(
            farkas_from_integer.num_variables(),
            farkas_from_relaxed.num_variables()
        );
        assert_eq!(
            farkas_from_integer.num_constraints(),
            farkas_from_relaxed.num_constraints()
        );
        assert_eq!(farkas_from_integer.c, farkas_from_relaxed.c);
        assert_eq!(farkas_from_integer.b, farkas_from_relaxed.b);
        assert_eq!(
            dense_rows(&farkas_from_integer),
            dense_rows(&farkas_from_relaxed)
        );
        assert_eq!(farkas_from_integer.lb, farkas_from_relaxed.lb);
        assert_eq!(farkas_from_integer.ub, farkas_from_relaxed.ub);
    }

    #[test]
    fn tidy_dual_solution_filters_zero_entries() {
        let model = LinearTriadModel::new("tidy");
        let tidy = model.tidy_dual_solution(&[0.0, 1e-3, 0.0, -2.0]);
        assert_eq!(tidy, vec![(1, 1e-3), (3, -2.0)]);
    }
}

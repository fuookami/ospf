//! 弹性模型构建器。
//! Elastic model builders.

use super::{LinearTriadModel, QuadraticTetradModel};
use crate::model::ObjectiveCategory;
use crate::token::Token;
use crate::variable::{ContinuousVariableItem, VariableId, VariableType};

/// 线性弹性模型构建器。
/// Linear elastic-model builder.
#[derive(Debug, Clone, Copy)]
pub struct LinearElasticBuilder<'a> {
    model: &'a LinearTriadModel,
    penalty: f64,
}

impl<'a> LinearElasticBuilder<'a> {
    /// 创建线性弹性模型构建器。
    /// Create a linear elastic-model builder.
    pub fn new(model: &'a LinearTriadModel) -> Self {
        Self {
            model,
            penalty: 1_000.0,
        }
    }

    /// 设置每个松弛变量的惩罚系数。
    /// Set the penalty coefficient for each slack variable.
    pub fn penalty(mut self, penalty: f64) -> Self {
        self.penalty = penalty;
        self
    }

    /// 构建弹性模型。
    /// Build the elastic model.
    pub fn finish(self) -> LinearTriadModel {
        self.model.elastic(self.penalty)
    }

    /// 构建弹性模型。
    /// Build the elastic model.
    pub fn build(self) -> LinearTriadModel {
        self.finish()
    }
}

/// 二次弹性模型构建器。
/// Quadratic elastic-model builder.
#[derive(Debug, Clone, Copy)]
pub struct QuadraticElasticBuilder<'a> {
    model: &'a QuadraticTetradModel,
    penalty: f64,
    linear_constraints: bool,
}

impl<'a> QuadraticElasticBuilder<'a> {
    /// 创建二次弹性模型构建器。
    /// Create a quadratic elastic-model builder.
    pub fn new(model: &'a QuadraticTetradModel) -> Self {
        Self {
            model,
            penalty: 1_000.0,
            linear_constraints: true,
        }
    }

    /// 设置每个线性约束松弛变量的惩罚系数。
    /// Set the penalty coefficient for each linear-constraint slack variable.
    pub fn penalty(mut self, penalty: f64) -> Self {
        self.penalty = penalty;
        self
    }

    /// 是否弹性化线性约束；二次约束当前保持不变。
    /// Whether to elasticize linear constraints; quadratic constraints are currently preserved.
    pub fn linear_constraints(mut self, enabled: bool) -> Self {
        self.linear_constraints = enabled;
        self
    }

    /// 构建弹性模型。
    /// Build the elastic model.
    pub fn finish(self) -> QuadraticTetradModel {
        let mut model = self.model.clone();
        model.c.resize(model.num_variables(), 0.0);

        if !self.linear_constraints {
            return model;
        }

        let original_constraints = model.basic.linear.num_constraints();
        let mut next_group_id = model
            .basic
            .linear
            .variables
            .iter()
            .map(|token| token.id().group_id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);

        for constraint_index in 0..original_constraints {
            let slack = ContinuousVariableItem::create(
                VariableId::standalone(next_group_id),
                &format!("{}_elastic_s{}", model.basic.linear.name, constraint_index),
            );
            next_group_id = next_group_id.saturating_add(1);
            let solver_index = model.basic.linear.num_variables();

            let slack_index = model.basic.linear.add_variable_with_bounds(
                Token::from_generic(slack, solver_index),
                0.0,
                f64::INFINITY,
                VariableType::Continuous,
            );
            model.c.push(self.penalty);

            if let Some(row) = model.basic.linear.A.rows.get_mut(constraint_index) {
                row.add(slack_index, -1.0);
            }
        }

        model.objective_category = ObjectiveCategory::Minimum;
        model
    }

    /// 构建弹性模型。
    /// Build the elastic model.
    pub fn build(self) -> QuadraticTetradModel {
        self.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::intermediate::{
        BasicLinearTriadModel, BasicQuadraticTetradModel, SparseMatrix, SparseVector,
    };
    use crate::token::Token;
    use crate::variable::{ContinuousVariableItem, VariableType};

    #[test]
    fn linear_elastic_builder_uses_existing_elastic_model() {
        let mut basic = BasicLinearTriadModel::new("linear_elastic_builder");
        basic.add_variable_with_bounds(
            Token::from_generic(ContinuousVariableItem::auto("x"), 0),
            0.0,
            10.0,
            VariableType::Continuous,
        );
        basic.add_constraint(SparseVector::new(), 1.0);
        let model = LinearTriadModel::from_basic(basic);

        let elastic = model.elastic_builder().penalty(7.0).finish();
        assert_eq!(elastic.num_variables(), 2);
        assert_eq!(elastic.num_constraints(), 1);
        assert_eq!(elastic.c, vec![0.0, 7.0]);
        assert_eq!(elastic.objective_category, ObjectiveCategory::Minimum);
    }

    #[test]
    fn quadratic_elastic_builder_adds_slacks_to_linear_rows_only() {
        let mut basic = BasicQuadraticTetradModel::new("quadratic_elastic_builder");
        basic.linear.add_variable_with_bounds(
            Token::from_generic(ContinuousVariableItem::auto("x"), 0),
            0.0,
            10.0,
            VariableType::Continuous,
        );
        basic.linear.add_constraint(SparseVector::new(), 1.0);
        let mut model = QuadraticTetradModel::from_basic(basic);
        model.set_objective(vec![3.0], SparseMatrix::new(), ObjectiveCategory::Maximum);

        let elastic = model.elastic_builder().penalty(9.0).finish();
        assert_eq!(elastic.num_variables(), 2);
        assert_eq!(elastic.num_constraints(), 1);
        assert_eq!(elastic.c, vec![3.0, 9.0]);
        assert_eq!(elastic.objective_category, ObjectiveCategory::Minimum);
        assert_eq!(elastic.num_quadratic_constraints(), 0);
    }
}

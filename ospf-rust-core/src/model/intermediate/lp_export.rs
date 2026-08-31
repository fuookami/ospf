//! Intermediate-model LP export helpers.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::model::ConstraintRelation;
use crate::model::ObjectiveCategory;
use crate::variable::VariableType;

use super::{
    BasicLinearTriadModel, BasicQuadraticTetradModel, LinearTriadModel, QuadraticTetradModel,
    SparseMatrix, SparseVector,
};

/// Unified LP export interface for intermediate models.
pub trait LPExportableModel {
    /// Serialize model into LP-format text.
    fn to_lp_string(&self) -> String;

    /// Write LP-format text to file.
    fn write_lp<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        fs::write(path, self.to_lp_string())
    }
}

fn relation_to_str(relation: ConstraintRelation) -> &'static str {
    match relation {
        ConstraintRelation::LessEqual => "<=",
        ConstraintRelation::Equal => "=",
        ConstraintRelation::GreaterEqual => ">=",
    }
}

fn variable_name(index: usize) -> String {
    format!("x{index}")
}

fn format_number(value: f64) -> String {
    if value.is_infinite() {
        if value.is_sign_positive() {
            "+inf".to_string()
        } else {
            "-inf".to_string()
        }
    } else if value.abs() <= f64::EPSILON {
        "0".to_string()
    } else {
        let mut text = format!("{value:.12}");
        while text.contains('.') && text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
        if text.is_empty() {
            "0".to_string()
        } else {
            text
        }
    }
}

fn collect_sparse_row_terms(row: &SparseVector<f64>) -> Vec<(usize, f64)> {
    let mut combined: BTreeMap<usize, f64> = BTreeMap::new();
    for (index, coefficient) in &row.entries {
        let entry = combined.entry(*index).or_insert(0.0);
        *entry += *coefficient;
    }
    combined
        .into_iter()
        .filter(|(_, value)| value.abs() > f64::EPSILON)
        .collect()
}

fn format_linear_expression(terms: &[(usize, f64)], var_names: &[String], constant: f64) -> String {
    let mut output = String::new();
    let mut has_any = false;

    for (index, coefficient) in terms {
        if coefficient.abs() <= f64::EPSILON {
            continue;
        }
        let abs = coefficient.abs();
        if has_any {
            output.push_str(if *coefficient >= 0.0 { " + " } else { " - " });
        } else if *coefficient < 0.0 {
            output.push('-');
        }
        output.push_str(&format!("{} {}", format_number(abs), var_names[*index]));
        has_any = true;
    }

    if constant.abs() > f64::EPSILON {
        if has_any {
            output.push_str(if constant >= 0.0 { " + " } else { " - " });
            output.push_str(&format_number(constant.abs()));
        } else {
            output.push_str(&format_number(constant));
            has_any = true;
        }
    }

    if has_any { output } else { "0".to_string() }
}

fn collect_quadratic_objective_terms(
    matrix: &SparseMatrix<f64>,
) -> Vec<(usize, Option<usize>, f64)> {
    let mut terms = Vec::new();
    for (row_index, row) in matrix.rows.iter().enumerate() {
        for (col_index, coefficient) in &row.entries {
            if coefficient.abs() <= f64::EPSILON {
                continue;
            }
            terms.push((row_index, Some(*col_index), *coefficient));
        }
    }
    terms
}

fn format_quadratic_terms(terms: &[(usize, Option<usize>, f64)], var_names: &[String]) -> String {
    let mut output = String::new();
    let mut has_any = false;

    for (var_index1, var_index2, coefficient) in terms {
        if coefficient.abs() <= f64::EPSILON {
            continue;
        }
        let abs = coefficient.abs();
        if has_any {
            output.push_str(if *coefficient >= 0.0 { " + " } else { " - " });
        } else if *coefficient < 0.0 {
            output.push('-');
        }

        if let Some(var_index2) = var_index2 {
            if *var_index2 == *var_index1 {
                output.push_str(&format!(
                    "{} {} ^ 2",
                    format_number(abs),
                    var_names[*var_index1]
                ));
            } else {
                output.push_str(&format!(
                    "{} {} * {}",
                    format_number(abs),
                    var_names[*var_index1],
                    var_names[*var_index2]
                ));
            }
        } else {
            output.push_str(&format!(
                "{} {}",
                format_number(abs),
                var_names[*var_index1]
            ));
        }
        has_any = true;
    }

    if has_any { output } else { "0".to_string() }
}

fn format_mixed_expression(
    linear_terms: &[(usize, f64)],
    quadratic_terms: &[(usize, Option<usize>, f64)],
    var_names: &[String],
    constant: f64,
) -> String {
    let linear = format_linear_expression(linear_terms, var_names, constant);
    let quadratic = format_quadratic_terms(quadratic_terms, var_names);
    if quadratic == "0" {
        return linear;
    }
    if linear == "0" {
        return format!("[ {quadratic} ]");
    }
    format!("{linear} + [ {quadratic} ]")
}

fn append_bounds_and_domains(
    output: &mut String,
    var_names: &[String],
    lb: &[f64],
    ub: &[f64],
    var_types: &[VariableType],
) {
    output.push_str("Bounds\n");
    for index in 0..var_names.len() {
        let lower = lb[index];
        let upper = ub[index];
        let name = &var_names[index];
        if lower.is_infinite() && upper.is_infinite() {
            output.push_str(&format!("  {name} free\n"));
        } else if lower.is_infinite() {
            output.push_str(&format!("  {name} <= {}\n", format_number(upper)));
        } else if upper.is_infinite() {
            output.push_str(&format!("  {} <= {name}\n", format_number(lower)));
        } else {
            output.push_str(&format!(
                "  {} <= {name} <= {}\n",
                format_number(lower),
                format_number(upper)
            ));
        }
    }

    let binaries: Vec<&str> = var_types
        .iter()
        .enumerate()
        .filter_map(|(index, variable_type)| {
            if *variable_type == VariableType::Binary {
                Some(var_names[index].as_str())
            } else {
                None
            }
        })
        .collect();
    if !binaries.is_empty() {
        output.push_str("Binaries\n  ");
        output.push_str(&binaries.join(" "));
        output.push('\n');
    }

    let generals: Vec<&str> = var_types
        .iter()
        .enumerate()
        .filter_map(|(index, variable_type)| match variable_type {
            VariableType::Binary => None,
            VariableType::Ternary
            | VariableType::BalancedTernary
            | VariableType::Integer
            | VariableType::UInteger => Some(var_names[index].as_str()),
            _ => None,
        })
        .collect();
    if !generals.is_empty() {
        output.push_str("Generals\n  ");
        output.push_str(&generals.join(" "));
        output.push('\n');
    }
}

impl LPExportableModel for LinearTriadModel {
    fn to_lp_string(&self) -> String {
        let var_names: Vec<String> = (0..self.num_variables()).map(variable_name).collect();
        let mut output = String::new();

        output.push_str(match self.objective_category {
            ObjectiveCategory::Minimum => "Minimize\n",
            ObjectiveCategory::Maximum => "Maximize\n",
        });
        let objective_terms: Vec<(usize, f64)> = self
            .c
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, coefficient)| coefficient.abs() > f64::EPSILON)
            .collect();
        output.push_str(&format!(
            "  obj: {}\n",
            format_linear_expression(&objective_terms, &var_names, 0.0)
        ));

        output.push_str("Subject To\n");
        for row_index in 0..self.A.rows.len() {
            let row_terms = collect_sparse_row_terms(&self.A.rows[row_index]);
            let name = self
                .constraint_names
                .get(row_index)
                .cloned()
                .unwrap_or_else(|| format!("c{row_index}"));
            let rhs = self.b.get(row_index).copied().unwrap_or(0.0);
            output.push_str(&format!(
                "  {name}: {} <= {}\n",
                format_linear_expression(&row_terms, &var_names, 0.0),
                format_number(rhs)
            ));
        }

        append_bounds_and_domains(&mut output, &var_names, &self.lb, &self.ub, &self.var_types);
        output.push_str("End\n");
        output
    }
}

impl LPExportableModel for BasicLinearTriadModel {
    fn to_lp_string(&self) -> String {
        LinearTriadModel::from_basic(self.clone()).to_lp_string()
    }
}

impl LPExportableModel for QuadraticTetradModel {
    fn to_lp_string(&self) -> String {
        let var_names: Vec<String> = (0..self.num_variables()).map(variable_name).collect();
        let mut output = String::new();

        output.push_str(match self.objective_category {
            ObjectiveCategory::Minimum => "Minimize\n",
            ObjectiveCategory::Maximum => "Maximize\n",
        });
        let objective_linear_terms: Vec<(usize, f64)> = self
            .c
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, coefficient)| coefficient.abs() > f64::EPSILON)
            .collect();
        let objective_quadratic_terms = collect_quadratic_objective_terms(&self.Q);
        output.push_str(&format!(
            "  obj: {}\n",
            format_mixed_expression(
                &objective_linear_terms,
                &objective_quadratic_terms,
                &var_names,
                0.0
            )
        ));

        output.push_str("Subject To\n");
        for row_index in 0..self.basic.linear.A.rows.len() {
            let row_terms = collect_sparse_row_terms(&self.basic.linear.A.rows[row_index]);
            let name = self
                .basic
                .linear
                .constraint_names
                .get(row_index)
                .cloned()
                .unwrap_or_else(|| format!("c{row_index}"));
            let rhs = self.basic.linear.b.get(row_index).copied().unwrap_or(0.0);
            output.push_str(&format!(
                "  {name}: {} <= {}\n",
                format_linear_expression(&row_terms, &var_names, 0.0),
                format_number(rhs)
            ));
        }

        for (quadratic_index, inequality) in self.quadratic_constraints.iter().enumerate() {
            let mut linear_terms = Vec::new();
            let mut quadratic_terms = Vec::new();
            for monomial in inequality.polynomial.monomials() {
                if let Some(var_index2) = monomial.var_index2() {
                    quadratic_terms.push((
                        monomial.var_index1(),
                        Some(var_index2),
                        *monomial.coefficient(),
                    ));
                } else {
                    linear_terms.push((monomial.var_index1(), *monomial.coefficient()));
                }
            }
            let lhs = format_mixed_expression(
                &linear_terms,
                &quadratic_terms,
                &var_names,
                *inequality.polynomial.constant(),
            );
            let relation = relation_to_str(inequality.relation);
            let rhs = format_number(inequality.rhs);
            let name = self
                .quadratic_constraint_names
                .get(quadratic_index)
                .cloned()
                .unwrap_or_else(|| format!("qc{quadratic_index}"));
            output.push_str(&format!("  {name}: {lhs} {relation} {rhs}\n"));
        }

        append_bounds_and_domains(
            &mut output,
            &var_names,
            &self.basic.linear.lb,
            &self.basic.linear.ub,
            &self.basic.linear.var_types,
        );
        output.push_str("End\n");
        output
    }
}

impl LPExportableModel for BasicQuadraticTetradModel {
    fn to_lp_string(&self) -> String {
        QuadraticTetradModel::from_basic(self.clone()).to_lp_string()
    }
}

#[cfg(test)]
mod tests {
    use super::LPExportableModel;
    use super::*;
    use crate::model::QuadraticInequality;
    use crate::model::{ConstraintRelation, ObjectiveCategory};
    use crate::token::Token;
    use crate::variable::{BinaryVariableItem, UContinuousVariableItem, VariableType};

    #[test]
    fn linear_model_lp_export_contains_expected_sections() {
        let mut basic = BasicLinearTriadModel::new("lp_linear");
        basic.add_variable_with_bounds(
            Token::from_generic(BinaryVariableItem::auto("x"), 0),
            0.0,
            1.0,
            VariableType::Binary,
        );
        basic.add_variable_with_bounds(
            Token::from_generic(UContinuousVariableItem::auto("y"), 1),
            0.0,
            f64::INFINITY,
            VariableType::UContinuous,
        );
        let mut row = SparseVector::new();
        row.add(0, 1.0);
        row.add(1, 2.0);
        basic.add_constraint_with_metadata(row, 3.0, "cap".to_string(), None, false, 0, None, None);

        let mut model = LinearTriadModel::from_basic(basic);
        model.set_objective(vec![1.0, -2.0], ObjectiveCategory::Maximum);

        let lp = model.to_lp_string();
        assert!(lp.contains("Maximize"));
        assert!(lp.contains("Subject To"));
        assert!(lp.contains("Bounds"));
        assert!(lp.contains("Binaries"));
        assert!(lp.contains("cap:"));
        assert!(lp.contains("x0"));
    }

    #[test]
    fn quadratic_model_lp_export_contains_quadratic_sections() {
        let mut basic = BasicQuadraticTetradModel::new("lp_quadratic");
        basic.linear.add_variable_with_bounds(
            Token::from_generic(UContinuousVariableItem::auto("x"), 0),
            0.0,
            f64::INFINITY,
            VariableType::UContinuous,
        );
        basic.linear.add_variable_with_bounds(
            Token::from_generic(UContinuousVariableItem::auto("y"), 1),
            0.0,
            f64::INFINITY,
            VariableType::UContinuous,
        );

        let mut model = QuadraticTetradModel::from_basic(basic);
        let mut q = SparseMatrix::new();
        q.add_row(SparseVector::new());
        q.add_row(SparseVector::new());
        q.rows[0].add(0, 2.0);
        q.rows[0].add(1, 1.5);
        model.set_objective(vec![1.0, 0.0], q, ObjectiveCategory::Minimum);

        model.add_quadratic_constraint_with_metadata(
            QuadraticInequality::new(
                crate::flatten::Quadratic::new(
                    vec![
                        crate::flatten::QuadraticMonomial::new_quadratic(1.0, 0, 0),
                        crate::flatten::QuadraticMonomial::new_linear(-1.0, 1),
                    ],
                    0.5,
                ),
                ConstraintRelation::LessEqual,
                2.0,
            ),
            "qc_cap".to_string(),
            None,
            false,
            0,
            None,
            None,
        );

        let lp = model.to_lp_string();
        assert!(lp.contains("Minimize"));
        assert!(lp.contains("obj:"));
        assert!(lp.contains("["));
        assert!(lp.contains("qc_cap:"));
        assert!(lp.contains("^ 2"));
    }
}

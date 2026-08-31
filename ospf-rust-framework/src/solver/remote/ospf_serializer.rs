//! OSPF 模型远程序列化
//! OSPF model remote serialization

use ospf_rust_core::intermediate::{
    BasicLinearTriadModel, LinearTriadModel, QuadraticTetradModel, SparseMatrix,
};
use ospf_rust_core::model::{ConstraintRelation, ObjectiveCategory};
use ospf_rust_core::solver::{SolverOutput, SolverStatus};
use ospf_rust_core::variable::VariableType;

use super::domain::{
    RemoteSolverResult, SerializedConstraint, SerializedConstraintCell, SerializedConstraintSign,
    SerializedLinearModel, SerializedObjective, SerializedObjectiveCategory,
    SerializedObjectiveCell, SerializedQuadraticConstraint, SerializedQuadraticConstraintCell,
    SerializedQuadraticModel, SerializedQuadraticObjective, SerializedQuadraticObjectiveCell,
    SerializedSolution, SerializedVariable, SerializedVariableType, SolveResult,
};

/// OSPF 远程模型序列化器。
/// OSPF remote model serializer.
#[derive(Debug, Clone, Copy, Default)]
pub struct OspfRemoteModelSerializer;

impl OspfRemoteModelSerializer {
    /// 创建序列化器。
    /// Create serializer.
    pub fn new() -> Self {
        Self
    }

    /// 序列化线性三角模型。
    /// Serialize a linear triad model.
    pub fn serialize_linear(&self, model: &LinearTriadModel) -> SerializedLinearModel {
        serialize_linear_model(model)
    }

    /// 序列化二次四角模型。
    /// Serialize a quadratic tetrad model.
    pub fn serialize_quadratic(&self, model: &QuadraticTetradModel) -> SerializedQuadraticModel {
        serialize_quadratic_model(model)
    }
}

/// 序列化线性三角模型。
/// Serialize a linear triad model.
pub fn serialize_linear_model(model: &LinearTriadModel) -> SerializedLinearModel {
    SerializedLinearModel {
        name: model.basic.name.clone(),
        variables: serialize_variables(&model.basic),
        constraints: serialize_linear_constraints(&model.basic),
        objective: SerializedObjective {
            category: serialize_objective_category(model.objective_category),
            cells: serialize_linear_objective_cells(&model.c),
            constant: 0.0,
        },
    }
}

/// 序列化二次四角模型。
/// Serialize a quadratic tetrad model.
pub fn serialize_quadratic_model(model: &QuadraticTetradModel) -> SerializedQuadraticModel {
    let linear = &model.basic.linear;
    SerializedQuadraticModel {
        name: linear.name.clone(),
        variables: serialize_variables(linear),
        linear_constraints: serialize_linear_constraints(linear),
        quadratic_constraints: model
            .quadratic_constraints
            .iter()
            .enumerate()
            .map(|(row_index, inequality)| {
                let mut linear_cells = Vec::new();
                let mut quadratic_cells = Vec::new();

                for monomial in inequality.polynomial.monomials() {
                    match monomial.var_index2() {
                        Some(col_index2) => {
                            quadratic_cells.push(SerializedQuadraticConstraintCell {
                                row_index,
                                col_index1: monomial.var_index1(),
                                col_index2,
                                coefficient: *monomial.coefficient(),
                            });
                        }
                        None => {
                            linear_cells.push(SerializedConstraintCell {
                                row_index,
                                col_index: monomial.var_index1(),
                                coefficient: *monomial.coefficient(),
                            });
                        }
                    }
                }

                SerializedQuadraticConstraint {
                    linear_cells,
                    quadratic_cells,
                    sign: serialize_constraint_relation(inequality.relation),
                    rhs: inequality.rhs - *inequality.polynomial.constant(),
                    name: model
                        .quadratic_constraint_names
                        .get(row_index)
                        .cloned()
                        .unwrap_or_else(|| format!("qc{}", row_index)),
                }
            })
            .collect(),
        objective: SerializedQuadraticObjective {
            category: serialize_objective_category(model.objective_category),
            linear_cells: serialize_linear_objective_cells(&model.c),
            quadratic_cells: serialize_quadratic_objective_cells(&model.Q),
            constant: 0.0,
        },
    }
}

/// 将远程序列化解转换为核心求解输出。
/// Convert a remote serialized solution into core solver output.
pub fn serialized_solution_to_solver_output(solution: &SerializedSolution) -> SolverOutput {
    let mut output = SolverOutput::new(status_from_solution(
        solution.feasible,
        solution.optimal,
        &solution.solver_status,
        solution.message.as_deref(),
    ))
    .with_time(solution.elapsed);

    if let Some(objective_value) = solution.objective_value {
        output = output.with_objective(objective_value);
    }
    if !solution.variable_values.is_empty() {
        output = output.with_solution(solution.variable_values.clone());
    }
    output.mip_gap = solution.gap;
    output
}

/// 将远程求解结果转换为核心求解输出。
/// Convert a remote solve result into core solver output.
pub fn solve_result_to_solver_output(
    result: &SolveResult,
    solution: Option<Vec<f64>>,
) -> SolverOutput {
    let mut output = SolverOutput::new(status_from_solution(
        result.feasible,
        result.optimal,
        "",
        result.message.as_deref(),
    ))
    .with_time(result.elapsed);

    if let Some(objective_value) = result.objective_value {
        output = output.with_objective(objective_value);
    }
    if let Some(solution) = solution {
        output = output.with_solution(solution);
    }
    output.mip_gap = result.gap;
    output
}

/// 从 JSON 字节读取序列化解。
/// Read serialized solution from JSON bytes.
pub fn serialized_solution_from_json(bytes: &[u8]) -> RemoteSolverResult<SerializedSolution> {
    serde_json::from_slice(bytes).map_err(|err| {
        super::domain::RemoteSolverError::invalid_argument(format!(
            "Failed to parse serialized solution JSON: {}",
            err
        ))
    })
}

fn serialize_variables(model: &BasicLinearTriadModel) -> Vec<SerializedVariable> {
    model
        .variables
        .iter()
        .enumerate()
        .map(|(index, token)| SerializedVariable {
            index,
            name: token.name().to_string(),
            lower_bound: model.lb.get(index).copied().unwrap_or(f64::NEG_INFINITY),
            upper_bound: model.ub.get(index).copied().unwrap_or(f64::INFINITY),
            variable_type: model
                .var_types
                .get(index)
                .copied()
                .map(serialize_variable_type)
                .unwrap_or(SerializedVariableType::Continuous),
        })
        .collect()
}

fn serialize_linear_constraints(model: &BasicLinearTriadModel) -> Vec<SerializedConstraint> {
    model
        .A
        .rows
        .iter()
        .enumerate()
        .map(|(row_index, row)| SerializedConstraint {
            cells: row
                .entries
                .iter()
                .map(|(col_index, coefficient)| SerializedConstraintCell {
                    row_index,
                    col_index: *col_index,
                    coefficient: *coefficient,
                })
                .collect(),
            sign: SerializedConstraintSign::LessEqual,
            rhs: model.b.get(row_index).copied().unwrap_or(0.0),
            name: model
                .constraint_names
                .get(row_index)
                .cloned()
                .unwrap_or_else(|| format!("c{}", row_index)),
        })
        .collect()
}

fn serialize_linear_objective_cells(coefficients: &[f64]) -> Vec<SerializedObjectiveCell> {
    coefficients
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, coefficient)| *coefficient != 0.0)
        .map(|(col_index, coefficient)| SerializedObjectiveCell {
            col_index,
            coefficient,
        })
        .collect()
}

fn serialize_quadratic_objective_cells(
    matrix: &SparseMatrix<f64>,
) -> Vec<SerializedQuadraticObjectiveCell> {
    matrix
        .rows
        .iter()
        .enumerate()
        .flat_map(|(col_index1, row)| {
            row.entries.iter().map(move |(col_index2, coefficient)| {
                SerializedQuadraticObjectiveCell {
                    col_index1,
                    col_index2: *col_index2,
                    coefficient: *coefficient,
                }
            })
        })
        .collect()
}

fn serialize_variable_type(variable_type: VariableType) -> SerializedVariableType {
    match variable_type {
        VariableType::Binary => SerializedVariableType::Binary,
        VariableType::Ternary
        | VariableType::BalancedTernary
        | VariableType::Integer
        | VariableType::UInteger => SerializedVariableType::Integer,
        VariableType::Percentage | VariableType::Continuous | VariableType::UContinuous => {
            SerializedVariableType::Continuous
        }
    }
}

fn serialize_objective_category(category: ObjectiveCategory) -> SerializedObjectiveCategory {
    match category {
        ObjectiveCategory::Minimum => SerializedObjectiveCategory::Minimize,
        ObjectiveCategory::Maximum => SerializedObjectiveCategory::Maximize,
    }
}

fn serialize_constraint_relation(relation: ConstraintRelation) -> SerializedConstraintSign {
    match relation {
        ConstraintRelation::LessEqual => SerializedConstraintSign::LessEqual,
        ConstraintRelation::Equal => SerializedConstraintSign::Equal,
        ConstraintRelation::GreaterEqual => SerializedConstraintSign::GreaterEqual,
    }
}

fn status_from_solution(
    feasible: bool,
    optimal: bool,
    solver_status: &str,
    message: Option<&str>,
) -> SolverStatus {
    let normalized_status = solver_status.trim().to_ascii_uppercase();
    let normalized_message = message.unwrap_or_default().to_ascii_uppercase();

    if normalized_status.contains("UNBOUNDED") || normalized_message.contains("UNBOUNDED") {
        return SolverStatus::Unbounded;
    }
    if normalized_status.contains("INFEASIBLE") || normalized_message.contains("INFEASIBLE") {
        return SolverStatus::Infeasible;
    }
    if normalized_status.contains("TIME_LIMIT") || normalized_status.contains("TIME LIMIT") {
        return SolverStatus::TimeLimit;
    }
    if normalized_status.contains("ITERATION_LIMIT")
        || normalized_status.contains("ITERATION LIMIT")
    {
        return SolverStatus::IterationLimit;
    }
    if normalized_status.contains("NUMERIC") {
        return SolverStatus::NumericError;
    }
    if optimal || normalized_status.contains("OPTIMAL") {
        SolverStatus::Optimal
    } else if feasible || normalized_status.contains("FEASIBLE") {
        SolverStatus::Feasible
    } else if normalized_status.contains("INTERRUPT") {
        SolverStatus::UserInterrupt
    } else {
        SolverStatus::Unknown
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::time::Duration;

    use super::*;
    use ospf_rust_core::intermediate::{
        BasicLinearTriadModel, BasicQuadraticTetradModel, SparseVector,
    };
    use ospf_rust_core::model::{Quadratic, QuadraticInequality, QuadraticMonomial};
    use ospf_rust_core::token::Token;
    use ospf_rust_core::variable::{BinaryVariableItem, UContinuousVariableItem};

    fn row(entries: &[(usize, f64)]) -> SparseVector<f64> {
        let mut row = SparseVector::new();
        for (index, value) in entries {
            row.add(*index, *value);
        }
        row
    }

    #[test]
    fn serialize_linear_model_preserves_names_bounds_types_and_objective() {
        let mut basic = BasicLinearTriadModel::new("linear");
        basic.add_variable_with_bounds(
            Token::from_generic(BinaryVariableItem::auto("x"), 0),
            0.0,
            1.0,
            VariableType::Binary,
        );
        basic.add_variable_with_bounds(
            Token::from_generic(UContinuousVariableItem::auto("y"), 1),
            0.0,
            10.0,
            VariableType::UContinuous,
        );
        basic.add_constraint_with_metadata(
            row(&[(0, 1.0), (1, 2.0)]),
            5.0,
            "cap".to_string(),
            None,
            false,
            0,
            None,
            None,
        );
        let mut model = LinearTriadModel::from_basic(basic);
        model.set_objective(vec![3.0, 0.0], ObjectiveCategory::Maximum);

        let serialized = serialize_linear_model(&model);

        assert_eq!(serialized.name, "linear");
        assert_eq!(
            serialized.variables[0].variable_type,
            SerializedVariableType::Binary
        );
        assert_eq!(
            serialized.variables[1].variable_type,
            SerializedVariableType::Continuous
        );
        assert_eq!(serialized.constraints[0].name, "cap");
        assert_eq!(serialized.constraints[0].cells.len(), 2);
        assert_eq!(
            serialized.objective.category,
            SerializedObjectiveCategory::Maximize
        );
        assert_eq!(serialized.objective.cells.len(), 1);
    }

    #[test]
    fn serialize_quadratic_model_preserves_quadratic_terms_and_constant_rhs() {
        let mut basic = BasicQuadraticTetradModel::new("quadratic");
        basic.linear.add_variable_with_bounds(
            Token::from_generic(UContinuousVariableItem::auto("x"), 0),
            0.0,
            10.0,
            VariableType::UContinuous,
        );
        basic.linear.add_variable_with_bounds(
            Token::from_generic(UContinuousVariableItem::auto("y"), 1),
            0.0,
            10.0,
            VariableType::UContinuous,
        );
        let mut model = QuadraticTetradModel::from_basic(basic);

        let mut q_objective = SparseMatrix::new();
        q_objective.add_row(row(&[(1, 4.0)]));
        model.set_objective(vec![1.0, 0.0], q_objective, ObjectiveCategory::Minimum);

        let polynomial = Quadratic::new(
            vec![
                QuadraticMonomial::new_quadratic(2.0, 0, 1),
                QuadraticMonomial::new_linear(3.0, 1),
            ],
            7.0,
        );
        model.add_quadratic_constraint_with_metadata(
            QuadraticInequality::new(polynomial, ConstraintRelation::GreaterEqual, 11.0),
            "qcap".to_string(),
            None,
            false,
            0,
            None,
            None,
        );

        let serialized = serialize_quadratic_model(&model);

        assert_eq!(serialized.objective.quadratic_cells.len(), 1);
        assert_eq!(serialized.quadratic_constraints[0].name, "qcap");
        assert_eq!(
            serialized.quadratic_constraints[0].sign,
            SerializedConstraintSign::GreaterEqual
        );
        assert_eq!(serialized.quadratic_constraints[0].rhs, 4.0);
        assert_eq!(serialized.quadratic_constraints[0].linear_cells.len(), 1);
        assert_eq!(serialized.quadratic_constraints[0].quadratic_cells.len(), 1);
    }

    #[test]
    fn serialized_solution_converts_to_solver_output() {
        let solution = SerializedSolution {
            feasible: true,
            optimal: false,
            objective_value: Some(3.0),
            gap: Some(0.2),
            variable_values: vec![1.0, 2.0],
            elapsed: Duration::from_millis(9),
            solver_status: "TIME_LIMIT".to_string(),
            message: None,
        };

        let output = serialized_solution_to_solver_output(&solution);

        assert_eq!(output.status, SolverStatus::TimeLimit);
        assert_eq!(output.objective_value, Some(3.0));
        assert_eq!(output.solution, Some(vec![1.0, 2.0]));
        assert_eq!(output.mip_gap, Some(0.2));
        assert_eq!(output.solve_time, Duration::from_millis(9));
    }

    #[test]
    fn solve_result_converts_without_solution_vector() {
        let result = SolveResult {
            feasible: true,
            optimal: true,
            objective_value: Some(8.0),
            gap: Some(0.0),
            elapsed: Duration::from_millis(4),
            checkpoint_ref: None,
            result_ref: None,
            message: None,
            extension: BTreeMap::new(),
        };

        let output = solve_result_to_solver_output(&result, None);

        assert_eq!(output.status, SolverStatus::Optimal);
        assert_eq!(output.objective_value, Some(8.0));
        assert!(output.solution.is_none());
    }
}

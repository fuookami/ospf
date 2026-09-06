//! Demo11 模块 / Demo11 module
use std::error::Error;

use ospf_rust_core::model::{ConstraintRelation, MetaModel, ObjectiveCategory};
use ospf_rust_core::symbol::{LinearExpressionSymbol, flat_map1_indexed};
use ospf_rust_core::variable::{
    UInteger, VariableCombination1D, VariableCombination2D, VariableRange,
};
use ospf_rust_multiarray::Shape;

use super::common::{extract_coeffs, read_solution_value, solve_typed};

/// Node data structure
#[derive(Debug, Clone)]
struct Node {
    name: String,
}

impl Node {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

/// Arc capacity data structure
#[derive(Debug, Clone)]
struct ArcCapacity {
    from: usize,
    to: usize,
    capacity: f64,
}

impl ArcCapacity {
    fn new(from: usize, to: usize, capacity: f64) -> Self {
        Self { from, to, capacity }
    }
}

/// Max flow data structure
#[derive(Debug, Clone)]
struct MaxFlowData {
    nodes: Vec<Node>,
    root: usize,
    end: usize,
    capacities: Vec<ArcCapacity>,
}

impl MaxFlowData {
    fn sample() -> Self {
        Self {
            nodes: (0..9).map(|i| Node::new(&format!("N{}", i))).collect(),
            root: 0,
            end: 8,
            capacities: vec![
                ArcCapacity::new(0, 1, 15.0),
                ArcCapacity::new(0, 2, 10.0),
                ArcCapacity::new(0, 3, 40.0),
                ArcCapacity::new(1, 4, 15.0),
                ArcCapacity::new(2, 5, 10.0),
                ArcCapacity::new(2, 6, 35.0),
                ArcCapacity::new(3, 6, 30.0),
                ArcCapacity::new(3, 7, 20.0),
                ArcCapacity::new(4, 6, 10.0),
                ArcCapacity::new(5, 8, 10.0),
                ArcCapacity::new(6, 7, 10.0),
                ArcCapacity::new(7, 8, 45.0),
            ],
        }
    }
}

/// 最大流问题模型 / Maximum flow problem model
///
/// 所有变量和中间符号都是显式字段。
/// All variables and intermediate symbols are explicit fields.
struct NetworkFlowModel {
    /// 弧变量 / Arc variables
    arc_vars: VariableCombination2D<UInteger>,
    /// 弧模型索引数组 / Arc model index array
    arc_idx: ospf_rust_multiarray::MultiArray<usize, Shape<2>>,
    /// 流量变量 / Flow variable
    flow_vars: VariableCombination1D<UInteger>,
    /// 流量模型索引数组 / Flow model index array
    flow_idx: ospf_rust_multiarray::MultiArray<usize, Shape<1>>,
    /// 流出符号 / Flow out symbol
    flow_out: ospf_rust_core::symbol::SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    /// 流入符号 / Flow in symbol
    flow_in: ospf_rust_core::symbol::SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
}

impl NetworkFlowModel {
    /// 注册模型 / Register model
    fn register(model: &mut MetaModel<f64>, data: &MaxFlowData) -> Result<Self, Box<dyn Error>> {
        let node_count = data.nodes.len();

        // Register 2D arc variables with capacity bounds
        let arc_vars = VariableCombination2D::<UInteger>::with_name_and_range_generator(
            Shape::new([node_count, node_count]),
            "x",
            |_index, vector| format!("{}_{}", vector[0], vector[1]),
            |_index, vector| {
                data.capacities
                    .iter()
                    .find(|arc| arc.from == vector[0] && arc.to == vector[1])
                    .map(|arc| VariableRange::bounded(0.0, arc.capacity))
                    .unwrap_or_else(|| VariableRange::fixed(0.0))
            },
        );
        let arc_idx = model.register_combination(&arc_vars)?;

        // Register 1D flow variable
        let flow_vars = VariableCombination1D::<UInteger>::new(Shape::new([1]), "flow");
        let flow_idx = model.register_combination(&flow_vars)?;

        // Objective: maximize flow (use flow variable directly, no wrapper symbol needed)
        model.add_linear_objective(&[(flow_idx[0], 1.0)], "flow");
        model.set_objective_category(ObjectiveCategory::Maximum);

        // Flow out from each node: sum_j x[node][j]
        let flow_out = flat_map1_indexed(
            "flow_out",
            &data.nodes,
            |node, _n| {
                let monomials: Vec<_> = (0..node_count)
                    .map(|j| {
                        ospf_rust_core::symbol::flatten::LinearMonomial::new(
                            1.0,
                            arc_idx[&[node, j]],
                        )
                    })
                    .collect();
                ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
            },
            |i, _| format!("{}", i),
        );
        model.add_symbol_combination(&flow_out)?;

        // Flow in to each node: sum_i x[i][node]
        let flow_in = flat_map1_indexed(
            "flow_in",
            &data.nodes,
            |node, _n| {
                let monomials: Vec<_> = (0..node_count)
                    .map(|i| {
                        ospf_rust_core::symbol::flatten::LinearMonomial::new(
                            1.0,
                            arc_idx[&[i, node]],
                        )
                    })
                    .collect();
                ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
            },
            |i, _| format!("{}", i),
        );
        model.add_symbol_combination(&flow_in)?;

        Ok(NetworkFlowModel {
            arc_vars,
            arc_idx,
            flow_vars,
            flow_idx,
            flow_out,
            flow_in,
        })
    }

    /// 添加约束 / Add constraints
    fn add_constraints(
        &self,
        model: &mut MetaModel<f64>,
        data: &MaxFlowData,
    ) -> Result<(), Box<dyn Error>> {
        let node_count = data.nodes.len();

        // Flow balance constraints per node
        for node in 0..node_count {
            let out_coeffs = extract_coeffs(&self.flow_out[node]);
            let in_coeffs = extract_coeffs(&self.flow_in[node]);

            // Combine: flow_out - flow_in [+/- flow] = 0
            let mut coeffs = out_coeffs;
            for (idx, coeff) in in_coeffs {
                coeffs.push((idx, -coeff));
            }

            if node == data.root {
                coeffs.push((self.flow_idx[0], -1.0));
                model.add_linear_constraint(
                    &coeffs,
                    ConstraintRelation::Equal,
                    0.0,
                    "root_balance",
                )?;
            } else if node == data.end {
                coeffs.push((self.flow_idx[0], 1.0));
                model.add_linear_constraint(
                    &coeffs,
                    ConstraintRelation::Equal,
                    0.0,
                    "end_balance",
                )?;
            } else {
                model.add_linear_constraint(
                    &coeffs,
                    ConstraintRelation::Equal,
                    0.0,
                    &format!("balance_{}", node),
                )?;
            }
        }

        Ok(())
    }
}

/// Demo11 main function: Maximum flow problem
pub fn run() -> Result<(), Box<dyn Error>> {
    let data = MaxFlowData::sample();

    let mut model = MetaModel::<f64>::new("demo11");
    let network = NetworkFlowModel::register(&mut model, &data)?;

    network.add_constraints(&mut model, &data)?;

    let output = solve_typed(model)?;
    let solution = output.solution;

    println!("=== Demo11 ===");
    println!("status: {:?}", output.status);
    println!(
        "max flow: {:.2}",
        read_solution_value(&solution, network.flow_idx[0])
    );
    for arc in &data.capacities {
        let value = read_solution_value(&solution, network.arc_idx[&[arc.from, arc.to]]);
        if value > 0.0 {
            println!(
                "{} -> {} = {:.2}",
                data.nodes[arc.from].name, data.nodes[arc.to].name, value
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo11() {
        assert!(run().is_ok());
    }
}

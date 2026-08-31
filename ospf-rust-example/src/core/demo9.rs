use std::error::Error;

use ospf_rust_multiarray::{MultiArray, Shape};
use ospf_rust_core::model::{MetaModel, ObjectiveCategory, ConstraintRelation};
use ospf_rust_core::symbol::{
    SymbolCombination, LinearExpressionSymbol, flat_map1,
};
use ospf_rust_core::variable::{
    Integer, UContinuous, VariableCombination1D, VariableRange,
};

use super::common::{read_solution_value, solve_typed};

/// Settlement data structure
#[derive(Debug, Clone)]
struct Settlement {
    name: String,
    x: f64,
    y: f64,
}

impl Settlement {
    fn new(name: &str, x: f64, y: f64) -> Self {
        Self { name: name.to_string(), x, y }
    }
}

fn build_settlements() -> Vec<Settlement> {
    vec![
        Settlement::new("S0", 9.0, 2.0),
        Settlement::new("S1", 2.0, 1.0),
        Settlement::new("S2", 3.0, 8.0),
        Settlement::new("S3", 3.0, -2.0),
        Settlement::new("S4", 5.0, 9.0),
        Settlement::new("S5", 4.0, -2.0),
    ]
}

/// Facility location model using VariableCombination + SymbolCombination
struct LocationModel {
    x: VariableCombination1D<Integer>,
    y: VariableCombination1D<Integer>,
    x_idx: MultiArray<usize, Shape<1>>,
    y_idx: MultiArray<usize, Shape<1>>,
    dx: VariableCombination1D<UContinuous>,
    dy: VariableCombination1D<UContinuous>,
    dx_idx: MultiArray<usize, Shape<1>>,
    dy_idx: MultiArray<usize, Shape<1>>,
    distance_expr: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
}

impl LocationModel {
    fn register(
        model: &mut MetaModel<f64>,
        settlements: &[Settlement],
    ) -> Result<Self, Box<dyn Error>> {
        let n = settlements.len();

        // Scalar decision variables with bounded range
        let x = VariableCombination1D::with_range_generator(
            Shape::new([1]), "x",
            |_, _| VariableRange::bounded(-100.0, 100.0),
        );
        let y = VariableCombination1D::with_range_generator(
            Shape::new([1]), "y",
            |_, _| VariableRange::bounded(-100.0, 100.0),
        );
        let x_idx = model.register_combination(&x)?;
        let y_idx = model.register_combination(&y)?;

        // Per-settlement distance component variables
        let dx = VariableCombination1D::new(Shape::new([n]), "dx");
        let dy = VariableCombination1D::new(Shape::new([n]), "dy");
        let dx_idx = model.register_combination(&dx)?;
        let dy_idx = model.register_combination(&dy)?;

        // Objective symbol: each settlement contributes dx_i + dy_i
        let distance_expr = flat_map1("distance", settlements, |settlement| {
            let i = settlements.iter().position(|s| s.name == settlement.name).unwrap();
            ospf_rust_core::symbol::flatten::Linear::new(
                vec![
                    ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, dx_idx[i]),
                    ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, dy_idx[i]),
                ],
                0.0,
            )
        }, |_, settlement| settlement.name.clone());
        model.add_symbol_combination(&distance_expr)?;

        Ok(LocationModel {
            x, y, x_idx, y_idx,
            dx, dy, dx_idx, dy_idx,
            distance_expr,
        })
    }

    fn add_constraints(
        &self,
        model: &mut MetaModel<f64>,
        settlements: &[Settlement],
    ) -> Result<(), Box<dyn Error>> {
        // Aggregate objective: sum of all distance contributions
        let mut obj_monomials = Vec::new();
        for i in 0..settlements.len() {
            let poly = self.distance_expr.symbol_polynomial(i);
            for m in poly.monomials() {
                obj_monomials.push((m.var_index(), *m.coefficient()));
            }
        }
        model.add_linear_objective(&obj_monomials, "distance");
        model.set_objective_category(ObjectiveCategory::Minimum);

        // Manhattan distance constraints for each settlement
        for (i, settlement) in settlements.iter().enumerate() {
            // dx_i >= -x + settlement.x  =>  -x + dx_i >= -settlement.x
            let dx_lower = ospf_rust_core::symbol::flatten::Linear::new(
                vec![
                    ospf_rust_core::symbol::flatten::LinearMonomial::new(-1.0, self.x_idx[0]),
                    ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, self.dx_idx[i]),
                ],
                0.0,
            );
            let coeffs: Vec<_> = dx_lower.monomials().iter()
                .map(|m| (m.var_index(), *m.coefficient())).collect();
            model.add_linear_constraint(
                &coeffs, ConstraintRelation::GreaterEqual, -settlement.x,
                &format!("dx_lb1_{}", i),
            )?;

            // dx_i >= x - settlement.x  =>  x + dx_i >= settlement.x
            let dx_upper = ospf_rust_core::symbol::flatten::Linear::new(
                vec![
                    ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, self.x_idx[0]),
                    ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, self.dx_idx[i]),
                ],
                0.0,
            );
            let coeffs: Vec<_> = dx_upper.monomials().iter()
                .map(|m| (m.var_index(), *m.coefficient())).collect();
            model.add_linear_constraint(
                &coeffs, ConstraintRelation::GreaterEqual, settlement.x,
                &format!("dx_lb2_{}", i),
            )?;

            // dy_i >= -y + settlement.y  =>  -y + dy_i >= -settlement.y
            let dy_lower = ospf_rust_core::symbol::flatten::Linear::new(
                vec![
                    ospf_rust_core::symbol::flatten::LinearMonomial::new(-1.0, self.y_idx[0]),
                    ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, self.dy_idx[i]),
                ],
                0.0,
            );
            let coeffs: Vec<_> = dy_lower.monomials().iter()
                .map(|m| (m.var_index(), *m.coefficient())).collect();
            model.add_linear_constraint(
                &coeffs, ConstraintRelation::GreaterEqual, -settlement.y,
                &format!("dy_lb1_{}", i),
            )?;

            // dy_i >= y - settlement.y  =>  y + dy_i >= settlement.y
            let dy_upper = ospf_rust_core::symbol::flatten::Linear::new(
                vec![
                    ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, self.y_idx[0]),
                    ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, self.dy_idx[i]),
                ],
                0.0,
            );
            let coeffs: Vec<_> = dy_upper.monomials().iter()
                .map(|m| (m.var_index(), *m.coefficient())).collect();
            model.add_linear_constraint(
                &coeffs, ConstraintRelation::GreaterEqual, settlement.y,
                &format!("dy_lb2_{}", i),
            )?;
        }

        Ok(())
    }
}

/// Demo9 main function: Facility location problem (Manhattan distance)
pub fn run() -> Result<(), Box<dyn Error>> {
    let settlements = build_settlements();

    let mut model = MetaModel::<f64>::new("demo9");
    let loc = LocationModel::register(&mut model, &settlements)?;
    loc.add_constraints(&mut model, &settlements)?;

    let output = solve_typed(model)?;
    let solution = output.solution;

    println!("=== Demo9 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("distance sum: {:.2}", obj);
    }
    println!(
        "position: ({:.2}, {:.2})",
        read_solution_value(&solution, loc.x_idx[0]),
        read_solution_value(&solution, loc.y_idx[0])
    );
    for (i, settlement) in settlements.iter().enumerate() {
        println!(
            "{} ({:.2},{:.2}) distance components ({:.2},{:.2})",
            settlement.name,
            settlement.x,
            settlement.y,
            read_solution_value(&solution, loc.dx_idx[i]),
            read_solution_value(&solution, loc.dy_idx[i])
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo9() {
        assert!(run().is_ok());
    }
}

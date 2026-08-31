use std::error::Error;

use ospf_rust_multiarray::Shape;
use ospf_rust_core::model::{MetaModel, ObjectiveCategory, ConstraintRelation};
use ospf_rust_core::symbol::{
    LinearExpressionSymbol, flat_map1_indexed,
};
use ospf_rust_core::variable::{UInteger, VariableCombination2D};

use super::common::{read_solution_value, solve_typed, extract_coeffs};

/// Dealer data structure
#[derive(Debug, Clone)]
struct Dealer {
    name: String,
    demand: f64,
    distance_to_centers: Vec<f64>,
}

impl Dealer {
    fn new(name: &str, demand: f64, distance_to_centers: Vec<f64>) -> Self {
        Self {
            name: name.to_string(),
            demand,
            distance_to_centers,
        }
    }

    fn distance_to(&self, center_idx: usize) -> f64 {
        self.distance_to_centers[center_idx]
    }
}

/// Distribution center data structure
#[derive(Debug, Clone)]
struct Center {
    name: String,
    supply: f64,
}

impl Center {
    fn new(name: &str, supply: f64) -> Self {
        Self {
            name: name.to_string(),
            supply,
        }
    }
}

/// Build dealer list
fn build_dealers() -> Vec<Dealer> {
    vec![
        Dealer::new("D0", 100.0, vec![100.0, 50.0, 40.0]),
        Dealer::new("D1", 200.0, vec![150.0, 70.0, 90.0]),
        Dealer::new("D2", 150.0, vec![200.0, 60.0, 100.0]),
        Dealer::new("D3", 160.0, vec![140.0, 65.0, 150.0]),
        Dealer::new("D4", 140.0, vec![35.0, 80.0, 130.0]),
    ]
}

/// Build distribution center list
fn build_centers() -> Vec<Center> {
    vec![
        Center::new("C0", 400.0),
        Center::new("C1", 200.0),
        Center::new("C2", 150.0),
    ]
}

/// 车辆配送问题模型 / Vehicle delivery problem model
///
/// 所有变量和中间符号都是显式字段。
/// All variables and intermediate symbols are explicit fields.
struct TransportModel {
    /// 货运量决策变量 / Shipment decision variables
    x_vars: VariableCombination2D<UInteger>,
    /// 货运量模型索引数组 / Shipment model index array
    x_idx: ospf_rust_multiarray::MultiArray<usize, Shape<2>>,
    /// 车辆使用决策变量 / Vehicle usage decision variables
    y_vars: VariableCombination2D<UInteger>,
    /// 车辆使用模型索引数组 / Vehicle usage model index array
    y_idx: ospf_rust_multiarray::MultiArray<usize, Shape<2>>,
    /// 成本符号 / Cost symbol
    cost: ospf_rust_core::symbol::SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    /// 供应符号 / Supply symbol
    trans: ospf_rust_core::symbol::SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    /// 需求符号 / Demand symbol
    receive: ospf_rust_core::symbol::SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
}

impl TransportModel {
    /// 注册模型 / Register model
    fn register(
        model: &mut MetaModel<f64>,
        dealers: &[Dealer],
        centers: &[Center],
    ) -> Result<Self, Box<dyn Error>> {
        // Register 2D variable combinations
        let variable_shape = Shape::new([dealers.len(), centers.len()]);
        let x_vars: VariableCombination2D<UInteger> = VariableCombination2D::with_name_generator(
            variable_shape.clone(),
            "x",
            |_index, vector| format!("{}_{}", vector[0], vector[1]),
        );
        let y_vars: VariableCombination2D<UInteger> = VariableCombination2D::with_name_generator(
            variable_shape.clone(),
            "y",
            |_index, vector| format!("{}_{}", vector[0], vector[1]),
        );
        let x_idx = model.register_combination(&x_vars)?;
        let y_idx = model.register_combination(&y_vars)?;

        // Objective: minimize cost = sum(distance[d][c] * y[d][c])
        let cost = flat_map1_indexed("cost", dealers, |d, dealer| {
            let monomials: Vec<_> = (0..centers.len())
                .map(|c| ospf_rust_core::symbol::flatten::LinearMonomial::new(
                    dealer.distance_to(c),
                    y_idx[&[d, c]],
                ))
                .collect();
            ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
        }, |_, dealer| dealer.name.clone());
        model.add_symbol_combination(&cost)?;

        // Supply constraints per center: sum_d x[d][c] <= supply[c]
        let trans = flat_map1_indexed("trans", centers, |c, _center| {
            let monomials: Vec<_> = (0..dealers.len())
                .map(|d| ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[&[d, c]]))
                .collect();
            ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
        }, |_, center| center.name.clone());
        model.add_symbol_combination(&trans)?;

        // Demand constraints per dealer: sum_c x[d][c] >= demand[d]
        let receive = flat_map1_indexed("receive", dealers, |d, _dealer| {
            let monomials: Vec<_> = (0..centers.len())
                .map(|c| ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[&[d, c]]))
                .collect();
            ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
        }, |_, dealer| dealer.name.clone());
        model.add_symbol_combination(&receive)?;

        Ok(TransportModel {
            x_vars,
            x_idx,
            y_vars,
            y_idx,
            cost,
            trans,
            receive,
        })
    }

    /// 添加约束和目标 / Add constraints and objective
    fn add_constraints(
        &self,
        model: &mut MetaModel<f64>,
        dealers: &[Dealer],
        centers: &[Center],
        car_capacity: f64,
    ) -> Result<(), Box<dyn Error>> {
        // Aggregate cost objective
        let mut cost_coeffs = Vec::new();
        for d in 0..dealers.len() {
            let poly = self.cost.symbol_polynomial(d);
            for m in poly.monomials() {
                cost_coeffs.push((m.var_index(), *m.coefficient()));
            }
        }
        model.add_linear_objective(&cost_coeffs, "cost");
        model.set_objective_category(ObjectiveCategory::Minimum);

        // Supply constraints per center
        for c in 0..centers.len() {
            let coeffs = extract_coeffs(&self.trans[c]);
            model.add_linear_constraint(
                &coeffs,
                ConstraintRelation::LessEqual,
                centers[c].supply,
                &format!("supply_{}", c),
            )?;
        }

        // Demand constraints per dealer
        for d in 0..dealers.len() {
            let coeffs = extract_coeffs(&self.receive[d]);
            model.add_linear_constraint(
                &coeffs,
                ConstraintRelation::GreaterEqual,
                dealers[d].demand,
                &format!("demand_{}", d),
            )?;
        }

        // Truck capacity constraints: x[d][c] - capacity * y[d][c] <= 0
        for d in 0..dealers.len() {
            for c in 0..centers.len() {
                let truck = ospf_rust_core::symbol::flatten::Linear::new(
                    vec![
                        ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, self.x_idx[&[d, c]]),
                        ospf_rust_core::symbol::flatten::LinearMonomial::new(-car_capacity, self.y_idx[&[d, c]]),
                    ],
                    0.0,
                );
                let coeffs: Vec<_> = truck.monomials().iter()
                    .map(|m| (m.var_index(), *m.coefficient())).collect();
                model.add_linear_constraint(
                    &coeffs,
                    ConstraintRelation::LessEqual,
                    0.0,
                    &format!("truck_{}_{}", d, c),
                )?;
            }
        }

        Ok(())
    }
}

/// Demo13 main function: Vehicle delivery problem
pub fn run() -> Result<(), Box<dyn Error>> {
    let dealers = build_dealers();
    let centers = build_centers();
    let car_capacity = 18.0;

    let mut model = MetaModel::<f64>::new("demo13");
    let transport = TransportModel::register(&mut model, &dealers, &centers)?;

    transport.add_constraints(&mut model, &dealers, &centers, car_capacity)?;

    let output = solve_typed(model)?;
    let solution = output.solution;

    println!("=== Demo13 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("transport cost: {:.2}", obj);
    }
    for d in 0..dealers.len() {
        for c in 0..centers.len() {
            let shipped = read_solution_value(&solution, transport.x_idx[&[d, c]]);
            if shipped > 0.0 {
                let trucks = read_solution_value(&solution, transport.y_idx[&[d, c]]);
                println!(
                    "{} <- {}: ship {:.2}, trucks {:.2}",
                    dealers[d].name, centers[c].name, shipped, trucks
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo13() {
        assert!(run().is_ok());
    }
}

use std::error::Error;
use ospf_rust_multiarray::{MultiArrayBuilder, Shape};
use ospf_rust_math::symbol::{Linear, LinearMonomial};
use ospf_rust_core::model::{MetaModel, ObjectiveCategory};
use ospf_rust_core::variable::{UInteger, VariableCombination2D};
use super::common::{read_solution_value, solve_typed};

#[derive(Debug, Clone)]
struct Warehouse {
    name: String,
    stowage: f64,
    costs_to_stores: Vec<f64>,
}

impl Warehouse {
    fn new(name: &str, stowage: f64, costs_to_stores: Vec<f64>) -> Self {
        Self {
            name: name.to_string(),
            stowage,
            costs_to_stores,
        }
    }

    fn cost_to(&self, store_idx: usize) -> f64 {
        self.costs_to_stores[store_idx]
    }
}

#[derive(Debug, Clone)]
struct Store {
    name: String,
    demand: f64,
}

impl Store {
    fn new(name: &str, demand: f64) -> Self {
        Self {
            name: name.to_string(),
            demand,
        }
    }
}

fn build_warehouses() -> Vec<Warehouse> {
    vec![
        Warehouse::new("W0", 510.0, vec![12.0, 13.0, 21.0, 7.0]),
        Warehouse::new("W1", 470.0, vec![14.0, 17.0, 8.0, 18.0]),
        Warehouse::new("W2", 520.0, vec![10.0, 11.0, 9.0, 15.0]),
    ]
}

fn build_stores() -> Vec<Store> {
    vec![
        Store::new("S0", 200.0),
        Store::new("S1", 400.0),
        Store::new("S2", 600.0),
        Store::new("S3", 300.0),
    ]
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let warehouses = build_warehouses();
    let stores = build_stores();

    let mut model = MetaModel::<f64>::new("demo7");
    let x_shape = Shape::new([warehouses.len(), stores.len()]);
    let x_vars: VariableCombination2D<UInteger> =
        VariableCombination2D::with_name_generator(x_shape.clone(), "x", |_index, vector| {
            format!("{}_{}", vector[0], vector[1])
        });
    let x_idx = MultiArrayBuilder::from_list(
        x_shape,
        model.register_variables::<UInteger, _>(x_vars.iter().cloned())?,
    );

    let mut cost_terms = Vec::with_capacity(warehouses.len() * stores.len());
    for (w, warehouse) in warehouses.iter().enumerate() {
        for (s, _) in stores.iter().enumerate() {
            cost_terms.push(LinearMonomial::new(
                warehouse.cost_to(s),
                x_vars[&[w, s]].to_owned_symbol(),
            ));
        }
    }
    let cost = Linear::new(cost_terms, 0.0);
    let shipment = MultiArrayBuilder::new_by(Shape::<1>::new([warehouses.len()]), |_idx, vec| {
        let w = vec[0];
        Linear::new(
            stores
                .iter()
                .enumerate()
                .map(|(s, _)| LinearMonomial::new(1.0, x_vars[&[w, s]].to_owned_symbol()))
                .collect(),
            0.0,
        )
    });
    let purchase = MultiArrayBuilder::new_by(Shape::<1>::new([stores.len()]), |_idx, vec| {
        let s = vec[0];
        Linear::new(
            warehouses
                .iter()
                .enumerate()
                .map(|(w, _)| LinearMonomial::new(1.0, x_vars[&[w, s]].to_owned_symbol()))
                .collect(),
            0.0,
        )
    });

    model.set_math_linear_objective(cost, ObjectiveCategory::Minimum, "cost")?;

    for (w, _) in warehouses.iter().enumerate() {
        model.add_math_inequality(
            shipment[w].clone().le(warehouses[w].stowage),
            &format!("stowage_{}", w),
        );
    }

    for (s, _) in stores.iter().enumerate() {
        model.add_math_inequality(
            purchase[s].clone().ge(stores[s].demand),
            &format!("demand_{}", s),
        );
    }

    let output = solve_typed(model)?;
    let solution = output.solution;

    println!("=== Demo7 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("cost: {:.2}", obj);
    }
    for (w, warehouse) in warehouses.iter().enumerate() {
        for (s, store) in stores.iter().enumerate() {
            let value = read_solution_value(&solution, x_idx[&[w, s]]);
            if value >= 1.0 {
                println!("{} -> {} = {:.2}", warehouse.name, store.name, value);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo7() {
        assert!(run().is_ok());
    }
}

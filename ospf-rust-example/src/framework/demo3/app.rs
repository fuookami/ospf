use std::error::Error;

use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::CTUnit;
use ospf_rust_quantities::unit::derived::Meter;
use ospf_rust_framework_csp1d::{
    csp1d_problem, Csp1dColumnGeneration, Csp1dConfiguration, Csp1dSolutionStatus,
    DefaultQuantityArithmetic, FullSumGenerator, GenerationConstraints,
    Material, NSameGenerator, Product, ProductDemand, ProductLegacyInput,
    ReducedCostPricingGenerator, WidthRange,
};

fn quantity(value: f64) -> Quantity<f64, ospf_rust_quantities::unit::Unit> {
    Quantity::new(value, Meter::INSTANT.clone())
}

struct RawProduct {
    width: f64,
    demand: f64,
}

/// 运行 CSP1D 下料问题示例 / Run CSP1D cutting stock problem example
pub fn run() -> Result<(), Box<dyn Error>> {
    let raw_length = 1000.0_f64;
    let raw_products = vec![
        RawProduct { width: 450.0, demand: 97.0 },
        RawProduct { width: 360.0, demand: 610.0 },
        RawProduct { width: 310.0, demand: 395.0 },
        RawProduct { width: 140.0, demand: 211.0 },
    ];

    let products: Vec<Product<f64>> = raw_products
        .iter()
        .enumerate()
        .map(|(index, raw)| {
            Product::legacy(ProductLegacyInput {
                id: format!("p-{index}").into(),
                name: format!("product-{}", raw.width as i64),
                width: vec![raw.width],
                length: None,
                unit_weight: None,
                weight: None,
                max_over_produce_length: None,
                unit: Meter::INSTANT.clone(),
            })
        })
        .collect();

    let material = Material {
        id: "m-1000".into(),
        name: "material-1000".into(),
        width_range: WidthRange::new(quantity(0.0), quantity(raw_length)),
        length: None,
        unit_weight: None,
        machine_id: None,
        available_batches: u64::MAX,
    };

    let demands: Vec<ProductDemand<f64>> = raw_products
        .iter()
        .enumerate()
        .map(|(index, raw)| {
            ProductDemand::legacy_roll(products[index].clone(), raw.demand)
        })
        .collect();

    let problem = csp1d_problem::<f64, _>(|builder| {
        builder
            .products(products.clone())
            .material(material)
            .demands(demands)
            .configuration(Csp1dConfiguration {
                max_initial_plans: 16,
                max_pricing_plans: 32,
                iteration_limit: 16,
            });
    });

    let pricing_enumerator = FullSumGenerator::with_constraints(GenerationConstraints {
        max_knife_count: Some(8),
        ..GenerationConstraints::default()
    })
    .with_max_plans(256);
    let initial_generator = NSameGenerator::default().with_max_plans(16);
    let pricing_generator = ReducedCostPricingGenerator::new(pricing_enumerator);

    let solver = Csp1dColumnGeneration::with_generators(
        Box::new(initial_generator),
        Box::new(pricing_generator),
    );

    let result = solver.solve_with_trace(problem, None);

    let plan_descriptions: Vec<String> = result
        .solution
        .produce
        .cutting_plans
        .iter()
        .map(|usage| {
            let pattern: Vec<String> = usage
                .plan
                .slices
                .iter()
                .map(|slice| format!("{} * {}", slice.width.value, slice.amount))
                .collect();
            format!("{}: {}", pattern.join(","), usage.amount)
        })
        .collect();

    println!("{}", plan_descriptions.join(";"));
    println!(
        "termination={:?}; plans={}",
        result.trace.termination_reason, result.trace.final_plan_count
    );

    Ok(())
}

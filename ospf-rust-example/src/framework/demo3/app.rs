use std::error::Error;
use crate::framework::demo3::domain::{Product, initial_plans};
use crate::framework::demo3::rmp::Rmp;
use crate::framework::demo3::sp::Sp;

pub fn run() -> Result<(), Box<dyn Error>> {
    let stock_length = 1000u64;
    let products = vec![
        Product {
            length: 450,
            demand: 97,
        },
        Product {
            length: 360,
            demand: 610,
        },
        Product {
            length: 310,
            demand: 395,
        },
        Product {
            length: 140,
            demand: 211,
        },
    ];

    let initial = initial_plans(stock_length, &products);
    let mut rmp = Rmp::new(products.clone(), initial);
    let sp = Sp::new();

    for iteration in 0..128usize {
        let lp = rmp.solve_lp()?;
        let (new_plan, reduced_cost) = sp.solve(stock_length, &products, &lp.shadow_prices)?;
        println!(
            "iter {}: lp_obj={:.4}, reduced_cost={:.6}, plan={:?}",
            iteration, lp.objective, reduced_cost, new_plan.amounts
        );

        if reduced_cost >= -1e-6 {
            break;
        }
        if !rmp.add_column_if_new(new_plan) {
            break;
        }
    }

    let solution = rmp.solve_milp()?;
    println!("=== Framework Demo3 ===");
    for (plan, amount) in solution {
        if amount == 0 {
            continue;
        }
        let detail: Vec<String> = plan
            .amounts
            .iter()
            .enumerate()
            .filter(|(_, c)| **c > 0)
            .map(|(idx, count)| format!("{}x{}", products[idx].length, count))
            .collect();
        println!("{} -> {}", detail.join(","), amount);
    }
    Ok(())
}

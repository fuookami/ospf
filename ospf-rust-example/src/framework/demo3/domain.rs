#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Product {
    pub length: u64,
    pub demand: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CuttingPlan {
    pub amounts: Vec<u64>,
}

impl CuttingPlan {
    pub fn new(amounts: Vec<u64>) -> Self {
        Self { amounts }
    }
}

pub fn initial_plans(stock_length: u64, products: &[Product]) -> Vec<CuttingPlan> {
    products
        .iter()
        .enumerate()
        .map(|(idx, product)| {
            let mut amounts = vec![0u64; products.len()];
            amounts[idx] = stock_length / product.length;
            CuttingPlan::new(amounts)
        })
        .collect()
}

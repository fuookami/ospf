use std::cell::Cell;
use std::default::Default;
use ospf_rust_base::{Index, Indexed, Try, OK};
use ospf_rust_core::core::frontend::variable::BinVariable1;
use ospf_rust_derive::{AutoIndexed};
use ospf_rust_math::Symbol;
use ospf_rust_multiarray::Shape1;

#[derive(Default, Debug, Clone, AutoIndexed)]
struct Company {
    index: Index<Company>,
    pub capital: f64,
    pub liability: f64,
    pub profit: f64,
}

struct Demo1 {
    companies: Vec<Company>,
    min_capital: f64,
    max_capital: f64,

    x: Cell<Option<BinVariable1>>
}

impl Demo1 {
    pub fn new() -> Self {
        let companies = vec![
            Company {
                capital: 3.48,
                liability: 1.28,
                profit: 5400.0,
                ..Default::default()
            },
            Company {
                capital: 5.62,
                liability: 2.53,
                profit: 4600.0,
                ..Default::default()
            },
            Company {
                capital: 7.33,
                liability: 1.02,
                profit: 4600.0,
                ..Default::default()
            },
            Company {
                capital: 6.27,
                liability: 3.55,
                profit: 3300.0,
                ..Default::default()
            },
            Company {
                capital: 2.14,
                liability: 0.53,
                profit: 3300.0,
                ..Default::default()
            }
        ];

        Demo1 {
            companies,
            min_capital: 10.0,
            max_capital: 5.0,
            x: Cell::new(None),
        }
    }

    fn init_variable(&self) -> Try {
        let x = BinVariable1::new("x".to_string(), Shape1::new_with(self.companies.len()));
        for c in &self.companies {
            let base_name = x.name();
            x[*c.index].set_name(&format!("{}_{}", base_name, *c.index));
        }
        self.x.set(Some(x));
        OK.into()
    }
}

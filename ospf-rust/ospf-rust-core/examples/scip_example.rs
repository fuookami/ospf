extern crate libc;

use sfa_opf_rust_minimum::solvers::scip::*;
use std::env;

struct Company {
    index: usize,
    property: f64,
    liability: f64,
    profit: f64,
}

impl Company {
    fn new(index: usize, property: f64, liability: f64, profit: f64) -> Self {
        Self {
            index: index,
            property: property,
            liability: liability,
            profit: profit,
        }
    }
}

struct Demo1 {
    scip: *mut SCIP,
    min_property: f64,
    max_libability: f64,
    companies: Vec<Company>,
}

impl Demo1 {
    fn new() -> Demo1 {
        let mut companies = Vec::new();
        companies.push(Company::new(0, 3.48, 1.28, 5400.));
        companies.push(Company::new(1, 5.62, 2.53, 2300.));
        companies.push(Company::new(2, 7.33, 1.02, 4600.));
        companies.push(Company::new(3, 6.27, 3.55, 3300.));
        companies.push(Company::new(4, 2.14, 0.53, 980.));

        Self {
            scip: 0 as *mut SCIP,
            min_property: 10.,
            max_libability: 5.,
            companies: companies,
        }
    }
}

#[link(name = "libscip")]
fn main() {
    let mut prob = Demo1::new();

    unsafe {
        let ret_code = SCIPcreate(&mut prob.scip as *mut *mut SCIP);
        if ret_code != SCIP_Retcode_SCIP_OKAY {
            panic!("failed create!");
        }
    }
}

extern crate libc;

use sfa_opf_rust_minimum::solvers::gurobi::*;
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
    grb_env: *mut GRBenv,
    grb_model: *mut GRBmodel,
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
            grb_env: 0 as *mut GRBenv,
            grb_model: 0 as *mut GRBmodel,
            min_property: 10.,
            max_libability: 5.,
            companies: companies,
        }
    }
}

fn get_int_attr<const D: usize>(grb_model: *mut GRBmodel, attr: &'static [u8; D]) -> i32 {
    unsafe {
        let ret: Box<i32> = Box::new(0);
        let ptr = Box::into_raw(ret);
        let ret_code = GRBgetintattr(
            grb_model,
            attr as *const u8 as *const libc::c_char,
            ptr as *mut libc::c_int,
        );
        if ret_code != 0 {
            panic!("failed!");
        }

        let ret = Box::from_raw(ptr);
        *ret
    }
}

fn get_dbl_attr<const D: usize>(grb_model: *mut GRBmodel, attr: &'static [u8; D]) -> f64 {
    unsafe {
        let ret: Box<f64> = Box::new(0.);
        let ptr = Box::into_raw(ret);
        let ret_code = GRBgetdblattr(
            grb_model,
            attr as *const u8 as *const libc::c_char,
            ptr as *mut f64,
        );
        if ret_code != 0 {
            panic!("failed!");
        }

        let ret = Box::from_raw(ptr);
        *ret
    }
}

#[link(name = "gurobi81")]
fn main() {
    let mut prob = Demo1::new();

    unsafe {
        let ret_code = GRBloadenv(&mut prob.grb_env as *mut *mut GRBenv, 0 as *const i8);
        if ret_code != 0 {
            panic!("failed load env!");
        }
    }

    unsafe {
        let ret_code = GRBnewmodel(
            prob.grb_env,
            &mut prob.grb_model as *mut *mut GRBmodel,
            "prob".as_ptr() as *const libc::c_char,
            0,
            0 as *mut f64,
            0 as *mut f64,
            0 as *mut f64,
            0 as *mut i8,
            0 as *mut *mut i8,
        );
        if ret_code != 0 {
            panic!("failed create model!");
        }
    }

    unsafe {
        for company in &prob.companies {
            let var_name = format!("x_{}", company.index);
            let ret_code = GRBaddvar(
                prob.grb_model,
                0,
                0 as *mut libc::c_int,
                0 as *mut f64,
                -company.profit,
                0.,
                1.,
                GRB_BINARY as libc::c_char,
                var_name.as_ptr() as *const libc::c_char,
            );
            if ret_code != 0 {
                panic!("failed create var!");
            }
        }
    }

    unsafe {
        let mut indexs = Vec::<libc::c_int>::new();
        let mut vals = Vec::new();
        for company in &prob.companies {
            indexs.push(company.index as libc::c_int);
            vals.push(company.property);
        }
        let ret_code = GRBaddconstr(
            prob.grb_model,
            indexs.len() as libc::c_int,
            indexs.as_ptr() as *mut libc::c_int,
            vals.as_ptr() as *mut f64,
            GRB_GREATER_EQUAL as libc::c_char,
            prob.min_property,
            "".as_ptr() as *const libc::c_char,
        );
        if ret_code != 0 {
            panic!("failed add constrant1!");
        }
    }

    unsafe {
        let mut indexs = Vec::<libc::c_int>::new();
        let mut vals = Vec::new();
        for company in &prob.companies {
            indexs.push(company.index as libc::c_int);
            vals.push(company.liability);
        }
        let ret_code = GRBaddconstr(
            prob.grb_model,
            indexs.len() as libc::c_int,
            indexs.as_ptr() as *mut libc::c_int,
            vals.as_ptr() as *mut f64,
            GRB_LESS_EQUAL as libc::c_char,
            prob.max_libability,
            "".as_ptr() as *const libc::c_char,
        );
        if ret_code != 0 {
            panic!("failed add constrant2!");
        }
    }

    unsafe {
        let path = format!("{}/grb.lp", env::current_dir().unwrap().display());
        let ret_code = GRBwrite(prob.grb_model, path.as_ptr() as *const libc::c_char);
        if ret_code != 0 {
            println!("failed write lp!");
        }
    }

    unsafe {
        let ret_code = GRBoptimize(prob.grb_model);
        if ret_code != 0 {
            panic!("failed optimize!");
        }
    }

    let status = get_int_attr(prob.grb_model, GRB_INT_ATTR_STATUS) as u32;
    if status != GRB_OPTIMAL {
        panic!("not optimal!");
    }

    let solution_count = get_int_attr(prob.grb_model, GRB_INT_ATTR_SOLCOUNT) as u32;
    if solution_count == 0 {
        panic!("no solution!");
    }

    println!("obj: {}", get_dbl_attr(prob.grb_model, GRB_DBL_ATTR_OBJ));
    println!(
        "time: {}",
        get_dbl_attr(prob.grb_model, GRB_DBL_ATTR_RUNTIME)
    );
    println!(
        "possible best obj: {}",
        get_dbl_attr(prob.grb_model, GRB_DBL_ATTR_OBJBOUND)
    );
    println!("gap: {}", get_dbl_attr(prob.grb_model, GRB_DBL_ATTR_MIPGAP));

    let mut selection = Vec::new();
    unsafe {
        let mut solution = Vec::new();
        for _ in &prob.companies {
            solution.push(0.);
        }
        let ret_code = GRBgetdblattrarray(
            prob.grb_model,
            GRB_DBL_ATTR_X as *const u8 as *const libc::c_char,
            0,
            prob.companies.len() as libc::c_int,
            solution.as_mut_ptr(),
        );
        if ret_code != 0 {
            panic!("failed get solution!");
        }

        for i in 0..solution.len() {
            if (solution[i] - 1.).abs() < 1e-5 {
                selection.push(i);
            }
        }
    }

    if selection == [0, 1, 2] {
        println!("right!");
    }

    println!("ok!");
}

use std::fmt::{Debug, Display, Formatter};
use std::io::Write;
use std::ops::Deref;

use ospf_rust_math::RealNumber;

use crate::core::frontend::variable::VariableType;
use super::model::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct LinearConstraintCell<T> {
    pub row_index: usize,
    pub col_index: usize,
    pub coefficient: T,
}

impl<T: Clone> ModelCell for LinearConstraintCell<T> {
    type CoefficientValueType = T;

    fn coefficient(&self) -> &T {
        &self.coefficient
    }
}

impl<T: Clone> ConstraintCell for LinearConstraintCell<T> {
    fn row_index(&self) -> usize {
        self.row_index
    }
}

type LinearConstraint<'a, T> = Constraint<'a, LinearConstraintCell<T>>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct LinearObjectiveCell<T> {
    pub col_index: usize,
    pub coefficient: T,
}

impl<T: Clone> ModelCell for LinearObjectiveCell<T> {
    type CoefficientValueType = T;

    fn coefficient(&self) -> &T {
        &self.coefficient
    }
}

type LinearObjective<'a, T> = Objective<'a, LinearObjectiveCell<T>>;

pub type BasicLinearTriadModelView<'a, C, T> = dyn BasicModelView<
    'a,
    SolutionValueType = T,
    CoefficientValueType = C,
    ConstraintCellType = LinearConstraintCell<C>,
>;
pub type LinearTriadModelView<'a, C, T> = dyn ModelView<
    'a,
    SolutionValueType = T,
    CoefficientValueType = C,
    ConstraintCellType = LinearConstraintCell<C>,
    ObjectiveCellType = LinearObjective<C>,
>;

pub struct BasicLinearTriadModel<'a, C, T> {
    pub variables: Vec<Variable<'a, T>>,
    pub constraints: LinearConstraint<'a, C>,
    pub name: String,
}

impl<'a, C, T> BasicLinearTriadModel<'a, C, T>
where
    T: RealNumber,
{
    fn normalized(&self) -> bool {
        self.variables.iter().all(|v| {
            (v.lower_bound.borrow().is_neg_inf()
                || v.lower_bound.borrow().as_ref().unwrap() == T::ZERO)
                && (v.upper_bound.borrow().is_inf()
                || v.upper_bound.borrow().as_ref().unwrap() == T::ZERO)
        })
    }

    fn linear_relax(&mut self) {
        self.variables.iter().for_each(|v| {
            match v.variable_type.get().as_ref().unwrap() {
                VariableType::Binary => v.variable_type.set(VariableType::Percentage),
                VariableType::Ternary | VariableType::UInteger => {
                    v.variable_type.set(VariableType::UContinuous)
                }
                VariableType::BalancedTernary | VariableType::Integer => {
                    v.variable_type.set(VariableType::Continuous)
                }
                _ => {}
            }
        })
    }

    fn normalize(&mut self) {
        todo!()
    }
}

impl<'a, C, T> BasicModelView<'a> for BasicLinearTriadModel<'a, C, T> {
    type SolutionValueType = T;
    type CoefficientValueType = C;
    type ConstraintCellType = LinearConstraintCell<C>;

    fn variables(&self) -> &[Variable<'a, T>] {
        self.variables.as_slice()
    }

    fn constraints(&self) -> &LinearConstraint<'a, C> {
        &self.constraints
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn export_lp(&self, writer: &dyn Write) -> std::io::Result<()> {
        todo!()
    }
}

pub struct LinearTriadModel<'a, C, T> {
    basic: BasicLinearTriadModel<'a, C, T>,
    pub objective: LinearObjective<'a, C>,
}

impl<'a, C, T> LinearTriadModel<'a, C, T>
where
    T: RealNumber,
{
    fn normalized(&self) -> bool {
        self.basic.normalized()
    }

    fn linear_relax(&mut self) {
        self.basic.linear_relax();
    }

    fn normalize(&mut self) {
        self.basic.normalize();
    }

    fn dual(&self) -> LinearTriadModel<'a, T, T> {
        todo!()
    }
}

impl<'a, C, T> Debug for LinearTriadModel<'a, C, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.basic.name)
    }
}

impl<'a, C, T> Display for LinearTriadModel<'a, C, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.basic.name)
    }
}

impl<'a, C, T> Deref for LinearTriadModel<'a, C, T> {
    type Target = BasicLinearTriadModel<'a, C, T>;

    fn deref(&self) -> &Self::Target {
        &self.basic
    }
}

impl<'a, C, T> BasicModelView for LinearTriadModel<'a, C, T> {
    type SolutionValueType = T;
    type CoefficientValueType = C;
    type ConstraintCellType = LinearConstraintCell<C>;

    fn variables(&self) -> &[Variable<'a, Self::SolutionValueType>] {
        self.basic.variables.as_slice()
    }

    fn constraints(&self) -> &LinearConstraint<'a, C> {
        &self.basic.constraints
    }

    fn name(&self) -> &str {
        &self.basic.name
    }

    fn contains_continuous(&self) -> bool {
        self.basic.contains_continuous()
    }

    fn contains_binary(&self) -> bool {
        self.basic.contains_binary()
    }

    fn contains_integer(&self) -> bool {
        self.basic.contains_integer()
    }

    fn contains_not_binary_integer(&self) -> bool {
        self.basic.contains_not_binary_integer()
    }

    fn export_lp(&self, writer: &dyn Write) -> std::io::Result<()> {
        self.basic.export_lp(writer)?;
        todo!()
    }
}

impl<'a, C, T> ModelView<'a> for LinearTriadModel<'a, C, T> {
    type ObjectiveCellType = LinearObjectiveCell<C>;

    fn objective(&self) -> &LinearObjective<C> {
        &self.objective
    }
}

use std::io::Write;
use std::ops::Deref;

use super::model::*;
use crate::core::frontend::variable::VariableType;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct QuadraticConstraintCell<T> {
    pub row_index: usize,
    pub col_index1: usize,
    pub col_index2: Option<usize>,
    pub coefficient: T,
}

impl<T: Clone> ModelCell for QuadraticConstraintCell<T> {
    type CoefficientValueType = T;

    fn coefficient(&self) -> T {
        &self.coefficient
    }
}

impl<T: Clone> ConstraintCell for QuadraticConstraintCell<T> {
    fn row_index(&self) -> usize {
        self.row_index
    }
}

type QuadraticConstraint<'a, T> = Constraint<'a, QuadraticConstraintCell<T>>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct QuadraticObjectiveCell<T> {
    pub col_index1: usize,
    pub col_index2: Option<usize>,
    pub coefficient: T,
}

impl<T: Clone> ModelCell for QuadraticObjectiveCell<T> {
    type CoefficientValueType = T;

    fn coefficient(&self) -> T {
        &self.coefficient
    }
}

type QuadraticObjective<'a, T> = Objective<'a, QuadraticObjectiveCell<T>>;

pub type BasicQuadraticTetradModelView<'a, C, T> = dyn BasicModelView<
    'a,
    SolutionValueType = T,
    CoefficientValueType = C,
    ConstraintCellType = QuadraticConstraintCell<C>,
>;
pub type QuadraticTetradModelView<'a, C, T> = dyn ModelView<
    'a,
    SolutionValueType = T,
    CoefficientValueType = C,
    ConstraintCellType = QuadraticConstraintCell<C>,
    ObjectiveCellType = QuadraticObjectiveCell<C>,
>;

pub struct BasicQuadraticTetradModel<'a, C, T> {
    pub variables: Vec<Variable<'a, T>>,
    pub constraints: Vec<QuadraticConstraint<'a, C>>,
    pub name: String,
}

impl<'a, C, T> BasicModelView<'a> for BasicQuadraticTetradModel<'a, C, T> {
    type SolutionValueType = T;
    type CoefficientValueType = C;
    type ConstraintCellType = QuadraticConstraintCell<C>;

    fn variables(&self) -> &[Variable<'a, T>] {
        self.variables.as_slice()
    }

    fn constraints(&self) -> &QuadraticConstraint<'a, C> {
        &self.constraints
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn export_lp(&self, writer: &dyn Write) -> std::io::Result<()> {
        todo!()
    }
}

pub struct QuadraticTetradModel<'a, C, T> {
    basic: BasicQuadraticTetradModel<'a, C, T>,
    pub objective: QuadraticObjective<'a, C>,
}

impl<'a, C, T> Deref for QuadraticTetradModel<'a, C, T> {
    type Target = BasicQuadraticTetradModel<'a, C, T>;

    fn deref(&self) -> &Self::Target {
        &self.basic
    }
}

impl<'a, C, T> BasicModelView for QuadraticTetradModel<'a, C, T> {
    type SolutionValueType = T;
    type CoefficientValueType = C;
    type ConstraintCellType = QuadraticConstraintCell<C>;

    fn variables(&self) -> &[Variable<'a, Self::SolutionValueType>] {
        self.basic.variables.as_slice()
    }

    fn constraints(&self) -> &QuadraticConstraint<'a, C> {
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

impl<'a, C, T> ModelView<'a> for QuadraticTetradModel<'a, C, T> {
    type ObjectiveCellType = QuadraticObjectiveCell<T>;

    fn objective(&self) -> &QuadraticObjective<'a, C> {
        &self.objective
    }
}

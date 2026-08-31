use std::cell::{Cell, RefCell};
use std::fmt::{Debug, Display, Formatter};
use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::core::frontend::model::mechanism::objective_category::ObjectiveCategory;
use crate::core::frontend::model::mechanism::Sign;
use crate::core::frontend::variable::VariableType;

#[derive(Clone)]
pub(crate) struct Variable<'a, T> {
    pub index: usize,
    pub lower_bound: RefCell<T>,
    pub upper_bound: RefCell<T>,
    pub variable_type: Cell<VariableType>,
    pub name: &'a str,
    pub initial_result: Option<T>,
}

impl<'a, T> Clone for Variable<'a, T> {
    fn clone(&self) -> Self {
        Variable {
            index: self.index,
            lower_bound: RefCell::new(self.lower_bound.get()),
            upper_bound: RefCell::new(self.upper_bound.get()),
            variable_type: self.variable_type.clone(),
            name: self.name,
            initial_result: self.initial_result.clone(),
        }
    }
}

impl<'a, T> Debug for Variable<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl<'a, T> Display for Variable<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

pub(super) trait ModelCell: Clone {
    type CoefficientValueType: Clone;

    fn coefficient(&self) -> &Self::CoefficientValueType;
}

pub(super) trait ConstraintCell: ModelCell {
    fn row_index(&self) -> usize;
}

#[derive(Clone)]
pub(super) struct Constraint<'a, C>
where
    C: ConstraintCell,
{
    pub lhs: Vec<Vec<C>>,
    pub signs: Vec<Sign>,
    pub rhs: Vec<<C as ModelCell>::CoefficientValueType>,
    pub names: Vec<&'a str>,
}

#[derive(Clone)]
pub(super) struct Objective<'a, C>
where
    C: ModelCell,
{
    pub category: ObjectiveCategory,
    pub obj: Vec<C>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ModelFileFormat {
    LP,
}

impl Display for ModelFileFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelFileFormat::LP => write!(f, "lp"),
        }
    }
}

pub(super) trait BasicModelView<'a> {
    type SolutionValueType: Clone;
    type CoefficientValueType: Clone;
    type ConstraintCellType: ConstraintCell<CoefficientValueType = Self::CoefficientValueType>;

    fn variables(&self) -> &[Variable<'a, Self::SolutionValueType>];
    fn constraints(&self) -> &Constraint<'a, Self::ConstraintCellType>;
    fn name(&self) -> &str;

    fn contains_continuous(&self) -> bool {
        self.variables()
            .iter()
            .any(|v| v.variable_type.get().is_continuous())
    }

    fn contains_binary(&self) -> bool {
        self.variables().iter().any(|v| v.variable_type.get().is_binary())
    }

    fn contains_integer(&self) -> bool {
        self.variables()
            .iter()
            .any(|v| v.variable_type.get().is_integer())
    }

    fn contains_not_binary_integer(&self) -> bool {
        self.variables()
            .iter()
            .any(|v| v.variable_type.get().is_not_binary_integer())
    }

    fn export_lp(&self, writer: &dyn Write) -> std::io::Result<()>;
}

pub(super) trait ModelView<'a>: BasicModelView<'a> {
    type ObjectiveCellType: ModelCell<CoefficientValueType = Self::CoefficientValueType>;

    fn objective(&self) -> &Objective<Self::ObjectiveCellType>;

    fn export(&self, format: ModelFileFormat) -> std::io::Result<()> {
        self.export_to_file(Path::new("."), format)
    }

    fn export_to_file(&self, path: &Path, format: ModelFileFormat) -> std::io::Result<()> {
        let path = if (path.is_dir()) {
            path.join(format!("{}.{}", self.name(), format)).as_path()
        } else {
            path
        };
        let file = if (path.exists()) {
            File::open(path)?
        } else {
            File::create_new(path)?
        };
        let mut writer = std::io::BufWriter::new(file);
        self.export_to(&writer, format)?;
        Ok(())
    }

    fn export_to(&self, writer: &mut dyn Write, format: ModelFileFormat) -> std::io::Result<()> {
        match format {
            ModelFileFormat::LP => self.export_lp(&writer)?,
        }
        Ok(())
    }
}

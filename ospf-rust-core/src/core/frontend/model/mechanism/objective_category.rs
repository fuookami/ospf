use std::fmt::{Debug, Display, Formatter};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ObjectiveCategory {
    Maximum,
    Minimum,
}

impl Debug for ObjectiveCategory {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            ObjectiveCategory::Maximum => write!(f, "Maximum"),
            ObjectiveCategory::Minimum => write!(f, "Minimum"),
        }
    }
}

impl Display for ObjectiveCategory {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            ObjectiveCategory::Maximum => write!(f, "Maximum"),
            ObjectiveCategory::Minimum => write!(f, "Minimum"),
        }
    }
}

impl ObjectiveCategory {
    pub fn reverse(&self) -> ObjectiveCategory {
        match self {
            ObjectiveCategory::Maximum => ObjectiveCategory::Minimum,
            ObjectiveCategory::Minimum => ObjectiveCategory::Maximum,
        }
    }
}

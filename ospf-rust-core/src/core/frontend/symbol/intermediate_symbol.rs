use ospf_rust_math::{Category, Symbol};

pub struct IntermediateSymbolIdentifier {
    name: String
}

pub trait IntermediateSymbol<const category: Category> : Symbol {
    type Identifier = IntermediateSymbolIdentifier;
}

pub trait LinearIntermediateSymbol : IntermediateSymbol<{ Category::Linear }> {}

pub trait QuadraticIntermediateSymbol : IntermediateSymbol<{ Category::Quadratic }> {}

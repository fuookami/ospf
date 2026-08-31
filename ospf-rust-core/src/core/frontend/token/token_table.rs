use crate::core::frontend::token::{AbstractTokenList, TokenValueType};

pub trait AbstractTokenTable<T: TokenValueType> {
    type List: AbstractTokenList<T>;

    fn token_list(&self) -> &Self::List;
}

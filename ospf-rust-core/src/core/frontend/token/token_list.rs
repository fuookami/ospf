use super::token::{Token, TokenValueType, VariableItemWrapper};
use crate::core::frontend::variable::item::{VariableItem, VariableKey};
use bimap::BiMap;
use ospf_rust_base::{Error, ErrorCode, RuntimeError, OK};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::{Deref, Index};
use std::sync::Mutex;
use typed_arena::Arena;

pub struct TokenExistError {
    name: String,
}

impl Error for TokenExistError {
    fn msg(&self) -> &str {
        &self.name
    }
}

impl RuntimeError for TokenExistError {
    fn code(&self) -> ErrorCode {
        ErrorCode::TokenExisted
    }
}

fn token_index_map<T: TokenValueType, It: Iterator<Item = *const Token<T>>>(
    tokens: It,
) -> BiMap<*const Token<T>, usize> {
    unsafe {
        let mut map = BiMap::new();
        let mut sorted_tokens = tokens.collect::<Vec<_>>();
        sorted_tokens.sort_by(|a, b| {
            let lhs = (**a).solver_index;
            let rhs = (**b).solver_index;
            return lhs.cmp(&rhs);
        });
        for (i, token) in sorted_tokens.iter().enumerate() {
            map.insert(*token, i);
        }
        map
    }
}

pub trait AbstractTokenList<T: TokenValueType>: Index<usize, Output = Token<T>> {
    fn tokens<'a>(&'a self) -> impl Iterator<Item = &'a Token<T>>
    where
        T: 'a;

    fn cached_solution(&self) -> bool {
        self.tokens().all(|t| t.result().is_some())
    }

    fn index_of(&self, token: &Token<T>) -> usize;
    fn index_of_variable(&self, variable: VariableItemWrapper) -> Option<usize>;
    fn index_of_var<I: VariableItem>(&self, variable: &I) -> Option<usize>
    where
        VariableItemWrapper: for<'a> From<&'a I>,
    {
        self.index_of_variable(variable.into())
    }

    fn find(&self, index: usize) -> Option<&Token<T>>;
    fn find_variable(&self, variable: VariableItemWrapper) -> Option<&Token<T>>;
    fn find_var<I: VariableItem>(&self, variable: &I) -> Option<&Token<T>>
    where
        VariableItemWrapper: for<'a> From<&'a I>,
    {
        self.find_variable(variable.into())
    }

    fn set_solution<It: Iterator<Item = T>>(&self, solution: It);
    fn set_solution_map<It: Iterator<Item = (VariableItemWrapper, T)>>(&self, solution: It);
    fn clear_solution(&self);
}

pub struct TokenList<T: TokenValueType> {
    list: HashMap<VariableKey, *const Token<T>>,
    token_index_map: BiMap<*const Token<T>, usize>,
}

impl<T: TokenValueType> Index<usize> for TokenList<T> {
    type Output = Token<T>;

    fn index(&self, index: usize) -> &Token<T> {
        if let Some(token) = self.find(index) {
            token
        } else {
            unsafe {
                self.list
                    .values()
                    .find(|token| (***token).solver_index == index)
                    .unwrap()
                    .as_ref()
                    .unwrap()
            }
        }
    }
}

impl<T: TokenValueType> From<&MutableTokenList<T>> for TokenList<T> {
    fn from(list: &MutableTokenList<T>) -> Self {
        let list = list
            .list
            .borrow()
            .iter()
            .map(|(k, v)| (*k, *v))
            .collect::<HashMap<_, _>>();
        let token_index_map = token_index_map(list.values().into_iter().map(|v| *v));
        Self {
            list,
            token_index_map,
        }
    }
}

impl<T: TokenValueType> From<&AutoTokenList<T>> for TokenList<T> {
    fn from(list: &AutoTokenList<T>) -> Self {
        Self::from(&list.inner)
    }
}

impl<T: TokenValueType> From<&ManualTokenList<T>> for TokenList<T> {
    fn from(list: &ManualTokenList<T>) -> Self {
        Self::from(&list.inner)
    }
}

impl<T: TokenValueType> AbstractTokenList<T> for TokenList<T> {
    fn tokens<'a>(&'a self) -> impl Iterator<Item = &'a Token<T>>
    where
        T: 'a,
    {
        unsafe {
            self.list
                .values()
                .into_iter()
                .map(|token| token.as_ref().unwrap())
        }
    }

    fn index_of(&self, token: &Token<T>) -> usize {
        *self
            .token_index_map
            .get_by_left(&(token as *const _))
            .unwrap()
    }

    fn index_of_variable(&self, variable: VariableItemWrapper) -> Option<usize> {
        let token = self.find_variable(variable)?;
        Some(self.index_of(token))
    }

    fn find(&self, index: usize) -> Option<&Token<T>> {
        let token = self.token_index_map.get_by_right(&index)?;
        unsafe { Some(&**token) }
    }

    fn find_variable(&self, variable: VariableItemWrapper) -> Option<&Token<T>> {
        unsafe { self.list.get(&variable.key()).unwrap().as_ref() }
    }

    fn set_solution<It: Iterator<Item = T>>(&self, solution: It) {
        solution.enumerate().for_each(|(i, value)| {
            let token = self.token_index_map.get_by_right(&i).unwrap();
            unsafe { (**token).result.set(Some(value)) }
        });
    }

    fn set_solution_map<It: for<'a> Iterator<Item = (VariableItemWrapper, T)>>(
        &self,
        solution: It,
    ) {
        solution.for_each(|(variable, value)| {
            if let Some(token) = self.list.get(&variable.key()) {
                unsafe { (**token).result.set(Some(value)) }
            }
        });
    }

    fn clear_solution(&self) {
        unsafe {
            self.list
                .values()
                .for_each(|token| (**token).result.set(None));
        }
    }
}

pub trait AbstractMutableTokenList<T: TokenValueType>: AbstractTokenList<T> {
    fn add_var<I: VariableItem>(&self, var: &I) -> Result<&Token<T>, TokenExistError>
    where
        VariableItemWrapper: for<'a> From<&'a I>,
    {
        self.add_variable(var.into())
    }

    fn add_variable(&self, var: VariableItemWrapper) -> Result<&Token<T>, TokenExistError>;

    fn add_vars<'a, I: VariableItem + 'a, It: Iterator<Item = &'a I>>(
        &self,
        vars: It,
    ) -> Result<(), TokenExistError>
    where
        VariableItemWrapper: From<&'a I>,
    {
        for var in vars {
            self.add_variable(var.into())?;
        }
        OK.into()
    }

    fn remove<I: VariableItem>(&self, var: &I)
    where
        VariableItemWrapper: for<'a> From<&'a I>;
}

struct MutableTokenList<T: TokenValueType> {
    list: RefCell<HashMap<VariableKey, *const Token<T>>>,
    current_index: Mutex<usize>,
    token_index_map: RefCell<BiMap<*const Token<T>, usize>>,
    alloc: Arena<Token<T>>,
}

impl<T: TokenValueType> Index<usize> for MutableTokenList<T> {
    type Output = Token<T>;

    fn index(&self, index: usize) -> &Token<T> {
        unsafe {
            if let Some(token) = self.token_index_map.borrow().get_by_right(&index) {
                token.as_ref().unwrap()
            } else {
                self.list
                    .borrow()
                    .values()
                    .find(|token| (***token).solver_index == index)
                    .unwrap()
                    .as_ref()
                    .unwrap()
            }
        }
    }
}

impl<T: TokenValueType> AbstractTokenList<T> for MutableTokenList<T> {
    fn tokens<'a>(&'a self) -> impl Iterator<Item = &'a Token<T>>
    where
        T: 'a,
    {
        unsafe {
            let binding = self.list.as_ptr().as_ref().unwrap();
            binding
                .values()
                .into_iter()
                .map(|token| token.as_ref().unwrap())
        }
    }

    fn index_of(&self, token: &Token<T>) -> usize {
        *self
            .token_index_map
            .borrow()
            .get_by_left(&(token as *const _))
            .unwrap()
    }

    fn index_of_variable(&self, variable: VariableItemWrapper) -> Option<usize> {
        let token = self.find_variable(variable)?;
        Some(self.index_of(token))
    }

    fn find(&self, index: usize) -> Option<&Token<T>> {
        unsafe {
            let binding = self.token_index_map.borrow();
            let token = binding.get_by_right(&index)?;
            Some(&**token)
        }
    }

    fn find_variable(&self, variable: VariableItemWrapper) -> Option<&Token<T>> {
        todo!("not implemented")
    }

    fn set_solution<It: Iterator<Item = T>>(&self, solution: It) {
        unsafe {
            solution.enumerate().for_each(|(i, value)| {
                let binding = self.token_index_map.borrow();
                let token = binding.get_by_right(&i).unwrap();
                (**token).result.set(Some(value))
            });
        }
    }

    fn set_solution_map<It: Iterator<Item = (VariableItemWrapper, T)>>(&self, solution: It) {
        unsafe {
            solution.for_each(|(variable, value)| {
                if let Some(token) = self.list.borrow().get(&variable.key()) {
                    (**token).result.set(Some(value))
                }
            })
        };
    }

    fn clear_solution(&self) {
        unsafe {
            self.list
                .borrow()
                .values()
                .for_each(|token| (**token).result.set(None));
        }
    }
}

impl<T: TokenValueType> AbstractMutableTokenList<T> for MutableTokenList<T> {
    fn add_variable(&self, var: VariableItemWrapper) -> Result<&Token<T>, TokenExistError> {
        let mut index = self.current_index.lock().unwrap();
        if self.list.borrow().contains_key(&var.key()) {
            return Err(TokenExistError {
                name: var.name().to_string(),
            });
        }
        let key = var.key();
        let token = self.alloc.alloc(Token::new(var, *index));
        self.token_index_map
            .borrow_mut()
            .insert(token as *const _, *index);
        *index += 1;
        self.list.borrow_mut().insert(key, token as *const _);
        Ok(token)
    }

    fn remove<I: VariableItem>(&self, var: &I)
    where
        VariableItemWrapper: for<'a> From<&'a I>,
    {
        if let Some(token) = self.list.borrow_mut().remove(&var.key()) {
            self.token_index_map.borrow_mut().remove_by_left(&token);
        }
    }
}

impl<T: TokenValueType> MutableTokenList<T> {
    pub fn new() -> Self {
        Self {
            list: RefCell::new(HashMap::new()),
            current_index: Mutex::new(0),
            token_index_map: RefCell::new(BiMap::new()),
            alloc: Arena::new(),
        }
    }
}

struct AutoTokenList<T: TokenValueType> {
    inner: MutableTokenList<T>,
}

impl<T: TokenValueType> Deref for AutoTokenList<T> {
    type Target = MutableTokenList<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: TokenValueType> Index<usize> for AutoTokenList<T> {
    type Output = Token<T>;

    fn index(&self, index: usize) -> &Self::Output {
        self.inner.index(index)
    }
}

impl<T: TokenValueType> AbstractTokenList<T> for AutoTokenList<T> {
    fn tokens<'a>(&'a self) -> impl Iterator<Item = &'a Token<T>>
    where
        T: 'a,
    {
        self.inner.tokens()
    }

    fn index_of(&self, token: &Token<T>) -> usize {
        self.inner.index_of(token)
    }

    fn index_of_variable(&self, variable: VariableItemWrapper) -> Option<usize> {
        self.inner.index_of_variable(variable)
    }

    fn find(&self, index: usize) -> Option<&Token<T>> {
        self.inner.find(index)
    }

    fn find_variable(&self, variable: VariableItemWrapper) -> Option<&Token<T>> {
        self.inner.find_variable(variable)
    }

    fn set_solution<It: Iterator<Item = T>>(&self, solution: It) {
        self.inner.set_solution(solution);
    }

    fn set_solution_map<It: Iterator<Item = (VariableItemWrapper, T)>>(&self, solution: It) {
        self.inner.set_solution_map(solution);
    }

    fn clear_solution(&self) {
        self.inner.clear_solution();
    }
}

impl<T: TokenValueType> AbstractMutableTokenList<T> for AutoTokenList<T> {
    fn add_var<I: VariableItem>(&self, var: &I) -> Result<&Token<T>, TokenExistError>
    where
        VariableItemWrapper: for<'a> From<&'a I>,
    {
        self.inner.add_var(var)
    }

    fn add_variable(&self, var: VariableItemWrapper) -> Result<&Token<T>, TokenExistError> {
        self.inner.add_variable(var)
    }

    fn add_vars<'a, I: VariableItem + 'a, It: Iterator<Item = &'a I>>(
        &self,
        vars: It,
    ) -> Result<(), TokenExistError>
    where
        VariableItemWrapper: From<&'a I>,
    {
        self.inner.add_vars(vars)
    }

    fn remove<I: VariableItem>(&self, var: &I)
    where
        VariableItemWrapper: for<'a> From<&'a I>,
    {
        self.inner.remove(var)
    }
}

impl<T: TokenValueType> AutoTokenList<T> {
    pub fn new() -> Self {
        Self {
            inner: MutableTokenList::new(),
        }
    }
}

struct ManualTokenList<T: TokenValueType> {
    inner: MutableTokenList<T>,
}

impl<T: TokenValueType> Deref for ManualTokenList<T> {
    type Target = MutableTokenList<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: TokenValueType> Index<usize> for ManualTokenList<T> {
    type Output = Token<T>;

    fn index(&self, index: usize) -> &Self::Output {
        self.inner.index(index)
    }
}

impl<T: TokenValueType> AbstractTokenList<T> for ManualTokenList<T> {
    fn tokens<'a>(&'a self) -> impl Iterator<Item = &'a Token<T>>
    where
        T: 'a,
    {
        self.inner.tokens()
    }

    fn index_of(&self, token: &Token<T>) -> usize {
        self.inner.index_of(token)
    }

    fn index_of_variable(&self, variable: VariableItemWrapper) -> Option<usize> {
        self.inner.index_of_variable(variable)
    }

    fn find(&self, index: usize) -> Option<&Token<T>> {
        self.inner.find(index)
    }

    fn find_variable(&self, variable: VariableItemWrapper) -> Option<&Token<T>> {
        let token = self.inner.find_variable(variable.clone());
        if let Some(token) = token {
            Some(token)
        } else {
            if let Ok(token) = self.add_variable(variable) {
                Some(token)
            } else {
                None
            }
        }
    }

    fn set_solution<It: Iterator<Item = T>>(&self, solution: It) {
        self.inner.set_solution(solution);
    }

    fn set_solution_map<It: Iterator<Item = (VariableItemWrapper, T)>>(&self, solution: It) {
        self.inner.set_solution_map(solution);
    }

    fn clear_solution(&self) {
        self.inner.clear_solution();
    }
}

impl<T: TokenValueType> AbstractMutableTokenList<T> for ManualTokenList<T> {
    fn add_var<I: VariableItem>(&self, var: &I) -> Result<&Token<T>, TokenExistError>
    where
        VariableItemWrapper: for<'a> From<&'a I>,
    {
        self.inner.add_var(var)
    }

    fn add_variable(&self, var: VariableItemWrapper) -> Result<&Token<T>, TokenExistError> {
        self.inner.add_variable(var)
    }

    fn add_vars<'a, I: VariableItem + 'a, It: Iterator<Item = &'a I>>(
        &self,
        vars: It,
    ) -> Result<(), TokenExistError>
    where
        VariableItemWrapper: From<&'a I>,
    {
        self.inner.add_vars(vars)
    }

    fn remove<I: VariableItem>(&self, var: &I)
    where
        VariableItemWrapper: for<'a> From<&'a I>,
    {
        self.inner.remove(var)
    }
}

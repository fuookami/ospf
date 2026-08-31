//! 符号值提供者适配
//! Symbol value provider adapters

use std::collections::HashMap;
use crate::symbol::{Linear, LinearMonomial, OwnedSymbol, Quadratic, QuadraticMonomial};

/// 缺失值处理策略 / Missing value handling policy
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MissingValuePolicy {
    /// 返回 `None` / Return `None`
    ReturnNone,
    /// 缺失值视为零 / Treat missing values as zero
    AsZero,
    /// 缺失值返回错误 / Return an error for missing values
    Fail,
}

/// 值提供者错误 / Value provider error
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValueProviderError {
    /// 符号缺少绑定值 / Symbol value is missing
    MissingValue(OwnedSymbol),
}

/// 符号值提供者 / Symbol value provider
pub trait ValueProvider<T> {
    /// 获取符号绑定值 / Get the bound value for a symbol
    fn get(&self, symbol: &OwnedSymbol) -> Option<T>;
}

impl<T, F> ValueProvider<T> for F
where
    F: Fn(&OwnedSymbol) -> Option<T>,
{
    fn get(&self, symbol: &OwnedSymbol) -> Option<T> {
        self(symbol)
    }
}

impl<T: Clone> ValueProvider<T> for HashMap<OwnedSymbol, T> {
    fn get(&self, symbol: &OwnedSymbol) -> Option<T> {
        HashMap::get(self, symbol).cloned()
    }
}

impl<T: Clone> ValueProvider<T> for &HashMap<OwnedSymbol, T> {
    fn get(&self, symbol: &OwnedSymbol) -> Option<T> {
        HashMap::get(*self, symbol).cloned()
    }
}

/// 基于 `HashMap` 的值提供者 / `HashMap` backed value provider
#[derive(Clone, Debug, PartialEq)]
pub struct MapValueProvider<T> {
    values: HashMap<OwnedSymbol, T>,
}

impl<T> MapValueProvider<T> {
    /// 创建 map 值提供者 / Create a map value provider
    pub fn new(values: HashMap<OwnedSymbol, T>) -> Self {
        Self { values }
    }

    /// 访问内部映射 / Access the inner map
    pub fn values(&self) -> &HashMap<OwnedSymbol, T> {
        &self.values
    }

    /// 消费并返回内部映射 / Consume and return the inner map
    pub fn into_values(self) -> HashMap<OwnedSymbol, T> {
        self.values
    }
}

impl<T: Clone> ValueProvider<T> for MapValueProvider<T> {
    fn get(&self, symbol: &OwnedSymbol) -> Option<T> {
        self.values.get(symbol).cloned()
    }
}

impl<T: Clone> ValueProvider<T> for &MapValueProvider<T> {
    fn get(&self, symbol: &OwnedSymbol) -> Option<T> {
        self.values.get(symbol).cloned()
    }
}

/// 按缺失值策略获取值 / Get a value according to missing value policy
pub fn value_or_policy<P, T>(
    provider: &P,
    symbol: &OwnedSymbol,
    policy: MissingValuePolicy,
) -> Result<Option<T>, ValueProviderError>
where
    P: ValueProvider<T> + ?Sized,
    T: num_traits::Zero,
{
    match provider.get(symbol) {
        Some(value) => Ok(Some(value)),
        None => match policy {
            MissingValuePolicy::ReturnNone => Ok(None),
            MissingValuePolicy::AsZero => Ok(Some(T::zero())),
            MissingValuePolicy::Fail => Err(ValueProviderError::MissingValue(symbol.clone())),
        },
    }
}

/// 使用值提供者求值 / Evaluate with a value provider
pub trait EvaluateWithProvider<T> {
    /// 按缺失值策略求值 / Evaluate according to missing value policy
    fn evaluate_with_provider<P>(
        &self,
        provider: &P,
        policy: MissingValuePolicy,
    ) -> Result<Option<T>, ValueProviderError>
    where
        P: ValueProvider<T> + ?Sized,
        T: super::Evaluatable + Clone;
}

fn insert_value_from_provider<P, T>(
    values: &mut HashMap<OwnedSymbol, T>,
    provider: &P,
    symbol: &OwnedSymbol,
    policy: MissingValuePolicy,
) -> Result<bool, ValueProviderError>
where
    P: ValueProvider<T> + ?Sized,
    T: num_traits::Zero,
{
    if values.contains_key(symbol) {
        return Ok(true);
    }

    match value_or_policy(provider, symbol, policy)? {
        Some(value) => {
            values.insert(symbol.clone(), value);
            Ok(true)
        }
        None => Ok(false),
    }
}

impl<T> EvaluateWithProvider<T> for LinearMonomial<T> {
    fn evaluate_with_provider<P>(
        &self,
        provider: &P,
        policy: MissingValuePolicy,
    ) -> Result<Option<T>, ValueProviderError>
    where
        P: ValueProvider<T> + ?Sized,
        T: super::Evaluatable + Clone,
    {
        let mut values = HashMap::new();
        if !insert_value_from_provider(&mut values, provider, &self.symbol, policy)? {
            return Ok(None);
        }
        Ok(Some(super::Evaluate::evaluate(self, &values)))
    }
}

impl<T> EvaluateWithProvider<T> for QuadraticMonomial<T> {
    fn evaluate_with_provider<P>(
        &self,
        provider: &P,
        policy: MissingValuePolicy,
    ) -> Result<Option<T>, ValueProviderError>
    where
        P: ValueProvider<T> + ?Sized,
        T: super::Evaluatable + Clone,
    {
        let mut values = HashMap::new();
        if !insert_value_from_provider(&mut values, provider, &self.symbol1, policy)? {
            return Ok(None);
        }
        if let Some(symbol2) = &self.symbol2 {
            if !insert_value_from_provider(&mut values, provider, symbol2, policy)? {
                return Ok(None);
            }
        }
        Ok(Some(super::Evaluate::evaluate(self, &values)))
    }
}

impl<T> EvaluateWithProvider<T> for Linear<T> {
    fn evaluate_with_provider<P>(
        &self,
        provider: &P,
        policy: MissingValuePolicy,
    ) -> Result<Option<T>, ValueProviderError>
    where
        P: ValueProvider<T> + ?Sized,
        T: super::Evaluatable + Clone,
    {
        let mut values = HashMap::new();
        for monomial in &self.monomials {
            if !insert_value_from_provider(&mut values, provider, &monomial.symbol, policy)? {
                return Ok(None);
            }
        }
        Ok(Some(super::Evaluate::evaluate(self, &values)))
    }
}

impl<T> EvaluateWithProvider<T> for Quadratic<T> {
    fn evaluate_with_provider<P>(
        &self,
        provider: &P,
        policy: MissingValuePolicy,
    ) -> Result<Option<T>, ValueProviderError>
    where
        P: ValueProvider<T> + ?Sized,
        T: super::Evaluatable + Clone,
    {
        let mut values = HashMap::new();
        for monomial in &self.monomials {
            if !insert_value_from_provider(&mut values, provider, &monomial.symbol1, policy)? {
                return Ok(None);
            }
            if let Some(symbol2) = &monomial.symbol2 {
                if !insert_value_from_provider(&mut values, provider, symbol2, policy)? {
                    return Ok(None);
                }
            }
        }
        Ok(Some(super::Evaluate::evaluate(self, &values)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol::{OwnedSymbol, test_utils::SimpleSymbol};

    fn make_symbol(id: usize, name: &str) -> OwnedSymbol {
        OwnedSymbol::new(SimpleSymbol::with_id(id, name))
    }

    #[test]
    fn map_value_provider_returns_values() {
        let x = make_symbol(1, "x");
        let provider = MapValueProvider::new(HashMap::from([(x.clone(), 2.0)]));

        assert_eq!(provider.get(&x), Some(2.0));
    }

    #[test]
    fn missing_value_policy_handles_missing_symbols() {
        let x = make_symbol(1, "x");
        let provider = MapValueProvider::<f64>::new(HashMap::new());

        assert_eq!(
            value_or_policy(&provider, &x, MissingValuePolicy::ReturnNone),
            Ok(None)
        );
        assert_eq!(
            value_or_policy(&provider, &x, MissingValuePolicy::AsZero),
            Ok(Some(0.0))
        );
        assert_eq!(
            value_or_policy(&provider, &x, MissingValuePolicy::Fail),
            Err(ValueProviderError::MissingValue(x))
        );
    }

    #[test]
    fn evaluate_with_provider_respects_policy() {
        let x = make_symbol(1, "x");
        let y = make_symbol(2, "y");
        let polynomial = Linear::new(
            vec![
                LinearMonomial::new(2.0, x.clone()),
                LinearMonomial::new(3.0, y.clone()),
            ],
            1.0,
        );
        let provider = MapValueProvider::new(HashMap::from([(x, 4.0)]));

        assert_eq!(
            polynomial.evaluate_with_provider(&provider, MissingValuePolicy::ReturnNone),
            Ok(None)
        );
        assert_eq!(
            polynomial.evaluate_with_provider(&provider, MissingValuePolicy::AsZero),
            Ok(Some(9.0))
        );
    }
}

//! 符号表达式分类
//! Symbolic expression category

use crate::operator::Exponent;
use crate::symbol::{
    Canonical, CanonicalMonomial, Linear, LinearMonomial, Quadratic, QuadraticMonomial,
};

/// 表达式按多项式能力分类。
/// Classifies expressions by polynomial capability.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u64)]
pub enum Category {
    /// 线性表达式 / Linear expression
    Linear = 1,
    /// 二次表达式 / Quadratic expression
    Quadratic = 2,
    /// 标准多项式表达式 / Standard polynomial expression
    Standard = 3,
    /// 非线性表达式 / Nonlinear expression
    Nonlinear = 10,
}

impl Category {
    /// 获取分类代码 / Get category code
    pub const fn code(self) -> u64 {
        self as u64
    }

    /// 返回两个分类中较高的一个 / Return the higher category
    pub fn op(self, rhs: Self) -> Self {
        self.max(rhs)
    }

    /// 返回两个分类中较高的一个 / Return the higher category
    pub fn max(self, rhs: Self) -> Self {
        if self.code() < rhs.code() { rhs } else { self }
    }
}

/// 返回两个分类中较高的一个 / Return the higher category
pub fn max_category(lhs: Category, rhs: Category) -> Category {
    lhs.max(rhs)
}

/// 返回集合中最高的分类 / Return the highest category in a collection
pub fn max_category_in<I>(categories: I) -> Category
where
    I: IntoIterator<Item = Category>,
{
    max_category_or_none(categories).expect("category collection must not be empty")
}

/// 返回集合中最高的分类，如果集合为空则返回 `None`
/// Return the highest category in a collection, or `None` if empty
pub fn max_category_or_none<I>(categories: I) -> Option<Category>
where
    I: IntoIterator<Item = Category>,
{
    categories
        .into_iter()
        .max_by_key(|category| category.code())
}

/// 可分类的符号表达式 / Categorized symbolic expression
pub trait Categorized {
    /// 获取表达式分类 / Get expression category
    fn category(&self) -> Category;
}

impl<T> Categorized for LinearMonomial<T> {
    fn category(&self) -> Category {
        Category::Linear
    }
}

impl<T> Categorized for Linear<T> {
    fn category(&self) -> Category {
        Category::Linear
    }
}

impl<T> Categorized for QuadraticMonomial<T> {
    fn category(&self) -> Category {
        if self.is_quadratic() {
            Category::Quadratic
        } else {
            Category::Linear
        }
    }
}

impl<T> Categorized for Quadratic<T> {
    fn category(&self) -> Category {
        if self.monomials.iter().any(QuadraticMonomial::is_quadratic) {
            Category::Quadratic
        } else {
            Category::Linear
        }
    }
}

impl<T, E> Categorized for CanonicalMonomial<T, E>
where
    E: Exponent + Copy + Ord + num_traits::Zero + num_traits::One + std::iter::Sum,
{
    fn category(&self) -> Category {
        let one = E::one();
        let two = one + one;
        let degree = self.total_degree();
        if degree <= one {
            Category::Linear
        } else if degree <= two {
            Category::Quadratic
        } else {
            Category::Standard
        }
    }
}

impl<T, E> Categorized for Canonical<T, E>
where
    E: Exponent + Copy + Ord + num_traits::Zero + num_traits::One + std::iter::Sum,
{
    fn category(&self) -> Category {
        self.monomials
            .iter()
            .map(Categorized::category)
            .max_by_key(|category| category.code())
            .unwrap_or(Category::Linear)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol::{OwnedSymbol, test_utils::SimpleSymbol};
    use std::collections::HashMap;

    fn make_symbol(id: usize, name: &str) -> OwnedSymbol {
        OwnedSymbol::new(SimpleSymbol::with_id(id, name))
    }

    #[test]
    fn category_order_matches_kotlin_codes() {
        assert_eq!(Category::Linear.code(), 1);
        assert_eq!(Category::Quadratic.code(), 2);
        assert_eq!(Category::Standard.code(), 3);
        assert_eq!(Category::Nonlinear.code(), 10);
        assert_eq!(Category::Linear.op(Category::Standard), Category::Standard);
    }

    #[test]
    fn max_category_handles_collections() {
        assert_eq!(
            max_category_in([Category::Linear, Category::Quadratic, Category::Standard]),
            Category::Standard
        );
        assert_eq!(max_category_or_none([]), None);
    }

    #[test]
    fn polynomial_categories_follow_degree() {
        let x = make_symbol(1, "x");
        let y = make_symbol(2, "y");
        let linear = Linear::new(vec![LinearMonomial::new(2.0, x.clone())], 1.0);
        let quadratic = Quadratic::new(
            vec![QuadraticMonomial::quadratic(3.0, x.clone(), y.clone())],
            0.0,
        );
        let mut powers = HashMap::new();
        powers.insert(x, 3_i32);
        let canonical = Canonical::new(vec![CanonicalMonomial::new(1.0, powers)], 0.0);

        assert_eq!(linear.category(), Category::Linear);
        assert_eq!(quadratic.category(), Category::Quadratic);
        assert_eq!(canonical.category(), Category::Standard);
    }
}

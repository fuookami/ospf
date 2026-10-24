//! 定价策略 / Pricing policies.

use super::model::EspprcLabel;
use crate::domain::vrp::Route;
use ospf_rust_core::solver::value::SolveValue;
use ospf_rust_quantities::unit::UnitConversionValue;

/// 标签支配策略 / Label-dominance policy.
pub trait LabelDominancePolicy: Send + Sync {
    /// 认证保留最优路径及主问题系数；未认证策略不会用于证明性剪枝。 / Certify preservation of optimal paths and master coefficients; uncertified policies are not used for proof pruning.
    fn is_certified(&self) -> bool { false }

    /// 判断标签 `a` 是否支配标签 `b` / Check whether label `a` dominates `b`.
    fn dominates(&self, a: &EspprcLabel, b: &EspprcLabel) -> bool;
}

/// 默认三资源与 bitset 集合支配 / Default three-resource and bitset-set dominance.
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultLabelDominancePolicy;

impl LabelDominancePolicy for DefaultLabelDominancePolicy {
    fn is_certified(&self) -> bool { true }

    fn dominates(&self, a: &EspprcLabel, b: &EspprcLabel) -> bool {
        a.current_node == b.current_node
            && a.reduced_cost <= b.reduced_cost
            && a.time <= b.time
            && a.load <= b.load
            && a.visited.is_subset_of(&b.visited)
            // 只有限制更少的标签才能支配限制更多的标签。
            // Only a label with fewer forbidden customers may dominate a more restricted label.
            && a.forbidden.is_subset_of(&b.forbidden)
            && (a.reduced_cost < b.reduced_cost || a.time < b.time || a.load < b.load)
    }
}

/// 负列选择策略 / Negative-column selection policy.
pub trait PricingColumnSelector<V>: Send + Sync
where
    V: SolveValue + UnitConversionValue,
{
    /// 选择加入 RMP 的路线 / Select routes to add to the RMP.
    fn select(&self, routes: &[Route<V>], max_columns: usize) -> Vec<Route<V>>;
}

/// 保留定价入口的 reduced-cost 排序，仅截取输出 / Preserve the pricer's reduced-cost ranking and truncate only output.
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultPricingColumnSelector;

impl<V> PricingColumnSelector<V> for DefaultPricingColumnSelector
where
    V: SolveValue + UnitConversionValue,
{
    fn select(&self, routes: &[Route<V>], max_columns: usize) -> Vec<Route<V>> {
        routes.iter().take(max_columns).cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{DefaultLabelDominancePolicy, LabelDominancePolicy};
    use crate::domain::route_generation::{EspprcLabel, ForbiddenCustomers, VisitedCustomers};

    #[test]
    fn a_more_restricted_label_cannot_dominate_a_less_restricted_label() {
        let allowed = EspprcLabel {
            reduced_cost: 1.0,
            time: 1.0,
            load: 1.0,
            current_node: 0,
            visited: VisitedCustomers::empty(1),
            forbidden: ForbiddenCustomers::empty(1),
            predecessor: None,
            predecessor_arc: None,
        };
        let restricted = EspprcLabel {
            reduced_cost: 2.0,
            time: 2.0,
            load: 2.0,
            current_node: 0,
            visited: VisitedCustomers::empty(1),
            forbidden: ForbiddenCustomers::empty(1).add(0),
            predecessor: None,
            predecessor_arc: None,
        };
        let policy = DefaultLabelDominancePolicy;
        assert!(policy.dominates(&allowed, &restricted));
        assert!(!policy.dominates(&restricted, &allowed));
    }
}

//! 列生成完成度与请求作用域；空列不是证明。 / Column-generation completion and request scope; an empty pool is not a proof.

use std::collections::BTreeSet;

/// 定价请求完成状态 / Pricing-request completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnGenerationCompletion {
    /// 完整搜索 / Exhaustive search.
    Exhausted,
    /// 计算预算耗尽 / Computational budget exhausted.
    BudgetLimit,
    /// 协作中断 / Cooperative interruption.
    Interrupted,
    /// 未认证启发式 / Uncertified heuristic.
    Heuristic,
}

/// 证据适用范围；标识一致不代替 LP 证书来源验证。 / Evidence scope; matching identities do not validate LP provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnGenerationScope {
    /// 本轮唯一请求 / Unique current request.
    pub request: String,
    /// 覆盖的业务组 / Covered business groups.
    pub groups: BTreeSet<String>,
    /// 模型版本 / Model revision.
    pub model: String,
    /// 已验证对偶版本 / Verified dual revision.
    pub dual: String,
    /// 分支与固定状态版本 / Branch and fixing revision.
    pub branch: String,
    /// 是否覆盖声明的完整业务域 / Whether the full declared domain is covered.
    pub global: bool,
    /// 执行器时隙身份 / Executor-slot identities.
    pub executor_slots: BTreeSet<String>,
}

impl ColumnGenerationScope {
    fn is_valid(&self) -> bool {
        [&self.request, &self.model, &self.dual, &self.branch].iter()
            .all(|identity| !identity.trim().is_empty())
            && self.groups.iter().chain(&self.executor_slots)
                .all(|identity| !identity.trim().is_empty())
    }
}

/// 单次或聚合定价结果，不自行证明整数最优。 / Individual or aggregate pricing result, not an integer-optimality proof.
#[derive(Debug, Clone)]
pub struct ColumnGenerationBatch<C> {
    /// 候选列 / Candidate columns.
    pub columns: Vec<C>,
    /// 真实完成状态 / Actual completion.
    pub completion: ColumnGenerationCompletion,
    /// 声明的证据域 / Declared evidence scope.
    pub scope: Option<ColumnGenerationScope>,
    /// 独立无改进证据，未知时为空 / Independent no-improvement evidence, absent when unknown.
    pub no_improving_columns: Option<bool>,
    /// 分组子请求 / Grouped child requests.
    pub children: Vec<ColumnGenerationBatch<C>>,
}

impl<C> ColumnGenerationBatch<C> {
    /// 包装无证据的旧生成器结果。 / Wrap legacy generator output without evidence.
    pub fn heuristic(columns: Vec<C>) -> Self {
        Self {
            columns,
            completion: ColumnGenerationCompletion::Heuristic,
            scope: None,
            no_improving_columns: None,
            children: Vec::new(),
        }
    }

    /// 验证完整作用域及两轴组合的子请求覆盖，不验证 LP 对偶来源。 / Check exhaustive scope and child coverage of axis combinations, not LP dual provenance.
    pub fn certifies(&self, expected: &ColumnGenerationScope) -> bool {
        if self.completion != ColumnGenerationCompletion::Exhausted
            || !expected.global || !expected.is_valid()
            || self.scope.as_ref() != Some(expected)
        {
            return false;
        }
        if self.children.is_empty() {
            return true;
        }
        let mut groups = BTreeSet::new();
        let mut slots = BTreeSet::new();
        let mut requests = BTreeSet::new();
        for child in &self.children {
            let Some(scope) = &child.scope else { return false; };
            if !requests.insert(&scope.request)
                || !(scope.request == expected.request
                    || scope.request.starts_with(&format!("{}:", expected.request)))
                || scope.model != expected.model || scope.dual != expected.dual
                || scope.branch != expected.branch || !child.certifies(scope)
            {
                return false;
            }
            groups.extend(scope.groups.iter().cloned());
            slots.extend(scope.executor_slots.iter().cloned());
        }
        groups == expected.groups && slots == expected.executor_slots
            // 分别覆盖两轴不足以覆盖任务-时隙的组合。 / Covering each axis separately does not cover their combinations.
            && expected.groups.iter().all(|group| expected.executor_slots.iter().all(|slot| {
                self.children.iter().any(|child| child.scope.as_ref()
                    .is_some_and(|scope| scope.groups.contains(group) && scope.executor_slots.contains(slot)))
            }))
    }

    /// 检查完整域无改进证据；空候选不自动成立。 / Check scoped no-improvement evidence; empty output alone is insufficient.
    pub fn certifies_no_improving_columns(&self, expected: &ColumnGenerationScope) -> bool {
        self.no_improving_columns == Some(true) && self.certifies(expected)
            && self.children.iter().all(|child| child.scope.as_ref()
                .is_some_and(|scope| child.certifies_no_improving_columns(scope)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope() -> ColumnGenerationScope {
        ColumnGenerationScope {
            request: "round-1".into(), groups: ["a".into(), "b".into()].into(),
            model: "model-1".into(), dual: "dual-1".into(), branch: "root".into(),
            global: true, executor_slots: ["a:0".into(), "b:0".into()].into(),
        }
    }

    fn exhausted(scope: ColumnGenerationScope) -> ColumnGenerationBatch<()> {
        ColumnGenerationBatch {
            columns: Vec::new(), completion: ColumnGenerationCompletion::Exhausted,
            scope: Some(scope), no_improving_columns: Some(true), children: Vec::new(),
        }
    }

    #[test]
    fn empty_legacy_batch_is_not_evidence() {
        assert!(!ColumnGenerationBatch::<()>::heuristic(Vec::new()).certifies(&scope()));
        let mut batch = exhausted(scope());
        batch.no_improving_columns = None;
        assert!(batch.certifies(&scope()));
        assert!(!batch.certifies_no_improving_columns(&scope()));
        for completion in [ColumnGenerationCompletion::BudgetLimit,
            ColumnGenerationCompletion::Interrupted, ColumnGenerationCompletion::Heuristic]
        {
            batch.completion = completion;
            assert!(!batch.certifies(&scope()));
        }
    }

    #[test]
    fn child_evidence_must_cover_the_current_domain() {
        let mut parent = exhausted(scope());
        for group in ["a", "b"] {
            let mut child = scope();
            child.request = format!("round-1:{group}");
            child.groups = [group.to_string()].into();
            child.executor_slots = scope().executor_slots;
            parent.children.push(exhausted(child));
        }
        assert!(parent.certifies_no_improving_columns(&scope()));
        parent.children[1].scope.as_mut().unwrap().dual = "stale".into();
        assert!(!parent.certifies(&scope()));
        parent.children.pop();
        assert!(!parent.certifies(&scope()));
    }

    #[test]
    fn independent_axis_coverage_does_not_certify_the_cartesian_domain() {
        let mut parent = exhausted(scope());
        for group in ["a", "b"] {
            let mut child = scope();
            child.request = format!("round-1:{group}");
            child.groups = [group.to_string()].into();
            child.executor_slots = [format!("{group}:0")].into();
            parent.children.push(exhausted(child));
        }
        assert!(!parent.certifies_no_improving_columns(&scope()));
    }
}

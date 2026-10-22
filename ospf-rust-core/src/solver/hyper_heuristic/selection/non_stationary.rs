//! 非平稳信用分配与派发时前驱 / Non-stationary credit assignment and dispatch-time predecessors

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use serde::{Deserialize, Serialize};
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::result::Result;
use crate::solver::heuristic::runner::fdlibm_log::log;
use crate::solver::heuristic::session::SearchRandom;
use crate::solver::hyper_heuristic::config::ComponentSpec;
use crate::solver::hyper_heuristic::invalid;
use super::{PolicySnapshot, SelectionContext, SelectionPolicy, SelectionTransition};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreditObservation { action: String, reward: f64 }

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreditDispatch { action: String, previous: Option<String> }

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CreditState {
    definition: Vec<f64>,
    window: VecDeque<CreditObservation>,
    quality: BTreeMap<String, f64>,
    pairs: BTreeMap<String, BTreeMap<String, f64>>,
    probabilities: BTreeMap<String, f64>,
    last_used: BTreeMap<String, u64>,
    previous: Option<String>,
    pending: BTreeMap<u64, CreditDispatch>,
    clock: u64,
}

/// 窗口秩 MAB、二阶选择函数或保底概率追逐 / Window-rank MAB, second-order choice function, or probability-floor pursuit
pub struct NonStationarySelector {
    kind: String,
    actions: Vec<String>,
    learning: bool,
    window_size: usize,
    exploration: f64,
    decay: f64,
    rate: f64,
    minimum_probability: f64,
    recency_weight: f64,
    revision: u64,
    state: CreditState,
}

impl NonStationarySelector {
    /// 校验显式定义并建立独立信用状态 / Validate the explicit definition and create independent credit state
    pub fn new(
        spec: &ComponentSpec,
        actions: Vec<String>,
        learning: bool,
    ) -> Result<Self> {
        spec.validate_parameters(&["window", "exploration", "decay", "rate", "minimumProbability", "recencyWeight"])?;
        let window_size = usize::try_from(spec.integer("window", 50)?).map_err(|_| invalid("selector window overflow"))?;
        let exploration = spec.number("exploration", 0.5)?;
        let decay = spec.number("decay", 0.8)?;
        let rate = spec.number("rate", 0.2)?;
        let minimum_probability = spec.number("minimumProbability", 0.01)?;
        let recency_weight = spec.number("recencyWeight", 0.1)?;
        if !Self::supports(&spec.r#type) || actions.is_empty() || actions.iter().any(|id| id.trim().is_empty())
            || actions.iter().collect::<BTreeSet<_>>().len() != actions.len()
            || !(1..=1_000_000).contains(&window_size) || exploration < 0.0 || !(0.0..=1.0).contains(&decay)
            || rate <= 0.0 || rate > 1.0 || minimum_probability < 0.0 || minimum_probability * actions.len() as f64 > 1.0
            || recency_weight < 0.0 || spec.r#type == "frr-mab" && window_size < actions.len() {
            return Err(invalid("invalid non-stationary selector parameters / 非平稳选择器参数无效"));
        }
        let state = CreditState {
            definition: vec![window_size as f64, exploration, decay, rate, minimum_probability, recency_weight],
            window: VecDeque::new(), quality: actions.iter().map(|id| (id.clone(), 0.0)).collect(), pairs: BTreeMap::new(),
            probabilities: actions.iter().map(|id| (id.clone(), 1.0 / actions.len() as f64)).collect(),
            last_used: actions.iter().map(|id| (id.clone(), 0)).collect(), previous: None, pending: BTreeMap::new(), clock: 0,
        };
        Ok(Self { kind: spec.r#type.clone(), actions, learning, window_size, exploration, decay, rate,
            minimum_probability, recency_weight, revision: 0, state })
    }

    pub(crate) fn supports(kind: &str) -> bool { ["frr-mab", "choice-function", "adaptive-pursuit"].contains(&kind) }

    fn mask(&self, available: &[String], empty_allowed: bool) -> Result<Vec<usize>> {
        if !empty_allowed && available.is_empty() || available.iter().collect::<BTreeSet<_>>().len() != available.len()
            || available.iter().any(|id| !self.state.quality.contains_key(id)) {
            return Err(invalid("invalid non-stationary action mask / 非平稳动作掩码无效"));
        }
        Ok(self.actions.iter().enumerate().filter_map(|(index, id)| available.contains(id).then_some(index)).collect())
    }

    fn maximum(&self, mask: &[usize], scores: &[f64]) -> Result<usize> {
        if mask.iter().any(|&index| !scores[index].is_finite()) { return Err(invalid("selector score overflow / 选择器分数溢出")); }
        Ok(mask.iter().copied().skip(1).fold(mask[0], |best, index| if scores[index] > scores[best] { index } else { best }))
    }

    fn frr(&self, mask: &[usize]) -> Result<usize> {
        let scale = self.state.window.iter().fold(0.0_f64, |value, observation| value.max(observation.reward.max(0.0)));
        let scale = if scale == 0.0 { 1.0 } else { scale };
        let positions: BTreeMap<_, _> = self.actions.iter().enumerate().map(|(index, id)| (id, index)).collect();
        let mut counts = vec![0; self.actions.len()];
        let mut totals = vec![0.0; self.actions.len()];
        for observation in &self.state.window {
            let index = positions[&observation.action];
            counts[index] += 1;
            totals[index] += observation.reward.max(0.0) / scale;
        }
        let mut ranking: Vec<_> = (0..self.actions.len()).collect();
        ranking.sort_by(|&a, &b| totals[b].partial_cmp(&totals[a]).unwrap_or(std::cmp::Ordering::Equal));
        let mut credits = vec![0.0; self.actions.len()];
        for (rank, &index) in ranking.iter().enumerate() {
            if totals[index] > 0.0 { credits[index] = libm::pow(self.decay, rank as f64) * (self.actions.len() - rank) as f64; }
        }
        if !self.learning { return self.maximum(mask, &credits); }
        if let Some(&index) = mask.iter().find(|&&index| counts[index] == 0) { return Ok(index); }
        let total = ranking.iter().map(|&index| credits[index]).sum::<f64>().max(1.0);
        let scores: Vec<_> = (0..self.actions.len()).map(|index| {
            credits[index] / total + self.exploration * libm::sqrt(2.0 * log(self.state.window.len().max(1) as f64 + 1.0) / counts[index] as f64)
        }).collect();
        self.maximum(mask, &scores)
    }

    fn choice(&self, mask: &[usize]) -> Result<usize> {
        let normalized = |values: &BTreeMap<String, f64>, action: &String| {
            let scale = values.values().fold(0.0_f64, |scale, value| scale.max(value.abs()));
            if scale == 0.0 { 0.0 } else { values.get(action).copied().unwrap_or(0.0) / scale }
        };
        let empty = BTreeMap::new();
        let pair = self.state.previous.as_ref().and_then(|id| self.state.pairs.get(id)).unwrap_or(&empty);
        let scores: Vec<_> = self.actions.iter().map(|id| normalized(&self.state.quality, id) + normalized(pair, id)
            + if self.learning { self.recency_weight * (self.state.clock - self.state.last_used[id]) as f64 } else { 0.0 }).collect();
        self.maximum(mask, &scores)
    }

    fn validate_state(&self, state: &CreditState) -> Result<()> {
        let known = |id: &String| self.state.quality.contains_key(id);
        let full = |values: &BTreeMap<String, f64>| values.len() == self.actions.len()
            && values.iter().all(|(id, value)| known(id) && value.is_finite());
        let total = self.actions.iter().map(|id| state.probabilities.get(id).copied().unwrap_or(f64::NAN)).sum::<f64>();
        if state.definition != self.state.definition || state.clock > i64::MAX as u64 || state.window.len() > self.window_size
            || state.window.iter().any(|item| !known(&item.action) || !item.reward.is_finite())
            || !full(&state.quality) || !full(&state.probabilities)
            || state.last_used.len() != self.actions.len() || state.last_used.iter().any(|(id, clock)| !known(id) || *clock > state.clock)
            || state.probabilities.values().any(|value| *value < self.minimum_probability || *value > 1.0)
            || !total.is_finite() || (total - 1.0).abs() > 1e-9
            || state.pairs.iter().any(|(id, row)| !known(id) || row.iter().any(|(next, value)| !known(next) || !value.is_finite()))
            || state.previous.as_ref().is_some_and(|id| !known(id))
            || state.pending.iter().any(|(decision, dispatch)| *decision > i64::MAX as u64 || !known(&dispatch.action)
                || dispatch.previous.as_ref().is_some_and(|id| !known(id))) {
            return Err(invalid("invalid non-stationary credit state / 非平稳信用状态无效"));
        }
        Ok(())
    }
}

impl SelectionPolicy for NonStationarySelector {
    fn revision(&self) -> u64 { self.revision }

    fn select(
        &mut self,
        context: &SelectionContext,
        available: &[String],
        random: &mut SearchRandom,
    ) -> Result<String> {
        let mask = self.mask(available, false)?;
        if context.decision > i64::MAX as u64 || context.features.iter().any(|value| !value.is_finite()) {
            return Err(invalid("invalid non-stationary context / 非平稳上下文无效"));
        }
        if self.learning && self.kind == "choice-function" && self.state.pending.contains_key(&context.decision) {
            return Err(invalid("decision already awaiting feedback / 决策正在等待反馈"));
        }
        let index = match self.kind.as_str() {
            "frr-mab" => self.frr(&mask)?,
            "choice-function" => self.choice(&mask)?,
            _ => {
                let total = mask.iter().map(|&index| self.state.probabilities[&self.actions[index]]).sum::<f64>();
                let draw = random.next_unit() * total;
                let mut cumulative = 0.0;
                mask.iter().copied().find(|&index| {
                    cumulative += self.state.probabilities[&self.actions[index]];
                    cumulative > draw
                }).unwrap_or(*mask.last().ok_or_else(|| invalid("empty mask"))?)
            }
        };
        let action = self.actions[index].clone();
        if self.learning && self.kind == "choice-function" {
            self.state.pending.insert(context.decision, CreditDispatch { action: action.clone(), previous: self.state.previous.clone() });
        }
        Ok(action)
    }

    fn observe(&mut self, transition: &SelectionTransition) -> Result<()> {
        self.mask(&transition.available, false)?;
        self.mask(&transition.next_available, true)?;
        if !self.state.quality.contains_key(&transition.action) || !transition.available.contains(&transition.action)
            || !transition.reward.is_finite() || transition.decision > i64::MAX as u64 || transition.evaluations > i64::MAX as u64
            || transition.elapsed_nanos > i64::MAX as u64 || transition.previous_action.as_ref().is_some_and(|id| !self.state.quality.contains_key(id))
            || self.state.pending.get(&transition.decision).is_some_and(|dispatch| dispatch.action != transition.action
                || transition.previous_action.as_ref().is_some_and(|id| dispatch.previous.as_ref() != Some(id))) {
            return Err(invalid("invalid credit observation / 信用反馈无效"));
        }
        if !self.learning { return Ok(()); }
        let revision = self.revision.checked_add(1).filter(|value| *value <= i64::MAX as u64).ok_or_else(|| invalid("credit revision overflow"))?;
        let mut state = self.state.clone();
        let predecessor = match state.pending.remove(&transition.decision) {
            Some(dispatch) => dispatch.previous,
            None => transition.previous_action.clone(),
        };
        let action = &transition.action;
        let value = transition.reward;
        state.window.push_back(CreditObservation { action: action.clone(), reward: value });
        while state.window.len() > self.window_size { state.window.pop_front(); }
        state.quality.insert(action.clone(), (1.0 - self.rate) * state.quality[action] + self.rate * value);
        if let Some(previous) = predecessor {
            let row = state.pairs.entry(previous).or_default();
            let old = row.get(action).copied().unwrap_or(0.0);
            row.insert(action.clone(), (1.0 - self.rate) * old + self.rate * value);
        }
        if self.kind == "adaptive-pursuit" {
            let scores: Vec<_> = self.actions.iter().map(|id| state.quality[id]).collect();
            let winner = self.maximum(&(0..self.actions.len()).collect::<Vec<_>>(), &scores)?;
            for (index, id) in self.actions.iter().enumerate() {
                let target = if index == winner { 1.0 - self.minimum_probability * (self.actions.len() - 1) as f64 } else { self.minimum_probability };
                let old = state.probabilities[id];
                state.probabilities.insert(id.clone(), if self.rate == 1.0 { target } else { old + self.rate * (target - old) });
            }
        }
        state.clock = state.clock.checked_add(1).ok_or_else(|| invalid("credit clock overflow"))?;
        state.last_used.insert(action.clone(), state.clock);
        state.previous = if transition.terminal { None } else { Some(action.clone()) };
        self.validate_state(&state)?;
        self.state = state;
        self.revision = revision;
        Ok(())
    }

    fn discard(&mut self, decision: u64) { self.state.pending.remove(&decision); }

    fn predecessor(&self, decision: u64) -> Option<String> {
        self.state.pending.get(&decision).and_then(|dispatch| dispatch.previous.clone())
    }

    fn snapshot(&self) -> Result<PolicySnapshot> {
        Ok(PolicySnapshot { version: 1, r#type: self.kind.clone(), actions: self.actions.clone(), revision: self.revision,
            cursor: 0, counts: Vec::new(), sums: Vec::new(), table: Vec::new(),
            extension: serde_json::to_value(&self.state).map_err(|error| invalid(error.to_string()))? })
    }

    fn restore(&mut self, snapshot: &PolicySnapshot) -> Result<()> {
        if snapshot.version != 1 || snapshot.r#type != self.kind || snapshot.actions.len() != self.actions.len()
            || snapshot.actions.iter().collect::<BTreeSet<_>>() != self.actions.iter().collect::<BTreeSet<_>>()
            || snapshot.revision > i64::MAX as u64 || snapshot.cursor != 0 || !snapshot.counts.is_empty()
            || !snapshot.sums.is_empty() || !snapshot.table.is_empty() {
            return Err(invalid("incompatible non-stationary policy snapshot / 非平稳策略快照不兼容"));
        }
        let state: CreditState = serde_json::from_value(snapshot.extension.clone()).map_err(|error| invalid(error.to_string()))?;
        self.validate_state(&state)?;
        if state.clock != snapshot.revision || state.window.len() as u64 > state.clock {
            return Err(invalid("inconsistent credit counters / 信用计数不一致"));
        }
        self.state = state;
        self.revision = snapshot.revision;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actions() -> Vec<String> { vec!["a".into(), "b".into(), "c".into()] }

    fn policy(kind: &str) -> NonStationarySelector {
        NonStationarySelector::new(&ComponentSpec::new(kind), actions(), true).unwrap()
    }

    fn event(action: &str, decision: u64, reward: f64) -> SelectionTransition {
        SelectionTransition { episode: "test".into(), decision, action: action.into(),
            before: SelectionContext { decision, ..Default::default() }, after: SelectionContext::default(),
            available: actions(), next_available: actions(), reward, evaluations: 1, terminal: false, policy_version: 0,
            feature_version: "progress:1".into(), scope: "algorithm".into(), steps: 1, failed: false,
            merged_improvement: false, elapsed_nanos: 0, previous_action: None }
    }

    #[test]
    fn invalid_definitions_masks_and_contexts_are_rejected_without_mutation() {
        for spec in [ComponentSpec::new("unknown"), ComponentSpec::new("frr-mab").parameter("window", 2),
            ComponentSpec::new("frr-mab").parameter("window", 1_000_001), ComponentSpec::new("choice-function").parameter("rate", 0.0),
            ComponentSpec::new("adaptive-pursuit").parameter("minimumProbability", 0.4),
            ComponentSpec::new("choice-function").parameter("states", 64), ComponentSpec::new("frr-mab").parameter("decay", -0.1)] {
            assert!(NonStationarySelector::new(&spec, actions(), true).is_err());
        }
        assert!(NonStationarySelector::new(&ComponentSpec::new("frr-mab"), vec!["a".into(), "a".into()], true).is_err());
        let mut policy = policy("choice-function");
        let before = policy.snapshot().unwrap();
        let mut random = SearchRandom::new(42);
        for mask in [vec![], vec!["a".into(), "a".into()], vec!["bad".into()]] {
            assert!(policy.select(&SelectionContext::default(), &mask, &mut random).is_err());
        }
        assert!(policy.select(&SelectionContext { features: vec![f64::NAN], ..Default::default() }, &actions(), &mut random).is_err());
        assert_eq!(policy.snapshot().unwrap(), before);
        assert_eq!(random.state(), 42);
    }

    #[test]
    fn frr_bootstraps_only_unobserved_masked_actions_in_canonical_order() {
        let mut policy = policy("frr-mab");
        let mut random = SearchRandom::new(42);
        let mask = vec!["c".into(), "b".into()];
        assert_eq!(policy.select(&SelectionContext::default(), &mask, &mut random).unwrap(), "b");
        policy.observe(&event("b", 0, 10.0)).unwrap();
        assert_eq!(policy.select(&SelectionContext::default(), &mask, &mut random).unwrap(), "c");
        assert_eq!(random.state(), 42);
    }

    #[test]
    fn frr_window_forgets_old_rewards_and_normalizes_extreme_positive_credit() {
        let spec = ComponentSpec::new("frr-mab").parameter("window", 3).parameter("exploration", 0.0);
        let mut policy = NonStationarySelector::new(&spec, actions(), true).unwrap();
        for (decision, (action, reward)) in [("a", f64::MAX), ("b", 1.0), ("c", -2.0)].into_iter().enumerate() {
            policy.observe(&event(action, decision as u64, reward)).unwrap();
        }
        assert_eq!(policy.select(&SelectionContext::default(), &actions(), &mut SearchRandom::new(42)).unwrap(), "a");
        for (decision, (action, reward)) in [("a", 0.0), ("b", 0.0), ("c", 4.0)].into_iter().enumerate() {
            policy.observe(&event(action, decision as u64 + 3, reward)).unwrap();
        }
        assert_eq!(policy.select(&SelectionContext::default(), &actions(), &mut SearchRandom::new(42)).unwrap(), "c");
        assert_eq!(policy.snapshot().unwrap().extension["window"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn pursuit_probabilities_follow_hand_computed_ema_and_masked_draws() {
        let spec = ComponentSpec::new("adaptive-pursuit").parameter("rate", 0.2).parameter("minimumProbability", 0.1);
        let mut policy = NonStationarySelector::new(&spec, actions(), true).unwrap();
        policy.observe(&event("b", 0, 1.0)).unwrap();
        let saved = policy.snapshot().unwrap();
        let probabilities = &saved.extension["probabilities"];
        assert_eq!(probabilities["a"].as_f64().unwrap(), 1.0 / 3.0 + 0.2 * (0.1 - 1.0 / 3.0));
        assert_eq!(probabilities["b"].as_f64().unwrap(), 1.0 / 3.0 + 0.2 * (0.8 - 1.0 / 3.0));
        let mut random = SearchRandom::new(42);
        let mut expected = random.clone();
        for decision in 0..32 {
            let a = probabilities["a"].as_f64().unwrap();
            let b = probabilities["b"].as_f64().unwrap();
            let draw = expected.next_unit() * (a + b);
            assert_eq!(policy.select(&SelectionContext { decision, ..Default::default() }, &["b".into(), "a".into()], &mut random).unwrap(),
                if draw < a { "a" } else { "b" });
        }
        assert_eq!(policy.snapshot().unwrap(), saved);
    }

    #[test]
    fn pursuit_full_rate_retains_exact_probability_floor() {
        let spec = ComponentSpec::new("adaptive-pursuit").parameter("rate", 1.0).parameter("minimumProbability", 0.1);
        let mut policy = NonStationarySelector::new(&spec, actions(), true).unwrap();
        policy.observe(&event("a", 0, 2.0)).unwrap();
        let snapshot = policy.snapshot().unwrap();
        assert_eq!(snapshot.extension["probabilities"]["b"], 0.1);
        policy.restore(&snapshot).unwrap();
    }

    #[test]
    fn choice_predecessors_are_frozen_for_out_of_order_completions_and_restore() {
        let mut policy = policy("choice-function");
        policy.observe(&event("a", 0, 2.0)).unwrap();
        let mut random = SearchRandom::new(42);
        for (decision, action) in [(1, "b"), (2, "c")] {
            assert_eq!(policy.select(&SelectionContext { decision, ..Default::default() }, &[action.into()], &mut random).unwrap(), action);
            assert_eq!(policy.predecessor(decision).as_deref(), Some("a"));
        }
        let saved = policy.snapshot().unwrap();
        let mut restored = super::tests::policy("choice-function");
        restored.restore(&saved).unwrap();
        for policy in [&mut policy, &mut restored] {
            let mut second = event("c", 2, 4.0); second.previous_action = Some("a".into());
            policy.observe(&second).unwrap();
            assert_eq!(policy.predecessor(1).as_deref(), Some("a"));
            let mut first = event("b", 1, 3.0); first.previous_action = Some("a".into());
            policy.observe(&first).unwrap();
            let state = policy.snapshot().unwrap().extension;
            assert_eq!(state["pairs"]["a"]["b"], 0.2 * 3.0);
            assert_eq!(state["pairs"]["a"]["c"], 0.2 * 4.0);
            assert!(state["pairs"].get("c").is_none());
            assert!(state["pending"].as_object().unwrap().is_empty());
        }
        assert_eq!(policy.snapshot().unwrap(), restored.snapshot().unwrap());
    }

    #[test]
    fn choice_discard_and_terminal_clear_only_the_intended_lifecycle() {
        let mut policy = policy("choice-function");
        let context = SelectionContext { decision: 7, ..Default::default() };
        let mut random = SearchRandom::new(42);
        policy.select(&context, &actions(), &mut random).unwrap();
        assert!(policy.select(&context, &actions(), &mut random).is_err());
        policy.discard(7);
        assert!(policy.snapshot().unwrap().extension["pending"].as_object().unwrap().is_empty());
        let mut feedback = event("b", 8, 1.0); feedback.terminal = true;
        policy.observe(&feedback).unwrap();
        policy.select(&context, &actions(), &mut random).unwrap();
        assert_eq!(policy.predecessor(7), None);
        assert_eq!(policy.revision(), 1);
    }

    #[test]
    fn frozen_select_and_observe_never_mutate_credit_state() {
        for kind in ["frr-mab", "choice-function", "adaptive-pursuit"] {
            let spec = ComponentSpec::new(kind);
            let mut learned = NonStationarySelector::new(&spec, actions(), true).unwrap();
            learned.observe(&event("b", 0, 2.0)).unwrap();
            let mut frozen = NonStationarySelector::new(&spec, actions(), false).unwrap();
            frozen.restore(&learned.snapshot().unwrap()).unwrap();
            let before = frozen.snapshot().unwrap();
            let mut random = SearchRandom::new(1);
            for _ in 0..8 {
                frozen.select(&SelectionContext::default(), &actions(), &mut random).unwrap();
                frozen.observe(&event("a", 1, 10.0)).unwrap();
                frozen.discard(2);
            }
            assert_eq!(frozen.snapshot().unwrap(), before, "{kind}");
        }
    }

    #[test]
    fn restoration_reorders_by_ids_and_rejects_bad_credit_atomically() {
        let mut policy = policy("choice-function");
        policy.observe(&event("a", 0, 2.0)).unwrap();
        let saved = policy.snapshot().unwrap();
        let mut target = NonStationarySelector::new(&ComponentSpec::new("choice-function"), vec!["c".into(), "b".into(), "a".into()], true).unwrap();
        target.restore(&saved).unwrap();
        assert_eq!(target.snapshot().unwrap().extension, saved.extension);
        for (path, value) in [("/quality/a", serde_json::json!("bad")), ("/probabilities/a", serde_json::json!(-0.1)),
            ("/lastUsed/a", serde_json::json!(2)), ("/definition/0", serde_json::json!(51)), ("/clock", serde_json::json!(2))] {
            let mut bad = saved.clone(); *bad.extension.pointer_mut(path).unwrap() = value;
            assert!(policy.restore(&bad).is_err(), "{path}");
            assert_eq!(policy.snapshot().unwrap(), saved);
        }
        let mut bad = saved.clone(); bad.extension["pending"] = serde_json::json!({"-1":{"action":"a","previous":null}});
        assert!(policy.restore(&bad).is_err());
        let mut bad = saved.clone(); bad.actions = vec!["a".into(), "a".into(), "b".into()];
        assert!(policy.restore(&bad).is_err());
    }

    #[test]
    fn invalid_feedback_keeps_pending_credit_and_revision_unchanged() {
        let mut policy = policy("choice-function");
        policy.select(&SelectionContext::default(), &["a".into()], &mut SearchRandom::new(42)).unwrap();
        let before = policy.snapshot().unwrap();
        for mut transition in [event("b", 0, 1.0), event("a", 0, f64::NAN), event("a", 0, 1.0)] {
            if transition.reward == 1.0 && transition.action == "a" { transition.previous_action = Some("c".into()); }
            assert!(policy.observe(&transition).is_err());
            assert_eq!(policy.snapshot().unwrap(), before);
        }
    }
}

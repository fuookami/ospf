//! 任务对换管理模块 / Flight task reverse management module
use std::collections::HashMap;
use time::Duration;

/// 可对换任务对 / Reversible pair (对齐 FSRA FlightTaskReverse.ReversiblePair)
#[derive(Debug, Clone)]
pub struct ReversiblePair {
    /// 前序任务标识 / Preceding task identifier
    pub prev_task_id: String,
    /// 后序任务标识 / Succeeding task identifier
    pub succ_task_id: String,
    /// 是否对称 / Whether symmetrical
    pub symmetrical: bool,
}

/// 任务对换管理器 / Flight task reverse (对齐 FSRA FlightTaskReverse)
#[derive(Debug, Clone)]
pub struct FlightTaskReverse {
    symmetrical_pairs: Vec<ReversiblePair>,
    left_mapper: HashMap<String, Vec<ReversiblePair>>,
    right_mapper: HashMap<String, Vec<ReversiblePair>>,
}

impl FlightTaskReverse {
    /// 默认时间差限制 / Default time difference limit
    pub const DEFAULT_TIME_DIFFERENCE_LIMIT: Duration = Duration::hours(5);

    /// 创建任务对换管理器 / Create flight task reverse
    pub fn new(
        pairs: Vec<(String, String)>,
        origin_bunches: &[Vec<String>],
        locked_tasks: &[String],
        time_difference_limit: Duration,
    ) -> Self {
        let mut symmetrical_pairs = Vec::new();
        let mut left_mapper: HashMap<String, Vec<ReversiblePair>> = HashMap::new();
        let mut right_mapper: HashMap<String, Vec<ReversiblePair>> = HashMap::new();

        for (prev_id, succ_id) in &pairs {
            let symmetrical = Self::check_symmetrical(
                origin_bunches,
                prev_id,
                succ_id,
                locked_tasks,
            );
            let pair = ReversiblePair {
                prev_task_id: prev_id.clone(),
                succ_task_id: succ_id.clone(),
                symmetrical,
            };

            left_mapper
                .entry(prev_id.clone())
                .or_default()
                .push(pair.clone());
            right_mapper
                .entry(succ_id.clone())
                .or_default()
                .push(pair.clone());

            if symmetrical {
                symmetrical_pairs.push(pair);
            }
        }

        Self {
            symmetrical_pairs,
            left_mapper,
            right_mapper,
        }
    }

    /// 检查是否对称 / Check if symmetrical
    fn check_symmetrical(
        origin_bunches: &[Vec<String>],
        prev_id: &str,
        succ_id: &str,
        _locked_tasks: &[String],
    ) -> bool {
        origin_bunches.iter().any(|bunch| {
            bunch.windows(2).any(|w| w[0] == *prev_id && w[1] == *succ_id)
        })
    }

    /// 是否包含对换关系 / Contains reverse pair
    pub fn contains(&self, prev_id: &str, succ_id: &str) -> bool {
        self.left_mapper
            .get(prev_id)
            .map(|pairs| pairs.iter().any(|p| p.succ_task_id == succ_id))
            .unwrap_or(false)
    }

    /// 是否对称 / Is symmetrical
    pub fn symmetrical(&self, prev_id: &str, succ_id: &str) -> bool {
        self.left_mapper
            .get(prev_id)
            .and_then(|pairs| pairs.iter().find(|p| p.succ_task_id == succ_id))
            .map(|p| p.symmetrical)
            .unwrap_or(false)
    }

    /// 左查找 / Left find (by prev task)
    pub fn left_find(&self, task_id: &str) -> &[ReversiblePair] {
        self.left_mapper
            .get(task_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// 右查找 / Right find (by succ task)
    pub fn right_find(&self, task_id: &str) -> &[ReversiblePair] {
        self.right_mapper
            .get(task_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// 对称任务对 / Symmetrical pairs
    pub fn symmetrical_pairs(&self) -> &[ReversiblePair] {
        &self.symmetrical_pairs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_returns_true_for_existing_pair() {
        let reverse = FlightTaskReverse::new(
            vec![("T1".into(), "T2".into())],
            &[],
            &[],
            Duration::hours(5),
        );
        assert!(reverse.contains("T1", "T2"));
        assert!(!reverse.contains("T2", "T1"));
        assert!(!reverse.contains("T1", "T3"));
    }

    #[test]
    fn left_find_returns_pairs_by_prev_task() {
        let reverse = FlightTaskReverse::new(
            vec![("T1".into(), "T2".into()), ("T1".into(), "T3".into())],
            &[],
            &[],
            Duration::hours(5),
        );
        let pairs = reverse.left_find("T1");
        assert_eq!(pairs.len(), 2);
        assert!(pairs.iter().any(|p| p.succ_task_id == "T2"));
        assert!(pairs.iter().any(|p| p.succ_task_id == "T3"));
    }

    #[test]
    fn right_find_returns_pairs_by_succ_task() {
        let reverse = FlightTaskReverse::new(
            vec![("T1".into(), "T2".into()), ("T3".into(), "T2".into())],
            &[],
            &[],
            Duration::hours(5),
        );
        let pairs = reverse.right_find("T2");
        assert_eq!(pairs.len(), 2);
    }

    #[test]
    fn symmetrical_detected_from_origin_bunches() {
        let reverse = FlightTaskReverse::new(
            vec![("T1".into(), "T2".into())],
            &[vec!["T1".into(), "T2".into()]],
            &[],
            Duration::hours(5),
        );
        assert!(reverse.symmetrical("T1", "T2"));
    }

    #[test]
    fn empty_reverse_has_no_pairs() {
        let reverse = FlightTaskReverse::new(vec![], &[], &[], Duration::hours(5));
        assert!(reverse.symmetrical_pairs().is_empty());
        assert!(reverse.left_find("T1").is_empty());
    }
}

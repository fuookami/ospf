//! 版本化调度录制与实际执行重放 / Versioned schedule recording and real-execution replay

use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use crate::solver::heuristic::result::Result;
use crate::solver::hyper_heuristic::invalid;
use crate::solver::hyper_heuristic::persistence::fingerprint;

fn zero(value: &u64) -> bool { *value == 0 }
fn signed_zero(value: &i64) -> bool { *value == 0 }
fn no(value: &bool) -> bool { !*value }
fn version() -> u32 { 1 }
fn one(value: &u32) -> bool { *value == 1 }

/// 协调器事件；预算与随机位模式用于验证而非替代真实计算 / Coordinator event; budgets and random bits validate rather than replace execution
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleEvent {
    /// begin/dispatch/complete/drain/yield/end / begin/dispatch/complete/drain/yield/end
    pub kind: String,
    /// 派发序号 / Dispatch sequence
    #[serde(skip_serializing_if = "zero")]
    pub decision: u64,
    /// worker 标识 / Worker identifier
    #[serde(skip_serializing_if = "String::is_empty")]
    pub action: String,
    /// 预留或实际评估请求 / Reserved or actual evaluation requests
    #[serde(skip_serializing_if = "zero")]
    pub evaluations: u64,
    /// 预留或完整提交步数 / Reserved or complete committed steps
    #[serde(skip_serializing_if = "zero")]
    pub steps: u64,
    /// 协调器随机流位模式 / Coordinator random stream bits
    #[serde(skip_serializing_if = "signed_zero")]
    pub random: i64,
    /// 按选择器输入顺序的可用动作 / Available actions in selector input order
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub available: Vec<String>,
    /// 已观察到停止请求 / Observed stop request
    #[serde(skip_serializing_if = "no")]
    pub stopped: bool,
    /// 失败标志 / Failure flag
    #[serde(skip_serializing_if = "no")]
    pub failed: bool,
}

/// 可校验的调度日志；不包含目标值替身 / Verifiable schedule log without substitute objective values
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScheduleLog {
    /// 协议版本 / Protocol version
    #[serde(default = "version", skip_serializing_if = "one")]
    pub version: u32,
    /// 搜索配置指纹 / Search configuration fingerprint
    pub configuration: String,
    /// 问题标识和数据指纹 / Problem identifier and data fingerprint
    pub problem: String,
    /// 有序事件 / Ordered events
    pub events: Vec<ScheduleEvent>,
    /// 以上字段的规范 JSON SHA-256 / Canonical JSON SHA-256 of the preceding fields
    pub checksum: String,
}

impl ScheduleLog {
    fn digest(&self) -> Result<String> {
        let mut value = serde_json::to_value(self).map_err(|error| invalid(error.to_string()))?;
        value.as_object_mut().ok_or_else(|| invalid("invalid schedule"))?.remove("checksum");
        Ok(fingerprint(&value))
    }

    /// 验证完整源日志，包括事件配对与结束标记 / Validate a complete source log, including pairing and its end marker
    pub fn validate(&self) -> Result<()> { self.validate_prefix(true) }

    fn validate_prefix(&self, sealed: bool) -> Result<()> {
        if self.version != 1 || self.configuration.is_empty() || self.problem.is_empty() || self.checksum != self.digest()? {
            return Err(invalid("invalid schedule identity, version or checksum"));
        }
        let mut active = BTreeMap::new();
        let mut decision = 0;
        let mut evaluations_total = 0_u64;
        let mut steps_total = 0_u64;
        let mut advancing = false;
        let mut ended = false;
        for event in &self.events {
            if ended { return Err(invalid("schedule contains events after end")); }
            match event.kind.as_str() {
                "begin" if !advancing => { advancing = true; }
                "dispatch" if advancing && event.decision == decision && event.evaluations > 0 && event.steps > 0
                    && event.available.contains(&event.action) && !active.contains_key(&event.action) => {
                    active.insert(event.action.clone(), (decision, event.evaluations, event.steps)); decision += 1;
                }
                "complete" => {
                    let Some((id, evaluations, steps)) = active.remove(&event.action) else { return Err(invalid("unexpected schedule completion")); };
                    if id != event.decision || event.evaluations > evaluations || event.steps > steps {
                        return Err(invalid("inconsistent schedule completion allowances"));
                    }
                    evaluations_total = evaluations_total.checked_add(event.evaluations).ok_or_else(|| invalid("schedule evaluation overflow"))?;
                    steps_total = steps_total.checked_add(event.steps).ok_or_else(|| invalid("schedule step overflow"))?;
                }
                "drain" if !active.is_empty() => {}
                "yield" if advancing => { advancing = false; }
                "end" if !advancing && active.is_empty() && event.evaluations == evaluations_total && event.steps == steps_total => { ended = true; }
                _ => return Err(invalid("invalid schedule event ordering")),
            }
        }
        if advancing || !active.is_empty() || (sealed && !ended) { return Err(invalid("truncated or non-quiescent schedule")); }
        Ok(())
    }
}

/// 静止检查点中的已消费日志前缀和游标 / Consumed log prefix and cursor in a quiescent checkpoint
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduleCheckpoint {
    /// 下一事件下标 / Index of the next event
    pub cursor: usize,
    /// 带身份与校验和的已消费前缀 / Consumed prefix with identity and checksum
    pub log: ScheduleLog,
}

#[derive(Default)]
pub(crate) struct ScheduleControl {
    history: Option<ScheduleLog>,
    source: Option<ScheduleLog>,
    bound: bool,
    pub(crate) diverged: bool,
}

impl ScheduleControl {
    pub(crate) fn bind(&mut self, configuration: String, problem: String, source: Option<&ScheduleLog>) -> Result<()> {
        if let Some(source) = source {
            source.validate()?;
            if source.configuration != configuration || source.problem != problem { return Err(invalid("incompatible schedule identity")); }
        }
        let history = self.history.get_or_insert_with(|| ScheduleLog { version: 1, configuration, problem, events: Vec::new(), checksum: String::new() });
        if let Some(source) = source {
            if !source.events.starts_with(&history.events) { return Err(invalid("schedule checkpoint prefix mismatch")); }
        }
        self.source = source.cloned(); self.bound = true;
        Ok(())
    }

    pub(crate) fn replaying(&self) -> bool { self.source.is_some() && !self.diverged }
    pub(crate) fn enabled(&self) -> bool { self.history.is_some() }
    pub(crate) fn ready(&self) -> Result<()> {
        if self.history.is_some() && !self.bound { Err(invalid("bind record_schedule or replay_schedule after checkpoint restoration")) } else { Ok(()) }
    }
    pub(crate) fn peek(&self) -> Option<&ScheduleEvent> {
        self.source.as_ref().and_then(|source| source.events.get(self.history.as_ref().map_or(0, |history| history.events.len())))
    }
    pub(crate) fn ended(&self) -> bool { self.history.as_ref().and_then(|log| log.events.last()).is_some_and(|event| event.kind == "end") }

    pub(crate) fn event(&mut self, event: ScheduleEvent) -> Result<()> {
        self.ready()?;
        if self.diverged || self.history.is_none() { return Ok(()); }
        if self.source.is_some() && self.peek() != Some(&event) {
            self.diverged = true;
            return Err(invalid(format!("schedule diverged at event {}: expected {:?}, actual {:?}", self.history.as_ref().map_or(0, |log| log.events.len()), self.peek(), event)));
        }
        if self.ended() { return Err(invalid("schedule already ended")); }
        if let Some(history) = &mut self.history { history.events.push(event); }
        Ok(())
    }

    pub(crate) fn snapshot(&self) -> Result<Option<ScheduleCheckpoint>> {
        let Some(mut log) = self.history.clone() else { return Ok(None); };
        if self.diverged { return Err(invalid("cannot checkpoint a diverged replay")); }
        log.checksum = log.digest()?;
        log.validate_prefix(false)?;
        Ok(Some(ScheduleCheckpoint { cursor: log.events.len(), log }))
    }

    pub(crate) fn restore(&mut self, saved: Option<&ScheduleCheckpoint>) -> Result<()> {
        if let Some(saved) = saved {
            saved.log.validate_prefix(false)?;
            if saved.cursor != saved.log.events.len() { return Err(invalid("invalid schedule cursor")); }
            self.history = Some(saved.log.clone()); self.bound = false; self.source = None;
        }
        Ok(())
    }

    pub(crate) fn export(&self) -> Result<ScheduleLog> {
        let log = self.snapshot()?.ok_or_else(|| invalid("schedule recording is not enabled"))?.log;
        log.validate()?;
        Ok(log)
    }
}

//! Logic-Based Benders 求解合同 / Logic-Based Benders solve contract.
//!
//! 本模块与经典 dual/Farkas Benders 并行，负责把稳定 master binding、CP subproblem
//! 结果和全局有效 cut 组合成统一 `SolveReport`。它不实现 CP 搜索，也不依赖 master
//! column index 或变量名称作为长期身份。/ This module is parallel to classical
//! dual/Farkas Benders. It combines stable master bindings, CP subproblem results, and
//! globally valid cuts into the unified `SolveReport`. It does not implement CP search and
//! never uses master column indices or variable names as long-lived identities.

use ospf_rust_core::error::{CoreError, Result, SolverError};
use ospf_rust_core::model::ObjectiveCategory;
use ospf_rust_core::model::constraint_programming::ConstraintProgrammingSnapshot;
#[cfg(feature = "serde")]
use ospf_rust_core::model::constraint_programming::ConstraintProgrammingSnapshotArtifact;
use ospf_rust_core::solver::{
    AuditFingerprint, ConstraintProgrammingAssumption, ConstraintProgrammingAssumptionId,
    ProblemStatus, ProgressValue, SolveDiagnostics, SolveHandle, SolveIssue,
    SolveIterationSnapshot, SolveProof, SolveReport, SolveSolution, SolveStage, SolveTrace,
    SolverProvenance, StableVariableId, TerminationReason, cancelled_solve_report,
    require_infeasibility_certificate, sha256_fingerprint, snapshot_with_assumptions,
};
use std::collections::{BTreeMap, BTreeSet};

use super::FrameworkSolveOptions;
use super::column_generation_solver::emit_combinatorial_progress;

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::Solver(SolverError::InvalidInput(message.into()))
}

fn contract(message: impl Into<String>) -> CoreError {
    CoreError::Solver(SolverError::ContractViolation(message.into()))
}

fn failure_report(
    reason: TerminationReason,
    issue: SolveIssue,
    provenance_name: &str,
) -> Result<SolveReport<f64>> {
    let mut diagnostics = SolveDiagnostics::default();
    diagnostics.issues.push(issue);
    SolveReport::builder(ProblemStatus::Unknown, reason)
        .diagnostics(diagnostics)
        .provenance(ospf_rust_core::solver::SolverProvenance {
            solver_id: provenance_name.to_owned(),
            backend_name: "logic-based-benders".to_owned(),
            deterministic: Some(true),
            ..ospf_rust_core::solver::SolverProvenance::default()
        })
        .build()
}

fn approximately_equal(left: f64, right: f64, tolerance: f64) -> bool {
    (left - right).abs() <= tolerance
}

fn cp_assignment_fingerprint(assignment: &BTreeMap<StableVariableId, i64>) -> AuditFingerprint {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(assignment.len() as u64).to_le_bytes());
    for (stable_id, value) in assignment {
        bytes.extend_from_slice(&(stable_id.0.len() as u64).to_le_bytes());
        bytes.extend_from_slice(stable_id.0.as_bytes());
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    sha256_fingerprint("ospf.lbb.cp.assignment", &bytes)
}

fn cp_master_assignment_fingerprint(assignment: &MasterAssignment) -> AuditFingerprint {
    cp_assignment_fingerprint(&assignment.values)
}

fn cp_report_fingerprint(report: &SolveReport<i64>) -> AuditFingerprint {
    sha256_fingerprint("ospf.lbb.cp.report", format!("{report:?}").as_bytes())
}

fn validate_master_report_identity(
    report: &SolveReport<f64>,
    expected: Option<&AuditFingerprint>,
) -> Result<()> {
    if let Some(expected) = expected
        && report.fingerprints.model.as_ref() != Some(expected)
    {
        return Err(contract(
            "Logic-Based Benders master report does not match the master identity",
        ));
    }
    Ok(())
}

const MAX_EXACT_INTEGER_F64: i128 = 1_i128 << 53;

fn exact_f64_integer(value: i128, context: &str) -> Result<f64> {
    if value.unsigned_abs() > MAX_EXACT_INTEGER_F64 as u128 {
        return Err(invalid(format!(
            "{context}={value} exceeds the exact f64 integer range"
        )));
    }
    let converted = value as f64;
    if !converted.is_finite() || converted as i128 != value {
        return Err(invalid(format!(
            "{context}={value} cannot be represented exactly as f64"
        )));
    }
    Ok(converted)
}

fn exact_f64_integer_value(value: f64, context: &str) -> Result<i64> {
    if !value.is_finite() || value.fract() != 0.0 {
        return Err(invalid(format!(
            "{context}={value} is not a finite integer"
        )));
    }
    let integer = value as i128;
    if integer as f64 != value {
        return Err(invalid(format!(
            "{context}={value} cannot be represented exactly as an i64"
        )));
    }
    i64::try_from(integer)
        .map_err(|_| invalid(format!("{context}={value} is outside the i64 range")))
}

fn rounded_f64_integer_value(value: f64, round_up: bool, context: &str) -> Result<i64> {
    if !value.is_finite() {
        return Err(invalid(format!("{context}={value} is not finite")));
    }
    let rounded = if round_up {
        value.ceil()
    } else {
        value.floor()
    };
    exact_f64_integer_value(rounded, context)
}

fn validate_feasible_payload(
    report: &SolveReport<i64>,
    assignment: &BTreeMap<StableVariableId, i64>,
    objective: Option<f64>,
    best_bound: Option<f64>,
) -> Result<()> {
    report.validate()?;
    if report.problem_status != ProblemStatus::Feasible || !report.has_incumbent() {
        return Err(contract(
            "feasible CP subproblem result requires a feasible incumbent report",
        ));
    }
    let solution = report
        .solution
        .as_ref()
        .ok_or_else(|| contract("feasible CP subproblem result has no solution"))?;
    if solution.stable_values != *assignment {
        return Err(contract(
            "CP feasible assignment does not match the report stable values",
        ));
    }
    if objective.is_some_and(|value| !value.is_finite()) {
        return Err(invalid("CP feasible objective must be finite"));
    }
    if best_bound.is_some_and(|value| !value.is_finite()) {
        return Err(invalid("CP feasible best bound must be finite"));
    }
    if let Some(objective) = objective
        && !solution
            .objective_value
            .or_else(|| solution.objective.map(|value| value as f64))
            .is_some_and(|report_objective| approximately_equal(objective, report_objective, 1e-9))
    {
        return Err(contract(
            "CP feasible objective does not match the report objective",
        ));
    }
    if let Some(best_bound) = best_bound {
        let report_bound = report
            .statistics
            .best_bound_value
            .or_else(|| report.statistics.best_bound.map(|value| value as f64));
        if !report_bound
            .is_some_and(|report_bound| approximately_equal(best_bound, report_bound, 1e-9))
        {
            return Err(contract(
                "CP feasible best bound does not match the report bound",
            ));
        }
    }
    Ok(())
}

fn validate_snapshot_feasible_payload(
    report: &SolveReport<i64>,
    snapshot: &ConstraintProgrammingSnapshot,
    assignment: &BTreeMap<StableVariableId, i64>,
    objective: Option<f64>,
    best_bound: Option<f64>,
) -> Result<()> {
    validate_feasible_payload(report, assignment, objective, best_bound)?;
    if report.fingerprints.model.as_ref() != Some(&snapshot.fingerprint) {
        return Err(contract(
            "CP feasible report model fingerprint does not match the snapshot",
        ));
    }

    let solution = report
        .solution
        .as_ref()
        .ok_or_else(|| contract("feasible CP subproblem result has no solution"))?;
    let expected_objective = snapshot.objective_value(assignment)?;
    let report_objective = solution.objective;
    let report_objective_value = solution.objective_value;
    match expected_objective {
        Some(expected) => {
            if report_objective != Some(expected) {
                return Err(contract(
                    "CP feasible report objective does not match the snapshot objective",
                ));
            }
            let expected_value =
                exact_f64_integer(i128::from(expected), "CP feasible snapshot objective")?;
            if let Some(report_objective_value) = report_objective_value
                && !approximately_equal(report_objective_value, expected_value, 1e-9)
            {
                return Err(contract(
                    "CP feasible report floating objective does not match the snapshot objective",
                ));
            }
            if let Some(objective) = objective
                && !approximately_equal(objective, expected_value, 1e-9)
            {
                return Err(contract(
                    "CP feasible objective does not match the snapshot objective",
                ));
            }
        }
        None => {
            if report_objective.is_some() || report_objective_value.is_some() {
                return Err(contract(
                    "satisfaction CP snapshot cannot carry an objective value",
                ));
            }
            if objective.is_some() {
                return Err(contract(
                    "satisfaction CP snapshot cannot accept an objective value",
                ));
            }
        }
    }

    let typed_bound = report.statistics.best_bound;
    let float_bound = report
        .statistics
        .best_bound_value
        .map(|value| {
            let round_up = snapshot
                .objective
                .as_ref()
                .is_some_and(|objective| objective.category.is_minimum());
            rounded_f64_integer_value(value, round_up, "CP feasible report best bound")
        })
        .transpose()?;
    if let (Some(typed_bound), Some(float_bound)) = (typed_bound, float_bound)
        && typed_bound != float_bound
    {
        return Err(contract(
            "CP feasible typed best bound does not match its floating snapshot",
        ));
    }
    let report_bound = report
        .statistics
        .best_bound_value
        .or_else(|| report.statistics.best_bound.map(|value| value as f64));
    if let Some(best_bound) = best_bound
        && !report_bound
            .is_some_and(|report_bound| approximately_equal(best_bound, report_bound, 1e-9))
    {
        return Err(contract(
            "CP feasible best bound does not match the snapshot report bound",
        ));
    }
    if expected_objective.is_none() && (report_bound.is_some() || best_bound.is_some()) {
        return Err(contract(
            "satisfaction CP snapshot cannot carry a best bound",
        ));
    }
    if let (Some(expected), Some(report_bound)) = (expected_objective, report_bound) {
        let expected = i128::from(expected);
        let valid_direction = snapshot.objective.as_ref().is_some_and(|objective| {
            if objective.category.is_minimum() {
                report_bound <= expected as f64 + 1e-9
            } else {
                report_bound + 1e-9 >= expected as f64
            }
        });
        if !valid_direction {
            return Err(contract(
                "CP feasible best bound has the wrong direction for the snapshot objective",
            ));
        }
    }
    Ok(())
}

fn stable_component(id: &StableVariableId) -> String {
    let escaped =
        id.0.bytes()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
    format!("{}:{escaped}", id.0.len())
}

pub(crate) fn bounded_auxiliary_id(
    family_id: &str,
    variable_id: &StableVariableId,
    side: &str,
) -> StableVariableId {
    StableVariableId(format!(
        "{family_id}/{}/{}",
        stable_component(variable_id),
        side
    ))
}

/// master 变量的值域 / Domain of one master variable.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MasterVariableDomain {
    /// 二进制变量 / Binary variable.
    Binary,
    /// 有限整数区间 / Bounded integer interval.
    Integer {
        /// 下界 / Lower bound.
        lower: i64,
        /// 上界 / Upper bound.
        upper: i64,
    },
}

impl MasterVariableDomain {
    pub(crate) fn validate(self) -> Result<()> {
        if let Self::Integer { lower, upper } = self {
            if lower > upper {
                return Err(invalid(format!(
                    "master integer domain has lower {lower} above upper {upper}"
                )));
            }
            exact_f64_integer(lower as i128, "master integer lower bound")?;
            exact_f64_integer(upper as i128, "master integer upper bound")?;
        }
        Ok(())
    }

    fn contains(self, value: i64) -> bool {
        match self {
            Self::Binary => (0..=1).contains(&value),
            Self::Integer { lower, upper } => (lower..=upper).contains(&value),
        }
    }
}

/// 稳定 master 变量绑定 / Stable master-variable binding.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasterVariableBinding {
    /// 稳定变量 ID / Stable variable ID.
    pub id: StableVariableId,
    /// 整数值域 / Integer domain.
    pub domain: MasterVariableDomain,
}

impl MasterVariableBinding {
    /// 创建二进制绑定 / Create a binary binding.
    pub fn binary(id: impl Into<StableVariableId>) -> Self {
        Self {
            id: id.into(),
            domain: MasterVariableDomain::Binary,
        }
    }

    /// 创建有界整数绑定 / Create a bounded-integer binding.
    pub fn integer(id: impl Into<StableVariableId>, lower: i64, upper: i64) -> Self {
        Self {
            id: id.into(),
            domain: MasterVariableDomain::Integer { lower, upper },
        }
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if self.id.0.trim().is_empty() {
            return Err(invalid("master binding ID must not be empty"));
        }
        self.domain.validate()
    }
}

/// 稳定 master 绑定集合 / Stable set of master bindings.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasterBinding {
    bindings: BTreeMap<StableVariableId, MasterVariableDomain>,
}

impl MasterBinding {
    /// 创建并校验绑定集合 / Create and validate a binding set.
    pub fn new(bindings: impl IntoIterator<Item = MasterVariableBinding>) -> Result<Self> {
        let mut normalized = BTreeMap::new();
        for binding in bindings {
            binding.validate()?;
            if normalized
                .insert(binding.id.clone(), binding.domain)
                .is_some()
            {
                return Err(invalid(format!(
                    "duplicate master binding ID {}",
                    binding.id.0
                )));
            }
        }
        if normalized.is_empty() {
            return Err(invalid(
                "Logic-Based Benders requires at least one master binding",
            ));
        }
        Ok(Self {
            bindings: normalized,
        })
    }

    /// 获取绑定值域 / Get a binding domain.
    pub fn domain(&self, id: &StableVariableId) -> Option<MasterVariableDomain> {
        self.bindings.get(id).copied()
    }

    /// 获取稳定绑定 ID / Get stable binding IDs.
    pub fn ids(&self) -> impl Iterator<Item = &StableVariableId> {
        self.bindings.keys()
    }

    /// 获取稳定绑定及其值域 / Get stable bindings and their domains.
    #[cfg(feature = "serde")]
    pub(crate) fn entries(
        &self,
    ) -> impl Iterator<Item = (&StableVariableId, MasterVariableDomain)> {
        self.bindings.iter().map(|(id, domain)| (id, *domain))
    }

    /// 获取绑定数量 / Get binding count.
    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    /// 判断绑定是否为空 / Check whether the binding is empty.
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    fn validate_assignment(&self, assignment: &MasterAssignment) -> Result<()> {
        if assignment.values.len() != self.bindings.len()
            || assignment
                .values
                .keys()
                .any(|id| !self.bindings.contains_key(id))
        {
            return Err(contract(
                "master assignment does not match the stable binding set",
            ));
        }
        for (id, domain) in &self.bindings {
            let value = assignment.values.get(id).copied().ok_or_else(|| {
                contract(format!(
                    "master assignment is missing stable variable {}",
                    id.0
                ))
            })?;
            if !domain.contains(value) {
                return Err(contract(format!(
                    "master assignment value {}={} is outside its domain",
                    id.0, value
                )));
            }
        }
        Ok(())
    }
}

/// 经过严格转换的 master incumbent / Strictly converted master incumbent.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasterAssignment {
    /// 按稳定 ID 排序的整数值 / Integer values keyed by stable ID.
    pub values: BTreeMap<StableVariableId, i64>,
}

impl MasterAssignment {
    /// 创建 assignment / Create an assignment.
    pub fn new(values: BTreeMap<StableVariableId, i64>, binding: &MasterBinding) -> Result<Self> {
        let assignment = Self { values };
        binding.validate_assignment(&assignment)?;
        Ok(assignment)
    }

    /// 获取稳定变量值 / Get one stable variable value.
    pub fn get(&self, id: &StableVariableId) -> Option<i64> {
        self.values.get(id).copied()
    }

    /// 将统一 master 报告转换为整数 assignment / Convert a unified master report to an integer assignment.
    pub fn from_report(
        report: &SolveReport<f64>,
        binding: &MasterBinding,
        integrality_tolerance: f64,
    ) -> Result<Self> {
        report.validate()?;
        if !report.is_optimal() {
            return Err(contract(
                "Logic-Based Benders master assignment requires a verified optimal report",
            ));
        }
        if !integrality_tolerance.is_finite() || integrality_tolerance < 0.0 {
            return Err(invalid(
                "master integrality tolerance must be finite and non-negative",
            ));
        }
        let solution = report
            .solution
            .as_ref()
            .ok_or_else(|| contract("optimal master report has no solution"))?;
        let mut values = BTreeMap::new();
        for (id, domain) in &binding.bindings {
            let raw = solution.stable_values.get(id).copied().ok_or_else(|| {
                contract(format!(
                    "optimal master report is missing stable variable {}",
                    id.0
                ))
            })?;
            if !raw.is_finite() {
                return Err(contract(format!(
                    "master stable variable {} has a non-finite value",
                    id.0
                )));
            }
            let rounded = raw.round();
            if (raw - rounded).abs() > integrality_tolerance {
                return Err(contract(format!(
                    "master stable variable {} is not integral: {raw}",
                    id.0
                )));
            }
            if rounded < i64::MIN as f64 || rounded > i64::MAX as f64 {
                return Err(contract(format!(
                    "master stable variable {} is outside the i64 range",
                    id.0
                )));
            }
            if rounded.abs() > MAX_EXACT_INTEGER_F64 as f64 {
                return Err(contract(format!(
                    "master stable variable {} is outside the exact f64 integer range",
                    id.0
                )));
            }
            let value = rounded as i64;
            if !domain.contains(value) {
                return Err(contract(format!(
                    "master stable variable {}={} is outside its declared domain",
                    id.0, value
                )));
            }
            values.insert(id.clone(), value);
        }
        Self::new(values, binding)
    }

    fn canonical_key(&self, ids: impl Iterator<Item = StableVariableId>) -> String {
        let mut encoded = String::new();
        for id in ids {
            if let Some(value) = self.values.get(&id) {
                encoded.push_str(&format!("{}={value};", stable_component(&id)));
            }
        }
        encoded
    }
}

/// cut 的全局有效性 / Validity scope of a master cut.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MasterCutValidity {
    /// 对所有原问题 master assignment 有效 / Valid for every original master assignment.
    Global,
    /// 只对当前 branch/local 区域有效 / Valid only in a local branch region.
    Local,
}

/// cut 证明状态 / Proof status of a master cut.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MasterCutProofStatus {
    /// 已验证 / Verified.
    Verified,
    /// 仅声明 / Claimed.
    Claimed,
    /// 无证明 / None.
    None,
}

/// bounded-integer no-good 的行类型 / Row type of a bounded-integer no-good.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundedIntegerNoGoodRow {
    /// 选择低侧 / Select the lower side.
    LowerSide(StableVariableId),
    /// 选择高侧 / Select the upper side.
    UpperSide(StableVariableId),
    /// 每个变量最多选择一侧 / At most one side per variable.
    AtMostOne(StableVariableId),
    /// 至少选择一个非等值侧 / Select at least one non-equal side.
    RequireDifference,
}

/// cut 的表达式编码类型 / Expression encoding type of a master cut.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MasterCutEncoding {
    /// 普通线性 cut / Ordinary linear cut.
    Linear,
    /// 完整或子集 binary no-good / Full or subset binary no-good.
    BinaryNoGood {
        /// 被排除的 assignment / Assignment excluded by this cut.
        assignment: BTreeMap<StableVariableId, i64>,
    },
    /// 有界整数多行 no-good / Bounded-integer multi-row no-good.
    BoundedIntegerNoGood {
        /// 被排除的 assignment / Assignment excluded by this cut family.
        assignment: BTreeMap<StableVariableId, i64>,
        /// cut family 稳定身份 / Stable family identity.
        family_id: String,
        /// 当前行类型 / Row type for this row.
        row: BoundedIntegerNoGoodRow,
    },
}

/// 稳定 master cut / Stable master cut.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct MasterCut {
    /// 稳定 cut ID / Stable cut ID.
    pub id: String,
    /// 稀疏线性系数 / Sparse linear coefficients.
    pub coefficients: BTreeMap<StableVariableId, f64>,
    /// 线性关系方向 / Linear relation sense.
    pub sense: super::CutSense,
    /// 右端项 / Right-hand side.
    pub rhs: f64,
    /// 有效性范围 / Validity scope.
    pub validity: MasterCutValidity,
    /// 证明状态 / Proof status.
    pub proof_status: MasterCutProofStatus,
    /// 证明引用 / Proof reference.
    pub proof_reference: Option<String>,
    /// 来源 / Origin.
    pub origin: String,
    /// 原因 / Reason.
    pub reason: String,
    /// 辅助变量声明 / Auxiliary-variable declarations.
    pub auxiliary_variables: Vec<MasterVariableBinding>,
    /// 表达式编码 / Expression encoding.
    pub encoding: MasterCutEncoding,
}

impl MasterCut {
    /// 创建普通线性 cut / Create an ordinary linear cut.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        coefficients: impl IntoIterator<Item = (StableVariableId, f64)>,
        sense: super::CutSense,
        rhs: f64,
        validity: MasterCutValidity,
        proof_status: MasterCutProofStatus,
        proof_reference: Option<String>,
        origin: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        let mut normalized_coefficients = BTreeMap::new();
        for (id, coefficient) in coefficients {
            *normalized_coefficients.entry(id).or_insert(0.0) += coefficient;
        }
        Self {
            id: id.into(),
            coefficients: normalized_coefficients,
            sense,
            rhs,
            validity,
            proof_status,
            proof_reference,
            origin: origin.into(),
            reason: reason.into(),
            auxiliary_variables: Vec::new(),
            encoding: MasterCutEncoding::Linear,
        }
    }

    /// 创建已验证全局线性 cut / Create a verified global linear cut.
    pub fn verified_global(
        id: impl Into<String>,
        coefficients: impl IntoIterator<Item = (StableVariableId, f64)>,
        sense: super::CutSense,
        rhs: f64,
        proof_reference: impl Into<String>,
        origin: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::new(
            id,
            coefficients,
            sense,
            rhs,
            MasterCutValidity::Global,
            MasterCutProofStatus::Verified,
            Some(proof_reference.into()),
            origin,
            reason,
        )
    }

    /// 规范化 cut / Normalize a cut.
    pub fn normalize(mut self) -> Result<Self> {
        if self.id.trim().is_empty() {
            return Err(invalid("master cut ID must not be empty"));
        }
        if !self.rhs.is_finite() {
            return Err(invalid(format!(
                "master cut {} has a non-finite rhs",
                self.id
            )));
        }
        let mut coefficients = BTreeMap::new();
        for (id, coefficient) in self.coefficients {
            if id.0.trim().is_empty() || !coefficient.is_finite() {
                return Err(invalid(format!(
                    "master cut {} has an invalid coefficient",
                    self.id
                )));
            }
            if coefficient != 0.0 {
                *coefficients.entry(id).or_insert(0.0) += coefficient;
            }
        }
        if coefficients.values().any(|value| !value.is_finite()) {
            return Err(invalid(format!(
                "master cut {} coefficient aggregation overflowed",
                self.id
            )));
        }
        self.coefficients = coefficients
            .into_iter()
            .filter(|(_, coefficient)| *coefficient != 0.0)
            .collect();
        self.proof_reference = self
            .proof_reference
            .take()
            .map(|reference| reference.trim().to_owned())
            .filter(|reference| !reference.is_empty());
        if self.proof_status == MasterCutProofStatus::Verified && self.proof_reference.is_none() {
            return Err(invalid(format!(
                "verified master cut {} must carry a proof reference",
                self.id
            )));
        }
        let mut auxiliary_ids = BTreeSet::new();
        for variable in &self.auxiliary_variables {
            variable.validate()?;
            if !auxiliary_ids.insert(variable.id.clone()) {
                return Err(invalid(format!(
                    "master cut {} declares duplicate auxiliary variable {}",
                    self.id, variable.id.0
                )));
            }
        }
        Ok(self)
    }

    /// 获取规范表达式身份 / Get normalized expression identity.
    pub fn expression_key(&self) -> String {
        let mut key = format!("sense={:?};rhs={:016x};", self.sense, self.rhs.to_bits());
        for (id, coefficient) in &self.coefficients {
            key.push_str(&format!(
                "{}:{}={:016x};",
                id.0.len(),
                id.0,
                coefficient.to_bits()
            ));
        }
        key
    }

    pub(crate) fn validate_variables(
        &self,
        binding: &MasterBinding,
        known_auxiliary: &BTreeMap<StableVariableId, MasterVariableDomain>,
    ) -> Result<()> {
        let own_auxiliary = self
            .auxiliary_variables
            .iter()
            .map(|variable| (variable.id.clone(), variable.domain))
            .collect::<BTreeMap<_, _>>();
        for id in self.coefficients.keys() {
            if !binding.bindings.contains_key(id)
                && !known_auxiliary.contains_key(id)
                && !own_auxiliary.contains_key(id)
            {
                return Err(contract(format!(
                    "master cut {} references unknown stable variable {}",
                    self.id, id.0
                )));
            }
        }
        for variable in &self.auxiliary_variables {
            if binding.bindings.contains_key(&variable.id) {
                return Err(contract(format!(
                    "master cut {} auxiliary variable {} collides with a master binding",
                    self.id, variable.id.0
                )));
            }
        }
        Ok(())
    }

    fn violates_at(&self, assignment: &MasterAssignment, tolerance: f64) -> Result<bool> {
        if !matches!(
            self.encoding,
            MasterCutEncoding::Linear | MasterCutEncoding::BinaryNoGood { .. }
        ) {
            return Err(contract(format!(
                "bounded-integer cut {} must be validated as a complete family",
                self.id
            )));
        }
        let lhs = self
            .coefficients
            .iter()
            .map(|(id, coefficient)| {
                assignment
                    .values
                    .get(id)
                    .copied()
                    .map(|value| *coefficient * value as f64)
                    .ok_or_else(|| {
                        contract(format!(
                            "assignment is missing variable {} for cut {}",
                            id.0, self.id
                        ))
                    })
            })
            .try_fold(0.0, |sum, value| value.map(|value| sum + value))?;
        if !lhs.is_finite() {
            return Err(contract(format!(
                "cut {} evaluation is non-finite",
                self.id
            )));
        }
        Ok(match self.sense {
            super::CutSense::LessOrEqual => lhs > self.rhs + tolerance,
            super::CutSense::GreaterOrEqual => lhs < self.rhs - tolerance,
            super::CutSense::Equal => (lhs - self.rhs).abs() > tolerance,
        })
    }
}

/// 已验证 master conflict 子集 / Verified master conflict subset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedMasterConflict {
    /// conflict 中的稳定 master ID / Stable master IDs in the conflict.
    pub variable_ids: BTreeSet<StableVariableId>,
    /// 产生 conflict 的 subproblem proof 引用 / Subproblem proof reference.
    pub proof_reference: String,
}

impl VerifiedMasterConflict {
    /// 创建 conflict 子集 / Create a conflict subset.
    pub fn new(
        variable_ids: impl IntoIterator<Item = StableVariableId>,
        proof_reference: impl Into<String>,
    ) -> Result<Self> {
        let variable_ids = variable_ids.into_iter().collect::<BTreeSet<_>>();
        let proof_reference = proof_reference.into();
        if variable_ids.is_empty() || proof_reference.trim().is_empty() {
            return Err(invalid(
                "verified master conflict requires members and a proof reference",
            ));
        }
        Ok(Self {
            variable_ids,
            proof_reference,
        })
    }

    /// 从 CP assumption ID 构造映射后的 conflict / Create a mapped conflict from CP assumption IDs.
    pub fn from_assumption_ids(
        assumption_ids: impl IntoIterator<Item = ConstraintProgrammingAssumptionId>,
        mapping: &BTreeMap<ConstraintProgrammingAssumptionId, StableVariableId>,
        proof_reference: impl Into<String>,
    ) -> Result<Self> {
        let mut variable_ids = BTreeSet::new();
        for id in assumption_ids {
            let variable = mapping.get(&id).ok_or_else(|| {
                invalid(format!(
                    "CP assumption {} has no master binding mapping",
                    id.0
                ))
            })?;
            variable_ids.insert(variable.clone());
        }
        Self::new(variable_ids, proof_reference)
    }
}

/// 当前 master 与 CP assumptions 的绑定 / Binding between the current master and CP assumptions.
///
/// 每个 assumption 必须映射到当前 master assignment 中的稳定变量，并且其值关系必须在
/// 当前点成立。/ Every assumption must map to a stable variable in the current master
/// assignment, and its value relation must hold at that point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasterAssignmentAssumptionBinding {
    /// 当前 subproblem 使用的 assumptions / Assumptions used by the current subproblem.
    pub assumptions: Vec<ConstraintProgrammingAssumption>,
    /// assumption 身份到 master 稳定变量的映射 / Mapping from assumption identity to a master stable variable.
    pub master_variables: BTreeMap<ConstraintProgrammingAssumptionId, StableVariableId>,
}

impl MasterAssignmentAssumptionBinding {
    /// 创建并校验 assumption 绑定结构 / Create and validate the assumption binding structure.
    pub fn new(
        assumptions: impl IntoIterator<Item = ConstraintProgrammingAssumption>,
        master_variables: impl IntoIterator<
            Item = (ConstraintProgrammingAssumptionId, StableVariableId),
        >,
    ) -> Result<Self> {
        let assumptions = assumptions.into_iter().collect::<Vec<_>>();
        let mut ids = BTreeSet::new();
        for assumption in &assumptions {
            if !ids.insert(assumption.stable_id()) {
                return Err(invalid("CP master assumption identities must be unique"));
            }
        }
        let mut mapped = BTreeMap::new();
        for (id, variable) in master_variables {
            if mapped.insert(id, variable).is_some() {
                return Err(invalid("CP master assumption mappings must be unique"));
            }
        }
        if mapped.len() != assumptions.len()
            || assumptions
                .iter()
                .any(|assumption| !mapped.contains_key(&assumption.stable_id()))
        {
            return Err(invalid(
                "CP master assumption mappings must cover every assumption exactly once",
            ));
        }
        Ok(Self {
            assumptions,
            master_variables: mapped,
        })
    }

    fn validate(&self, master_assignment: &MasterAssignment) -> Result<()> {
        for assumption in &self.assumptions {
            let master_id = self
                .master_variables
                .get(&assumption.stable_id())
                .ok_or_else(|| contract("CP master assumption mapping is incomplete"))?;
            let master_value = master_assignment
                .values
                .get(master_id)
                .copied()
                .ok_or_else(|| {
                    contract(format!(
                        "CP master assumption references unknown master variable {}",
                        master_id.0
                    ))
                })?;
            let satisfied = match assumption {
                ConstraintProgrammingAssumption::Literal(literal) => {
                    if !matches!(master_value, 0 | 1) {
                        return Err(contract(format!(
                            "Boolean master assumption variable {} is not binary",
                            master_id.0
                        )));
                    }
                    if literal.negated {
                        master_value == 0
                    } else {
                        master_value == 1
                    }
                }
                ConstraintProgrammingAssumption::Equal(_, expected) => master_value == *expected,
                ConstraintProgrammingAssumption::LowerBound(_, lower) => master_value >= *lower,
                ConstraintProgrammingAssumption::UpperBound(_, upper) => master_value <= *upper,
            };
            if !satisfied {
                return Err(contract(format!(
                    "CP assumption {} does not bind the current master assignment",
                    assumption.stable_id()
                )));
            }
        }
        Ok(())
    }
}

/// subproblem 可行 incumbent / Feasible CP subproblem incumbent.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintProgrammingSubproblemIncumbent {
    /// CP 稳定赋值 / Stable CP assignment.
    pub assignment: BTreeMap<StableVariableId, i64>,
    /// 可选 objective / Optional objective.
    pub objective: Option<f64>,
}

/// 已由 immutable CP snapshot 复验的证明标记 / Proof marker for immutable CP snapshot verification.
///
/// 该类型携带私有 snapshot 指纹，调用方不能伪造或改绑证明身份。
/// The fingerprint is private, so callers cannot manufacture or retarget the proof identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedCpSnapshotProof {
    fingerprint: AuditFingerprint,
    assumption_snapshot_fingerprint: AuditFingerprint,
    master_assignment_fingerprint: Option<AuditFingerprint>,
}

/// CP 可行赋值的不可伪造证明 / Unforgeable proof for a feasible CP assignment.
///
/// 这是公共 opaque 类型，但字段保持私有；它只能在 immutable snapshot 复验 assignment 和
/// report 后生成。/ This public opaque type has private fields and is produced only after an
/// immutable snapshot revalidates the assignment and report. A public result field cannot opt
/// into the Exact gate by setting a boolean.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedCpAssignmentProof {
    snapshot_fingerprint: AuditFingerprint,
    assumption_snapshot_fingerprint: AuditFingerprint,
    assignment_fingerprint: AuditFingerprint,
    report_fingerprint: AuditFingerprint,
    master_assignment_fingerprint: Option<AuditFingerprint>,
}

fn validate_verified_cp_assignment_proof(
    report: &SolveReport<i64>,
    assignment: &BTreeMap<StableVariableId, i64>,
    proof: &VerifiedCpAssignmentProof,
) -> Result<()> {
    if report.fingerprints.model.as_ref() != Some(&proof.assumption_snapshot_fingerprint) {
        return Err(contract(
            "verified CP assignment proof is not bound to the assumption-bound report model fingerprint",
        ));
    }
    if proof.snapshot_fingerprint.schema_version.trim().is_empty()
        || proof.snapshot_fingerprint.algorithm.trim().is_empty()
        || proof.snapshot_fingerprint.value.trim().is_empty()
    {
        return Err(contract(
            "verified CP assignment proof has a blank base snapshot fingerprint",
        ));
    }
    if cp_assignment_fingerprint(assignment) != proof.assignment_fingerprint {
        return Err(contract(
            "verified CP assignment proof is not bound to the assignment",
        ));
    }
    if cp_report_fingerprint(report) != proof.report_fingerprint {
        return Err(contract(
            "verified CP assignment proof is not bound to the report",
        ));
    }
    if let Some(master_assignment_fingerprint) = proof.master_assignment_fingerprint.as_ref()
        && master_assignment_fingerprint
            .schema_version
            .trim()
            .is_empty()
    {
        return Err(contract(
            "verified CP assignment proof has a blank master assignment fingerprint",
        ));
    }
    Ok(())
}

/// CP subproblem 的四态结果 / Four-state CP subproblem result.
/// 四态结果直接携带统一报告；该布局避免 Exact gate 丢失终态证据 / Each state carries the unified report so the Exact gate cannot lose terminal evidence.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintProgrammingSubproblemResult {
    /// 结构上可行；Exact 模式需使用 [`Self::feasible_from_snapshot`] / Structurally feasible; Exact mode requires [`Self::feasible_from_snapshot`].
    Feasible {
        /// 统一 CP 报告 / Unified CP report.
        report: SolveReport<i64>,
        /// 已复验的 CP assignment / Verified CP assignment.
        assignment: BTreeMap<StableVariableId, i64>,
        /// 可选 subproblem objective / Optional subproblem objective.
        objective: Option<f64>,
        /// 可选有效下界 / Optional valid bound.
        best_bound: Option<f64>,
        /// 由 immutable snapshot 产生的私有可行性证明 / Private proof produced by immutable snapshot verification.
        assignment_proof: Option<VerifiedCpAssignmentProof>,
    },
    /// 不可行结果；Exact 模式必须由 snapshot 入口创建 / Infeasible result; Exact mode requires the snapshot constructor.
    Infeasible {
        /// 统一 CP 报告 / Unified CP report.
        report: SolveReport<i64>,
        /// 可选 verified master conflict 子集 / Optional verified master conflict subset.
        conflict: Option<VerifiedMasterConflict>,
        /// 仅由 `infeasible_from_snapshot` 生成的证明标记 / Proof marker created only by `infeasible_from_snapshot`.
        snapshot_proof: Option<VerifiedCpSnapshotProof>,
    },
    /// 限制或取消导致未完成 / Incomplete because of a limit or cancellation.
    Incomplete {
        /// 统一 CP 报告 / Unified CP report.
        report: SolveReport<i64>,
        /// 可选已复验 incumbent / Optional verified incumbent.
        incumbent: Option<ConstraintProgrammingSubproblemIncumbent>,
    },
    /// 构建、绑定或 backend 失败 / Build, binding, or backend failure.
    Failed {
        /// 结构化失败 issue / Structured failure issue.
        issue: SolveIssue,
    },
}

impl ConstraintProgrammingSubproblemResult {
    /// 创建可行结果并从报告提取稳定 assignment / Create a feasible result from a report assignment.
    pub fn feasible(report: SolveReport<i64>) -> Result<Self> {
        report.validate()?;
        if report.problem_status != ProblemStatus::Feasible || !report.has_incumbent() {
            return Err(contract(
                "feasible CP subproblem result requires a feasible incumbent report",
            ));
        }
        let assignment = report
            .solution
            .as_ref()
            .map(|solution| solution.stable_values.clone())
            .unwrap_or_default();
        let result = Self::Feasible {
            report,
            assignment,
            objective: None,
            best_bound: None,
            assignment_proof: None,
        };
        result.validate()?;
        Ok(result)
    }

    /// 从 CP 快照创建并独立复验可行结果 / Create and independently verify a feasible result from a CP snapshot.
    pub fn feasible_from_snapshot(
        report: SolveReport<i64>,
        snapshot: &ConstraintProgrammingSnapshot,
        objective: Option<f64>,
        best_bound: Option<f64>,
    ) -> Result<Self> {
        Self::feasible_from_snapshot_internal(report, snapshot, None, None, objective, best_bound)
    }

    /// 从 CP 快照创建并绑定当前 master assignment 的 Exact 可行结果 /
    /// Create an Exact feasible result bound to the current master assignment and CP snapshot.
    pub fn feasible_from_snapshot_for_master(
        report: SolveReport<i64>,
        snapshot: &ConstraintProgrammingSnapshot,
        master_assignment: &MasterAssignment,
        assumption_binding: &MasterAssignmentAssumptionBinding,
        objective: Option<f64>,
        best_bound: Option<f64>,
    ) -> Result<Self> {
        Self::feasible_from_snapshot_internal(
            report,
            snapshot,
            Some(master_assignment),
            Some(assumption_binding),
            objective,
            best_bound,
        )
    }

    fn feasible_from_snapshot_internal(
        report: SolveReport<i64>,
        snapshot: &ConstraintProgrammingSnapshot,
        master_assignment: Option<&MasterAssignment>,
        assumption_binding: Option<&MasterAssignmentAssumptionBinding>,
        objective: Option<f64>,
        best_bound: Option<f64>,
    ) -> Result<Self> {
        report.validate()?;
        snapshot.validate_identity()?;
        if let (Some(master_assignment), Some(assumption_binding)) =
            (master_assignment, assumption_binding)
        {
            assumption_binding.validate(master_assignment)?;
        } else if master_assignment.is_some() || assumption_binding.is_some() {
            return Err(contract(
                "CP master assignment proof requires an assumption binding",
            ));
        }
        let assumption_snapshot = if let Some(assumption_binding) = assumption_binding {
            snapshot_with_assumptions(snapshot, &assumption_binding.assumptions)?
        } else {
            snapshot.clone()
        };
        let assignment = report
            .solution
            .as_ref()
            .ok_or_else(|| contract("feasible CP subproblem result has no solution"))?
            .stable_values
            .clone();
        assumption_snapshot.validate_assignment(&assignment)?;
        validate_snapshot_feasible_payload(
            &report,
            &assumption_snapshot,
            &assignment,
            objective,
            best_bound,
        )?;
        let assignment_proof = VerifiedCpAssignmentProof {
            snapshot_fingerprint: snapshot.fingerprint.clone(),
            assumption_snapshot_fingerprint: assumption_snapshot.fingerprint.clone(),
            assignment_fingerprint: cp_assignment_fingerprint(&assignment),
            report_fingerprint: cp_report_fingerprint(&report),
            master_assignment_fingerprint: master_assignment.map(cp_master_assignment_fingerprint),
        };
        let result = Self::Feasible {
            report,
            assignment,
            objective,
            best_bound,
            assignment_proof: Some(assignment_proof),
        };
        result.validate()?;
        Ok(result)
    }

    /// 创建不可行结果 / Create an infeasible result.
    pub fn infeasible(
        report: SolveReport<i64>,
        conflict: Option<VerifiedMasterConflict>,
    ) -> Result<Self> {
        report.validate()?;
        require_infeasibility_certificate(&report)?;
        Self::infeasible_with_fingerprint(report, conflict, None)
    }

    /// 从 CP 快照创建并绑定不可行结果 / Create an infeasible result bound to a CP snapshot.
    pub fn infeasible_from_snapshot(
        report: SolveReport<i64>,
        snapshot: &ConstraintProgrammingSnapshot,
        conflict: Option<VerifiedMasterConflict>,
    ) -> Result<Self> {
        snapshot.validate_identity()?;
        if report.fingerprints.model.as_ref() != Some(&snapshot.fingerprint) {
            return Err(contract(
                "CP infeasibility report model fingerprint does not match the snapshot",
            ));
        }
        Self::infeasible_with_fingerprint(
            report,
            conflict,
            Some(VerifiedCpSnapshotProof {
                fingerprint: snapshot.fingerprint.clone(),
                assumption_snapshot_fingerprint: snapshot.fingerprint.clone(),
                master_assignment_fingerprint: None,
            }),
        )
    }

    /// 从 CP 快照创建并绑定当前 master assignment 的 Exact 不可行结果 /
    /// Create an Exact infeasible result bound to the current master assignment and CP snapshot.
    pub fn infeasible_from_snapshot_for_master(
        report: SolveReport<i64>,
        snapshot: &ConstraintProgrammingSnapshot,
        master_assignment: &MasterAssignment,
        assumption_binding: &MasterAssignmentAssumptionBinding,
        conflict: Option<VerifiedMasterConflict>,
    ) -> Result<Self> {
        snapshot.validate_identity()?;
        assumption_binding.validate(master_assignment)?;
        let assumption_snapshot =
            snapshot_with_assumptions(snapshot, &assumption_binding.assumptions)?;
        if report.fingerprints.model.as_ref() != Some(&assumption_snapshot.fingerprint) {
            return Err(contract(
                "CP infeasibility report model fingerprint does not match the assumption-bound snapshot",
            ));
        }
        Self::infeasible_with_fingerprint(
            report,
            conflict,
            Some(VerifiedCpSnapshotProof {
                fingerprint: snapshot.fingerprint.clone(),
                assumption_snapshot_fingerprint: assumption_snapshot.fingerprint,
                master_assignment_fingerprint: Some(cp_master_assignment_fingerprint(
                    master_assignment,
                )),
            }),
        )
    }

    fn infeasible_with_fingerprint(
        report: SolveReport<i64>,
        conflict: Option<VerifiedMasterConflict>,
        snapshot_proof: Option<VerifiedCpSnapshotProof>,
    ) -> Result<Self> {
        report.validate()?;
        require_infeasibility_certificate(&report)?;
        if let Some(conflict) = &conflict
            && report
                .proof
                .as_ref()
                .and_then(|proof| proof.reference.as_deref())
                != Some(conflict.proof_reference.as_str())
        {
            return Err(contract(
                "verified CP conflict proof reference does not match the infeasibility report",
            ));
        }
        let result = Self::Infeasible {
            report,
            conflict,
            snapshot_proof,
        };
        result.validate()?;
        Ok(result)
    }

    /// 创建未完成结果 / Create an incomplete result.
    pub fn incomplete(
        report: SolveReport<i64>,
        incumbent: Option<ConstraintProgrammingSubproblemIncumbent>,
    ) -> Result<Self> {
        report.validate()?;
        if matches!(
            report.problem_status,
            ProblemStatus::Infeasible
                | ProblemStatus::Unbounded
                | ProblemStatus::InfeasibleOrUnbounded
        ) || report.is_optimal()
        {
            return Err(contract(
                "incomplete CP subproblem result cannot carry a completed mathematical proof",
            ));
        }
        if let Some(incumbent) = &incumbent {
            if incumbent
                .assignment
                .values()
                .any(|value| *value == i64::MIN)
            {
                return Err(invalid(
                    "CP incomplete incumbent contains an invalid assignment value",
                ));
            }
            if incumbent.objective.is_some_and(|value| !value.is_finite()) {
                return Err(invalid("CP incomplete incumbent objective must be finite"));
            }
        }
        let result = Self::Incomplete { report, incumbent };
        result.validate()?;
        Ok(result)
    }

    /// 创建失败结果 / Create a failed result.
    pub fn failed(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Failed {
            issue: SolveIssue::new(code, message),
        }
    }

    fn report(&self) -> Option<&SolveReport<i64>> {
        match self {
            Self::Feasible { report, .. }
            | Self::Infeasible { report, .. }
            | Self::Incomplete { report, .. } => Some(report),
            Self::Failed { .. } => None,
        }
    }

    pub(crate) fn validate(&self) -> Result<()> {
        match self {
            Self::Feasible {
                report,
                assignment,
                objective,
                best_bound,
                assignment_proof,
            } => {
                validate_feasible_payload(report, assignment, *objective, *best_bound)?;
                if let Some(proof) = assignment_proof {
                    validate_verified_cp_assignment_proof(report, assignment, proof)?;
                }
                Ok(())
            }
            Self::Infeasible {
                report,
                conflict,
                snapshot_proof,
            } => {
                require_infeasibility_certificate(report)?;
                let report_reference = report
                    .proof
                    .as_ref()
                    .and_then(|proof| proof.reference.as_deref());
                if let Some(conflict) = conflict
                    && report_reference != Some(conflict.proof_reference.as_str())
                {
                    return Err(contract(
                        "verified CP conflict is not bound to the infeasibility report",
                    ));
                }
                if let Some(snapshot_proof) = snapshot_proof
                    && report.fingerprints.model.as_ref()
                        != Some(&snapshot_proof.assumption_snapshot_fingerprint)
                {
                    return Err(contract(
                        "verified CP snapshot proof is not bound to its assumption-bound snapshot fingerprint",
                    ));
                }
                if let Some(snapshot_proof) = snapshot_proof
                    && let Some(master_assignment_fingerprint) =
                        snapshot_proof.master_assignment_fingerprint.as_ref()
                    && master_assignment_fingerprint
                        .schema_version
                        .trim()
                        .is_empty()
                {
                    return Err(contract(
                        "verified CP snapshot proof has a blank master assignment fingerprint",
                    ));
                }
                Ok(())
            }
            Self::Incomplete { report, incumbent } => {
                report.validate()?;
                if matches!(
                    report.problem_status,
                    ProblemStatus::Infeasible
                        | ProblemStatus::Unbounded
                        | ProblemStatus::InfeasibleOrUnbounded
                ) || report.is_optimal()
                {
                    return Err(contract(
                        "incomplete CP subproblem result cannot carry a completed mathematical proof",
                    ));
                }
                if let Some(incumbent) = incumbent {
                    if incumbent
                        .assignment
                        .values()
                        .any(|value| *value == i64::MIN)
                    {
                        return Err(invalid(
                            "CP incomplete incumbent contains an invalid assignment value",
                        ));
                    }
                    if incumbent.objective.is_some_and(|value| !value.is_finite()) {
                        return Err(invalid("CP incomplete incumbent objective must be finite"));
                    }
                }
                Ok(())
            }
            Self::Failed { .. } => Ok(()),
        }
    }

    fn has_verified_assignment(&self) -> bool {
        matches!(
            self,
            Self::Feasible {
                assignment_proof: Some(_),
                ..
            }
        )
    }

    fn has_verified_assignment_for_master(&self, master_assignment: &MasterAssignment) -> bool {
        if !self.has_verified_assignment() {
            return false;
        }
        matches!(
            self,
            Self::Feasible {
                assignment_proof:
                    Some(VerifiedCpAssignmentProof {
                        master_assignment_fingerprint: Some(expected),
                        ..
                    }),
                ..
            } if expected == &cp_master_assignment_fingerprint(master_assignment)
        )
    }

    fn has_verified_infeasibility_for_master(&self, master_assignment: &MasterAssignment) -> bool {
        matches!(
            self,
            Self::Infeasible {
                snapshot_proof:
                    Some(VerifiedCpSnapshotProof {
                        master_assignment_fingerprint: Some(expected),
                        ..
                    }),
                ..
            } if expected == &cp_master_assignment_fingerprint(master_assignment)
        )
    }

    pub(crate) fn model_fingerprint(&self) -> Option<&AuditFingerprint> {
        match self {
            Self::Feasible {
                assignment_proof: Some(assignment_proof),
                ..
            } => Some(&assignment_proof.snapshot_fingerprint),
            Self::Infeasible {
                snapshot_proof: Some(snapshot_proof),
                ..
            } => Some(&snapshot_proof.fingerprint),
            Self::Feasible { .. } | Self::Infeasible { .. }
            // 未验证、未完成和失败结果没有完成 CP snapshot 证明，不能参与跨迭代模型身份绑定。
            // Unverified, incomplete, and failed results carry no completed CP snapshot proof
            // and must not bind the model identity across iterations.
            | Self::Incomplete { .. }
            | Self::Failed { .. } => None,
        }
    }
}

/// Logic-Based Benders master 求解器 / Logic-Based Benders master solver.
pub trait LogicBasedBendersMaster {
    /// 在当前 cuts 下求解 master / Solve the master with the current cuts.
    fn solve_master(
        &mut self,
        cuts: &[MasterCut],
        options: &FrameworkSolveOptions,
    ) -> Result<SolveReport<f64>>;

    /// 返回可恢复的 master 身份 / Return the recoverable master identity.
    ///
    /// 普通求解可以不提供该身份；portable checkpoint 恢复会强制要求它，并且每次
    /// master report 都必须携带相同的模型指纹。/ Ordinary solves may omit this identity;
    /// portable checkpoint restore requires it, and every master report must carry the same
    /// model fingerprint.
    fn master_fingerprint(&self) -> Option<&AuditFingerprint> {
        None
    }
}

impl<F> LogicBasedBendersMaster for F
where
    F: FnMut(&[MasterCut], &FrameworkSolveOptions) -> Result<SolveReport<f64>>,
{
    fn solve_master(
        &mut self,
        cuts: &[MasterCut],
        options: &FrameworkSolveOptions,
    ) -> Result<SolveReport<f64>> {
        self(cuts, options)
    }
}

/// 带稳定身份的 LBB master / Logic-Based Benders master with a stable identity.
#[derive(Debug, Clone)]
pub struct IdentifiedMaster<M> {
    master: M,
    fingerprint: AuditFingerprint,
}

impl<M> IdentifiedMaster<M> {
    /// 创建带 master 指纹的包装器 / Create a master wrapper with a stable fingerprint.
    pub fn new(master: M, fingerprint: AuditFingerprint) -> Result<Self> {
        if fingerprint.schema_version.trim().is_empty()
            || fingerprint.algorithm.trim().is_empty()
            || fingerprint.value.trim().is_empty()
        {
            return Err(invalid("master fingerprint cannot contain blank fields"));
        }
        Ok(Self {
            master,
            fingerprint,
        })
    }

    /// 取回被包装的 master / Return the wrapped master.
    pub fn into_inner(self) -> M {
        self.master
    }

    /// 返回 master 指纹 / Return the master fingerprint.
    pub fn fingerprint(&self) -> &AuditFingerprint {
        &self.fingerprint
    }
}

impl<M> LogicBasedBendersMaster for IdentifiedMaster<M>
where
    M: LogicBasedBendersMaster,
{
    fn solve_master(
        &mut self,
        cuts: &[MasterCut],
        options: &FrameworkSolveOptions,
    ) -> Result<SolveReport<f64>> {
        self.master.solve_master(cuts, options)
    }

    fn master_fingerprint(&self) -> Option<&AuditFingerprint> {
        Some(&self.fingerprint)
    }
}

/// CP subproblem 求解器 / CP subproblem solver.
pub trait ConstraintProgrammingSubproblemSolver {
    /// 在 master assignment 下求解 CP subproblem / Solve the CP subproblem for a master assignment.
    fn solve_subproblem(
        &mut self,
        assignment: &MasterAssignment,
        options: &FrameworkSolveOptions,
    ) -> Result<ConstraintProgrammingSubproblemResult>;

    /// 返回可恢复的 subproblem factory 身份 / Return the recoverable subproblem-factory identity.
    ///
    /// 普通求解可以不提供该身份；portable checkpoint 恢复会强制要求它。
    /// Ordinary solves may omit this identity; portable checkpoint restore requires it.
    fn subproblem_factory_fingerprint(&self) -> Option<&AuditFingerprint> {
        None
    }
}

impl<F> ConstraintProgrammingSubproblemSolver for F
where
    F: FnMut(
        &MasterAssignment,
        &FrameworkSolveOptions,
    ) -> Result<ConstraintProgrammingSubproblemResult>,
{
    fn solve_subproblem(
        &mut self,
        assignment: &MasterAssignment,
        options: &FrameworkSolveOptions,
    ) -> Result<ConstraintProgrammingSubproblemResult> {
        self(assignment, options)
    }
}

/// 带稳定身份的 CP subproblem solver / CP subproblem solver with a stable identity.
///
/// 该包装器适用于普通求解；它只保存运行时 solver 的工厂指纹，不承担 checkpoint 重建。
/// Checkpoint 恢复必须使用 `ConstraintProgrammingSubproblemFactory`，由 factory 接收保存的
/// snapshot 并实际创建新的 solver。
///
/// This wrapper is for ordinary solves only. It stores a runtime factory fingerprint but does
/// not rebuild checkpoints. Checkpoint restore must use
/// `ConstraintProgrammingSubproblemFactory`, which receives the saved snapshot and creates a
/// fresh solver.
#[derive(Debug, Clone)]
pub struct IdentifiedSubproblemSolver<S> {
    solver: S,
    fingerprint: AuditFingerprint,
}

/// 基于 CP snapshot 创建全新 subproblem solver 的工厂 / Factory that creates a fresh subproblem solver from a CP snapshot.
#[cfg(feature = "serde")]
pub trait ConstraintProgrammingSubproblemFactory {
    /// 工厂创建的 solver 类型 / Solver type created by the factory.
    type Solver: ConstraintProgrammingSubproblemSolver;

    /// 返回稳定工厂身份 / Return the stable factory identity.
    fn subproblem_factory_fingerprint(&self) -> &AuditFingerprint;

    /// 从保存的 CP snapshot 重建新的 solver/session / Rebuild a fresh solver/session from the saved CP snapshot.
    fn rebuild_subproblem(
        &self,
        snapshot: &ConstraintProgrammingSnapshotArtifact,
    ) -> Result<Self::Solver>;
}

/// 闭包驱动的 snapshot factory / Closure-backed snapshot factory.
#[cfg(feature = "serde")]
pub struct SnapshotSubproblemFactory<F, S> {
    factory: F,
    fingerprint: AuditFingerprint,
    marker: std::marker::PhantomData<fn() -> S>,
}

#[cfg(feature = "serde")]
impl<F, S> SnapshotSubproblemFactory<F, S> {
    /// 创建 snapshot factory / Create a snapshot factory.
    pub fn new(factory: F, fingerprint: AuditFingerprint) -> Result<Self> {
        if fingerprint.schema_version.trim().is_empty()
            || fingerprint.algorithm.trim().is_empty()
            || fingerprint.value.trim().is_empty()
        {
            return Err(invalid(
                "subproblem factory fingerprint cannot contain blank fields",
            ));
        }
        Ok(Self {
            factory,
            fingerprint,
            marker: std::marker::PhantomData,
        })
    }
}

#[cfg(feature = "serde")]
impl<F, S> ConstraintProgrammingSubproblemFactory for SnapshotSubproblemFactory<F, S>
where
    F: Fn(&ConstraintProgrammingSnapshotArtifact) -> Result<S>,
    S: ConstraintProgrammingSubproblemSolver,
{
    type Solver = S;

    fn subproblem_factory_fingerprint(&self) -> &AuditFingerprint {
        &self.fingerprint
    }

    fn rebuild_subproblem(
        &self,
        snapshot: &ConstraintProgrammingSnapshotArtifact,
    ) -> Result<Self::Solver> {
        snapshot.validate()?;
        (self.factory)(snapshot)
    }
}

impl<S> IdentifiedSubproblemSolver<S> {
    /// 创建带身份的 solver / Create an identified solver.
    pub fn new(solver: S, fingerprint: AuditFingerprint) -> Result<Self> {
        if fingerprint.schema_version.trim().is_empty()
            || fingerprint.algorithm.trim().is_empty()
            || fingerprint.value.trim().is_empty()
        {
            return Err(invalid(
                "subproblem factory fingerprint cannot contain blank fields",
            ));
        }
        Ok(Self {
            solver,
            fingerprint,
        })
    }

    /// 取回被包装的 solver / Return the wrapped solver.
    pub fn into_inner(self) -> S {
        self.solver
    }

    /// 返回工厂指纹 / Return the factory fingerprint.
    pub fn fingerprint(&self) -> &AuditFingerprint {
        &self.fingerprint
    }
}

impl<S> ConstraintProgrammingSubproblemSolver for IdentifiedSubproblemSolver<S>
where
    S: ConstraintProgrammingSubproblemSolver,
{
    fn solve_subproblem(
        &mut self,
        assignment: &MasterAssignment,
        options: &FrameworkSolveOptions,
    ) -> Result<ConstraintProgrammingSubproblemResult> {
        self.solver.solve_subproblem(assignment, options)
    }

    fn subproblem_factory_fingerprint(&self) -> Option<&AuditFingerprint> {
        Some(&self.fingerprint)
    }
}

impl<F> LogicBasedBendersCutOracle for F
where
    F: FnMut(&MasterAssignment, &ConstraintProgrammingSubproblemResult) -> Result<Vec<MasterCut>>,
{
    fn cuts_for_infeasible(
        &mut self,
        assignment: &MasterAssignment,
        result: &ConstraintProgrammingSubproblemResult,
    ) -> Result<Vec<MasterCut>> {
        self(assignment, result)
    }
}

/// Logic-Based Benders 割生成器 / Logic-Based Benders cut oracle.
pub trait LogicBasedBendersCutOracle {
    /// 为不可行 subproblem 生成 cuts / Generate cuts for an infeasible subproblem.
    fn cuts_for_infeasible(
        &mut self,
        assignment: &MasterAssignment,
        result: &ConstraintProgrammingSubproblemResult,
    ) -> Result<Vec<MasterCut>>;

    /// 使用当前 subproblem attempt 证明上下文生成 cuts / Generate cuts with the current subproblem proof context.
    fn cuts_for_infeasible_with_context(
        &mut self,
        assignment: &MasterAssignment,
        result: &ConstraintProgrammingSubproblemResult,
        _context: &LogicBasedBendersProofContext,
    ) -> Result<Vec<MasterCut>> {
        self.cuts_for_infeasible(assignment, result)
    }

    /// 为可行 subproblem 生成可选 optimality cuts / Generate optional optimality cuts for a feasible subproblem.
    fn cuts_for_feasible(
        &mut self,
        _assignment: &MasterAssignment,
        _result: &ConstraintProgrammingSubproblemResult,
    ) -> Result<Vec<MasterCut>> {
        Ok(Vec::new())
    }

    /// 使用当前 subproblem attempt 证明上下文生成可行 cuts / Generate feasible cuts with the current proof context.
    fn cuts_for_feasible_with_context(
        &mut self,
        assignment: &MasterAssignment,
        result: &ConstraintProgrammingSubproblemResult,
        _context: &LogicBasedBendersProofContext,
    ) -> Result<Vec<MasterCut>> {
        self.cuts_for_feasible(assignment, result)
    }
}

/// 当前 subproblem attempt 的 proof 绑定 / Proof binding for the current subproblem attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicBasedBendersProofContext {
    /// 稳定 subproblem attempt ID / Stable subproblem attempt ID.
    pub subproblem_attempt_id: String,
    /// backend 报告中的原始 proof reference / Original proof reference in the backend report.
    pub report_proof_reference: Option<String>,
    /// 供 cut 使用的本轮绑定 reference / Attempt-bound reference to be used by cuts.
    pub proof_reference: String,
}

impl LogicBasedBendersProofContext {
    fn for_result(
        result: &ConstraintProgrammingSubproblemResult,
        subproblem_attempt_id: impl Into<String>,
    ) -> Result<Self> {
        let subproblem_attempt_id = subproblem_attempt_id.into();
        if subproblem_attempt_id.trim().is_empty() {
            return Err(invalid("subproblem attempt ID must not be blank"));
        }
        let report = result
            .report()
            .ok_or_else(|| contract("failed CP subproblem has no proof context"))?;
        let report_proof_reference = report
            .proof
            .as_ref()
            .and_then(|proof| proof.reference.clone())
            .filter(|reference| !reference.trim().is_empty());
        let report_component = report_proof_reference
            .clone()
            .unwrap_or_else(|| "verified-report".to_owned());
        Ok(Self {
            proof_reference: format!("{subproblem_attempt_id}::{report_component}"),
            subproblem_attempt_id,
            report_proof_reference,
        })
    }
}

/// Logic-Based Benders 目标评估器 / Logic-Based Benders objective evaluator.
pub trait LogicBasedBendersObjectiveEvaluator {
    /// 计算当前 master/CP assignment 的原问题目标 / Evaluate the original-problem objective.
    fn evaluate(
        &mut self,
        assignment: &MasterAssignment,
        subproblem: &ConstraintProgrammingSubproblemResult,
        master_report: &SolveReport<f64>,
    ) -> Result<f64>;
}

/// 默认 objective evaluator：使用 master objective / Default evaluator using the master objective.
#[derive(Debug, Clone, Copy, Default)]
pub struct MasterObjectiveEvaluator;

impl LogicBasedBendersObjectiveEvaluator for MasterObjectiveEvaluator {
    fn evaluate(
        &mut self,
        _assignment: &MasterAssignment,
        _subproblem: &ConstraintProgrammingSubproblemResult,
        master_report: &SolveReport<f64>,
    ) -> Result<f64> {
        master_report
            .solution
            .as_ref()
            .and_then(|solution| solution.objective_value.or(solution.objective))
            .filter(|value| value.is_finite())
            .ok_or_else(|| contract("master optimal report has no finite objective value"))
    }
}

/// 默认 feasibility no-good oracle / Default feasibility no-good oracle.
#[derive(Debug, Clone)]
pub struct DefaultFeasibilityCutOracle {
    /// 当前 master 的稳定绑定 / Stable bindings of the current master.
    pub binding: MasterBinding,
}

impl DefaultFeasibilityCutOracle {
    /// 创建默认 oracle / Create the default oracle.
    pub fn new(binding: MasterBinding) -> Self {
        Self { binding }
    }
}

impl LogicBasedBendersCutOracle for DefaultFeasibilityCutOracle {
    fn cuts_for_infeasible(
        &mut self,
        assignment: &MasterAssignment,
        result: &ConstraintProgrammingSubproblemResult,
    ) -> Result<Vec<MasterCut>> {
        let (conflict, proof_reference) = match result {
            ConstraintProgrammingSubproblemResult::Infeasible {
                report, conflict, ..
            } => (
                conflict.as_ref().map(|value| &value.variable_ids),
                conflict
                    .as_ref()
                    .map(|value| value.proof_reference.clone())
                    .or_else(|| {
                        report
                            .proof
                            .as_ref()
                            .and_then(|proof| proof.reference.clone())
                    })
                    .ok_or_else(|| {
                        contract("default feasibility cut requires a proof reference")
                    })?,
            ),
            _ => {
                return Err(contract(
                    "default feasibility oracle requires an infeasible subproblem result",
                ));
            }
        };
        let ids = conflict
            .filter(|ids| !ids.is_empty())
            .cloned()
            .unwrap_or_else(|| assignment.values.keys().cloned().collect());
        if ids.iter().any(|id| !assignment.values.contains_key(id)) {
            return Err(contract(
                "verified CP conflict contains a variable outside the current assignment",
            ));
        }
        if ids
            .iter()
            .all(|id| self.binding.domain(id) == Some(MasterVariableDomain::Binary))
        {
            return Ok(vec![binary_assignment_no_good(
                assignment,
                &ids,
                proof_reference,
            )?]);
        }
        bounded_integer_assignment_no_good(assignment, &ids, &self.binding, proof_reference)
    }

    fn cuts_for_infeasible_with_context(
        &mut self,
        assignment: &MasterAssignment,
        result: &ConstraintProgrammingSubproblemResult,
        context: &LogicBasedBendersProofContext,
    ) -> Result<Vec<MasterCut>> {
        let conflict = match result {
            ConstraintProgrammingSubproblemResult::Infeasible { conflict, .. } => {
                conflict.as_ref().map(|value| &value.variable_ids)
            }
            _ => {
                return Err(contract(
                    "default feasibility oracle requires an infeasible subproblem result",
                ));
            }
        };
        let ids = conflict
            .filter(|ids| !ids.is_empty())
            .cloned()
            .unwrap_or_else(|| assignment.values.keys().cloned().collect());
        if ids.iter().any(|id| !assignment.values.contains_key(id)) {
            return Err(contract(
                "verified CP conflict contains a variable outside the current assignment",
            ));
        }
        if ids
            .iter()
            .all(|id| self.binding.domain(id) == Some(MasterVariableDomain::Binary))
        {
            return Ok(vec![binary_assignment_no_good(
                assignment,
                &ids,
                context.proof_reference.clone(),
            )?]);
        }
        bounded_integer_assignment_no_good(
            assignment,
            &ids,
            &self.binding,
            context.proof_reference.clone(),
        )
    }
}

/// Benders proof 模式 / Benders proof mode.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicBasedBendersMode {
    /// 只接受可形成全局精确结论的路径 / Accept only globally certifying paths.
    Exact,
    /// 允许声明性/local cut，并准确降级终态 / Allow claimed/local cuts with honest downgrade.
    Heuristic,
}

/// Logic-Based Benders 引擎 / Logic-Based Benders engine.
#[derive(Debug, Clone)]
pub struct LogicBasedBendersEngine {
    /// 稳定 master 绑定 / Stable master binding.
    pub binding: MasterBinding,
    /// proof 模式 / Proof mode.
    pub mode: LogicBasedBendersMode,
    /// master 值的积分容差 / Master integrality tolerance.
    pub integrality_tolerance: f64,
    /// master 目标方向 / Master objective direction.
    pub objective_category: ObjectiveCategory,
    /// Exact 模式的外部 CP snapshot 身份 / External CP snapshot identity for Exact mode.
    pub expected_cp_snapshot_fingerprint: Option<AuditFingerprint>,
}

/// 可移植恢复所需的运行 primitive / Runtime primitives needed for portable recovery.
#[derive(Debug, Clone)]
pub(crate) struct LogicBasedBendersExecutionSeed {
    /// 已规范化 cuts / Normalized cuts.
    pub cuts: Vec<MasterCut>,
    /// 已声明的辅助变量 / Declared auxiliary variables.
    pub auxiliary: BTreeMap<StableVariableId, MasterVariableDomain>,
    /// 已保存的迭代快照 / Saved iteration snapshots.
    pub snapshots: Vec<SolveIterationSnapshot>,
    /// 已保存的迭代号 / Saved iteration number.
    pub iteration: usize,
    /// 已保存 incumbent / Saved incumbent.
    pub best_feasible: Option<SolveReport<f64>>,
    /// cuts 是否仍满足 exact gate / Whether all cuts still satisfy the exact gate.
    pub all_cuts_exact: bool,
    /// 随执行恢复的 CP snapshot 身份 / CP snapshot identity restored with the execution.
    pub cp_snapshot_fingerprint: Option<AuditFingerprint>,
    /// 随执行恢复的 CP snapshot artifact / CP snapshot artifact restored with the execution.
    #[cfg(feature = "serde")]
    pub cp_snapshot: Option<ConstraintProgrammingSnapshotArtifact>,
    /// 随执行恢复的源 attempt 身份 / Source attempt identity restored with the execution.
    pub source_attempt_id: Option<String>,
    /// 随执行恢复的源 parent 身份 / Source parent identity restored with the execution.
    pub source_parent_attempt_id: Option<String>,
    /// 随执行恢复的源 solver provenance / Source solver provenance restored with the execution.
    pub source_provenance: Option<SolverProvenance>,
    /// 随执行恢复的 master 身份 / Master identity restored with the execution.
    pub master_fingerprint: Option<AuditFingerprint>,
    /// 随执行恢复的 subproblem factory 身份 / Subproblem factory identity restored with the execution.
    pub subproblem_factory_fingerprint: Option<AuditFingerprint>,
}

impl LogicBasedBendersEngine {
    /// 创建 exact 引擎 / Create an exact engine.
    pub fn new(binding: MasterBinding) -> Self {
        Self {
            binding,
            mode: LogicBasedBendersMode::Exact,
            integrality_tolerance: 1e-7,
            objective_category: ObjectiveCategory::Minimum,
            expected_cp_snapshot_fingerprint: None,
        }
    }

    /// 设置 proof 模式 / Set proof mode.
    pub fn with_mode(mut self, mode: LogicBasedBendersMode) -> Self {
        self.mode = mode;
        self
    }

    /// 设置积分容差 / Set integrality tolerance.
    pub fn with_integrality_tolerance(mut self, tolerance: f64) -> Result<Self> {
        if !tolerance.is_finite() || tolerance < 0.0 {
            return Err(invalid(
                "master integrality tolerance must be finite and non-negative",
            ));
        }
        self.integrality_tolerance = tolerance;
        Ok(self)
    }

    /// 设置 master 目标方向 / Set the master objective direction.
    pub fn with_objective_category(mut self, category: ObjectiveCategory) -> Self {
        self.objective_category = category;
        self
    }

    /// 设置 Exact 模式使用的外部 CP snapshot 指纹 /
    /// Set the external CP snapshot fingerprint used by Exact mode.
    pub fn with_expected_cp_snapshot_fingerprint(mut self, fingerprint: AuditFingerprint) -> Self {
        self.expected_cp_snapshot_fingerprint = Some(fingerprint);
        self
    }

    /// 使用默认 no-good oracle 求解 / Solve using the default no-good oracle.
    pub fn solve<M, S>(
        &self,
        master: &mut M,
        subproblem: &mut S,
        options: &FrameworkSolveOptions,
    ) -> Result<SolveReport<f64>>
    where
        M: LogicBasedBendersMaster,
        S: ConstraintProgrammingSubproblemSolver,
    {
        let mut oracle = DefaultFeasibilityCutOracle::new(self.binding.clone());
        self.solve_with_oracle(master, subproblem, &mut oracle, options)
    }

    /// 使用注入 cut oracle 求解 / Solve using an injected cut oracle.
    pub fn solve_with_oracle<M, S, O>(
        &self,
        master: &mut M,
        subproblem: &mut S,
        oracle: &mut O,
        options: &FrameworkSolveOptions,
    ) -> Result<SolveReport<f64>>
    where
        M: LogicBasedBendersMaster,
        S: ConstraintProgrammingSubproblemSolver,
        O: LogicBasedBendersCutOracle,
    {
        let mut objective = MasterObjectiveEvaluator;
        self.solve_with_oracle_and_objective(master, subproblem, oracle, &mut objective, options)
    }

    /// 使用 cut 与 objective oracle 求解 / Solve using cut and objective oracles.
    pub fn solve_with_oracle_and_objective<M, S, O, E>(
        &self,
        master: &mut M,
        subproblem: &mut S,
        oracle: &mut O,
        objective: &mut E,
        options: &FrameworkSolveOptions,
    ) -> Result<SolveReport<f64>>
    where
        M: LogicBasedBendersMaster,
        S: ConstraintProgrammingSubproblemSolver,
        O: LogicBasedBendersCutOracle,
        E: LogicBasedBendersObjectiveEvaluator,
    {
        self.solve_with_oracle_and_objective_from_seed(
            master, subproblem, oracle, objective, options, None,
        )
    }

    #[cfg(feature = "serde")]
    /// 从 portable checkpoint 继续求解 / Resume solving from a portable checkpoint.
    #[allow(clippy::too_many_arguments)]
    pub fn resume_from_checkpoint<M, F, O, E>(
        &self,
        artifact: &super::logic_based_benders_checkpoint::LogicBasedBendersCheckpointArtifact,
        expected: &super::logic_based_benders_checkpoint::LogicBasedBendersResumeIdentity,
        master: &mut M,
        factory: &F,
        oracle: &mut O,
        objective: &mut E,
        options: &FrameworkSolveOptions,
    ) -> Result<SolveReport<f64>>
    where
        M: LogicBasedBendersMaster,
        F: ConstraintProgrammingSubproblemFactory,
        O: LogicBasedBendersCutOracle,
        E: LogicBasedBendersObjectiveEvaluator,
    {
        if factory.subproblem_factory_fingerprint() != &expected.subproblem_factory_fingerprint {
            return Err(contract(
                "checkpoint resume requires a matching snapshot subproblem factory",
            ));
        }
        self.resume_from_checkpoint_with_identity(
            artifact, expected, master, factory, oracle, objective, options,
        )
    }

    #[cfg(feature = "serde")]
    /// 按完整身份从 portable checkpoint 继续求解 / Resume from a portable checkpoint with full identity.
    #[allow(clippy::too_many_arguments)]
    pub fn resume_from_checkpoint_with_identity<M, F, O, E>(
        &self,
        artifact: &super::logic_based_benders_checkpoint::LogicBasedBendersCheckpointArtifact,
        expected: &super::logic_based_benders_checkpoint::LogicBasedBendersResumeIdentity,
        master: &mut M,
        factory: &F,
        oracle: &mut O,
        objective: &mut E,
        options: &FrameworkSolveOptions,
    ) -> Result<SolveReport<f64>>
    where
        M: LogicBasedBendersMaster,
        F: ConstraintProgrammingSubproblemFactory,
        O: LogicBasedBendersCutOracle,
        E: LogicBasedBendersObjectiveEvaluator,
    {
        artifact.validate_resume_identity(expected)?;
        if master.master_fingerprint() != Some(&expected.master_fingerprint) {
            return Err(contract(
                "checkpoint resume requires a matching master identity",
            ));
        }
        if factory.subproblem_factory_fingerprint() != &expected.subproblem_factory_fingerprint {
            return Err(contract(
                "checkpoint resume snapshot factory fingerprint does not match",
            ));
        }
        let snapshot = artifact
            .state
            .cp_snapshot
            .as_ref()
            .ok_or_else(|| contract("checkpoint resume requires a CP snapshot artifact"))?;
        let mut subproblem = factory.rebuild_subproblem(snapshot)?;
        if subproblem.subproblem_factory_fingerprint()
            != Some(factory.subproblem_factory_fingerprint())
        {
            return Err(contract(
                "rebuilt subproblem does not preserve the snapshot factory identity",
            ));
        }
        let state = artifact.rebuild_state()?;
        if state.binding != self.binding
            || state.mode != self.mode
            || state.objective_category != self.objective_category
            || (state.integrality_tolerance - self.integrality_tolerance).abs() > f64::EPSILON
        {
            return Err(invalid(
                "checkpoint engine configuration does not match the requested engine",
            ));
        }
        let seed = state.execution_seed();
        self.solve_with_oracle_and_objective_from_seed(
            master,
            &mut subproblem,
            oracle,
            objective,
            options,
            Some(seed),
        )
    }

    fn solve_with_oracle_and_objective_from_seed<M, S, O, E>(
        &self,
        master: &mut M,
        subproblem: &mut S,
        oracle: &mut O,
        objective: &mut E,
        options: &FrameworkSolveOptions,
        seed: Option<LogicBasedBendersExecutionSeed>,
    ) -> Result<SolveReport<f64>>
    where
        M: LogicBasedBendersMaster,
        S: ConstraintProgrammingSubproblemSolver,
        O: LogicBasedBendersCutOracle,
        E: LogicBasedBendersObjectiveEvaluator,
    {
        if !self.integrality_tolerance.is_finite() || self.integrality_tolerance < 0.0 {
            return Err(invalid(
                "master integrality tolerance must be finite and non-negative",
            ));
        }
        if options.max_iterations == 0 {
            return Err(invalid(
                "Logic-Based Benders max_iterations must be positive",
            ));
        }
        if options.tolerance < 0.0 || !options.tolerance.is_finite() {
            return Err(invalid(
                "Logic-Based Benders tolerance must be finite and non-negative",
            ));
        }
        if let (Some(engine_fingerprint), Some(options_fingerprint)) = (
            self.expected_cp_snapshot_fingerprint.as_ref(),
            options.expected_cp_snapshot_fingerprint.as_ref(),
        ) && engine_fingerprint != options_fingerprint
        {
            return Err(contract(
                "Logic-Based Benders engine and options CP snapshot identities do not match",
            ));
        }
        let solver_name = options.name.as_deref().unwrap_or("logic-based-benders");
        if let Some(handle) = options.cancellation_handle.as_ref()
            && handle.is_cancelled()
        {
            return cancelled_solve_report(
                ospf_rust_core::solver::SolverProvenance {
                    solver_id: solver_name.to_owned(),
                    backend_name: "logic-based-benders".to_owned(),
                    ..ospf_rust_core::solver::SolverProvenance::default()
                },
                handle,
            );
        }

        let started = std::time::Instant::now();
        let seed = seed.unwrap_or_else(|| LogicBasedBendersExecutionSeed {
            cuts: Vec::new(),
            auxiliary: BTreeMap::new(),
            snapshots: Vec::new(),
            iteration: 0,
            best_feasible: None,
            all_cuts_exact: true,
            cp_snapshot_fingerprint: None,
            #[cfg(feature = "serde")]
            cp_snapshot: None,
            source_attempt_id: None,
            source_parent_attempt_id: None,
            source_provenance: None,
            master_fingerprint: None,
            subproblem_factory_fingerprint: None,
        });
        let mut cuts = seed.cuts;
        let mut cut_ids = cuts
            .iter()
            .map(|cut| (cut.id.clone(), cut.expression_key()))
            .collect::<BTreeMap<_, _>>();
        let mut cut_keys = cuts.iter().map(MasterCut::expression_key).collect();
        let mut auxiliary = seed.auxiliary;
        let mut snapshots = seed.snapshots;
        let mut best_feasible = seed.best_feasible;
        let mut all_cuts_exact = seed.all_cuts_exact;
        if let (Some(seed_fingerprint), Some(requested_fingerprint)) = (
            seed.cp_snapshot_fingerprint.as_ref(),
            options.expected_cp_snapshot_fingerprint.as_ref(),
        ) && seed_fingerprint != requested_fingerprint
        {
            return Err(contract(
                "execution seed CP snapshot does not match the requested CP snapshot identity",
            ));
        }
        if let (Some(seed_fingerprint), Some(engine_fingerprint)) = (
            seed.cp_snapshot_fingerprint.as_ref(),
            self.expected_cp_snapshot_fingerprint.as_ref(),
        ) && seed_fingerprint != engine_fingerprint
        {
            return Err(contract(
                "execution seed CP snapshot does not match the engine CP snapshot identity",
            ));
        }
        let mut cp_snapshot_fingerprint = seed
            .cp_snapshot_fingerprint
            .or_else(|| options.expected_cp_snapshot_fingerprint.clone())
            .or_else(|| self.expected_cp_snapshot_fingerprint.clone());
        #[cfg(feature = "serde")]
        let cp_snapshot = seed.cp_snapshot;
        let source_attempt_id = seed.source_attempt_id;
        let source_parent_attempt_id = seed.source_parent_attempt_id;
        let source_provenance = seed.source_provenance;
        let master_fingerprint = seed
            .master_fingerprint
            .or_else(|| master.master_fingerprint().cloned());
        let subproblem_factory_fingerprint = seed.subproblem_factory_fingerprint;

        #[cfg(feature = "serde")]
        if let Some(snapshot) = cp_snapshot.as_ref() {
            snapshot.validate()?;
            if cp_snapshot_fingerprint.as_ref() != Some(&snapshot.snapshot.fingerprint) {
                return Err(contract(
                    "execution seed CP snapshot does not match its fingerprint",
                ));
            }
        }
        if source_attempt_id.is_some() != source_provenance.is_some() {
            return Err(contract(
                "execution seed source attempt and provenance must be restored together",
            ));
        }
        if source_parent_attempt_id.is_some() && source_parent_attempt_id == source_attempt_id {
            return Err(contract(
                "execution seed source parent cannot equal the source attempt",
            ));
        }
        let current_master_fingerprint = master.master_fingerprint().cloned();
        if let Some(expected_master) = master_fingerprint.as_ref()
            && current_master_fingerprint.as_ref() != Some(expected_master)
        {
            return Err(contract(
                "execution seed master fingerprint does not match the master",
            ));
        }
        if let Some(expected_factory) = subproblem_factory_fingerprint.as_ref()
            && subproblem.subproblem_factory_fingerprint() != Some(expected_factory)
        {
            return Err(contract(
                "execution seed subproblem factory fingerprint does not match the solver",
            ));
        }

        for iteration in seed.iteration.saturating_add(1)..=options.max_iterations {
            if let Some(handle) = options.cancellation_handle.as_ref()
                && handle.is_cancelled()
            {
                return self.finish_cancelled(
                    best_feasible,
                    snapshots,
                    cuts.len(),
                    handle,
                    solver_name,
                    started,
                    options,
                );
            }
            let iteration_progress = ospf_rust_core::solver::ProgressValue::known(
                (iteration.saturating_sub(1) as f64 / options.max_iterations as f64) * 100.0,
            )?;
            emit_combinatorial_progress(
                options,
                options.name.as_deref().unwrap_or("logic-based-benders"),
                SolveStage::Combinatorial,
                vec![
                    "logic-based-benders".to_owned(),
                    format!("iteration/{iteration}"),
                    "master".to_owned(),
                ],
                ProgressValue::known(0.0)?,
                iteration_progress,
                started.elapsed(),
                None,
                None,
                None,
                false,
            )?;
            let master_attempt_id = format!("{solver_name}/master/{iteration}");
            let master_report = match master.solve_master(&cuts, options) {
                Ok(report) => report,
                Err(error) => {
                    return self.finish_backend_failure(
                        best_feasible.as_ref(),
                        snapshots,
                        cuts.len(),
                        started,
                        options,
                        solver_name,
                        SolveIssue::new("LogicBasedBendersMasterFailed", error.to_string()),
                    );
                }
            };
            master_report.validate()?;
            validate_master_report_identity(&master_report, master_fingerprint.as_ref())?;
            if !master_report.is_optimal() {
                let mut report = master_report;
                if let Some(previous) = best_feasible {
                    report.problem_status = ProblemStatus::Feasible;
                    report.solution = previous.solution;
                } else if report.problem_status == ProblemStatus::Feasible {
                    report.problem_status = ProblemStatus::Unknown;
                    report.solution = None;
                }
                report.proof = None;
                report.statistics.best_bound = None;
                report.statistics.best_bound_value = None;
                report.statistics.absolute_gap = None;
                report.statistics.relative_gap = None;
                report.solution_presence = if report.solution.is_some() {
                    ospf_rust_core::solver::SolutionPresence::Incumbent
                } else {
                    ospf_rust_core::solver::SolutionPresence::None
                };
                report.diagnostics.issues.push(SolveIssue::new(
                    "LogicBasedBendersMasterNotOptimal",
                    "subproblem was not called because the master report was not a verified optimum",
                ));
                snapshots.push(master_snapshot(
                    iteration,
                    &report,
                    &master_attempt_id,
                    None,
                    cuts.len(),
                    cuts.len(),
                    false,
                    None,
                    None,
                ));
                return self.finish_report(report, snapshots, cuts.len(), started, options);
            }
            let assignment = match MasterAssignment::from_report(
                &master_report,
                &self.binding,
                self.integrality_tolerance,
            ) {
                Ok(assignment) => assignment,
                Err(error) => {
                    return self.finish_backend_failure(
                        best_feasible.as_ref(),
                        snapshots,
                        cuts.len(),
                        started,
                        options,
                        solver_name,
                        SolveIssue::new("LogicBasedBendersMasterBindingFailed", error.to_string()),
                    );
                }
            };
            if let Some(handle) = options.cancellation_handle.as_ref()
                && handle.is_cancelled()
            {
                return self.finish_cancelled(
                    best_feasible,
                    snapshots,
                    cuts.len(),
                    handle,
                    solver_name,
                    started,
                    options,
                );
            }
            if self.mode == LogicBasedBendersMode::Exact && cp_snapshot_fingerprint.is_none() {
                return self.finish_report(
                    self.downgrade_report(
                        master_report,
                        TerminationReason::Suboptimal,
                        "LogicBasedBendersCpSnapshotIdentityMissing",
                    )?,
                    snapshots,
                    cuts.len(),
                    started,
                    options,
                );
            }
            let master_objective = master_report
                .solution
                .as_ref()
                .and_then(|solution| solution.objective_value.or(solution.objective));
            emit_combinatorial_progress(
                options,
                options.name.as_deref().unwrap_or("logic-based-benders"),
                SolveStage::Combinatorial,
                vec![
                    "logic-based-benders".to_owned(),
                    format!("iteration/{iteration}"),
                    "subproblem".to_owned(),
                ],
                ProgressValue::known(50.0)?,
                iteration_progress,
                started.elapsed(),
                master_objective,
                master_report
                    .statistics
                    .best_bound_value
                    .or(master_report.statistics.best_bound),
                master_report.statistics.relative_gap,
                false,
            )?;
            let sub_attempt_id = format!("{solver_name}/subproblem/{iteration}");
            let sub_result = match subproblem.solve_subproblem(&assignment, options) {
                Ok(result) => result,
                Err(error) => {
                    return self.finish_backend_failure(
                        best_feasible.as_ref(),
                        snapshots,
                        cuts.len(),
                        started,
                        options,
                        solver_name,
                        SolveIssue::new("LogicBasedBendersSubproblemFailed", error.to_string()),
                    );
                }
            };
            sub_result.validate()?;
            let result_fingerprint = sub_result.model_fingerprint().cloned();
            if let Some(result_fingerprint) = result_fingerprint.as_ref() {
                if let Some(expected) = cp_snapshot_fingerprint.as_ref()
                    && result_fingerprint != expected
                {
                    return Err(contract(
                        "CP subproblem result does not match the checkpoint CP snapshot",
                    ));
                }
                if self.mode != LogicBasedBendersMode::Exact && cp_snapshot_fingerprint.is_none() {
                    cp_snapshot_fingerprint = Some(result_fingerprint.clone());
                }
            }
            let proof_context = match &sub_result {
                ConstraintProgrammingSubproblemResult::Feasible { .. }
                | ConstraintProgrammingSubproblemResult::Infeasible { .. } => Some(
                    LogicBasedBendersProofContext::for_result(&sub_result, sub_attempt_id.clone())?,
                ),
                ConstraintProgrammingSubproblemResult::Incomplete { .. }
                | ConstraintProgrammingSubproblemResult::Failed { .. } => None,
            };
            let newly_added;
            let mut proof_reference = proof_context
                .as_ref()
                .map(|context| context.proof_reference.clone());
            match &sub_result {
                ConstraintProgrammingSubproblemResult::Feasible { report, .. } => {
                    let proof_context = proof_context
                        .as_ref()
                        .ok_or_else(|| contract("feasible subproblem has no proof context"))?;
                    if self.mode == LogicBasedBendersMode::Exact
                        && !sub_result.has_verified_assignment_for_master(&assignment)
                    {
                        return self.finish_report(
                            self.downgrade_report(
                                master_report,
                                TerminationReason::Suboptimal,
                                "LogicBasedBendersSubproblemAssignmentNotVerified",
                            )?,
                            snapshots,
                            cuts.len(),
                            started,
                            options,
                        );
                    }
                    if self.mode == LogicBasedBendersMode::Exact && !report.is_optimal() {
                        return self.finish_report(
                            self.downgrade_report(
                                master_report,
                                TerminationReason::Suboptimal,
                                "LogicBasedBendersSubproblemNotCertified",
                            )?,
                            snapshots,
                            cuts.len(),
                            started,
                            options,
                        );
                    }
                    let objective_value =
                        objective.evaluate(&assignment, &sub_result, &master_report)?;
                    if !objective_value.is_finite() {
                        return Err(contract(
                            "Logic-Based Benders objective evaluator returned a non-finite value",
                        ));
                    }
                    let master_objective = master_report
                        .solution
                        .as_ref()
                        .and_then(|solution| solution.objective_value.or(solution.objective));
                    let objective_preserves_master_bound = master_objective
                        .is_some_and(|value| approximately_equal(value, objective_value, 1e-9));
                    let mut feasible_report = master_report.clone();
                    if let Some(solution) = feasible_report.solution.as_mut() {
                        solution.objective = Some(objective_value);
                        solution.objective_value = Some(objective_value);
                    }
                    if !objective_preserves_master_bound {
                        // A custom evaluator changes the objective semantics. The master bound and
                        // its optimality proof cannot be reused without an explicit recourse bound.
                        // 自定义 evaluator 改变目标语义时，不得复用 master bound 或最优性证明。
                        feasible_report.proof = None;
                        feasible_report.statistics.best_bound = None;
                        feasible_report.statistics.best_bound_value = None;
                        feasible_report.statistics.absolute_gap = None;
                        feasible_report.statistics.relative_gap = None;
                        feasible_report.solution_presence =
                            ospf_rust_core::solver::SolutionPresence::Incumbent;
                        all_cuts_exact = false;
                    }
                    feasible_report.validate()?;
                    best_feasible = Some(feasible_report);
                    let generated = oracle.cuts_for_feasible_with_context(
                        &assignment,
                        &sub_result,
                        proof_context,
                    )?;
                    newly_added = self.normalize_and_validate_cuts(
                        generated,
                        &assignment,
                        false,
                        proof_reference.as_deref(),
                        &mut cut_ids,
                        &mut cut_keys,
                        &mut auxiliary,
                    )?;
                    if newly_added.iter().any(|cut| {
                        cut.validity != MasterCutValidity::Global
                            || cut.proof_status != MasterCutProofStatus::Verified
                    }) {
                        all_cuts_exact = false;
                    }
                }
                ConstraintProgrammingSubproblemResult::Infeasible { report, .. } => {
                    let proof_context = proof_context
                        .as_ref()
                        .ok_or_else(|| contract("infeasible subproblem has no proof context"))?;
                    if self.mode == LogicBasedBendersMode::Exact
                        && !sub_result.has_verified_infeasibility_for_master(&assignment)
                    {
                        return self.finish_report(
                            self.downgrade_report(
                                master_report,
                                TerminationReason::Suboptimal,
                                "LogicBasedBendersInfeasibilityNotVerified",
                            )?,
                            snapshots,
                            cuts.len(),
                            started,
                            options,
                        );
                    }
                    require_infeasibility_certificate(report)?;
                    if let Some(expected) = cp_snapshot_fingerprint.as_ref()
                        && sub_result.model_fingerprint() != Some(expected)
                    {
                        return Err(contract(
                            "verified CP infeasibility proof does not match the current CP snapshot",
                        ));
                    }
                    let generated = oracle.cuts_for_infeasible_with_context(
                        &assignment,
                        &sub_result,
                        proof_context,
                    )?;
                    if generated.is_empty() {
                        return failure_report(
                            TerminationReason::BackendFailure,
                            SolveIssue::new(
                                "LogicBasedBendersNoCut",
                                "verified infeasible subproblem produced no feasibility cut",
                            ),
                            solver_name,
                        );
                    }
                    newly_added = self.normalize_and_validate_cuts(
                        generated,
                        &assignment,
                        true,
                        proof_reference.as_deref(),
                        &mut cut_ids,
                        &mut cut_keys,
                        &mut auxiliary,
                    )?;
                    if newly_added.is_empty() {
                        return failure_report(
                            TerminationReason::BackendFailure,
                            SolveIssue::new(
                                "LogicBasedBendersDuplicateCut",
                                "verified infeasible subproblem produced no new cut",
                            ),
                            solver_name,
                        );
                    }
                    if newly_added.iter().any(|cut| {
                        cut.validity != MasterCutValidity::Global
                            || cut.proof_status != MasterCutProofStatus::Verified
                    }) {
                        all_cuts_exact = false;
                    }
                    proof_reference = Some(proof_context.proof_reference.clone());
                }
                ConstraintProgrammingSubproblemResult::Incomplete { report, .. } => {
                    let reason = report.termination_reason;
                    if reason == TerminationReason::Cancelled
                        && let Some(handle) = options.cancellation_handle.as_ref()
                    {
                        return self.finish_cancelled(
                            best_feasible,
                            snapshots,
                            cuts.len(),
                            handle,
                            solver_name,
                            started,
                            options,
                        );
                    }
                    let final_report = if let Some(previous) = best_feasible {
                        self.downgrade_report_with_source(
                            previous,
                            reason,
                            "LogicBasedBendersSubproblemIncomplete",
                            Some(report),
                        )?
                    } else {
                        failure_report(
                            reason,
                            SolveIssue::new(
                                "LogicBasedBendersSubproblemIncomplete",
                                "subproblem did not produce a verified feasibility conclusion",
                            ),
                            solver_name,
                        )?
                    };
                    return self.finish_report(
                        final_report,
                        snapshots,
                        cuts.len(),
                        started,
                        options,
                    );
                }
                ConstraintProgrammingSubproblemResult::Failed { issue } => {
                    return self.finish_backend_failure(
                        best_feasible.as_ref(),
                        snapshots,
                        cuts.len(),
                        started,
                        options,
                        solver_name,
                        issue.clone(),
                    );
                }
            }
            let total_cuts = cuts.len().saturating_add(newly_added.len());
            cuts.extend(newly_added.iter().cloned());
            emit_combinatorial_progress(
                options,
                options.name.as_deref().unwrap_or("logic-based-benders"),
                SolveStage::Combinatorial,
                vec![
                    "logic-based-benders".to_owned(),
                    format!("iteration/{iteration}"),
                    "cuts".to_owned(),
                ],
                ProgressValue::known(100.0)?,
                ProgressValue::known((iteration as f64 / options.max_iterations as f64) * 100.0)?,
                started.elapsed(),
                master_objective,
                master_report
                    .statistics
                    .best_bound_value
                    .or(master_report.statistics.best_bound),
                master_report.statistics.relative_gap,
                false,
            )?;
            snapshots.push(master_snapshot(
                iteration,
                &master_report,
                &master_attempt_id,
                Some(&sub_attempt_id),
                newly_added.len(),
                total_cuts,
                all_cuts_exact,
                proof_reference,
                sub_result.report(),
            ));
            if newly_added.is_empty() {
                let mut report = best_feasible.unwrap_or(master_report);
                if self.mode == LogicBasedBendersMode::Heuristic || !all_cuts_exact {
                    report.proof = None;
                    report.solution_presence = if report.solution.is_some() {
                        ospf_rust_core::solver::SolutionPresence::Incumbent
                    } else {
                        ospf_rust_core::solver::SolutionPresence::None
                    };
                    report.diagnostics.issues.push(SolveIssue::new(
                        "LogicBasedBendersHeuristicConclusion",
                        "the result contains a feasible incumbent but the exact global proof gate was not closed",
                    ));
                }
                return self.finish_report(report, snapshots, cuts.len(), started, options);
            }
        }

        let report = best_feasible
            .map(|report| {
                self.downgrade_report(
                    report,
                    TerminationReason::IterationLimit,
                    "LogicBasedBendersIterationLimit",
                )
            })
            .transpose()?
            .unwrap_or(failure_report(
                TerminationReason::IterationLimit,
                SolveIssue::new(
                    "LogicBasedBendersIterationLimit",
                    "iteration limit reached before a verified feasible master assignment was found",
                ),
                solver_name,
            )?);
        self.finish_report(report, snapshots, cuts.len(), started, options)
    }

    #[allow(clippy::too_many_arguments)]
    fn normalize_and_validate_cuts(
        &self,
        generated: Vec<MasterCut>,
        assignment: &MasterAssignment,
        infeasibility_cut: bool,
        expected_proof_reference: Option<&str>,
        cut_ids: &mut BTreeMap<String, String>,
        cut_keys: &mut BTreeSet<String>,
        auxiliary: &mut BTreeMap<StableVariableId, MasterVariableDomain>,
    ) -> Result<Vec<MasterCut>> {
        let mut normalized = Vec::new();
        let mut batch_auxiliary = auxiliary.clone();
        for cut in generated {
            let cut = cut.normalize()?;
            cut.validate_variables(&self.binding, &batch_auxiliary)?;
            if self.mode == LogicBasedBendersMode::Exact
                && (cut.validity != MasterCutValidity::Global
                    || cut.proof_status != MasterCutProofStatus::Verified)
            {
                return Err(contract(format!(
                    "exact Logic-Based Benders rejected non-global or unverified cut {}",
                    cut.id
                )));
            }
            if cut.proof_status == MasterCutProofStatus::Verified
                && (expected_proof_reference.is_none()
                    || cut.proof_reference.as_deref() != expected_proof_reference)
            {
                return Err(contract(format!(
                    "verified master cut {} is not bound to the current subproblem proof",
                    cut.id
                )));
            }
            for variable in &cut.auxiliary_variables {
                let previous = batch_auxiliary.insert(variable.id.clone(), variable.domain);
                if previous.is_some_and(|domain| domain != variable.domain) {
                    return Err(contract(format!(
                        "auxiliary variable {} changed domain across cuts",
                        variable.id.0
                    )));
                }
            }
            if infeasibility_cut {
                match &cut.encoding {
                    MasterCutEncoding::BinaryNoGood {
                        assignment: excluded,
                    } => {
                        if excluded != &assignment.values
                            && !excluded
                                .iter()
                                .all(|(id, value)| assignment.values.get(id) == Some(value))
                        {
                            return Err(contract(format!(
                                "feasibility cut {} does not identify the current assignment",
                                cut.id
                            )));
                        }
                        if !cut.violates_at(assignment, 1e-9)? {
                            return Err(contract(format!(
                                "feasibility cut {} does not exclude the current assignment",
                                cut.id
                            )));
                        }
                    }
                    MasterCutEncoding::BoundedIntegerNoGood { .. } => {}
                    MasterCutEncoding::Linear => {
                        if !cut.violates_at(assignment, 1e-9)? {
                            return Err(contract(format!(
                                "feasibility cut {} does not exclude the current assignment",
                                cut.id
                            )));
                        }
                    }
                }
            }
            let expression_key = cut.expression_key();
            if let Some(previous) = cut_ids.get(&cut.id)
                && previous != &expression_key
            {
                return Err(contract(format!(
                    "stable master cut ID {} was reused for a different expression",
                    cut.id
                )));
            }
            cut_ids.insert(cut.id.clone(), expression_key.clone());
            if cut_keys.insert(expression_key) {
                normalized.push(cut);
            }
        }
        *auxiliary = batch_auxiliary;
        if infeasibility_cut {
            validate_bounded_integer_families(&normalized, assignment, &self.binding)?;
        }
        Ok(normalized)
    }

    fn downgrade_report(
        &self,
        mut report: SolveReport<f64>,
        termination: TerminationReason,
        issue_code: &str,
    ) -> Result<SolveReport<f64>> {
        report.termination_reason = termination;
        report.proof = None;
        report.statistics.best_bound = None;
        report.statistics.best_bound_value = None;
        report.statistics.absolute_gap = None;
        report.statistics.relative_gap = None;
        report.solution_presence = if report.solution.is_some() {
            ospf_rust_core::solver::SolutionPresence::Incumbent
        } else {
            ospf_rust_core::solver::SolutionPresence::None
        };
        report.diagnostics.issues.push(SolveIssue::new(
            issue_code,
            "Logic-Based Benders retained only the incumbent because the exact proof gate was not closed",
        ));
        report.validate()?;
        Ok(report)
    }

    fn downgrade_report_with_source(
        &self,
        mut report: SolveReport<f64>,
        termination: TerminationReason,
        issue_code: &str,
        source: Option<&SolveReport<i64>>,
    ) -> Result<SolveReport<f64>> {
        if let Some(source) = source {
            for key in ["cancellation.origin", "cancellation.requestedAtEpochMs"] {
                if let Some(value) = source.diagnostics.extensions.get(key) {
                    report
                        .diagnostics
                        .extensions
                        .insert(key.to_owned(), value.clone());
                }
            }
        }
        self.downgrade_report(report, termination, issue_code)
    }

    #[allow(clippy::too_many_arguments)]
    fn finish_backend_failure(
        &self,
        incumbent: Option<&SolveReport<f64>>,
        snapshots: Vec<SolveIterationSnapshot>,
        cut_count: usize,
        started: std::time::Instant,
        options: &FrameworkSolveOptions,
        solver_name: &str,
        issue: SolveIssue,
    ) -> Result<SolveReport<f64>> {
        let mut report = if let Some(incumbent) = incumbent {
            self.downgrade_report(
                incumbent.clone(),
                TerminationReason::BackendFailure,
                "LogicBasedBendersBackendFailureWithIncumbent",
            )?
        } else {
            failure_report(
                TerminationReason::BackendFailure,
                issue.clone(),
                solver_name,
            )?
        };
        if incumbent.is_some() {
            report.diagnostics.issues.push(issue);
            report.validate()?;
        }
        self.finish_report(report, snapshots, cut_count, started, options)
    }

    #[allow(clippy::too_many_arguments)]
    fn finish_cancelled(
        &self,
        best_feasible: Option<SolveReport<f64>>,
        snapshots: Vec<SolveIterationSnapshot>,
        cut_count: usize,
        handle: &SolveHandle,
        solver_name: &str,
        started: std::time::Instant,
        options: &FrameworkSolveOptions,
    ) -> Result<SolveReport<f64>> {
        let reason = TerminationReason::Cancelled;
        let mut report = if let Some(report) = best_feasible {
            self.downgrade_report(report, reason, "LogicBasedBendersCancelled")?
        } else {
            cancelled_solve_report(
                ospf_rust_core::solver::SolverProvenance {
                    solver_id: solver_name.to_owned(),
                    backend_name: "logic-based-benders".to_owned(),
                    ..ospf_rust_core::solver::SolverProvenance::default()
                },
                handle,
            )?
        };
        if let Some(cancellation) = handle.cancellation() {
            report.diagnostics.extensions.insert(
                "cancellation.origin".to_owned(),
                cancellation.origin.to_string(),
            );
            report.diagnostics.extensions.insert(
                "cancellation.requestedAtEpochMs".to_owned(),
                cancellation.requested_at_epoch_ms.to_string(),
            );
        }
        self.finish_report(report, snapshots, cut_count, started, options)
    }

    fn finish_report(
        &self,
        mut report: SolveReport<f64>,
        snapshots: Vec<SolveIterationSnapshot>,
        cut_count: usize,
        started: std::time::Instant,
        options: &FrameworkSolveOptions,
    ) -> Result<SolveReport<f64>> {
        let objective_value = report
            .solution
            .as_ref()
            .and_then(|solution| solution.objective_value);
        let reported_bound = report
            .statistics
            .best_bound_value
            .or(report.statistics.best_bound);
        let valid_bound = objective_value
            .zip(reported_bound)
            .is_some_and(|(objective, bound)| {
                if self.objective_category.is_minimum() {
                    bound <= objective + 1e-9
                } else {
                    bound + 1e-9 >= objective
                }
            });
        if reported_bound.is_some() && !valid_bound {
            report.statistics.best_bound = None;
            report.statistics.best_bound_value = None;
            report.statistics.absolute_gap = None;
            report.statistics.relative_gap = None;
            if report.solution.is_some() {
                report.solution_presence = ospf_rust_core::solver::SolutionPresence::Incumbent;
            }
            report.proof = None;
            report.diagnostics.issues.push(SolveIssue::new(
                "LogicBasedBendersInvalidBound",
                "the master bound failed the configured objective-direction check and was discarded",
            ));
        }
        let bound = valid_bound.then_some(reported_bound).flatten();
        report.statistics.best_bound_value = bound;
        let absolute_gap = objective_value
            .zip(bound)
            .map(|(objective, bound)| (objective - bound).abs());
        report.statistics.absolute_gap = absolute_gap;
        report.statistics.relative_gap = absolute_gap
            .zip(objective_value)
            .map(|(gap, objective)| gap / objective.abs().max(1.0));
        let (global_lower_bound, upper_bound) = if self.objective_category.is_minimum() {
            (bound, objective_value)
        } else {
            (objective_value, bound)
        };
        let relative_gap = absolute_gap
            .zip(objective_value)
            .map(|(gap, objective)| gap / objective.abs().max(1.0));
        report.trace = SolveTrace {
            global_lower_bound,
            upper_bound,
            relative_gap,
            total_iterations: snapshots.len(),
            generated_columns: cut_count,
            elapsed: started.elapsed(),
            iteration_snapshots: snapshots,
            ..SolveTrace::default()
        };
        report.statistics.iterations = Some(report.trace.total_iterations);
        report
            .statistics
            .extensions
            .insert("algorithm".to_owned(), "logic-based-benders".to_owned());
        let progress = ospf_rust_core::solver::ProgressValue::known(100.0)?;
        emit_combinatorial_progress(
            options,
            options.name.as_deref().unwrap_or("logic-based-benders"),
            SolveStage::Completed,
            vec!["logic-based-benders".to_owned(), "completed".to_owned()],
            progress,
            progress,
            started.elapsed(),
            upper_bound,
            global_lower_bound,
            relative_gap,
            true,
        )?;
        report.validate()?;
        Ok(report)
    }
}

/// 创建 binary assignment no-good / Create a binary assignment no-good cut.
pub fn binary_assignment_no_good(
    assignment: &MasterAssignment,
    variable_ids: &BTreeSet<StableVariableId>,
    proof_reference: impl Into<String>,
) -> Result<MasterCut> {
    if variable_ids.is_empty() {
        return Err(invalid("binary no-good requires at least one variable"));
    }
    let proof_reference = proof_reference.into();
    let mut coefficients = BTreeMap::new();
    let mut ones = 0i64;
    for id in variable_ids {
        let value = assignment.values.get(id).copied().ok_or_else(|| {
            invalid(format!(
                "binary no-good references unknown assignment variable {}",
                id.0
            ))
        })?;
        if !(0..=1).contains(&value) {
            return Err(invalid(format!(
                "binary no-good variable {} has non-binary value {}",
                id.0, value
            )));
        }
        if value == 1 {
            ones = ones.saturating_add(1);
            coefficients.insert(id.clone(), -1.0);
        } else {
            coefficients.insert(id.clone(), 1.0);
        }
    }
    let key = assignment.canonical_key(variable_ids.iter().cloned());
    let mut cut = MasterCut::verified_global(
        format!("logic/no-good/{key}"),
        coefficients,
        super::CutSense::GreaterOrEqual,
        1.0 - ones as f64,
        proof_reference,
        "verified-assignment-no-good",
        "exclude the verified infeasible binary master assignment",
    );
    cut.encoding = MasterCutEncoding::BinaryNoGood {
        assignment: variable_ids
            .iter()
            .filter_map(|id| {
                assignment
                    .values
                    .get(id)
                    .copied()
                    .map(|value| (id.clone(), value))
            })
            .collect(),
    };
    cut.normalize()
}

/// 创建有界整数 assignment no-good / Create a bounded-integer assignment no-good family.
pub fn bounded_integer_assignment_no_good(
    assignment: &MasterAssignment,
    variable_ids: &BTreeSet<StableVariableId>,
    binding: &MasterBinding,
    proof_reference: impl Into<String>,
) -> Result<Vec<MasterCut>> {
    if variable_ids.is_empty() {
        return Err(invalid(
            "bounded-integer no-good requires at least one variable",
        ));
    }
    let proof_reference = proof_reference.into();
    if proof_reference.trim().is_empty() {
        return Err(invalid(
            "bounded-integer no-good requires a proof reference",
        ));
    }
    let family_id = format!(
        "logic/int-no-good/{}",
        assignment.canonical_key(variable_ids.iter().cloned())
    );
    // 先固定整个 family 的 auxiliary 集合，再生成每一行，确保任意一行都声明完整 formulation。
    // Freeze the complete family auxiliary set before creating rows so every row declares the full formulation.
    let mut auxiliary = Vec::with_capacity(variable_ids.len().saturating_mul(2));
    for id in variable_ids {
        let lower_id = bounded_auxiliary_id(&family_id, id, "lower");
        let upper_id = bounded_auxiliary_id(&family_id, id, "upper");
        auxiliary.push(MasterVariableBinding::binary(lower_id.clone()));
        auxiliary.push(MasterVariableBinding::binary(upper_id.clone()));
    }

    let mut cuts = Vec::new();
    for id in variable_ids {
        let domain = binding
            .domain(id)
            .ok_or_else(|| invalid(format!("unknown bounded-integer binding {}", id.0)))?;
        let (lower, upper) = match domain {
            MasterVariableDomain::Binary => (0, 1),
            MasterVariableDomain::Integer { lower, upper } => (lower, upper),
        };
        let value = assignment
            .values
            .get(id)
            .copied()
            .ok_or_else(|| invalid(format!("assignment is missing variable {}", id.0)))?;
        if !domain.contains(value) {
            return Err(invalid(format!(
                "assignment value is outside domain for {}",
                id.0
            )));
        }
        let lower_id = bounded_auxiliary_id(&family_id, id, "lower");
        let upper_id = bounded_auxiliary_id(&family_id, id, "upper");
        let m_upper = exact_f64_integer(
            upper as i128 - (value as i128 - 1),
            &format!("no-good upper big-M for {}", id.0),
        )?;
        let m_lower = exact_f64_integer(
            (value as i128 + 1) - lower as i128,
            &format!("no-good lower big-M for {}", id.0),
        )?;
        let value_f64 = exact_f64_integer(value as i128, &format!("no-good value for {}", id.0))?;
        let mut lower_side = MasterCut::verified_global(
            format!("{family_id}/lower/{}", id.0),
            [(id.clone(), 1.0), (lower_id.clone(), m_upper)],
            super::CutSense::LessOrEqual,
            value_f64 - 1.0 + m_upper,
            proof_reference.clone(),
            "bounded-integer-assignment-no-good",
            "select the lower side of a non-equal integer assignment",
        );
        lower_side.auxiliary_variables = auxiliary.clone();
        lower_side.encoding = MasterCutEncoding::BoundedIntegerNoGood {
            assignment: assignment
                .values
                .iter()
                .filter(|(candidate, _)| variable_ids.contains(*candidate))
                .map(|(candidate, value)| (candidate.clone(), *value))
                .collect(),
            family_id: family_id.clone(),
            row: BoundedIntegerNoGoodRow::LowerSide(id.clone()),
        };
        cuts.push(lower_side.normalize()?);

        let mut upper_side = MasterCut::verified_global(
            format!("{family_id}/upper/{}", id.0),
            [(id.clone(), 1.0), (upper_id.clone(), -m_lower)],
            super::CutSense::GreaterOrEqual,
            value_f64 + 1.0 - m_lower,
            proof_reference.clone(),
            "bounded-integer-assignment-no-good",
            "select the upper side of a non-equal integer assignment",
        );
        upper_side.auxiliary_variables = auxiliary.clone();
        upper_side.encoding = MasterCutEncoding::BoundedIntegerNoGood {
            assignment: assignment
                .values
                .iter()
                .filter(|(candidate, _)| variable_ids.contains(*candidate))
                .map(|(candidate, value)| (candidate.clone(), *value))
                .collect(),
            family_id: family_id.clone(),
            row: BoundedIntegerNoGoodRow::UpperSide(id.clone()),
        };
        cuts.push(upper_side.normalize()?);

        let mut at_most_one = MasterCut::verified_global(
            format!("{family_id}/at-most-one/{}", id.0),
            [(lower_id, 1.0), (upper_id, 1.0)],
            super::CutSense::LessOrEqual,
            1.0,
            proof_reference.clone(),
            "bounded-integer-assignment-no-good",
            "a variable cannot select both non-equal sides",
        );
        at_most_one.auxiliary_variables = auxiliary.clone();
        at_most_one.encoding = MasterCutEncoding::BoundedIntegerNoGood {
            assignment: assignment
                .values
                .iter()
                .filter(|(candidate, _)| variable_ids.contains(*candidate))
                .map(|(candidate, value)| (candidate.clone(), *value))
                .collect(),
            family_id: family_id.clone(),
            row: BoundedIntegerNoGoodRow::AtMostOne(id.clone()),
        };
        cuts.push(at_most_one.normalize()?);
    }
    let coefficients = auxiliary
        .iter()
        .map(|variable| (variable.id.clone(), 1.0))
        .collect::<Vec<_>>();
    let mut require_difference = MasterCut::verified_global(
        format!("{family_id}/require-difference"),
        coefficients,
        super::CutSense::GreaterOrEqual,
        1.0,
        proof_reference,
        "bounded-integer-assignment-no-good",
        "at least one master integer must differ from the excluded assignment",
    );
    require_difference.auxiliary_variables = auxiliary;
    require_difference.encoding = MasterCutEncoding::BoundedIntegerNoGood {
        assignment: assignment
            .values
            .iter()
            .filter(|(candidate, _)| variable_ids.contains(*candidate))
            .map(|(candidate, value)| (candidate.clone(), *value))
            .collect(),
        family_id,
        row: BoundedIntegerNoGoodRow::RequireDifference,
    };
    cuts.push(require_difference.normalize()?);
    Ok(cuts)
}

pub(crate) fn validate_bounded_integer_families(
    cuts: &[MasterCut],
    assignment: &MasterAssignment,
    binding: &MasterBinding,
) -> Result<()> {
    let mut families = BTreeMap::<String, Vec<&MasterCut>>::new();
    for cut in cuts {
        if let MasterCutEncoding::BoundedIntegerNoGood {
            family_id,
            assignment: excluded,
            ..
        } = &cut.encoding
        {
            if !excluded
                .iter()
                .all(|(id, value)| assignment.values.get(id) == Some(value))
            {
                return Err(contract(format!(
                    "bounded-integer cut family {} does not identify the current assignment",
                    family_id
                )));
            }
            families.entry(family_id.clone()).or_default().push(cut);
        }
    }
    for (family_id, family) in families {
        let assignment = match &family[0].encoding {
            MasterCutEncoding::BoundedIntegerNoGood { assignment, .. } => assignment,
            _ => unreachable!("family contains only bounded-integer rows"),
        };
        if assignment.is_empty() {
            return Err(contract(format!(
                "bounded-integer cut family {family_id} cannot exclude an empty assignment family"
            )));
        }
        if family.iter().any(|cut| {
            !matches!(
                &cut.encoding,
                MasterCutEncoding::BoundedIntegerNoGood {
                    family_id: candidate_family,
                    assignment: candidate_assignment,
                    ..
                } if candidate_family == &family_id && candidate_assignment == assignment
            )
        }) {
            return Err(contract(format!(
                "bounded-integer cut family {family_id} mixes family identities or assignments"
            )));
        }
        let expected_rows = assignment.len().saturating_mul(3).saturating_add(1);
        if family.len() != expected_rows {
            return Err(contract(format!(
                "bounded-integer cut family {} has {} rows, expected {}",
                family_id,
                family.len(),
                expected_rows
            )));
        }
        let mut row_counts = BTreeMap::<String, usize>::new();
        for cut in &family {
            let row_key = match &cut.encoding {
                MasterCutEncoding::BoundedIntegerNoGood { row, .. } => match row {
                    BoundedIntegerNoGoodRow::LowerSide(id) => format!("lower:{}", id.0),
                    BoundedIntegerNoGoodRow::UpperSide(id) => format!("upper:{}", id.0),
                    BoundedIntegerNoGoodRow::AtMostOne(id) => format!("at-most-one:{}", id.0),
                    BoundedIntegerNoGoodRow::RequireDifference => "require-difference".to_owned(),
                },
                _ => unreachable!("family contains only bounded-integer rows"),
            };
            *row_counts.entry(row_key).or_default() += 1;
        }
        for id in assignment.keys() {
            for row in [
                format!("lower:{}", id.0),
                format!("upper:{}", id.0),
                format!("at-most-one:{}", id.0),
            ] {
                if row_counts.get(&row) != Some(&1) {
                    return Err(contract(format!(
                        "bounded-integer cut family {family_id} has an invalid row multiplicity for {}",
                        id.0
                    )));
                }
            }
        }
        if row_counts.get("require-difference") != Some(&1) {
            return Err(contract(format!(
                "bounded-integer cut family {family_id} has an invalid require-difference row multiplicity"
            )));
        }
        let declared_auxiliary = family[0]
            .auxiliary_variables
            .iter()
            .map(|variable| (variable.id.clone(), variable.domain))
            .collect::<BTreeMap<_, _>>();
        let expected_auxiliary = assignment
            .keys()
            .flat_map(|id| {
                [
                    (
                        bounded_auxiliary_id(&family_id, id, "lower"),
                        MasterVariableDomain::Binary,
                    ),
                    (
                        bounded_auxiliary_id(&family_id, id, "upper"),
                        MasterVariableDomain::Binary,
                    ),
                ]
            })
            .collect::<BTreeMap<_, _>>();
        if declared_auxiliary != expected_auxiliary
            || family.iter().any(|cut| {
                cut.auxiliary_variables
                    .iter()
                    .map(|variable| (variable.id.clone(), variable.domain))
                    .collect::<BTreeMap<_, _>>()
                    != expected_auxiliary
            })
        {
            return Err(contract(format!(
                "bounded-integer cut family {family_id} does not declare its complete auxiliary set"
            )));
        }
        for id in assignment.keys() {
            let domain = binding
                .domain(id)
                .ok_or_else(|| contract(format!("bounded family references unknown {}", id.0)))?;
            let (lower, upper) = match domain {
                MasterVariableDomain::Binary => (0, 1),
                MasterVariableDomain::Integer { lower, upper } => (lower, upper),
            };
            let value = assignment[id];
            let family_prefix = family_id.as_str();
            let lower_id = bounded_auxiliary_id(family_prefix, id, "lower");
            let upper_id = bounded_auxiliary_id(family_prefix, id, "upper");
            let has_lower = family.iter().any(|cut| {
                matches!(
                    &cut.encoding,
                    MasterCutEncoding::BoundedIntegerNoGood {
                        row: BoundedIntegerNoGoodRow::LowerSide(candidate), ..
                    } if candidate == id
                ) && cut.sense == super::CutSense::LessOrEqual
                    && cut.coefficients.len() == 2
                    && cut.coefficients.get(id) == Some(&1.0)
                    && cut.coefficients.get(&lower_id).is_some_and(|coefficient| {
                        approximately_equal(
                            *coefficient,
                            (upper as i128 - (value as i128 - 1)) as f64,
                            1e-12,
                        )
                    })
                    && approximately_equal(
                        cut.rhs,
                        value as f64 - 1.0 + (upper as i128 - (value as i128 - 1)) as f64,
                        1e-12,
                    )
            });
            let has_upper = family.iter().any(|cut| {
                matches!(
                    &cut.encoding,
                    MasterCutEncoding::BoundedIntegerNoGood {
                        row: BoundedIntegerNoGoodRow::UpperSide(candidate), ..
                    } if candidate == id
                ) && cut.sense == super::CutSense::GreaterOrEqual
                    && cut.coefficients.len() == 2
                    && cut.coefficients.get(id) == Some(&1.0)
                    && cut.coefficients.get(&upper_id).is_some_and(|coefficient| {
                        approximately_equal(
                            *coefficient,
                            -((value as i128 + 1) - lower as i128) as f64,
                            1e-12,
                        )
                    })
                    && approximately_equal(
                        cut.rhs,
                        value as f64 + 1.0 - ((value as i128 + 1) - lower as i128) as f64,
                        1e-12,
                    )
            });
            let has_at_most_one = family.iter().any(|cut| {
                matches!(
                    &cut.encoding,
                    MasterCutEncoding::BoundedIntegerNoGood {
                        row: BoundedIntegerNoGoodRow::AtMostOne(candidate), ..
                    } if candidate == id
                ) && cut.sense == super::CutSense::LessOrEqual
                    && cut.rhs == 1.0
                    && cut.coefficients.len() == 2
                    && cut.coefficients.get(&lower_id) == Some(&1.0)
                    && cut.coefficients.get(&upper_id) == Some(&1.0)
            });
            if !(has_lower && has_upper && has_at_most_one) {
                return Err(contract(format!(
                    "bounded-integer cut family {} is malformed for {}",
                    family_id, id.0
                )));
            }
        }
        let require_difference = family.iter().any(|cut| {
            matches!(
                &cut.encoding,
                MasterCutEncoding::BoundedIntegerNoGood {
                    row: BoundedIntegerNoGoodRow::RequireDifference,
                    ..
                }
            ) && cut.sense == super::CutSense::GreaterOrEqual
                && cut.rhs == 1.0
                && cut.coefficients.keys().cloned().collect::<BTreeSet<_>>()
                    == expected_auxiliary.keys().cloned().collect::<BTreeSet<_>>()
                && cut
                    .coefficients
                    .values()
                    .all(|coefficient| *coefficient == 1.0)
        });
        if !require_difference {
            return Err(contract(format!(
                "bounded-integer cut family {} has no require-difference row",
                family_id
            )));
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn master_snapshot(
    iteration: usize,
    report: &SolveReport<f64>,
    master_attempt_id: &str,
    subproblem_attempt_id: Option<&str>,
    cuts_added: usize,
    items_total: usize,
    bound_valid: bool,
    proof_reference: Option<String>,
    sub_report: Option<&SolveReport<i64>>,
) -> SolveIterationSnapshot {
    let objective_value = report
        .solution
        .as_ref()
        .and_then(|solution| solution.objective_value);
    let best_bound = bound_valid
        .then_some(report.statistics.best_bound_value)
        .flatten();
    let relative_gap = objective_value
        .zip(best_bound)
        .map(|(objective, bound)| (objective - bound).abs() / objective.abs().max(1.0));
    SolveIterationSnapshot {
        iteration,
        stage: "logic-based-benders/master-subproblem".to_owned(),
        objective_value,
        best_bound,
        relative_gap,
        items_added: cuts_added,
        items_total,
        proof_reference,
        master_attempt_id: Some(master_attempt_id.to_owned()),
        subproblem_attempt_id: subproblem_attempt_id.map(str::to_owned),
        master_problem_status: Some(report.problem_status),
        master_termination_reason: Some(report.termination_reason),
        subproblem_problem_status: sub_report.map(|report| report.problem_status),
        subproblem_termination_reason: sub_report.map(|report| report.termination_reason),
        bound_valid: Some(bound_valid),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ospf_rust_core::model::constraint_programming::{
        ConstraintProgrammingModel, IntegerDomain, IntegerExpression, IntegerObjective,
        IntegerTerm, IntegerVariable,
    };
    use ospf_rust_core::solver::{
        CancellationOrigin, ProblemStatus, SolveFingerprints, SolveProof, SolveSolution,
        SolveStatistics, SolverProvenance, TerminationReason,
    };

    fn binding_binary() -> MasterBinding {
        MasterBinding::new([
            MasterVariableBinding::binary("x"),
            MasterVariableBinding::binary("y"),
        ])
        .expect("binding")
    }

    fn master_report(values: &[(StableVariableId, f64)], objective: f64) -> SolveReport<f64> {
        let stable_values = values.iter().cloned().collect::<BTreeMap<_, _>>();
        SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed)
            .solution(SolveSolution {
                values: values.iter().map(|(_, value)| *value).collect(),
                stable_values,
                objective: Some(objective),
                objective_value: Some(objective),
                ..SolveSolution::vector(Vec::new())
            })
            .proof(SolveProof::optimality())
            .provenance(SolverProvenance {
                solver_id: "fake-master".to_owned(),
                backend_name: "fake".to_owned(),
                ..SolverProvenance::default()
            })
            .build()
            .expect("master report")
    }

    fn cp_test_snapshot() -> ConstraintProgrammingSnapshot {
        let variable = IntegerVariable::new("z");
        let mut model = ConstraintProgrammingModel::new("lbb-test-subproblem");
        model
            .register_variable(variable, IntegerDomain::boolean())
            .expect("CP variable");
        model.freeze().expect("CP snapshot")
    }

    fn exact_engine(binding: MasterBinding) -> LogicBasedBendersEngine {
        LogicBasedBendersEngine::new(binding)
            .with_expected_cp_snapshot_fingerprint(cp_test_snapshot().fingerprint)
    }

    fn cp_feasible_report(values: &[(&str, i64)]) -> SolveReport<i64> {
        let stable_values = values
            .iter()
            .map(|(id, value)| (StableVariableId::from(*id), *value))
            .collect();
        SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed)
            .solution(SolveSolution {
                stable_values,
                ..SolveSolution::vector(Vec::new())
            })
            .proof(SolveProof::optimality())
            .build()
            .expect("CP report")
    }

    fn cp_feasible_report_for_snapshot(
        values: &[(&str, i64)],
        snapshot: &ConstraintProgrammingSnapshot,
    ) -> SolveReport<i64> {
        let mut report = cp_feasible_report(values);
        report.fingerprints.model = Some(snapshot.fingerprint.clone());
        report
            .validate()
            .expect("CP report with snapshot fingerprint");
        report
    }

    fn cp_master_assumption_binding(
        assignment: &MasterAssignment,
        variable_id: &str,
    ) -> MasterAssignmentAssumptionBinding {
        let value = assignment
            .get(&StableVariableId::from("x"))
            .expect("master x assignment");
        let variable = IntegerVariable::new(variable_id);
        MasterAssignmentAssumptionBinding::new(
            [ConstraintProgrammingAssumption::Equal(
                variable.clone(),
                value,
            )],
            [(
                ConstraintProgrammingAssumption::Equal(variable, value).stable_id(),
                StableVariableId::from("x"),
            )],
        )
        .expect("master assumption binding")
    }

    fn cp_feasible_result_for_master(
        assignment: &MasterAssignment,
        snapshot: &ConstraintProgrammingSnapshot,
        variable_id: &str,
    ) -> Result<ConstraintProgrammingSubproblemResult> {
        let value = assignment
            .get(&StableVariableId::from("x"))
            .ok_or_else(|| contract("master x assignment is missing"))?;
        let binding = cp_master_assumption_binding(assignment, variable_id);
        let assumption_snapshot = snapshot_with_assumptions(snapshot, &binding.assumptions)?;
        ConstraintProgrammingSubproblemResult::feasible_from_snapshot_for_master(
            cp_feasible_report_for_snapshot(&[(variable_id, value)], &assumption_snapshot),
            snapshot,
            assignment,
            &binding,
            None,
            None,
        )
    }

    fn objective_snapshot(
        category: ospf_rust_core::model::ObjectiveCategory,
    ) -> ConstraintProgrammingSnapshot {
        let variable = IntegerVariable::new("z");
        let mut model = ConstraintProgrammingModel::new("lbb-objective-subproblem");
        model
            .register_variable(variable.clone(), IntegerDomain::boolean())
            .expect("CP variable");
        model.set_objective(IntegerObjective {
            category,
            expression: IntegerExpression::linear(
                2,
                [IntegerTerm {
                    variable,
                    coefficient: 3,
                }],
            )
            .expect("objective expression"),
        });
        model.freeze().expect("CP snapshot")
    }

    fn cp_objective_report(
        snapshot: &ConstraintProgrammingSnapshot,
        objective: i64,
        best_bound: Option<i64>,
    ) -> SolveReport<i64> {
        let mut report =
            SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed)
                .solution(SolveSolution {
                    stable_values: [(StableVariableId::from("z"), 1)].into_iter().collect(),
                    objective: Some(objective),
                    objective_value: Some(objective as f64),
                    ..SolveSolution::vector(Vec::new())
                })
                .proof(SolveProof::optimality())
                .fingerprints(SolveFingerprints {
                    model: Some(snapshot.fingerprint.clone()),
                    ..SolveFingerprints::default()
                })
                .build()
                .expect("CP objective report");
        if let Some(best_bound) = best_bound {
            report.statistics = SolveStatistics {
                best_bound: Some(best_bound),
                best_bound_value: Some(best_bound as f64),
                absolute_gap: Some((objective - best_bound).abs() as f64),
                relative_gap: Some(
                    (objective - best_bound).abs() as f64 / (objective.abs().max(1) as f64),
                ),
                ..SolveStatistics::default()
            };
            report.validate().expect("CP objective report with bound");
        }
        report
    }

    fn cp_infeasible_result(
        master_assignment: &MasterAssignment,
    ) -> ConstraintProgrammingSubproblemResult {
        let snapshot = cp_test_snapshot();
        let mut report =
            SolveReport::builder(ProblemStatus::Infeasible, TerminationReason::Completed)
                .proof(SolveProof::infeasibility())
                .fingerprints(SolveFingerprints {
                    model: Some(snapshot.fingerprint.clone()),
                    ..SolveFingerprints::default()
                })
                .build()
                .expect("CP infeasible report");
        report
            .proof
            .as_mut()
            .expect("infeasibility proof")
            .reference = Some("fake-cp-infeasibility".to_owned());
        let binding = cp_master_assumption_binding(master_assignment, "z");
        let assumption_snapshot = snapshot_with_assumptions(&snapshot, &binding.assumptions)
            .expect("master assumption snapshot");
        report.fingerprints.model = Some(assumption_snapshot.fingerprint);
        ConstraintProgrammingSubproblemResult::infeasible_from_snapshot_for_master(
            report,
            &snapshot,
            master_assignment,
            &binding,
            None,
        )
        .expect("verified CP infeasibility result")
    }

    fn cp_unverified_infeasible_result() -> ConstraintProgrammingSubproblemResult {
        let foreign_snapshot = cp_other_snapshot();
        let report = SolveReport::builder(ProblemStatus::Infeasible, TerminationReason::Completed)
            .proof(SolveProof::infeasibility())
            .fingerprints(SolveFingerprints {
                model: Some(foreign_snapshot.fingerprint),
                ..SolveFingerprints::default()
            })
            .build()
            .expect("CP infeasible report");
        ConstraintProgrammingSubproblemResult::infeasible(report, None)
            .expect("structurally valid CP infeasible result")
    }

    fn cp_other_snapshot() -> ConstraintProgrammingSnapshot {
        let variable = IntegerVariable::new("other-z");
        let mut model = ConstraintProgrammingModel::new("other-lbb-test-subproblem");
        model
            .register_variable(variable, IntegerDomain::boolean())
            .expect("CP variable");
        model.freeze().expect("other CP snapshot")
    }

    fn solve_bounded_integer_engine_with_mutation(
        mutate: impl Fn(&mut MasterCut),
    ) -> Result<SolveReport<f64>> {
        let binding = MasterBinding::new([MasterVariableBinding::integer("x", 0, 3)])?;
        let engine = exact_engine(binding.clone());
        let mut master = |_cuts: &[MasterCut], _options: &FrameworkSolveOptions| {
            Ok(master_report(&[(StableVariableId::from("x"), 1.0)], 1.0))
        };
        let mut subproblem = |assignment: &MasterAssignment, _options: &FrameworkSolveOptions| {
            Ok(cp_infeasible_result(assignment))
        };
        let mut oracle =
            |assignment: &MasterAssignment, _result: &ConstraintProgrammingSubproblemResult| {
                let ids = binding.ids().cloned().collect::<BTreeSet<_>>();
                let mut cuts = bounded_integer_assignment_no_good(
                    assignment,
                    &ids,
                    &binding,
                    "logic-based-benders/subproblem/1::fake-cp-infeasibility",
                )?;
                for cut in &mut cuts {
                    if matches!(
                        cut.encoding,
                        MasterCutEncoding::BoundedIntegerNoGood {
                            row: BoundedIntegerNoGoodRow::AtMostOne(_),
                            ..
                        }
                    ) {
                        mutate(cut);
                    }
                }
                Ok(cuts)
            };
        engine.solve_with_oracle(
            &mut master,
            &mut subproblem,
            &mut oracle,
            &FrameworkSolveOptions::default(),
        )
    }

    #[test]
    fn binary_no_good_excludes_only_the_selected_assignment() {
        let binding = binding_binary();
        let assignment = MasterAssignment::new(
            [
                (StableVariableId::from("x"), 1),
                (StableVariableId::from("y"), 0),
            ]
            .into_iter()
            .collect(),
            &binding,
        )
        .expect("assignment");
        let ids = binding.ids().cloned().collect::<BTreeSet<_>>();
        let cut = binary_assignment_no_good(&assignment, &ids, "cp/proof").expect("cut");
        assert!(cut.violates_at(&assignment, 1e-9).expect("evaluation"));
        let other = MasterAssignment::new(
            [
                (StableVariableId::from("x"), 1),
                (StableVariableId::from("y"), 1),
            ]
            .into_iter()
            .collect(),
            &binding,
        )
        .expect("other assignment");
        assert!(!cut.violates_at(&other, 1e-9).expect("evaluation"));
    }

    #[test]
    fn bounded_integer_no_good_has_stable_auxiliaries_and_exact_family_shape() {
        let binding =
            MasterBinding::new([MasterVariableBinding::integer("x", 0, 3)]).expect("binding");
        let assignment = MasterAssignment::new(
            [(StableVariableId::from("x"), 2)].into_iter().collect(),
            &binding,
        )
        .expect("assignment");
        let ids = [StableVariableId::from("x")].into_iter().collect();
        let cuts = bounded_integer_assignment_no_good(&assignment, &ids, &binding, "cp/proof")
            .expect("cuts");
        assert_eq!(cuts.len(), 4);
        assert!(cuts.iter().all(|cut| !cut.auxiliary_variables.is_empty()));
        validate_bounded_integer_families(&cuts, &assignment, &binding).expect("family");
    }

    #[test]
    fn bounded_integer_no_good_declares_the_complete_auxiliary_set_on_every_row() {
        let binding = MasterBinding::new([
            MasterVariableBinding::integer("x", 0, 3),
            MasterVariableBinding::integer("y", -2, 2),
        ])
        .expect("binding");
        let assignment = MasterAssignment::new(
            [
                (StableVariableId::from("x"), 1),
                (StableVariableId::from("y"), 0),
            ]
            .into_iter()
            .collect(),
            &binding,
        )
        .expect("assignment");
        let ids = binding.ids().cloned().collect::<BTreeSet<_>>();
        let cuts = bounded_integer_assignment_no_good(&assignment, &ids, &binding, "cp/proof")
            .expect("cuts");
        let auxiliary = cuts[0]
            .auxiliary_variables
            .iter()
            .map(|variable| variable.id.clone())
            .collect::<BTreeSet<_>>();
        assert_eq!(auxiliary.len(), 4);
        assert!(cuts.iter().all(|cut| {
            cut.auxiliary_variables
                .iter()
                .map(|variable| variable.id.clone())
                .collect::<BTreeSet<_>>()
                == auxiliary
        }));
        validate_bounded_integer_families(&cuts, &assignment, &binding).expect("family");
    }

    #[test]
    fn bounded_integer_exact_gate_rejects_malformed_at_most_one_rows() {
        let binding =
            MasterBinding::new([MasterVariableBinding::integer("x", 0, 3)]).expect("binding");
        let assignment = MasterAssignment::new(
            [(StableVariableId::from("x"), 2)].into_iter().collect(),
            &binding,
        )
        .expect("assignment");
        let ids = [StableVariableId::from("x")].into_iter().collect();
        let mut cuts = bounded_integer_assignment_no_good(&assignment, &ids, &binding, "cp/proof")
            .expect("cuts");
        let at_most_one = cuts
            .iter_mut()
            .find(|cut| {
                matches!(
                    cut.encoding,
                    MasterCutEncoding::BoundedIntegerNoGood {
                        row: BoundedIntegerNoGoodRow::AtMostOne(_),
                        ..
                    }
                )
            })
            .expect("at-most-one row");
        at_most_one.rhs = 2.0;
        assert!(validate_bounded_integer_families(&cuts, &assignment, &binding).is_err());

        let mut cuts = bounded_integer_assignment_no_good(&assignment, &ids, &binding, "cp/proof")
            .expect("cuts");
        let at_most_one = cuts
            .iter_mut()
            .find(|cut| {
                matches!(
                    cut.encoding,
                    MasterCutEncoding::BoundedIntegerNoGood {
                        row: BoundedIntegerNoGoodRow::AtMostOne(_),
                        ..
                    }
                )
            })
            .expect("at-most-one row");
        at_most_one
            .coefficients
            .insert(StableVariableId::from("unexpected"), 1.0);
        assert!(validate_bounded_integer_families(&cuts, &assignment, &binding).is_err());
    }

    #[test]
    fn bounded_integer_exact_gate_rejects_an_empty_assignment_family() {
        let binding =
            MasterBinding::new([MasterVariableBinding::integer("x", 0, 3)]).expect("binding");
        let assignment = MasterAssignment::new(
            [(StableVariableId::from("x"), 2)].into_iter().collect(),
            &binding,
        )
        .expect("assignment");
        let ids = [StableVariableId::from("x")].into_iter().collect();
        let mut cuts = bounded_integer_assignment_no_good(&assignment, &ids, &binding, "cp/proof")
            .expect("cuts");
        for cut in &mut cuts {
            if let MasterCutEncoding::BoundedIntegerNoGood { assignment, .. } = &mut cut.encoding {
                assignment.clear();
            }
        }
        assert!(validate_bounded_integer_families(&cuts, &assignment, &binding).is_err());
    }

    #[test]
    fn public_exact_engine_rejects_bounded_integer_at_most_one_rhs_mutation() {
        let result = solve_bounded_integer_engine_with_mutation(|cut| {
            cut.rhs = 2.0;
        });
        assert!(result.is_err());
    }

    #[test]
    fn public_exact_engine_rejects_bounded_integer_at_most_one_extra_coefficient() {
        let result = solve_bounded_integer_engine_with_mutation(|cut| {
            cut.coefficients.insert(StableVariableId::from("x"), 1.0);
        });
        assert!(result.is_err());
    }

    #[test]
    fn bounded_integer_exact_gate_rejects_malformed_side_and_difference_rows() {
        let binding = MasterBinding::new([
            MasterVariableBinding::integer("x", 0, 3),
            MasterVariableBinding::integer("y", 0, 2),
        ])
        .expect("binding");
        let assignment = MasterAssignment::new(
            [
                (StableVariableId::from("x"), 2),
                (StableVariableId::from("y"), 1),
            ]
            .into_iter()
            .collect(),
            &binding,
        )
        .expect("assignment");
        let ids = binding.ids().cloned().collect::<BTreeSet<_>>();

        let mut lower = bounded_integer_assignment_no_good(&assignment, &ids, &binding, "cp/proof")
            .expect("cuts");
        lower
            .iter_mut()
            .find(|cut| {
                matches!(
                    cut.encoding,
                    MasterCutEncoding::BoundedIntegerNoGood {
                        row: BoundedIntegerNoGoodRow::LowerSide(_),
                        ..
                    }
                )
            })
            .expect("lower row")
            .coefficients
            .insert(StableVariableId::from("unexpected"), 1.0);
        assert!(validate_bounded_integer_families(&lower, &assignment, &binding).is_err());

        let mut upper = bounded_integer_assignment_no_good(&assignment, &ids, &binding, "cp/proof")
            .expect("cuts");
        upper
            .iter_mut()
            .find(|cut| {
                matches!(
                    cut.encoding,
                    MasterCutEncoding::BoundedIntegerNoGood {
                        row: BoundedIntegerNoGoodRow::UpperSide(_),
                        ..
                    }
                )
            })
            .expect("upper row")
            .coefficients
            .insert(StableVariableId::from("unexpected"), 1.0);
        assert!(validate_bounded_integer_families(&upper, &assignment, &binding).is_err());

        let mut difference =
            bounded_integer_assignment_no_good(&assignment, &ids, &binding, "cp/proof")
                .expect("cuts");
        let difference_row = difference
            .iter_mut()
            .find(|cut| {
                matches!(
                    cut.encoding,
                    MasterCutEncoding::BoundedIntegerNoGood {
                        row: BoundedIntegerNoGoodRow::RequireDifference,
                        ..
                    }
                )
            })
            .expect("difference row");
        let removed_id = difference_row
            .coefficients
            .keys()
            .next()
            .cloned()
            .expect("difference coefficient");
        difference_row.coefficients.remove(&removed_id);
        assert!(validate_bounded_integer_families(&difference, &assignment, &binding).is_err());

        let mut wrong_row_variable =
            bounded_integer_assignment_no_good(&assignment, &ids, &binding, "cp/proof")
                .expect("cuts");
        let lower_row = wrong_row_variable
            .iter_mut()
            .find(|cut| {
                matches!(
                    cut.encoding,
                    MasterCutEncoding::BoundedIntegerNoGood {
                        row: BoundedIntegerNoGoodRow::LowerSide(_),
                        ..
                    }
                )
            })
            .expect("lower row");
        lower_row.encoding = MasterCutEncoding::BoundedIntegerNoGood {
            family_id: "cp/proof".to_owned(),
            assignment: assignment.values.clone(),
            row: BoundedIntegerNoGoodRow::LowerSide(StableVariableId::from("y")),
        };
        assert!(
            validate_bounded_integer_families(&wrong_row_variable, &assignment, &binding).is_err()
        );
    }

    #[test]
    fn logic_based_benders_publishes_nonterminal_iteration_progress() {
        use ospf_rust_core::solver::{SolveProgressReporter, SolveProgressSnapshot};
        use std::sync::{Arc, Mutex};

        let binding = binding_binary();
        let engine = exact_engine(binding.clone());
        let mut master_calls = 0usize;
        let mut master = move |cuts: &[MasterCut], _options: &FrameworkSolveOptions| {
            master_calls += 1;
            let assignment = if cuts.is_empty() {
                [("x", 0.0), ("y", 0.0)]
            } else {
                [("x", 1.0), ("y", 0.0)]
            };
            Ok(master_report(
                &assignment
                    .into_iter()
                    .map(|(id, value)| (StableVariableId::from(id), value))
                    .collect::<Vec<_>>(),
                master_calls as f64,
            ))
        };
        let mut subproblem = |assignment: &MasterAssignment, _options: &FrameworkSolveOptions| {
            if assignment.values[&StableVariableId::from("x")] == 0 {
                Ok(cp_infeasible_result(assignment))
            } else {
                let snapshot = cp_test_snapshot();
                cp_feasible_result_for_master(assignment, &snapshot, "z")
            }
        };
        let snapshots = Arc::new(Mutex::new(Vec::<SolveProgressSnapshot>::new()));
        let captured = snapshots.clone();
        let reporter: SolveProgressReporter = Arc::new(move |snapshot| {
            captured
                .lock()
                .expect("progress snapshots")
                .push(snapshot.clone());
            Ok(())
        });
        engine
            .solve(
                &mut master,
                &mut subproblem,
                &FrameworkSolveOptions::default().with_progress_reporter(Some(reporter)),
            )
            .expect("engine");
        let snapshots = snapshots.lock().expect("progress snapshots");
        assert!(snapshots.len() >= 7);
        assert!(snapshots.iter().any(|snapshot| {
            !snapshot.terminal
                && snapshot
                    .stage_path
                    .last()
                    .is_some_and(|stage| stage == "master")
                && snapshot
                    .overall_progress
                    .as_known()
                    .is_some_and(|value| value < 100.0)
        }));
        assert!(snapshots.iter().any(|snapshot| {
            !snapshot.terminal
                && snapshot
                    .stage_path
                    .last()
                    .is_some_and(|stage| stage == "subproblem")
        }));
        assert!(snapshots.iter().any(|snapshot| {
            !snapshot.terminal
                && snapshot
                    .stage_path
                    .last()
                    .is_some_and(|stage| stage == "cuts")
        }));
        let final_snapshot = snapshots.last().expect("terminal progress");
        assert!(final_snapshot.terminal);
        assert_eq!(final_snapshot.stage, SolveStage::Completed);
    }

    #[test]
    fn master_integer_binding_rejects_values_outside_the_exact_f64_integer_range() {
        let first_inexact = (1_i64 << 53) + 1;
        assert!(
            MasterBinding::new([MasterVariableBinding::integer(
                "too-large",
                first_inexact,
                first_inexact,
            )])
            .is_err()
        );
        assert!(
            MasterBinding::new([MasterVariableBinding::integer(
                "exact",
                1_i64 << 53,
                1_i64 << 53,
            )])
            .is_ok()
        );
    }

    #[test]
    fn bounded_integer_no_good_exhaustive_oracle_excludes_only_the_target_point() {
        let binding = MasterBinding::new([
            MasterVariableBinding::integer("x", 0, 2),
            MasterVariableBinding::integer("y", -1, 1),
        ])
        .expect("binding");
        let target = MasterAssignment::new(
            [
                (StableVariableId::from("x"), 1),
                (StableVariableId::from("y"), 0),
            ]
            .into_iter()
            .collect(),
            &binding,
        )
        .expect("target");
        let ids = binding.ids().cloned().collect::<BTreeSet<_>>();
        let cuts =
            bounded_integer_assignment_no_good(&target, &ids, &binding, "cp/proof").expect("cuts");
        for x in 0..=2 {
            for y in -1..=1 {
                let assignment = MasterAssignment::new(
                    [
                        (StableVariableId::from("x"), x),
                        (StableVariableId::from("y"), y),
                    ]
                    .into_iter()
                    .collect(),
                    &binding,
                )
                .expect("assignment");
                let target_point = x == 1 && y == 0;
                let feasible = auxiliary_feasible(&cuts, &assignment);
                assert_eq!(feasible, !target_point, "assignment ({x}, {y})");
            }
        }
    }

    #[test]
    fn bounded_integer_conflict_subset_excludes_only_the_selected_coordinate() {
        let binding = MasterBinding::new([
            MasterVariableBinding::integer("x", 0, 2),
            MasterVariableBinding::integer("y", -1, 1),
        ])
        .expect("binding");
        let target = MasterAssignment::new(
            [
                (StableVariableId::from("x"), 1),
                (StableVariableId::from("y"), 0),
            ]
            .into_iter()
            .collect(),
            &binding,
        )
        .expect("target");
        let ids = [StableVariableId::from("x")].into_iter().collect();
        let cuts =
            bounded_integer_assignment_no_good(&target, &ids, &binding, "cp/proof").expect("cuts");
        validate_bounded_integer_families(&cuts, &target, &binding).expect("family");
        for x in 0..=2 {
            for y in -1..=1 {
                let assignment = MasterAssignment::new(
                    [
                        (StableVariableId::from("x"), x),
                        (StableVariableId::from("y"), y),
                    ]
                    .into_iter()
                    .collect(),
                    &binding,
                )
                .expect("assignment");
                assert_eq!(
                    auxiliary_feasible(&cuts, &assignment),
                    x != 1,
                    "assignment ({x}, {y})"
                );
            }
        }
    }

    fn auxiliary_feasible(cuts: &[MasterCut], assignment: &MasterAssignment) -> bool {
        let auxiliary_ids = cuts
            .iter()
            .flat_map(|cut| {
                cut.auxiliary_variables
                    .iter()
                    .map(|variable| variable.id.clone())
            })
            .collect::<BTreeSet<_>>();
        let auxiliary_ids = auxiliary_ids.into_iter().collect::<Vec<_>>();
        let combinations = 1usize << auxiliary_ids.len();
        (0..combinations).any(|mask| {
            cuts.iter().all(|cut| {
                let lhs = cut
                    .coefficients
                    .iter()
                    .map(|(id, coefficient)| {
                        assignment
                            .values
                            .get(id)
                            .copied()
                            .or_else(|| {
                                auxiliary_ids
                                    .iter()
                                    .position(|candidate| candidate == id)
                                    .map(|index| ((mask >> index) & 1) as i64)
                            })
                            .map(|value| *coefficient * value as f64)
                            .unwrap_or(f64::NAN)
                    })
                    .sum::<f64>();
                match cut.sense {
                    crate::solver::CutSense::LessOrEqual => lhs <= cut.rhs + 1e-9,
                    crate::solver::CutSense::GreaterOrEqual => lhs >= cut.rhs - 1e-9,
                    crate::solver::CutSense::Equal => (lhs - cut.rhs).abs() <= 1e-9,
                }
            })
        })
    }

    #[test]
    fn engine_adds_verified_no_good_and_reaches_exact_feasible_master() {
        let binding = binding_binary();
        let engine = exact_engine(binding.clone());
        let mut master_calls = 0usize;
        let mut master = move |cuts: &[MasterCut], _options: &FrameworkSolveOptions| {
            master_calls += 1;
            let assignment = if cuts.is_empty() {
                [("x", 0.0), ("y", 0.0)]
            } else {
                [("x", 1.0), ("y", 0.0)]
            };
            Ok(master_report(
                &assignment
                    .into_iter()
                    .map(|(id, value)| (StableVariableId::from(id), value))
                    .collect::<Vec<_>>(),
                master_calls as f64,
            ))
        };
        let mut subproblem = |assignment: &MasterAssignment, _options: &FrameworkSolveOptions| {
            if assignment.values[&StableVariableId::from("x")] == 0 {
                Ok(cp_infeasible_result(assignment))
            } else {
                let snapshot = cp_test_snapshot();
                cp_feasible_result_for_master(assignment, &snapshot, "z")
            }
        };
        let report = engine
            .solve(
                &mut master,
                &mut subproblem,
                &FrameworkSolveOptions::default(),
            )
            .expect("engine");
        assert_eq!(report.problem_status, ProblemStatus::Feasible);
        assert!(report.is_optimal(), "{report:?}");
        assert_eq!(report.trace.total_iterations, 2);
        assert_eq!(report.trace.generated_columns, 1);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn checkpoint_resume_replays_saved_cuts_before_calling_master() {
        use ospf_rust_core::solver::{AuditFingerprint, SolveCheckpoint, SolveIterationSnapshot};

        let binding = MasterBinding::new([MasterVariableBinding::binary("x")]).expect("binding");
        let engine = exact_engine(binding.clone());
        let cut = MasterCut::verified_global(
            "saved-cut",
            [(StableVariableId::from("x"), 1.0)],
            super::super::CutSense::GreaterOrEqual,
            1.0,
            "saved-proof",
            "test",
            "saved cut",
        );
        let cp_snapshot = cp_test_snapshot()
            .to_artifact()
            .expect("CP snapshot artifact");
        let model_fingerprint = cp_snapshot.snapshot.fingerprint.clone();
        let state =
            super::super::logic_based_benders_checkpoint::LogicBasedBendersCheckpointState::new_with_master_and_factory(
                &binding,
                LogicBasedBendersMode::Exact,
                1e-7,
                ObjectiveCategory::Minimum,
                vec![cut],
                Vec::new(),
                vec![SolveIterationSnapshot {
                    iteration: 1,
                    stage: "logic-based-benders/master-subproblem".to_owned(),
                    items_added: 1,
                    items_total: 1,
                    ..SolveIterationSnapshot::default()
                }],
                1,
                true,
                model_fingerprint.clone(),
                Some(ospf_rust_core::solver::sha256_fingerprint(
                    "test.master",
                    b"master",
                )),
                None,
                Some(cp_snapshot.clone()),
                Some(ospf_rust_core::solver::sha256_fingerprint(
                    "test.subproblem-factory",
                    b"factory",
                )),
            )
            .expect("state");
        let checkpoint = SolveCheckpoint::new(
            "run-1",
            "attempt-1",
            None,
            model_fingerprint,
            AuditFingerprint {
                schema_version: "1.0".to_owned(),
                algorithm: "sha256".to_owned(),
                value: "config".to_owned(),
            },
            AuditFingerprint {
                schema_version: "1.0".to_owned(),
                algorithm: "sha256".to_owned(),
                value: "solver".to_owned(),
            },
            SolverProvenance {
                solver_id: "fake/lbb".to_owned(),
                backend_name: "fake".to_owned(),
                ..SolverProvenance::default()
            },
            1,
            None,
            None,
            None,
            AuditFingerprint {
                schema_version: "1.0".to_owned(),
                algorithm: "sha256".to_owned(),
                value: "placeholder".to_owned(),
            },
        )
        .expect("checkpoint");
        let artifact =
            super::super::logic_based_benders_checkpoint::LogicBasedBendersCheckpointArtifact::new(
                checkpoint, state,
            )
            .expect("artifact");
        let expected =
            super::super::logic_based_benders_checkpoint::LogicBasedBendersResumeIdentity {
                run_id: "run-1".to_owned(),
                source_attempt_id: "attempt-1".to_owned(),
                source_parent_attempt_id: None,
                model_fingerprint: artifact.checkpoint.checkpoint.model_fingerprint.clone(),
                configuration_fingerprint: artifact
                    .checkpoint
                    .checkpoint
                    .configuration_fingerprint
                    .clone(),
                solver_fingerprint: artifact.checkpoint.checkpoint.solver_fingerprint.clone(),
                provenance: artifact.checkpoint.checkpoint.provenance.clone(),
                cancellation_chain: artifact.checkpoint.checkpoint.cancellation_chain.clone(),
                cp_snapshot_fingerprint: artifact
                    .state
                    .cp_snapshot
                    .as_ref()
                    .expect("CP snapshot")
                    .snapshot
                    .fingerprint
                    .clone(),
                master_fingerprint: artifact
                    .state
                    .master_fingerprint
                    .clone()
                    .expect("master fingerprint"),
                subproblem_factory_fingerprint: artifact
                    .state
                    .subproblem_factory_fingerprint
                    .clone()
                    .expect("factory fingerprint"),
            };
        let mut seen = Vec::new();
        let mut master = IdentifiedMaster::new(
            |cuts: &[MasterCut], _options: &FrameworkSolveOptions| {
                seen.push(cuts.iter().map(|cut| cut.id.clone()).collect::<Vec<_>>());
                let mut report = master_report(&[(StableVariableId::from("x"), 1.0)], 1.0);
                report.fingerprints.model = Some(expected.master_fingerprint.clone());
                Ok(report)
            },
            expected.master_fingerprint.clone(),
        )
        .expect("identified master");
        let factory_fingerprint = artifact
            .state
            .subproblem_factory_fingerprint
            .clone()
            .expect("factory fingerprint");
        let factory = SnapshotSubproblemFactory::new(
            {
                let factory_fingerprint = factory_fingerprint.clone();
                move |artifact: &ConstraintProgrammingSnapshotArtifact| {
                    let snapshot = artifact.snapshot.clone();
                    IdentifiedSubproblemSolver::new(
                        move |assignment: &MasterAssignment, _options: &FrameworkSolveOptions| {
                            cp_feasible_result_for_master(assignment, &snapshot, "z")
                        },
                        factory_fingerprint.clone(),
                    )
                }
            },
            factory_fingerprint.clone(),
        )
        .expect("snapshot factory");
        let mut oracle = DefaultFeasibilityCutOracle::new(binding);
        let options = FrameworkSolveOptions::default().with_iterations(2, 1e-9);
        let report = engine
            .resume_from_checkpoint(
                &artifact,
                &expected,
                &mut master,
                &factory,
                &mut oracle,
                &mut MasterObjectiveEvaluator,
                &options,
            )
            .expect("resume");
        assert_eq!(seen.first(), Some(&vec!["saved-cut".to_owned()]));
        assert_eq!(report.trace.total_iterations, 2);

        let wrong_master_fingerprint =
            ospf_rust_core::solver::sha256_fingerprint("test.master", b"different-master");
        let mut wrong_master = IdentifiedMaster::new(
            {
                let wrong_master_fingerprint = wrong_master_fingerprint.clone();
                move |_cuts: &[MasterCut], _options: &FrameworkSolveOptions| {
                    let mut report = master_report(&[(StableVariableId::from("x"), 1.0)], 1.0);
                    report.fingerprints.model = Some(wrong_master_fingerprint.clone());
                    Ok(report)
                }
            },
            wrong_master_fingerprint,
        )
        .expect("wrong identified master");
        assert!(
            engine
                .resume_from_checkpoint(
                    &artifact,
                    &expected,
                    &mut wrong_master,
                    &factory,
                    &mut oracle,
                    &mut MasterObjectiveEvaluator,
                    &options,
                )
                .is_err(),
            "resume must reject a master with a different model identity"
        );

        let wrong_factory = SnapshotSubproblemFactory::new(
            move |_artifact: &ConstraintProgrammingSnapshotArtifact| {
                let snapshot = cp_test_snapshot();
                IdentifiedSubproblemSolver::new(
                    move |assignment: &MasterAssignment, _options: &FrameworkSolveOptions| {
                        cp_feasible_result_for_master(assignment, &snapshot, "z")
                    },
                    ospf_rust_core::solver::sha256_fingerprint(
                        "test.subproblem-factory",
                        b"different-factory",
                    ),
                )
            },
            ospf_rust_core::solver::sha256_fingerprint(
                "test.subproblem-factory",
                b"different-factory",
            ),
        )
        .expect("alternate snapshot factory");
        let mut factory_mismatch_master = IdentifiedMaster::new(
            {
                let master_fingerprint = expected.master_fingerprint.clone();
                move |_cuts: &[MasterCut], _options: &FrameworkSolveOptions| {
                    let mut report = master_report(&[(StableVariableId::from("x"), 1.0)], 1.0);
                    report.fingerprints.model = Some(master_fingerprint.clone());
                    Ok(report)
                }
            },
            expected.master_fingerprint.clone(),
        )
        .expect("factory mismatch master");
        assert!(
            engine
                .resume_from_checkpoint(
                    &artifact,
                    &expected,
                    &mut factory_mismatch_master,
                    &wrong_factory,
                    &mut oracle,
                    &mut MasterObjectiveEvaluator,
                    &options,
                )
                .is_err()
        );

        let same_factory_fingerprint = factory_fingerprint.clone();
        let same_factory_wrong_snapshot = SnapshotSubproblemFactory::new(
            move |_artifact: &ConstraintProgrammingSnapshotArtifact| {
                let snapshot = cp_other_snapshot();
                IdentifiedSubproblemSolver::new(
                    move |assignment: &MasterAssignment, _options: &FrameworkSolveOptions| {
                        cp_feasible_result_for_master(assignment, &snapshot, "other-z")
                    },
                    same_factory_fingerprint.clone(),
                )
            },
            factory_fingerprint,
        )
        .expect("same-identity alternate snapshot factory");
        let mut wrong_snapshot_master = IdentifiedMaster::new(
            {
                let master_fingerprint = expected.master_fingerprint.clone();
                move |_cuts: &[MasterCut], _options: &FrameworkSolveOptions| {
                    let mut report = master_report(&[(StableVariableId::from("x"), 1.0)], 1.0);
                    report.fingerprints.model = Some(master_fingerprint.clone());
                    Ok(report)
                }
            },
            expected.master_fingerprint.clone(),
        )
        .expect("snapshot mismatch master");
        assert!(
            engine
                .resume_from_checkpoint(
                    &artifact,
                    &expected,
                    &mut wrong_snapshot_master,
                    &same_factory_wrong_snapshot,
                    &mut oracle,
                    &mut MasterObjectiveEvaluator,
                    &options,
                )
                .is_err()
        );
    }

    #[test]
    fn snapshot_feasible_result_requires_matching_model_and_objective() {
        let snapshot = objective_snapshot(ospf_rust_core::model::ObjectiveCategory::Minimum);
        let report = cp_objective_report(&snapshot, 5, Some(2));
        let result = ConstraintProgrammingSubproblemResult::feasible_from_snapshot(
            report.clone(),
            &snapshot,
            Some(5.0),
            Some(2.0),
        )
        .expect("matching CP payload");
        assert!(result.has_verified_assignment());

        assert!(
            ConstraintProgrammingSubproblemResult::feasible_from_snapshot(
                cp_feasible_report(&[("z", 1)]),
                &snapshot,
                Some(5.0),
                Some(2.0),
            )
            .is_err()
        );

        assert!(
            ConstraintProgrammingSubproblemResult::feasible_from_snapshot(
                report,
                &snapshot,
                Some(4.0),
                Some(2.0),
            )
            .is_err()
        );
    }

    #[test]
    fn exact_mode_requires_a_private_snapshot_proof_for_feasible_results() {
        let result =
            ConstraintProgrammingSubproblemResult::feasible(cp_feasible_report(&[("z", 1)]))
                .expect("structurally feasible CP result");
        assert!(!result.has_verified_assignment());
        assert!(result.model_fingerprint().is_none());

        let mut forged = result;
        if let ConstraintProgrammingSubproblemResult::Feasible {
            report,
            assignment,
            assignment_proof,
            ..
        } = &mut forged
        {
            *assignment_proof = Some(VerifiedCpAssignmentProof {
                snapshot_fingerprint: cp_test_snapshot().fingerprint,
                assumption_snapshot_fingerprint: cp_test_snapshot().fingerprint,
                assignment_fingerprint: cp_assignment_fingerprint(assignment),
                report_fingerprint: sha256_fingerprint("ospf.lbb.cp.report", b"forged"),
                master_assignment_fingerprint: None,
            });
            assert!(report.fingerprints.model.is_none());
        }
        assert!(forged.validate().is_err());
    }

    #[test]
    fn exact_mode_rejects_a_feasible_proof_from_another_master_assignment() {
        let binding = binding_binary();
        let engine = exact_engine(binding.clone());
        let stale_assignment = MasterAssignment::new(
            [
                (StableVariableId::from("x"), 0),
                (StableVariableId::from("y"), 0),
            ]
            .into_iter()
            .collect(),
            &binding,
        )
        .expect("stale master assignment");
        let snapshot = cp_test_snapshot();
        let stale_result = cp_feasible_result_for_master(&stale_assignment, &snapshot, "z")
            .expect("stale proof fixture");

        let mut master = |_cuts: &[MasterCut], _options: &FrameworkSolveOptions| {
            Ok(master_report(
                &[
                    (StableVariableId::from("x"), 1.0),
                    (StableVariableId::from("y"), 0.0),
                ],
                1.0,
            ))
        };
        let mut subproblem = move |_assignment: &MasterAssignment,
                                   _options: &FrameworkSolveOptions| {
            Ok(stale_result.clone())
        };
        let report = engine
            .solve(
                &mut master,
                &mut subproblem,
                &FrameworkSolveOptions::default(),
            )
            .expect("stale proof should be downgraded");
        assert_eq!(report.termination_reason, TerminationReason::Suboptimal);
        assert!(
            report
                .diagnostics
                .issues
                .iter()
                .any(|issue| { issue.code == "LogicBasedBendersSubproblemAssignmentNotVerified" })
        );
    }

    #[test]
    fn feasible_master_proof_cannot_be_resigned_from_an_old_assumption_report() {
        let binding = binding_binary();
        let old_assignment = MasterAssignment::new(
            [
                (StableVariableId::from("x"), 0),
                (StableVariableId::from("y"), 0),
            ]
            .into_iter()
            .collect(),
            &binding,
        )
        .expect("old assignment");
        let current_assignment = MasterAssignment::new(
            [
                (StableVariableId::from("x"), 1),
                (StableVariableId::from("y"), 0),
            ]
            .into_iter()
            .collect(),
            &binding,
        )
        .expect("current assignment");
        let snapshot = cp_test_snapshot();
        let old_binding = cp_master_assumption_binding(&old_assignment, "z");
        let old_snapshot = snapshot_with_assumptions(&snapshot, &old_binding.assumptions)
            .expect("old assumption snapshot");
        let old_report = cp_feasible_report_for_snapshot(&[("z", 0)], &old_snapshot);
        let current_binding = cp_master_assumption_binding(&current_assignment, "z");
        assert!(
            ConstraintProgrammingSubproblemResult::feasible_from_snapshot_for_master(
                old_report,
                &snapshot,
                &current_assignment,
                &current_binding,
                None,
                None,
            )
            .is_err()
        );
    }

    #[test]
    fn infeasible_master_proof_cannot_be_reused_at_another_master_assignment() {
        let binding = binding_binary();
        let old_assignment = MasterAssignment::new(
            [
                (StableVariableId::from("x"), 0),
                (StableVariableId::from("y"), 0),
            ]
            .into_iter()
            .collect(),
            &binding,
        )
        .expect("old assignment");
        let current_assignment = MasterAssignment::new(
            [
                (StableVariableId::from("x"), 1),
                (StableVariableId::from("y"), 0),
            ]
            .into_iter()
            .collect(),
            &binding,
        )
        .expect("current assignment");
        let stale_result = cp_infeasible_result(&old_assignment);
        assert!(stale_result.has_verified_infeasibility_for_master(&old_assignment));
        assert!(!stale_result.has_verified_infeasibility_for_master(&current_assignment));
    }

    #[test]
    fn exact_mode_does_not_bootstrap_snapshot_identity_from_the_first_result() {
        let binding = binding_binary();
        let engine = LogicBasedBendersEngine::new(binding.clone());
        let subproblem_calls = std::cell::Cell::new(0usize);
        let snapshot = cp_test_snapshot();
        let mut master = |_cuts: &[MasterCut], _options: &FrameworkSolveOptions| {
            Ok(master_report(
                &[
                    (StableVariableId::from("x"), 0.0),
                    (StableVariableId::from("y"), 0.0),
                ],
                0.0,
            ))
        };
        let mut subproblem = |assignment: &MasterAssignment, _options: &FrameworkSolveOptions| {
            subproblem_calls.set(subproblem_calls.get() + 1);
            cp_feasible_result_for_master(assignment, &snapshot, "z")
        };
        let report = engine
            .solve(
                &mut master,
                &mut subproblem,
                &FrameworkSolveOptions::default(),
            )
            .expect("missing external snapshot identity should be explicit");
        assert_eq!(subproblem_calls.get(), 0);
        assert_eq!(report.termination_reason, TerminationReason::Suboptimal);
        assert!(
            report
                .diagnostics
                .issues
                .iter()
                .any(|issue| { issue.code == "LogicBasedBendersCpSnapshotIdentityMissing" })
        );
    }

    #[test]
    fn exact_mode_rejects_conflicting_engine_and_options_snapshot_identities() {
        let binding = binding_binary();
        let engine = exact_engine(binding.clone());
        let options = FrameworkSolveOptions::default().with_expected_cp_snapshot_fingerprint(
            ospf_rust_core::solver::sha256_fingerprint("test.cp.snapshot", b"different"),
        );
        let master_calls = std::cell::Cell::new(0usize);
        let mut master = |_cuts: &[MasterCut], _options: &FrameworkSolveOptions| {
            master_calls.set(master_calls.get() + 1);
            Ok(master_report(
                &[
                    (StableVariableId::from("x"), 0.0),
                    (StableVariableId::from("y"), 0.0),
                ],
                0.0,
            ))
        };
        let mut subproblem = |_assignment: &MasterAssignment, _options: &FrameworkSolveOptions| {
            Ok(ConstraintProgrammingSubproblemResult::failed(
                "unexpected-subproblem-call",
                "conflicting snapshot identities should fail before solving",
            ))
        };
        let error = engine
            .solve(&mut master, &mut subproblem, &options)
            .expect_err("conflicting snapshot identities must be rejected");
        assert!(error.to_string().contains("engine and options"));
        assert_eq!(master_calls.get(), 0);
    }

    #[test]
    fn snapshot_feasible_result_rejects_a_bound_in_the_wrong_direction() {
        let snapshot = objective_snapshot(ospf_rust_core::model::ObjectiveCategory::Minimum);
        let report = cp_objective_report(&snapshot, 5, Some(6));
        assert!(
            ConstraintProgrammingSubproblemResult::feasible_from_snapshot(
                report,
                &snapshot,
                Some(5.0),
                Some(6.0),
            )
            .is_err()
        );

        let snapshot = objective_snapshot(ospf_rust_core::model::ObjectiveCategory::Maximum);
        let report = cp_objective_report(&snapshot, 5, Some(4));
        assert!(
            ConstraintProgrammingSubproblemResult::feasible_from_snapshot(
                report,
                &snapshot,
                Some(5.0),
                Some(4.0),
            )
            .is_err()
        );
    }

    #[test]
    fn infeasibility_requires_the_current_snapshot_for_exact_benders() {
        let snapshot = cp_test_snapshot();
        let report = SolveReport::builder(ProblemStatus::Infeasible, TerminationReason::Completed)
            .proof(SolveProof::infeasibility())
            .fingerprints(SolveFingerprints {
                model: Some(snapshot.fingerprint.clone()),
                ..SolveFingerprints::default()
            })
            .build()
            .expect("CP infeasible report");
        let result = ConstraintProgrammingSubproblemResult::infeasible_from_snapshot(
            report,
            &cp_other_snapshot(),
            None,
        );
        assert!(result.is_err());

        let binding = binding_binary();
        let engine = exact_engine(binding);
        let mut master = |_cuts: &[MasterCut], _options: &FrameworkSolveOptions| {
            Ok(master_report(
                &[
                    (StableVariableId::from("x"), 0.0),
                    (StableVariableId::from("y"), 0.0),
                ],
                0.0,
            ))
        };
        let mut subproblem = |_assignment: &MasterAssignment, _options: &FrameworkSolveOptions| {
            Ok(cp_unverified_infeasible_result())
        };
        let unverified = cp_unverified_infeasible_result();
        assert!(unverified.model_fingerprint().is_none());
        let report = engine
            .solve(
                &mut master,
                &mut subproblem,
                &FrameworkSolveOptions::default(),
            )
            .expect("exact engine should downgrade unverified infeasibility");
        assert_eq!(report.problem_status, ProblemStatus::Feasible);
        assert_eq!(report.termination_reason, TerminationReason::Suboptimal);
        assert!(report.proof.is_none());
        assert!(
            report
                .diagnostics
                .issues
                .iter()
                .any(|issue| issue.code == "LogicBasedBendersInfeasibilityNotVerified")
        );
    }

    #[test]
    fn engine_does_not_call_subproblem_for_nonoptimal_master() {
        let binding = binding_binary();
        let engine = exact_engine(binding);
        let mut sub_calls = 0usize;
        let mut master = |_cuts: &[MasterCut], _options: &FrameworkSolveOptions| {
            SolveReport::builder(ProblemStatus::Unknown, TerminationReason::TimeLimit).build()
        };
        let mut subproblem = |_assignment: &MasterAssignment, _options: &FrameworkSolveOptions| {
            sub_calls += 1;
            Ok(ConstraintProgrammingSubproblemResult::failed(
                "unexpected",
                "subproblem should not be called",
            ))
        };
        let report = engine
            .solve(
                &mut master,
                &mut subproblem,
                &FrameworkSolveOptions::default(),
            )
            .expect("engine");
        assert_eq!(sub_calls, 0);
        assert_eq!(report.termination_reason, TerminationReason::TimeLimit);
        assert!(
            report
                .diagnostics
                .issues
                .iter()
                .any(|issue| issue.code == "LogicBasedBendersMasterNotOptimal")
        );
    }

    #[test]
    fn incomplete_subproblem_is_not_promoted_to_infeasible() {
        let binding = binding_binary();
        let engine = exact_engine(binding);
        let mut master = |_cuts: &[MasterCut], _options: &FrameworkSolveOptions| {
            Ok(master_report(
                &[
                    (StableVariableId::from("x"), 0.0),
                    (StableVariableId::from("y"), 0.0),
                ],
                0.0,
            ))
        };
        let mut subproblem = |_assignment: &MasterAssignment, _options: &FrameworkSolveOptions| {
            ConstraintProgrammingSubproblemResult::incomplete(
                SolveReport::builder(ProblemStatus::Unknown, TerminationReason::TimeLimit)
                    .build()
                    .expect("incomplete report"),
                None,
            )
        };
        let report = engine
            .solve(
                &mut master,
                &mut subproblem,
                &FrameworkSolveOptions::default(),
            )
            .expect("engine");
        assert_eq!(report.problem_status, ProblemStatus::Unknown);
        assert_ne!(report.problem_status, ProblemStatus::Infeasible);
    }

    #[test]
    fn exact_mode_rejects_local_or_claimed_cut() {
        let binding = binding_binary();
        let engine = exact_engine(binding);
        let mut master = |_cuts: &[MasterCut], _options: &FrameworkSolveOptions| {
            Ok(master_report(
                &[
                    (StableVariableId::from("x"), 0.0),
                    (StableVariableId::from("y"), 0.0),
                ],
                0.0,
            ))
        };
        let mut subproblem = |assignment: &MasterAssignment, _options: &FrameworkSolveOptions| {
            Ok(cp_infeasible_result(assignment))
        };
        let mut oracle =
            |assignment: &MasterAssignment, _result: &ConstraintProgrammingSubproblemResult| {
                let id = StableVariableId::from("x");
                Ok(vec![MasterCut::new(
                    "claimed",
                    [(id, 1.0)],
                    super::super::CutSense::GreaterOrEqual,
                    assignment.values[&StableVariableId::from("x")] as f64 + 1.0,
                    MasterCutValidity::Local,
                    MasterCutProofStatus::Claimed,
                    None,
                    "test",
                    "test",
                )])
            };
        let result = engine.solve_with_oracle(
            &mut master,
            &mut subproblem,
            &mut oracle,
            &FrameworkSolveOptions::default(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn exact_mode_rejects_a_verified_cut_bound_to_another_attempt() {
        let binding = binding_binary();
        let engine = exact_engine(binding.clone());
        let mut master = |_cuts: &[MasterCut], _options: &FrameworkSolveOptions| {
            Ok(master_report(
                &[
                    (StableVariableId::from("x"), 0.0),
                    (StableVariableId::from("y"), 0.0),
                ],
                0.0,
            ))
        };
        let mut subproblem = |assignment: &MasterAssignment, _options: &FrameworkSolveOptions| {
            Ok(cp_infeasible_result(assignment))
        };
        let ids = binding.ids().cloned().collect::<BTreeSet<_>>();
        let mut oracle =
            move |assignment: &MasterAssignment,
                  _result: &ConstraintProgrammingSubproblemResult| {
                Ok(vec![binary_assignment_no_good(
                    assignment,
                    &ids,
                    "other-attempt",
                )?])
            };
        let result = engine.solve_with_oracle(
            &mut master,
            &mut subproblem,
            &mut oracle,
            &FrameworkSolveOptions::default(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn failed_subproblem_is_mapped_to_backend_failure() {
        let engine = exact_engine(binding_binary());
        let mut master = |_cuts: &[MasterCut], _options: &FrameworkSolveOptions| {
            Ok(master_report(
                &[
                    (StableVariableId::from("x"), 0.0),
                    (StableVariableId::from("y"), 0.0),
                ],
                0.0,
            ))
        };
        let mut subproblem = |_assignment: &MasterAssignment, _options: &FrameworkSolveOptions| {
            Ok(ConstraintProgrammingSubproblemResult::failed(
                "cp-backend",
                "subproblem backend failed",
            ))
        };
        let report = engine
            .solve(
                &mut master,
                &mut subproblem,
                &FrameworkSolveOptions::default(),
            )
            .expect("failed subproblem report");
        assert_eq!(report.termination_reason, TerminationReason::BackendFailure);
        assert!(
            report
                .diagnostics
                .issues
                .iter()
                .any(|issue| issue.code == "cp-backend")
        );
    }

    #[test]
    fn backend_failure_after_a_feasible_iteration_retains_the_incumbent() {
        let binding = binding_binary();
        let engine = exact_engine(binding);
        let mut master = |cuts: &[MasterCut], _options: &FrameworkSolveOptions| {
            let value = if cuts.is_empty() { 0.0 } else { 1.0 };
            Ok(master_report(
                &[
                    (StableVariableId::from("x"), value),
                    (StableVariableId::from("y"), 0.0),
                ],
                value,
            ))
        };
        let mut subproblem = |assignment: &MasterAssignment, _options: &FrameworkSolveOptions| {
            if assignment.values[&StableVariableId::from("x")] == 0 {
                let snapshot = cp_test_snapshot();
                cp_feasible_result_for_master(assignment, &snapshot, "z")
            } else {
                Ok(ConstraintProgrammingSubproblemResult::failed(
                    "cp-backend",
                    "subproblem backend failed after an incumbent",
                ))
            }
        };
        let mut oracle = ContinuingFeasibleOracle;
        let report = engine
            .solve_with_oracle(
                &mut master,
                &mut subproblem,
                &mut oracle,
                &FrameworkSolveOptions::default(),
            )
            .expect("backend failure report");
        assert_eq!(report.termination_reason, TerminationReason::BackendFailure);
        assert_eq!(report.problem_status, ProblemStatus::Feasible);
        assert_eq!(
            report.solution_presence,
            ospf_rust_core::solver::SolutionPresence::Incumbent
        );
        assert!(report.solution.is_some());
        assert!(report.proof.is_none());
        assert!(
            report
                .diagnostics
                .issues
                .iter()
                .any(|issue| issue.code == "cp-backend")
        );
    }

    struct ContinuingFeasibleOracle;

    impl LogicBasedBendersCutOracle for ContinuingFeasibleOracle {
        fn cuts_for_infeasible(
            &mut self,
            _assignment: &MasterAssignment,
            _result: &ConstraintProgrammingSubproblemResult,
        ) -> Result<Vec<MasterCut>> {
            Ok(Vec::new())
        }

        fn cuts_for_feasible_with_context(
            &mut self,
            assignment: &MasterAssignment,
            _result: &ConstraintProgrammingSubproblemResult,
            context: &LogicBasedBendersProofContext,
        ) -> Result<Vec<MasterCut>> {
            if assignment.values[&StableVariableId::from("x")] == 0 {
                Ok(vec![MasterCut::verified_global(
                    "continue-after-feasible",
                    [(StableVariableId::from("x"), 1.0)],
                    super::super::CutSense::GreaterOrEqual,
                    0.0,
                    context.proof_reference.clone(),
                    "test",
                    "continue to exercise cancellation after an incumbent",
                )])
            } else {
                Ok(Vec::new())
            }
        }
    }

    #[test]
    fn cancellation_keeps_the_incumbent_and_cancellation_origin() {
        let binding = binding_binary();
        let engine = exact_engine(binding);
        let handle = SolveHandle::new();
        let options =
            FrameworkSolveOptions::default().with_cancellation_handle(Some(handle.clone()));
        let mut master = |cuts: &[MasterCut], _options: &FrameworkSolveOptions| {
            let value = if cuts.is_empty() { 0.0 } else { 1.0 };
            Ok(master_report(
                &[
                    (StableVariableId::from("x"), value),
                    (StableVariableId::from("y"), 0.0),
                ],
                value,
            ))
        };
        let mut subproblem = |assignment: &MasterAssignment, _options: &FrameworkSolveOptions| {
            if assignment.values[&StableVariableId::from("x")] == 0 {
                let snapshot = cp_test_snapshot();
                cp_feasible_result_for_master(assignment, &snapshot, "z")
            } else {
                handle.cancel(CancellationOrigin::User);
                ConstraintProgrammingSubproblemResult::incomplete(
                    SolveReport::builder(ProblemStatus::Unknown, TerminationReason::Cancelled)
                        .build()
                        .expect("cancelled CP report"),
                    None,
                )
            }
        };
        let mut oracle = ContinuingFeasibleOracle;
        let report = engine
            .solve_with_oracle(&mut master, &mut subproblem, &mut oracle, &options)
            .expect("cancelled report");
        assert_eq!(report.termination_reason, TerminationReason::Cancelled);
        assert!(report.has_incumbent());
        assert_eq!(
            report.diagnostics.extensions.get("cancellation.origin"),
            Some(&CancellationOrigin::User.to_string())
        );
    }

    #[test]
    fn cancellation_before_start_skips_master_and_subproblem() {
        let binding = MasterBinding::new([MasterVariableBinding::binary("x")]).expect("binding");
        let engine = exact_engine(binding);
        let handle = SolveHandle::new();
        assert!(handle.cancel(CancellationOrigin::User));
        let options = FrameworkSolveOptions::default().with_cancellation_handle(Some(handle));
        let mut master_calls = 0usize;
        let mut master = |_cuts: &[MasterCut], _options: &FrameworkSolveOptions| {
            master_calls += 1;
            Ok(master_report(&[(StableVariableId::from("x"), 0.0)], 0.0))
        };
        let mut subproblem = |_assignment: &MasterAssignment, _options: &FrameworkSolveOptions| {
            Ok(ConstraintProgrammingSubproblemResult::failed(
                "unexpected",
                "unexpected",
            ))
        };
        let report = engine
            .solve(&mut master, &mut subproblem, &options)
            .expect("cancelled engine");
        assert_eq!(master_calls, 0);
        assert_eq!(report.termination_reason, TerminationReason::Cancelled);
    }
}

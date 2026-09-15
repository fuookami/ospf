//! 跨语言语义契约测试（计划 12.1 / 12.3 S0–S6 阶段门）。
//! Cross-language semantic contract test (plan 12.1 / 12.3 S0–S6 gates).
//!
//! 读取两端共同维护的 `analysis-fixtures`，断言 Rust 实现与该契约完全一致。
//! Kotlin 侧由 `AnalysisFixtureContractTest.kt` 断言同一份文件。
//! Reads the shared `analysis-fixtures` directory and asserts that the Rust implementation
//! matches the contract exactly. The Kotlin side asserts the same files from
//! `AnalysisFixtureContractTest.kt`.

#![cfg(test)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::analysis::{
    minimal_correction_set, weighted_correction_set,
    AnalysisStatus, CandidateFunnelRanking, CandidateTier,
    ConflictExtractionTier, ConflictMinimality, CorrectionCandidate, CriticalityKind,
    CriticalityObservation, DiagnosticSource, ObjectiveTarget, RelaxabilityPolicy,
    ACTIVATION_REPORT_SCHEMA_VERSION, ACTIVITY_REPORT_SCHEMA_VERSION,
    ADAPTIVE_PERTURBATION_REPORT_SCHEMA_VERSION, CONFLICT_REPORT_SCHEMA_VERSION,
    CRITICAL_ANALYSIS_REPORT_SCHEMA_VERSION, FIXED_INTEGER_LP_REPORT_SCHEMA_VERSION,
    MULTI_TARGET_ANALYSIS_REPORT_SCHEMA_VERSION, PERTURBATION_REPORT_SCHEMA_VERSION,
    TARGET_FEASIBILITY_REPORT_SCHEMA_VERSION,
    classify_candidate,
};
use crate::analysis::pipeline::combine_status;
use crate::solver::report::ProblemStatus;

/// 定位两端共同维护的 `analysis-fixtures`。
///
/// 只检出单个仓库时返回 `None`，测试显式跳过而不是静默通过。
/// Locate the shared `analysis-fixtures`. When only a single repository is checked out this
/// returns `None` and the test skips explicitly instead of passing silently.
fn fixture_root() -> Option<PathBuf> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    [
        manifest.join("../../analysis-fixtures"),
        manifest.join("../analysis-fixtures"),
        manifest.join("analysis-fixtures"),
    ]
    .into_iter()
    .find(|candidate| candidate.is_dir())
}

fn read_sections(name: &str) -> Option<BTreeMap<String, Vec<Vec<String>>>> {
    let root = fixture_root()?;
    let text = std::fs::read_to_string(root.join(name)).ok()?;
    let mut sections: BTreeMap<String, Vec<Vec<String>>> = BTreeMap::new();
    let mut current: Option<String> = None;
    for raw in text.lines() {
        let line = raw.trim_end();
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        if let Some(inner) = line.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')) {
            current = Some(inner.to_owned());
            sections.entry(inner.to_owned()).or_default();
            continue;
        }
        let Some(section) = current.clone() else {
            continue;
        };
        // 数据行既支持 TAB 分隔，也支持单行 `key=value`。
        // A data row is either TAB-separated or a single `key=value` pair.
        let fields: Vec<String> = line.split('\t').map(|field| field.trim().to_owned()).collect();
        let normalized = if fields.len() == 1 && fields[0].contains('=') {
            let (key, value) = fields[0].split_once('=').expect("key=value");
            vec![key.to_owned(), value.to_owned()]
        } else {
            fields
        };
        sections.entry(section).or_default().push(normalized);
    }
    Some(sections)
}

macro_rules! require_fixtures {
    ($sections:expr) => {
        match $sections {
            Some(sections) => sections,
            None => {
                eprintln!(
                    "skipping cross-language fixture contract: analysis-fixtures not found \
                     (requires sibling ospf-kotlin and ospf-rust checkouts)"
                );
                return;
            }
        }
    };
}

#[test]
fn fixture_contract_vocabulary_matches_both_languages() {
    let sections = require_fixtures!(read_sections("analysis-contract.tsv"));

    // 状态枚举必须完全一致。
    let status: Vec<&str> = sections["status"]
        .iter()
        .map(|row| row[0].as_str())
        .collect();
    assert_eq!(status, ["Reachable", "Unreachable", "Unknown", "Unsupported"]);
    for name in &status {
        let parsed = match *name {
            "Reachable" => AnalysisStatus::Reachable,
            "Unreachable" => AnalysisStatus::Unreachable,
            "Unknown" => AnalysisStatus::Unknown,
            "Unsupported" => AnalysisStatus::Unsupported,
            other => panic!("unknown status in contract: {other}"),
        };
        assert_eq!(parsed.is_proven(), *name != "Unknown" && *name != "Unsupported");
    }

    // "已证明"判定必须一致：Unknown/Unsupported 永远不是已证明结论。
    for row in &sections["proven"] {
        let proven = row[1].parse::<bool>().expect("proven flag");
        let status = match row[0].as_str() {
            "Reachable" => AnalysisStatus::Reachable,
            "Unreachable" => AnalysisStatus::Unreachable,
            "Unknown" => AnalysisStatus::Unknown,
            "Unsupported" => AnalysisStatus::Unsupported,
            other => panic!("unknown status: {other}"),
        };
        assert_eq!(status.is_proven(), proven, "proven flag for {}", row[0]);
    }

    // 允许公开的证据类型：只包含原始模型身份。
    let evidence: Vec<&str> = sections["evidence-kind"]
        .iter()
        .map(|row| row[0].as_str())
        .collect();
    assert_eq!(
        evidence,
        [
            "constraint",
            "variable-lower-bound",
            "variable-upper-bound",
            "sparse-domain",
            "objective-target"
        ]
    );

    // 禁止出现在公开报告中的 solver 生成物。
    let forbidden: Vec<&str> = sections["forbidden-evidence"]
        .iter()
        .map(|row| row[0].as_str())
        .collect();
    assert_eq!(
        forbidden,
        [
            "solver_row",
            "solver_column",
            "auxiliary_variable",
            "auxiliary_constraint",
            "native_handle",
        ]
    );

    // 候选分层：Tier C 与未分层都必须被保留，只是不默认进入短名单。
    assert_eq!(sections["candidate-tier"].len(), 4);
    for row in &sections["candidate-tier"] {
        let tier = match row[0].as_str() {
            "TierA" => CandidateTier::TierA,
            "TierB" => CandidateTier::TierB,
            "TierC" => CandidateTier::TierC,
            "Unclassified" => CandidateTier::Unclassified,
            other => panic!("unknown tier: {other}"),
        };
        assert_eq!(
            tier.is_shortlist_default(),
            row[1].parse::<bool>().expect("shortlist flag"),
            "shortlist flag for {}",
            row[0]
        );
        // 每一层都必须保留，Tier C 只是被降权。
        assert!(
            row[2].parse::<bool>().expect("retained flag"),
            "every tier must be retained: {}",
            row[0]
        );
    }

    // 最小性：只有 Irreducible 是"已证明最小"。
    for row in &sections["minimality"] {
        let minimality = match row[0].as_str() {
            "Irreducible" => ConflictMinimality::Irreducible,
            "Partial" => ConflictMinimality::Partial,
            "NotChecked" => ConflictMinimality::NotChecked,
            other => panic!("unknown minimality: {other}"),
        };
        assert_eq!(
            minimality.is_verified(),
            row[1].parse::<bool>().expect("minimality flag"),
            "minimality flag for {}",
            row[0]
        );
    }

    // 提取层级：只有原生 core 被标记为 native。
    let tiers = &sections["extraction-tier"];
    assert_eq!(tiers.len(), 3);
    let expected = [
        ("NativeUnsatCore", ConflictExtractionTier::NativeUnsatCore),
        (
            "AssumptionExtraction",
            ConflictExtractionTier::AssumptionExtraction,
        ),
        ("RepeatedSolving", ConflictExtractionTier::RepeatedSolving),
    ];
    for (row, (name, tier)) in tiers.iter().zip(expected) {
        assert_eq!(row[1], name);
        assert_eq!(row[2].parse::<bool>().expect("native flag"), tier.is_native());
        assert_eq!(
            row[0].parse::<usize>().expect("priority"),
            match tier {
                ConflictExtractionTier::NativeUnsatCore => 1,
                ConflictExtractionTier::AssumptionExtraction => 2,
                ConflictExtractionTier::RepeatedSolving => 3,
                ConflictExtractionTier::Unavailable => 0,
            }
        );
        assert!(tier.is_available());
    }

    // 对偶作用域：绝不使用"全局影子价格"措辞。
    let scope: BTreeMap<&str, &str> = sections["scope"]
        .iter()
        .map(|row| (row[0].as_str(), row[1].as_str()))
        .collect();
    assert_eq!(scope["local_dual"], "FixedIntegerIncumbent");
    assert_eq!(scope["forbidden_label"], "MILP global shadow price");

    let criticality_kinds: Vec<&str> = sections["criticality-kind"]
        .iter()
        .map(|row| row[0].as_str())
        .collect();
    assert_eq!(
        criticality_kinds,
        ["LocalBottleneck", "PersistentBottleneck", "StructuralBottleneck"]
    );
    let policy_defaults: BTreeMap<&str, &str> = sections["relaxability-policy"]
        .iter()
        .map(|row| (row[0].as_str(), row[1].as_str()))
        .collect();
    assert_eq!(policy_defaults["max_candidates"], "32");
    assert_eq!(policy_defaults["max_plans"], "8");
    assert_eq!(policy_defaults["require_positive_weight"], "true");
    assert_eq!(
        policy_defaults["numeric_relaxation_requires_revalidation"],
        "true"
    );
    let default_policy = RelaxabilityPolicy::default();
    assert_eq!(default_policy.max_candidates, policy_defaults["max_candidates"].parse::<usize>().unwrap());
    assert_eq!(default_policy.max_plans, policy_defaults["max_plans"].parse::<usize>().unwrap());
    assert_eq!(
        default_policy.require_positive_weight,
        policy_defaults["require_positive_weight"].parse::<bool>().unwrap()
    );
}

#[test]
fn fixture_contract_schema_versions_match_both_languages() {
    let sections = require_fixtures!(read_sections("analysis-contract.tsv"));
    let schema: BTreeMap<&str, &str> = sections["schema"]
        .iter()
        .map(|row| (row[0].as_str(), row[1].as_str()))
        .collect();

    assert_eq!(schema["critical_analysis_report"], CRITICAL_ANALYSIS_REPORT_SCHEMA_VERSION);
    assert_eq!(schema["activity_report"], ACTIVITY_REPORT_SCHEMA_VERSION);
    assert_eq!(schema["fixed_integer_lp_report"], FIXED_INTEGER_LP_REPORT_SCHEMA_VERSION);
    assert_eq!(schema["perturbation_report"], PERTURBATION_REPORT_SCHEMA_VERSION);
    assert_eq!(
        schema["adaptive_perturbation_report"],
        ADAPTIVE_PERTURBATION_REPORT_SCHEMA_VERSION
    );
    assert_eq!(schema["target_feasibility_report"], TARGET_FEASIBILITY_REPORT_SCHEMA_VERSION);
    assert_eq!(
        schema["multi_target_analysis_report"],
        MULTI_TARGET_ANALYSIS_REPORT_SCHEMA_VERSION
    );
    assert_eq!(schema["conflict_report"], CONFLICT_REPORT_SCHEMA_VERSION);
    assert_eq!(schema["activation_protocol"], ACTIVATION_REPORT_SCHEMA_VERSION);
    assert_eq!(schema["criticality_profile"], "1.0");
    assert_eq!(schema["correction_set"], "1.0");
    let profile = crate::analysis::build_criticality_profile(Vec::new()).expect("empty profile");
    assert_eq!(schema["criticality_profile"], profile.schema_version);
}

#[test]
fn fixture_contract_status_mapping_cases_match_the_shared_fixture() {
    let sections = require_fixtures!(read_sections("analysis-cases.tsv"));
    let cases = &sections["status-map"];
    assert!(!cases.is_empty());

    for row in cases {
        let status = match row[0].as_str() {
            "Feasible" => ProblemStatus::Feasible,
            "Infeasible" => ProblemStatus::Infeasible,
            "Unbounded" => ProblemStatus::Unbounded,
            "InfeasibleOrUnbounded" => ProblemStatus::InfeasibleOrUnbounded,
            "Unknown" => ProblemStatus::Unknown,
            other => panic!("unknown problem status: {other}"),
        };
        let proven = row[1].parse::<bool>().expect("proven flag");
        let expected = match row[2].as_str() {
            "Reachable" => AnalysisStatus::Reachable,
            "Unreachable" => AnalysisStatus::Unreachable,
            "Unknown" => AnalysisStatus::Unknown,
            "Unsupported" => AnalysisStatus::Unsupported,
            other => panic!("unknown expected status: {other}"),
        };
        let actual = AnalysisStatus::from_problem_status_with_proof(status, proven);
        assert_eq!(expected, actual, "status-map {}/{}", row[0], proven);
        // 未证明时永远不得给出已证明结论。
        if !proven {
            assert!(
                !actual.is_proven(),
                "unproven result must not be a proven conclusion: {}",
                row[0]
            );
        }
    }
}

#[test]
fn fixture_contract_classification_cases_match_the_shared_fixture() {
    let sections = require_fixtures!(read_sections("analysis-cases.tsv"));
    let cases = &sections["classify"];
    assert!(!cases.is_empty());

    for row in cases {
        let activity = match row[0].as_str() {
            "Active" => crate::analysis::ActivityStatus::Active,
            "NearlyActive" => crate::analysis::ActivityStatus::NearlyActive,
            "Inactive" => crate::analysis::ActivityStatus::Inactive,
            "SatisfiedWithoutSlackMetric" => {
                crate::analysis::ActivityStatus::SatisfiedWithoutSlackMetric
            }
            "Violated" => crate::analysis::ActivityStatus::Violated,
            "Unknown" => crate::analysis::ActivityStatus::Unknown,
            other => panic!("unknown activity status: {other}"),
        };
        let dual_available = row[1].parse::<bool>().expect("dual flag");
        let dual_abs: f64 = row[2].parse().expect("dual abs");
        let threshold: f64 = row[3].parse().expect("threshold");
        let expected = match row[4].as_str() {
            "TierA" => CandidateTier::TierA,
            "TierB" => CandidateTier::TierB,
            "TierC" => CandidateTier::TierC,
            "Unclassified" => CandidateTier::Unclassified,
            other => panic!("unknown tier: {other}"),
        };
        let dual = if dual_available { Some(dual_abs) } else { None };
        let actual = classify_candidate(activity, dual, threshold);
        assert_eq!(
            expected, actual,
            "classify {}/{}/{}",
            row[0], dual_available, dual_abs
        );
        // 没有对偶证据时不得声称分层。
        if !dual_available && (row[0] == "Active" || row[0] == "NearlyActive") {
            assert_eq!(CandidateTier::Unclassified, actual);
        }
    }
}

#[test]
fn fixture_contract_combine_cases_match_the_shared_fixture() {
    let sections = require_fixtures!(read_sections("analysis-cases.tsv"));
    let cases = &sections["combine"];
    assert!(!cases.is_empty());

    let parse = |name: &str| match name {
        "Reachable" => AnalysisStatus::Reachable,
        "Unreachable" => AnalysisStatus::Unreachable,
        "Unknown" => AnalysisStatus::Unknown,
        "Unsupported" => AnalysisStatus::Unsupported,
        other => panic!("unknown status: {other}"),
    };
    for row in cases {
        assert_eq!(
            parse(&row[2]),
            combine_status(parse(&row[0]), Some(parse(&row[1]))),
            "combine {}+{}",
            row[0],
            row[1]
        );
    }
}

#[test]
fn fixture_contract_shortlist_cases_keep_lower_tiers() {
    let sections = require_fixtures!(read_sections("analysis-cases.tsv"));
    let cases = &sections["shortlist"];
    assert!(!cases.is_empty());

    for row in cases {
        let tier_a: usize = row[0].parse().expect("tier a");
        let tier_b: usize = row[1].parse().expect("tier b");
        let tier_c: usize = row[2].parse().expect("tier c");
        let limit: usize = row[3].parse().expect("limit");
        let returned: usize = row[4].parse().expect("returned");
        let tier_c_returned: usize = row[5].parse().expect("tier c returned");
        let unclassified: usize = row[6].parse().expect("unclassified");
        let unclassified_returned: usize = row[7].parse().expect("unclassified returned");

        let mut candidates = Vec::new();
        for index in 0..tier_a {
            candidates.push(candidate(&format!("a-{index}"), CandidateTier::TierA, candidates.len()));
        }
        for index in 0..tier_b {
            candidates.push(candidate(&format!("b-{index}"), CandidateTier::TierB, candidates.len()));
        }
        for index in 0..tier_c {
            candidates.push(candidate(&format!("c-{index}"), CandidateTier::TierC, candidates.len()));
        }
        for index in 0..unclassified {
            candidates.push(candidate(
                &format!("u-{index}"),
                CandidateTier::Unclassified,
                candidates.len(),
            ));
        }
        let ranking = CandidateFunnelRanking {
            schema_version: CRITICAL_ANALYSIS_REPORT_SCHEMA_VERSION.to_owned(),
            dual_threshold: 1e-9,
            candidates,
            tier_a,
            tier_b,
            tier_c,
            unclassified,
        };
        let shortlist = ranking.shortlist(limit);
        assert_eq!(returned, shortlist.len(), "shortlist size for {row:?}");
        assert_eq!(
            tier_c_returned,
            shortlist
                .iter()
                .filter(|entry| entry.tier == CandidateTier::TierC)
                .count(),
            "shortlist tier C count for {row:?}"
        );
        assert_eq!(
            unclassified_returned,
            shortlist
                .iter()
                .filter(|entry| entry.tier == CandidateTier::Unclassified)
                .count(),
            "shortlist unclassified count for {row:?}"
        );
        // 进入短名单的必然是优先级最高的那批。

        for (index, entry) in shortlist.iter().enumerate() {
            assert_eq!(index, entry.priority, "shortlist must preserve priority order");
        }
    }
}

#[test]
fn fixture_contract_criticality_cases_use_only_verified_unreachable_evidence() {
    let sections = require_fixtures!(read_sections("analysis-cases.tsv"));
    let cases = &sections["criticality"];
    assert!(!cases.is_empty());

    for row in cases {
        let profile = crate::analysis::build_criticality_profile(parse_criticality_observations(&row[1]))
            .expect("criticality profile");
        let proven_count = profile
            .observations
            .iter()
            .filter(|observation| observation.status == AnalysisStatus::Unreachable)
            .count();
        assert_eq!(proven_count, row[5].parse::<usize>().unwrap(), "proven targets for {row:?}");
        assert_eq!(
            expected_ids(&row[2]),
            profile
                .constraints(CriticalityKind::LocalBottleneck)
                .into_iter()
                .map(|id| id.0)
                .collect(),
            "local profile for {row:?}"
        );
        assert_eq!(
            expected_ids(&row[3]),
            profile
                .constraints(CriticalityKind::PersistentBottleneck)
                .into_iter()
                .map(|id| id.0)
                .collect(),
            "persistent profile for {row:?}"
        );
        assert_eq!(
            expected_ids(&row[4]),
            profile
                .constraints(CriticalityKind::StructuralBottleneck)
                .into_iter()
                .map(|id| id.0)
                .collect(),
            "structural profile for {row:?}"
        );
    }
}

#[test]
fn fixture_contract_correction_cases_share_weight_ordering_policy_and_minimality_boundary() {
    let sections = require_fixtures!(read_sections("analysis-cases.tsv"));
    let cases = &sections["correction"];
    assert!(!cases.is_empty());

    for row in cases {
        let policy = RelaxabilityPolicy {
            max_candidates: row[2].parse().unwrap(),
            max_plans: row[3].parse().unwrap(),
            require_positive_weight: row[4].parse().unwrap(),
        };
        let candidates = parse_correction_candidates(&row[6]);
        let result = match row[1].as_str() {
            "weighted" => weighted_correction_set(candidates, &policy),
            "minimal" => minimal_correction_set(candidates, &policy, row[5].parse().unwrap()),
            operation => panic!("unknown correction operation: {operation}"),
        };
        let accepted: bool = row[10].parse().unwrap();
        assert_eq!(accepted, result.is_ok(), "accepted correction for {row:?}");
        if let Ok(correction) = result {
            let actual_ids: Vec<String> = correction
                .members
                .iter()
                .map(|member| member.source.stable_id().trim_start_matches("constraint:").to_owned())
                .collect();
            assert_eq!(expected_id_list(&row[7]), actual_ids, "member order for {row:?}");
            let expected_cost: f64 = row[8].parse().unwrap();
            assert!((correction.total_cost - expected_cost).abs() < 1e-9, "cost for {row:?}");
            assert_eq!(row[9].parse::<bool>().unwrap(), correction.minimal, "minimality for {row:?}");
            assert!(correction.requires_revalidation());
        }
    }
}

fn expected_ids(raw: &str) -> std::collections::BTreeSet<String> {
    if raw == "-" {
        BTreeSet::new()
    } else {
        raw.split(',').map(str::to_owned).collect()
    }
}

fn expected_id_list(raw: &str) -> Vec<String> {
    if raw == "-" {
        Vec::new()
    } else {
        raw.split(',').map(str::to_owned).collect()
    }
}

fn parse_criticality_observations(raw: &str) -> Vec<CriticalityObservation> {
    raw.split(';')
        .enumerate()
        .map(|(index, encoded)| {
            let fields: Vec<&str> = encoded.split('|').collect();
            assert_eq!(fields.len(), 3, "invalid criticality observation: {encoded}");
            let status = match fields[0] {
                "Reachable" => AnalysisStatus::Reachable,
                "Unreachable" => AnalysisStatus::Unreachable,
                "Unknown" => AnalysisStatus::Unknown,
                "Unsupported" => AnalysisStatus::Unsupported,
                other => panic!("unknown profile status: {other}"),
            };
            let verified: bool = fields[1].parse().unwrap();
            let blocking_constraint_ids = if status == AnalysisStatus::Unreachable && verified {
                expected_ids(fields[2])
                    .into_iter()
                    .map(crate::solver::StableConstraintId)
                    .collect()
            } else {
                BTreeSet::new()
            };
            CriticalityObservation {
                target: ObjectiveTarget::at_least(format!("fixture-objective-{index}"), index as f64)
                    .unwrap(),
                status,
                blocking_constraint_ids,
            }
        })
        .collect()
}

fn parse_correction_candidates(raw: &str) -> Vec<CorrectionCandidate> {
    raw.split(';')
        .filter(|encoded| !encoded.is_empty())
        .map(|encoded| {
            let fields: Vec<&str> = encoded.split(':').collect();
            let (source, weight_index) = match fields.len() {
                3 => (
                    DiagnosticSource::constraint(fields[0]).expect("constraint source"),
                    1,
                ),
                4 => {
                    let source = match fields[0] {
                        "variable-lower-bound" => {
                            DiagnosticSource::variable_lower_bound(fields[1])
                                .expect("variable lower-bound source")
                        }
                        "sparse-domain" => {
                            DiagnosticSource::sparse_domain(fields[1])
                                .expect("sparse-domain source")
                        }
                        other => panic!("unknown correction source: {other}"),
                    };
                    (source, 2)
                }
                _ => panic!("invalid correction candidate: {encoded}"),
            };
            CorrectionCandidate {
                source,
                weight: fields[weight_index].parse().unwrap(),
                relaxation: fields[weight_index + 1].parse().unwrap(),
            }
        })
        .collect()
}

fn candidate(id: &str, tier: CandidateTier, priority: usize) -> crate::analysis::ConstraintCandidate {
    crate::analysis::ConstraintCandidate {
        constraint_id: id.into(),
        group: None,
        tier,
        activity: if tier == CandidateTier::TierC {
            crate::analysis::ActivityStatus::Inactive
        } else {
            crate::analysis::ActivityStatus::Active
        },
        slack: Some(0.0),
        dual_value: Some(0.0),
        local_effective: Some(false),
        priority,
    }
}

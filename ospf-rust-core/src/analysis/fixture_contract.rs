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
/// 候选顺序：仓库同级检出、父仓库嵌套检出、仓内镜像。
/// 仓内镜像保证单仓库检出（CI、外部贡献者）也能真实执行契约测试，
/// 而不是静默通过。镜像与共享副本的一致性由 `fixture_mirror_matches_shared_copy` 守护。
///
/// Locate the shared `analysis-fixtures`. Candidates are: sibling checkout, parent
/// checkout, and an in-repo mirror. The mirror keeps a single-repository checkout
/// (CI, outside contributors) genuinely asserting the contract instead of passing
/// silently. `fixture_mirror_matches_shared_copy` guards mirror/shared equality.
fn fixture_root() -> Option<PathBuf> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    [
        manifest.join("../../analysis-fixtures"),
        manifest.join("../analysis-fixtures"),
        manifest.join("analysis-fixtures"),
        manifest.join("tests/fixtures/analysis-fixtures"),
    ]
    .into_iter()
    .find(|candidate| candidate.is_dir())
}

/// 显式跳过开关：仅在明确设置时才允许跳过跨语言契约。
/// Explicit skip switch: the cross-language contract may only be skipped when set.
const SKIP_ENV: &str = "OSPF_SKIP_CROSS_LANGUAGE_FIXTURE";

fn skip_requested() -> bool {
    std::env::var(SKIP_ENV).is_ok_and(|value| !value.is_empty() && value != "0")
}

/// 仓内镜像目录（相对 crate 根）。 / In-repo mirror directory, relative to the crate root.
const MIRROR_RELATIVE: &str = "tests/fixtures/analysis-fixtures";

/// 契约文件清单。 / The contract file list.
const CONTRACT_FILES: [&str; 4] = [
    "analysis-contract.tsv",
    "analysis-cases.tsv",
    "checkpoint-wire-contract.tsv",
    "checkpoint-envelope-v3.json",
];

/// 去掉行尾符差异后再比较。
///
/// 契约文件在三个仓库间以 `* text=auto` 管理，Windows 检出会把 LF 变成 CRLF。
/// 两侧解析器都按行切分并 `trim_end()`，因此行尾符风格**不承载任何契约语义**；
/// 若按原始字节比较，跨平台检出会产生"内容一致但字节不同"的假失败。
///
/// Compare after normalizing line terminators. The contract files are tracked with
/// `* text=auto`, so a Windows checkout rewrites LF as CRLF. Both parsers split on lines
/// and `trim_end()`, so line-ending style carries **no contract meaning**; comparing raw
/// bytes would raise a false failure for identical content on a differently-configured clone.
fn line_normalized(bytes: &[u8]) -> Vec<u8> {
    String::from_utf8_lossy(bytes)
        .replace("\r\n", "\n")
        .into_bytes()
}

/// 守护仓内镜像与共享副本内容一致。
///
/// 单仓库检出的 CI 会读取镜像，而开发者本地读共享副本；两者漂移会导致
/// "本地绿、CI 红"或更糟的相反情况。因此只要两者同时存在就强制比较内容。
/// 单仓库检出时共享副本本就不存在（这正是镜像的意义），此时无事可比对，直接返回。
///
/// Guard that the in-repo mirror matches the shared copy. A single-repo CI checkout reads
/// the mirror while a developer reads the shared copy; drift between them produces the
/// worst outcome — one side green, the other red. Whenever both exist they must agree;
/// in a single-repo checkout there is nothing to compare.
#[test]
fn fixture_mirror_matches_shared_copy() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mirror = manifest.join(MIRROR_RELATIVE);
    let shared = manifest.join("../../analysis-fixtures");

    if !mirror.is_dir() || !shared.is_dir() {
        return;
    }

    for name in CONTRACT_FILES {
        let mirror_bytes = std::fs::read(mirror.join(name)).expect("mirror file should be readable");
        let shared_bytes = std::fs::read(shared.join(name)).expect("shared file should be readable");
        assert_eq!(
            line_normalized(&mirror_bytes),
            line_normalized(&shared_bytes),
            "{name} differs between the in-repo mirror and the shared copy; \
             they must stay in sync"
        );
    }
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

/// 取回 fixture 段落；缺失时**显式失败**，与 Kotlin 侧行为一致。
///
/// 跨语言契约只有在真正被断言时才有价值。这里绝不静默 `return`，
/// 否则 fixture 缺失会伪装成"测试通过"（历史上确实发生过）。
/// 唯一的跳过途径是显式设置 `OSPF_SKIP_CROSS_LANGUAGE_FIXTURE`，并打印醒目警告。
///
/// Fetch fixture sections; **fail loudly** when absent, matching the Kotlin side.
/// A cross-language contract is only worth something when actually asserted, so this
/// never silently `return`s — an absent fixture must not masquerade as a pass.
/// The only skip path is an explicit `OSPF_SKIP_CROSS_LANGUAGE_FIXTURE`, with a warning.
macro_rules! require_fixtures {
    ($sections:expr) => {
        match $sections {
            Some(sections) => sections,
            None => {
                if skip_requested() {
                    eprintln!(
                        "WARNING: skipping cross-language fixture contract because {SKIP_ENV} is set. \
                         This contract is NOT being verified."
                    );
                    return;
                }
                panic!(
                    "analysis-fixtures not found. The cross-language semantic contract cannot be \
                     verified. Expected it at one of: <repo>/../../analysis-fixtures, \
                     <repo>/../analysis-fixtures, <repo>/analysis-fixtures, \
                     <repo>/tests/fixtures/analysis-fixtures. \
                     Provide the shared directory, or set {SKIP_ENV}=1 to skip explicitly."
                );
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

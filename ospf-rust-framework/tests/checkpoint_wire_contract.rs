//! portable checkpoint envelope 的线格式契约测试 / Wire-contract test for the portable checkpoint envelope.
//!
//! 本文件逐段断言 `analysis-fixtures/checkpoint-wire-contract.tsv` 与 Rust 实现一致：字段名与顺序、
//! 取消来源规范代码、provenance 字段集、摘要可复算前提以及 envelope schema 版本。Kotlin 侧对同一份
//! 文件做等价断言，因此任一偏离都会被两侧同时拦下。
//!
//! This file asserts, section by section, that `analysis-fixtures/checkpoint-wire-contract.tsv` matches the
//! Rust implementation: field names and order, the canonical cancellation-origin codes, the provenance
//! field set, the digest-recomputation preconditions, and the envelope schema version. The Kotlin side
//! asserts the same file, so any drift is caught on both sides.

#![cfg(feature = "remote-solver")]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use ospf_rust_core::solver::{
    AuditFingerprint, CancellationOrigin, CancellationRecord, SolveCheckpoint, SolveCheckpointArtifact,
    SolverProvenance, sha256_fingerprint,
};
use ospf_rust_framework::solver::remote::{
    CURRENT_REMOTE_CHECKPOINT_ARTIFACT_SCHEMA_VERSION, PortableCancellationRecord,
    PortableSolverProvenance, RUST_CHECKPOINT_RESIDUAL_METADATA_PREFIX,
    RemoteCheckpointArtifactDto, RemoteSolverErrorCode, checkpoint_artifact_from_json,
    checkpoint_artifact_to_json,
};

// ============================================================================
// fixture 读取 / Fixture loading
// ============================================================================

/// 契约文件在三个仓库间共享，仓内镜像位于 core crate。 / The contract is shared across repositories; its in-repo mirror lives in the core crate.
const CONTRACT_FILE: &str = "checkpoint-wire-contract.tsv";

/// 由 Rust 序列化产出的完整 envelope，用作跨语言互操作夹具。
/// A complete envelope produced by Rust serialization, used as the cross-language interop fixture.
const CROSS_LANGUAGE_ENVELOPE: &str = "checkpoint-envelope-v3.json";

/// 显式跳过开关：与 core 的 `fixture_contract` 测试共用同一变量。
/// Explicit skip switch, shared with the core `fixture_contract` test.
const SKIP_ENV: &str = "OSPF_SKIP_CROSS_LANGUAGE_FIXTURE";

/// 定位契约文件；缺失时返回 `None`，由 `require_contract!` 决定失败还是显式跳过。
///
/// 候选顺序：本 crate 的 fixture 目录、core crate 的仓内镜像、仓库同级的共享目录。仓内镜像保证
/// 单仓库检出（CI、外部贡献者）也能真实执行契约测试，而不是静默通过。
///
/// Locate the contract file, returning `None` when absent so `require_contract!` decides between a loud
/// failure and an explicit skip. Candidates are: this crate's fixture directory, the core crate's in-repo
/// mirror, and the shared sibling directory. The mirror keeps a single-repository checkout (CI, outside
/// contributors) genuinely asserting the contract instead of passing silently.
fn contract_path() -> Option<PathBuf> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    [
        manifest.join("tests/fixtures/analysis-fixtures"),
        manifest.join("../ospf-rust-core/tests/fixtures/analysis-fixtures"),
        manifest.join("../../analysis-fixtures"),
        manifest.join("../analysis-fixtures"),
    ]
    .into_iter()
    .map(|root| root.join(CONTRACT_FILE))
    .find(|candidate| candidate.is_file())
}

fn skip_requested() -> bool {
    std::env::var(SKIP_ENV).is_ok_and(|value| !value.is_empty() && value != "0")
}

/// 定位共享的跨语言 envelope 夹具。 / Locate the shared cross-language envelope fixture.
///
/// 候选顺序与 [`contract_path`] 一致：本 crate 的夹具目录、core 仓内镜像、以及共享的同级目录。
/// 两侧必须读取**同一份**文档，否则"两侧一致"只是各自对自己实现的复述。
///
/// Candidates match [`contract_path`]: this crate's fixture directory, the core crate's in-repo
/// mirror, and the shared sibling directory. Both sides must read the **same** document, otherwise
/// "the two sides agree" merely restates each side's own output.
fn cross_language_envelope_path() -> Option<PathBuf> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    [
        manifest.join("tests/fixtures/analysis-fixtures"),
        manifest.join("../ospf-rust-core/tests/fixtures/analysis-fixtures"),
        manifest.join("../../analysis-fixtures"),
        manifest.join("../analysis-fixtures"),
    ]
    .into_iter()
    .map(|root| root.join(CROSS_LANGUAGE_ENVELOPE))
    .find(|candidate| candidate.is_file())
}

/// 段落表：段名 → 行 → 列 / Section table: section name → rows → columns.
type Sections = BTreeMap<String, Vec<Vec<String>>>;

/// 读取契约文件；缺失时**显式失败**，绝不静默通过。
///
/// 跨语言契约只有在真正被断言时才有价值，因此 fixture 缺失必须让测试变红。唯一的跳过途径是显式
/// 设置 `OSPF_SKIP_CROSS_LANGUAGE_FIXTURE`，并打印醒目警告。
///
/// Read the contract file and **fail loudly** when it is absent; never pass silently. A cross-language
/// contract is only worth something when actually asserted. The only skip path is an explicit
/// `OSPF_SKIP_CROSS_LANGUAGE_FIXTURE`, with a warning.
macro_rules! require_contract {
    () => {
        match read_contract_sections() {
            Some(sections) => sections,
            None => {
                if skip_requested() {
                    eprintln!(
                        "WARNING: skipping the checkpoint wire contract because {SKIP_ENV} is set. \
                         This contract is NOT being verified."
                    );
                    return;
                }
                panic!(
                    "checkpoint wire contract not found. The cross-language wire contract cannot be \
                     verified. Expected {CONTRACT_FILE} at one of: <framework>/tests/fixtures/\
                     analysis-fixtures, <framework>/../ospf-rust-core/tests/fixtures/analysis-fixtures, \
                     <framework>/../../analysis-fixtures. Provide the shared directory, or set \
                     {SKIP_ENV}=1 to skip explicitly."
                );
            }
        }
    };
}

/// 解析契约文件的 `[section]` 结构，行格式为 TAB 分隔或单行 `key=value`。
///
/// Parse the contract file's `[section]` structure; a row is either TAB-separated or a single `key=value`.
fn read_contract_sections() -> Option<Sections> {
    let text = std::fs::read_to_string(contract_path()?).ok()?;
    let mut sections: Sections = BTreeMap::new();
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

/// 取某段的 jsonName 列（第 2 列）/ Take the jsonName column (column 2) of one section.
fn field_names(sections: &Sections, section: &str) -> Vec<String> {
    sections[section]
        .iter()
        .map(|row| {
            assert!(row.len() >= 4, "contract row in [{section}] must carry 4 columns: {row:?}");
            row[1].clone()
        })
        .collect()
}

// ============================================================================
// JSON 与摘要辅助 / JSON and digest helpers
// ============================================================================

/// 取回顶层 key 序列（含数量），用于逐字校验序列化顺序。
///
/// `serde_json::Value` 的 map 会按键排序，无法反映序列化顺序，因此这里直接扫描 JSON 文本：只有位于
/// 根对象内且后跟冒号的字符串字面量才是键。
///
/// Collect the top-level key sequence (with its count) to check serialization order verbatim. A
/// `serde_json::Value` map sorts its keys and therefore cannot reveal serialization order, so this scans the
/// JSON text: a string literal followed by a colon inside the root object is a key.
fn top_level_json_keys(json: &str) -> Vec<String> {
    let bytes = json.as_bytes();
    let mut index = 0;
    let mut depth = 0usize;
    let mut keys = Vec::new();
    while index < bytes.len() {
        match bytes[index] {
            b'{' | b'[' => {
                depth += 1;
                index += 1;
            }
            b'}' | b']' => {
                depth = depth.saturating_sub(1);
                index += 1;
            }
            b'"' => {
                let mut cursor = index + 1;
                let mut escaped = false;
                let mut text = String::new();
                while cursor < bytes.len() {
                    let byte = bytes[cursor];
                    if escaped {
                        text.push(byte as char);
                        escaped = false;
                    } else if byte == b'\\' {
                        escaped = true;
                    } else if byte == b'"' {
                        break;
                    } else {
                        text.push(byte as char);
                    }
                    cursor += 1;
                }
                let mut lookahead = cursor + 1;
                while lookahead < bytes.len() && bytes[lookahead].is_ascii_whitespace() {
                    lookahead += 1;
                }
                if depth == 1 && lookahead < bytes.len() && bytes[lookahead] == b':' {
                    keys.push(text);
                }
                index = cursor + 1;
            }
            _ => index += 1,
        }
    }
    keys
}

fn hex_sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// 摘要排除的字段：置空后计算 SHA-256 / The digest-excluded field: blanked before hashing.
fn digest_of(dto: &RemoteCheckpointArtifactDto) -> String {
    let mut unsigned = dto.clone();
    unsigned.integrity_sha256.clear();
    let bytes = serde_json::to_vec(&unsigned).expect("checkpoint envelope should serialize");
    hex_sha256(&bytes)
}

/// 重算并写回摘要，使手工改动的 envelope 仍满足完整性校验。
/// Recompute and store the digest so a hand-modified envelope still passes the integrity check.
fn reseal(dto: &mut RemoteCheckpointArtifactDto) {
    dto.integrity_sha256 = digest_of(dto);
}

// ============================================================================
// 被测数据 / Test data
// ============================================================================

fn fingerprint(value: &str) -> AuditFingerprint {
    AuditFingerprint {
        schema_version: "1.0".to_owned(),
        algorithm: "sha256".to_owned(),
        value: value.to_owned(),
    }
}

/// 一个最小但完整的 portable checkpoint artifact / A minimal yet complete portable checkpoint artifact.
fn artifact_fixture() -> SolveCheckpointArtifact {
    let state = b"portable-state-v3".to_vec();
    let mut checkpoint = SolveCheckpoint::new(
        "run-wire",
        "attempt-wire",
        None,
        fingerprint("model"),
        fingerprint("configuration"),
        fingerprint("solver"),
        SolverProvenance {
            solver_id: "rust-cp/1.0".to_owned(),
            backend_name: "fake".to_owned(),
            backend_version: Some("9.9".to_owned()),
            requested_configuration: BTreeMap::from([("threads".to_owned(), "4".to_owned())]),
            effective_configuration: BTreeMap::from([("threads".to_owned(), "4".to_owned())]),
            thread_count: Some(4),
            random_seed: Some(7),
            deterministic: Some(true),
            environment_summary: BTreeMap::from([("host".to_owned(), "node-1".to_owned())]),
            ..SolverProvenance::default()
        },
        2,
        Some(3.0),
        Some(2.0),
        Some(1.0 / 3.0),
        fingerprint("state"),
    )
    .expect("checkpoint fixture should be valid");
    checkpoint.state_digest = sha256_fingerprint("ospf.solve.checkpoint.state", &state);
    checkpoint
        .with_state(state)
        .expect("artifact fixture should be valid")
}

fn encoded_envelope(artifact: &SolveCheckpointArtifact) -> Vec<u8> {
    checkpoint_artifact_to_json(artifact).expect("checkpoint envelope should encode")
}

/// 用给定的 provenance 字面量与摘要拼出一份 3.0 envelope。
///
/// `integrity` 传空串即可得到用于计算摘要的"置空形态"，再传入算出的摘要即得到可校验的 envelope。
///
/// Assemble a schema-3.0 envelope from a provenance literal and a digest. Passing an empty `integrity`
/// yields the blank form used for digest computation; passing the computed digest then yields a
/// verifiable envelope.
fn provenance_envelope(provenance: &str, model_fingerprint: &str, integrity: &str) -> String {
    format!(
        concat!(
            r#"{{"schemaVersion":"3.0","sourceFormat":"v2","migratedFromLegacy":false,"#,
            r#""checkpointId":"wire-provenance","identitySchemaVersion":"1.0","#,
            r#""identityNamespace":"ospf-rust","modelName":"provenance","modelFingerprint":"{fingerprint}","#,
            r#""configurationFingerprint":"config","solverFingerprint":"solver","runId":"run-provenance","#,
            r#""attemptId":"attempt-provenance","parentCheckpointId":null,"createdAtEpochMs":1700000000000,"#,
            r#""snapshotJson":"{{}}","incumbent":null,"bestBound":null,"gap":null,"assumptions":[],"#,
            r#""conflicts":[],"benders":null,"cancellationChain":[],"provenance":{provenance},"#,
            r#""integritySha256":"{integrity}"}}"#
        ),
        fingerprint = model_fingerprint,
        provenance = provenance,
        integrity = integrity
    )
}

// ============================================================================
// 契约测试 / Contract tests
// ============================================================================

#[test]
fn envelope_with_null_provenance_materializes_without_fabricating_one() {
    // 契约把 `provenance` 定义为**可空**，Kotlin 的 `capture(provenance = null)` 是合法输入。
    // 因此 Rust 必须能物化这类信封。此前 `into_artifact` 把 null 兜底为
    // `SolverProvenance::default()`（全空字符串），而核心 `SolveCheckpoint::validate` 要求
    // solver_id / backend_name 非空，于是对端信封会被**误判为损坏**而拒绝——
    // 表面像是"跨语言不兼容"，实际是 Rust 单方把可空字段当成了必填。
    //
    // The contract defines `provenance` as **nullable**, and Kotlin's `capture(provenance = null)`
    // is legitimate input, so Rust must be able to materialize such an envelope. Previously
    // `into_artifact` fell back to `SolverProvenance::default()` (all-blank strings) while the core
    // `SolveCheckpoint::validate` requires a non-blank solver_id / backend_name, so a peer envelope
    // was **misread as corrupt** and rejected: it looked like cross-language incompatibility when
    // Rust alone was treating a nullable field as required.
    let model_fingerprint = hex_sha256(b"{}");
    let blank = provenance_envelope("null", &model_fingerprint, "");
    let envelope = provenance_envelope("null", &model_fingerprint, &hex_sha256(blank.as_bytes()));
    let mut dto: RemoteCheckpointArtifactDto =
        serde_json::from_str(&envelope).expect("envelope should decode");
    reseal(&mut dto);
    assert!(
        dto.provenance.is_none(),
        "the fixture must actually carry provenance: null"
    );

    let json = serde_json::to_vec(&dto).expect("sealed envelope should encode");
    let artifact = checkpoint_artifact_from_json(&json)
        .expect("an envelope carrying provenance: null must materialize");

    // 缺失 provenance 不得被替换成一份**看起来真实**的来源；它必须留下可识别的"未知"标记。
    // A missing provenance must not be replaced by a plausible-looking origin; it has to leave a
    // recognizable "unknown" marker instead.
    let provenance = &artifact.checkpoint.provenance;
    assert!(
        !provenance.solver_id.trim().is_empty() && !provenance.backend_name.trim().is_empty(),
        "materialized provenance must satisfy the core validation contract"
    );
    assert_eq!(
        provenance.solver_id, "unknown",
        "an absent provenance must be marked unknown rather than fabricated"
    );
    assert_eq!(provenance.backend_name, "unknown");
    assert!(
        provenance.effective_configuration.is_empty()
            && provenance.environment_summary.is_empty(),
        "an absent provenance must not invent configuration or environment data"
    );

    // 重新编码后仍必须是显式的 `unknown` 来源，而不是被悄悄升级为"真实"来源。
    // Re-encoding must still carry the explicit `unknown` origin, never silently upgrade it to a
    // plausible one.
    let reencoded = encoded_envelope(&artifact);
    let round_tripped: RemoteCheckpointArtifactDto =
        serde_json::from_slice(&reencoded).expect("re-encoded envelope should decode");
    let provenance = round_tripped
        .provenance
        .as_ref()
        .expect("provenance must stay explicit rather than disappearing");
    assert_eq!(provenance.solver_id, "unknown");
    assert_eq!(provenance.backend_name, "unknown");
}

#[test]
fn checkpoint_envelope_fields_match_the_shared_contract() {
    let sections = require_contract!();
    let expected = field_names(&sections, "envelope-field");
    assert_eq!(expected.len(), 25, "envelope must declare 25 wire fields");

    // 契约版本与实现常量必须一致 / The contract version and the implementation constant must agree.
    let schema = &sections["schema"][0];
    assert_eq!(schema[0], "schema_version");
    assert_eq!(schema[1], CURRENT_REMOTE_CHECKPOINT_ARTIFACT_SCHEMA_VERSION);
    assert_eq!(schema[1], "3.0");

    // 顶层 key 序列必须**恰好等于**契约列出的字段名序列（含数量）。
    // The top-level key sequence must be **exactly** the contract's field-name sequence, count included.
    let artifact = artifact_fixture();
    let json = String::from_utf8(encoded_envelope(&artifact)).expect("envelope should be UTF-8");
    let keys = top_level_json_keys(&json);
    assert_eq!(
        keys.len(),
        expected.len(),
        "the envelope must emit exactly the contract's field count"
    );
    assert_eq!(keys, expected);

    // 可空字段必须显式输出 `null`，数组字段必须显式输出 `[]`；缺失的 key 同样视为违约。
    // Nullable fields must emit an explicit `null` and array fields an explicit `[]`; a missing key fails too.
    let minimal = r#"{"schemaVersion":"3.0","sourceFormat":"v2","checkpointId":"minimal-checkpoint","identitySchemaVersion":"1.0","identityNamespace":"model-local","modelName":"minimal","modelFingerprint":"00","createdAtEpochMs":1,"snapshotJson":"{}"}"#;
    let dto: RemoteCheckpointArtifactDto =
        serde_json::from_str(minimal).expect("a minimal envelope should decode through defaults");
    let reserialized = serde_json::to_string(&dto).expect("minimal envelope should re-encode");
    assert_eq!(top_level_json_keys(&reserialized), expected);
    let value: serde_json::Value =
        serde_json::from_str(&reserialized).expect("re-encoded envelope should parse");
    let object = value.as_object().expect("envelope should be a JSON object");
    for row in &sections["envelope-field"] {
        let (name, json_type, nullable) = (&row[1], &row[2], &row[3]);
        let field = object
            .get(name)
            .unwrap_or_else(|| panic!("nullable/default field {name} must not be omitted"));
        if nullable == "yes" {
            assert!(field.is_null(), "nullable field {name} must be emitted as null");
        } else if json_type.ends_with("[]") {
            assert!(field.is_array(), "array field {name} must be emitted as an array");
        }
    }
}

#[test]
fn portable_cancellation_and_provenance_shapes_match_the_shared_contract() {
    let sections = require_contract!();

    // 取消记录的形状 / The shape of one cancellation record.
    let record = PortableCancellationRecord {
        origin: CancellationOrigin::User.to_wire_code().to_owned(),
        requested_at_epoch_ms: 1_700_000_000_000,
        reason: None,
    };
    let record_json = serde_json::to_string(&record).expect("portable cancellation record should encode");
    assert_eq!(
        top_level_json_keys(&record_json),
        field_names(&sections, "cancellation-record")
    );
    assert!(
        record_json.contains(r#""reason":null"#),
        "a nullable reason must be emitted explicitly: {record_json}"
    );

    // provenance 的形状 / The shape of provenance.
    let provenance = PortableSolverProvenance {
        solver_id: "rust-cp/1.0".to_owned(),
        backend_name: "fake".to_owned(),
        ..PortableSolverProvenance::default()
    };
    let provenance_json =
        serde_json::to_string(&provenance).expect("portable provenance should encode");
    assert_eq!(
        top_level_json_keys(&provenance_json),
        field_names(&sections, "provenance-field")
    );
    for field in ["backendVersion", "pluginVersion", "threadCount", "randomSeed", "deterministic"] {
        assert!(
            provenance_json.contains(&format!(r#""{field}":null"#)),
            "nullable provenance field {field} must be emitted explicitly: {provenance_json}"
        );
    }
    for field in ["requestedConfiguration", "effectiveConfiguration", "environmentSummary"] {
        assert!(
            provenance_json.contains(&format!(r#""{field}":{{}}"#)),
            "empty provenance map {field} must be emitted explicitly: {provenance_json}"
        );
    }
}

#[test]
fn cancellation_origin_codes_match_the_shared_contract() {
    let sections = require_contract!();
    let rows = &sections["cancellation-origin"];
    assert!(!rows.is_empty(), "the contract must list the origin vocabulary");

    for row in rows {
        let (code, rust_variant, kotlin_variant) = (row[0].as_str(), row[1].as_str(), row[2].as_str());
        assert!(!kotlin_variant.trim().is_empty(), "every row must name its Kotlin variant");
        if rust_variant == "Other" {
            // 契约把该代码映射到 Rust 的兜底变体，代码文本必须原样保留。
            // The contract maps this code to Rust's catch-all variant and keeps the code text verbatim.
            assert_eq!(
                CancellationOrigin::from_wire_code(code),
                CancellationOrigin::Other(code.to_owned())
            );
            assert_eq!(
                CancellationOrigin::Other(code.to_owned()).to_wire_code(),
                code
            );
            continue;
        }
        let origin = rust_variant_named(rust_variant);
        assert_eq!(origin.to_wire_code(), code, "wire code for {rust_variant}");
        assert_eq!(
            CancellationOrigin::from_wire_code(code),
            origin,
            "round trip for {rust_variant}"
        );
    }
}

/// 把契约里的 Rust 变体名解析为枚举值 / Resolve a contract Rust-variant name into an enum value.
fn rust_variant_named(name: &str) -> CancellationOrigin {
    match name {
        "User" => CancellationOrigin::User,
        "External" => CancellationOrigin::External,
        "Callback" => CancellationOrigin::Callback,
        "FrameworkLoser" => CancellationOrigin::FrameworkLoser,
        "RemoteStop" => CancellationOrigin::RemoteStop,
        "TokioTaskAbort" => CancellationOrigin::TokioTaskAbort,
        "Backend" => CancellationOrigin::Backend,
        other => panic!("unknown Rust cancellation-origin variant in the contract: {other}"),
    }
}

#[test]
fn cancellation_chain_round_trips_origin_codes_timestamps_and_reasons() {
    let mut artifact = artifact_fixture();
    let chain = [
        (CancellationOrigin::User, 1_700_000_000_101u64),
        (CancellationOrigin::RemoteStop, 1_700_000_000_102),
        // 契约无法识别的来源必须原样保留文本 / A source outside the vocabulary must keep its text verbatim.
        (CancellationOrigin::Other("vendor-supervisor".to_owned()), 1_700_000_000_103),
    ];
    for (origin, requested_at_epoch_ms) in chain.clone() {
        artifact
            .checkpoint
            .record_cancellation(CancellationRecord {
                origin,
                requested_at_epoch_ms,
                reason: None,
            })
            .expect("the fixture cancellation chain should be ordered");
    }

    let bytes = encoded_envelope(&artifact);
    let dto: RemoteCheckpointArtifactDto =
        serde_json::from_slice(&bytes).expect("envelope should decode");
    assert_eq!(
        dto.cancellation_chain
            .iter()
            .map(|record| record.origin.as_str())
            .collect::<Vec<_>>(),
        ["user", "remoteStop", "vendor-supervisor"],
        "the wire must carry canonical origin codes"
    );
    assert_eq!(
        dto.cancellation_chain
            .iter()
            .map(|record| record.requested_at_epoch_ms)
            .collect::<Vec<_>>(),
        [1_700_000_000_101, 1_700_000_000_102, 1_700_000_000_103]
    );
    assert!(
        dto.cancellation_chain.iter().all(|record| record.reason.is_none()),
        "the core cancellation fact carries no reason"
    );

    // artifact 往返：来源（含未知文本）与时间戳必须完全保持。
    // Artifact round trip: origins (unknown text included) and timestamps must be preserved exactly.
    let restored = checkpoint_artifact_from_json(&bytes).expect("envelope should materialize");
    assert_eq!(restored.checkpoint.cancellation_chain, artifact.checkpoint.cancellation_chain);
    assert_eq!(
        restored.checkpoint.cancellation_chain[2].origin,
        CancellationOrigin::Other("vendor-supervisor".to_owned())
    );

    // 线格式往返：`reason` 是契约字段，必须在 DTO 编解码中保持（含显式 null）。
    // Wire round trip: `reason` is a contract field and must survive DTO encode/decode (null included).
    let mut with_reasons = dto;
    with_reasons.cancellation_chain[0].reason = Some("operator pressed stop".to_owned());
    with_reasons.cancellation_chain[1].reason = None;
    with_reasons.cancellation_chain[2].reason = Some("vendor supervisor requested a stop".to_owned());
    reseal(&mut with_reasons);
    let encoded = serde_json::to_vec(&with_reasons).expect("sealed envelope should encode");
    assert!(
        String::from_utf8_lossy(&encoded).contains(r#""reason":null"#),
        "an absent reason must stay an explicit null"
    );
    let decoded: RemoteCheckpointArtifactDto =
        serde_json::from_slice(&encoded).expect("sealed envelope should decode");
    assert_eq!(
        decoded
            .cancellation_chain
            .iter()
            .map(|record| record.reason.clone())
            .collect::<Vec<_>>(),
        [
            Some("operator pressed stop".to_owned()),
            None,
            Some("vendor supervisor requested a stop".to_owned()),
        ]
    );
    assert_eq!(decoded.cancellation_chain, with_reasons.cancellation_chain);
    // 关键：原因必须能穿过**物化**进入核心 `CancellationRecord`，而不只是在 DTO 层往返。
    // 只断言 envelope "仍然校验通过" 会让 `from_record` 丢弃原因的行为悄悄通过——那样线格式上
    // 的原因会在 DTO → artifact 的转换中被吞掉，审计轨迹随之失真。
    //
    // Critical: a reason must survive **materialization** into the core `CancellationRecord`, not
    // merely round-trip at the DTO level. Asserting only that the envelope "still validates" would
    // let `from_record` silently drop the reason, so a wire reason would be swallowed by the
    // DTO → artifact conversion and the audit trail would lose it.
    let materialized =
        checkpoint_artifact_from_json(&encoded).expect("sealed envelope should still validate");
    assert_eq!(
        materialized
            .checkpoint
            .cancellation_chain
            .iter()
            .map(|record| record.reason.clone())
            .collect::<Vec<_>>(),
        [
            Some("operator pressed stop".to_owned()),
            None,
            Some("vendor supervisor requested a stop".to_owned()),
        ],
        "materializing the envelope must preserve every cancellation reason"
    );

    // 反向物化：由核心 artifact 重新编码后，原因同样不得丢失。
    // Reverse direction: re-encoding from the core artifact must not lose the reasons either.
    let reencoded = encoded_envelope(&materialized);
    let round_tripped: RemoteCheckpointArtifactDto =
        serde_json::from_slice(&reencoded).expect("re-encoded envelope should decode");
    assert_eq!(
        round_tripped
            .cancellation_chain
            .iter()
            .map(|record| record.reason.clone())
            .collect::<Vec<_>>(),
        [
            Some("operator pressed stop".to_owned()),
            None,
            Some("vendor supervisor requested a stop".to_owned()),
        ],
        "re-encoding a core artifact must preserve every cancellation reason"
    );
}

#[test]
fn provenance_round_trips_maps_in_ascending_key_order() {
    // 手工构造一份 provenance map 乱序的 envelope，模拟对端未排序或字段顺序不同的编码。
    // Hand-build an envelope whose provenance maps are not sorted, simulating a peer that emits a
    // different key order.
    let shuffled = concat!(
        r#"{"solverId":"rust-cp/1.0","backendName":"fake","backendVersion":"9.9","pluginVersion":null,"#,
        r#""requestedConfiguration":{"zeta":"26","alpha":"1","mid":"13"},"#,
        r#""effectiveConfiguration":{"threads":"4","deterministic":"true","barrier":"2"},"#,
        r#""threadCount":8,"randomSeed":42,"deterministic":false,"#,
        r#""environmentSummary":{"region":"eu","host":"node-7","arch":"x86_64"}}"#
    );
    let model_fingerprint = hex_sha256(b"{}");
    let blank = provenance_envelope(shuffled, &model_fingerprint, "");
    let envelope = provenance_envelope(shuffled, &model_fingerprint, &hex_sha256(blank.as_bytes()));

    // 解码即归一化：map 按键升序进入 BTreeMap，值不得被重排。
    // Decoding normalizes: keys land in a BTreeMap in ascending order while values keep their pairing.
    let dto: RemoteCheckpointArtifactDto =
        serde_json::from_str(&blank).expect("shuffled envelope should decode");
    let provenance = dto
        .provenance
        .clone()
        .expect("shuffled envelope should carry provenance");
    let requested_keys: Vec<&str> = provenance
        .requested_configuration
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(requested_keys, ["alpha", "mid", "zeta"]);
    assert_eq!(
        provenance.requested_configuration.values().cloned().collect::<Vec<_>>(),
        ["1", "13", "26"],
        "sorting keys must not reorder values"
    );
    assert_eq!(provenance.thread_count, Some(8));
    assert_eq!(provenance.random_seed, Some(42));
    assert_eq!(provenance.deterministic, Some(false));
    assert_eq!(provenance.backend_version.as_deref(), Some("9.9"));

    // 乱序 map 无法通过完整性校验：摘要是对排序后的规范形态计算，而 `[canonicalization]` 明确要求 map
    // 升序输出。这条断言把这个前提钉住，避免"只要解码成功就算合规"的错误结论。
    //
    // A shuffled map can never pass the integrity check: the digest covers the sorted canonical form and
    // `[canonicalization]` explicitly requires ascending map output. This pins that precondition down.
    let error = checkpoint_artifact_from_json(envelope.as_bytes())
        .expect_err("a shuffled map must not be accepted as a canonical envelope");
    assert_eq!(error.code, RemoteSolverErrorCode::CheckpointRestoreFailed);

    // 归一化后的规范 envelope 必须走通 artifact 往返，并保持 provenance 与排序。
    // The normalized canonical envelope must survive the artifact round trip with its provenance and ordering.
    let mut canonical_dto = dto;
    reseal(&mut canonical_dto);
    let canonical = serde_json::to_vec(&canonical_dto).expect("canonical envelope should encode");
    let artifact = checkpoint_artifact_from_json(&canonical)
        .expect("canonical envelope should materialize as a Rust artifact");
    let reencoded =
        String::from_utf8(encoded_envelope(&artifact)).expect("re-encoded envelope should be UTF-8");
    for expected in [
        r#""requestedConfiguration":{"alpha":"1","mid":"13","zeta":"26"}"#,
        r#""effectiveConfiguration":{"barrier":"2","deterministic":"true","threads":"4"}"#,
        r#""environmentSummary":{"arch":"x86_64","host":"node-7","region":"eu"}"#,
    ] {
        assert!(
            reencoded.contains(expected),
            "re-encoded provenance must sort map keys ascending, missing: {expected}"
        );
    }
    let reencoded_dto: RemoteCheckpointArtifactDto =
        serde_json::from_str(&reencoded).expect("re-encoded envelope should decode");
    let restored = reencoded_dto
        .provenance
        .as_ref()
        .expect("re-encoded envelope should keep provenance");
    assert_eq!(
        restored.requested_configuration,
        provenance.requested_configuration
    );
    assert_eq!(
        restored.effective_configuration,
        provenance.effective_configuration
    );
    assert_eq!(restored.environment_summary, provenance.environment_summary);
    assert_eq!(restored.thread_count, Some(8));
    assert_eq!(restored.random_seed, Some(42));
    assert_eq!(restored.deterministic, Some(false));
    reencoded_dto.validate().expect("re-encoded envelope should validate");
}

#[test]
fn integrity_digest_is_recomputable_from_its_own_payload() {
    let mut artifact = artifact_fixture();
    artifact
        .checkpoint
        .record_cancellation(CancellationRecord {
            origin: CancellationOrigin::User,
            requested_at_epoch_ms: 1_700_000_000_101,
            reason: None,
        })
        .expect("the fixture cancellation chain should be valid");
    let bytes = encoded_envelope(&artifact);
    let dto: RemoteCheckpointArtifactDto =
        serde_json::from_slice(&bytes).expect("envelope should decode");

    // 摘要可复算的前提：字段顺序、显式 null 与升序 map 都由 DTO 自身保证。
    // Recomputation preconditions — field order, explicit nulls, and ascending maps — are all carried by the
    // DTO itself.
    assert_eq!(dto.integrity_sha256, digest_of(&dto));
    assert_eq!(
        serde_json::to_vec(&dto).expect("DTO should re-encode"),
        bytes,
        "re-encoding a decoded envelope must reproduce the canonical bytes"
    );

    // 摘要必须覆盖两个新增的一等字段，否则改动它们不会破坏完整性。
    // The digest must cover the two new first-class fields, otherwise changing them would not break integrity.
    let mut chain_changed = dto.clone();
    chain_changed
        .cancellation_chain
        .push(PortableCancellationRecord {
            origin: CancellationOrigin::Callback.to_wire_code().to_owned(),
            requested_at_epoch_ms: 1_700_000_000_200,
            reason: None,
        });
    assert_ne!(digest_of(&chain_changed), dto.integrity_sha256);

    let mut provenance_changed = dto.clone();
    provenance_changed
        .provenance
        .as_mut()
        .expect("fixture provenance")
        .thread_count = Some(99);
    assert_ne!(digest_of(&provenance_changed), dto.integrity_sha256);

    // 篡改任一被摘要覆盖的字段后，未重算摘要的 envelope 必须被拒绝。
    // After tampering with any digested field, an envelope with a stale digest must be rejected.
    let mut tampered = dto;
    tampered.cancellation_chain[0].requested_at_epoch_ms = 1;
    let tampered_bytes = serde_json::to_vec(&tampered).expect("tampered envelope should encode");
    let error = checkpoint_artifact_from_json(&tampered_bytes)
        .expect_err("a stale digest must be rejected");
    assert_eq!(error.code, RemoteSolverErrorCode::CheckpointRestoreFailed);

    // 重算摘要后同一改动被接受，说明摘要确实是"对置空后的自身 JSON"求 SHA-256。
    // Resealing the same change is accepted, proving the digest is SHA-256 over the envelope's own JSON with
    // the digest field blanked.
    let mut resealed = tampered;
    reseal(&mut resealed);
    let resealed_bytes = serde_json::to_vec(&resealed).expect("resealed envelope should encode");
    let restored = checkpoint_artifact_from_json(&resealed_bytes)
        .expect("a resealed envelope should be accepted");
    assert_eq!(
        restored.checkpoint.cancellation_chain[0].requested_at_epoch_ms,
        1,
        "resealing must not lose the changed value"
    );
}

#[test]
fn checkpoint_schema_version_is_three_zero_and_legacy_two_zero_is_rejected() {
    let artifact = artifact_fixture();
    let bytes = encoded_envelope(&artifact);
    let mut value: serde_json::Value =
        serde_json::from_slice(&bytes).expect("envelope should parse as JSON");
    assert_eq!(value["schemaVersion"], serde_json::Value::from("3.0"));
    // 契约未规定 `sourceFormat` 取值，这里固定为 Kotlin codec 仍接受的 `v2`。
    // The contract does not pin `sourceFormat`; it stays `v2`, which the Kotlin codec still accepts.
    assert_eq!(value["sourceFormat"], serde_json::Value::from("v2"));

    // 旧 2.0 envelope（摘要有意保持原值）必须被 schema 校验拦下。
    // A legacy 2.0 envelope — digest left untouched on purpose — must be blocked by the schema check.
    value["schemaVersion"] = serde_json::Value::from("2.0");
    let legacy = serde_json::to_vec(&value).expect("legacy envelope should encode");
    let dto: RemoteCheckpointArtifactDto =
        serde_json::from_slice(&legacy).expect("legacy envelope should still decode");
    let error = dto.validate().expect_err("schema 2.0 must be rejected");
    assert_eq!(error.code, RemoteSolverErrorCode::UnsupportedProtocolVersion);
    let error =
        checkpoint_artifact_from_json(&legacy).expect_err("schema 2.0 must not materialize");
    assert_eq!(error.code, RemoteSolverErrorCode::UnsupportedProtocolVersion);

    // Kotlin 3.0 envelope 必须被接受，且常量本身不能再回落到 2.0。
    // A Kotlin 3.0 envelope must be accepted, and the constant itself must not fall back to 2.0.
    assert_eq!(CURRENT_REMOTE_CHECKPOINT_ARTIFACT_SCHEMA_VERSION, "3.0");
    checkpoint_artifact_from_json(&bytes).expect("schema 3.0 envelope should be accepted");
}

/// 契约文件必须真实存在，否则上面的测试无法证明任何事。
/// The contract file must really exist, otherwise the tests above prove nothing.
#[test]
fn shared_cross_language_envelope_is_accepted_and_its_digest_recomputes() {
    // **跨语言互操作的判据**：`analysis-fixtures/checkpoint-envelope-v3.json` 由 Rust 序列化产出，
    // 并被 Kotlin 侧的 `CheckpointWireContractTest` 同时读取。Rust 重新解码它、复算摘要、再编码，
    // 必须与原文档**逐字节相同** —— 任何字段顺序、null 输出或 map 键序的分歧都会破坏摘要。
    //
    // 两侧读取**同一份共享文档**（而非各自的副本），因此"两侧一致"是被真正验证的，而不是各自
    // 对着自己的实现自证。
    //
    // **The cross-language interoperability criterion**: `analysis-fixtures/checkpoint-envelope-v3.json`
    // is produced by Rust serialization and is read by Kotlin's `CheckpointWireContractTest` as well.
    // Rust must decode it, recompute its digest, and re-encode to a document **byte-identical** to the
    // original — any divergence in field order, null emission, or map key ordering breaks the digest.
    //
    // Both sides read the **same shared document** rather than per-side copies, so "the two sides
    // agree" is genuinely verified instead of each side proving itself against its own output.
    let Some(path) = cross_language_envelope_path() else {
        if skip_requested() {
            return;
        }
        panic!(
            "checkpoint-envelope-v3.json not found next to the contract fixture; set {} to skip explicitly",
            SKIP_ENV
        );
    };
    let document = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    let document = document.trim_end().to_owned();

    let dto: RemoteCheckpointArtifactDto =
        serde_json::from_str(&document).expect("the shared envelope must decode");
    assert_eq!(
        dto.integrity_sha256,
        digest_of(&dto),
        "Rust must recompute the digest written into the shared document"
    );

    // 一等字段必须完整可读。 / The first-class fields must read back completely.
    assert_eq!(dto.cancellation_chain.len(), 2);
    assert_eq!(dto.cancellation_chain[0].origin, "remoteStop");
    assert_eq!(dto.cancellation_chain[0].requested_at_epoch_ms, 1_700_000_000_001);
    assert_eq!(
        dto.cancellation_chain[0].reason.as_deref(),
        Some("remote dispatcher stopped the attempt")
    );
    assert_eq!(dto.cancellation_chain[1].origin, "frameworkLoser");
    assert_eq!(dto.cancellation_chain[1].reason, None);

    let provenance = dto.provenance.as_ref().expect("shared envelope carries provenance");
    assert_eq!(provenance.solver_id, "kotlin-cp/1.0");
    assert_eq!(provenance.thread_count, Some(4));

    // 结构化 metadata 是一等字段，必须能完整读回（并且已是升序规范形态）。
    // The structured metadata is a first-class field and must read back completely (already in
    // canonical ascending-key form).
    assert_eq!(
        dto.metadata,
        BTreeMap::from([
            ("alpha".to_owned(), "first".to_owned()),
            ("mid".to_owned(), "middle".to_owned()),
            ("zeta".to_owned(), "last".to_owned()),
        ])
    );

    // 再编码必须逐字节相同：这条断言把"两侧序列化等价"钉死，而不只是"能互相解析"。
    // Re-encoding must be byte-identical, pinning "the two sides serialize equivalently" rather than
    // merely "they can parse each other".
    let reencoded = serde_json::to_string(&dto).expect("re-encoding should succeed");
    assert_eq!(
        document, reencoded,
        "Rust's re-encoding must be byte-identical to the shared document"
    );

    // 物化与重建也不得丢失一等字段。 / Materialization and rebuild must not drop the first-class fields.
    let artifact = checkpoint_artifact_from_json(document.as_bytes())
        .expect("the shared envelope must materialize");
    assert_eq!(artifact.checkpoint.cancellation_chain.len(), 2);
    assert_eq!(
        artifact.checkpoint.cancellation_chain[0].reason.as_deref(),
        Some("remote dispatcher stopped the attempt")
    );
    assert_eq!(artifact.checkpoint.provenance.solver_id, "kotlin-cp/1.0");
}

/// 物化再重编码必须**保留**对端 envelope 的身份字段。
///
/// **Materializing then re-encoding must preserve a peer envelope's identity fields.**
///
/// `RemoteCheckpointArtifactDto::new()` 从核心 `checkpoint.metadata` 读取
/// `checkpointId` / `identityNamespace` / `identitySchemaVersion` / `modelName` /
/// `createdAtEpochMs`，缺项时落到兜底值（`checkpointId` → `attemptId`、namespace → `ospf-rust`、
/// modelName → `remote`、时间戳 → 0）。因此 `into_artifact()` **必须**把这些字段写回核心
/// `metadata`——那是跨物化唯一的字符串载体；否则"物化 → 重编码"会静默改写对端身份，对端读回时
/// 审计链已经失真。
///
/// 该簿记是 Rust 侧的内部载体，**不是**线格式的一部分：`new()` 在组装 envelope 时把它们从线格式
/// `metadata` 中剔除，所以对端看到的 `metadata` 仍然逐字节不变。
///
/// `RemoteCheckpointArtifactDto::new()` reads `checkpointId` / `identityNamespace` /
/// `identitySchemaVersion` / `modelName` / `createdAtEpochMs` from the core `checkpoint.metadata` and
/// substitutes defaults for missing ones (`checkpointId` → `attemptId`, namespace → `ospf-rust`,
/// modelName → `remote`, timestamp → 0). `into_artifact()` must therefore write those fields back into
/// the core `metadata` — the only string carrier across materialization; otherwise a
/// materialize-then-reencode silently rewrites a peer's identity and the peer reads back a drifted audit
/// chain.
///
/// That bookkeeping is a Rust-internal carrier and **not** part of the wire: `new()` strips it out of the
/// wire `metadata` while assembling the envelope, so the peer sees an unchanged `metadata`.
#[test]
fn foreign_envelope_identity_survives_a_materialize_reencode_round_trip() {
    let Some(path) = cross_language_envelope_path() else {
        if skip_requested() {
            return;
        }
        panic!(
            "checkpoint-envelope-v3.json not found next to the contract fixture; set {} to skip explicitly",
            SKIP_ENV
        );
    };
    let document = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    let document = document.trim_end().to_owned();

    let original: RemoteCheckpointArtifactDto =
        serde_json::from_str(&document).expect("the shared envelope must decode");
    assert_eq!(original.checkpoint_id, "kotlin-checkpoint");
    assert_eq!(original.model_name, "kotlin-fixture");

    let artifact = checkpoint_artifact_from_json(document.as_bytes())
        .expect("the shared envelope must materialize");
    let reencoded = RemoteCheckpointArtifactDto::new(artifact)
        .expect("the materialized artifact must re-encode");

    // 身份字段逐项保持——不再退化为兜底值。
    // Every identity field survives — none degrades into a fallback.
    assert_eq!(original.checkpoint_id, reencoded.checkpoint_id);
    assert_eq!(original.identity_namespace, reencoded.identity_namespace);
    assert_eq!(
        original.identity_schema_version,
        reencoded.identity_schema_version
    );
    assert_eq!(original.model_name, reencoded.model_name);
    assert_eq!(original.created_at_epoch_ms, reencoded.created_at_epoch_ms);

    // 运行/尝试身份、两个一等字段，以及线格式 `metadata` 都必须逐字节不变（簿记不得泄漏到线格式）。
    // Run/attempt identity, the two first-class fields, and the wire `metadata` must all be unchanged
    // (the bookkeeping must never leak onto the wire).
    assert_eq!(original.run_id, reencoded.run_id);
    assert_eq!(original.attempt_id, reencoded.attempt_id);
    assert_eq!(original.parent_checkpoint_id, reencoded.parent_checkpoint_id);
    assert_eq!(original.cancellation_chain, reencoded.cancellation_chain);
    assert_eq!(original.provenance, reencoded.provenance);
    assert_eq!(
        original.metadata, reencoded.metadata,
        "wire metadata must not gain the identity bookkeeping"
    );
    assert_eq!(
        original.cancellation_chain[0].reason,
        reencoded.cancellation_chain[0].reason
    );

    // 物化路径与纯 DTO 路径的唯一差别是 Rust 会把自己的残留载荷注入 `assumptions`
    // （`state` / `stateDigest` / `iteration` / 原始指纹的载体）。这是 Rust 侧设计使然，**不是**
    // 身份失真；"逐字节等价"由 DTO 层的 `shared_cross_language_envelope_is_accepted_...` 断言。
    //
    // The only difference between the materialization path and the pure DTO path is that Rust injects its
    // own residual payload into `assumptions` (the carrier for `state` / `stateDigest` / `iteration` /
    // the original fingerprints). That is by Rust-side design and **not** identity drift; the
    // byte-for-byte equivalence claim is asserted at the DTO level by
    // `shared_cross_language_envelope_is_accepted_...`.
    let reencoded_document = serde_json::to_string(&reencoded).expect("re-encoding should succeed");
    assert_eq!(
        reencoded.assumptions.len(),
        1,
        "materialization must inject exactly one residual payload"
    );
    assert!(
        reencoded.assumptions[0].starts_with(RUST_CHECKPOINT_RESIDUAL_METADATA_PREFIX),
        "the injected assumption must be the residual payload: {}",
        reencoded.assumptions[0]
    );
    // 把注入的残留载荷与随之变化的摘要还原后，整份文档必须与原始**逐字节相同**。
    // After restoring the injected residual payload and the digest that follows from it, the whole
    // document must be **byte-identical** to the original.
    let mut original_root: serde_json::Value =
        serde_json::from_str(&document).expect("the original document should parse");
    let mut reencoded_root: serde_json::Value =
        serde_json::from_str(&reencoded_document).expect("the re-encoded document should parse");
    for root in [&mut original_root, &mut reencoded_root] {
        let object = root.as_object_mut().expect("envelope should be an object");
        object.insert(
            "assumptions".to_owned(),
            serde_json::Value::Array(Vec::new()),
        );
        object.insert(
            "integritySha256".to_owned(),
            serde_json::Value::String(String::new()),
        );
    }
    assert_eq!(
        original_root, reencoded_root,
        "apart from the injected residual payload, the peer's document must be unchanged"
    );
}

#[test]
fn source_format_vocabulary_matches_the_shared_contract() {
    // `sourceFormat` 是一等字段，却是**来源族**标记而非版本号：当前 envelope 写 `v2` 而
    // `schemaVersion` 是 `3.0`。这一词表此前只存在于代码里，契约未规定，属于跨语言契约的缺口；
    // 本用例把它钉住，并断言实现与契约一致。
    //
    // `sourceFormat` is a first-class field but is a **source-family** tag rather than a version number:
    // the current envelope writes `v2` while `schemaVersion` is `3.0`. That vocabulary previously existed
    // only in code and was unspecified by the contract, a gap in the cross-language contract; this test
    // pins it and asserts the implementation agrees with the contract.
    let sections = require_contract!();
    let vocabulary = &sections["source-format"];
    assert_eq!(vocabulary.len(), 5, "source-format must declare five rows");

    // 产出方必须写 `v2`，且严格读取路径只接受 `v2`。
    // A producer must write `v2`, and the strict read path accepts only `v2`.
    let original = artifact_fixture();
    let dto = RemoteCheckpointArtifactDto::new(original).expect("artifact should encode");
    assert_eq!(dto.source_format, "v2");

    let mut forwarded: RemoteCheckpointArtifactDto =
        serde_json::from_slice(&encoded_envelope(&artifact_fixture()))
            .expect("envelope should decode");
    assert_eq!(forwarded.source_format, "v2");
    forwarded.validate().expect("v2 must validate");

    // 其它取值（含 `legacy-v1`）必须被**严格**路径拒绝——Rust 没有遗留数据需要迁移。
    // Any other value (including `legacy-v1`) must be rejected by the **strict** path: Rust has no legacy
    // data to migrate.
    for rejected in ["legacy-v1", "v3", ""] {
        forwarded.source_format = rejected.to_owned();
        let error = forwarded
            .validate()
            .expect_err("a non-v2 source format must be rejected");
        assert_eq!(
            error.code,
            RemoteSolverErrorCode::UnsupportedProtocolVersion,
            "rejecting {rejected:?} must report an unsupported protocol version"
        );
    }
}

#[test]
fn checkpoint_wire_contract_fixture_is_locatable() {
    let path = contract_path().expect("the checkpoint wire contract must be locatable");
    assert!(
        Path::new(&path).is_file(),
        "resolved contract path must be a file: {}",
        path.display()
    );
}

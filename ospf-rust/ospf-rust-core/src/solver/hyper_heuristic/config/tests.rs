use super::*;
use crate::solver::heuristic::session::{ConstraintHandlingConfig, ConstraintMode};
use crate::solver::hyper_heuristic::persistence::ConfigurationMigration;

fn fixture() -> HyperHeuristicConfig {
    let mut leaf = AlgorithmConfig::new("local", "genetic");
    leaf.constraints = Some(ConstraintHandlingConfig {
        mode: ConstraintMode::Epsilon, initial_epsilon: Some(0.75), horizon: 7,
        exponent: 1.5, objective_probability: 0.35,
    });
    leaf.updates.push(ComponentSpec::new("pipeline").parameter("stages", serde_json::json!([
        {"type": "vnd", "parameters": {"stages": [
            {"type": "elite-local", "parameters": {"amount": 2, "stages": [{"type": "pattern-search"}]}},
            {"type": "mutation-stage", "parameters": {"operator": {"type": "gaussian"}}}
        ]}}
    ])));
    let mut second = AlgorithmConfig::new("random", "shade");
    second.constraints = Some(ConstraintHandlingConfig {
        mode: ConstraintMode::StochasticRanking, ..ConstraintHandlingConfig::default()
    });
    let mut config = HyperHeuristicConfig::new(ComponentSpec::new("fixture").parameter("signedZero", -0.0).parameter("float", 5.0), 2048).algorithm(leaf).algorithm(second);
    config.selection = ComponentSpec::new("frr-mab").parameter("window", 17);
    config.learning.reward = ComponentSpec::new("hypervolume-delta").parameter("mode", "sampled")
        .parameter("reference", serde_json::json!([12.0, 13.0])).parameter("samples", 128);
    config.memory.acceptance = ComponentSpec::new("auto");
    config
}

#[test]
fn formats_roundtrip_nested_components_and_constraints() {
    let config = fixture();
    for format in [ConfigurationFormat::Json, ConfigurationFormat::Yaml, ConfigurationFormat::Toml] {
        let text = ConfigurationFormats::encode_format(&config, format).unwrap();
        let decoded = ConfigurationFormats::decode_format(&text, format).unwrap();
        assert_eq!(decoded.problem.parameters["signedZero"].as_f64().unwrap().to_bits(), (-0.0_f64).to_bits());
        assert_eq!(decoded, config, "{format:?}: {text}");
    }
}

#[test]
fn formats_reject_duplicates_nonfinite_and_oversized_documents() {
    for (format, text) in [
        (ConfigurationFormat::Json, "{\"version\":1,\"version\":1}"),
        (ConfigurationFormat::Yaml, "version: 1\nversion: 1"),
        (ConfigurationFormat::Toml, "version = 1\nversion = 1"),
        (ConfigurationFormat::Yaml, "value: .nan"),
        (ConfigurationFormat::Toml, "value = nan"),
        (ConfigurationFormat::Toml, "value = +inf"),
    ] {
        assert!(ConfigurationFormats::decode_format(text, format).is_err());
    }
    for format in [ConfigurationFormat::Json, ConfigurationFormat::Yaml, ConfigurationFormat::Toml] {
        assert!(ConfigurationFormats::decode_format(&" ".repeat(4 * 1024 * 1024 + 1), format).is_err());
    }
}

#[test]
fn formats_reject_unknown_fields_bad_versions_and_types() {
    for format in [ConfigurationFormat::Json, ConfigurationFormat::Yaml, ConfigurationFormat::Toml] {
        for change in [0, 1, 2] {
            let mut config = fixture();
            match change {
                0 => config.version = 2,
                1 => config.algorithms[0].constraints.as_mut().unwrap().horizon = 0,
                _ => config.budget.slice_steps = 0,
            }
            assert!(ConfigurationFormats::encode_format(&config, format).is_err());
        }
        let text = ConfigurationFormats::encode_format(&fixture(), format).unwrap();
        let unknown = match format {
            ConfigurationFormat::Json => text.replacen('{', "{\"unknown\":1,", 1),
            ConfigurationFormat::Yaml => format!("unknown: 1\n{text}"),
            ConfigurationFormat::Toml => format!("unknown = 1\n{text}"),
        };
        assert!(ConfigurationFormats::decode_format(&unknown, format).is_err());
    }
    assert!(ConfigurationFormats::decode("{\"version\":\"1\"}").is_err());
}

#[test]
fn toml_preserves_datetimes_as_text_and_rejects_business_nulls() {
    let config = fixture();
    let text = ConfigurationFormats::encode_format(&config, ConfigurationFormat::Toml).unwrap()
        .replace("[problem.parameters]", "[problem.parameters]\ndate = 2026-10-22T10:00:00Z");
    assert_eq!(ConfigurationFormats::decode_format(&text, ConfigurationFormat::Toml).unwrap().problem.parameters["date"], "2026-10-22T10:00:00Z");
    let mut config = config;
    config.problem.parameters.insert("businessNull".into(), Value::Null);
    assert!(ConfigurationFormats::encode_format(&config, ConfigurationFormat::Toml).is_err());
    assert_eq!(ConfigurationFormats::decode(&ConfigurationFormats::encode(&config).unwrap()).unwrap(), config);
}

#[test]
fn configuration_migration_is_explicit_and_shared_by_formats() {
    let mut migrations = BTreeMap::<u32, ConfigurationMigration>::new();
    migrations.insert(2, Box::new(|mut value| { value["version"] = 1.into(); Ok(value) }));
    let mut config = fixture();
    config.version = 2;
    for (format, text) in [
        (ConfigurationFormat::Json, serde_json::to_string(&config).unwrap()),
        (ConfigurationFormat::Yaml, serde_yaml_ng::to_string(&config).unwrap()),
        (ConfigurationFormat::Toml, toml::to_string(&config).unwrap()),
    ] {
        assert!(ConfigurationFormats::decode_format(&text, format).is_err());
        assert_eq!(ConfigurationFormats::decode_migrating(&text, format, &migrations).unwrap(), fixture());
    }
}

#[test]
fn checkpoint_constraints_match_kotlin_default_compression() {
    let mut config = fixture();
    config.algorithms[0].constraints = Some(ConstraintHandlingConfig::default());
    let value = checkpoint_configuration(&config).unwrap();
    assert_eq!(value["algorithms"][0]["constraints"], serde_json::json!({}));
    assert_eq!(value["algorithms"][1]["constraints"], serde_json::json!({"mode": "StochasticRanking"}));
    assert_eq!(serde_json::from_value::<HyperHeuristicConfig>(value).unwrap(), config);
}

//! 稳定模型和配置指纹 / Stable model and configuration fingerprints.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use crate::error::{CoreError, Result, SolverError};
use crate::model::intermediate::{LinearTriadModel, QuadraticTetradModel};
use crate::solver::{AuditFingerprint, ModelFingerprint};

/// 生成 SHA-256 审计指纹 / Create a SHA-256 audit fingerprint.
pub fn sha256_fingerprint(domain: &str, bytes: &[u8]) -> AuditFingerprint {
    let mut digest = Sha256::new();
    digest.update(domain.as_bytes());
    digest.update([0]);
    digest.update(bytes);
    let value = digest
        .finalize()
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect::<String>();
    AuditFingerprint {
        schema_version: "1.0".to_owned(),
        algorithm: "sha256".to_owned(),
        value,
    }
}

/// 对有序键值配置生成指纹 / Fingerprint an ordered key-value configuration.
pub fn configuration_fingerprint(configuration: &BTreeMap<String, String>) -> AuditFingerprint {
    let mut bytes = Vec::new();
    for (key, value) in configuration {
        append_string(&mut bytes, key);
        append_string(&mut bytes, value);
    }
    sha256_fingerprint("ospf.solve.configuration", &bytes)
}

/// 对求解器来源生成指纹 / Fingerprint solver provenance.
pub fn solver_provenance_fingerprint(
    provenance: &crate::solver::SolverProvenance,
) -> AuditFingerprint {
    let mut bytes = Vec::new();
    append_string(&mut bytes, &provenance.solver_id);
    append_string(&mut bytes, &provenance.backend_name);
    append_optional_string(&mut bytes, provenance.backend_version.as_deref());
    append_optional_string(&mut bytes, provenance.plugin_version.as_deref());
    append_map(&mut bytes, &provenance.effective_configuration);
    append_optional_usize(&mut bytes, provenance.thread_count);
    append_optional_u64(&mut bytes, provenance.random_seed);
    append_optional_bool(&mut bytes, provenance.deterministic);
    append_map(&mut bytes, &provenance.environment_summary);
    sha256_fingerprint("ospf.solve.solver", &bytes)
}

/// 生成线性模型指纹 / Create a fingerprint for a linear model.
pub fn linear_model_fingerprint(model: &LinearTriadModel) -> Result<ModelFingerprint> {
    validate_linear_model(model)?;
    let mut bytes = Vec::new();
    append_string(&mut bytes, "linear");
    append_string(&mut bytes, &model.basic.name);
    append_variables(
        &mut bytes,
        &model.basic.variables,
        &model.basic.lb,
        &model.basic.ub,
    );
    append_rows(
        &mut bytes,
        &model.basic.A.rows,
        &model.basic.b,
        &model.basic.constraint_names,
    )?;
    append_f64_slice(&mut bytes, &model.c);
    append_string(&mut bytes, &format!("{:?}", model.objective_category));
    Ok(sha256_fingerprint("ospf.solve.model", &bytes))
}

/// 生成二次模型指纹 / Create a fingerprint for a quadratic model.
pub fn quadratic_model_fingerprint(model: &QuadraticTetradModel) -> Result<ModelFingerprint> {
    validate_quadratic_model(model)?;
    let mut bytes = Vec::new();
    append_string(&mut bytes, "quadratic");
    append_string(&mut bytes, &model.basic.linear.name);
    append_variables(
        &mut bytes,
        &model.basic.linear.variables,
        &model.basic.linear.lb,
        &model.basic.linear.ub,
    );
    append_rows(
        &mut bytes,
        &model.basic.linear.A.rows,
        &model.basic.linear.b,
        &model.basic.linear.constraint_names,
    )?;
    append_f64_slice(&mut bytes, &model.c);
    append_sparse_rows(&mut bytes, &model.Q.rows)?;
    append_string(&mut bytes, &format!("{:?}", model.objective_category));
    for constraint in &model.quadratic_constraints {
        append_string(&mut bytes, &format!("{:?}", constraint.relation));
        append_f64(&mut bytes, constraint.rhs);
        let mut monomials = constraint
            .polynomial
            .monomials()
            .iter()
            .map(|monomial| {
                (
                    monomial.var_index1(),
                    monomial.var_index2(),
                    *monomial.coefficient(),
                )
            })
            .collect::<Vec<_>>();
        monomials.sort_by_key(|(first, second, _)| (*first, *second));
        append_canonical_monomials(&mut bytes, &monomials)?;
    }
    Ok(sha256_fingerprint("ospf.solve.model", &bytes))
}

fn validate_linear_model(model: &LinearTriadModel) -> Result<()> {
    crate::solver::audit::validate_linear_model_for_backend(model)
}

fn validate_quadratic_model(model: &QuadraticTetradModel) -> Result<()> {
    crate::solver::audit::validate_quadratic_model_for_backend(model)
}

fn ensure_finite(value: f64, context: &str) -> Result<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(CoreError::Solver(SolverError::NonFinite(format!(
            "{} contains {}",
            context, value
        ))))
    }
}

fn append_variables(
    bytes: &mut Vec<u8>,
    variables: &[crate::token::Token<f64>],
    lower: &[f64],
    upper: &[f64],
) {
    bytes.extend_from_slice(&(variables.len() as u64).to_le_bytes());
    for (index, variable) in variables.iter().enumerate() {
        append_string(bytes, variable.name());
        append_string(bytes, &format!("{:?}", variable.var_type()));
        append_optional_f64(bytes, lower.get(index).copied());
        append_optional_f64(bytes, upper.get(index).copied());
    }
}

fn append_rows(
    bytes: &mut Vec<u8>,
    rows: &[crate::model::intermediate::SparseVector<f64>],
    rhs: &[f64],
    names: &[String],
) -> Result<()> {
    bytes.extend_from_slice(&(rows.len() as u64).to_le_bytes());
    for (index, row) in rows.iter().enumerate() {
        append_string(bytes, names.get(index).map(String::as_str).unwrap_or(""));
        append_optional_f64(bytes, rhs.get(index).copied());
        let entries = canonical_sparse_entries(&row.entries)?;
        bytes.extend_from_slice(&(entries.len() as u64).to_le_bytes());
        for (column, value) in entries {
            bytes.extend_from_slice(&(column as u64).to_le_bytes());
            append_f64(bytes, value);
        }
    }
    Ok(())
}

fn append_sparse_rows(
    bytes: &mut Vec<u8>,
    rows: &[crate::model::intermediate::SparseVector<f64>],
) -> Result<()> {
    bytes.extend_from_slice(&(rows.len() as u64).to_le_bytes());
    for row in rows {
        let entries = canonical_sparse_entries(&row.entries)?;
        bytes.extend_from_slice(&(entries.len() as u64).to_le_bytes());
        for (column, value) in entries {
            bytes.extend_from_slice(&(column as u64).to_le_bytes());
            append_f64(bytes, value);
        }
    }
    Ok(())
}

fn canonical_sparse_entries(entries: &[(usize, f64)]) -> Result<Vec<(usize, f64)>> {
    let mut merged = BTreeMap::<usize, f64>::new();
    for (column, value) in entries {
        let entry = merged.entry(*column).or_insert(0.0);
        *entry += *value;
        ensure_finite(*entry, "merged sparse coefficient")?;
    }
    Ok(merged
        .into_iter()
        .filter(|(_, value)| *value != 0.0)
        .collect())
}

fn append_canonical_monomials(
    bytes: &mut Vec<u8>,
    monomials: &[(usize, Option<usize>, f64)],
) -> Result<()> {
    let mut merged = BTreeMap::<(usize, Option<usize>), f64>::new();
    for (first, second, coefficient) in monomials {
        let entry = merged.entry((*first, *second)).or_insert(0.0);
        *entry += *coefficient;
        ensure_finite(*entry, "merged quadratic coefficient")?;
    }
    let merged = merged
        .into_iter()
        .filter(|(_, coefficient)| *coefficient != 0.0)
        .collect::<Vec<_>>();
    bytes.extend_from_slice(&(merged.len() as u64).to_le_bytes());
    for ((first, second), coefficient) in merged {
        bytes.extend_from_slice(&(first as u64).to_le_bytes());
        append_optional_usize(bytes, second);
        append_f64(bytes, coefficient);
    }
    Ok(())
}

fn append_f64_slice(bytes: &mut Vec<u8>, values: &[f64]) {
    bytes.extend_from_slice(&(values.len() as u64).to_le_bytes());
    for value in values {
        append_f64(bytes, *value);
    }
}

fn append_f64(bytes: &mut Vec<u8>, value: f64) {
    if value.is_infinite() {
        bytes.extend_from_slice(&[1, value.is_sign_negative() as u8]);
    } else {
        let normalized = if value == 0.0 { 0.0 } else { value };
        bytes.push(0);
        bytes.extend_from_slice(&normalized.to_bits().to_le_bytes());
    }
}

fn append_optional_f64(bytes: &mut Vec<u8>, value: Option<f64>) {
    match value {
        Some(value) => {
            bytes.push(1);
            append_f64(bytes, value);
        }
        None => bytes.push(0),
    }
}

fn append_string(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u64).to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

fn append_optional_string(bytes: &mut Vec<u8>, value: Option<&str>) {
    match value {
        Some(value) => {
            bytes.push(1);
            append_string(bytes, value);
        }
        None => bytes.push(0),
    }
}

fn append_map(bytes: &mut Vec<u8>, values: &BTreeMap<String, String>) {
    bytes.extend_from_slice(&(values.len() as u64).to_le_bytes());
    for (key, value) in values {
        append_string(bytes, key);
        append_string(bytes, value);
    }
}

fn append_optional_usize(bytes: &mut Vec<u8>, value: Option<usize>) {
    match value {
        Some(value) => {
            bytes.push(1);
            bytes.extend_from_slice(&(value as u64).to_le_bytes());
        }
        None => bytes.push(0),
    }
}

fn append_optional_u64(bytes: &mut Vec<u8>, value: Option<u64>) {
    match value {
        Some(value) => {
            bytes.push(1);
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        None => bytes.push(0),
    }
}

fn append_optional_bool(bytes: &mut Vec<u8>, value: Option<bool>) {
    match value {
        Some(value) => bytes.extend_from_slice(&[1, value as u8]),
        None => bytes.push(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::Token;
    use crate::variable::{ContinuousVariableItem, VariableType};

    #[test]
    fn configuration_fingerprint_is_order_independent() {
        let mut first = BTreeMap::new();
        first.insert("b".to_owned(), "2".to_owned());
        first.insert("a".to_owned(), "1".to_owned());
        let mut second = BTreeMap::new();
        second.insert("a".to_owned(), "1".to_owned());
        second.insert("b".to_owned(), "2".to_owned());
        assert_eq!(
            configuration_fingerprint(&first),
            configuration_fingerprint(&second)
        );
    }

    #[test]
    fn negative_zero_has_the_same_fingerprint_as_zero() {
        let positive = sha256_fingerprint("test", &0.0f64.to_bits().to_le_bytes());
        let negative = sha256_fingerprint("test", &(-0.0f64).to_bits().to_le_bytes());
        assert_ne!(positive, negative);
        let mut bytes = Vec::new();
        append_f64(&mut bytes, -0.0);
        let normalized = sha256_fingerprint("test", &bytes);
        let mut zero_bytes = Vec::new();
        append_f64(&mut zero_bytes, 0.0);
        assert_eq!(normalized, sha256_fingerprint("test", &zero_bytes));
    }

    #[test]
    fn sparse_reordering_duplicates_and_zero_terms_have_the_same_fingerprint() {
        let mut first = LinearTriadModel::new("canonical");
        for index in 0..3 {
            first.basic.add_variable_with_bounds(
                Token::from_generic(ContinuousVariableItem::auto(&format!("x{}", index)), index),
                0.0,
                f64::INFINITY,
                VariableType::Continuous,
            );
        }
        first.c = vec![0.0; 3];
        let mut row = crate::model::intermediate::SparseVector::new();
        row.add(0, 1.0);
        row.add(2, 0.0);
        row.add(0, 2.0);
        first.basic.A.add_row(row);
        first.basic.b.push(3.0);
        first.basic.constraint_names.push("row".to_owned());
        let mut second = first.clone();
        second.basic.A.rows[0].entries = vec![(0, 3.0), (2, 0.0), (0, 0.0)];
        assert_eq!(
            linear_model_fingerprint(&first).unwrap(),
            linear_model_fingerprint(&second).unwrap()
        );
    }

    #[test]
    fn model_fingerprint_does_not_depend_on_process_generated_variable_ids() {
        let first = LinearTriadModel::new("stable");
        let second = first.clone();
        assert_eq!(
            linear_model_fingerprint(&first).unwrap(),
            linear_model_fingerprint(&second).unwrap()
        );
    }

    #[test]
    fn non_finite_model_values_are_rejected() {
        let mut model = LinearTriadModel::new("non_finite");
        model.basic.add_variable_with_bounds(
            Token::from_generic(ContinuousVariableItem::auto("x"), 0),
            0.0,
            f64::INFINITY,
            VariableType::Continuous,
        );
        model.c.push(f64::NAN);
        let error = linear_model_fingerprint(&model).expect_err("NaN must be rejected");
        assert!(error.to_string().contains("Non-finite"));
    }
}

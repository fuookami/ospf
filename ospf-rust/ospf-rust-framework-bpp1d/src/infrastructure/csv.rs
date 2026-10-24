use std::collections::{BTreeMap, BTreeSet};
use num_traits::One;
use ospf_rust_core::error::{ModelError, Result};
use ospf_rust_core::model::LinearModelValue;
use ospf_rust_framework_bpp::infrastructure::PackingValueAdapter;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::UnitTrait;
use crate::domain::{BinType, Item, PackingContext, PackingObjective, PackingProblem};

fn invalid(message: &str) -> ospf_rust_core::error::CoreError {
    ModelError::InvalidConstraint(format!("invalid BPP1D CSV: {message}")).into()
}

#[derive(PartialEq)]
enum FieldState { Start, Unquoted, Quoted, Closed }

fn rows(text: &str, schema: &[&str]) -> Result<Vec<BTreeMap<String, String>>> {
    let mut records = Vec::new();
    let mut record = Vec::new();
    let mut field = String::new();
    let mut state = FieldState::Start;
    let mut chars = text.strip_prefix('\u{feff}').unwrap_or(text).chars().peekable();
    while let Some(ch) = chars.next() {
        if state == FieldState::Quoted {
            if ch == '"' {
                if chars.peek() == Some(&'"') { chars.next(); field.push('"'); } else { state = FieldState::Closed; }
            } else { field.push(ch); }
            continue;
        }
        match ch {
            '"' if state == FieldState::Start => state = FieldState::Quoted,
            '"' => return Err(invalid("malformed quote")),
            ',' | '\r' | '\n' => {
                record.push(field.trim().to_string());
                field.clear();
                state = FieldState::Start;
                if ch != ',' {
                    if ch == '\r' && chars.peek() == Some(&'\n') { chars.next(); }
                    if record.len() != 1 || !record[0].is_empty() { records.push(std::mem::take(&mut record)); }
                    record.clear();
                }
            }
            ch if state == FieldState::Closed && !ch.is_whitespace() => return Err(invalid("characters after closing quote")),
            ch => {
                field.push(ch);
                if state == FieldState::Start { state = FieldState::Unquoted; }
            }
        }
    }
    if state == FieldState::Quoted { return Err(invalid("unclosed quote")); }
    record.push(field.trim().to_string());
    if record.len() != 1 || !record[0].is_empty() { records.push(record); }
    let header = records.first().ok_or_else(|| invalid("missing header"))?;
    let expected = schema.iter().copied().collect::<BTreeSet<_>>();
    if header.len() != schema.len() || header.iter().map(String::as_str).collect::<BTreeSet<_>>() != expected {
        return Err(invalid("closed schema mismatch or duplicate header"));
    }
    records.iter().skip(1).map(|row| {
        if row.len() != header.len() { return Err(invalid("row width")); }
        Ok(header.iter().cloned().zip(row.iter().cloned()).collect())
    }).collect()
}

fn count(text: &str) -> Result<u64> {
    if text.is_empty() || !text.bytes().all(|ch| ch.is_ascii_digit()) { return Err(invalid("invalid unsigned count")); }
    text.parse().map_err(|_| invalid("count overflow"))
}

/// CSV 声明口径及目标。 / Declared CSV units and objective.
pub struct PackingCsvOptions<V, L: UnitTrait, W: UnitTrait> {
    /// 长度单位。 / Length unit.
    pub length_unit: L,
    /// 重量单位。 / Weight unit.
    pub weight_unit: W,
    /// 相邻件间距。 / Inter-piece spacing.
    pub spacing: Quantity<V, L>,
    /// 目标口径。 / Objective metric.
    pub objective: PackingObjective,
}

/// 严格读取闭合表头，调用方解析 V，不经 f64；空库存表示无限。
/// Read strict closed schemas with a caller-provided V parser, never via f64; empty stock means unlimited.
pub fn load_packing_csv<V, L, W, F>(
    items_text: &str,
    bins_text: &str,
    options: PackingCsvOptions<V, L, W>,
    mut parse: F,
    length_adapter: &dyn PackingValueAdapter<V, L>,
    weight_adapter: &dyn PackingValueAdapter<V, W>,
) -> Result<PackingContext<V, L, W>>
where
    V: LinearModelValue + One,
    L: UnitTrait + Clone,
    W: UnitTrait + Clone,
    F: FnMut(&str) -> Result<V>,
{
    let items = rows(items_text, &["id", "length", "weight", "amount"])?;
    let bins = rows(bins_text, &["id", "length", "max_weight", "cost", "inventory"])?;
    let items = items.into_iter().map(|row| Ok(Item {
        id: row["id"].clone(), length: Quantity::new(parse(&row["length"])?, options.length_unit.clone()),
        weight: Quantity::new(parse(&row["weight"])?, options.weight_unit.clone()), amount: count(&row["amount"])?, materials: BTreeMap::new(),
    })).collect::<Result<Vec<_>>>()?;
    let bin_types = bins.into_iter().map(|row| Ok(BinType {
        id: row["id"].clone(), length: Quantity::new(parse(&row["length"])?, options.length_unit.clone()),
        max_weight: Quantity::new(parse(&row["max_weight"])?, options.weight_unit.clone()), cost: parse(&row["cost"])?,
        inventory: if row["inventory"].is_empty() { None } else { Some(count(&row["inventory"])?) },
    })).collect::<Result<Vec<_>>>()?;
    PackingContext::new(PackingProblem { items, bin_types, spacing: options.spacing, length_unit: options.length_unit,
        weight_unit: options.weight_unit, objective: options.objective }, length_adapter, weight_adapter)
}

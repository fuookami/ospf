//! 声明定价行的主问题组件。 / Master component for declared pricing rows.

use std::collections::BTreeMap;
use std::sync::Arc;
use ospf_rust_quantities::unit::UnitConversionValue;
use ospf_rust_core::model::{ConstraintGroup, ConstraintRelation, MetaModel};
use ospf_rust_core::solver::value::{SolveValue, SolveValueConversionPolicy};
use ospf_rust_core::variable::{Continuous, VariableRange};
use crate::domain::vrp::{Route, RoutePricingRows, VrpShadowPriceMap, VrptwInstance};
use crate::error::{NetworkSchedulingError, Result};
use super::{RouteVariableIndex, constraint_dual};

/// 声明目录及每模型列系数缓存。 / Declaration catalog and per-model column coefficient cache.
#[derive(Debug, Clone)]
pub struct RoutePricingRowCompilation<V: SolveValue + UnitConversionValue> {
    catalog: Arc<RoutePricingRows<V>>,
    bounds: BTreeMap<String, (Option<f64>, Option<f64>)>,
    coefficients: BTreeMap<String, BTreeMap<String, f64>>,
    artificials: BTreeMap<(String, &'static str), usize>,
}

impl<V: SolveValue + UnitConversionValue> RoutePricingRowCompilation<V> {
    pub(super) fn empty() -> Self {
        Self { catalog: Arc::new(RoutePricingRows::default()), bounds: BTreeMap::new(), coefficients: BTreeMap::new(), artificials: BTreeMap::new() }
    }

    /// 创建未注册组件；转换失败不会修改模型。 / Create an unregistered component; conversion failure does not modify a model.
    pub fn new(catalog: Arc<RoutePricingRows<V>>) -> Result<Self> {
        let bounds = catalog.rows().iter().map(|row| {
            Ok((row.id.clone(), (
                row.lower.as_ref().map(|value| solver_value(&value.value)).transpose()?,
                row.upper.as_ref().map(|value| solver_value(&value.value)).transpose()?,
            )))
        }).collect::<Result<_>>()?;
        Ok(Self { catalog, bounds, coefficients: BTreeMap::new(), artificials: BTreeMap::new() })
    }

    /// 返回主问题和定价共用目录。 / Return the catalog shared by master and pricing.
    pub fn catalog(&self) -> &Arc<RoutePricingRows<V>> { &self.catalog }

    pub(super) fn artificial_indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.artificials.values().copied()
    }

    pub(super) fn register_variables(&mut self, model: &mut MetaModel<f64>) -> Result<()> {
        for (id, (lower, upper)) in &self.bounds {
            for (side, present) in [("lower", lower.is_some()), ("upper", upper.is_some())] {
                if !present { continue; }
                let index = model.register_auto_variable_with_range::<Continuous>(
                    &format!("route_pricing_artificial:{side}:{id}"), VariableRange::with_lower(0.0),
                ).map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
                self.artificials.insert((id.clone(), side), index);
            }
        }
        Ok(())
    }

    pub(super) fn prepare(&mut self, routes: &[Route<V>], instance: &VrptwInstance<V>) -> Result<()> {
        let prepared = routes.iter().map(|route| {
            let values = self.catalog.route_coefficients(route, instance)?.into_iter()
                .map(|(id, value)| Ok((id, solver_value(&value)?)))
                .collect::<Result<_>>()?;
            Ok((route.signature(), values))
        }).collect::<Result<Vec<_>>>()?;
        self.coefficients.extend(prepared);
        Ok(())
    }

    pub(super) fn remove(&mut self, routes: &[Route<V>]) {
        for route in routes { self.coefficients.remove(&route.signature()); }
    }

    pub(super) fn validate_coefficients(&self, routes: &[Route<V>], instance: &VrptwInstance<V>) -> Result<()> {
        for route in routes {
            let current = self.catalog.route_coefficients(route, instance)?.into_iter()
                .map(|(id, value)| Ok((id, solver_value(&value)?))).collect::<Result<BTreeMap<_, _>>>()?;
            if self.coefficients.get(&route.signature()) != Some(&current) {
                return Err(NetworkSchedulingError::contract("定价提供器已偏离已注册列系数 / pricing provider no longer matches registered column coefficients"));
            }
        }
        Ok(())
    }

    pub(super) fn register_rows(
        &self,
        model: &mut MetaModel<f64>,
        group: &Arc<ConstraintGroup>,
        indices: &BTreeMap<String, RouteVariableIndex>,
    ) -> Result<()> {
        for row in self.catalog.rows() {
            let terms = indices.iter().map(|(signature, index)| {
                let value = self.coefficients.get(signature).and_then(|values| values.get(&row.id))
                    .copied().ok_or_else(|| NetworkSchedulingError::contract(
                        "缺少受管扩展列系数 / missing managed extension-column coefficient"))?;
                Ok((index.model_index, value))
            }).collect::<Result<Vec<_>>>()?;
            let (lower, upper) = self.bounds[&row.id];
            for (side, relation, bound) in [
                ("lower", ConstraintRelation::GreaterEqual, lower),
                ("upper", ConstraintRelation::LessEqual, upper),
            ] {
                if let Some(bound) = bound {
                    let mut terms = terms.clone();
                    let index = self.artificials.get(&(row.id.clone(), side)).copied().ok_or_else(||
                        NetworkSchedulingError::contract("缺少扩展行人工变量 / missing extension-row artificial variable"))?;
                    terms.push((index, if side == "lower" { 1.0 } else { -1.0 }));
                    model.add_linear_constraint_with_metadata(
                        &terms, relation, bound, &row_name(&row.id, side), Some(group.clone()),
                        false, 0, Some("route_pricing_row".to_owned()),
                    ).map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
                }
            }
        }
        Ok(())
    }

    pub(super) fn extract_duals(
        &self,
        model: &MetaModel<f64>,
        duals: &[f64],
        prices: &mut VrpShadowPriceMap,
    ) -> Result<()> {
        for row in self.catalog.rows() {
            let (lower, upper) = self.bounds[&row.id];
            let mut price = 0.0;
            for (side, present) in [("lower", lower.is_some()), ("upper", upper.is_some())] {
                if present { price += constraint_dual(model, duals, &row_name(&row.id, side))?; }
            }
            if !price.is_finite() {
                return Err(NetworkSchedulingError::contract("扩展对偶和非有限 / non-finite extension dual sum"));
            }
            prices.set_extension_dual(row.id.clone(), price);
        }
        Ok(())
    }
}

fn row_name(id: &str, side: &str) -> String { format!("route_pricing_row:{side}:{id}") }

fn solver_value<V: SolveValue>(value: &V) -> Result<f64> {
    let value = value.to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
        .map_err(|error| NetworkSchedulingError::Conversion { message: error.to_string() })?;
    if !value.is_finite() {
        return Err(NetworkSchedulingError::validation("扩展行 solver 转换结果非有限 / non-finite extension-row solver conversion"));
    }
    Ok(value)
}

// ============================================================================
// Package - 包装 / Package
// ============================================================================

/// 包装程序 / Packing program
#[derive(Debug, Clone)]
pub struct PackingProgram<V, U: UnitTrait> {
    /// 形状 / Shape
    pub shape: PackageShape<V, U>,
    /// 材料数量映射 / Material amounts map
    pub materials: Vec<(MaterialKey, u64)>,
    /// 材料贡献值 / Material contribution values
    pub material_values: Vec<(MaterialKey, PackingProgramMaterialValue<V, U>)>,
}

/// 包装 / Package
#[derive(Debug, Clone)]
pub struct Package<V, U: UnitTrait> {
    /// 编码 / Code
    pub code: Option<String>,
    /// 形状 / Shape
    pub shape: PackageShape<V, U>,
    /// 子包装 / Sub-packages
    pub packages: Option<Vec<Package<V, U>>>,
    /// 材料数量 / Material amounts
    pub materials: Vec<(MaterialKey, u64)>,
    /// 数量 / Amount
    pub amount: u64,
}

/// 包装程序材料值 / Packing program material value
#[derive(Debug, Clone)]
pub struct PackingProgramMaterialValue<V, U: UnitTrait> {
    /// 数量 / Amount
    pub amount: Option<u64>,
    /// 重量 / Weight
    pub weight: Option<Quantity<V, U>>,
}

impl<V, U: UnitTrait> PackingProgramMaterialValue<V, U> {
    /// 创建材料值 / Create material value
    pub fn new(amount: Option<u64>, weight: Option<Quantity<V, U>>) -> Option<Self> {
        if amount.is_none() && weight.is_none() {
            return None;
        }
        Some(Self { amount, weight })
    }

    /// 创建数量材料值 / Create amount material value
    pub fn amount(amount: u64) -> Self {
        Self {
            amount: Some(amount),
            weight: None,
        }
    }

    /// 创建重量材料值 / Create weight material value
    pub fn weight(weight: Quantity<V, U>) -> Self {
        Self {
            amount: None,
            weight: Some(weight),
        }
    }
}

impl<V, U> PackingProgram<V, U>
where
    V: Clone + Field + num_traits::Float + num_traits::FloatConst + ToPrimitive,
    U: CTUnit + Default + Clone,
{
    /// 物料数量 / Material amounts
    pub fn material_amounts(&self) -> Vec<(MaterialKey, u64)> {
        let mut material_amounts = HashMap::<MaterialKey, u64>::new();
        if self.material_values.is_empty() {
            for (material_key, amount) in &self.materials {
                let entry = material_amounts.entry(material_key.clone()).or_insert(0);
                *entry = entry.saturating_add(*amount);
            }
        } else {
            for (material_key, material_value) in &self.material_values {
                if let Some(amount) = material_value.amount {
                    let entry = material_amounts.entry(material_key.clone()).or_insert(0);
                    *entry = entry.saturating_add(amount);
                }
            }
        }
        let mut material_amounts = material_amounts.into_iter().collect::<Vec<_>>();
        material_amounts.sort_by(|(lhs, _), (rhs, _)| lhs.cmp(rhs));
        material_amounts
    }

    /// 物料重量 / Material weights
    pub fn material_weights(&self, material_catalog: &[Material<V, U>]) -> Vec<(MaterialKey, Quantity<V, U>)> {
        let catalog = material_catalog
            .iter()
            .map(|material| (material.key(), material.weight.clone()))
            .collect::<HashMap<_, _>>();
        let mut material_weights = HashMap::<MaterialKey, V>::new();
        if self.material_values.is_empty() {
            for (material_key, amount) in &self.materials {
                let Some(unit_weight) = catalog.get(material_key) else {
                    continue;
                };
                let entry = material_weights.entry(material_key.clone()).or_insert_with(V::zero);
                *entry = *entry + unit_weight.value.clone() * V::from(*amount).unwrap_or_else(V::zero);
            }
        } else {
            for (material_key, material_value) in &self.material_values {
                if let Some(weight) = &material_value.weight {
                    let entry = material_weights.entry(material_key.clone()).or_insert_with(V::zero);
                    *entry = *entry + weight.value.clone();
                } else if let Some(amount) = material_value.amount {
                    let Some(unit_weight) = catalog.get(material_key) else {
                        continue;
                    };
                    let entry = material_weights.entry(material_key.clone()).or_insert_with(V::zero);
                    *entry = *entry + unit_weight.value.clone() * V::from(amount).unwrap_or_else(V::zero);
                }
            }
        }
        let mut material_weights = material_weights
            .into_iter()
            .map(|(material_key, weight)| (material_key, Quantity::new_ct(weight)))
            .collect::<Vec<_>>();
        material_weights.sort_by(|(lhs, _), (rhs, _)| lhs.cmp(rhs));
        material_weights
    }

    /// 物料单项数量 / Material amount for a single key
    pub fn material_amount(&self, material: &MaterialKey) -> u64 {
        self.material_amounts()
            .into_iter()
            .find(|(material_key, _)| material_key == material)
            .map(|(_, amount)| amount)
            .unwrap_or(0)
    }
}


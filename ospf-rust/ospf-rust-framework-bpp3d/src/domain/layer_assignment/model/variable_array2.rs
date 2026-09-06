// ============================================================================
// VariableArray2 - 二维变量集合 / 2D variable array
// ============================================================================

/// 二维索引变量集合 / Two-dimensional indexed variable array
#[derive(Debug, Clone)]
pub struct VariableArray2<K1, K2, VT>
where
    K1: Debug + Clone + Eq + Hash,
    K2: Debug + Clone + Eq + Hash,
    VT: Debug + Clone,
{
    /// 变量名前缀 / Variable name prefix
    pub prefix: String,
    /// (K1, K2) 到模型索引的映射 / (K1, K2) to model index mapping
    pub indices: HashMap<(K1, K2), usize>,
    _phantom: std::marker::PhantomData<VT>,
}

impl<K1, K2, VT> VariableArray2<K1, K2, VT>
where
    K1: Debug + Clone + Eq + Hash,
    K2: Debug + Clone + Eq + Hash,
    VT: Debug + Clone,
{
    /// 创建二维变量集合 / Create a 2D variable array
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            indices: HashMap::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// 获取变量数量 / Get number of variables
    pub fn len(&self) -> usize {
        self.indices.len()
    }

    /// 是否为空 / Whether empty
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// 获取键对应的模型索引 / Get model index for key pair
    pub fn index(&self, k1: &K1, k2: &K2) -> Option<usize> {
        self.indices.get(&(k1.clone(), k2.clone())).copied()
    }

    fn variable_name(&self, k1: &K1, k2: &K2) -> String {
        format!("{}_{}_{}", self.prefix, key_to_string(k1), key_to_string(k2))
    }
}

impl<K1, K2> VariableArray2<K1, K2, BinaryVariableItem>
where
    K1: Debug + Clone + Eq + Hash,
    K2: Debug + Clone + Eq + Hash,
{
    /// 注册二值变量 / Register binary variables
    pub fn register_binary(
        &mut self,
        keys1: &[K1],
        keys2: &[K2],
        model: &mut MetaModel<f64>,
    ) -> Result<(), String> {
        for k1 in keys1 {
            for k2 in keys2 {
                let name = self.variable_name(k1, k2);
                let var = BinaryVariableItem::auto(&name);
                let idx = model.register_variable(var)
                    .map_err(|e| format!("Failed to register variable {}: {:?}", name, e))?;
                self.indices.insert((k1.clone(), k2.clone()), idx);
            }
        }
        Ok(())
    }
}

impl<K1, K2> VariableArray2<K1, K2, ContinuousVariableItem>
where
    K1: Debug + Clone + Eq + Hash,
    K2: Debug + Clone + Eq + Hash,
{
    /// 注册连续变量 / Register continuous variables
    pub fn register_continuous(
        &mut self,
        keys1: &[K1],
        keys2: &[K2],
        model: &mut MetaModel<f64>,
    ) -> Result<(), String> {
        for k1 in keys1 {
            for k2 in keys2 {
                let name = self.variable_name(k1, k2);
                let var = ContinuousVariableItem::auto(&name);
                let idx = model.register_variable(var)
                    .map_err(|e| format!("Failed to register variable {}: {:?}", name, e))?;
                self.indices.insert((k1.clone(), k2.clone()), idx);
            }
        }
        Ok(())
    }
}


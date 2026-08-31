// ============================================================================
// VariableArray1 - 一维变量集合 / 1D variable array
// ============================================================================

/// 一维索引变量集合 / One-dimensional indexed variable array
///
/// 将领域键映射到模型变量索引，支持注册和结果提取。
/// Maps domain keys to model variable indices, supporting registration and
/// solution extraction.
#[derive(Debug, Clone)]
pub struct VariableArray1<K, VT>
where
    K: Debug + Clone + Eq + Hash,
    VT: Debug + Clone,
{
    /// 变量名前缀 / Variable name prefix
    pub prefix: String,
    /// 键到模型索引的映射 / Key to model index mapping
    pub indices: HashMap<K, usize>,
    /// 键到变量类型的映射 / Key to variable type mapping
    _phantom: std::marker::PhantomData<VT>,
}

impl<K, VT> VariableArray1<K, VT>
where
    K: Debug + Clone + Eq + Hash,
    VT: Debug + Clone,
{
    /// 创建一维变量集合 / Create a 1D variable array
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

    /// 获取键对应的模型索引 / Get model index for key
    pub fn index(&self, key: &K) -> Option<usize> {
        self.indices.get(key).copied()
    }

    /// 生成变量名 / Generate variable name
    fn variable_name(&self, key: &K) -> String {
        format!("{}_{}", self.prefix, key_to_string(key))
    }
}

/// 辅助：将键转为字符串 / Helper: convert key to string
fn key_to_string<K: Debug>(key: &K) -> String {
    format!("{:?}", key)
}

impl<K> VariableArray1<K, BinaryVariableItem>
where
    K: Debug + Clone + Eq + Hash,
{
    /// 注册二值变量 / Register binary variables
    pub fn register_binary(&mut self, keys: &[K], model: &mut MetaModel<f64>) -> Result<(), String> {
        for key in keys {
            let name = self.variable_name(key);
            let var = BinaryVariableItem::auto(&name);
            let idx = model.register_variable(var)
                .map_err(|e| format!("Failed to register variable {}: {:?}", name, e))?;
            self.indices.insert(key.clone(), idx);
        }
        Ok(())
    }
}

impl<K> VariableArray1<K, ContinuousVariableItem>
where
    K: Debug + Clone + Eq + Hash,
{
    /// 注册连续变量 / Register continuous variables
    pub fn register_continuous(&mut self, keys: &[K], model: &mut MetaModel<f64>) -> Result<(), String> {
        for key in keys {
            let name = self.variable_name(key);
            let var = ContinuousVariableItem::auto(&name);
            let idx = model.register_variable(var)
                .map_err(|e| format!("Failed to register variable {}: {:?}", name, e))?;
            self.indices.insert(key.clone(), idx);
        }
        Ok(())
    }
}

impl<K> VariableArray1<K, UContinuousVariableItem>
where
    K: Debug + Clone + Eq + Hash,
{
    /// 注册无符号连续变量 / Register unsigned continuous variables
    pub fn register_unsigned_continuous(
        &mut self,
        keys: &[K],
        model: &mut MetaModel<f64>,
    ) -> Result<(), String> {
        for key in keys {
            let name = self.variable_name(key);
            let var = UContinuousVariableItem::auto(&name);
            let idx = model.register_variable(var)
                .map_err(|e| format!("Failed to register variable {}: {:?}", name, e))?;
            self.indices.insert(key.clone(), idx);
        }
        Ok(())
    }

    /// 注册带上界的无符号连续变量 / Register bounded unsigned continuous variables
    pub fn register_unsigned_continuous_with_upper_bounds<F>(
        &mut self,
        keys: &[K],
        model: &mut MetaModel<f64>,
        upper_bound: F,
    ) -> Result<(), String>
    where
        F: Fn(&K) -> Option<f64>,
    {
        for key in keys {
            let name = self.variable_name(key);
            let range = VariableRange::new(Some(0.0), upper_bound(key));
            let var = UContinuousVariableItem::auto_with_range(&name, range);
            let idx = model.register_variable(var)
                .map_err(|e| format!("Failed to register variable {}: {:?}", name, e))?;
            self.indices.insert(key.clone(), idx);
        }
        Ok(())
    }

    /// 注册单个带上界的无符号连续变量 / Register one bounded unsigned continuous variable
    pub fn register_unsigned_continuous_with_upper_bound(
        &mut self,
        key: K,
        model: &mut MetaModel<f64>,
        upper_bound: Option<f64>,
    ) -> Result<usize, String> {
        let name = self.variable_name(&key);
        let range = VariableRange::new(Some(0.0), upper_bound);
        let var = UContinuousVariableItem::auto_with_range(&name, range);
        let idx = model.register_variable(var)
            .map_err(|e| format!("Failed to register variable {}: {:?}", name, e))?;
        self.indices.insert(key, idx);
        Ok(idx)
    }
}


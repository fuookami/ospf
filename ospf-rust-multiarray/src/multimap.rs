use std::collections::HashMap;
use std::hash::Hash;

/// 二维多键映射 / Two-key nested map.
#[derive(Clone, Debug, Default)]
pub struct MultiMap2<K1, K2, V> {
    data: HashMap<K1, HashMap<K2, V>>,
}

impl<K1, K2, V> MultiMap2<K1, K2, V>
where
    K1: Eq + Hash,
    K2: Eq + Hash,
{
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn len(&self) -> usize {
        self.data.values().map(HashMap::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn insert(&mut self, k1: K1, k2: K2, value: V) -> Option<V> {
        self.data.entry(k1).or_default().insert(k2, value)
    }

    /// 获取值，缺失时插入默认值。
    /// Get a value, inserting the default when it is missing.
    pub fn get_or_insert_with<F>(&mut self, k1: K1, k2: K2, default: F) -> &mut V
    where
        F: FnOnce() -> V,
    {
        self.data
            .entry(k1)
            .or_default()
            .entry(k2)
            .or_insert_with(default)
    }

    pub fn get(&self, k1: &K1, k2: &K2) -> Option<&V> {
        self.data.get(k1)?.get(k2)
    }

    pub fn get_mut(&mut self, k1: &K1, k2: &K2) -> Option<&mut V> {
        self.data.get_mut(k1)?.get_mut(k2)
    }

    pub fn remove(&mut self, k1: &K1, k2: &K2) -> Option<V> {
        let mut remove_level1 = false;
        let value = {
            let level2 = self.data.get_mut(k1)?;
            let removed = level2.remove(k2);
            remove_level1 = level2.is_empty();
            removed
        };

        if remove_level1 {
            self.data.remove(k1);
        }

        value
    }

    /// 按可选键过滤，None 表示通配 / Filter by optional keys; None means wildcard.
    pub fn values_match(&self, k1: Option<&K1>, k2: Option<&K2>) -> Vec<&V> {
        let mut values = Vec::new();

        for (key1, level2) in &self.data {
            if let Some(k1_expected) = k1 {
                if key1 != k1_expected {
                    continue;
                }
            }

            for (key2, value) in level2 {
                if let Some(k2_expected) = k2 {
                    if key2 != k2_expected {
                        continue;
                    }
                }
                values.push(value);
            }
        }

        values
    }

    pub fn values_all(&self) -> Vec<&V> {
        self.values_match(None, None)
    }

    pub fn as_nested_map(&self) -> &HashMap<K1, HashMap<K2, V>> {
        &self.data
    }

    pub fn as_nested_map_mut(&mut self) -> &mut HashMap<K1, HashMap<K2, V>> {
        &mut self.data
    }
}

/// 三维多键映射 / Three-key nested map.
#[derive(Clone, Debug, Default)]
pub struct MultiMap3<K1, K2, K3, V> {
    data: HashMap<K1, HashMap<K2, HashMap<K3, V>>>,
}

impl<K1, K2, K3, V> MultiMap3<K1, K2, K3, V>
where
    K1: Eq + Hash,
    K2: Eq + Hash,
    K3: Eq + Hash,
{
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn len(&self) -> usize {
        self.data
            .values()
            .map(|level2| level2.values().map(HashMap::len).sum::<usize>())
            .sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn insert(&mut self, k1: K1, k2: K2, k3: K3, value: V) -> Option<V> {
        self.data
            .entry(k1)
            .or_default()
            .entry(k2)
            .or_default()
            .insert(k3, value)
    }

    /// 获取值，缺失时插入默认值。
    /// Get a value, inserting the default when it is missing.
    pub fn get_or_insert_with<F>(&mut self, k1: K1, k2: K2, k3: K3, default: F) -> &mut V
    where
        F: FnOnce() -> V,
    {
        self.data
            .entry(k1)
            .or_default()
            .entry(k2)
            .or_default()
            .entry(k3)
            .or_insert_with(default)
    }

    pub fn get(&self, k1: &K1, k2: &K2, k3: &K3) -> Option<&V> {
        self.data.get(k1)?.get(k2)?.get(k3)
    }

    pub fn get_mut(&mut self, k1: &K1, k2: &K2, k3: &K3) -> Option<&mut V> {
        self.data.get_mut(k1)?.get_mut(k2)?.get_mut(k3)
    }

    pub fn remove(&mut self, k1: &K1, k2: &K2, k3: &K3) -> Option<V> {
        let mut remove_level2 = false;
        let mut remove_level1 = false;

        let value = {
            let level2 = self.data.get_mut(k1)?;
            let level3 = level2.get_mut(k2)?;
            let removed = level3.remove(k3);
            remove_level2 = level3.is_empty();
            if remove_level2 {
                level2.remove(k2);
            }
            remove_level1 = level2.is_empty();
            removed
        };

        if remove_level1 {
            self.data.remove(k1);
        }

        value
    }

    /// 按可选键过滤，None 表示通配 / Filter by optional keys; None means wildcard.
    pub fn values_match(&self, k1: Option<&K1>, k2: Option<&K2>, k3: Option<&K3>) -> Vec<&V> {
        let mut values = Vec::new();

        for (key1, level2) in &self.data {
            if let Some(k1_expected) = k1 {
                if key1 != k1_expected {
                    continue;
                }
            }

            for (key2, level3) in level2 {
                if let Some(k2_expected) = k2 {
                    if key2 != k2_expected {
                        continue;
                    }
                }

                for (key3, value) in level3 {
                    if let Some(k3_expected) = k3 {
                        if key3 != k3_expected {
                            continue;
                        }
                    }
                    values.push(value);
                }
            }
        }

        values
    }

    pub fn values_all(&self) -> Vec<&V> {
        self.values_match(None, None, None)
    }

    pub fn as_nested_map(&self) -> &HashMap<K1, HashMap<K2, HashMap<K3, V>>> {
        &self.data
    }

    pub fn as_nested_map_mut(&mut self) -> &mut HashMap<K1, HashMap<K2, HashMap<K3, V>>> {
        &mut self.data
    }
}

/// 四维多键映射 / Four-key nested map.
#[derive(Clone, Debug, Default)]
pub struct MultiMap4<K1, K2, K3, K4, V> {
    data: HashMap<K1, HashMap<K2, HashMap<K3, HashMap<K4, V>>>>,
}

impl<K1, K2, K3, K4, V> MultiMap4<K1, K2, K3, K4, V>
where
    K1: Eq + Hash,
    K2: Eq + Hash,
    K3: Eq + Hash,
    K4: Eq + Hash,
{
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn len(&self) -> usize {
        self.data
            .values()
            .map(|level2| {
                level2
                    .values()
                    .map(|level3| level3.values().map(HashMap::len).sum::<usize>())
                    .sum::<usize>()
            })
            .sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn insert(&mut self, k1: K1, k2: K2, k3: K3, k4: K4, value: V) -> Option<V> {
        self.data
            .entry(k1)
            .or_default()
            .entry(k2)
            .or_default()
            .entry(k3)
            .or_default()
            .insert(k4, value)
    }

    /// 获取值，缺失时插入默认值。
    /// Get a value, inserting the default when it is missing.
    pub fn get_or_insert_with<F>(&mut self, k1: K1, k2: K2, k3: K3, k4: K4, default: F) -> &mut V
    where
        F: FnOnce() -> V,
    {
        self.data
            .entry(k1)
            .or_default()
            .entry(k2)
            .or_default()
            .entry(k3)
            .or_default()
            .entry(k4)
            .or_insert_with(default)
    }

    pub fn get(&self, k1: &K1, k2: &K2, k3: &K3, k4: &K4) -> Option<&V> {
        self.data.get(k1)?.get(k2)?.get(k3)?.get(k4)
    }

    pub fn get_mut(&mut self, k1: &K1, k2: &K2, k3: &K3, k4: &K4) -> Option<&mut V> {
        self.data.get_mut(k1)?.get_mut(k2)?.get_mut(k3)?.get_mut(k4)
    }

    pub fn remove(&mut self, k1: &K1, k2: &K2, k3: &K3, k4: &K4) -> Option<V> {
        let mut remove_level3 = false;
        let mut remove_level2 = false;
        let mut remove_level1 = false;

        let value = {
            let level2 = self.data.get_mut(k1)?;
            let level3 = level2.get_mut(k2)?;
            let level4 = level3.get_mut(k3)?;

            let removed = level4.remove(k4);

            remove_level3 = level4.is_empty();
            if remove_level3 {
                level3.remove(k3);
            }

            remove_level2 = level3.is_empty();
            if remove_level2 {
                level2.remove(k2);
            }

            remove_level1 = level2.is_empty();
            removed
        };

        if remove_level1 {
            self.data.remove(k1);
        }

        value
    }

    /// 按可选键过滤，None 表示通配 / Filter by optional keys; None means wildcard.
    pub fn values_match(
        &self,
        k1: Option<&K1>,
        k2: Option<&K2>,
        k3: Option<&K3>,
        k4: Option<&K4>,
    ) -> Vec<&V> {
        let mut values = Vec::new();

        for (key1, level2) in &self.data {
            if let Some(k1_expected) = k1 {
                if key1 != k1_expected {
                    continue;
                }
            }

            for (key2, level3) in level2 {
                if let Some(k2_expected) = k2 {
                    if key2 != k2_expected {
                        continue;
                    }
                }

                for (key3, level4) in level3 {
                    if let Some(k3_expected) = k3 {
                        if key3 != k3_expected {
                            continue;
                        }
                    }

                    for (key4, value) in level4 {
                        if let Some(k4_expected) = k4 {
                            if key4 != k4_expected {
                                continue;
                            }
                        }
                        values.push(value);
                    }
                }
            }
        }

        values
    }

    pub fn values_all(&self) -> Vec<&V> {
        self.values_match(None, None, None, None)
    }

    pub fn as_nested_map(&self) -> &HashMap<K1, HashMap<K2, HashMap<K3, HashMap<K4, V>>>> {
        &self.data
    }

    pub fn as_nested_map_mut(
        &mut self,
    ) -> &mut HashMap<K1, HashMap<K2, HashMap<K3, HashMap<K4, V>>>> {
        &mut self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multimap2_insert_get_match() {
        let mut map = MultiMap2::<&str, &str, i32>::new();
        map.insert("A", "x", 1);
        map.insert("A", "y", 2);
        map.insert("B", "x", 3);

        *map.get_or_insert_with("B", "z", || 4) += 1;

        assert_eq!(map.get(&"A", &"x"), Some(&1));
        assert_eq!(map.get(&"B", &"y"), None);
        assert_eq!(map.get(&"B", &"z"), Some(&5));

        let all_x = map.values_match(None, Some(&"x"));
        assert_eq!(all_x.len(), 2);

        let only_a = map.values_match(Some(&"A"), None);
        assert_eq!(only_a.len(), 2);
    }

    #[test]
    fn test_multimap2_remove_cleanup() {
        let mut map = MultiMap2::<&str, &str, i32>::new();
        map.insert("A", "x", 1);
        map.insert("A", "y", 2);

        assert_eq!(map.remove(&"A", &"x"), Some(1));
        assert_eq!(map.len(), 1);
        assert_eq!(map.remove(&"A", &"y"), Some(2));
        assert!(map.is_empty());
    }

    #[test]
    fn test_multimap3_match() {
        let mut map = MultiMap3::<&str, &str, &str, i32>::new();
        map.insert("A", "B", "C", 10);
        map.insert("A", "B", "D", 20);
        map.insert("X", "B", "C", 30);
        *map.get_or_insert_with("X", "Y", "Z", || 40) += 2;

        assert_eq!(map.values_match(Some(&"A"), Some(&"B"), None).len(), 2);
        assert_eq!(map.values_match(None, Some(&"B"), Some(&"C")).len(), 2);
        assert_eq!(map.get(&"X", &"B", &"C"), Some(&30));
        assert_eq!(map.get(&"X", &"Y", &"Z"), Some(&42));
    }

    #[test]
    fn test_multimap4_match_and_remove() {
        let mut map = MultiMap4::<&str, &str, &str, &str, i32>::new();
        map.insert("A", "B", "C", "D", 1);
        map.insert("A", "B", "C", "E", 2);
        map.insert("A", "B", "X", "Y", 3);
        *map.get_or_insert_with("K", "L", "M", "N", || 4) += 1;

        assert_eq!(
            map.values_match(Some(&"A"), Some(&"B"), Some(&"C"), None)
                .len(),
            2
        );
        assert_eq!(map.get(&"A", &"B", &"X", &"Y"), Some(&3));
        assert_eq!(map.get(&"K", &"L", &"M", &"N"), Some(&5));

        assert_eq!(map.remove(&"A", &"B", &"X", &"Y"), Some(3));
        assert_eq!(map.get(&"A", &"B", &"X", &"Y"), None);
    }
}

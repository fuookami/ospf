use crate::error::MappingIndexError;
use crate::multi_array::{MultiArray, MultiArrayCollection, MultiArrayToView};
use crate::multi_array_view::MultiArrayView;
use crate::multimap::{MultiMap2, MultiMap3, MultiMap4};
use crate::shape::AbstractShape;
use std::collections::HashMap;
use std::hash::Hash;

/// Map 全值访问扩展 / Get all values extension for HashMap.
pub trait MapAllValuesExt<V> {
    fn values_all(&self) -> Vec<&V>;
}

impl<K, V> MapAllValuesExt<V> for HashMap<K, V>
where
    K: Eq + Hash,
{
    fn values_all(&self) -> Vec<&V> {
        self.values().collect()
    }
}

/// HashMap<K, MultiArray<...>> 访问扩展 / Access extension for HashMap<K, MultiArray<...>>.
pub trait MapMultiArrayExt<K, T, S, C>
where
    S: AbstractShape,
    C: MultiArrayCollection<T>,
{
    fn array_get_linear(&self, key: &K, index: usize) -> Option<&T>;

    fn array_get_vector(&self, key: &K, vector: &S::VectorType) -> Option<&T>;

    fn array_view_by_dummy(
        &self,
        key: &K,
        dummy: &S::DummyVectorType,
    ) -> Option<Result<MultiArrayView<'_, T, S, crate::concept::AccessOrder, C>, MappingIndexError>>;
}

impl<K, T, S, C> MapMultiArrayExt<K, T, S, C> for HashMap<K, MultiArray<T, S, C>>
where
    K: Eq + Hash,
    S: AbstractShape<StorageOrder = crate::concept::StorageOrder>,
    C: MultiArrayCollection<T>,
{
    fn array_get_linear(&self, key: &K, index: usize) -> Option<&T> {
        let array = self.get(key)?;
        if index >= array.shape.len() {
            None
        } else {
            Some(&array[index])
        }
    }

    fn array_get_vector(&self, key: &K, vector: &S::VectorType) -> Option<&T> {
        let array = self.get(key)?;
        let linear = array.shape.index_of(vector).ok()?;
        Some(&array[linear])
    }

    fn array_view_by_dummy(
        &self,
        key: &K,
        dummy: &S::DummyVectorType,
    ) -> Option<Result<MultiArrayView<'_, T, S, crate::concept::AccessOrder, C>, MappingIndexError>>
    {
        Some(self.get(key)?.view(dummy))
    }
}

/// HashMap<K, MultiArray<...>> 可变访问扩展 / Mutable access extension for HashMap<K, MultiArray<...>>.
pub trait MapMultiArrayMutExt<K, T, S, C>
where
    S: AbstractShape,
    C: MultiArrayCollection<T>,
{
    fn array_set_linear(&mut self, key: &K, index: usize, value: T) -> bool;

    fn array_set_vector(&mut self, key: &K, vector: &S::VectorType, value: T) -> bool;
}

impl<K, T, S, C> MapMultiArrayMutExt<K, T, S, C> for HashMap<K, MultiArray<T, S, C>>
where
    K: Eq + Hash,
    S: AbstractShape<StorageOrder = crate::concept::StorageOrder>,
    C: MultiArrayCollection<T>,
{
    fn array_set_linear(&mut self, key: &K, index: usize, value: T) -> bool {
        let Some(array) = self.get_mut(key) else {
            return false;
        };

        if index >= array.shape.len() {
            false
        } else {
            array[index] = value;
            true
        }
    }

    fn array_set_vector(&mut self, key: &K, vector: &S::VectorType, value: T) -> bool {
        let Some(array) = self.get_mut(key) else {
            return false;
        };

        let Ok(linear) = array.shape.index_of(vector) else {
            return false;
        };

        array[linear] = value;
        true
    }
}

/// MultiMap2 中 MultiArray 值的访问扩展 / Access extension for MultiMap2 values as MultiArray.
pub trait MultiMap2ArrayExt<K1, K2, T, S, C>
where
    S: AbstractShape,
    C: MultiArrayCollection<T>,
{
    fn array_get_linear(&self, k1: &K1, k2: &K2, index: usize) -> Option<&T>;

    fn array_get_vector(&self, k1: &K1, k2: &K2, vector: &S::VectorType) -> Option<&T>;
}

impl<K1, K2, T, S, C> MultiMap2ArrayExt<K1, K2, T, S, C> for MultiMap2<K1, K2, MultiArray<T, S, C>>
where
    K1: Eq + Hash,
    K2: Eq + Hash,
    S: AbstractShape,
    C: MultiArrayCollection<T>,
{
    fn array_get_linear(&self, k1: &K1, k2: &K2, index: usize) -> Option<&T> {
        let array = self.get(k1, k2)?;
        if index >= array.shape.len() {
            None
        } else {
            Some(&array[index])
        }
    }

    fn array_get_vector(&self, k1: &K1, k2: &K2, vector: &S::VectorType) -> Option<&T> {
        let array = self.get(k1, k2)?;
        let linear = array.shape.index_of(vector).ok()?;
        Some(&array[linear])
    }
}

pub trait MultiMap2ArrayMutExt<K1, K2, T, S, C>
where
    S: AbstractShape,
    C: MultiArrayCollection<T>,
{
    fn array_set_linear(&mut self, k1: &K1, k2: &K2, index: usize, value: T) -> bool;

    fn array_set_vector(&mut self, k1: &K1, k2: &K2, vector: &S::VectorType, value: T) -> bool;
}

impl<K1, K2, T, S, C> MultiMap2ArrayMutExt<K1, K2, T, S, C>
    for MultiMap2<K1, K2, MultiArray<T, S, C>>
where
    K1: Eq + Hash,
    K2: Eq + Hash,
    S: AbstractShape,
    C: MultiArrayCollection<T>,
{
    fn array_set_linear(&mut self, k1: &K1, k2: &K2, index: usize, value: T) -> bool {
        let Some(array) = self.get_mut(k1, k2) else {
            return false;
        };

        if index >= array.shape.len() {
            false
        } else {
            array[index] = value;
            true
        }
    }

    fn array_set_vector(&mut self, k1: &K1, k2: &K2, vector: &S::VectorType, value: T) -> bool {
        let Some(array) = self.get_mut(k1, k2) else {
            return false;
        };

        let Ok(linear) = array.shape.index_of(vector) else {
            return false;
        };

        array[linear] = value;
        true
    }
}

/// MultiMap3 中 MultiArray 值的访问扩展 / Access extension for MultiMap3 values as MultiArray.
pub trait MultiMap3ArrayExt<K1, K2, K3, T, S, C>
where
    S: AbstractShape,
    C: MultiArrayCollection<T>,
{
    fn array_get_linear(&self, k1: &K1, k2: &K2, k3: &K3, index: usize) -> Option<&T>;

    fn array_get_vector(&self, k1: &K1, k2: &K2, k3: &K3, vector: &S::VectorType) -> Option<&T>;
}

impl<K1, K2, K3, T, S, C> MultiMap3ArrayExt<K1, K2, K3, T, S, C>
    for MultiMap3<K1, K2, K3, MultiArray<T, S, C>>
where
    K1: Eq + Hash,
    K2: Eq + Hash,
    K3: Eq + Hash,
    S: AbstractShape,
    C: MultiArrayCollection<T>,
{
    fn array_get_linear(&self, k1: &K1, k2: &K2, k3: &K3, index: usize) -> Option<&T> {
        let array = self.get(k1, k2, k3)?;
        if index >= array.shape.len() {
            None
        } else {
            Some(&array[index])
        }
    }

    fn array_get_vector(&self, k1: &K1, k2: &K2, k3: &K3, vector: &S::VectorType) -> Option<&T> {
        let array = self.get(k1, k2, k3)?;
        let linear = array.shape.index_of(vector).ok()?;
        Some(&array[linear])
    }
}

pub trait MultiMap3ArrayMutExt<K1, K2, K3, T, S, C>
where
    S: AbstractShape,
    C: MultiArrayCollection<T>,
{
    fn array_set_linear(&mut self, k1: &K1, k2: &K2, k3: &K3, index: usize, value: T) -> bool;

    fn array_set_vector(
        &mut self,
        k1: &K1,
        k2: &K2,
        k3: &K3,
        vector: &S::VectorType,
        value: T,
    ) -> bool;
}

impl<K1, K2, K3, T, S, C> MultiMap3ArrayMutExt<K1, K2, K3, T, S, C>
    for MultiMap3<K1, K2, K3, MultiArray<T, S, C>>
where
    K1: Eq + Hash,
    K2: Eq + Hash,
    K3: Eq + Hash,
    S: AbstractShape,
    C: MultiArrayCollection<T>,
{
    fn array_set_linear(&mut self, k1: &K1, k2: &K2, k3: &K3, index: usize, value: T) -> bool {
        let Some(array) = self.get_mut(k1, k2, k3) else {
            return false;
        };

        if index >= array.shape.len() {
            false
        } else {
            array[index] = value;
            true
        }
    }

    fn array_set_vector(
        &mut self,
        k1: &K1,
        k2: &K2,
        k3: &K3,
        vector: &S::VectorType,
        value: T,
    ) -> bool {
        let Some(array) = self.get_mut(k1, k2, k3) else {
            return false;
        };

        let Ok(linear) = array.shape.index_of(vector) else {
            return false;
        };

        array[linear] = value;
        true
    }
}

/// MultiMap4 中 MultiArray 值的访问扩展 / Access extension for MultiMap4 values as MultiArray.
pub trait MultiMap4ArrayExt<K1, K2, K3, K4, T, S, C>
where
    S: AbstractShape,
    C: MultiArrayCollection<T>,
{
    fn array_get_linear(&self, k1: &K1, k2: &K2, k3: &K3, k4: &K4, index: usize) -> Option<&T>;

    fn array_get_vector(
        &self,
        k1: &K1,
        k2: &K2,
        k3: &K3,
        k4: &K4,
        vector: &S::VectorType,
    ) -> Option<&T>;
}

impl<K1, K2, K3, K4, T, S, C> MultiMap4ArrayExt<K1, K2, K3, K4, T, S, C>
    for MultiMap4<K1, K2, K3, K4, MultiArray<T, S, C>>
where
    K1: Eq + Hash,
    K2: Eq + Hash,
    K3: Eq + Hash,
    K4: Eq + Hash,
    S: AbstractShape,
    C: MultiArrayCollection<T>,
{
    fn array_get_linear(&self, k1: &K1, k2: &K2, k3: &K3, k4: &K4, index: usize) -> Option<&T> {
        let array = self.get(k1, k2, k3, k4)?;
        if index >= array.shape.len() {
            None
        } else {
            Some(&array[index])
        }
    }

    fn array_get_vector(
        &self,
        k1: &K1,
        k2: &K2,
        k3: &K3,
        k4: &K4,
        vector: &S::VectorType,
    ) -> Option<&T> {
        let array = self.get(k1, k2, k3, k4)?;
        let linear = array.shape.index_of(vector).ok()?;
        Some(&array[linear])
    }
}

pub trait MultiMap4ArrayMutExt<K1, K2, K3, K4, T, S, C>
where
    S: AbstractShape,
    C: MultiArrayCollection<T>,
{
    fn array_set_linear(
        &mut self,
        k1: &K1,
        k2: &K2,
        k3: &K3,
        k4: &K4,
        index: usize,
        value: T,
    ) -> bool;

    fn array_set_vector(
        &mut self,
        k1: &K1,
        k2: &K2,
        k3: &K3,
        k4: &K4,
        vector: &S::VectorType,
        value: T,
    ) -> bool;
}

impl<K1, K2, K3, K4, T, S, C> MultiMap4ArrayMutExt<K1, K2, K3, K4, T, S, C>
    for MultiMap4<K1, K2, K3, K4, MultiArray<T, S, C>>
where
    K1: Eq + Hash,
    K2: Eq + Hash,
    K3: Eq + Hash,
    K4: Eq + Hash,
    S: AbstractShape,
    C: MultiArrayCollection<T>,
{
    fn array_set_linear(
        &mut self,
        k1: &K1,
        k2: &K2,
        k3: &K3,
        k4: &K4,
        index: usize,
        value: T,
    ) -> bool {
        let Some(array) = self.get_mut(k1, k2, k3, k4) else {
            return false;
        };

        if index >= array.shape.len() {
            false
        } else {
            array[index] = value;
            true
        }
    }

    fn array_set_vector(
        &mut self,
        k1: &K1,
        k2: &K2,
        k3: &K3,
        k4: &K4,
        vector: &S::VectorType,
        value: T,
    ) -> bool {
        let Some(array) = self.get_mut(k1, k2, k3, k4) else {
            return false;
        };

        let Ok(linear) = array.shape.index_of(vector) else {
            return false;
        };

        array[linear] = value;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MultiArrayBuilder;
    use crate::shape::Shape;

    #[test]
    fn test_hash_map_multi_array_ext() {
        let shape: Shape<2> = Shape::new([2, 2]);
        let mut array = MultiArrayBuilder::new_with(shape, 0i32);
        array[&[1, 1]] = 5;

        let mut map: HashMap<&str, _> = HashMap::new();
        map.insert("m", array);

        assert_eq!(map.array_get_linear(&"m", 3), Some(&5));
        assert_eq!(map.array_get_vector(&"m", &[1, 1]), Some(&5));

        assert!(map.array_set_linear(&"m", 0, 9));
        assert_eq!(map.array_get_linear(&"m", 0), Some(&9));

        assert!(map.array_set_vector(&"m", &[0, 1], 7));
        assert_eq!(map.array_get_vector(&"m", &[0, 1]), Some(&7));
    }

    #[test]
    fn test_multimap2_array_ext() {
        let shape: Shape<2> = Shape::new([2, 2]);
        let mut array = MultiArrayBuilder::new_with(shape, 1i32);
        array[&[1, 1]] = 6;

        let mut mm = MultiMap2::new();
        mm.insert("A", "B", array);

        assert_eq!(mm.array_get_vector(&"A", &"B", &[1, 1]), Some(&6));
        assert!(mm.array_set_linear(&"A", &"B", 0, 8));
        assert_eq!(mm.array_get_linear(&"A", &"B", 0), Some(&8));
    }

    #[test]
    fn test_multimap3_and_4_array_ext() {
        let shape: Shape<2> = Shape::new([2, 2]);

        let mut mm3 = MultiMap3::new();
        mm3.insert(
            "A",
            "B",
            "C",
            MultiArrayBuilder::new_with(shape.clone(), 2i32),
        );
        assert_eq!(mm3.array_get_linear(&"A", &"B", &"C", 1), Some(&2));
        assert!(mm3.array_set_vector(&"A", &"B", &"C", &[1, 1], 9));
        assert_eq!(mm3.array_get_vector(&"A", &"B", &"C", &[1, 1]), Some(&9));

        let mut mm4 = MultiMap4::new();
        mm4.insert("A", "B", "C", "D", MultiArrayBuilder::new_with(shape, 3i32));
        assert_eq!(mm4.array_get_linear(&"A", &"B", &"C", &"D", 2), Some(&3));
        assert!(mm4.array_set_linear(&"A", &"B", &"C", &"D", 2, 10));
        assert_eq!(mm4.array_get_linear(&"A", &"B", &"C", &"D", 2), Some(&10));
    }
}

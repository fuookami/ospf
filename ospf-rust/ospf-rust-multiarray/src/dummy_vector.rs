use crate::Shape;
use std::ops::{
    Bound, Range, RangeBounds, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive,
};

pub trait DummyIndexRange {
    fn start_bound(&self) -> Bound<isize>;
    fn end_bound(&self) -> Bound<isize>;
    fn contains(&self, v: isize) -> bool;
}

impl<T> DummyIndexRange for T
where
    T: RangeBounds<isize>,
{
    fn start_bound(&self) -> Bound<isize> {
        match RangeBounds::start_bound(self) {
            Bound::Included(value) => Bound::Included(*value),
            Bound::Excluded(value) => Bound::Excluded(*value),
            Bound::Unbounded => Bound::Unbounded,
        }
    }

    fn end_bound(&self) -> Bound<isize> {
        match RangeBounds::end_bound(self) {
            Bound::Included(value) => Bound::Included(*value),
            Bound::Excluded(value) => Bound::Excluded(*value),
            Bound::Unbounded => Bound::Unbounded,
        }
    }

    fn contains(&self, value: isize) -> bool {
        RangeBounds::contains(self, &value)
    }
}

enum DummyIndexIterator {
    Continuous(Range<usize>),
    Discrete(Vec<usize>),
}

impl DummyIndexIterator {
    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = usize> + 'a> {
        match self {
            DummyIndexIterator::Continuous(range) => Box::new(range.clone().into_iter()),
            DummyIndexIterator::Discrete(indexes) => Box::new(indexes.into_iter().map(|x| *x)),
        }
    }
}

pub enum DummyIndex {
    Index(isize),
    Range(Box<dyn DummyIndexRange>),
    IndexArray(Vec<isize>),
}

impl DummyIndex {
    fn iterator_of<S: Shape>(&self, shape: &S, dimension: usize) -> DummyIndexIterator {
        match self {
            DummyIndex::Index(index) => match shape.actual_index(dimension, *index) {
                Some(value) => DummyIndexIterator::Continuous(Range {
                    start: value,
                    end: value + 1,
                }),
                None => DummyIndexIterator::Continuous(Range { start: 0, end: 1 }),
            },
            DummyIndex::Range(range) => {
                let lower_bound = match range.start_bound() {
                    Bound::Included(value) => shape.actual_index(dimension, value),
                    Bound::Excluded(value) => shape.actual_index(dimension, value - 1),
                    Bound::Unbounded => Some(0),
                };
                let upper_bound = match range.end_bound() {
                    Bound::Included(value) => shape.actual_index(dimension, value + 1),
                    Bound::Excluded(value) => shape.actual_index(dimension, value),
                    Bound::Unbounded => Some(shape.len_of_dimension(dimension).unwrap()),
                };
                if lower_bound.is_some() && upper_bound.is_some() {
                    DummyIndexIterator::Continuous(Range {
                        start: lower_bound.unwrap(),
                        end: upper_bound.unwrap(),
                    })
                } else {
                    DummyIndexIterator::Continuous(Range { start: 0, end: 1 })
                }
            }
            DummyIndex::IndexArray(indexes) => DummyIndexIterator::Discrete(
                indexes
                    .iter()
                    .filter_map(move |index| shape.actual_index(dimension, *index))
                    .collect(),
            ),
        }
    }
}

impl From<isize> for DummyIndex {
    fn from(value: isize) -> Self {
        Self::Index(value)
    }
}

impl From<usize> for DummyIndex {
    fn from(value: usize) -> Self {
        Self::Index(value as isize)
    }
}

impl From<Range<isize>> for DummyIndex {
    fn from(range: Range<isize>) -> Self {
        Self::Range(Box::new(range))
    }
}

impl From<RangeFrom<isize>> for DummyIndex {
    fn from(range: RangeFrom<isize>) -> Self {
        Self::Range(Box::new(range))
    }
}

impl From<RangeInclusive<isize>> for DummyIndex {
    fn from(range: RangeInclusive<isize>) -> Self {
        Self::Range(Box::new(range))
    }
}

impl From<RangeTo<isize>> for DummyIndex {
    fn from(range: RangeTo<isize>) -> Self {
        Self::Range(Box::new(range))
    }
}

impl From<RangeToInclusive<isize>> for DummyIndex {
    fn from(range: RangeToInclusive<isize>) -> Self {
        Self::Range(Box::new(range))
    }
}

impl From<RangeFull> for DummyIndex {
    fn from(range: RangeFull) -> Self {
        Self::Range(Box::new(range))
    }
}

impl From<&[isize]> for DummyIndex {
    fn from(indexes: &[isize]) -> Self {
        Self::IndexArray(indexes.to_vec())
    }
}

impl From<Vec<isize>> for DummyIndex {
    fn from(indexes: Vec<isize>) -> Self {
        Self::IndexArray(indexes)
    }
}

pub(crate) struct DummyAccessPolicy<'a, S: Shape> {
    pub(self) shape: &'a S,
    pub(self) iterators: Vec<DummyIndexIterator>,
}

impl<'a, 'b, S: Shape> DummyAccessPolicy<'a, S> {
    pub(crate) fn new(vector: &'a S::DummyVectorType, shape: &'a S) -> Self {
        Self {
            shape: shape,
            iterators: (0..shape.dimension())
                .map(|i| vector[i].iterator_of(shape, i))
                .collect(),
        }
    }

    pub(crate) fn iter(&'b self) -> DummyAccessIterator<'a, 'b, S> {
        DummyAccessIterator::new(self)
    }
}

pub(crate) struct DummyAccessIterator<'a, 'b, S: Shape> {
    pub(self) policy: *const DummyAccessPolicy<'a, S>,
    pub(self) iterators: Vec<Box<dyn Iterator<Item = usize> + 'b>>,
    pub(crate) now: S::VectorType,
}

impl<'a, 'b, S: Shape> DummyAccessIterator<'a, 'b, S> {
    pub(crate) fn new(policy: &'b DummyAccessPolicy<'a, S>) -> Self {
        let mut ret = Self {
            policy: policy,
            iterators: policy.iterators.iter().map(|iter| iter.iter()).collect(),
            now: policy.shape.zero(),
        };
        for i in 0..(ret.iterators.len() - 1) {
            ret.now[i] = ret.iterators[i].next().unwrap();
        }
        ret
    }

    pub(crate) fn next(&mut self) -> Option<&S::VectorType> {
        for i in (0..self.iterators.len()).rev() {
            match self.iterators[i].next() {
                Some(value) => {
                    self.now[i] = value;
                    return Some(&self.now);
                }
                None => unsafe {
                    self.iterators[i] = (*self.policy).iterators[i].iter();
                    self.now[i] = self.iterators[i].next().unwrap();
                },
            }
        }
        return None;
    }
}

macro_rules! dummy_index {
    ($x:literal) => {
        DummyIndex::from($x as isize)
    };
    ($x:expr) => {
        DummyIndex::from($x)
    };
}

#[macro_export]
macro_rules! dummy {
    ($($x:expr),*) => {
        [$(dummy_index!($x),)*]
    };
}

#[macro_export]
macro_rules! dyn_dummy {
    ($($x:expr),*) => {
        vec!($(dummy_index!($x),)*)
    };
}

use crate::dummy_vector::DummyIndexRange;
use std::ops::{Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};

pub struct PlaceHolder {
    index: usize,
}

const _0: PlaceHolder = PlaceHolder { index: 0 };
const _1: PlaceHolder = PlaceHolder { index: 1 };
const _2: PlaceHolder = PlaceHolder { index: 2 };
const _3: PlaceHolder = PlaceHolder { index: 3 };
const _4: PlaceHolder = PlaceHolder { index: 4 };
const _5: PlaceHolder = PlaceHolder { index: 5 };
const _6: PlaceHolder = PlaceHolder { index: 6 };
const _7: PlaceHolder = PlaceHolder { index: 7 };
const _8: PlaceHolder = PlaceHolder { index: 8 };
const _9: PlaceHolder = PlaceHolder { index: 9 };
const _10: PlaceHolder = PlaceHolder { index: 10 };
const _11: PlaceHolder = PlaceHolder { index: 11 };
const _12: PlaceHolder = PlaceHolder { index: 12 };
const _13: PlaceHolder = PlaceHolder { index: 13 };
const _14: PlaceHolder = PlaceHolder { index: 14 };
const _15: PlaceHolder = PlaceHolder { index: 15 };
const _16: PlaceHolder = PlaceHolder { index: 16 };
const _17: PlaceHolder = PlaceHolder { index: 17 };
const _18: PlaceHolder = PlaceHolder { index: 18 };
const _19: PlaceHolder = PlaceHolder { index: 19 };
const _20: PlaceHolder = PlaceHolder { index: 20 };

pub(super) enum MapIndex<'a> {
    Index(isize),
    Range(Box<dyn DummyIndexRange>),
    IndexArray(Box<dyn Iterator<Item = isize> + 'a>),
    Map(PlaceHolder),
}

impl<'a> From<isize> for MapIndex<'a> {
    fn from(value: isize) -> Self {
        Self::Index(value)
    }
}

impl<'a> From<usize> for MapIndex<'a> {
    fn from(value: usize) -> Self {
        Self::Index(value as isize)
    }
}

impl<'a> From<Range<isize>> for MapIndex<'a> {
    fn from(range: Range<isize>) -> Self {
        Self::Range(Box::new(range))
    }
}

impl<'a> From<RangeFrom<isize>> for MapIndex<'a> {
    fn from(range: RangeFrom<isize>) -> Self {
        Self::Range(Box::new(range))
    }
}

impl<'a> From<RangeInclusive<isize>> for MapIndex<'a> {
    fn from(range: RangeInclusive<isize>) -> Self {
        Self::Range(Box::new(range))
    }
}

impl<'a> From<RangeTo<isize>> for MapIndex<'a> {
    fn from(range: RangeTo<isize>) -> Self {
        Self::Range(Box::new(range))
    }
}

impl<'a> From<RangeToInclusive<isize>> for MapIndex<'a> {
    fn from(range: RangeToInclusive<isize>) -> Self {
        Self::Range(Box::new(range))
    }
}

impl<'a> From<RangeFull> for MapIndex<'a> {
    fn from(range: RangeFull) -> Self {
        Self::Range(Box::new(range))
    }
}

impl<'a> From<&'a [isize]> for MapIndex<'a> {
    fn from(indexes: &'a [isize]) -> Self {
        Self::IndexArray(Box::new(indexes.iter().map(|index| *index)))
    }
}

impl<'a> From<PlaceHolder> for MapIndex<'a> {
    fn from(holder: PlaceHolder) -> Self {
        Self::Map(holder)
    }
}

// pub(super) struct MapAccessPolicy<'a, S: Shape> {
//     vector: &'a S::MapVectorType,
//     shape: S,
//     now: RefCell<S::VectorType>,
// }

macro_rules! map_index {
    ($x:literal) => {
        MapIndex::from($x as isize)
    };
    ($x:expr) => {
        MapIndex::from($x)
    };
}

#[macro_export]
macro_rules! map {
    ($($x:expr),*) => {
        &[$(map_index!($x),)*]
    };
}

#[macro_export]
macro_rules! dyn_map {
    ($($x:expr),*) => {
        &vec!($(map_index!($x),)*)
    };
}

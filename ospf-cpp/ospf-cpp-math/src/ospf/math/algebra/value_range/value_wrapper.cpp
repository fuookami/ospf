#include <ospf/math/algebra/value_range/value_wrapper.hpp>

namespace ospf::math::algebra::value_range
{
    template class ValueWrapper<i8>;
    template class ValueWrapper<u8>;
    template class ValueWrapper<i16>;
    template class ValueWrapper<u16>;
    template class ValueWrapper<i32>;
    template class ValueWrapper<u32>;
    template class ValueWrapper<i64>;
    template class ValueWrapper<u64>;
    //template class ValueWrapper<i128>;
    //template class ValueWrapper<u128>;
    //template class ValueWrapper<i256>;
    //template class ValueWrapper<u256>;
    //template class ValueWrapper<i512>;
    //template class ValueWrapper<u512>;
    //template class ValueWrapper<i1024>;
    //template class ValueWrapper<u1024>;
    //template class ValueWrapper<bigint>;

    template class ValueWrapper<f32>;
    template class ValueWrapper<f64>;
    //template class Valuewrapper<f128>;
    //template class Valuewrapper<f256>;
    //template class Valuewrapper<f512>;
    //template class Valuewrapper<dec50>;
    //template class Valuewrapper<dec100>;
};

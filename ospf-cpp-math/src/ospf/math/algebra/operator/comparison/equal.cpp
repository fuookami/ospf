#include <ospf/math/algebra/operator/comparison/equal.hpp>

namespace ospf::math::algebra::comparison_operator
{
    template class Equal<i8>;
    template class Equal<u8>;
    template class Equal<i16>;
    template class Equal<u16>;
    template class Equal<i32>;
    template class Equal<u32>;
    template class Equal<i64>;
    template class Equal<u64>;
    template class Equal<i128>;
    template class Equal<u128>;
    template class Equal<i256>;
    template class Equal<u256>;
    template class Equal<i512>;
    template class Equal<u512>;
    template class Equal<i1024>;
    template class Equal<u1024>;
    template class Equal<bigint>;

    template class Equal<f32>;
    template class Equal<f64>;
    template class Equal<f128>;
    template class Equal<f256>;
    template class Equal<f512>;
    template class Equal<dec50>;
    template class Equal<dec100>;
};

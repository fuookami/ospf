#include <ospf/math/algebra/operator/comparison/less_equal.hpp>

namespace ospf::math::algebra::comparison_operator
{
    template class LessEqual<i8>;
    template class LessEqual<u8>;
    template class LessEqual<i16>;
    template class LessEqual<u16>;
    template class LessEqual<i32>;
    template class LessEqual<u32>;
    template class LessEqual<i64>;
    template class LessEqual<u64>;
    template class LessEqual<i128>;
    template class LessEqual<u128>;
    template class LessEqual<i256>;
    template class LessEqual<u256>;
    template class LessEqual<i512>;
    template class LessEqual<u512>;
    template class LessEqual<i1024>;
    template class LessEqual<u1024>;
    template class LessEqual<bigint>;

    template class LessEqual<f32>;
    template class LessEqual<f64>;
    template class LessEqual<f128>;
    template class LessEqual<f256>;
    template class LessEqual<f512>;
    template class LessEqual<dec50>;
    template class LessEqual<dec100>;
};

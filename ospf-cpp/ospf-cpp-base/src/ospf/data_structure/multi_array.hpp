#pragma once

#include <ospf/data_structure/multi_array/dummy_index.hpp>
#include <ospf/data_structure/multi_array/map_index.hpp>
#include <ospf/data_structure/multi_array/shape.hpp>
#include <ospf/data_structure/multi_array/map_view.hpp>
#include <ospf/data_structure/multi_array/view.hpp>
#include <ospf/data_structure/multi_array/one_dimension.hpp>
#include <ospf/data_structure/multi_array/static_dimension.hpp>
#include <ospf/data_structure/multi_array/dynamic_dimension.hpp>

namespace ospf
{
    inline namespace data_structure
    {
        template<
            typename T,
            usize dim
        >
            requires NotSameAs<T, void>
        using MultiArray = multi_array::MultiArray<OriginType<T>, dim>;

        template<
            typename T,
            usize dim,
            multi_array::ShapeType S = DynShape
        >
            requires NotSameAs<T, void> && (S::dim == dynamic_dimension)
        using MultiArrayView = multi_array::MultiArrayView<MultiArray<T, dim>, S>;

        template<typename T>
        using MultiArray1 = MultiArray<T, 1_uz>;
        template<typename T>
        using MultiArrayView1 = MultiArrayView<T, 1_uz>;

        template<typename T>
        using MultiArray2 = MultiArray<T, 2_uz>;
        template<typename T>
        using MultiArrayView2 = MultiArrayView<T, 2_uz>;

        template<typename T>
        using MultiArray3 = MultiArray<T, 3_uz>;
        template<typename T>
        using MultiArrayView3 = MultiArrayView<T, 3_uz>;

        template<typename T>
        using MultiArray4 = MultiArray<T, 4_uz>;
        template<typename T>
        using MultiArrayView4 = MultiArrayView<T, 4_uz>;

        template<typename T>
        using MultiArray5 = MultiArray<T, 5_uz>;
        template<typename T>
        using MultiArrayView5 = MultiArrayView<T, 5_uz>;

        template<typename T>
        using MultiArray6 = MultiArray<T, 6_uz>;
        template<typename T>
        using MultiArrayView6 = MultiArrayView<T, 6_uz>;

        template<typename T>
        using MultiArray7 = MultiArray<T, 7_uz>;
        template<typename T>
        using MultiArrayView7 = MultiArrayView<T, 7_uz>;

        template<typename T>
        using MultiArray8 = MultiArray<T, 8_uz>;
        template<typename T>
        using MultiArrayView8 = MultiArrayView<T, 8_uz>;

        template<typename T>
        using MultiArray9 = MultiArray<T, 9_uz>;
        template<typename T>
        using MultiArrayView9 = MultiArrayView<T, 9_uz>;

        template<typename T>
        using MultiArray10 = MultiArray<T, 10_uz>;
        template<typename T>
        using MultiArrayView10 = MultiArrayView<T, 10_uz>;

        template<typename T>
        using MultiArray11 = MultiArray<T, 11_uz>;
        template<typename T>
        using MultiArrayView11 = MultiArrayView<T, 11_uz>;

        template<typename T>
        using MultiArray12 = MultiArray<T, 12_uz>;
        template<typename T>
        using MultiArrayView12 = MultiArrayView<T, 12_uz>;

        template<typename T>
        using MultiArray13 = MultiArray<T, 13_uz>;
        template<typename T>
        using MultiArrayView13 = MultiArrayView<T, 13_uz>;

        template<typename T>
        using MultiArray14 = MultiArray<T, 14_uz>;
        template<typename T>
        using MultiArrayView14 = MultiArrayView<T, 14_uz>;

        template<typename T>
        using MultiArray15 = MultiArray<T, 15_uz>;
        template<typename T>
        using MultiArrayView15 = MultiArrayView<T, 15_uz>;

        template<typename T>
        using MultiArray16 = MultiArray<T, 16_uz>;
        template<typename T>
        using MultiArrayView16 = MultiArrayView<T, 16_uz>;

        template<typename T>
        using MultiArray17 = MultiArray<T, 17_uz>;
        template<typename T>
        using MultiArrayView17 = MultiArrayView<T, 17_uz>;

        template<typename T>
        using MultiArray18 = MultiArray<T, 18_uz>;
        template<typename T>
        using MultiArrayView18 = MultiArrayView<T, 18_uz>;

        template<typename T>
        using MultiArray19 = MultiArray<T, 19_uz>;
        template<typename T>
        using MultiArrayView19 = MultiArrayView<T, 19_uz>;

        template<typename T>
        using MultiArray20 = MultiArray<T, 20_uz>;
        template<typename T>
        using MultiArrayView20 = MultiArrayView<T, 20_uz>;

        template<typename T>
        using DynMultiArray = MultiArray<T, dynamic_dimension>;
        template<typename T>
        using DynMultiArrayView = MultiArrayView<T, dynamic_dimension>;
    };
};

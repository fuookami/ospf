#pragma once

#include <ospf/concepts.hpp>
#include <concepts>
#include <format>
#include <string>
#include <string_view>
#include <type_traits>

namespace ospf
{
    inline namespace physics
    {
        namespace quantity
        {
            inline namespace fundamental_quantity
            {
                struct FundamentalDimensionBase {};

                template<typename T>
                concept FundamentalDimension = std::derived_from<T, FundamentalDimensionBase>
                    && requires
                {
                    { T::name } -> DecaySameAs<std::string_view>;
                    { T::description } -> DecaySameAs<std::string_view>;
                };

                template<FundamentalDimension Dim, i64 i>
                struct FundamentalQuantityType
                {
                    using Dimension = Dim;
                    static constexpr const i64 index = i;
                };

                template<typename T>
                concept FundamentalQuantity = requires
                {
                    requires FundamentalDimension<typename T::Dimension>;
                    { T::index } -> DecaySameAs<i64>;
                };

                template<FundamentalQuantity FQ, FundamentalDimension FD>
                struct IsDimension
                {
                    static constexpr const bool value = std::is_same_v<typename FQ::Dimension, FD>;
                };
                template<FundamentalQuantity FQ, FundamentalDimension FD>
                static constexpr const bool is_dimension = IsDimension<FQ, FD>::value;

                template<FundamentalQuantity FQ1, FundamentalQuantity FQ2>
                struct IsDimensionSame
                {
                    static constexpr const bool value = std::is_same_v<typename FQ1::Dimension, typename FQ2::Dimension>;
                };
                template<FundamentalQuantity FQ1, FundamentalQuantity FQ2>
                static constexpr const bool is_dimension_same = IsDimensionSame<FQ1, FQ2>::value;

                template<FundamentalQuantity FQ1, FundamentalQuantity FQ2>
                    requires is_dimension_same<FQ1, FQ2>
                struct Multiply
                {
                    using Dimension = typename FQ1::Dimension;
                    static constexpr const i64 index = FQ1::index + FQ2::index;
                };

                template<FundamentalQuantity FQ1, FundamentalQuantity FQ2>
                    requires is_dimension_same<FQ1, FQ2>
                struct Divide
                {
                    using Dimension = typename FQ1::Dimension;
                    static constexpr const i64 index = FQ1::index - FQ2::index;
                };

                template<FundamentalQuantity FQ>
                struct Neg
                {
                    using Dimension = typename FQ::Dimension;
                    static constexpr const i64 index = -FQ::index;
                };

                template<FundamentalQuantity FQ, i64 pow_index>
                struct Pow
                {
                    using Dimension = typename FQ::Dimension;
                    static constexpr const i64 index = FQ::index * pow_index;
                };

#ifndef OSPF_FUNDAMENTAL_DIMENSION_TEMPLATE
#define OSPF_FUNDAMENTAL_DIMENSION_TEMPLATE(Dim, Name, Desc) struct Dim: public FundamentalDimensionBase\
{\
    static constexpr const std::string_view name = Name;\
    static constexpr const std::string_view description = Desc;\
};
#endif

#ifndef OSPF_FUNDAMENTAL_QUANTITY_TEMPLATE
#define OSPF_FUNDAMENTAL_QUANTITY_TEMPLATE(Dim, Suffix, Index) struct Dim##_##Suffix\
{\
    using Dimension = Dim;\
    static constexpr const i64 index = Index;\
};
#endif
#ifndef OSPF_FUNDAMENTAL_QUANTITIES_TEMPLATE
#define OSPF_FUNDAMENTAL_QUANTITIES_TEMPLATE(Dim, Name, Desc) OSPF_FUNDAMENTAL_DIMENSION_TEMPLATE(Dim, Name, Desc)\
OSPF_FUNDAMENTAL_QUANTITY_TEMPLATE(Dim, 0, 0)\
OSPF_FUNDAMENTAL_QUANTITY_TEMPLATE(Dim, 1, 1)\
OSPF_FUNDAMENTAL_QUANTITY_TEMPLATE(Dim, N1, -1)\
OSPF_FUNDAMENTAL_QUANTITY_TEMPLATE(Dim, 2, 2)\
OSPF_FUNDAMENTAL_QUANTITY_TEMPLATE(Dim, N2, -2)\
OSPF_FUNDAMENTAL_QUANTITY_TEMPLATE(Dim, 3, 3)\
OSPF_FUNDAMENTAL_QUANTITY_TEMPLATE(Dim, N3, -3)\
OSPF_FUNDAMENTAL_QUANTITY_TEMPLATE(Dim, 4, 4)\
OSPF_FUNDAMENTAL_QUANTITY_TEMPLATE(Dim, N4, -4)\
OSPF_FUNDAMENTAL_QUANTITY_TEMPLATE(Dim, 5, 5)\
OSPF_FUNDAMENTAL_QUANTITY_TEMPLATE(Dim, N5, -5)\
OSPF_FUNDAMENTAL_QUANTITY_TEMPLATE(Dim, 6, 6)\
OSPF_FUNDAMENTAL_QUANTITY_TEMPLATE(Dim, N6, -6)\
OSPF_FUNDAMENTAL_QUANTITY_TEMPLATE(Dim, 7, 7)\
OSPF_FUNDAMENTAL_QUANTITY_TEMPLATE(Dim, N7, -7)\
OSPF_FUNDAMENTAL_QUANTITY_TEMPLATE(Dim, 8, 8)\
OSPF_FUNDAMENTAL_QUANTITY_TEMPLATE(Dim, N8, -8)\
OSPF_FUNDAMENTAL_QUANTITY_TEMPLATE(Dim, 9, 9)\
OSPF_FUNDAMENTAL_QUANTITY_TEMPLATE(Dim, N9, -9)\
OSPF_FUNDAMENTAL_QUANTITY_TEMPLATE(Dim, 10, 10)\
OSPF_FUNDAMENTAL_QUANTITY_TEMPLATE(Dim, N10, 10)
#endif
                OSPF_FUNDAMENTAL_QUANTITIES_TEMPLATE(L, "L", "length");
                OSPF_FUNDAMENTAL_QUANTITIES_TEMPLATE(M, "m", "mass");
                OSPF_FUNDAMENTAL_QUANTITIES_TEMPLATE(T, "t", "time");
                OSPF_FUNDAMENTAL_QUANTITIES_TEMPLATE(I, "I", "current");
                OSPF_FUNDAMENTAL_QUANTITIES_TEMPLATE(O, "жи", "temperature");
                OSPF_FUNDAMENTAL_QUANTITIES_TEMPLATE(N, "n", "substance amount");
                OSPF_FUNDAMENTAL_QUANTITIES_TEMPLATE(J, "l", "luminous intensity");
                OSPF_FUNDAMENTAL_QUANTITIES_TEMPLATE(R, "rad", "plane angle");
                OSPF_FUNDAMENTAL_QUANTITIES_TEMPLATE(S, "sr", "solid angle");
                OSPF_FUNDAMENTAL_QUANTITIES_TEMPLATE(B, "i", "information");
#undef OSPF_FUNDAMENTAL_DIMENSION_TEMPLATE
#undef OSPF_FUNDAMENTAL_QUANTITIES_TEMPLATE
#undef OSPF_FUNDAMENTAL_QUANTITY_TEMPLATE
            };
        };
    };
};

template<ospf::physics::quantity::fundamental_quantity::FundamentalDimension FD, typename CharT>
struct std::formatter<FD, CharT> : std::formatter<std::string_view, CharT> {
    template<class FormatContext>
    auto format(const FD fd, FormatContext& fc) {
        return std::formatter<std::string_view, CharT>::format(FD::name, fc);
    }
};

template<ospf::physics::quantity::fundamental_quantity::FundamentalQuantity FQ, typename CharT>
struct std::formatter<FQ, CharT> : std::formatter<std::string_view, CharT> {
    template<class FormatContext>
    auto format(const FQ fq, FormatContext& fc) {
        return std::formatter<std::string_view, CharT>::format((typename FQ::Dimension)::name + std::to_string(fq.index), fc);
    }
};

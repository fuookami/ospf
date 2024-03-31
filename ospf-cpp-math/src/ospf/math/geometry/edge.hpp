#pragma once

#include <ospf/functional/value_or_reference.hpp>
#include <ospf/math/geometry/point.hpp>

namespace ospf
{
    inline namespace math
    {
        inline namespace geometry
        {
            template<usize dim, RealNumber T = f64>
            class Edge
            {
            public:
                using ValueType = OriginType<T>;
                using PointType = Point<dim, ValueType>;

            public:
                inline static constexpr Edge between(ArgCLRefType<COW<PointType>> from, ArgCLRefType<COW<PointType>> to) noexcept
                {
                    return Edge{ from, to };
                }

                template<typename = void>
                    requires ReferenceFaster<COW<PointType>> && std::movable<COW<PointType>>
                inline static constexpr Edge between(ArgRRefType<COW<PointType>> from, ArgRRefType<COW<PointType>> to) noexcept
                {
                    return Edge{ move<COW<PointType>>(from), move<COW<PointType>>(to) };
                }

            public:
                constexpr Edge(ArgCLRefType<COW<PointType>> from, ArgCLRefType<COW<PointType>> to)
                    : _from(from), _to(to) {}

                template<typename = void>
                    requires ReferenceFaster<COW<PointType>> && std::movable<COW<PointType>>
                constexpr Edge(ArgRRefType<COW<PointType>> from, ArgRRefType<COW<PointType>> to)
                    : _from(move<COW<PointType>>(from)), _to(move<COW<PointType>>(to)) {}

            public:
                constexpr Edge(const Edge& ano) = default;
                constexpr Edge(Edge&& ano) noexcept = default;
                constexpr Edge& operator=(const Edge& rhs) = default;
                constexpr Edge& operator=(Edge&& rhs) noexcept = default;
                constexpr ~Edge(void) noexcept = default;

            public:
                inline constexpr ArgCLRefType<PointType> from(void) const noexcept
                {
                    return *_from;
                }

                inline constexpr ArgCLRefType<PointType> to(void) const noexcept
                {
                    return *_to;
                }

            public:
                inline constexpr void copy_if_reference(void) noexcept
                {
                    _from.copy_if_reference();
                    _to.copy_if_reference();
                }

            public:
                inline constexpr RetType<ValueType> length(void) const noexcept
                {
                    ValueType sum{ ArithmeticTrait<ValueType>::zero() };
                    for (usize i{ 0_uz }; i != dim; ++i)
                    {
                        sum += sqr((*_to)[i] - (*_from)[i]);
                    }
                    return **sqrt(sum);
                }

            public:
                inline constexpr operator RetType<Vector<dim, ValueType>>(void) const noexcept
                {
                    return Vector<dim, ValueType>
                    { 
                        make_array<ValueType, dim>([this](const usize i) -> RetType<ValueType>
                        {
                                return (*this->_to)[i] - (*this->_from)[i];
                        })
                    };
                }

            public:
                inline constexpr const bool operator==(const Edge& rhs) const noexcept
                {
                    return _from == rhs._from
                        && _to == rhs._to;
                }

                inline constexpr const bool operator!=(const Edge& rhs) const noexcept
                {
                    return _from != rhs._from
                        || _to != rhs._to;
                }

            private:
                COW<PointType> _from;
                COW<PointType> _to;
            };

            template<RealNumber T = f64>
            using Edge2 = Edge<2_uz, T>;

            template<RealNumber T = f64>
            using Edge3 = Edge<3_uz, T>;

            extern template class Edge<2_uz, f64>;
            extern template class Edge<3_uz, f64>;
        };
    };
};

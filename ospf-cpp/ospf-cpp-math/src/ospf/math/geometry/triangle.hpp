#pragma once

#include <ospf/math/geometry/point.hpp>
#include <ospf/math/geometry/edge.hpp>

namespace ospf
{
    inline namespace math
    {
        inline namespace geometry
        {
            template<usize dim, RealNumber T = f64>
                requires (dim >= 2_uz)
            class Triangle
            {
            public:
                using ValueType = OriginType<T>;
                using PointType = Point<dim, ValueType>;
                using EdgeType = Edge<dim, ValueType>;

            public:
                constexpr Triangle(ArgCLRefType<COW<PointType>> p1, ArgCLRefType<COW<PointType>> p2, ArgCLRefType<COW<PointType>> p3)
                    : _p1(p1), _p2(p2), _p3(p3) {}

                template<typename = void>
                    requires ReferenceFaster<COW<PointType>> && std::movable<COW<PointType>>
                constexpr Triangle(ArgRRefType<COW<PointType>> p1, ArgRRefType<COW<PointType>> p2, ArgRRefType<COW<PointType>> p3)
                    : _p1(move<PointType>(p1)), _p2(move<PointType>(p2)), _p3(move<PointType>(p3)) {}

            public:
                constexpr Triangle(const Triangle& ano) = default;
                constexpr Triangle(Triangle&& ano) noexcept = default;
                constexpr Triangle& operator=(const Triangle& rhs) = default;
                constexpr Triangle& operator=(Triangle&& rhs) noexcept = default;
                constexpr ~Triangle(void) noexcept = default;

            public:
                inline constexpr ArgCLRefType<PointType> p1(void) const noexcept
                {
                    return *_p1;
                }

                inline constexpr ArgCLRefType<PointType> p2(void) const noexcept
                {
                    return *_p2;
                }

                inline constexpr ArgCLRefType<PointType> p3(void) const noexcept
                {
                    return *_p3;
                }

            public:
                inline constexpr void copy_if_reference(void) noexcept
                {
                    _p1.copy_if_reference();
                    _p2.copy_if_reference();
                    _p3.copy_if_reference();
                }

            public:
                inline constexpr RetType<EdgeType> e1(void) const noexcept
                {
                    return EdgeType::between(_p1, _p2);
                }

                inline constexpr RetType<EdgeType> e2(void) const noexcept
                {
                    return EdgeType::between(_p2, _p3);
                }

                inline constexpr RetType<EdgeType> e3(void) const noexcept
                {
                    return EdgeType::between(_p3, _p1);
                }

            public:
                // Heron's formula
                inline constexpr RetType<ValueType> area(void) const noexcept
                {
                    const auto a = e1().length();
                    const auto b = e2().length();
                    const auto c = e3().length();
                    const auto p = (a + b + c) / RealNumberTrait<ValueType>::two();
                    const auto s = p * (p - a) * (p - b) * (p - c);
                    return **sqrt(s);
                }

                inline constexpr const bool illegal(void) const noexcept
                {
                    static const Equal<ValueType> op{};
                    for (usize i{ 0_uz }; i != dim; ++i)
                    {
                        if (op((*_p1)[i], (*_p2)[i]) && op((*_p2)[i], (*_p3)[i]))
                        {
                            return true;
                        }
                    }
                    return false;
                }

            private:
                COW<PointType> _p1;
                COW<PointType> _p2;
                COW<PointType> _p3;
            };

            template<RealNumber T = f64>
            using Triangle2 = Triangle<2_uz, T>;

            template<RealNumber T = f64>
            using Triangle3 = Triangle<3_uz, T>;

            extern template class Triangle<2_uz, f64>;
            extern template class Triangle<3_uz, f64>;
        };
    };
};

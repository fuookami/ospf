#pragma once

#include <ospf/math/geometry/point.hpp>
#include <ospf/math/geometry/triangle.hpp>

namespace ospf
{
    inline namespace math
    {
        inline namespace geometry
        {
            template<usize dim, RealNumber T = f64>
                requires (dim >= 2_uz)
            class Circle
            {
            public:
                using ValueType = OriginType<T>;
                using PointType = Point<dim, ValueType>;
                using VectorType = Vector<dim, ValueType>;

            public:
                constexpr Circle(ArgCLRefType<COW<PointType>> center, ArgCLRefType<VectorType> radius)
                    : _center(center), _direction(radius.unit()), _radius(radius.norm()) {}

                template<typename = void>
                    requires ReferenceFaster<COW<PointType>> && std::movable<PointType>
                constexpr Circle(ArgRRefType<COW<PointType>> center, ArgCLRefType<VectorType> radius)
                    : _center(move<COW<PointType>>(center)), _direction(radius.unit()), _radius(radius.norm()) {}

            public:
                constexpr Circle(const Circle& ano) = default;
                constexpr Circle(Circle&& ano) noexcept = default;
                constexpr Circle& operator=(const Circle& rhs) = default;
                constexpr Circle& operator=(Circle&& rhs) noexcept = default;
                constexpr ~Circle(void) = default;

            public:
                inline constexpr ArgCLRefType<PointType> center(void) const noexcept
                {
                    return _center;
                }

                inline constexpr ArgCLRefType<ValueType> radius(void) const noexcept
                {
                    return _radius;
                }

            private:
                PointType _center;
                VectorType _direction;
                ValueType _radius;
            };

            template<RealNumber T>
            class Circle<2_uz, T>
            {
            public:
                using ValueType = OriginType<T>;
                using PointType = Point2<ValueType>;

            public:
                inline static Circle circum_circle_of(const Triangle2<ValueType>& triangle) noexcept
                {
                    const auto ax = triangle.p2().x() - triangle.p1().x();
                    const auto ay = triangle.p2().y() - triangle.p1().y();
                    const auto bx = triangle.p3().x() - triangle.p1().x();
                    const auto by = triangle.p3().y() - triangle.p1().y();

                    const auto m = sqr(triangle.p2().x()) - sqr(triangle.p1().x()) + sqr(triangle.p2().y()) - sqr(triangle.p1().y());
                    const auto u = sqr(triangle.p3().x()) - sqr(triangle.p1().x()) + sqr(triangle.p3().y()) - sqr(triangle.p1().y());
                    const auto s = ArithmeticTrait<ValueType>::one() / (RealNumberTrait<ValueType>::two() * (ax * by - ay * bx));

                    auto x = ((triangle.p3().y() - triangle.p1().y()) * m + (triangle.p1().y() - triangle.p2().y()) * u) * s;
                    auto y = ((triangle.p1().x() - triangle.p3().x()) * m + (triangle.p2().x() - triangle.p1().x()) * u) * s;
                    auto r = **sqrt(sqr(triangle.p1().x() - x) + sqr(triangle.p1().y() - y));
                    
                    return Circle{ PointType{ move<ValueType>(x), move<ValueType>(y) }, move<ValueType>(r) };
                }

            public:
                constexpr Circle(ArgCLRefType<COW<PointType>> center, ArgCLRefType<ValueType> radius)
                    : _center(center), _radius(radius) {}

                template<typename = void>
                    requires ReferenceFaster<COW<PointType>> && std::movable<PointType>
                constexpr Circle(ArgCLRefType<COW<PointType>> center, ArgCLRefType<ValueType> radius)
                    : _center(move<COW<PointType>>(center)), _radius(radius) {}

                template<typename = void>
                    requires ReferenceFaster<COW<PointType>> && std::movable<PointType>
                        && ReferenceFaster<ValueType> && std::movable<ValueType>
                constexpr Circle(ArgCLRefType<COW<PointType>> center, ArgRRefType<ValueType> radius)
                    : _center(move<COW<PointType>>(center)), _radius(move<ValueType>(radius)) {}

            public:
                inline constexpr ArgCLRefType<PointType> center(void) const noexcept
                {
                    return _center;
                }

                inline constexpr ArgCLRefType<ValueType> x(void) const noexcept
                {
                    return _center.x();
                }

                inline constexpr ArgCLRefType<ValueType> y(void) const noexcept
                {
                    return _center.y();
                }

                inline constexpr ArgCLRefType<ValueType> radius(void) const noexcept
                {
                    return _radius;
                }

            private:
                PointType _center;
                ValueType _radius;
            };

            template<RealNumber T = f64>
            using Circle2 = Circle<2_uz, T>;

            extern template class Circle<2_uz, f64>;
        };
    };
};

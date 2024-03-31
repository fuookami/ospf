#pragma once

#include <ospf/math/algebra/vector.hpp>

namespace ospf
{
    inline namespace math
    {
        inline namespace geometry
        {
            template<usize dim, RealNumber T = f64>
            class Point
            {
            public:
                using ValueType = OriginType<T>;
                using VectorType = Vector<dim, ValueType>;

            public:
                constexpr Point(std::array<ValueType, dim> coordinate)
                    : _coordinate(std::move(coordinate)) {}

            public:
                constexpr Point(const Point& ano) = default;
                constexpr Point(Point&& ano) noexcept = default;
                constexpr Point& operator=(const Point& rhs) = default;
                constexpr Point& operator=(Point&& rhs) = default;
                constexpr ~Point(void) noexcept = default;

            public:
                inline constexpr operator RetType<VectorType>(void) const noexcept
                {
                    return VectorType{ _coordinate };
                }

            public:
                inline constexpr ArgCLRefType<ValueType> operator[](const usize i) const noexcept
                {
                    return _coordinate[i];
                }

            public:
                inline constexpr Point operator+(ArgCLRefType<VectorType> vector) const noexcept
                {
                    return Point
                    { 
                        make_array<ValueType, dim>([this, &vector](const usize i) -> RetType<ValueType>
                        {
                            return this->_coordinate[i] + vector[i];
                        })
                    };
                }

                inline constexpr Point operator-(ArgCLRefType<VectorType> vector) const noexcept
                {
                    return Point
                    { 
                        make_array<ValueType, dim>([this, &vector](const usize i) -> RetType<ValueType>
                        {
                            return this->_coordinate[i] - vector[i];
                        })
                    };
                }

            public:
                inline constexpr const bool operator==(const Point& rhs) const noexcept
                {
                    static const Equal<ValueType> op{};
                    for (usize i{ 0_uz }; i != dim; ++i)
                    {
                        if (!op(_coordinate[i], rhs._coordinate[i]))
                        {
                            return false;
                        }
                    }
                    return true;
                }

                inline constexpr const bool operator!=(const Point& rhs) const noexcept
                {
                    static const Equal<ValueType> op{};
                    for (usize i{ 0_uz }; i != dim; ++i)
                    {
                        if (!op(_coordinate[i], rhs._coordinate[i]))
                        {
                            return false;
                        }
                    }
                    return true;
                }

            private:
                std::array<ValueType, dim> _coordinate;
            };

            template<RealNumber T>
            class Point<2_uz, T>
            {
            public:
                using ValueType = OriginType<T>;
                using VectorType = Vector2<ValueType>;

            public:
                constexpr Point(std::array<ValueType, 2_uz> coordinate)
                    : _coordinate(std::move(coordinate)) {}

                constexpr Point(ArgCLRefType<ValueType> x, ArgCLRefType<ValueType> y)
                    : _coordinate(x, y) {}

                template<typename = void>
                    requires ReferenceFaster<ValueType> && std::movable<ValueType>
                constexpr Point(ArgRRefType<ValueType> x, ArgRRefType<ValueType> y)
                    : _coordinate(move<ValueType>(x), move<ValueType>(y)) {}

            public:
                constexpr Point(const Point& ano) = default;
                constexpr Point(Point&& ano) noexcept = default;
                constexpr Point& operator=(const Point& rhs) = default;
                constexpr Point& operator=(Point&& rhs) noexcept = default;
                constexpr ~Point(void) noexcept = default;

            public:
                inline constexpr ArgCLRefType<ValueType> x(void) const noexcept
                {
                    return _coordinate[0_uz];
                }

                inline constexpr ArgCLRefType<ValueType> y(void) const noexcept
                {
                    return _coordinate[1_uz];
                }

            public:
                inline constexpr operator RetType<VectorType>(void) const noexcept
                {
                    return VectorType{ _coordinate };
                }

            public:
                inline constexpr ArgCLRefType<ValueType> operator[](const usize i) const noexcept
                {
                    return _coordinate[i];
                }

            public:
                inline constexpr Point operator+(ArgCLRefType<VectorType> rhs) const noexcept
                {
                    return Point
                    { 
                        x() + rhs.x(),
                        y() + rhs.y()
                    };
                }

                inline constexpr Point operator-(ArgCLRefType<VectorType> rhs) const noexcept
                {
                    return Point
                    {
                        x() - rhs.x(),
                        y() - rhs.y()
                    };
                }

            public:
                inline constexpr const bool operator==(const Point& rhs) const noexcept
                {
                    static const Equal<ValueType> op{};
                    return op(x(), rhs.x()) && op(y(), rhs.y());
                }

                inline constexpr const bool operator!=(const Point& rhs) const noexcept
                {
                    static const Unequal<ValueType> op{};
                    return op(x(), rhs.y()) || op(y(), rhs.y());
                }

            private:
                std::array<ValueType, 2_uz> _coordinate;
            };

            template<RealNumber T>
            class Point<3_uz, T>
            {
            public:
                using ValueType = OriginType<T>;
                using VectorType = Vector3<ValueType>;

            public:
                constexpr Point(std::array<ValueType, 3_uz> coordinate)
                    : _coordinate(std::move(coordinate)) {}

                constexpr Point(ArgCLRefType<ValueType> x, ArgCLRefType<ValueType> y, ArgCLRefType<ValueType> z)
                    : _coordinate(x, y, z) {}

                template<typename = void>
                    requires ReferenceFaster<ValueType> && std::movable<ValueType>
                constexpr Point(ArgRRefType<ValueType> x, ArgRRefType<ValueType> y, ArgRRefType<ValueType> z)
                    : _coordinate(move<ValueType>(x), move<ValueType>(y), move<ValueType>(z)) {}

            public:
                constexpr Point(const Point& ano) = default;
                constexpr Point(Point&& ano) noexcept = default;
                constexpr Point& operator=(const Point& rhs) = default;
                constexpr Point& operator=(Point&& rhs) noexcept = default;
                constexpr ~Point(void) noexcept = default;

            public:
                inline constexpr ArgCLRefType<ValueType> x(void) const noexcept
                {
                    return _coordinate[0_uz];
                }

                inline constexpr ArgCLRefType<ValueType> y(void) const noexcept
                {
                    return _coordinate[1_uz];
                }

                inline constexpr ArgCLRefType<ValueType> z(void) const noexcept
                {
                    return _coordinate[2_uz];
                }

            public:
                inline constexpr operator RetType<VectorType>(void) const noexcept
                {
                    return VectorType{ _coordinate };
                }

            public:
                inline constexpr ArgCLRefType<ValueType> operator[](const usize i) const noexcept
                {
                    return _coordinate[i];
                }

            public:
                inline constexpr Point operator+(ArgCLRefType<VectorType> rhs) const noexcept
                {
                    return Point
                    {
                        x() + rhs.x(),
                        y() + rhs.y(),
                        z() + rhs.z()
                    };
                }

                inline constexpr Point operator-(ArgCLRefType<VectorType> rhs) const noexcept
                {
                    return Point
                    {
                        x() - rhs.x(),
                        y() - rhs.y(),
                        x() - rhs.z()
                    };
                }

            public:
                inline constexpr const bool operator==(const Point& rhs) const noexcept
                {
                    static const Equal<ValueType> op{};
                    return op(x(), rhs.x()) && op(y(), rhs.y()) && op(z(), rhs.z());
                }

                inline constexpr const bool operator!=(const Point& rhs) const noexcept
                {
                    static const Unequal<ValueType> op{};
                    return op(x(), rhs.y()) || op(y(), rhs.y()) || op(z(), rhs.z());
                }

            private:
                std::array<ValueType, 2_uz> _coordinate;
            };

            template<RealNumber T = f64>
            using Point2 = Point<2_uz, T>;

            template<RealNumber T = f64>
            using Point3 = Point<3_uz, T>;

            extern template class Point<2_uz, f64>;
            extern template class Point<2_uz, f64>;
        };
    };
};

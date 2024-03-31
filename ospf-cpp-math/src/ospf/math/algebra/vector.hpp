#pragma once

#include <ospf/functional/array.hpp>
#include <ospf/math/algebra/concepts/real_number.hpp>
#include <ospf/math/algebra/operator/arithmetic/pow.hpp>
#include <ospf/math/algebra/operator/arithmetic/sqrt.hpp>
#include <ospf/math/algebra/operator/comparison/equal.hpp>
#include <ospf/math/algebra/operator/comparison/unequal.hpp>

namespace ospf
{
    inline namespace math
    {
        inline namespace algebra
        {
            template<usize dim, RealNumber T = f64>
            class Vector
            {
            public:
                using ValueType = OriginType<T>;

            public:
                constexpr Vector(std::array<ValueType, dim> values)
                    : _values(std::move(_values)) {}

            public:
                constexpr Vector(const Vector& ano) = default;
                constexpr Vector(Vector&& ano) noexcept = default;
                constexpr Vector& operator=(const Vector& rhs) = default;
                constexpr Vector& operator=(Vector&& rhs) noexcept = default;
                constexpr ~Vector(void) noexcept = default;

            public:
                inline constexpr RetType<ValueType> norm(void) const noexcept
                {
                    ValueType sum{ ArithmeticTrait<ValueType>::zero() };
                    for (usize i{ 0_uz }; i != dim; ++i)
                    {
                        sum += sqr(_values[i]);
                    }
                    return **sqrt(sum);
                }

                inline constexpr Vector unit(void) const noexcept
                {
                    const auto norm = this->norm();
                    return Vector
                    {
                        make_array<ValueType, dim>([this, &norm](const usize i)
                        {
                            return this->_values[i] / norm;
                        })
                    };
                }

            public:
                inline constexpr ArgCLRefType<ValueType> operator[](const usize i) const noexcept
                {
                    return _values[i];
                }

            public:
                inline constexpr Vector operator+(const Vector& rhs) const noexcept
                {
                    return Vector
                    {
                        make_array<ValueType, dim>([this, &rhs](const usize i)
                        {
                            return this->_values[i] + rhs._values[i];
                        })
                    };
                }

                inline constexpr Vector operator-(const Vector& rhs) const noexcept
                {
                    return Vector
                    {
                        make_array<ValueType, dim>([this, &rhs](const usize i)
                        {
                            return this->_values[i] - rhs._values[i];
                        })
                    };
                }

            public:
                inline constexpr const bool operator==(const Vector& rhs) const noexcept
                {
                    static const Equal<ValueType> op{};
                    for (usize i{ 0_uz }; i != dim; ++i)
                    {
                        if (!op(_values[i], rhs._values[i]))
                        {
                            return false;
                        }
                    }
                    return true;
                }

                inline constexpr const bool operator!=(const Vector& rhs) const noexcept
                {
                    static const Unequal<ValueType> op{};
                    for (usize i{ 0_uz }; i != dim; ++i)
                    {
                        if (!op(_values[i], rhs._values[i]))
                        {
                            return false;
                        }
                    }
                    return true;
                }

            private:
                std::array<ValueType, dim> _values;
            };

            template<RealNumber T>
            class Vector<2_uz, T>
            {
            public:
                using ValueType = OriginType<T>;
                
            public:
                constexpr Vector(std::array<ValueType, 2_uz> values)
                    : _values(std::move(values)) {}

                constexpr Vector(ArgCLRefType<ValueType> x, ArgCLRefType<ValueType> y)
                    : _values(x, y) {}

                template<typename = void>
                    requires ReferenceFaster<ValueType> && std::movable<ValueType>
                constexpr Vector(ArgRRefType<ValueType> x, ArgRRefType<ValueType> y)
                    : _values(move<ValueType>(x), move<ValueType>(y)) {}
                
            public:
                constexpr Vector(const Vector& ano) = default;
                constexpr Vector(Vector&& ano) noexcept = default;
                constexpr Vector& operator=(const Vector& rhs) = default;
                constexpr Vector& operator=(Vector&& rhs) noexcept = default;
                constexpr ~Vector(void) noexcept = default;

            public:
                inline constexpr ArgCLRefType<ValueType> x(void) const noexcept
                {
                    return _values[0_uz];
                }

                inline constexpr ArgCLRefType<ValueType> y(void) const noexcept
                {
                    return _values[1_uz];
                }

            public:
                inline constexpr RetType<ValueType> norm(void) const noexcept
                {
                    return **sqrt(sqr(x()) + sqr(y()));
                }

                inline constexpr Vector unit(void) const noexcept
                {
                    const auto this_norm = this->norm();
                    return Vector
                    {
                        x() / this_norm,
                        y() / this_norm
                    };
                }

            public:
                inline constexpr ArgCLRefType<ValueType> operator[](const usize i) const noexcept
                {
                    return _values[i];
                }

            public:
                inline constexpr Vector operator+(const Vector& rhs) const noexcept
                {
                    return Vector
                    {
                        x() + rhs.x(),
                        y() + rhs.y()
                    };
                }

                inline constexpr Vector operator-(const Vector& rhs) const noexcept
                {
                    return Vector
                    {
                        x() - rhs.x(),
                        y() - rhs.y()
                    };
                }

            public:
                inline constexpr const bool operator==(const Vector& rhs) const noexcept
                {
                    static const Equal<ValueType> op{};
                    return op(x(), rhs.x()) && op(y(), rhs.y());
                }

                inline constexpr const bool operator!=(const Vector& rhs) const noexcept
                {
                    static const Unequal<ValueType> op{};
                    return op(x(), rhs.x()) || op(y(), rhs.y());
                }

            private:
                std::array<ValueType, 2_uz> _values;
            };

            template<RealNumber T>
            class Vector<3_uz, T>
            {
            public:
                using ValueType = OriginType<T>;

            public:
                constexpr Vector(std::array<ValueType, 3_uz> values)
                    : _values(std::move(values)) {}

                constexpr Vector(ArgCLRefType<ValueType> x, ArgCLRefType<ValueType> y, ArgCLRefType<ValueType> z)
                    : _values(x, y, z) {}

                template<typename = void>
                    requires ReferenceFaster<ValueType> && std::movable<ValueType>
                constexpr Vector(ArgRRefType<ValueType> x, ArgRRefType<ValueType> y, ArgRRefType<ValueType> z)
                    : _values(move<ValueType>(x), move<ValueType>(y), move<ValueType>(z)) {}

            public:
                constexpr Vector(const Vector& ano) = default;
                constexpr Vector(Vector&& ano) noexcept = default;
                constexpr Vector& operator=(const Vector& rhs) = default;
                constexpr Vector& operator=(Vector&& rhs) noexcept = default;
                constexpr ~Vector(void) noexcept = default;

            public:
                inline constexpr ArgCLRefType<ValueType> x(void) const noexcept
                {
                    return _values[0_uz];
                }

                inline constexpr ArgCLRefType<ValueType> y(void) const noexcept
                {
                    return _values[1_uz];
                }

                inline constexpr ArgCLRefType<ValueType> z(void) const noexcept
                {
                    return _values[2_uz];
                }

            public:
                inline constexpr RetType<ValueType> norm(void) const noexcept
                {
                    return **sqrt(sqr(x()) + sqr(y()));
                }

                inline constexpr Vector unit(void) const noexcept
                {
                    const auto this_norm = this->norm();
                    return Vector
                    {
                        x() / this_norm,
                        y() / this_norm,
                        z() / this_norm
                    };
                }

            public:
                inline constexpr ArgCLRefType<ValueType> operator[](const usize i) const noexcept
                {
                    return _values[i];
                }

            public:
                inline constexpr Vector operator+(const Vector& rhs) const noexcept
                {
                    return Vector
                    {
                        x() + rhs.x(),
                        y() + rhs.y(),
                        z() + rhs.z()
                    };
                }

                inline constexpr Vector operator-(const Vector& rhs) const noexcept
                {
                    return Vector
                    {
                        x() - rhs.x(),
                        y() - rhs.y(),
                        z() - rhs.z()
                    };
                }

            public:
                inline constexpr const bool operator==(const Vector& rhs) const noexcept
                {
                    static const Equal<ValueType> op{};
                    return op(x(), rhs.x()) && op(y(), rhs.y()) && op(z(), rhs.z());
                }

                inline constexpr const bool operator!=(const Vector& rhs) const noexcept
                {
                    static const Unequal<ValueType> op{};
                    return op(x(), rhs.x()) || op(y(), rhs.y()) || op(z(), rhs.z());
                }

            private:
                std::array<ValueType, 3_uz> _values;
            };

            template<RealNumber T = f64>
            using Vector2 = Vector<2_uz, f64>;

            template<RealNumber T = f64>
            using Vector3 = Vector<3_uz, f64>;

            extern template class Vector<2_uz, f64>;
            extern template class Vector<3_uz, f64>;
        };
    };
};

namespace ospf
{
    template<usize dim, RealNumber T>
    struct ArithmeticTrait<Vector<dim, T>>
    {
        inline static const Vector<dim, T>& zero(void) noexcept
        {
            static const Vector<dim, T> value{ make_array<T, dim>([](const usize i) { return ArithmeticTrait<T>::zero(); }) };
            return value;
        }

        inline static const Vector<dim, T>& one(void) noexcept
        {
            static const Vector<dim, T> value{ make_array<T, dim>([](const usize i) { return ArithmeticTrait<T>::one(); }) };
            return value;
        }
    };

    template<RealNumber T>
    struct ArithmeticTrait<Vector<2_uz, T>>
    {
        inline static const Vector<2_uz, T>& zero(void) noexcept
        {
            static const Vector<2_uz, T> value{ ArithmeticTrait<T>::zero(), ArithmeticTrait<T>::zero() };
            return value;
        }

        inline static Vector<2_uz, T> one(void) noexcept
        {
            static const Vector<2_uz, T> value{ ArithmeticTrait<T>::one(), ArithmeticTrait<T>::one() };
            return value;
        }
    };

    template<RealNumber T>
    struct ArithmeticTrait<Vector<3_uz, T>>
    {
        inline static const Vector<3_uz, T>& zero(void) noexcept
        {
            static const Vector<3_uz, T> value{ ArithmeticTrait<T>::zero(), ArithmeticTrait<T>::zero(), ArithmeticTrait<T>::zero() };
            return value;
        }

        inline static const Vector<3_uz, T>& one(void) noexcept
        {
            static const Vector<3_uz, T> value{ ArithmeticTrait<T>::one(), ArithmeticTrait<T>::one(), ArithmeticTrait<T>::one() };
            return value;
        }
    };
};

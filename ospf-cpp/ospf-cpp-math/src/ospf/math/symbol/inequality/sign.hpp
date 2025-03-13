#pragma once

#include <ospf/concepts/enum.hpp>
#include <ospf/math/algebra/operator/comparison/less.hpp>
#include <ospf/math/algebra/operator/comparison/less_equal.hpp>
#include <ospf/math/algebra/operator/comparison/greater.hpp>
#include <ospf/math/algebra/operator/comparison/greater_equal.hpp>
#include <ospf/math/algebra/operator/comparison/equal.hpp>
#include <ospf/math/algebra/operator/comparison/unequal.hpp>
#include <ospf/math/functional/predicate.hpp>

namespace ospf
{
    inline namespace math
    {
        inline namespace symbol
        {
            inline namespace inequality
            {
                enum class InequalitySign : u8
                {
                    Less,
                    LessEqual,
                    Greater,
                    GreaterEqual,
                    Equal,
                    Unequal
                };

                template<Invariant T = f64>
                    requires WithoutPrecision<T>
                inline Comparer<OriginType<T>> comparison_operator_of(const InequalitySign sign, ArgCLRefType<T> precision = ArithmeticTrait<T>::zero()) noexcept
                {
                    switch (sign)
                    {
                    case ospf::InequalitySign::Less:
                        return Less<T>{ precision };
                    case ospf::InequalitySign::LessEqual:
                        return LessEqual<T>{ precision };
                    case ospf::InequalitySign::Greater:
                        return Greater<T>{ precision };
                    case ospf::InequalitySign::GreaterEqual:
                        return GreaterEqual<T>{ precision };
                    case ospf::InequalitySign::Equal:
                        return Equal<T>{ precision };
                    case ospf::InequalitySign::Unequal:
                        return Unequal<T>{ precision };
                    default:
                        break;
                    }
                    assert(false);
                    return Equal<T>{ precision };
                }

                template<Invariant T = f64>
                    requires WithPrecision<T>
                inline Comparer<OriginType<T>> comparison_operator_of(const InequalitySign sign, ArgCLRefType<T> precision = PrecisionTrait<T>::decimal_precision()) noexcept
                {
                    switch (sign)
                    {
                    case ospf::InequalitySign::Less:
                        return Less<T>{ precision };
                    case ospf::InequalitySign::LessEqual:
                        return LessEqual<T>{ precision };
                    case ospf::InequalitySign::Greater:
                        return Greater<T>{ precision };
                    case ospf::InequalitySign::GreaterEqual:
                        return GreaterEqual<T>{ precision };
                    case ospf::InequalitySign::Equal:
                        return Equal<T>{ precision };
                    case ospf::InequalitySign::Unequal:
                        return Unequal<T>{ precision };
                    default:
                        break;
                    }
                    assert(false);
                    return Equal<T>{ precision };
                }

                template<Invariant T = f64>
                    requires ReferenceFaster<T> && std::movable<T>
                inline Comparer<OriginType<T>> comparison_operator_of(const InequalitySign sign, ArgRRefType<T> precision) noexcept
                {
                    switch (sign)
                    {
                    case ospf::InequalitySign::Less:
                        return Less<T>{ move<T>(precision) };
                    case ospf::InequalitySign::LessEqual:
                        return LessEqual<T>{ move<T>(precision) };
                    case ospf::InequalitySign::Greater:
                        return Greater<T>{ move<T>(precision) };
                    case ospf::InequalitySign::GreaterEqual:
                        return GreaterEqual<T>{ move<T>(precision) };
                    case ospf::InequalitySign::Equal:
                        return Equal<T>{ move<T>(precision) };
                    case ospf::InequalitySign::Unequal:
                        return Unequal<T>{ move<T>(precision) };
                    default:
                        break;
                    }
                    assert(false);
                    return Equal<T>{ move<T>(precision) };
                }
            };
        };
    };
};

namespace magic_enum::customize
{
    template <>
    constexpr customize_t enum_name<ospf::InequalitySign>(const ospf::InequalitySign sign) noexcept
    {
        switch (sign) {
        case ospf::InequalitySign::Less:
            return "<";
        case ospf::InequalitySign::LessEqual:
            return "<=";
        case ospf::InequalitySign::Greater:
            return ">";
        case ospf::InequalitySign::GreaterEqual:
            return ">=";
        case ospf::InequalitySign::Equal:
            return "=";
        case ospf::InequalitySign::Unequal:
            return "!=";
        default:
            return default_tag;
        }
    }
};

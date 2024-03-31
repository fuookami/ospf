#pragma once

#include <ospf/math/symbol/symbol.hpp>
#include <ospf/math/algebra/operator/comparison/equal.hpp>
#include <ospf/math/algebra/operator/arithmetic/neg.hpp>

namespace ospf
{
    inline namespace math
    {
        inline namespace symbol
        {
            template<Invariant T, Invariant ST, ExpressionCategory cat, typename Cell>
                requires ExpressionTypeOf<Cell, T, ST, cat>
            class Monomial
                : public Expression<T, ST, cat, Monomial<T, ST, cat, Cell>>
            {
                using Impl = Expression<T, ST, cat, Monomial>;

            public:
                using ValueType = OriginType<T>;
                using SymbolValueType = OriginType<ST>;
                using CellType = OriginType<Cell>;
                using PureSymbolType = typename CellType::PureSymbolType;
                using ExprSymbolType = typename CellType::ExprSymbolType;

            public:
                constexpr Monomial(CellType cell)
                    : _coefficient(ArithmeticTrait<ValueType>::one()), _cell(std::move(cell)) {}

                template<typename... Args>
                    requires std::constructible_from<CellType, Args...>
                constexpr Monomial(Args&&... args)
                    : _coefficient(ArithmeticTrait<ValueType>::one()), _cell({ std::forward<Args>(args)... }) {}

                constexpr Monomial(ArgCLRefType<ValueType> coefficient, CellType cell)
                    : _coefficient(coefficient), _cell(std::move(cell)) {}

                template<typename... Args>
                    requires std::constructible_from<CellType, Args...>
                constexpr Monomial(ArgCLRefType<ValueType> coefficient, Args&&... args)
                    : _coefficient(coefficient), _cell({ std::forward<Args>(args)... }) {}

                template<typename = void>
                    requires ReferenceFaster<ValueType> && std::movable<ValueType>
                constexpr Monomial(ArgRRefType<ValueType> coefficient, CellType cell)
                    : _coefficient(move<ValueType>(coefficient)), _cell(std::move(cell)) {}

                template<typename... Args>
                    requires std::constructible_from<CellType, Args...> && ReferenceFaster<ValueType> && std::movable<ValueType>
                constexpr Monomial(ArgRRefType<ValueType> coefficient, Args&&... args)
                    : _coefficient(move<ValueType>(coefficient)), _cell({ std::forward<Args>(args)... }) {}

            public:
                constexpr Monomial(const Monomial& ano) = default;
                constexpr Monomial(Monomial&& ano) noexcept = default;
                constexpr Monomial& operator=(const Monomial& rhs) = default;
                constexpr Monomial& operator=(Monomial&& rhs) noexcept = default;
                constexpr ~Monomial(void) noexcept = default;

            public:
                template<Invariant V>
                    requires DecayNotSameAs<ValueType, V> && std::convertible_to<ValueType, OriginType<V>>
                inline constexpr operator Monomial<OriginType<V>, SymbolValueType, cat, CellType>(void) const & noexcept
                {
                    using ToValueType = OriginType<V>;
                    return Monomial<ToValueType, SymbolValueType, cat, CellType>{ static_cast<ToValueType>(_coefficient), _cell };
                }

                template<Invariant V>
                    requires DecayNotSameAs<ValueType, V> && std::convertible_to<ValueType, OriginType<V>>
                inline constexpr operator Monomial<V, SymbolValueType, cat, CellType>(void) && noexcept
                {
                    using ToValueType = OriginType<V>;
                    return Monomial<ToValueType, SymbolValueType, cat, CellType>{ static_cast<ToValueType>(_coefficient), move<CellType>(_cell) };
                }

            public:
                inline constexpr ArgCLRefType<ValueType> coefficient(void) const noexcept
                {
                    return _coefficient;
                }

                inline constexpr CLRefType<CellType> cell(void) const & noexcept
                {
                    return _cell;
                }

                inline constexpr CellType cell(void) && noexcept
                {
                    return move<CellType>(_cell);
                }

            public:
                inline constexpr Monomial operator-(void) const & noexcept
                {
                    return Monomial{ -_coefficient, _cell };
                }

                inline constexpr Monomial operator-(void) && noexcept
                {
                    return Monomial{ -_coefficient, move<CellType>(_cell) };
                }

            public:
                inline constexpr Monomial& operator*=(ArgCLRefType<ValueType> rhs) noexcept
                {
                    _coefficient *= rhs;
                    return *this;
                }

                inline constexpr Monomial& operator/=(ArgCLRefType<ValueType> rhs) noexcept
                {
                    _coefficient /= rhs;
                    return *this;
                }

                template<typename = void>
                    requires requires (CellType& lhs, const CellType& rhs) { lhs *= rhs; }
                inline constexpr Monomial& operator*=(const Monomial& rhs)
                {
                    _coefficient *= rhs._coefficient;
                    _cell *= rhs._cell;
                    return *this;
                }

            OSPF_CRTP_PERMISSION:
                inline constexpr RetType<ValueType> OSPF_CRTP_FUNCTION(get_value_by)(const std::function<Result<SymbolValueType>(const std::string_view)>& values) const noexcept
                {
                    OSPF_TRY_GET(symbol_value, _cell.value(values));
                    return _coefficient * symbol_value;
                }

            private:
                ValueType _coefficient;
                CellType _cell;
            };

            template<typename T>
            concept MonomialType = ExpressionType<T>
                && ExpressionType<typename T::CellType>
                && requires (const T& monomial)
                {
                    { monomial.coefficient() } -> DecaySameAs<typename T::ValueType>;
                    { monomial.cell() } -> DecaySameAs<typename T::CellType>;
                };

            template<typename... Ts>
            struct IsAllMonomialType;

            template<typename T>
            struct IsAllMonomialType<T>
            {
                static constexpr const bool value = MonomialType<T>;
            };

            template<typename T, typename... Ts>
            struct IsAllMonomialType<T, Ts...>
            {
                static constexpr const bool value = MonomialType<T> && IsAllMonomialType<Ts...>::value;
            };

            template<typename... Ts>
            static constexpr const bool is_all_monomial_type = IsAllMonomialType<Ts...>::value;

            template<typename... Ts>
            concept AllMonomialType = is_all_monomial_type<Ts...>;

            template<typename T, typename V, typename SV, ExpressionCategory cat>
            concept MonomialTypeOf = ExpressionTypeOf<T, V, SV, cat>
                && ExpressionTypeOf<typename T::CellType, V, SV, cat>
                && requires (const T& monomial)
                {
                    { monomial.coefficient() } -> DecaySameAs<typename T::ValueType>;
                    { monomial.cell() } -> DecaySameAs<typename T::CellType>;
                };

            template<typename V, typename SV, ExpressionCategory cat, typename... Ts>
            struct IsAllMonomialTypeOf;

            template<typename V, typename SV, ExpressionCategory cat, typename T>
            struct IsAllMonomialTypeOf<V, SV, cat, T>
            {
                static constexpr const bool value = MonomialTypeOf<V, SV, cat, T>
            };

            template<typename V, typename SV, ExpressionCategory cat, typename T, typename... Ts>
            struct IsAllMonomialTypeOf<V, SV, cat, T, Ts...>
            {
                static constexpr const bool value = MonomialTypeOf<V, SV, cat, T> && IsAllMonomialTypeOf<V, SV, cat, Ts...>::value;
            };

            template<typename V, typename SV, ExpressionCategory cat, typename... Ts>
            static constexpr const bool is_all_monomial_type_of = IsAllMonomialTypeOf<V, SV, cat, Ts...>::value;

            template<typename V, typename SV, ExpressionCategory cat, typename... Ts>
            concept AllMonomialTypeOf = is_all_monomial_type_of<V, SV, cat, Ts...>;
        };
    };
};

namespace std
{
    template<ospf::Invariant T, ospf::Invariant ST, ospf::ExpressionCategory cat, typename Cell>
    struct formatter<ospf::Monomial<T, ST, cat, Cell>>
        : public formatter<string_view, char>
    {
        template<typename FormatContext>
        inline decltype(auto) format(const ospf::Monomial<T, ST, cat, Cell>& monomial, FormatContext& fc) const
        {
            static const auto _formatter = formatter<string_view, char>{};
            static const ospf::Equal<T> eq{};
            if (eq(monomial.coefficient(), ospf::ArithmeticTrait<T>::zero()))
            {
                return _formatter.format("0", fc);
            }
            else if (eq(monomial.coefficient(), ospf::ArithmeticTrait<T>::one()))
            {
                return _formatter.format(std::format("{}", monomial.cell()), fc);
            }
            else
            {
                if constexpr (ospf::Signed<T> && ospf::Neg<T>)
                {
                    if (eq(monomial.coefficient(), -ospf::ArithmeticTrait<T>::one()))
                    {
                        return _formatter.format(std::format("-{}", monomial.cell()), fc);
                    }
                    else
                    {
                        return _formatter.format(std::format("{} * {}", monomial.coefficient(), monomial.cell()), fc);
                    }
                }
                else
                {
                    return _formatter.format(std::format("{} * {}", monomial.coefficient(), monomial.cell()), fc);
                }
            }
        }
    };

    template<ospf::Invariant T, ospf::Invariant ST, ospf::ExpressionCategory cat, typename Cell>
    struct formatter<ospf::Monomial<T, ST, cat, Cell>>
        : public formatter<wstring_view, ospf::wchar>
    {
        template<typename FormatContext>
        inline decltype(auto) format(const ospf::Monomial<T, ST, cat, Cell>& monomial, FormatContext& fc) const
        {
            static const auto _formatter = formatter<wstring_view, ospf::wchar>{};
            static const ospf::Equal<T> eq{};
            if (eq(monomial.coefficient(), ospf::ArithmeticTrait<T>::zero()))
            {
                return _formatter.format(L"0", fc);
            }
            else if (eq(monomial.coefficient(), ospf::ArithmeticTrait<T>::one()))
            {
                return _formatter.format(std::format(L"{}", monomial.cell()), fc);
            }
            else
            {
                if constexpr (ospf::Signed<T> && ospf::Neg<T>)
                {
                    if (eq(monomial.coefficient(), -ospf::ArithmeticTrait<T>::one()))
                    {
                        return _formatter.format(std::format(L"-{}", monomial.cell()), fc);
                    }
                    else
                    {
                        return _formatter.format(std::format(L"{} * {}", monomial.coefficient(), monomial.cell()), fc);
                    }
                }
                else
                {
                    return _formatter.format(std::format(L"{} * {}", monomial.coefficient(), monomial.cell()), fc);
                }
            }
        }
    };
};

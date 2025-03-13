#pragma once

#include <ospf/math/algebra/operator/arithmetic/abs.hpp>
#include <ospf/math/symbol/monomial/concepts.hpp>
#include <span>

namespace ospf
{
    inline namespace math
    {
        inline namespace symbol
        {
            template<Invariant T, Invariant ST, ExpressionCategory cat, typename Mono>
                requires MonomialTypeOf<Mono, T, ST, cat>
            class Polynomial
                : public Expression<T, ST, cat, Polynomial<T, ST, cat, Mono>>
            {
                using Impl = Expression<T, ST, cat, Polynomial>;

            public:
                using ValueType = OriginType<T>;
                using SymbolValueType = OriginType<ST>;
                using MonomialType = OriginType<Mono>;

            public:
                constexpr Polynomial(ArgCLRefType<ValueType> constant = ArithmeticTrait<ValueType>::zero())
                    : _constant(constant) {}

                template<typename = void>
                    requires ReferenceFaster<ValueType> && std::movable<ValueType>
                constexpr Polynomial(ArgRRefType<ValueType> constant)
                    : _constant(move<ValueType>(constant)) {}

                constexpr Polynomial(std::vector<MonomialType> monomials, ArgCLRefType<ValueType> constant = ArithmeticTrait<ValueType>::zero())
                    : _monos(std::move(monomials)), _constant(constant) {}

                template<typename = void>
                    requires ReferenceFaster<ValueType> && std::movable<ValueType>
                constexpr Polynomial(std::vector<MonomialType> monomials, ArgRRefType<ValueType> constant)
                    : _monos(std::move(monomials)), _constant(move<ValueType>(constant)) {}

            public:
                constexpr Polynomial(const Polynomial& ano) = default;
                constexpr Polynomial(Polynomial&& ano) noexcept = default;
                constexpr Polynomial& operator=(const Polynomial& rhs) = default;
                constexpr Polynomial& operator=(Polynomial&& rhs) noexcept = default;
                constexpr ~Polynomial(void) noexcept = default;

            public:
                template<Invariant V>
                    requires DecayNotSameAs<ValueType, V>&& std::convertible_to<ValueType, OriginType<V>>
                inline constexpr operator Polynomial<OriginType<V>, SymbolValueType, cat, MonomialType>(void) const & noexcept
                {
                    using ToValueType = OriginType<V>;
                    using ToMonomialType = Monomial<ToValueType, SymbolValueType, cat, typename MonomialType::CellType>;
                    return Polynomial<ToValueType, SymbolValueType, cat, ToMonomialType>{ std::vector<ToMonomialType>
                    {
                        boost::make_transform_iterator(_monos.cbegin(), [](const auto& mono) -> ToMonomialType { return static_cast<ToMonomialType>(mono); }),
                        boost::make_transform_iterator(_monos.cend(), [](const auto& mono) -> ToMonomialType { return static_cast<ToMonomialType>(mono); })
                    }, static_cast<ToValueType>(_constant) };
                }

                template<Invariant V>
                    requires DecayNotSameAs<ValueType, V>&& std::convertible_to<ValueType, OriginType<V>>
                inline constexpr operator Polynomial<OriginType<V>, SymbolValueType, cat, MonomialType>(void) && noexcept
                {
                    using ToValueType = OriginType<V>;
                    using ToMonomialType = Monomial<ToValueType, SymbolValueType, cat, typename MonomialType::CellType>;
                    return Polynomial<ToValueType, SymbolValueType, cat, ToMonomialType>{ std::vector<ToMonomialType>
                    {
                        boost::make_transform_iterator(_monos.begin(), [](auto& mono) -> ToMonomialType { return static_cast<ToMonomialType>(move<MonomialType>(mono)); }),
                        boost::make_transform_iterator(_monos.end(), [](auto& mono) -> ToMonomialType { return static_cast<ToMonomialType>(move<MonomialType>(mono)); })
                    }, static_cast<ToValueType>(_constant) };
                }

            public:
                inline constexpr const std::span<const MonomialType> monomials(void) const noexcept
                {
                    return _monos;
                }

                inline constexpr ArgCLRefType<ValueType> constant(void) const noexcept
                {
                    return _constant;
                }

            public:
                inline constexpr Polynomial operator-(void) const & noexcept
                {
                    return Polynomial{ std::vector<MonomialType>
                    {
                        boost::make_transform_iterator(_monos.cbegin(), [](const auto& mono) -> MonomialType { return -mono; }),
                        boost::make_transform_iterator(_monos.cend(), [](const auto& mono) -> MonomialType { return -mono; })
                    }, -_constant };
                }

                inline constexpr Polynomial operator-(void) && noexcept
                {
                    return Polynomial{ std::vector<MonomialType>
                    {
                        boost::make_transform_iterator(_monos.begin(), [](auto& mono) -> MonomialType { return -move<MonomialType>(mono); }),
                        boost::make_transform_iterator(_monos.end(), [](auto& mono) -> MonomialType { return -move<MonomialType>(mono); })
                    }, -_constant };
                }

            public:
                inline constexpr Polynomial& operator+=(ArgCLRefType<ValueType> rhs) noexcept
                {
                    _constant += rhs;
                    return *this;
                }

                inline constexpr Polynomial& operator-=(ArgCLRefType<ValueType> rhs) noexcept
                {
                    _constant -= rhs;
                    return *this;
                }

                inline constexpr Polynomial& operator*=(ArgCLRefType<ValueType> rhs) noexcept
                {
                    for (auto& mono : _monos)
                    {
                        mono *= rhs;
                    }
                    _constant *= rhs;
                    return *this;
                }

                inline constexpr Polynomial& operator/=(ArgCLRefType<ValueType> rhs) noexcept
                {
                    for (auto& mono : _monos)
                    {
                        mono /= rhs;
                    }
                    _constant /= rhs;
                    return *this;
                }

                inline constexpr Polynomial& operator+=(ArgCLRefType<MonomialType> rhs) noexcept
                {
                    _monos.push_back(rhs);
                    return *this;
                }

                template<typename = void>
                    requires ReferenceFaster<MonomialType> && std::movable<MonomialType>
                inline constexpr Polynomial& operator+=(ArgRRefType<MonomialType> rhs) noexcept
                {
                    _monos.push_back(move<MonomialType>(rhs));
                    return *this;
                }

                inline constexpr Polynomial& operator-=(ArgCLRefType<MonomialType> rhs) noexcept
                {
                    _monos.push_back(-rhs);
                    return *this;
                }

                template<typename = void>
                    requires ReferenceFaster<MonomialType> && std::movable<MonomialType>
                inline constexpr Polynomial& operator-=(ArgRRefType<MonomialType> rhs) noexcept
                {
                    _monos.push_back(-move<MonomialType>(rhs));
                    return *this;
                }

                template<typename = void>
                    requires requires (MonomialType& lhs, const MonomialType& rhs) { lhs *= rhs; }
                inline constexpr Polynomial& operator*=(ArgCLRefType<MonomialType> rhs)
                {
                    for (auto& mono : _monos)
                    {
                        mono *= rhs;
                    }
                    _monos.push_back(_constant * rhs);
                    _constant = ArithmeticTrait<ValueType>::zero();
                    return *this;
                }

            OSPF_CRTP_PERMISSION:
                inline constexpr RetType<ValueType> OSPF_CRTP_FUNCTION(get_value_by)(const std::function<Result<SymbolValueType>(const std::string_view)>& values) const noexcept
                {
                    auto ret = _constant;
                    for (const auto& mono : _monos)
                    {
                        OSPF_TRY_GET(value, mono.value(values));
                        ret += value;
                    }
                    return ret;
                }

            private:
                std::vector<MonomialType> _monos;
                ValueType _constant;
            };

            template<typename T>
            concept PolynomialType = ExpressionType<T>
                && MonomialType<typename T::MonomialType>
                && requires (const T& polynomial)
                {
                    { polynomial.monomials() } -> std::ranges::output_range<typename T::MonomialType>;
                    { polynomial.constant() } -> DecaySameAs<typename T::ValueType>;
                };

            template<typename... Ts>
            struct IsAllPolynomialType;

            template<typename T>
            struct IsAllPolynomialType<T>
            {
                static constexpr const bool value = PolynomialType<T>;
            };

            template<typename T, typename... Ts>
            struct IsAllPolynomialType<T, Ts...>
            {
                static constexpr const bool value = PolynomialType<T> && IsAllPolynomialType<Ts...>;
            };

            template<typename V, typename SV, ExpressionCategory cat, typename T>
            concept PolynomialTypeOf = ExpressionTypeOf<T, V, SV, cat>
                && MonomialTypeOf<typename T::MonomialType, V, SV, cat>
                && requires (const T& polynomial)
                {
                    { polynomial.monomials() } -> std::ranges::output_range<typename T::MonomialType>;
                    { polynomial.constant() } -> DecaySameAs<typename T::ValueType>;
                };

            template<typename V, typename SV, ExpressionCategory cat, typename... Ts>
            struct IsAllPolynomialTypeOf;

            template<typename V, typename SV, ExpressionCategory cat, typename T>
            struct IsAllPolynomialTypeOf<V, SV, cat, T>
            {
                static constexpr const bool value = PolynomialTypeOf<T, V, SV, cat>;
            };

            template<typename V, typename SV, ExpressionCategory cat, typename T, typename... Ts>
            struct IsAllPolynomialTypeOf<V, SV, cat, T, Ts...>
            {
                static constexpr const bool value = PolynomialTypeOf<T, V, SV, cat> && IsAllPolynomialTypeOf<V, SV, cat, Ts...>::value;
            };

            template<typename V, typename SV, ExpressionCategory cat, typename... Ts>
            static constexpr const bool is_all_polynomial_type_of = IsAllPolynomialTypeOf<V, SV, cat, Ts...>;

            template<typename V, typename SV, ExpressionCategory cat, typename... Ts>
            concept AllPolynomialTypeOf = is_all_polynomial_type_of<V, SV, cat, Ts...>;
        };
    };
};

namespace std
{
    template<ospf::Invariant T, ospf::Invariant ST, ospf::ExpressionCategory cat, typename Mono>
    struct formatter<ospf::Polynomial<T, ST, cat, Mono>>
        : public formatter<string_view, char>
    {
        template<typename FormatContext>
        inline decltype(auto) format(const ospf::Polynomial<T, ST, cat, Mono>& polynomial, FormatContext& fc) const
        {
            std::ostringstream sout;

            static const ospf::Equal<T> eq{};
            auto j{ 0_uz };
            for (ospf::usize i{ 0_uz }; i != polynomial.monomials(); ++i)
            {
                const auto& monomial = polynomial.monomials()[i];
                if (eq(monomial.coefficient(), ospf::ArithmeticTrait<T>::zero()))
                {
                    continue;
                }

                if (j == 0_uz)
                {
                    sout << std::format("{}", monomial);
                }
                else
                {
                    if constexpr (ospf::Signed<T> && ospf::Abs<T>)
                    {
                        if (ospf::is_positive(monomial.coefficient()))
                        {
                            sout << " + ";
                        }
                        else
                        {
                            sout << " - ";
                        }
                        const auto coefficient = abs(monomial.coefficient());
                        if (eq(coefficient, ospf::ArithmeticTrait<T>::one()))
                        {
                            sout << std::format("{}", monomial.cell());
                        }
                        else
                        {
                            sout << std::format("{} * {}", coefficient, monomial.cell());
                        }
                    }
                    else
                    {
                        sout << std::format(" + {}", monomial);
                    }
                }
                ++j;
            }

            if (eq(polynomial.constant(), ospf::ArithmeticTrait<T>::zero()) && j == 0_uz)
            {
                sout << "0";
            }
            else if (!eq(polynomial.constant(), ospf::ArithmeticTrait<T>::zero()))
            {
                if (j == 0_uz)
                {
                    sout << polynomial.constant();
                }
                else
                {
                    if constexpr (ospf::Signed<T> && ospf::Abs<T>)
                    {
                        if (ospf::is_positive(monomial.coefficient()))
                        {
                            sout << " + ";
                        }
                        else
                        {
                            sout << " - ";
                        }
                        sout << abs(polynomial.constant());
                    }
                    else
                    {
                        sout << " + " << polynomial.constant();
                    }
                }
            }

            static const auto _formatter = formatter<string_view, char>{};
            return _formatter.format(sout.str(), fc);
        }
    };

    template<ospf::Invariant T, ospf::Invariant ST, ospf::ExpressionCategory cat, typename Mono>
    struct formatter<ospf::Polynomial<T, ST, cat, Mono>>
        : public formatter<wstring_view, ospf::wchar>
    {
        template<typename FormatContext>
        inline decltype(auto) format(const ospf::Polynomial<T, ST, cat, Mono>& polynomial, FormatContext& fc) const
        {
            std::wostringstream sout;

            static const ospf::Equal<T> eq{};
            auto j{ 0_uz };
            for (ospf::usize i{ 0_uz }; i != polynomial.monomials(); ++i)
            {
                const auto& monomial = polynomial.monomials()[i];
                if (eq(monomial.coefficient(), ospf::ArithmeticTrait<T>::zero()))
                {
                    continue;
                }

                if (j == 0_uz)
                {
                    sout << std::format(L"{}", monomial);
                }
                else
                {
                    if constexpr (ospf::Signed<T> && ospf::Abs<T>)
                    {
                        if (ospf::is_positive(monomial.coefficient()))
                        {
                            sout << L" + ";
                        }
                        else
                        {
                            sout << L" - ";
                        }
                        const auto coefficient = abs(monomial.coefficient());
                        if (eq(coefficient, ospf::ArithmeticTrait<T>::one()))
                        {
                            sout << std::format(L"{}", monomial.cell());
                        }
                        else
                        {
                            sout << std::format(L"{} * {}", coefficient, monomial.cell());
                        }
                    }
                    else
                    {
                        sout << std::format(L" + {}", monomial);
                    }
                }
                ++j;
            }

            if (eq(polynomial.constant(), ospf::ArithmeticTrait<T>::zero()) && j == 0_uz)
            {
                sout << L"0";
            }
            else if (!eq(polynomial.constant(), ospf::ArithmeticTrait<T>::zero()))
            {
                if (j == 0_uz)
                {
                    sout << polynomial.constant();
                }
                else
                {
                    if constexpr (ospf::Signed<T> && ospf::Abs<T>)
                    {
                        if (ospf::is_positive(monomial.coefficient()))
                        {
                            sout << L" + ";
                        }
                        else
                        {
                            sout << L" - ";
                        }
                        sout << abs(polynomial.constant());
                    }
                    else
                    {
                        sout << L" + " << polynomial.constant();
                    }
                }
            }

            static const auto _formatter = formatter<wstring_view, ospf::wchar>{};
            return _formatter.format(sout.str(), fc);
        }
    };
};

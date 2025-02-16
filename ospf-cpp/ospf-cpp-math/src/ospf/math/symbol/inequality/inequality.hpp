#pragma once

#include <ospf/math/symbol/expression.hpp>
#include <ospf/math/symbol/inequality/sign.hpp>
#include <ospf/math/symbol/monomial/linear.hpp>
#include <ospf/math/symbol/monomial/quadratic.hpp>
#include <ospf/math/symbol/monomial/standard.hpp>
#include <ospf/math/symbol/polynomial/linear.hpp>
#include <ospf/math/symbol/polynomial/quadratic.hpp>
#include <ospf/math/symbol/polynomial/standard.hpp>

namespace ospf
{
    inline namespace math
    {
        inline namespace symbol
        {
            template<ExpressionType Lhs, ExpressionType Rhs = Lhs>
                requires DecaySameAsOrConvertibleTo<typename Lhs::SymbolValueType, typename Rhs::SymbolValueType>
            class Inequality
                : public Expression<bool, typename Lhs::SymbolValueType, std::max(Lhs::category, Rhs::category), Inequality<Lhs, Rhs>>
            {
            public:
                using LhsType = OriginType<Lhs>;
                using LhsValueType = typename LhsType::ValueType;
                using RhsType = OriginType<Rhs>;
                using RhsValueType = typename RhsType::ValueType;

                using ValueType = bool;
                using SymbolValueType = typename Lhs::SymbolValueType;

            public:
                constexpr Inequality(ArgCLRefType<LhsType> lhs, ArgCLRefType<RhsType> rhs, const InequalitySign sign)
                    : _lhs(lhs), _rhs(rhs), _sign(sign) {}

                template<typename = void>
                    requires ReferenceFaster<LhsType> && std::movable<LhsType>
                constexpr Inequality(ArgRRefType<LhsType> lhs, ArgCLRefType<RhsType> rhs, const InequalitySign sign)
                    : _lhs(move<LhsType>(lhs)), _rhs(rhs), _sign(sign) {}

                template<typename = void>
                    requires ReferenceFaster<RhsType> && std::movable<RhsType>
                constexpr Inequality(ArgCLRefType<LhsType> lhs, ArgRRefType<RhsType> rhs, const InequalitySign sign)
                    : _lhs(lhs), _rhs(move<RhsType>(rhs)), _sign(sign) {}

                template<typename = void>
                    requires ReferenceFaster<LhsType> && std::movable<LhsType> &&
                        ReferenceFaster<RhsType> && std::movable<RhsType>
                constexpr Inequality(ArgRRefType<RhsType> lhs, ArgRRefType<RhsType> rhs, const InequalitySign sign)
                    : _lhs(move<LhsType>(lhs)), _rhs(move<RhsType>(rhs)), _sign(sign) {}

            public:
                constexpr Inequality(const Inequality& ano) = default;
                constexpr Inequality(Inequality&& ano) noexcept = default;
                constexpr Inequality& operator=(const Inequality& rhs) = default;
                constexpr Inequality& operator=(Inequality&& rhs) noexcept = default;
                constexpr ~Inequality(void) noexcept = default;

            public:
                inline constexpr ArgCLRefType<LhsType> lhs(void) const noexcept
                {
                    return _lhs;
                }

                inline constexpr ArgCLRefType<RhsType> rhs(void) const noexcept
                {
                    return _rhs;
                }

                inline constexpr const InequalitySign sign(void) const noexcept
                {
                    return _sign;
                }

                inline constexpr decltype(auto) comparison_operator(void) const noexcept
                {
                    using ValueType = OriginType<decltype(std::declval<LhsValueType>() - std::declval<RhsValueType>())>;
                    return comparison_operator_of<ValueType>(_sign);
                }

            OSPF_CRTP_PERMISSION:
                inline constexpr Result<bool> OSPF_CRTP_FUNCTION(get_value_by)(const std::function<Result<SymbolValueType>(const std::string_view)>& values) const noexcept
                {
                    OSPF_TRY_GET(lhs_value, _lhs.value(values));
                    OSPF_TRY_GET(rhs_value, _rhs.value(values));

                    const auto op = comparison_operator();
                    return op(lhs_value, rhs_value);
                }

            private:
                LhsType _lhs;
                RhsType _rhs;
                InequalitySign _sign;
            };

            template<ExpressionType Lhs, ExpressionType Rhs>
            inline constexpr decltype(auto) operator==(Lhs&& lhs, Rhs&& rhs) noexcept
            {
                return Inequality<OriginType<Lhs>, OriginType<Rhs>>{ std::forward<Lhs>(lhs), std::forward<Rhs>(rhs), InequalitySign::Equal };
            }

            template<ExpressionType Lhs, ExpressionType Rhs>
            inline constexpr decltype(auto) operator!=(Lhs&& lhs, Rhs&& rhs) noexcept
            {
                return Inequality<OriginType<Lhs>, OriginType<Rhs>>{ std::forward<Lhs>(lhs), std::forward<Rhs>(rhs), InequalitySign::Unequal };
            }

            template<ExpressionType Lhs, ExpressionType Rhs>
            inline constexpr decltype(auto) operator<(Lhs&& lhs, Rhs&& rhs) noexcept
            {
                return Inequality<OriginType<Lhs>, OriginType<Rhs>>{ std::forward<Lhs>(lhs), std::forward<Rhs>(rhs), InequalitySign::Less };
            }

            template<ExpressionType Lhs, ExpressionType Rhs>
            inline constexpr decltype(auto) operator<=(Lhs&& lhs, Rhs&& rhs) noexcept
            {
                return Inequality<OriginType<Lhs>, OriginType<Rhs>>{ std::forward<Lhs>(lhs), std::forward<Rhs>(rhs), InequalitySign::LessEqual };
            }

            template<ExpressionType Lhs, ExpressionType Rhs>
            inline constexpr decltype(auto) operator>(Lhs&& lhs, Rhs&& rhs) noexcept
            {
                return Inequality<OriginType<Lhs>, OriginType<Rhs>>{ std::forward<Lhs>(lhs), std::forward<Rhs>(rhs), InequalitySign::Greater };
            }

            template<ExpressionType Lhs, ExpressionType Rhs>
            inline constexpr decltype(auto) operator>=(Lhs&& lhs, Rhs&& rhs) noexcept
            {
                return Inequality<OriginType<Lhs>, OriginType<Rhs>>{ std::forward<Lhs>(lhs), std::forward<Rhs>(rhs), InequalitySign::GreaterEqual };
            }

            namespace linear
            {
                // operators between symbol and symbol
                template<PureSymbolType Lhs, PureSymbolType Rhs>
                inline constexpr decltype(auto) operator==(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using LhsType = LinearMonomial<f64, f64, OriginType<Lhs>>;
                    using RhsType = LinearMonomial<f64, f64, OriginType<Rhs>>;
                    return Inequality<LhsType, RhsType>{ LhsType{ std::forward<Lhs>(lhs) }, RhsType{ std::forward<Rhs>(rhs) }, InequalitySign::Equal };
                }

                template<PureSymbolType Lhs, PureSymbolType Rhs>
                inline constexpr decltype(auto) operator!=(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using LhsType = LinearMonomial<f64, f64, OriginType<Lhs>>;
                    using RhsType = LinearMonomial<f64, f64, OriginType<Rhs>>;
                    return Inequality<LhsType, RhsType>{ LhsType{ std::forward<Lhs>(lhs) }, RhsType{ std::forward<Rhs>(rhs) }, InequalitySign::Unequal };
                }

                template<PureSymbolType Lhs, PureSymbolType Rhs>
                inline constexpr decltype(auto) operator<(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using LhsType = LinearMonomial<f64, f64, OriginType<Lhs>>;
                    using RhsType = LinearMonomial<f64, f64, OriginType<Rhs>>;
                    return Inequality<LhsType, RhsType>{ LhsType{ std::forward<Lhs>(lhs) }, RhsType{ std::forward<Rhs>(rhs) }, InequalitySign::Less };
                }

                template<PureSymbolType Lhs, PureSymbolType Rhs>
                inline constexpr decltype(auto) operator<=(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using LhsType = LinearMonomial<f64, f64, OriginType<Lhs>>;
                    using RhsType = LinearMonomial<f64, f64, OriginType<Rhs>>;
                    return Inequality<LhsType, RhsType>{ LhsType{ std::forward<Lhs>(lhs) }, RhsType{ std::forward<Rhs>(rhs) }, InequalitySign::LessEqual };
                }

                template<PureSymbolType Lhs, PureSymbolType Rhs>
                inline constexpr decltype(auto) operator>(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using LhsType = LinearMonomial<f64, f64, OriginType<Lhs>>;
                    using RhsType = LinearMonomial<f64, f64, OriginType<Rhs>>;
                    return Inequality<LhsType, RhsType>{ LhsType{ std::forward<Lhs>(lhs) }, RhsType{ std::forward<Rhs>(rhs) }, InequalitySign::Greater };
                }

                template<PureSymbolType Lhs, PureSymbolType Rhs>
                inline constexpr decltype(auto) operator>=(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using LhsType = LinearMonomial<f64, f64, OriginType<Lhs>>;
                    using RhsType = LinearMonomial<f64, f64, OriginType<Rhs>>;
                    return Inequality<LhsType, RhsType>{ LhsType{ std::forward<Lhs>(lhs) }, RhsType{ std::forward<Rhs>(rhs) }, InequalitySign::GreaterEqual };
                }

                // operators between value and symbol

                template<Invariant Lhs, PureSymbolType Rhs>
                inline constexpr decltype(auto) operator==(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using LhsType = LinearPolynomial<OriginType<Lhs>>;
                    using RhsType = LinearMonomial<f64, f64, OriginType<Rhs>>;
                    return Inequality<LhsType, RhsType>{ LhsType{ std::forward<Lhs>(lhs) }, RhsType{ std::forward<Rhs>(rhs) }, InequalitySign::Equal };
                }

                template<Invariant Lhs, PureSymbolType Rhs>
                inline constexpr decltype(auto) operator!=(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using LhsType = LinearPolynomial<OriginType<Lhs>>;
                    using RhsType = LinearMonomial<f64, f64, OriginType<Rhs>>;
                    return Inequality<LhsType, RhsType>{ LhsType{ std::forward<Lhs>(lhs) }, RhsType{ std::forward<Rhs>(rhs) }, InequalitySign::Unequal };
                }

                template<Invariant Lhs, PureSymbolType Rhs>
                inline constexpr decltype(auto) operator<(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using LhsType = LinearPolynomial<OriginType<Lhs>>;
                    using RhsType = LinearMonomial<f64, f64, OriginType<Rhs>>;
                    return Inequality<LhsType, RhsType>{ LhsType{ std::forward<Lhs>(lhs) }, RhsType{ std::forward<Rhs>(rhs) }, InequalitySign::Less };
                }

                template<Invariant Lhs, PureSymbolType Rhs>
                inline constexpr decltype(auto) operator<=(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using LhsType = LinearPolynomial<OriginType<Lhs>>;
                    using RhsType = LinearMonomial<f64, f64, OriginType<Rhs>>;
                    return Inequality<LhsType, RhsType>{ LhsType{ std::forward<Lhs>(lhs) }, RhsType{ std::forward<Rhs>(rhs) }, InequalitySign::LessEqual };
                }

                template<Invariant Lhs, PureSymbolType Rhs>
                inline constexpr decltype(auto) operator>(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using LhsType = LinearPolynomial<OriginType<Lhs>>;
                    using RhsType = LinearMonomial<f64, f64, OriginType<Rhs>>;
                    return Inequality<LhsType, RhsType>{ LhsType{ std::forward<Lhs>(lhs) }, RhsType{ std::forward<Rhs>(rhs) }, InequalitySign::Greater };
                }

                template<Invariant Lhs, PureSymbolType Rhs>
                inline constexpr decltype(auto) operator>=(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using LhsType = LinearPolynomial<OriginType<Lhs>>;
                    using RhsType = LinearMonomial<f64, f64, OriginType<Rhs>>;
                    return Inequality<LhsType, RhsType>{ LhsType{ std::forward<Lhs>(lhs) }, RhsType{ std::forward<Rhs>(rhs) }, InequalitySign::GreaterEqual };
                }

                // operators between symbol and value

                template<PureSymbolType Lhs, Invariant Rhs>
                inline constexpr decltype(auto) operator==(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using LhsType = LinearMonomial<f64, f64, OriginType<Lhs>>;
                    using RhsType = LinearPolynomial<OriginType<Rhs>>;
                    return Inequality<LhsType, RhsType>{ LhsType{ std::forward<Lhs>(lhs) }, RhsType{ std::forward<Rhs>(rhs) }, InequalitySign::Equal };
                }

                template<PureSymbolType Lhs, Invariant Rhs>
                inline constexpr decltype(auto) operator!=(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using LhsType = LinearMonomial<f64, f64, OriginType<Lhs>>;
                    using RhsType = LinearPolynomial<OriginType<Rhs>>;
                    return Inequality<LhsType, RhsType>{ LhsType{ std::forward<Lhs>(lhs) }, RhsType{ std::forward<Rhs>(rhs) }, InequalitySign::Unequal };
                }

                template<PureSymbolType Lhs, Invariant Rhs>
                inline constexpr decltype(auto) operator<(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using LhsType = LinearMonomial<f64, f64, OriginType<Lhs>>;
                    using RhsType = LinearPolynomial<OriginType<Rhs>>;
                    return Inequality<LhsType, RhsType>{ LhsType{ std::forward<Lhs>(lhs) }, RhsType{ std::forward<Rhs>(rhs) }, InequalitySign::Less };
                }

                template<PureSymbolType Lhs, Invariant Rhs>
                inline constexpr decltype(auto) operator<=(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using LhsType = LinearMonomial<f64, f64, OriginType<Lhs>>;
                    using RhsType = LinearPolynomial<OriginType<Rhs>>;
                    return Inequality<LhsType, RhsType>{ LhsType{ std::forward<Lhs>(lhs) }, RhsType{ std::forward<Rhs>(rhs) }, InequalitySign::LessEqual };
                }

                template<PureSymbolType Lhs, Invariant Rhs>
                inline constexpr decltype(auto) operator<=(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using LhsType = LinearMonomial<f64, f64, OriginType<Lhs>>;
                    using RhsType = LinearPolynomial<OriginType<Rhs>>;
                    return Inequality<LhsType, RhsType>{ LhsType{ std::forward<Lhs>(lhs) }, RhsType{ std::forward<Rhs>(rhs) }, InequalitySign::Greater };
                }

                template<PureSymbolType Lhs, Invariant Rhs>
                inline constexpr decltype(auto) operator<=(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using LhsType = LinearMonomial<f64, f64, OriginType<Lhs>>;
                    using RhsType = LinearPolynomial<OriginType<Rhs>>;
                    return Inequality<LhsType, RhsType>{ LhsType{ std::forward<Lhs>(lhs) }, RhsType{ std::forward<Rhs>(rhs) }, InequalitySign::GreaterEqual };
                }

                // operators between value and expression

                template<Invariant Lhs, ExpressionType Rhs>
                    requires (Rhs::category == ExpressionCategory::Linear)
                inline constexpr decltype(auto) operator==(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using LhsType = LinearPolynomial<OriginType<Lhs>>;
                    return Inequality<LhsType, OriginType<Rhs>>{ LhsType{ std::forward<Lhs>(lhs) }, std::forward<Rhs>(rhs), InequalitySign::Equal };
                }

                template<Invariant Lhs, ExpressionType Rhs>
                    requires (Rhs::category == ExpressionCategory::Linear)
                inline constexpr decltype(auto) operator!=(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using LhsType = LinearPolynomial<OriginType<Lhs>>;
                    return Inequality<LhsType, OriginType<Rhs>>{ LhsType{ std::forward<Lhs>(lhs) }, std::forward<Rhs>(rhs), InequalitySign::Unequal };
                }

                template<Invariant Lhs, ExpressionType Rhs>
                    requires (Rhs::category == ExpressionCategory::Linear)
                inline constexpr decltype(auto) operator<(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using LhsType = LinearPolynomial<OriginType<Lhs>>;
                    return Inequality<LhsType, OriginType<Rhs>>{ LhsType{ std::forward<Lhs>(lhs) }, std::forward<Rhs>(rhs), InequalitySign::Less };
                }

                template<Invariant Lhs, ExpressionType Rhs>
                    requires (Rhs::category == ExpressionCategory::Linear)
                inline constexpr decltype(auto) operator<=(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using LhsType = LinearPolynomial<OriginType<Lhs>>;
                    return Inequality<LhsType, OriginType<Rhs>>{ LhsType{ std::forward<Lhs>(lhs) }, std::forward<Rhs>(rhs), InequalitySign::LessEqual };
                }

                template<Invariant Lhs, ExpressionType Rhs>
                    requires (Rhs::category == ExpressionCategory::Linear)
                inline constexpr decltype(auto) operator>(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using LhsType = LinearPolynomial<OriginType<Lhs>>;
                    return Inequality<LhsType, OriginType<Rhs>>{ LhsType{ std::forward<Lhs>(lhs) }, std::forward<Rhs>(rhs), InequalitySign::Greater };
                }

                template<Invariant Lhs, ExpressionType Rhs>
                    requires (Rhs::category == ExpressionCategory::Linear)
                inline constexpr decltype(auto) operator>=(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using LhsType = LinearPolynomial<OriginType<Lhs>>;
                    return Inequality<LhsType, OriginType<Rhs>>{ LhsType{ std::forward<Lhs>(lhs) }, std::forward<Rhs>(rhs), InequalitySign::GreaterEqual };
                }

                // operators between expression and value

                template<ExpressionType Lhs, Invariant Rhs>
                    requires (Lhs::category == ExpressionCategory::Linear)
                inline constexpr decltype(auto) operator==(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using RhsType = LinearPolynomial<OriginType<Rhs>>;
                    return Inequality<OriginType<Lhs>, RhsType>{ std::forward<Lhs>(lhs), RhsType{ std::forward<Rhs>(rhs) }, InequalitySign::Equal };
                }

                template<ExpressionType Lhs, Invariant Rhs>
                    requires (Lhs::category == ExpressionCategory::Linear)
                inline constexpr decltype(auto) operator!=(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using RhsType = LinearPolynomial<OriginType<Rhs>>;
                    return Inequality<OriginType<Lhs>, RhsType>{ std::forward<Lhs>(lhs), RhsType{ std::forward<Rhs>(rhs) }, InequalitySign::Unequal };
                }

                template<ExpressionType Lhs, Invariant Rhs>
                    requires (Lhs::category == ExpressionCategory::Linear)
                inline constexpr decltype(auto) operator<(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using RhsType = LinearPolynomial<OriginType<Rhs>>;
                    return Inequality<OriginType<Lhs>, RhsType>{ std::forward<Lhs>(lhs), RhsType{ std::forward<Rhs>(rhs) }, InequalitySign::Less };
                }

                template<ExpressionType Lhs, Invariant Rhs>
                    requires (Lhs::category == ExpressionCategory::Linear)
                inline constexpr decltype(auto) operator<=(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using RhsType = LinearPolynomial<OriginType<Rhs>>;
                    return Inequality<OriginType<Lhs>, RhsType>{ std::forward<Lhs>(lhs), RhsType{ std::forward<Rhs>(rhs) }, InequalitySign::LessEqual };
                }

                template<ExpressionType Lhs, Invariant Rhs>
                    requires (Lhs::category == ExpressionCategory::Linear)
                inline constexpr decltype(auto) operator>(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using RhsType = LinearPolynomial<OriginType<Rhs>>;
                    return Inequality<OriginType<Lhs>, RhsType>{ std::forward<Lhs>(lhs), RhsType{ std::forward<Rhs>(rhs) }, InequalitySign::Greater };
                }

                template<ExpressionType Lhs, Invariant Rhs>
                    requires (Lhs::category == ExpressionCategory::Linear)
                inline constexpr decltype(auto) operator>=(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using RhsType = LinearPolynomial<OriginType<Rhs>>;
                    return Inequality<OriginType<Lhs>, RhsType>{ std::forward<Lhs>(lhs), RhsType{ std::forward<Rhs>(rhs) }, InequalitySign::Greater };
                }
                
                // operators between symbol and expression
                
                template<PureSymbolType Lhs, ExpressionType Rhs>
                    requires (Rhs::category == ExpressionCategory::Linear)
                inline constexpr decltype(auto) operator==(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using LhsType = LinearMonomial<f64, f64, OriginType<Lhs>>;
                    return Inequality<LhsType, OriginType<Rhs>>{ LhsType{ std::forward<Lhs>(lhs) }, std::forward<Rhs>(rhs), InequalitySign::Equal };
                }

                template<PureSymbolType Lhs, ExpressionType Rhs>
                    requires (Rhs::category == ExpressionCategory::Linear)
                inline constexpr decltype(auto) operator!=(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using LhsType = LinearMonomial<f64, f64, OriginType<Lhs>>;
                    return Inequality<LhsType, OriginType<Rhs>>{ LhsType{ std::forward<Lhs>(lhs) }, std::forward<Rhs>(rhs), InequalitySign::Unequal };
                }

                template<PureSymbolType Lhs, ExpressionType Rhs>
                    requires (Rhs::category == ExpressionCategory::Linear)
                inline constexpr decltype(auto) operator<(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using LhsType = LinearMonomial<f64, f64, OriginType<Lhs>>;
                    return Inequality<LhsType, OriginType<Rhs>>{ LhsType{ std::forward<Lhs>(lhs) }, std::forward<Rhs>(rhs), InequalitySign::Less };
                }

                template<PureSymbolType Lhs, ExpressionType Rhs>
                    requires (Rhs::category == ExpressionCategory::Linear)
                inline constexpr decltype(auto) operator<=(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using LhsType = LinearMonomial<f64, f64, OriginType<Lhs>>;
                    return Inequality<LhsType, OriginType<Rhs>>{ LhsType{ std::forward<Lhs>(lhs) }, std::forward<Rhs>(rhs), InequalitySign::LessEqual };
                }

                template<PureSymbolType Lhs, ExpressionType Rhs>
                    requires (Rhs::category == ExpressionCategory::Linear)
                inline constexpr decltype(auto) operator>(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using LhsType = LinearMonomial<f64, f64, OriginType<Lhs>>;
                    return Inequality<LhsType, OriginType<Rhs>>{ LhsType{ std::forward<Lhs>(lhs) }, std::forward<Rhs>(rhs), InequalitySign::Greater };
                }

                template<PureSymbolType Lhs, ExpressionType Rhs>
                    requires (Rhs::category == ExpressionCategory::Linear)
                inline constexpr decltype(auto) operator>=(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using LhsType = LinearMonomial<f64, f64, OriginType<Lhs>>;
                    return Inequality<LhsType, OriginType<Rhs>>{ LhsType{ std::forward<Lhs>(lhs) }, std::forward<Rhs>(rhs), InequalitySign::GreaterEqual };
                }

                // operators between expression and symbol

                template<ExpressionType Lhs, PureSymbolType Rhs>
                    requires (Rhs::category == ExpressionCategory::Linear)
                inline constexpr decltype(auto) operator==(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using RhsType = LinearMonomial<f64, f64, OriginType<Rhs>>;
                    return Inequality<OriginType<Lhs>, RhsType>{ std::forward<Lhs>(lhs), RhsType{ std::forward<Rhs>(rhs) }, InequalitySign::Equal };
                }

                template<ExpressionType Lhs, PureSymbolType Rhs>
                    requires (Rhs::category == ExpressionCategory::Linear)
                inline constexpr decltype(auto) operator!=(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using RhsType = LinearMonomial<f64, f64, OriginType<Rhs>>;
                    return Inequality<OriginType<Lhs>, RhsType>{ std::forward<Lhs>(lhs), RhsType{ std::forward<Rhs>(rhs) }, InequalitySign::Unequal };
                }

                template<ExpressionType Lhs, PureSymbolType Rhs>
                    requires (Rhs::category == ExpressionCategory::Linear)
                inline constexpr decltype(auto) operator<(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using RhsType = LinearMonomial<f64, f64, OriginType<Rhs>>;
                    return Inequality<OriginType<Lhs>, RhsType>{ std::forward<Lhs>(lhs), RhsType{ std::forward<Rhs>(rhs) }, InequalitySign::Less };
                }

                template<ExpressionType Lhs, PureSymbolType Rhs>
                    requires (Rhs::category == ExpressionCategory::Linear)
                inline constexpr decltype(auto) operator<=(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using RhsType = LinearMonomial<f64, f64, OriginType<Rhs>>;
                    return Inequality<OriginType<Lhs>, RhsType>{ std::forward<Lhs>(lhs), RhsType{ std::forward<Rhs>(rhs) }, InequalitySign::LessEqual };
                }

                template<ExpressionType Lhs, PureSymbolType Rhs>
                    requires (Rhs::category == ExpressionCategory::Linear)
                inline constexpr decltype(auto) operator>(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using RhsType = LinearMonomial<f64, f64, OriginType<Rhs>>;
                    return Inequality<OriginType<Lhs>, RhsType>{ std::forward<Lhs>(lhs), RhsType{ std::forward<Rhs>(rhs) }, InequalitySign::Greater };
                }

                template<ExpressionType Lhs, PureSymbolType Rhs>
                    requires (Rhs::category == ExpressionCategory::Linear)
                inline constexpr decltype(auto) operator>=(Lhs&& lhs, Rhs&& rhs) noexcept
                {
                    using RhsType = LinearMonomial<f64, f64, OriginType<Rhs>>;
                    return Inequality<OriginType<Lhs>, RhsType>{ std::forward<Lhs>(lhs), RhsType{ std::forward<Rhs>(rhs) }, InequalitySign::GreaterEqual };
                }
            };

            namespace quadratic
            {
                // operators
            };

            namespace standard
            {
                // operators
            };
        };
    };
};

namespace std
{
    template<ospf::ExpressionType Lhs, ospf::ExpressionType Rhs>
    struct formatter<ospf::Inequality<Lhs, Rhs>, char>
        : public formatter<string_view, char>
    {
        template<typename FormatContext>
        inline decltype(auto) format(const ospf::Inequality<Lhs, Rhs>& ineq, FormatContext& fc) const
        {
            static const auto _formatter = formatter<string_view, char>{};
            return _formatter(std::format("{} {} {}", ineq.lhs(), ospf::to_string(ineq.sign()), ineq.rhs()), fc);
        }
    };

    template<ospf::ExpressionType Lhs, ospf::ExpressionType Rhs>
    struct formatter<ospf::Inequality<Lhs, Rhs>, ospf::wchar>
        : public formatter<wstring_view, ospf::wchar>
    {
        template<typename FormatContext>
        inline decltype(auto) format(const ospf::Inequality<Lhs, Rhs>& ineq, FormatContext& fc) const
        {
            static const auto _formatter = formatter<wstring_view, ospf::wchar>{};
            return _formatter(std::format("L{} {} {}", ineq.lhs(), ospf::to_string<ospf::InequalitySign, ospf::wchar>(ineq.sign()), ineq.rhs()), fc);
        }
    };
};

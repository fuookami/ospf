#pragma once

#include <ospf/functional/result.hpp>
#include <ospf/functional/value_or_reference.hpp>
#include <ospf/math/algebra/concepts/real_number.hpp>
#include <ospf/math/symbol/category.hpp>

namespace ospf
{
    inline namespace math
    {
        inline namespace symbol
        {
            template<Invariant T, Invariant ST, ExpressionCategory cat, typename Self>
            class Expression
            {
                OSPF_CRTP_IMPL;

            public:
                using ValueType = OriginType<T>;
                using SymbolValueType = OriginType<ST>;
                static constexpr const ExpressionCategory category = cat;

            public:
                constexpr Expression(void) = default;
                constexpr Expression(const Expression& ano) = default;
                constexpr Expression(Expression&& ano) noexcept = default;
                constexpr Expression& operator=(const Expression& rhs) = default;
                constexpr Expression& operator=(Expression&& rhs) noexcept = default;
                constexpr ~Expression(void) noexcept = default;

            public:
                inline constexpr Result<ValueType> value(const std::function<Result<SymbolValueType>(const std::string_view)>& values) const noexcept
                {
                    return Trait::get_value(self(), values);
                }

                inline constexpr Result<ValueType> value(const StringHashMap<std::string_view, SymbolValueType>& values) const noexcept
                {
                    return value([&values](const std::string_view symbol) -> Result<SymbolValueType>
                        {
                            const auto it = values.find(symbol);
                            if (it == values.cend())
                            {
                                return OSPFError{ OSPFErrCode::ApplicationFail, std::format("value of symbol \"{}\" unmatched", symbol) };
                            }
                            return it->second;
                        });
                }

                template<Invariant V>
                    requires DecayNotSameAs<V, SymbolValueType> && std::convertible_to<V, SymbolValueType>
                inline constexpr Result<ValueType> value(const StringHashMap<std::string_view, V>& values) const noexcept
                {
                    return value([&values](const std::string_view symbol) -> Result<SymbolValueType>
                        {
                            const auto it = values.find(symbol);
                            if (it == values.cend())
                            {
                                return OSPFError{ OSPFErrCode::ApplicationFail, std::format("value of symbol \"{}\" unmatched", symbol) };
                            }

                            using ThisType = OriginType<decltype(it->second)>;
                            if constexpr (DecaySameAs<ThisType, SymbolValueType>)
                            {
                                return it->second;
                            }
                            else
                            {
                                return static_cast<SymbolValueType>(it->second);
                            }
                        });
                }

                template<typename F>
                    requires DecaySameAsOrConvertibleTo<std::invoke_result_t<F, std::string_view>, SymbolValueType>
                inline constexpr Result<ValueType> value(const F& values) const noexcept
                {
                    return value([&values](const std::string_view symbol) -> Result<SymbolValueType>
                        {
                            using ThisType = OriginType<decltype(values(symbol))>;
                            if constexpr (DecaySameAs<ThisType, SymbolValueType>)
                            {
                                return values(symbol);
                            }
                            else
                            {
                                return static_cast<SymbolValueType>(values(symbol));
                            }
                        });
                }

            private:
                struct Trait : public Self
                {
                    inline static constexpr Result<ValueType> get_value(const Self& self, const std::function<Result<SymbolValueType>(const std::string_view)>& values) noexcept
                    {
                        static const auto get_impl = &Self::OSPF_CRTP_FUNCTION(get_value_by);
                        return (self.*get_impl)(values);
                    }
                };
            };

            template<typename T>
            concept ExpressionType = Invariant<typename T::ValueType>
                && Invariant<typename T::SymbolValueType>
                && requires (const T& expression, const std::function<Result<typename T::SymbolValueType>(const std::string_view)>& values)
                {
                    { T::category } -> DecaySameAs<ExpressionCategory>;
                    { expression.value(values) } -> DecaySameAs<Result<typename T::ValueType>>;
                };

            template<typename... Ts>
            struct IsAllExpressionType;

            template<typename T>
            struct IsAllExpressionType<T>
            {
                static constexpr const bool value = ExpressionType<T>;
            };

            template<typename T, typename... Ts>
            struct IsAllExpressionType<T, Ts...>
            {
                static constexpr const bool value = ExpressionType<T> && IsAllExpressionType<Ts...>::value;
            };

            template<typename... Ts>
            inline static constexpr const bool is_all_expression_type = IsAllExpressionType<Ts...>::value;

            template<typename... Ts>
            concept AllExpressionType = is_all_expression_type<Ts...>;

            template<typename T, typename V, typename SV, ExpressionCategory cat>
            concept ExpressionTypeOf = ExpressionType<T>
                && Invariant<V> 
                && Invariant<SV>
                && DecaySameAsOrConvertibleTo<typename T::ValueType, V>
                && DecaySameAsOrConvertibleTo<SV, typename T::SymbolValueType>
                && T::category == cat
                && requires (const T& expression, const std::function<Result<SV>(const std::string_view)>& values)
                {
                    { expression.value(values) } -> DecaySameAs<Result<typename T::ValueType>>;
                };

            template<typename V, typename SV, ExpressionCategory cat, typename... Ts>
            struct IsAllExpressionTypeOf;

            template<typename V, typename SV, ExpressionCategory cat, typename T>
            struct IsAllExpressionTypeOf<V, SV, cat, T>
            {
                static constexpr const bool value = ExpressionTypeOf<T, V, SV, cat>;
            };

            template<typename V, typename SV, ExpressionCategory cat, typename T, typename... Ts>
            struct IsAllExpressionTypeOf<V, SV, cat, T, Ts...>
            {
                static constexpr const bool value = ExpressionTypeOf<T, V, SV, cat> && IsAllExpressionTypeOf<V, SV, cat, Ts...>::value;
            };

            template<typename V, typename SV, ExpressionCategory cat, typename... Ts>
            inline static constexpr const bool is_all_expression_type_of = IsAllExpressionTypeOf<V, SV, cat, Ts...>::value;

            template<typename V, typename SV, ExpressionCategory cat, typename... Ts>
            concept AllExpressionTypeOf = is_all_expression_type_of<V, SV, cat, Ts...>;
        };
    };
};

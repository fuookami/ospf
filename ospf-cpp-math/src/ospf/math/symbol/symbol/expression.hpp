#pragma once

#include <ospf/math/functional/predicate.hpp>
#include <ospf/math/symbol/symbol/concepts.hpp>
#include <ospf/math/symbol/expression.hpp>
#include <ospf/meta_programming/type_info.hpp>

namespace ospf
{
    inline namespace math
    {
        inline namespace symbol
        {
            template<Invariant T, Invariant ST, ExpressionCategory cat>
            class IExprSymbol
                : public Symbol<IExprSymbol<T, ST, cat>>, public Expression<T, ST, cat, IExprSymbol<T, ST, cat>>
            {
                using SymbolImpl = Symbol<IExprSymbol<T, ST, cat>>;
                using ExprImpl = Expression<T, ST, cat, IExprSymbol<T, ST, cat>>;

            public:
                using ValueType = OriginType<T>;
                using SymbolValueType = OriginType<ST>;
                using TransferType = Extractor<SymbolValueType, ValueType>;
                static constexpr ExpressionCategory category = cat;

            public:
                template<typename = void>
                    requires DecaySameAsOrConvertibleTo<SymbolValueType, ValueType>
                constexpr IExprSymbol(std::string name)
                    : SymbolImpl(std::move(name)) {}

                template<typename = void>
                    requires DecaySameAsOrConvertibleTo<SymbolValueType, ValueType>
                constexpr IExprSymbol(std::string name, std::string display_name)
                    : SymbolImpl(std::move(name), std::move(display_name)) {}

                constexpr IExprSymbol(std::string name, TransferType transfer)
                    : SymbolImpl(std::move(name)), _transfer(std::move(transfer)) {}

                constexpr IExprSymbol(std::string name, std::string display_name, TransferType transfer)
                    : SymbolImpl(std::move(name), std::move(display_name)), _transfer(std::move(transfer)) {}

                constexpr IExprSymbol(const IExprSymbol& ano) = default;
                constexpr IExprSymbol(IExprSymbol&& ano) noexcept = default;
                constexpr IExprSymbol& operator=(const IExprSymbol& rhs) = default;
                constexpr IExprSymbol& operator=(IExprSymbol&& rhs) noexcept = default;
                virtual ~IExprSymbol(void) noexcept = default;

            protected:
                virtual RetType<ValueType> value_by(const std::function<Result<ValueType>(const std::string_view)>& values) const noexcept = 0;

            OSPF_CRTP_PERMISSION:
                inline constexpr const bool OSPF_CRTP_FUNCTION(is_pure)(void) const noexcept
                {
                    return false;
                }

                inline constexpr RetType<ValueType> OSPF_CRTP_FUNCTION(get_value_by)(const std::function<Result<SymbolValueType>(const std::string_view)>& values) const noexcept
                {
                    auto ret = values(this->name());
                    if (ret.is_succeeded())
                    {
                        if (_transfer.has_value())
                        {
                            return (*_transfer)(std::move(ret).unwrap());
                        }
                        else
                        {
                            if constexpr (DecaySameAs<SymbolValueType, ValueType>)
                            {
                                return std::move(ret).unwrap();
                            }
                            else if (std::convertible_to<SymbolValueType, ValueType>)
                            {
                                return static_cast<ValueType>(std::move(ret).unwrap());
                            }
                            else
                            {
                                return OSPFError{ OSPFErrCode::ApplicationFail, std::format("lost transfer for {} to {}", TypeInfo<SymbolValueType>::name(), TypeInfo<ValueType>::name()) };
                            }
                        }
                    }
                    else
                    {
                        return value_by(values);
                    }
                }

            private:
                std::optional<TransferType> _transfer;
            };

            template<ExpressionType Expr>
            class ExprSymbol
                : public IExprSymbol<typename Expr::ValueType, typename Expr::SymbolValueType, Expr::category>
            {
                using Interface = IExprSymbol<typename Expr::ValueType, typename Expr::SymbolValueType, Expr::category>;

            public:
                using ExpressionType = OriginType<Expr>;
                using typename Interface::ValueType;
                using typename Interface::SymbolValueType;
                using typename Interface::TransferType;

            public:
                template<typename = void>
                    requires DecaySameAsOrConvertibleTo<SymbolValueType, ValueType>
                constexpr ExprSymbol(std::string name)
                    : Interface(std::move(name)) {}

                template<typename = void>
                    requires DecaySameAsOrConvertibleTo<SymbolValueType, ValueType>
                constexpr ExprSymbol(std::string name, std::string display_name)
                    : Interface(std::move(name), std::move(display_name)) {}

                constexpr ExprSymbol(std::string name, TransferType transfer)
                    : Interface(std::move(name), std::move(transfer)) {}

                constexpr ExprSymbol(std::string name, std::string display_name, TransferType transfer)
                    : Interface(std::move(name), std::move(display_name), std::move(transfer)) {}

            public:
                constexpr ExprSymbol(const ExprSymbol& ano) = default;
                constexpr ExprSymbol(ExprSymbol&& ano) noexcept = default;
                constexpr ExprSymbol& operator=(const ExprSymbol& rhs) = default;
                constexpr ExprSymbol& operator=(ExprSymbol&& rhs) noexcept = default;
                constexpr ~ExprSymbol(void) noexcept = default;

            public:
                inline RetType<ValueType> value_of(const std::function<Result<ValueType>(const std::string_view)>& values) const noexcept override
                {
                    return _expr.value(values);
                }

            private:
                ExpressionType _expr;
            };

            template<typename T>
            concept ExpressionSymbolType = ExpressionType<T> && SymbolType<T>;

            template<typename... Ts>
            concept AllExpressionSymbolType = AllExpressionType<Ts...> && AllSymbolType<Ts...>;

            template<typename T, typename V, typename SV, ExpressionCategory cat>
            concept ExpressionSymbolTypeOf = ExpressionTypeOf<T, V, SV, cat> && SymbolType<T>;

            template<typename V, typename SV, ExpressionCategory cat, typename... Ts>
            concept AllExpressionSymbolTypeOf = AllExpressionTypeOf<V, SV, cat, Ts...> && AllSymbolType<Ts...>;
        };
    };
};

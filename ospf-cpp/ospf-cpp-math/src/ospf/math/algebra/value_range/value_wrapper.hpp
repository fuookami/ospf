#pragma once

#include <ospf/concepts/with_default.hpp>
#include <ospf/exception.hpp>
#include <ospf/math/algebra/concepts/real_number.hpp>
#include <ospf/math/algebra/operator/arithmetic/add.hpp>
#include <ospf/math/algebra/operator/arithmetic/sub.hpp>
#include <ospf/math/algebra/operator/arithmetic/mul.hpp>
#include <ospf/math/algebra/operator/arithmetic/div.hpp>
#include <ospf/math/algebra/operator/comparison/equal.hpp>
#include <ospf/math/algebra/operator/comparison/unequal.hpp>
#include <ospf/math/algebra/operator/comparison/less.hpp>
#include <ospf/math/algebra/operator/comparison/less_equal.hpp>
#include <ospf/math/algebra/operator/comparison/greater.hpp>
#include <ospf/math/algebra/operator/comparison/greater_equal.hpp>
#include <ospf/type_family.hpp>
#include <variant>

namespace ospf
{
    inline namespace math
    {
        inline namespace algebra
        {
            namespace value_range
            {
                template<Invariant T>
                class ValueWrapper
                {
                    using Variant = std::variant<OriginType<T>, Infinity, NegativeInfinity>;

                public:
                    using ValueType = OriginType<T>;

                public:
                    inline static constexpr Variant wrap(ArgCLRefType<ValueType> value)
                    {
                        if constexpr (RealNumber<ValueType>)
                        {
                            if (RealNumberTrait<ValueType>::is_inf(value))
                            {
                                return Variant{ std::in_place_index<1_uz>, inf };
                            }
                            else if (RealNumberTrait<ValueType>::is_neg_inf(value))
                            {
                                return Variant{ std::in_place_index<2_uz>, neg_inf };
                            }
                            else if (RealNumberTrait<ValueType>::is_nan(value))
                            {
                                throw OSPFException{ OSPFErrCode::ApplicationError, "invalid argument NaN for value range" };
                                return Variant{ std::in_place_index<1_uz>, inf };
                            }
                            else
                            {
                                return Variant{ std::in_place_index<0_uz>, value };
                            }
                        }
                        else
                        {
                            return Variant{ std::in_place_index<0_uz>, value };
                        }
                    }

                    template<typename = void>
                        requires ReferenceFaster<ValueType> && std::movable<ValueType>
                    inline static constexpr Variant wrap(ArgRRefType<ValueType> value)
                    {
                        if constexpr (RealNumber<ValueType>)
                        {
                            if (RealNumberTrait<ValueType>::is_inf(value))
                            {
                                return Variant{ std::in_place_index<1_uz>, inf };
                            }
                            else if (RealNumberTrait<ValueType>::is_neg_inf(value))
                            {
                                return Variant{ std::in_place_index<2_uz>, neg_inf };
                            }
                            else if (RealNumberTrait<ValueType>::is_nan(value))
                            {
                                throw OSPFException{ OSPFErrCode::ApplicationError, "invalid argument NaN for value range" };
                                return Variant{ std::in_place_index<1_uz>, inf };
                            }
                            else
                            {
                                return Variant{ std::in_place_index<0_uz>, move<ValueType>(value) };
                            }
                        }
                        else
                        {
                            return Variant{ std::in_place_index<0_uz>, move<ValueType>(value) };
                        }
                    }

                public:
                    template<typename = void>
                        requires WithDefault<ValueType>
                    constexpr ValueWrapper(void)
                        : ValueWrapper(DefaultValue<ValueType>::value()) {}

                    constexpr ValueWrapper(ArgCLRefType<ValueType> value)
                        : _variant(wrap(value)) {}

                    template<typename = void>
                        requires ReferenceFaster<ValueType> && std::movable<ValueType>
                    constexpr ValueWrapper(ArgRRefType<ValueType> value)
                        : _variant(wrap(move<ValueType>(value))) {}

                    constexpr ValueWrapper(const Infinity _)
                        : _variant(std::in_place_index<1_uz>, inf) {}

                    constexpr ValueWrapper(const NegativeInfinity _)
                        : _variant(std::in_place_index<2_uz>, neg_inf) {}

                public:
                    constexpr ValueWrapper(const ValueWrapper& ano) = default;
                    constexpr ValueWrapper(ValueWrapper&& ano) noexcept = default;
                    constexpr ValueWrapper& operator=(const ValueWrapper& rhs) = default;
                    constexpr ValueWrapper& operator=(ValueWrapper&& rhs) noexcept = default;
                    constexpr ~ValueWrapper(void) noexcept = default;

                public:
                    inline constexpr operator const Variant&(void) const noexcept
                    {
                        return _variant;
                    }

                    inline constexpr const bool is_inf_or_neg_inf(void) const noexcept
                    {
                        return _variant.index() != 0_uz;
                    }

                    // todo: it is not must to return a copy, may be reference
                    inline constexpr RetType<ValueType> unwrap(void) const noexcept
                    {
                        if constexpr (RealNumber<ValueType>)
                        {
                            return std::visit([](const auto& value) -> ValueType
                                {
                                    using ThisValueType = OriginType<decltype(value)>;
                                    if constexpr (DecaySameAs<ThisValueType, Infinity>)
                                    {
                                        if constexpr (DecaySameAs<decltype(RealNumberTrait<ValueType>::inf()), std::optional<ValueType>>)
                                        {
                                            return RealNumberTrait<ValueType>::inf().value_or(*BoundedTrait<ValueType>::maximum());
                                        }
                                        else
                                        {
                                            return *RealNumberTrait<ValueType>::inf().value_or(*BoundedTrait<ValueType>::maximum());
                                        }
                                    }
                                    else if constexpr (DecaySameAs<ThisValueType, NegativeInfinity>)
                                    {
                                        if constexpr (DecaySameAs<decltype(RealNumberTrait<ValueType>::neg_inf()), std::optional<ValueType>>)
                                        {
                                            return RealNumberTrait<ValueType>::neg_inf().value_or(*BoundedTrait<ValueType>::minimum());
                                        }
                                        else
                                        {
                                            return *RealNumberTrait<ValueType>::neg_inf().value_or(*BoundedTrait<ValueType>::minimum());
                                        }
                                    }
                                    else
                                    {
                                        return value;
                                    }
                                }, _variant);
                        }
                        else
                        {
                            return std::visit([](const auto& value) -> ValueType
                                {
                                    using ThisValueType = OriginType<decltype(value)>;
                                    if constexpr (DecaySameAs<ThisValueType, Infinity>)
                                    {
                                        if constexpr (DecaySameAs<decltype(BoundedTrait<ValueType>::maximum()), std::optional<ValueType>>)
                                        {
                                            return *BoundedTrait<ValueType>::maximum();
                                        }
                                        else
                                        {
                                            return **BoundedTrait<ValueType>::maximum();
                                        }
                                    }
                                    else if constexpr (DecaySameAs<ThisValueType, NegativeInfinity>)
                                    {
                                        if constexpr (DecaySameAs<decltype(BoundedTrait<ValueType>::minimum()), std::optional<ValueType>>)
                                        {
                                            return *BoundedTrait<ValueType>::minimum();
                                        }
                                        else
                                        {
                                            return **BoundedTrait<ValueType>::minimum();
                                        }
                                    }
                                    else
                                    {
                                        return value;
                                    }
                                }, _variant);
                        }
                    }

                    template<Invariant U>
                        requires std::convertible_to<ValueType, U>
                    inline constexpr ValueWrapper<U> to(void) const noexcept
                    {
                        return std::visit([](const auto& value) -> ValueWrapper<U>
                            {
                                using ValueType = OriginType<decltype(value)>;
                                if constexpr (DecaySameAs<ValueType, Infinity, NegativeInfinity>)
                                {
                                    return value;
                                }
                                else
                                {
                                    return static_cast<U>(value);
                                }
                            }, _variant);
                    }

                public:
                    template<typename = void>
                        requires Neg<ValueType>
                    inline constexpr ValueWrapper operator-(void) noexcept
                    {
                        return std::visit([](const auto& this_value) -> ValueWrapper
                            {
                                using ThisValueType = OriginType<decltype(this_value)>;
                                if constexpr (DecaySameAs<ThisValueType, Infinity>)
                                {
                                    return neg_inf;
                                }
                                else if constexpr (DecaySameAs<ThisValueType, NegativeInfinity>)
                                {
                                    return inf;
                                }
                                else
                                {
                                    return -this_value;
                                }
                            }, _variant);
                    }

                public:
                    template<typename = void>
                        requires Add<ValueType>
                    inline constexpr ValueWrapper operator+(const ValueWrapper& value) const
                    {
                        return std::visit([&value](const auto& lhs_value) -> ValueWrapper
                            {
                                using LhsValueType = OriginType<decltype(lhs_value)>;
                                if constexpr (DecaySameAs<LhsValueType, Infinity>)
                                {
                                    return std::visit([](const auto& rhs_value) -> ValueWrapper
                                        {
                                            using RhsValueType = OriginType<decltype(rhs_value)>;
                                            if constexpr (DecaySameAs<RhsValueType, NegativeInfinity>)
                                            {
                                                throw OSPFException{ OSPFErrCode::ApplicationError, "invalid plus between inf and -inf" };
                                                return ArithmeticTrait<ValueType>::zero();
                                            }
                                            else
                                            {
                                                return inf;
                                            }
                                        }, value);
                                }
                                else if constexpr (DecaySameAs<LhsValueType, NegativeInfinity>)
                                {
                                    return std::visit([](const auto& rhs_value) -> ValueWrapper
                                        {
                                            using RhsValueType = OriginType<decltype(rhs_value)>;
                                            if constexpr (DecaySameAs<RhsValueType, Infinity>)
                                            {
                                                throw OSPFException{ OSPFErrCode::ApplicationError, "invalid plus between inf and -inf" };
                                                return ArithmeticTrait<ValueType>::zero();
                                            }
                                            else
                                            {
                                                return neg_inf;
                                            }
                                        }, value);
                                }
                                else
                                {
                                    return std::visit([&lhs_value](const auto& rhs_value) -> ValueWrapper
                                        {
                                            using RhsValueType = OriginType<decltype(rhs_value)>;
                                            if constexpr (DecaySameAs<RhsValueType, Infinity>)
                                            {
                                                return inf;
                                            }
                                            else if constexpr (DecaySameAs<RhsValueType, NegativeInfinity>)
                                            {
                                                return neg_inf;
                                            }
                                            else
                                            {
                                                return lhs_value + rhs_value;
                                            }
                                        }, value);
                                }
                            }, _variant);
                    }

                    template<typename = void>
                        requires AddAssign<ValueType>
                    inline constexpr ValueWrapper& operator+=(const ValueWrapper& value)
                    {
                        std::visit([this, &value](auto& lhs_value)
                            {
                                using LhsValueType = OriginType<decltype(lhs_value)>;
                                if constexpr (DecaySameAs<LhsValueType, Infinity>)
                                {
                                    std::visit([](const auto& rhs_value)
                                        {
                                            using RhsValueType = OriginType<decltype(rhs_value)>;
                                            if constexpr (DecaySameAs<RhsValueType, NegativeInfinity>)
                                            {
                                                throw OSPFException{ OSPFErrCode::ApplicationError, "invalid plus between inf and -inf" };
                                            }
                                        }, value);
                                }
                                else if constexpr (DecaySameAs<LhsValueType, NegativeInfinity>)
                                {
                                    std::visit([](const auto& rhs_value)
                                        {
                                            using RhsValueType = OriginType<decltype(rhs_value)>;
                                            if constexpr (DecaySameAs<RhsValueType, Infinity>)
                                            {
                                                throw OSPFException{ OSPFErrCode::ApplicationError, "invalid plus between inf and -inf" };
                                            }
                                        }, value);
                                }
                                else
                                {
                                    std::visit([this, &lhs_value](const auto& rhs_value)
                                        {
                                            using RhsValueType = OriginType<decltype(rhs_value)>;
                                            if constexpr (DecaySameAs<RhsValueType, Infinity>)
                                            {
                                                this->_variant = Variant{ std::in_place_index<1_uz>, inf };
                                            }
                                            else if constexpr (DecaySameAs<RhsValueType, NegativeInfinity>)
                                            {
                                                this->_variant = Variant{ std::in_place_index<2_uz>, neg_inf };
                                            }
                                            else
                                            {
                                                lhs_value += rhs_value;
                                            }
                                        }, value);
                                }
                            }, _variant);
                        return *this;
                    }

                    template<typename = void>
                        requires Add<ValueType>
                    inline constexpr ValueWrapper operator+(ArgCLRefType<ValueType> value) const
                    {
                        if (RealNumber<ValueType>)
                        {
                            if (RealNumberTrait<ValueType>::is_nan(value))
                            {
                                throw OSPFException{ OSPFErrCode::ApplicationError, "invalid argument NaN for value range"};
                            }
                            return std::visit([&value](const auto& this_value) -> ValueWrapper
                                {
                                    using ThisValueType = OriginType<decltype(this_value)>;
                                    if constexpr (DecaySameAs<ThisValueType, Infinity>)
                                    {
                                        if (RealNumberTrait<ValueType>::is_neg_inf(value))
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid plus between inf and -inf"};
                                        }
                                        return inf;
                                    }
                                    else if constexpr (DecaySameAs<ThisValueType, NegativeInfinity>)
                                    {
                                        if (RealNumberTrait<ValueType>::is_inf(value))
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid plus between -inf and inf"};
                                        }
                                        return neg_inf;
                                    }
                                    else
                                    {
                                        return this_value + value;
                                    }
                                }, _variant);
                        }
                        else
                        {
                            return std::visit([&value](const auto& this_value) -> ValueWrapper
                                {
                                    using ThisValueType = OriginType<decltype(this_value)>;
                                    if constexpr (DecaySameAs<ThisValueType, Infinity>)
                                    {
                                        return inf;
                                    }
                                    else if constexpr (DecaySameAs<ThisValueType, NegativeInfinity>)
                                    {
                                        return neg_inf;
                                    }
                                    else
                                    {
                                        return this_value + value;
                                    }
                                }, _variant);
                        }
                    }

                    template<typename = void>
                        requires AddAssign<ValueType>
                    inline constexpr ValueWrapper& operator+=(ArgCLRefType<ValueType> value)
                    {
                        if (RealNumber<ValueType>)
                        {
                            if (RealNumberTrait<ValueType>::is_nan(value))
                            {
                                throw OSPFException{ OSPFErrCode::ApplicationError, "invalid argument NaN for value range"};
                            }
                            std::visit([this, &value](auto& this_value) -> ValueWrapper
                                {
                                    using ThisValueType = OriginType<decltype(this_value)>;
                                    if constexpr (DecaySameAs<ThisValueType, Infinity>)
                                    {
                                        if (RealNumberTrait<ValueType>::is_neg_inf(value))
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid plus between inf and -inf" };
                                        }
                                    }
                                    if constexpr (DecaySameAs<ThisValueType, NegativeInfinity>)
                                    {
                                        if (RealNumberTrait<ValueType>::is_inf(value))
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid plus between inf and -inf" };
                                        }
                                    }
                                    else
                                    {
                                        if (RealNumberTrait<ValueType>::is_inf(value))
                                        {
                                            this->_variant = Variant{ std::in_place_index<1_uz>, inf };
                                        }
                                        else if (RealNumberTrait<ValueType>::is_neg_inf(value))
                                        {
                                            this->_variant = Variant{ std::in_place_index<2_uz>, neg_inf };
                                        }
                                        else
                                        {
                                            this_value += value;
                                        }
                                    }
                                }, _variant);
                            return *this;
                        }
                        else
                        {
                            std::visit([&value](auto& this_value)
                                {
                                    using ThisValueType = OriginType<decltype(this_value)>;
                                    if constexpr (DecaySameAs<ThisValueType, ValueType>)
                                    {
                                        this_value += value;
                                    }
                                }, _variant);
                            return *this;
                        }
                    }

                    template<typename = void>
                        requires Sub<ValueType>
                    inline constexpr ValueWrapper operator-(const ValueWrapper& value) const
                    {
                        return std::visit([&value](const auto& lhs_value) -> ValueWrapper
                            {
                                using LhsValueType = OriginType<decltype(lhs_value)>;
                                if constexpr (DecaySameAs<LhsValueType, Infinity>)
                                {
                                    return std::visit([](const auto& rhs_value) -> ValueWrapper
                                        {
                                            using RhsValueType = OriginType<decltype(rhs_value)>;
                                            if constexpr (DecaySameAs<RhsValueType, Infinity>)
                                            {
                                                throw OSPFException{ OSPFErrCode::ApplicationError, "invalid minus between inf and inf" };
                                                return ArithmeticTrait<ValueType>::zero();
                                            }
                                            else
                                            {
                                                return inf;
                                            }
                                        }, value);
                                }
                                else if constexpr (DecaySameAs<LhsValueType, NegativeInfinity>)
                                {
                                    return std::visit([](const auto& rhs_value) -> ValueWrapper 
                                        {
                                            using RhsValueType = OriginType<decltype(rhs_value)>;
                                            if constexpr (DecaySameAs<RhsValueType, NegativeInfinity>)
                                            {
                                                throw OSPFException{ OSPFErrCode::ApplicationError, "invalid minus between -inf and -inf" };
                                                return ArithmeticTrait<ValueType>::zero();
                                            }
                                            else
                                            {
                                                return neg_inf;
                                            }
                                        }, value);
                                }
                                else
                                {
                                    return std::visit([&lhs_value](const auto& rhs_value) -> ValueWrapper 
                                        {
                                            using RhsValueType = OriginType<decltype(rhs_value)>;
                                            if constexpr (DecaySameAs<RhsValueType, Infinity>)
                                            {
                                                return neg_inf;
                                            }
                                            else if constexpr (DecaySameAs<RhsValueType, NegativeInfinity>)
                                            {
                                                return inf;
                                            }
                                            else
                                            {
                                                return lhs_value - rhs_value;
                                            }
                                        }, value);
                                }
                            }, _variant);
                    }

                    template<typename = void>
                        requires SubAssign<ValueType>
                    inline constexpr ValueWrapper& operator-=(const ValueWrapper& value)
                    {
                        std::visit([this, &value](auto& lhs_value)
                            {
                                using LhsValueType = OriginType<decltype(lhs_value)>;
                                if constexpr (DecaySameAs<LhsValueType, Infinity>)
                                {
                                    std::visit([](const auto& rhs_value)
                                        {
                                            using RhsValueType = OriginType<decltype(rhs_value)>;
                                            if constexpr (DecaySameAs<RhsValueType, Infinity>)
                                            {
                                                throw OSPFException{ OSPFErrCode::ApplicationError, "invalid minus between inf and inf" };
                                            }
                                        }, value);
                                }
                                else if constexpr (DecaySameAs<LhsValueType, NegativeInfinity>)
                                {
                                    std::visit([](const auto& rhs_value)
                                        {
                                            using RhsValueType = OriginType<decltype(rhs_value)>;
                                            if constexpr (DecaySameAs<RhsValueType, NegativeInfinity>)
                                            {
                                                throw OSPFException{ OSPFErrCode::ApplicationError, "invalid minus between -inf and -inf" };
                                            }
                                        }, value);
                                }
                                else
                                {
                                    std::visit([this, &lhs_value](const auto& rhs_value)
                                        {
                                            using RhsValueType = OriginType<decltype(rhs_value)>;
                                            if constexpr (DecaySameAs<RhsValueType, Infinity>)
                                            {
                                                this->_variant = Variant{ std::in_place_index<2_uz>, neg_inf };
                                            }
                                            else if constexpr (DecaySameAs<RhsValueType, NegativeInfinity>)
                                            {
                                                this->_variant = Variant{ std::in_place_index<1_uz>, inf };
                                            }
                                            else
                                            {
                                                lhs_value -= rhs_value;
                                            }
                                        }, value);
                                }
                            }, _variant);
                        return *this;
                    }

                    template<typename = void>
                        requires Sub<ValueType>
                    inline constexpr ValueWrapper operator-(ArgCLRefType<ValueType> value) const
                    {
                        if constexpr (RealNumber<ValueType>)
                        {
                            if (RealNumberTrait<ValueType>::is_nan(value))
                            {
                                throw OSPFException{ OSPFErrCode::ApplicationError, "invalid argument NaN for value range" };
                            }
                            return std::visit([&value](const auto& this_value) -> ValueWrapper
                                {
                                    using ThisValueType = OriginType<decltype(this_value)>;
                                    if constexpr (DecaySameAs<ThisValueType, Infinity>)
                                    {
                                        if (RealNumberTrait<ValueType>::is_inf(value))
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid minus between inf and inf" };
                                        }
                                        return inf;
                                    }
                                    else if constexpr (DecaySameAs<ThisValueType, NegativeInfinity>)
                                    {
                                        if (RealNumberTrait<ValueType>::is_neg_inf(value))
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid minus between -inf and -inf" };
                                        }
                                        return neg_inf;
                                    }
                                    else
                                    {
                                        return this_value - value;
                                    }
                                }, _variant);
                        }
                        else
                        {
                            return std::visit([&value](const auto& this_value) -> ValueWrapper
                                {
                                    using ThisValueType = OriginType<decltype(this_value)>;
                                    if constexpr (DecaySameAs<ThisValueType, Infinity>)
                                    {
                                        return inf;
                                    }
                                    else if constexpr (DecaySameAs<ThisValueType, NegativeInfinity>)
                                    {
                                        return neg_inf;
                                    }
                                    else
                                    {
                                        return this_value - value;
                                    }
                                }, _variant);
                        }
                    }

                    template<typename = void>
                        requires SubAssign<ValueType>
                    inline constexpr ValueWrapper& operator-=(ArgCLRefType<ValueType> value)
                    {
                        if constexpr (RealNumber<ValueType>)
                        {
                            if (RealNumberTrait<ValueType>::is_nan(value))
                            {
                                throw OSPFException{ OSPFErrCode::ApplicationError, "invalid argument NaN for value range" };
                            }
                            std::visit([this, &value](auto& this_value) -> ValueWrapper
                                {
                                    using ThisValueType = OriginType<decltype(this_value)>;
                                    if constexpr (DecaySameAs<ThisValueType, Infinity>)
                                    {
                                        if (RealNumberTrait<ValueType>::is_inf(value))
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid minus between inf and inf" };
                                        }
                                    }
                                    else if constexpr (DecaySameAs<ThisValueType, NegativeInfinity>)
                                    {
                                        if (RealNumberTrait<ValueType>::is_neg_inf(value))
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid minus between -inf and -inf" };
                                        }
                                    }
                                    else
                                    {
                                        if (RealNumberTrait<ValueType>::is_inf(value))
                                        {
                                            this->_variant = Variant{ std::in_place_index<2_uz>, neg_inf };
                                        }
                                        else if (RealNumberTrait<ValueType>::is_neg_inf(value))
                                        {
                                            this->_variant = Variant{ std::in_place_index<1_uz>, inf };
                                        }
                                        else
                                        {
                                            this_value -= value;
                                        }
                                    }
                                }, _variant);
                            return *this;
                        }
                        else
                        {
                            std::visit([&value](auto& this_value) -> ValueWrapper
                                {
                                    using ThisValueType = OriginType<decltype(this_value)>;
                                    if constexpr (DecaySameAs<ThisValueType, ValueType>)
                                    {
                                        this_value -= value;
                                    }
                                }, _variant);
                            return *this;
                        }
                    }

                    template<typename = void>
                        requires Mul<ValueType>
                    inline constexpr ValueWrapper operator*(const ValueWrapper& value) const
                    {
                        return std::visit([&value](const auto& lhs_value) -> ValueWrapper
                            {
                                using LhsValueType = OriginType<decltype(lhs_value)>;
                                if constexpr (DecaySameAs<LhsValueType, Infinity>)
                                {
                                    return std::visit([](const auto& rhs_value) -> ValueWrapper
                                        {
                                            using RhsValueType = OriginType<decltype(rhs_value)>;
                                            if constexpr (DecaySameAs<RhsValueType, Infinity>)
                                            {
                                                return inf;
                                            }
                                            else if constexpr (DecaySameAs<RhsValueType, NegativeInfinity>)
                                            {
                                                return neg_inf;
                                            }
                                            else
                                            {
                                                if (is_positive(rhs_value))
                                                {
                                                    return inf;
                                                }
                                                else if (is_negative(rhs_value))
                                                {
                                                    return neg_inf;
                                                }
                                                else
                                                {
                                                    throw OSPFException{ OSPFErrCode::ApplicationError, "invalid multiply between inf and 0" };
                                                    return ArithmeticTrait<ValueType>::zero();
                                                }
                                            }
                                        }, value);
                                }
                                else if constexpr (DecaySameAs<LhsValueType, NegativeInfinity>)
                                {
                                    return std::visit([](const auto& rhs_value) -> ValueWrapper
                                        {
                                            using RhsValueType = OriginType<decltype(rhs_value)>;
                                            if constexpr (DecaySameAs<RhsValueType, Infinity>)
                                            {
                                                return neg_inf;
                                            }
                                            else if constexpr (DecaySameAs<RhsValueType, NegativeInfinity>)
                                            {
                                                return inf;
                                            }
                                            else
                                            {
                                                if (is_positive(rhs_value))
                                                {
                                                    return neg_inf;
                                                }
                                                else if (is_negative(rhs_value))
                                                {
                                                    return inf;
                                                }
                                                else
                                                {
                                                    throw OSPFException{ OSPFErrCode::ApplicationError, "invalid multiply between -inf and 0" };
                                                    return ArithmeticTrait<ValueType>::zero();
                                                }
                                            }
                                        }, value);
                                }
                                else
                                {
                                    return std::visit([&lhs_value](const auto& rhs_value) -> ValueWrapper
                                        {
                                            using RhsValueType = OriginType<decltype(rhs_value)>;
                                            if constexpr (DecaySameAs<RhsValueType, Infinity>)
                                            {
                                                if (is_positive(lhs_value))
                                                {
                                                    return inf;
                                                }
                                                else if (is_negative(lhs_value))
                                                {
                                                    return neg_inf;
                                                }
                                                else
                                                {
                                                    throw OSPFException{ OSPFErrCode::ApplicationError, "invalid multiply between inf and 0" };
                                                    return ArithmeticTrait<ValueType>::zero();
                                                }
                                            }
                                            else if constexpr (DecaySameAs<RhsValueType, NegativeInfinity>)
                                            {
                                                if (is_positive(lhs_value))
                                                {
                                                    return neg_inf;
                                                }
                                                else if (is_negative(lhs_value))
                                                {
                                                    return inf;
                                                }
                                                else
                                                {
                                                    throw OSPFException{ OSPFErrCode::ApplicationError, "invalid multiply between -inf and 0" };
                                                    return ArithmeticTrait<ValueType>::zero();
                                                }
                                            }
                                            else
                                            {
                                                return lhs_value * rhs_value;
                                            }
                                        }, value);
                                }
                            }, _variant);
                    }

                    template<typename = void>
                        requires MulAssign<ValueType>
                    inline constexpr ValueWrapper& operator*=(const ValueWrapper& value)
                    {
                        std::visit([this, &value](auto& lhs_value)
                            {
                                using LhsValueType = OriginType<decltype(lhs_value)>;
                                if constexpr (DecaySameAs<LhsValueType, Infinity>)
                                {
                                    std::visit([this](const auto& rhs_value)
                                        {
                                            using RhsValueType = OriginType<decltype(rhs_value)>;
                                            if constexpr (DecaySameAs<RhsValueType, Infinity>)
                                            {
                                                // nothing to do
                                            }
                                            else if constexpr (DecaySameAs<RhsValueType, NegativeInfinity>)
                                            {
                                                this->_variant = VariantTrait{ std::in_place_index<2_uz>, neg_inf };
                                            }
                                            else
                                            {
                                                if (is_negative(rhs_value))
                                                {
                                                    this->_variant = VariantTrait{ std::in_place_index<2_uz>, neg_inf };
                                                }
                                                else if (!is_positive(rhs_value))
                                                {
                                                    throw OSPFException{ OSPFErrCode::ApplicationError, "invalid multiply between inf and 0" };
                                                }
                                            }
                                        }, value);
                                }
                                else if constexpr (DecaySameAs<LhsValueType, NegativeInfinity>)
                                {
                                    std::visit([this](const auto& rhs_value)
                                        {
                                            using RhsValueType = OriginType<decltype(rhs_value)>;
                                            if constexpr (DecaySameAs<RhsValueType, Infinity>)
                                            {
                                                // nothing to do
                                            }
                                            else if constexpr (DecaySameAs<RhsValueType, NegativeInfinity>)
                                            {
                                                this->_variant = VariantTrait{ std::in_place_index<1_uz>, inf };
                                            }
                                            else
                                            {
                                                if (is_negative(rhs_value))
                                                {
                                                    this->_variant = VariantTrait{ std::in_place_index<1_uz>, inf };
                                                }
                                                else if (!is_positive(rhs_value))
                                                {
                                                    throw OSPFException{ OSPFErrCode::ApplicationError, "invalid multiply between -inf and 0" };
                                                }
                                            }
                                        }, value);
                                }
                                else
                                {
                                    std::visit([this, &lhs_value](const auto& rhs_value)
                                        {
                                            using RhsValueType = OriginType<decltype(rhs_value)>;
                                            if constexpr (DecaySameAs<RhsValueType, Infinity>)
                                            {
                                                if (is_positive(lhs_value))
                                                {
                                                    this->_variant = VariantTrait{ std::in_place_index<1_uz>, inf };
                                                }
                                                else if (is_negative(lhs_value))
                                                {
                                                    this->_variant = VariantTrait{ std::in_place_index<2_uz>, neg_inf };
                                                }
                                                else
                                                {
                                                    throw OSPFException{ OSPFErrCode::ApplicationError, "invalid multiply between inf and 0" };
                                                }
                                            }
                                            else if constexpr (DecaySameAs<RhsValueType, NegativeInfinity>)
                                            {
                                                if (is_positive(lhs_value))
                                                {
                                                    this->_variant = VariantTrait{ std::in_place_index<2_uz>, neg_inf };
                                                }
                                                else if (is_negative(lhs_value))
                                                {
                                                    this->_variant = VariantTrait{ std::in_place_index<1_uz>, inf };
                                                }
                                                else
                                                {
                                                    throw OSPFException{ OSPFErrCode::ApplicationError, "invalid multiply between -inf and 0" };
                                                }
                                            }
                                            else
                                            {
                                                lhs_value *= rhs_value;
                                            }
                                        }, value);
                                }
                            }, _variant);
                        return *this;
                    }

                    template<typename = void>
                        requires Mul<ValueType>
                    inline constexpr ValueWrapper operator*(ArgCLRefType<ValueType> value) const
                    {
                        if constexpr (RealNumber<ValueType>)
                        {
                            if (RealNumberTrait<ValueType>::is_nan(value))
                            {
                                throw OSPFException{ OSPFErrCode::ApplicationError, "invalid argument NaN for value range" };
                            }
                            return std::visit([&value](const auto& this_value) -> ValueWrapper
                                {
                                    using ThisValueType = OriginType<decltype(this_value)>;
                                    if constexpr (DecaySameAs<ThisValueType, Infinity>)
                                    {
                                        if (is_positive(value))
                                        {
                                            return inf;
                                        }
                                        else if (is_negative(value))
                                        {
                                            return neg_inf;
                                        }
                                        else
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid multiply between 0 and inf" };
                                            return ArithmeticTrait<ValueType>::zero();
                                        }
                                    }
                                    else if constexpr (DecaySameAs<ThisValueType, NegativeInfinity>)
                                    {
                                        if (is_positive(value))
                                        {
                                            return neg_inf;
                                        }
                                        else if (is_negative(value))
                                        {
                                            return inf;
                                        }
                                        else
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid multiply between 0 and -inf" };
                                            return ArithmeticTrait<ValueType>::zero();
                                        }
                                    }
                                    else
                                    {
                                        return this_value * value;
                                    }
                                }, _variant);
                        }
                        else
                        {
                            return std::visit([&value](const auto& this_value) -> ValueWrapper
                                {
                                    using ThisValueType = OriginType<decltype(this_value)>;
                                    if constexpr (DecaySameAs<ThisValueType, Infinity>)
                                    {
                                        if (is_positive(value))
                                        {
                                            return inf;
                                        }
                                        else if (is_negative(value))
                                        {
                                            return neg_inf;
                                        }
                                        else
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid multiply between 0 and inf" };
                                            return ArithmeticTrait<ValueType>::zero();
                                        }
                                    }
                                    else if constexpr (DecaySameAs<ThisValueType, NegativeInfinity>)
                                    {
                                        if (is_positive(value))
                                        {
                                            return neg_inf;
                                        }
                                        else if (is_negative(value))
                                        {
                                            return inf;
                                        }
                                        else
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid multiply between 0 and -inf" };
                                            return ArithmeticTrait<ValueType>::zero();
                                        }
                                    }
                                    else
                                    {
                                        return this_value * value;
                                    }
                                }, _variant);
                        }
                    }

                    template<typename = void>
                        requires MulAssign<ValueType>
                    inline constexpr ValueWrapper& operator*=(ArgCLRefType<ValueType> value)
                    {
                        if constexpr (RealNumber<ValueType>)
                        {
                            if (RealNumberTrait<ValueType>::is_nan(value))
                            {
                                throw OSPFException{ OSPFErrCode::ApplicationError, "invalid argument NaN for value range" };
                            }
                            std::visit([this, &value](auto& this_value)
                                {
                                    using ThisValueType = OriginType<decltype(this_value)>;
                                    if constexpr (DecaySameAs<ThisValueType, Infinity>)
                                    {
                                        if (is_negative(value))
                                        {
                                            this->_variant = Variant{ std::in_place_index<2_uz>, neg_inf };
                                        }
                                        else if (!is_positive(value))
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid multiply between 0 and inf" };
                                        }
                                    }
                                    else if constexpr (DecaySameAs<ThisValueType, NegativeInfinity>)
                                    {
                                        if (is_negative(value))
                                        {
                                            this->_variant = Variant{ std::in_place_index<1_uz>, inf };
                                        }
                                        else if (!is_positive(value))
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid multiply between 0 and -inf" };
                                        }
                                    }
                                    else
                                    {
                                        if (RealNumberTrait<ValueType>::is_inf(value))
                                        {
                                            if (is_positive(this_value))
                                            {
                                                this->_variant = Variant{ std::in_place_index<1_uz>, inf };
                                            }
                                            else if (is_negative(this_value))
                                            {
                                                this->_variant = Variant{ std::in_place_index<2_uz>, neg_inf };
                                            }
                                            else
                                            {
                                                throw OSPFException{ OSPFErrCode::ApplicationError, "invalid multiply between 0 and inf" };
                                            }
                                        }
                                        else if (RealNumberTrait<ValueType>::is_neg_inf(value))
                                        {
                                            if (is_positive(this_value))
                                            {
                                                this->_variant = Variant{ std::in_place_index<2_uz>, neg_inf };
                                            }
                                            else if (is_negative(this_value))
                                            {
                                                this->_variant = Variant{ std::in_place_index<1_uz>, inf };
                                            }
                                            else
                                            {
                                                throw OSPFException{ OSPFErrCode::ApplicationError, "invalid multiply between 0 and -inf" };
                                            }
                                        }
                                        else
                                        {
                                            this_value *= value;
                                        }
                                    }
                                }, _variant);
                            return *this;
                        }
                        else
                        {
                            std::visit([this, &value](auto& this_value)
                                {
                                    using ThisValueType = OriginType<decltype(this_value)>;
                                    if constexpr (DecaySameAs<ThisValueType, Infinity>)
                                    {
                                        if (is_negative(value))
                                        {
                                            this->_variant = Variant{ std::in_place_index<2_uz>, neg_inf };
                                        }
                                        else if (!is_positive(value))
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid multiply between 0 and inf" };
                                        }
                                    }
                                    else if constexpr (DecaySameAs<ThisValueType, NegativeInfinity>)
                                    {
                                        if (is_negative(value))
                                        {
                                            this->_variant = Variant{ std::in_place_index<1_uz>, inf };
                                        }
                                        else if (!is_positive(value))
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid multiply between 0 and -inf"};
                                        }
                                    }
                                    else
                                    {
                                        this_value *= value;
                                    }
                                }, _variant);
                            return *this;
                        }
                    }

                    template<typename = void>
                        requires Div<ValueType>
                    inline constexpr ValueWrapper operator/(const ValueWrapper& value) const
                    {
                        return std::visit([&value](const auto& lhs_value) -> ValueWrapper
                            {
                                using LhsValueType = OriginType<decltype(lhs_value)>;
                                if constexpr (DecaySameAs<LhsValueType, Infinity>)
                                {
                                    return std::visit([](const auto& rhs_value) -> ValueWrapper
                                        {
                                            using RhsValueType = OriginType<decltype(rhs_value)>;
                                            if constexpr (DecaySameAs<RhsValueType, Infinity>)
                                            {
                                                throw OSPFException{ OSPFErrCode::ApplicationError, "invalid divide between inf and inf" };
                                                return ArithmeticTrait<ValueType>::zero();
                                            }
                                            else if constexpr (DecaySameAs<RhsValueType, NegativeInfinity>)
                                            {
                                                throw OSPFException{ OSPFErrCode::ApplicationError, "invalid divide between inf and -inf" };
                                                return ArithmeticTrait<ValueType>::zero();
                                            }
                                            else
                                            {
                                                if (is_positive(rhs_value))
                                                {
                                                    return inf;
                                                }
                                                else if (is_negative(rhs_value))
                                                {
                                                    return neg_inf;
                                                }
                                                else
                                                {
                                                    throw OSPFException{ OSPFErrCode::ApplicationError, "invalid divide between inf and 0" };
                                                    return ArithmeticTrait<ValueType>::zero();
                                                }
                                            }
                                        }, value);
                                }
                                else if constexpr (DecaySameAs<LhsValueType, NegativeInfinity>)
                                {
                                    return std::visit([](const auto& rhs_value) -> ValueWrapper
                                        {
                                            using RhsValueType = OriginType<decltype(rhs_value)>;
                                            if constexpr (DecaySameAs<RhsValueType, Infinity>)
                                            {
                                                throw OSPFException{ OSPFErrCode::ApplicationError, "invalid divide between -inf and inf" };
                                                return ArithmeticTrait<ValueType>::zero();
                                            }
                                            else if constexpr (DecaySameAs<RhsValueType, NegativeInfinity>)
                                            {
                                                throw OSPFException{ OSPFErrCode::ApplicationError, "invalid divide between -inf and 0" };
                                                return ArithmeticTrait<ValueType>::zero();
                                            }
                                            else
                                            {
                                                if (is_positive(rhs_value))
                                                {
                                                    return neg_inf;
                                                }
                                                else if (is_positive(rhs_value))
                                                {
                                                    return inf;
                                                }
                                                else
                                                {
                                                    throw OSPFException{ OSPFErrCode::ApplicationError, "invalid divide between -inf and 0" };
                                                    return ArithmeticTrait<ValueType>::zero();
                                                }
                                            }
                                        }, value);
                                }
                                else
                                {
                                    return std::visit([&lhs_value](const auto& rhs_value) -> ValueWrapper
                                        {
                                            using RhsValueType = OriginType<decltype(rhs_value)>;
                                            if constexpr (DecaySameAs<RhsValueType, Infinity, NegativeInfinity>)
                                            {
                                                return ArithmeticTrait<ValueType>::zero();
                                            }
                                            else
                                            {
                                                return lhs_value / rhs_value;
                                            }
                                        }, value);
                                }
                            }, _variant);
                    }

                    template<typename = void>
                        requires DivAssign<ValueType>
                    inline constexpr ValueWrapper& operator/=(const ValueWrapper& value)
                    {
                        std::visit([this, &value](auto& lhs_value)
                            {
                                using LhsValueType = OriginType<decltype(lhs_value)>;
                                if constexpr (DecaySameAs<LhsValueType, Infinity>)
                                {
                                    std::visit([this](const auto& rhs_value)
                                        {
                                            using RhsValueType = OriginType<decltype(rhs_value)>;
                                            if constexpr (DecaySameAs<RhsValueType, Infinity>)
                                            {
                                                throw OSPFException{ OSPFErrCode::ApplicationError, "invalid divide between inf and inf" };
                                            }
                                            else if constexpr (DecaySameAs<RhsValueType, NegativeInfinity>)
                                            {
                                                throw OSPFException{ OSPFErrCode::ApplicationError, "invalid divide between inf and -inf" };
                                            }
                                            else
                                            {
                                                if (is_negative(rhs_value))
                                                {
                                                    this->_variant = Variant{ std::in_place_index<2_uz>, neg_inf };
                                                }
                                                else if (!is_positive(rhs_value))
                                                {
                                                    throw OSPFException{ OSPFErrCode::ApplicationError, "invalid divide between inf and 0" };
                                                }
                                            }
                                        }, value);
                                }
                                else if constexpr (DecaySameAs<LhsValueType, NegativeInfinity>)
                                {
                                    std::visit([this](const auto& rhs_value)
                                        {
                                            using RhsValueType = OriginType<decltype(rhs_value)>;
                                            if constexpr (DecaySameAs<RhsValueType, Infinity>)
                                            {
                                                throw OSPFException{ OSPFErrCode::ApplicationError, "invalid divide between -inf and inf" };
                                            }
                                            else if constexpr (DecaySameAs<RhsValueType, NegativeInfinity>)
                                            {
                                                throw OSPFException{ OSPFErrCode::ApplicationError, "invalid divide between -inf and -inf" };
                                            }
                                            else
                                            {
                                                if (is_negative(rhs_value))
                                                {
                                                    this->_variant = Variant{ std::in_place_index<1_uz>, inf };
                                                }
                                                else if (!is_positive(rhs_value))
                                                {
                                                    throw OSPFException{ OSPFErrCode::ApplicationError, "invalid divide between -inf and 0" };
                                                }
                                            }
                                        }, value);
                                }
                                else
                                {
                                    std::visit([&lhs_value](const auto& rhs_value)
                                        {
                                            using RhsValueType = OriginType<decltype(rhs_value)>;
                                            if constexpr (DecaySameAs<RhsValueType, Infinity, NegativeInfinity>)
                                            {
                                                lhs_value = ArithmeticTrait<ValueType>::zero();
                                            }
                                            else
                                            {
                                                lhs_value /= rhs_value;
                                            }
                                        }, value);
                                }
                            }, _variant);
                        return *this;
                    }

                    template<typename = void>
                        requires Div<ValueType>
                    inline constexpr ValueWrapper operator/(ArgCLRefType<ValueType> value) const
                    {
                        if constexpr (RealNumber<ValueType>)
                        {
                            if (RealNumberTrait<ValueType>::is_nan(value))
                            {
                                throw OSPFException{ OSPFErrCode::ApplicationError, "invalid argument NaN for value range" };
                            }
                            return std::visit([&value](const auto& this_value) -> ValueWrapper
                                {
                                    using ThisValueType = OriginType<decltype(this_value)>;
                                    if constexpr (DecaySameAs<ThisValueType, Infinity>)
                                    {
                                        if (RealNumberTrait<ValueType>::is_inf(value))
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid divide between inf and inf" };
                                            return ArithmeticTrait<ValueType>::zero();
                                        }
                                        else if (RealNumberTrait<ValueType>::is_neg_inf(value))
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid divide between inf and -inf" };
                                            return ArithmeticTrait<ValueType>::zero();
                                        }
                                        else if (is_positive(value))
                                        {
                                            return inf;
                                        }
                                        else if (is_negative(value))
                                        {
                                            return neg_inf;
                                        }
                                        else
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid divide between inf and 0" };
                                            return ArithmeticTrait<ValueType>::zero();
                                        }
                                    }
                                    else if constexpr (DecaySameAs<ThisValueType, NegativeInfinity>)
                                    {
                                        if (RealNumberTrait<ValueType>::is_inf(value))
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid divide between -inf and inf" };
                                            return ArithmeticTrait<ValueType>::zero();
                                        }
                                        if (RealNumberTrait<ValueType>::is_neg_inf(value))
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid divide between -inf and -inf" };
                                            return ArithmeticTrait<ValueType>::zero();
                                        }
                                        else if (is_positive(value))
                                        {
                                            return neg_inf;
                                        }
                                        else if (is_negative(value))
                                        {
                                            return inf;
                                        }
                                        else
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid divide between -inf and 0" };
                                            return ArithmeticTrait<ValueType>::zero();
                                        }
                                    }
                                    else
                                    {
                                        if (value == ArithmeticTrait<ValueType>::zero())
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, std::format("invalid divide between {} and 0", this_value) };
                                        }
                                        return this_value / value;
                                    }
                                }, _variant);
                        }
                        else
                        {
                            return std::visit([&value](const auto& this_value) -> ValueWrapper
                                {
                                    using ThisValueType = OriginType<decltype(this_value)>;
                                    if constexpr (DecaySameAs<ThisValueType, Infinity>)
                                    {
                                        if (is_positive(value))
                                        {
                                            return inf;
                                        }
                                        else if (is_negative(value))
                                        {
                                            return neg_inf;
                                        }
                                        else
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid divide between inf and 0" };
                                            return ArithmeticTrait<ValueType>::zero();
                                        }
                                    }
                                    else if constexpr (DecaySameAs<ThisValueType, NegativeInfinity>)
                                    {
                                        if (is_positive(value))
                                        {
                                            return neg_inf;
                                        }
                                        else if (is_negative(value))
                                        {
                                            return inf;
                                        }
                                        else
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid divide between -inf and 0" };
                                            return ArithmeticTrait<ValueType>::zero();
                                        }
                                    }
                                    else
                                    {
                                        if (value == ArithmeticTrait<ValueType>::zero())
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, std::format("invalid divide between {} and 0", this_value) };
                                            return ArithmeticTrait<ValueType>::zero();
                                        }
                                        return this_value / value;
                                    }
                                }, _variant);
                        }
                    }

                    template<typename = void>
                        requires DivAssign<ValueType>
                    inline constexpr ValueWrapper& operator/=(ArgCLRefType<ValueType> value)
                    {
                        if constexpr (RealNumber<ValueType>)
                        {
                            if (RealNumberTrait<ValueType>::is_nan(value))
                            {
                                throw OSPFException{ OSPFErrCode::ApplicationError, "invalid argument NaN for value range" };
                            }
                            std::visit([this, &value](auto& this_value)
                                {
                                    using ThisValueType = OriginType<decltype(this_value)>;
                                    if constexpr (DecaySameAs<ThisValueType, Infinity>)
                                    {
                                        if (RealNumberTrait<ValueType>::is_inf(value))
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid divide between inf and inf" };
                                        }
                                        else if (RealNumberTrait<ValueType>::is_neg_inf(value))
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid divide between inf and -inf" };
                                        }
                                        else if (is_negative(value))
                                        {
                                            this->_variant = Variant{ std::in_place_index<2_uz>, neg_inf };
                                        }
                                        else if (!is_positive(value))
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid divide between inf and 0" };
                                        }
                                    }
                                    else if constexpr (DecaySameAs<ThisValueType, NegativeInfinity>)
                                    {
                                        if (RealNumberTrait<ValueType>::is_inf(value))
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid divide between -inf and inf" };
                                        }
                                        else if (RealNumberTrait<ValueType>::is_neg_inf(value))
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid divide between -inf and -inf" };
                                        }
                                        else if (is_negative(value))
                                        {
                                            this->_variant = Variant{ std::in_place_index<1_uz>, inf };
                                        }
                                        else if (!is_positive(value))
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid divide between -inf and 0" };
                                        }
                                    }
                                    else
                                    {
                                        if (value == ArithmeticTrait<ValueType>::zero())
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, std::format("invalid divide between {}, and 0", this_value) };
                                        }
                                        this_value /= value;
                                    }
                                }, _variant);
                            return *this;
                        }
                        else
                        {
                            std::visit([this, &value](auto& this_value)
                                {
                                    using ThisValueType = OriginType<decltype(this_value)>;
                                    if constexpr (DecaySameAs<ThisValueType, Infinity>)
                                    {
                                        if (is_negative(value))
                                        {
                                            this->_variant = Variant{ std::in_place_index<2_uz>, neg_inf };
                                        }
                                        else if (!is_positive(value))
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid divide between inf and 0" };
                                        }
                                    }
                                    else if constexpr (DecaySameAs<ThisValueType, NegativeInfinity>)
                                    {
                                        if (is_negative(value))
                                        {
                                            this->_variant = Variant{ std::in_place_index<1_uz>, inf };
                                        }
                                        else if (!is_positive(value))
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid divide between -inf and 0"};
                                        }
                                    }
                                    else
                                    {
                                        if (value == ArithmeticTrait<ValueType>::zero())
                                        {
                                            throw OSPFException{ OSPFErrCode::ApplicationError, std::format("invalid divide between {} and 0", this_value) };
                                        }
                                        this_value /= value;
                                    }
                                }, _variant);
                            return *this;
                        }
                    }

                public:
                    template<typename = void>
                        requires AddAssign<ValueType>
                    inline constexpr ValueWrapper& operator++(void) noexcept
                    {
                        return this->operator+=(ArithmeticTrait<ValueType>::one());
                    }

                    template<typename = void>
                        requires AddAssign<ValueType>
                    inline constexpr ValueWrapper& operator++(int) noexcept
                    {
                        return this->operator+=(ArithmeticTrait<ValueType>::one());
                    }

                    template<typename = void>
                        requires SubAssign<ValueType>
                    inline constexpr ValueWrapper& operator--(void) noexcept
                    {
                        return this->operator-=(ArithmeticTrait<ValueType>::one());
                    }

                    template<typename = void>
                        requires SubAssign<ValueType>
                    inline constexpr ValueWrapper& operator--(int) noexcept
                    {
                        return this->operator-=(ArithmeticTrait<ValueType>::one());
                    }

                public:
                    inline constexpr const bool operator==(const ValueWrapper& value) const noexcept
                    {
                        if (_variant.index() == 0_uz && _variant.index() == value._variant.index())
                        {
                            static const Equal<ValueType> eq{};
                            return eq(std::get<0_uz>(_variant), std::get<0_uz>(value._variant));
                        }
                        else if (_variant.index() == 1_uz && _variant.index() == value._variant.index())
                        {
                            return true;
                        }
                        else if (_variant.index() == 2_uz && _variant.index() == value._variant.index())
                        {
                            return true;
                        }
                        else
                        {
                            return false;
                        }
                    }

                    inline constexpr const bool operator!=(const ValueWrapper& value) const noexcept
                    {
                        if (_variant.index() == 0_uz && _variant.index() == value._variant.index())
                        {
                            static const Unequal<ValueType> neq{};
                            return neq(std::get<0_uz>(_variant), std::get<0_uz>(value._variant));
                        }
                        else if (_variant.index() == 1_uz && _variant.index() == value._variant.index())
                        {
                            return false;
                        }
                        else if (_variant.index() == 2_uz && _variant.index() == value._variant.index())
                        {
                            return false;
                        }
                        else
                        {
                            return true;
                        }
                    }

                    inline constexpr const bool operator==(ArgCLRefType<ValueType> value) const noexcept
                    {
                        if constexpr (RealNumber<ValueType>)
                        {
                            if (RealNumberTrait<ValueType>::is_nan(value))
                            {
                                return false;
                            }
                            return std::visit([&value](const auto& this_value)
                                {
                                    using ThisValueType = OriginType<decltype(this_value)>;
                                    if constexpr (DecaySameAs<ThisValueType, Infinity>)
                                    {
                                        return RealNumberTrait<ValueType>::is_inf(value);
                                    }
                                    else if constexpr (DecaySameAs<ThisValueType, NegativeInfinity>)
                                    {
                                        return RealNumberTrait<ValueType>::is_neg_inf(value);
                                    }
                                    else
                                    {
                                        static const Equal<ValueType> eq{};
                                        return eq(this_value, value);
                                    }
                                }, _variant);
                        }
                        else
                        {
                            return std::visit([&value](const auto& this_value)
                                {
                                    using ThisValueType = OriginType<decltype(this_value)>;
                                    if constexpr (DecaySameAs<ThisValueType, Infinity, NegativeInfinity>)
                                    {
                                        return false;
                                    }
                                    else
                                    {
                                        static const Equal<ValueType> eq{};
                                        return eq(this_value, value);
                                    }
                                }, _variant);
                        }
                    }

                    inline constexpr const bool operator!=(ArgCLRefType<ValueType> value) const noexcept
                    {
                        if constexpr (RealNumber<ValueType>)
                        {
                            if (RealNumberTrait<ValueType>::is_nan(value))
                            {
                                return true;
                            }
                            return std::visit([&value](const auto& this_value)
                                {
                                    using ThisValueType = OriginType<decltype(this_value)>;
                                    if constexpr (DecaySameAs<ThisValueType, Infinity>)
                                    {
                                        return !RealNumberTrait<ValueType>::is_inf(value);
                                    }
                                    else if constexpr (DecaySameAs<ThisValueType, NegativeInfinity>)
                                    {
                                        return !RealNumberTrait<ValueType>::is_neg_inf(value);
                                    }
                                    else
                                    {
                                        static const Unequal<ValueType> neq{};
                                        return neq(this_value, value);
                                    }
                                }, _variant);
                        }
                        else
                        {
                            return std::visit([&value](const auto& this_value)
                                {
                                    using ThisValueType = OriginType<decltype(this_value)>;
                                    if constexpr (DecaySameAs<ThisValueType, Infinity, NegativeInfinity>)
                                    {
                                        return true;
                                    }
                                    else
                                    {
                                        static const Unequal<ValueType> neq{};
                                        return neq(this_value, value);
                                    }
                                }, _variant);
                        }
                    }

                public:
                    inline constexpr const bool operator<(const ValueWrapper& value) const noexcept
                    {
                        return std::visit([&value](const auto& lhs)
                            {
                                using LhsValueType = OriginType<decltype(lhs)>;
                                if constexpr (DecaySameAs<LhsValueType, Infinity>)
                                {
                                    return false;
                                }
                                else if constexpr (DecaySameAs<LhsValueType, NegativeInfinity>)
                                {
                                    return std::visit([](const auto& rhs)
                                        {
                                            using RhsValueType = OriginType<decltype(lhs)>;
                                            if constexpr (DecaySameAs<RhsValueType, NegativeInfinity>)
                                            {
                                                return false;
                                            }
                                            else
                                            {
                                                return true;
                                            }
                                        }, value._variant);
                                }
                                else
                                {
                                    return std::visit([&lhs](const auto& rhs)
                                        {
                                            using RhsValueType = OriginType<decltype(rhs)>;
                                            if constexpr (DecaySameAs<RhsValueType, Infinity>)
                                            {
                                                return true;
                                            }
                                            else if constexpr (DecaySameAs<RhsValueType, NegativeInfinity>)
                                            {
                                                return false;
                                            }
                                            else
                                            {
                                                static const Less<ValueType> ls{};
                                                return ls(lhs, rhs);
                                            }
                                        }, value._variant);
                                }
                            }, _variant);
                    }

                    inline constexpr const bool operator<=(const ValueWrapper& value) const noexcept
                    {
                        return std::visit([&value](const auto& lhs) 
                            {
                                using LhsValueType = OriginType<decltype(lhs)>;
                                if constexpr (DecaySameAs<LhsValueType, Infinity>)
                                {
                                    return std::visit([](const auto& rhs)
                                        {
                                            using RhsValueType = OriginType<decltype(rhs)>;
                                            if constexpr (DecaySameAs<RhsValueType, Infinity>)
                                            {
                                                return true;
                                            }
                                            else
                                            {
                                                return false;
                                            }
                                        }, value._variant);
                                }
                                else if constexpr (DecaySameAs<LhsValueType, NegativeInfinity>)
                                {
                                    return true;
                                }
                                else
                                {
                                    return std::visit([&lhs](const auto& rhs)
                                        {
                                            using RhsValueType = OriginType<decltype(rhs)>;
                                            if constexpr (DecaySameAs<RhsValueType, Infinity>)
                                            {
                                                return true;
                                            }
                                            else if constexpr (DecaySameAs<RhsValueType, NegativeInfinity>)
                                            {
                                                return false;
                                            }
                                            else
                                            {
                                                static const LessEqual<ValueType> leq{};
                                                return leq(lhs, rhs);
                                            }
                                        }, value._variant);
                                }
                            }, _variant);
                    }

                    inline constexpr const bool operator>(const ValueWrapper& value) const noexcept
                    {
                        return std::visit([&value](const auto& lhs) 
                            {
                                using LhsValueType = OriginType<decltype(lhs)>;
                                if constexpr (DecaySameAs<LhsValueType, Infinity>)
                                {
                                    return std::visit([](const auto& rhs)
                                        {
                                            using RhsValueType = OriginType<decltype(rhs)>;
                                            if constexpr (DecaySameAs<RhsValueType, Infinity>)
                                            {
                                                return false;
                                            }
                                            else
                                            {
                                                return true;
                                            }
                                        }, value._variant);
                                }
                                else if constexpr (DecaySameAs<LhsValueType, NegativeInfinity>)
                                {
                                    return false;
                                }
                                else
                                {
                                    return std::visit([&lhs](const auto& rhs)
                                        {
                                            using RhsValueType = OriginType<decltype(rhs)>;
                                            if constexpr (DecaySameAs<RhsValueType, Infinity>)
                                            {
                                                return false;
                                            }
                                            else if constexpr (DecaySameAs<RhsValueType, NegativeInfinity>)
                                            {
                                                return true;
                                            }
                                            else
                                            {
                                                static const Greater<ValueType> gr{};
                                                return gr(lhs, rhs);
                                            }
                                        }, value._variant);
                                }
                            }, _variant);
                    }

                    inline constexpr const bool operator>=(const ValueWrapper& value) const noexcept
                    {
                        return std::visit([&value](const auto& lhs) 
                            {
                                using LhsValueType = OriginType<decltype(lhs)>;
                                if constexpr (DecaySameAs<LhsValueType, Infinity>)
                                {
                                    return true;
                                }
                                else if constexpr (DecaySameAs<LhsValueType, NegativeInfinity>)
                                {
                                    return std::visit([](const auto& rhs)
                                        {
                                            using RhsValueType = OriginType<decltype(rhs)>;
                                            if constexpr (DecaySameAs<RhsValueType, NegativeInfinity>)
                                            {
                                                return true;
                                            }
                                            else
                                            {
                                                return false;
                                            }
                                        }, value._variant);
                                }
                                else
                                {
                                    return std::visit([&lhs](const auto& rhs)
                                        {
                                            using RhsValueType = OriginType<decltype(rhs)>;
                                            if constexpr (DecaySameAs<RhsValueType, Infinity>)
                                            {
                                                return false;
                                            }
                                            else if constexpr (DecaySameAs<RhsValueType, NegativeInfinity>)
                                            {
                                                return true;
                                            }
                                            else
                                            {
                                                static const GreaterEqual<ValueType> geq{};
                                                return geq(lhs, rhs);
                                            }
                                        }, value._variant);
                                }
                            }, _variant);
                    }

                    inline constexpr const bool operator<(ArgCLRefType<ValueType> value) const noexcept
                    {
                        if constexpr (RealNumber<ValueType>)
                        {
                            if (RealNumberTrait<ValueType>::is_nan(value))
                            {
                                return false;
                            }
                            return std::visit([&value](const auto& this_value)
                                {
                                    using ThisValueType = OriginType<decltype(this_value)>;
                                    if constexpr (DecaySameAs<ThisValueType, Infinity>)
                                    {
                                        return false;
                                    }
                                    else if constexpr (DecaySameAs<ThisValueType, NegativeInfinity>)
                                    {
                                        return !RealNumberTrait<ValueType>::is_neg_inf(value);
                                    }
                                    else
                                    {
                                        static const Less<ValueType> ls{};
                                        return ls(this_value, value);
                                    }
                                }, _variant);
                        }
                        else
                        {
                            return std::visit([&value](const auto& this_value)
                                {
                                    using ThisValueType = OriginType<decltype(this_value)>;
                                    if constexpr (DecaySameAs<ThisValueType, Infinity>)
                                    {
                                        return false;
                                    }
                                    else if constexpr (DecaySameAs<ThisValueType, NegativeInfinity>)
                                    {
                                        return true;
                                    }
                                    else
                                    {
                                        static const Less<ValueType> ls{};
                                        return ls(this_value, value);
                                    }
                                }, _variant);
                        }
                    }

                    inline constexpr const bool operator<=(ArgCLRefType<ValueType> value) const noexcept
                    {
                        if constexpr (RealNumber<ValueType>)
                        {
                            if (RealNumberTrait<ValueType>::is_nan(value))
                            {
                                return false;
                            }
                            return std::visit([&value](const auto& this_value)
                                {
                                    using ThisValueType = OriginType<decltype(this_value)>;
                                    if constexpr (DecaySameAs<ThisValueType, Infinity>)
                                    {
                                        return RealNumberTrait<ValueType>::is_inf(value);
                                    }
                                    else if constexpr (DecaySameAs<ThisValueType, NegativeInfinity>)
                                    {
                                        return !RealNumberTrait<ValueType>::is_neg_inf(value);
                                    }
                                    else
                                    {
                                        static const LessEqual<ValueType> leq{};
                                        return leq(this_value, value);
                                    }
                                }, _variant);
                        }
                        else
                        {
                            return std::visit([&value](const auto& this_value)
                                {
                                    using ThisValueType = OriginType<decltype(this_value)>;
                                    if constexpr (DecaySameAs<ThisValueType, Infinity>)
                                    {
                                        return false;
                                    }
                                    else if constexpr (DecaySameAs<ThisValueType, NegativeInfinity>)
                                    {
                                        return true;
                                    }
                                    else
                                    {
                                        static const LessEqual<ValueType> leq{};
                                        return leq(this_value, value);
                                    }
                                }, _variant);
                        }
                    }

                    inline constexpr const bool operator>(ArgCLRefType<ValueType> value) const noexcept
                    {
                        if constexpr (RealNumber<ValueType>)
                        {
                            if (RealNumberTrait<ValueType>::is_nan(value))
                            {
                                return false;
                            }
                            return std::visit([&value](const auto& this_value)
                                {
                                    using ThisValueType = OriginType<decltype(this_value)>;
                                    if constexpr (DecaySameAs<ThisValueType, Infinity>)
                                    {
                                        return !RealNumberTrait<ValueType>::is_inf(value);
                                    }
                                    else if constexpr (DecaySameAs<ThisValueType, NegativeInfinity>)
                                    {
                                        return RealNumberTrait<ValueType>::is_neg_inf(value);
                                    }
                                    else
                                    {
                                        static const Greater<ValueType> gr{};
                                        return gr(this_value, value);
                                    }
                                }, _variant);
                        }
                        else
                        {
                            return std::visit([&value](const auto& this_value)
                                {
                                    using ThisValueType = OriginType<decltype(this_value)>;
                                    if constexpr (DecaySameAs<ThisValueType, Infinity>)
                                    {
                                        return true;
                                    }
                                    else if constexpr (DecaySameAs<ThisValueType, NegativeInfinity>)
                                    {
                                        return false;
                                    }
                                    else
                                    {
                                        static const Greater<ValueType> gr{};
                                        return gr(this_value, value);
                                    }
                                }, _variant);
                        }
                    }

                    inline constexpr const bool operator>=(ArgCLRefType<ValueType> value) const noexcept
                    {
                        if constexpr (RealNumber<ValueType>)
                        {
                            if (RealNumberTrait<ValueType>::is_nan(value))
                            {
                                return false;
                            }
                            return std::visit([&value](const auto& this_value)
                                {
                                    using ThisValueType = OriginType<decltype(this_value)>;
                                    if constexpr (DecaySameAs<ThisValueType, Infinity>)
                                    {
                                        return !RealNumberTrait<ValueType>::is_inf(value);
                                    }
                                    else if constexpr (DecaySameAs<ThisValueType, NegativeInfinity>)
                                    {
                                        return RealNumberTrait<ValueType>::is_neg_inf(value);
                                    }
                                    else
                                    {
                                        static const GreaterEqual<ValueType> geq{};
                                        return geq(this_value, value);
                                    }
                                }, _variant);
                        }
                        else
                        {
                            return std::visit([&value](const auto& this_value)
                                {
                                    using ThisValueType = OriginType<decltype(this_value)>;
                                    if constexpr (DecaySameAs<ThisValueType, Infinity>)
                                    {
                                        return true;
                                    }
                                    else if constexpr (DecaySameAs<ThisValueType, NegativeInfinity>)
                                    {
                                        return false;
                                    }
                                    else
                                    {
                                        static const GreaterEqual<ValueType> geq{};
                                        return geq(this_value, value);
                                    }
                                }, _variant);
                        }
                    }

                public:
                    template<typename = void>
                        requires std::three_way_comparable<ValueType>
                    inline constexpr std::compare_three_way_result_t<ValueType> operator<=>(const ValueWrapper& value) const noexcept
                    {
                        using RetType = std::compare_three_way_result_t<ValueType>;

                        return std::visit([&value](const auto& lhs_value) -> RetType 
                            {
                                using LhsValueType = OriginType<decltype(lhs_value)>;
                                if constexpr (DecaySameAs<LhsValueType, Infinity>)
                                {
                                    return std::visit([](const auto& rhs_value) -> RetType
                                        {
                                            using RhsValueType = OriginType<decltype(rhs_value)>;
                                            if constexpr (DecaySameAs<RhsValueType, Infinity>)
                                            {
                                                return std::strong_ordering::equal;
                                            }
                                            else
                                            {
                                                return std::strong_ordering::greater;
                                            }
                                        }, value._variant);
                                }
                                else if constexpr (DecaySameAs<LhsValueType, NegativeInfinity>)
                                {
                                    return std::visit([](const auto& rhs_value) -> RetType
                                        {
                                            using RhsValueType = OriginType<decltype(rhs_value)>;
                                            if constexpr (DecaySameAs<RhsValueType, NegativeInfinity>)
                                            {
                                                return std::strong_ordering::equal;
                                            }
                                            else
                                            {
                                                return std::strong_ordering::less;
                                            }
                                        }, value._variant);
                                }
                                else
                                {
                                     return std::visit([&lhs_value](const auto& rhs_value) -> RetType
                                        {
                                            using RhsValueType = OriginType<decltype(rhs_value)>;
                                            if constexpr (DecaySameAs<RhsValueType, Infinity>)
                                            {
                                                return std::strong_ordering::less;
                                            }
                                            if constexpr (DecaySameAs<RhsValueType, NegativeInfinity>)
                                            {
                                                return std::strong_ordering::greater;
                                            }
                                            else
                                            {
                                                return lhs_value <=> rhs_value;
                                            }
                                        }, value._variant);
                                }
                            }, _variant);
                    }

                    template<typename = void>
                        requires std::three_way_comparable<ValueType>
                    inline constexpr std::compare_three_way_result_t<ValueType> operator<=>(const ValueType& value) const noexcept
                    {
                        using RetType = std::compare_three_way_result_t<ValueType>;

                        if constexpr (RealNumber<ValueType>)
                        {
                            if (RealNumberTrait<ValueType>::is_nan(value))
                            {
                                return std::partial_ordering::unordered;
                            }
                            return std::visit([&value](const auto& this_value) -> RetType
                                {
                                    using ThisValueType = OriginType<decltype(this_value)>;
                                    if constexpr (DecaySameAs<ThisValueType, Infinity>)
                                    {
                                        return RealNumberTrait<ValueType>::is_inf(value) ? std::strong_ordering::equal : std::strong_ordering::greater;
                                    }
                                    else if constexpr (DecaySameAs<ThisValueType, NegativeInfinity>)
                                    {
                                        return RealNumberTrait<ValueType>::is_neg_inf(value) ? std::strong_ordering::equal : std::strong_ordering::less;
                                    }
                                    else
                                    {
                                        return this_value <=> value;
                                    }
                                }, _variant);
                        }
                        else
                        {
                            return std::visit([&value](const auto& this_value) -> RetType
                                {
                                    using ThisValueType = OriginType<decltype(this_value)>;
                                    if constexpr (DecaySameAs<ThisValueType, Infinity>)
                                    {
                                        return std::strong_ordering::greater;
                                    }
                                    else if constexpr (DecaySameAs<ThisValueType, NegativeInfinity>)
                                    {
                                        return std::strong_ordering::less;
                                    }
                                    else
                                    {
                                        return this_value <=> value;
                                    }
                                }, _variant);
                        }
                    }

                public:
                    inline void swap(ValueWrapper& ano) noexcept
                    {
                        std::swap(_variant, ano._variant);
                    }

                private:
                    Variant _variant;
                };

                extern template class ValueWrapper<i8>;
                extern template class ValueWrapper<u8>;
                extern template class ValueWrapper<i16>;
                extern template class ValueWrapper<u16>;
                extern template class ValueWrapper<i32>;
                extern template class ValueWrapper<u32>;
                extern template class ValueWrapper<i64>;
                extern template class ValueWrapper<u64>;
                //extern template class ValueWrapper<i128>;
                //extern template class ValueWrapper<u128>;
                //extern template class ValueWrapper<i256>;
                //extern template class ValueWrapper<u256>;
                //extern template class ValueWrapper<i512>;
                //extern template class ValueWrapper<u512>;
                //extern template class ValueWrapper<i1024>;
                //extern template class ValueWrapper<u1024>;
                //extern template class ValueWrapper<bigint>;

                extern template class ValueWrapper<f32>;
                extern template class ValueWrapper<f64>;
                //extern template class ValueWrapper<f128>;
                //extern template class ValueWrapper<f256>;
                //extern template class ValueWrapper<f512>;
                //extern template class ValueWrapper<dec50>;
                //extern template class ValueWrapper<dec100>;
            };
        };
    };
};

template<ospf::Invariant T>
    requires ospf::Add<T>
inline constexpr ospf::RetType<ospf::value_range::ValueWrapper<T>> operator+(const T& lhs, const ospf::value_range::ValueWrapper<T>& rhs)
{
    if constexpr (ospf::RealNumber<T>)
    {
        if (ospf::RealNumberTrait<T>::is_nan(lhs))
        {
            throw ospf::OSPFException{ ospf::OSPFErrCode::ApplicationError, "invalid argument NaN for value range" };
        }
        return std::visit([&lhs](const auto& rhs_value) -> ospf::value_range::ValueWrapper<T>
            {
                using RhsValueType = ospf::OriginType<decltype(rhs_value)>;
                if constexpr (ospf::DecaySameAs<RhsValueType, ospf::Infinity>)
                {
                    if (ospf::RealNumberTrait<T>::is_neg_inf(lhs))
                    {
                        throw ospf::OSPFException{ ospf::OSPFErrCode::ApplicationError, "invalid plus between inf and -inf" };
                    }
                    return ospf::inf;
                }
                else if constexpr (ospf::DecaySameAs<RhsValueType, ospf::NegativeInfinity>)
                {
                    if (ospf::RealNumberTrait<T>::is_inf(lhs))
                    {
                        throw ospf::OSPFException{ ospf::OSPFErrCode::ApplicationError, "invalid plus between inf and -inf" };
                    }
                    return ospf::neg_inf;
                }
                else
                {
                    return lhs + rhs_value;
                }
            }, rhs);
    }
    else
    {
        return std::visit([&lhs](const auto& rhs_value) -> ospf::value_range::ValueWrapper<T>
            {
                using RhsValueType = ospf::OriginType<decltype(rhs_value)>;
                if constexpr (ospf::DecaySameAs<RhsValueType, ospf::Infinity>)
                {
                    return ospf::inf;
                }
                else if constexpr (ospf::DecaySameAs<RhsValueType, ospf::NegativeInfinity>)
                {
                    return ospf::neg_inf;
                }
                else
                {
                    return lhs + rhs_value;
                }
            }, rhs);
    }
}

template<ospf::Invariant T>
    requires ospf::Sub<T>
inline constexpr ospf::RetType<ospf::value_range::ValueWrapper<T>> operator-(const T& lhs, const ospf::value_range::ValueWrapper<T>& rhs)
{
    if constexpr (ospf::RealNumber<T>)
    {
        if (ospf::RealNumberTrait<T>::is_nan(lhs))
        {
            throw ospf::OSPFException{ ospf::OSPFErrCode::ApplicationError, "invalid argument NaN for value range" };
        }
        return std::visit([&lhs](const auto& rhs_value) -> ospf::value_range::ValueWrapper<T>
            {
                using RhsValueType = ospf::OriginType<decltype(rhs_value)>;
                if constexpr (ospf::DecaySameAs<RhsValueType, ospf::Infinity>)
                {
                    if (ospf::RealNumberTrait<T>::is_inf(lhs))
                    {
                        throw ospf::OSPFException{ ospf::OSPFErrCode::ApplicationError, "invalid minus between inf and inf" };
                    }
                    return ospf::neg_inf;
                }
                else if constexpr (ospf::DecaySameAs<RhsValueType, ospf::NegativeInfinity>)
                {
                    if (ospf::RealNumberTrait<T>::is_neg_inf(lhs))
                    {
                        throw ospf::OSPFException{ ospf::OSPFErrCode::ApplicationError, "invalid minus between -inf and -inf" };
                    }
                    return ospf::inf;
                }
                else
                {
                    return lhs - rhs_value;
                }
            }, rhs);
    }
    else
    {
        return std::visit([&lhs](const auto& rhs_value) -> ospf::value_range::ValueWrapper<T>
            {
                using RhsValueType = ospf::OriginType<decltype(rhs_value)>;
                if constexpr (ospf::DecaySameAs<RhsValueType, ospf::Infinity>)
                {
                    return ospf::neg_inf;
                }
                else if constexpr (ospf::DecaySameAs<RhsValueType, ospf::NegativeInfinity>)
                {
                    return ospf::inf;
                }
                else
                {
                    return lhs - rhs_value;
                }
            }, rhs);
    }
}

template<ospf::Invariant T>
    requires ospf::Mul<T>
inline constexpr ospf::RetType<ospf::value_range::ValueWrapper<T>> operator*(const T& lhs, const ospf::value_range::ValueWrapper<T>& rhs)
{
    if constexpr (ospf::RealNumber<T>)
    {
        if (ospf::RealNumberTrait<T>::is_nan(lhs))
        {
            throw ospf::OSPFException{ ospf::OSPFErrCode::ApplicationError, "invalid argument NaN for value range" };
        }
        return std::visit([&lhs](const auto& rhs_value) -> ospf::value_range::ValueWrapper<T>
            {
                using ThisValueType = ospf::OriginType<decltype(rhs_value)>;
                if constexpr (ospf::DecaySameAs<ThisValueType, ospf::Infinity>)
                {
                    if (ospf::is_positive(lhs))
                    {
                        return ospf::inf;
                    }
                    else if (ospf::is_negative(lhs))
                    {
                        return ospf::neg_inf;
                    }
                    else
                    {
                        throw ospf::OSPFException{ ospf::OSPFErrCode::ApplicationError, "invalid multiply between 0 and inf" };
                        return ospf::ArithmeticTrait<T>::zero();
                    }
                }
                else if constexpr (ospf::DecaySameAs<ThisValueType, ospf::NegativeInfinity>)
                {
                    if (ospf::is_positive(lhs))
                    {
                        return ospf::neg_inf;
                    }
                    else if (ospf::is_negative(lhs))
                    {
                        return ospf::inf;
                    }
                    else
                    {
                        throw ospf::OSPFException{ ospf::OSPFErrCode::ApplicationError, "invalid multiply between 0 and -inf" };
                        return ospf::ArithmeticTrait<T>::zero();
                    }
                }
                else
                {
                    if (ospf::RealNumberTrait<T>::is_inf(lhs))
                    {
                        if (ospf::is_positive(rhs_value))
                        {
                            return ospf::inf;
                        }
                        else if (ospf::is_negative(rhs_value))
                        {
                            return ospf::neg_inf;
                        }
                        else
                        {
                            throw ospf::OSPFException{ ospf::OSPFErrCode::ApplicationError, "invalid multiply between 0 and inf" };
                            return ospf::ArithmeticTrait<T>::zero();
                        }
                    }
                    else if (ospf::RealNumberTrait<T>::is_neg_inf(lhs))
                    {
                        if (ospf::is_positive(rhs_value))
                        {
                            return ospf::neg_inf;
                        }
                        else if (ospf::is_negative(rhs_value))
                        {
                            return ospf::inf;
                        }
                        else
                        {
                            throw ospf::OSPFException{ ospf::OSPFErrCode::ApplicationError, "invalid multiply between 0 and -inf" };
                            return ospf::ArithmeticTrait<T>::zero();
                        }
                    }
                    else
                    {
                        return lhs * rhs_value;
                    }
                }
            }, rhs);
    }
    else
    {
        return std::visit([&lhs](const auto& rhs_value) -> ospf::value_range::ValueWrapper<T>
            {
                using RhsValueType = ospf::OriginType<decltype(rhs_value)>;
                if constexpr (ospf::DecaySameAs<RhsValueType, ospf::Infinity>)
                {
                    if (ospf::is_positive(lhs))
                    {
                        return ospf::inf;
                    }
                    else if (ospf::is_negative(lhs))
                    {
                        return ospf::neg_inf;
                    }
                    else
                    {
                        throw ospf::OSPFException{ ospf::OSPFErrCode::ApplicationError, "invalid multiply between 0 and inf" };
                        return ospf::ArithmeticTrait<T>::zero();
                    }
                }
                else if constexpr (ospf::DecaySameAs<RhsValueType, ospf::NegativeInfinity>)
                {
                    if (ospf::is_positive(lhs))
                    {
                        return ospf::neg_inf;
                    }
                    else if (ospf::is_negative(lhs))
                    {
                        return ospf::inf;
                    }
                    else
                    {
                        throw ospf::OSPFException{ ospf::OSPFErrCode::ApplicationError, "invalid multiply between 0 and -inf" };
                        return ospf::ArithmeticTrait<T>::zero();
                    }
                }
                else
                {
                    return lhs * rhs_value;
                }
            }, rhs);
    }
}

template<ospf::Invariant T>
    requires ospf::Div<T>
inline constexpr ospf::RetType<ospf::value_range::ValueWrapper<T>> operator/(const T& lhs, const ospf::value_range::ValueWrapper<T>& rhs)
{
    if constexpr (ospf::RealNumber<T>)
    {
        if (ospf::RealNumberTrait<T>::is_nan(lhs))
        {
            throw ospf::OSPFException{ ospf::OSPFErrCode::ApplicationError, "invalid argument NaN for value range" };
        }
        return std::visit([&lhs](const auto& rhs_value) -> ospf::value_range::ValueWrapper<T>
            {
                using RhsValueType = ospf::OriginType<decltype(rhs_value)>;
                if constexpr (ospf::DecaySameAs<RhsValueType, ospf::Infinity>)
                {
                    if (ospf::RealNumberTrait<T>::is_inf(lhs))
                    {
                        throw ospf::OSPFException{ ospf::OSPFErrCode::ApplicationError, "invalid divide between inf and inf" };
                        return ospf::ArithmeticTrait<T>::zero();
                    }
                    else if (ospf::RealNumberTrait<T>::is_neg_inf(lhs))
                    {
                        throw ospf::OSPFException{ ospf::OSPFErrCode::ApplicationError, "invalid divide between -inf and inf" };
                        return ospf::ArithmeticTrait<T>::zero();
                    }
                    else
                    {
                        return ospf::ArithmeticTrait<T>::zero();
                    }
                }
                else if constexpr (ospf::DecaySameAs<RhsValueType, ospf::NegativeInfinity>)
                {
                    if (ospf::RealNumberTrait<T>::is_inf(lhs))
                    {
                        throw ospf::OSPFException{ ospf::OSPFErrCode::ApplicationError, "invalid divide between inf and -inf" };
                        return ospf::ArithmeticTrait<T>::zero();
                    }
                    else if (ospf::RealNumberTrait<T>::is_neg_inf(lhs))
                    {
                        throw ospf::OSPFException{ ospf::OSPFErrCode::ApplicationError, "invalid divide between -inf and -inf" };
                        return ospf::ArithmeticTrait<T>::zero();
                    }
                    else
                    {
                        return ospf::ArithmeticTrait<T>::zero();
                    }
                }
                else
                {
                    if (ospf::RealNumberTrait<T>::is_inf(lhs))
                    {
                        if (ospf::is_positive(rhs_value))
                        {
                            return ospf::inf;
                        }
                        else if (ospf::is_negative(rhs_value))
                        {
                            return ospf::neg_inf;
                        }
                        else
                        {
                            throw ospf::OSPFException{ ospf::OSPFErrCode::ApplicationError, "invalid divide between inf and 0" };
                            return ospf::ArithmeticTrait<T>::zero();
                        }
                    }
                    else if (ospf::RealNumberTrait<T>::is_neg_inf(lhs))
                    {
                        if (ospf::is_positive(rhs_value))
                        {
                            return ospf::neg_inf;
                        }
                        else if (ospf::is_negative(rhs_value))
                        {
                            return ospf::inf;
                        }
                        else
                        {
                            throw ospf::OSPFException{ ospf::OSPFErrCode::ApplicationError, "invalid divide between -inf and 0" };
                            return ospf::ArithmeticTrait<T>::zero();
                        }
                    }
                    else
                    {
                        if (rhs_value == ospf::ArithmeticTrait<T>::zero())
                        {
                            throw ospf::OSPFException{ ospf::OSPFErrCode::ApplicationError, std::format("invalid divide between {} and 0", lhs) };
                            return ospf::ArithmeticTrait<T>::zero();
                        }
                        return lhs / rhs_value;
                    }
                }
            }, rhs);
    }
    else
    {
        return std::visit([&lhs](const auto& rhs_value) -> ospf::value_range::ValueWrapper<T>
            {
                using RhsValueType = ospf::OriginType<decltype(rhs_value)>;
                if constexpr (ospf::DecaySameAs<RhsValueType, ospf::Infinity, ospf::NegativeInfinity>)
                {
                    return ospf::ArithmeticTrait<T>::zero();
                }
                else
                {
                    if (rhs_value == ospf::ArithmeticTrait<T>::zero())
                    {
                        throw ospf::OSPFException{ ospf::OSPFErrCode::ApplicationError, std::format("invalid divide between {} and 0", lhs) };
                        return ospf::ArithmeticTrait<T>::zero();
                    }
                    return lhs / rhs_value;
                }
            }, rhs);
    }
}

template<ospf::Invariant T>
inline constexpr const bool operator==(const T& lhs, const ospf::value_range::ValueWrapper<T>& rhs) noexcept
{
    if constexpr (ospf::RealNumber<T>)
    {
        if (ospf::RealNumberTrait<T>::is_nan(lhs))
        {
            return false;
        }
        return std::visit([&lhs](const auto& rhs_value)
            {
                using RhsValueType = ospf::OriginType<decltype(rhs_value)>;
                if constexpr (ospf::DecaySameAs<RhsValueType, ospf::Infinity>)
                {
                    return ospf::RealNumberTrait<T>::is_inf(lhs);
                }
                else if constexpr (ospf::DecaySameAs<RhsValueType, ospf::NegativeInfinity>)
                {
                    return ospf::RealNumberTrait<T>::is_neg_inf(lhs);
                }
                else
                {
                    static const ospf::Equal<T> eq{};
                    return eq(lhs, rhs_value);
                }
            }, rhs);
    }
    else
    {
        return std::visit([&lhs](const auto& rhs_value)
            {
                using RhsValueType = ospf::OriginType<decltype(rhs_value)>;
                if constexpr (ospf::DecaySameAs<RhsValueType, ospf::Infinity, ospf::NegativeInfinity>)
                {
                    return false;
                }
                else
                {
                    static const ospf::Equal<T> eq{};
                    return eq(lhs, rhs_value);
                }
            }, rhs);
    }
}

template<ospf::Invariant T>
inline constexpr const bool operator!=(const T& lhs, const ospf::value_range::ValueWrapper<T>& rhs) noexcept
{
    if constexpr (ospf::RealNumber<T>)
    {
        if (ospf::RealNumberTrait<T>::is_nan(lhs))
        {
            return true;
        }
        return std::visit([&lhs](const auto& rhs_value)
            {
                using RhsValueType = ospf::OriginType<decltype(rhs_value)>;
                if constexpr (ospf::DecaySameAs<RhsValueType, ospf::Infinity>)
                {
                    return !ospf::RealNumberTrait<T>::is_inf(lhs);
                }
                else if constexpr (ospf::DecaySameAs<RhsValueType, ospf::NegativeInfinity>)
                {
                    return !ospf::RealNumberTrait<T>::is_neg_inf(lhs);
                }
                else
                {
                    static const ospf::Unequal<T> neq{};
                    return neq(lhs, rhs_value);
                }
            }, rhs);
    }
    else
    {
        return std::visit([&lhs](const auto& rhs_value)
            {
                using RhsValueType = ospf::OriginType<decltype(rhs_value)>;
                if constexpr (ospf::DecaySameAs<RhsValueType, ospf::Infinity, ospf::NegativeInfinity>)
                {
                    return true;
                }
                else
                {
                    static const ospf::Unequal<T> neq{};
                    return neq(lhs, rhs_value);
                }
            }, rhs);
    }
}

template<ospf::Invariant T>
inline constexpr const bool operator<(const T& lhs, const ospf::value_range::ValueWrapper<T>& rhs) noexcept
{
    if constexpr (ospf::RealNumber<T>)
    {
        if (ospf::RealNumberTrait<T>::is_nan(lhs))
        {
            return false;
        }
        return std::visit([&lhs](const auto& rhs_value) 
            {
                using RhsValueType = ospf::OriginType<decltype(rhs_value)>;
                if constexpr (ospf::DecaySameAs<RhsValueType, ospf::Infinity>)
                {
                    return !ospf::RealNumberTrait<T>::is_inf(lhs);
                }
                else if constexpr (ospf::DecaySameAs<RhsValueType, ospf::NegativeInfinity>)
                {
                    return false;
                }
                else
                {
                    static const ospf::Less<T> ls{};
                    return ls(lhs, rhs_value);
                }
            }, rhs);
    }
    else
    {
        return std::visit([&lhs](const auto& rhs_value)
            {
                using RhsValueType = ospf::OriginType<decltype(rhs_value)>;
                if constexpr (ospf::DecaySameAs<RhsValueType, ospf::Infinity>)
                {
                    return true;
                }
                else if constexpr (ospf::DecaySameAs<RhsValueType, ospf::NegativeInfinity>)
                {
                    return false;
                }
                else
                {
                    static const ospf::Less<T> ls{};
                    return ls(lhs, rhs_value);
                }
            }, rhs);
    }
}

template<ospf::Invariant T>
inline constexpr const bool operator<=(const T& lhs, const ospf::value_range::ValueWrapper<T>& rhs) noexcept
{
    if constexpr (ospf::RealNumber<T>)
    {
        if (ospf::RealNumberTrait<T>::is_nan(lhs))
        {
            return false;
        }
        return std::visit([&lhs](const auto& rhs_value) 
            {
                using RhsValueType = ospf::OriginType<decltype(rhs_value)>;
                if constexpr (ospf::DecaySameAs<RhsValueType, ospf::Infinity>)
                {
                    return !ospf::RealNumberTrait<T>::is_inf(lhs);
                }
                else if constexpr (ospf::DecaySameAs<RhsValueType, ospf::NegativeInfinity>)
                {
                    return ospf::RealNumberTrait<T>::is_neg_inf(lhs);
                }
                else
                {
                    static const ospf::LessEqual<T> leq{};
                    return leq(lhs, rhs_value);
                }
            }, rhs);
    }
    else
    {
        return std::visit([&lhs](const auto& rhs_value)
            {
                using RhsValueType = ospf::OriginType<decltype(rhs_value)>;
                if constexpr (ospf::DecaySameAs<RhsValueType, ospf::Infinity>)
                {
                    return true;
                }
                else if constexpr (ospf::DecaySameAs<RhsValueType, ospf::NegativeInfinity>)
                {
                    return false;
                }
                else
                {
                    static const ospf::LessEqual<T> leq{};
                    return leq(lhs, rhs_value);
                }
            }, rhs);
    }
}

template<ospf::Invariant T>
inline constexpr const bool operator>(const T& lhs, const ospf::value_range::ValueWrapper<T>& rhs) noexcept
{
    if constexpr (ospf::RealNumber<T>)
    {
        if (ospf::RealNumberTrait<T>::is_nan(lhs))
        {
            return false;
        }
        return std::visit([&lhs](const auto& rhs_value) 
            {
                using RhsValueType = ospf::OriginType<decltype(rhs_value)>;
                if constexpr (ospf::DecaySameAs<RhsValueType, ospf::Infinity>)
                {
                    return false;
                }
                else if constexpr (ospf::DecaySameAs<RhsValueType, ospf::NegativeInfinity>)
                {
                    return !ospf::RealNumberTrait<T>::is_neg_inf(lhs);
                }
                else
                {
                    static const ospf::Greater<T> gr{};
                    return gr(lhs, rhs_value);
                }
            }, rhs);
    }
    else
    {
        return std::visit([&lhs](const auto& rhs_value)
            {
                using RhsValueType = ospf::OriginType<decltype(rhs_value)>;
                if constexpr (ospf::DecaySameAs<RhsValueType, ospf::Infinity>)
                {
                    return false;
                }
                else if constexpr (ospf::DecaySameAs<RhsValueType, ospf::NegativeInfinity>)
                {
                    return true;
                }
                else
                {
                    static const ospf::Greater<T> gr{};
                    return gr(lhs, rhs_value);
                }
            }, rhs);
    }
}

template<ospf::Invariant T>
inline constexpr const bool operator>=(const T& lhs, const ospf::value_range::ValueWrapper<T>& rhs) noexcept
{
    if constexpr (ospf::RealNumber<T>)
    {
        if (ospf::RealNumberTrait<T>::is_nan(lhs))
        {
            return false;
        }
        return std::visit([&lhs](const auto& rhs_value) 
            {
                using RhsValueType = ospf::OriginType<decltype(rhs_value)>;
                if constexpr (ospf::DecaySameAs<RhsValueType, ospf::Infinity>)
                {
                    return ospf::RealNumberTrait<T>::is_inf(lhs);
                }
                else if constexpr (ospf::DecaySameAs<RhsValueType, ospf::NegativeInfinity>)
                {
                    return !ospf::RealNumberTrait<T>::is_neg_inf(lhs);
                }
                else
                {
                    static const ospf::GreaterEqual<T> geq{};
                    return geq(lhs, rhs_value);
                }
            }, rhs);
    }
    else
    {
        return std::visit([&lhs](const auto& rhs_value)
            {
                using RhsValueType = ospf::OriginType<decltype(rhs_value)>;
                if constexpr (ospf::DecaySameAs<RhsValueType, ospf::Infinity>)
                {
                    return false;
                }
                else if constexpr (ospf::DecaySameAs<RhsValueType, ospf::NegativeInfinity>)
                {
                    return true;
                }
                else
                {
                    static const ospf::GreaterEqual<T> geq{};
                    return geq(lhs, rhs_value);
                }
            }, rhs);
    }
}

template<ospf::Invariant T>
    requires std::three_way_comparable<T>
inline constexpr std::compare_three_way_result_t<T> operator<=>(const T& lhs, const ospf::value_range::ValueWrapper<T>& rhs) noexcept
{
    using RetType = std::compare_three_way_result_t<T>;

    if constexpr (ospf::RealNumber<T>)
    {
        if (ospf::RealNumberTrait<T>::is_nan(lhs))
        {
            return std::partial_ordering::unordered;
        }
        return std::visit([&lhs](const auto& rhs_value) -> RetType
            {
                using RhsValueType = ospf::OriginType<decltype(rhs_value)>;
                if constexpr (ospf::DecaySameAs<RhsValueType, ospf::Infinity>)
                {
                    return ospf::RealNumberTrait<T>::is_inf(lhs) ? std::strong_ordering::equal : std::strong_ordering::less;
                }
                else if constexpr (ospf::DecaySameAs<RhsValueType, ospf::NegativeInfinity>)
                {
                    return ospf::RealNumberTrait<T>::is_neg_inf(lhs) ? std::strong_ordering::equal : std::strong_ordering::greater;
                }
                else
                {
                    return lhs <=> rhs_value;
                }
            }, rhs);
    }
    else
    {
        return std::visit([&lhs](const auto& rhs_value) -> RetType
            {
                using RhsValueType = ospf::OriginType<decltype(rhs_value)>;
                if constexpr (ospf::DecaySameAs<RhsValueType, ospf::Infinity>)
                {
                    return std::strong_ordering::less;
                }
                else if constexpr (ospf::DecaySameAs<RhsValueType, ospf::NegativeInfinity>)
                {
                    return std::strong_ordering::greater;
                }
                else
                {
                    return lhs <=> rhs_value;
                }
            }, rhs);
    }
}

namespace std
{
    template<ospf::Invariant T>
    inline void swap(ospf::value_range::ValueWrapper<T>& lhs, ospf::value_range::ValueWrapper<T>& rhs) noexcept
    {
        lhs.swap(rhs);
    }

    template<ospf::Invariant T, ospf::CharType CharT>
    struct formatter<ospf::value_range::ValueWrapper<T>, CharT>
        : public formatter<std::basic_string_view<CharT>, CharT>
    {
        template<typename FormatContext>
        inline decltype(auto) format(ospf::ArgCLRefType<ospf::value_range::ValueWrapper<T>> value, FormatContext& fc) const
        {
            return std::visit([](const auto& value)
                {
                    static const formatter<ospf::OriginType<decltype(value)>, CharT> _formatter{};
                    return _formatter.format(value, fc);
                }, value);
        }
    };
};

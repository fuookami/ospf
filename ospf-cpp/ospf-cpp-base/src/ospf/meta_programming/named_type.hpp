#pragma once

#include <ospf/concepts/base.hpp>
#include <ospf/concepts/with_default.hpp>
#include <ospf/concepts/with_tag.hpp>
#include <ospf/type_family.hpp>

namespace ospf
{
    inline namespace meta_programming
    {
        template<typename T, typename>
        class NamedType
        {
        public:
            using ValueType = OriginType<T>;

        public:
            template<typename = void>
                requires WithDefault<T>
            constexpr NamedType(void)
                : _value(DefaultValue<T>::value()) {}

            constexpr NamedType(ArgCLRefType<ValueType> value)
                : _value(value) {}

            template<typename = void>
                requires ReferenceFaster<ValueType> && std::movable<ValueType>
            constexpr NamedType(ArgRRefType<ValueType> value)
                : _value(move<ValueType>(value)) {}

        public:
            constexpr NamedType(const NamedType& ano) = default;
            constexpr NamedType(NamedType&& ano) noexcept = default;
            constexpr NamedType& operator=(const NamedType& rhs) = default;
            constexpr NamedType& operator=(NamedType&& rhs) noexcept = default;

            constexpr NamedType& operator=(ArgCLRefType<ValueType> rhs)
            {
                _value = rhs;
            }

            template<typename = void>
                requires ReferenceFaster<ValueType> && std::movable<ValueType>
            constexpr NamedType& operator=(ArgRRefType<ValueType> rhs) noexcept
            {
                _value = move<ValueType>(rhs);
            }

        public:
            constexpr ~NamedType(void) noexcept = default;

        public:
            inline constexpr ValueType& unwrap(void) & noexcept
            {
                return _value;
            }

            inline constexpr const ValueType& unwrap(void) const & noexcept
            {
                return _value;
            }

            inline constexpr ValueType unwrap(void) && noexcept
            {
                return std::move(_value);
            }

        public:
            template<typename = void>
                requires requires (const ValueType& value) { { +value } -> DecaySameAs<ValueType>; }
            inline constexpr NamedType operator+(void) const noexcept
            {
                return NamedType{ +_value };
            }

            template<typename = void>
                requires requires (const ValueType& value) { { -value } -> DecaySameAs<ValueType>; }
            inline constexpr NamedType operator-(void) const noexcept
            {
                return NamedType{ -_value };
            }

            template<typename = void>
                requires requires (const ValueType& value) { { *value } -> NotVoidType; }
            inline constexpr decltype(auto) operator*(void) const noexcept
            {
                return *_value;
            }

            template<typename = void>
                requires requires (const ValueType& value) { { &value } -> DecaySameAs<ptraddr>; }
            inline constexpr const ptraddr operator&(void) const noexcept
            {
                return &_value;
            }

        public:
            template<typename = void>
                requires requires (const ValueType& lhs, const ValueType& rhs) { { lhs && rhs } -> NotVoidType; }
            inline constexpr decltype(auto) operator&&(const NamedType& rhs) const noexcept
            {
                return _value && rhs._value;
            }

            template<typename = void>
                requires requires (const ValueType& lhs, const ValueType& rhs) { { lhs || rhs } -> NotVoidType; }
            inline constexpr decltype(auto) operator||(const NamedType& rhs) const noexcept
            {
                return _value || rhs._value;
            }

            template<typename = void>
                requires requires (const ValueType& value) { { !value } -> NotVoidType; }
            inline constexpr decltype(auto) operator!(void) const noexcept
            {
                return !_value;
            }

        public:
            template<typename = void>
                requires requires (const ValueType& value) { { ~value } -> DecaySameAs<ValueType>; }
            inline constexpr NamedType operator~(void) const noexcept
            {
                return NamedType{ ~_value };
            }

            template<typename = void>
                requires requires (const ValueType& lhs, const ValueType& rhs) { { lhs& rhs } -> DecaySameAs<ValueType>; }
            inline constexpr NamedType operator&(const NamedType& rhs) const noexcept
            {
                return NamedType{ _value & rhs._value };
            }

            template<typename = void>
                requires requires (ValueType& lhs, const ValueType& rhs) { { lhs &= rhs } -> DecaySameAs<ValueType>; }
            inline constexpr NamedType& operator&=(const NamedType& rhs) noexcept
            {
                _value &= rhs._value;
                return *this;
            }

            template<typename = void>
                requires requires (const ValueType& lhs, const ValueType& rhs) { { lhs | rhs } -> DecaySameAs<ValueType>; }
            inline constexpr NamedType operator|(const NamedType& rhs) const noexcept
            {
                return NamedType{ _value | rhs._value };
            }

            template<typename = void>
                requires requires (ValueType& lhs, const ValueType& rhs) { { lhs |= rhs } -> DecaySameAs<ValueType>; }
            inline constexpr NamedType& operator|=(const NamedType& rhs) noexcept
            {
                _value |= rhs._value;
                return *this;
            }

            template<typename = void>
                requires requires (const ValueType& lhs, const ValueType& rhs) { { lhs ^ rhs } -> DecaySameAs<ValueType>; }
            inline constexpr NamedType operator^(const NamedType& rhs) const noexcept
            {
                return NamedType{ _value ^ rhs._value };
            }

            template<typename = void>
                requires requires (ValueType& lhs, const ValueType& rhs) { { lhs ^= rhs } -> DecaySameAs<ValueType>; }
            inline constexpr NamedType& operator^=(const NamedType& rhs) noexcept
            {
                _value ^= rhs._value;
                return *this;
            }

            template<typename = void>
                requires requires (const ValueType& lhs, const usize rhs) { { lhs << rhs } -> DecaySameAs<ValueType>; }
            inline constexpr NamedType operator<<(const usize rhs) const noexcept
            {
                return NamedType{ _value << rhs };
            }

            template<typename = void>
                requires requires (ValueType& lhs, const usize rhs) { { lhs <<= rhs } -> DecaySameAs<ValueType>; }
            inline constexpr NamedType& operator<<=(const usize len) noexcept
            {
                _value <<= len;
                return *this;
            }

            template<typename = void>
                requires requires (const ValueType& lhs, const usize rhs) { { lhs >> rhs } -> DecaySameAs<ValueType>; }
            inline constexpr NamedType operator>>(const usize rhs) const noexcept
            {
                return NamedType{ _value >> rhs };
            }

            template<typename = void>
                requires requires (ValueType& lhs, const usize rhs) { { lhs >>= rhs } -> DecaySameAs<ValueType>; }
            inline constexpr NamedType& operator>>=(const usize rhs) noexcept
            {
                _value >>= rhs;
                return *this;
            }

        public:
            template<typename = void>
                requires requires (const ValueType& lhs, const ValueType& rhs) { { lhs + rhs } -> DecaySameAs<ValueType>; }
            inline constexpr NamedType operator+(const NamedType& rhs) const noexcept
            {
                return NamedType{ _value + rhs._value };
            }

            template<typename = void>
                requires requires (ValueType& lhs, const ValueType& rhs) { { lhs += rhs } -> DecaySameAs<ValueType>; }
            inline constexpr NamedType& operator+=(const NamedType& rhs) noexcept
            {
                _value += rhs._value;
                return *this;
            }

            template<typename = void>
                requires requires (const ValueType& lhs, const ValueType& rhs) { { lhs - rhs } -> DecaySameAs<ValueType>; }
            inline constexpr NamedType operator-(const NamedType& rhs) const noexcept
            {
                return NamedType{ _value - rhs._value };
            }

            template<typename = void>
                requires requires (ValueType& lhs, const ValueType& rhs) { { lhs -= rhs } -> DecaySameAs<ValueType>; }
            inline constexpr NamedType& operator-=(const NamedType& rhs) noexcept
            {
                _value -= rhs._value;
                return *this;
            }

            template<typename = void>
                requires requires (const ValueType& lhs, const ValueType& rhs) { { lhs * rhs } -> DecaySameAs<ValueType>; }
            inline constexpr NamedType operator*(const NamedType& rhs) const noexcept
            {
                return NamedType{ _value * rhs._value };
            }

            template<typename = void>
                requires requires (ValueType& lhs, const ValueType& rhs) { { lhs *= rhs } -> DecaySameAs<ValueType>; }
            inline constexpr NamedType& operator*=(const NamedType& rhs) noexcept
            {
                _value *= rhs._value;
                return *this;
            }

            template<typename = void>
                requires requires (const ValueType& lhs, const ValueType& rhs) { { lhs / rhs } -> DecaySameAs<ValueType>; }
            inline constexpr NamedType operator/(const NamedType& rhs) const noexcept
            {
                return NamedType{ _value / rhs._value };
            }

            template<typename = void>
                requires requires (ValueType& lhs, const ValueType& rhs) { { lhs /= rhs } -> DecaySameAs<ValueType>; }
            inline constexpr NamedType& operator/=(const NamedType& rhs) noexcept
            {
                _value /= rhs._value;
                return *this;
            }

            template<typename = void>
                requires requires (const ValueType& lhs, const ValueType& rhs) { { lhs % rhs } -> DecaySameAs<ValueType>; }
            inline constexpr NamedType operator%(const NamedType& rhs) const noexcept
            {
                return NamedType{ _value % rhs._value };
            }

            template<typename = void>
                requires requires (ValueType& lhs, const ValueType& rhs) { { lhs %= rhs } -> DecaySameAs<ValueType>; }
            inline constexpr NamedType& operator%=(const NamedType& rhs) noexcept
            {
                _value %= rhs._value;
                return *this;
            }

        public:
            template<typename = void>
                requires requires (ValueType& value) { { ++value } -> DecaySameAs<ValueType>; }
            inline constexpr NamedType& operator++(void) noexcept
            {
                ++_value;
                return *this;
            }
            
            template<typename = void>
                requires requires (ValueType& value) { { value++ } -> DecaySameAs<ValueType>; }
            inline constexpr NamedType operator++(int) noexcept
            {
                NamedType ret{ _value };
                ++_value;
                return *this;
            }

            template<typename = void>
                requires requires (ValueType& value) { { --value } -> DecaySameAs<ValueType>; }
            inline constexpr NamedType& operator--(void) noexcept
            {
                --_value;
                return *this;
            }

            template<typename = void>
                requires requires (ValueType& value) { { value-- } -> DecaySameAs<ValueType>; }
            inline constexpr NamedType operator--(int) noexcept
            {
                NamedType ret{ _value };
                --_value;
                return *this;
            }

        public:
            template<typename = void>
                requires requires (const ValueType& lhs, const ValueType& rhs) { { lhs == rhs } -> NotVoidType; }
            inline constexpr decltype(auto) operator==(const NamedType& rhs) const noexcept
            {
                return _value == rhs._value;
            }

            template<typename = void>
                requires requires (const ValueType& lhs, const ValueType& rhs) { { lhs != rhs } -> NotVoidType; }
            inline constexpr decltype(auto) operator!=(const NamedType& rhs) const noexcept
            {
                return _value != rhs._value;
            }

        public:
            template<typename = void>
                requires requires (const ValueType& lhs, const ValueType& rhs) { { lhs < rhs } -> NotVoidType; }
            inline constexpr decltype(auto) operator<(const NamedType& rhs) const noexcept
            {
                return _value < rhs._value;
            }

            template<typename = void>
                requires requires (const ValueType& lhs, const ValueType& rhs) { { lhs <= rhs } -> NotVoidType; }
            inline constexpr decltype(auto) operator<=(const NamedType& rhs) const noexcept
            {
                return _value <= rhs._value;
            }

            template<typename = void>
                requires requires (const ValueType& lhs, const ValueType& rhs) { { lhs > rhs } -> NotVoidType; }
            inline constexpr decltype(auto) operator>(const NamedType& rhs) const noexcept
            {
                return _value > rhs._value;
            }

            template<typename = void>
                requires requires (const ValueType& lhs, const ValueType& rhs) { { lhs >= rhs } -> NotVoidType; }
            inline constexpr decltype(auto) operator>=(const NamedType& rhs) const noexcept
            {
                return _value >= rhs._value;
            }

        public:
            template<typename = void>
                requires std::three_way_comparable<ValueType>
            inline constexpr decltype(auto) operator<=>(const NamedType& rhs) const noexcept
            {
                return _value <=> rhs._value;
            }

        public:
            template<typename = void>
                requires requires (ValueType& lhs, ValueType& rhs) { std::swap(lhs, rhs); }
            inline constexpr void swap(ValueType& rhs) noexcept
            {
                std::swap(_value, rhs._value);
            }

        private:
            ValueType _value;
        };

        template<typename Os, typename T, typename P>
            requires requires (Os& os, const T& value) { { os << value } -> DecaySameAs<Os>; }
        inline Os& operator<<(Os& os, const NamedType<T, P>& value) noexcept
        {
            return os << value.unwrap();
        }

        template<typename Is, typename T, typename P>
            requires requires (Is& is, T& value) { { is >> value } -> DecaySameAs<Is>; }
        inline Is& operator>>(Is& is, NamedType<T, P>& value) noexcept
        {
            return is >> value.unwrap();
        }
    };
};

#ifndef OSPF_NAMED_TYPE
#define OSPF_NAMED_TYPE(N, T) using N = ::ospf::NamedType<T, struct N##Param>;
#endif

namespace std
{
    template<typename T, typename P>
    struct hash<ospf::NamedType<T, P>>
    {
        inline constexpr const ospf::usize operator()(ospf::ArgCLRefType<ospf::NamedType<T, P>> value) const noexcept
        {
            static const hash<T> hasher{};
            return hasher(value.unwrap());
        }
    };

    template<typename T, typename P>
        requires requires (const T& value) { { std::abs(value) } -> ospf::DecaySameAs<T>; }
    inline constexpr ospf::NamedType<T, P> abs(const ospf::NamedType<T, P>& value) noexcept
    {
        return ospf::NamedType<T, P>(std::abs(value.unwrap()));
    }

    template<typename T, typename P>
        requires requires (T& lhs, T& rhs) { std::swap(lhs, rhs); }
    inline constexpr void swap(ospf::NamedType<T, P>& lhs, ospf::NamedType<T, P>& rhs) noexcept
    {
        return lhs.swap(rhs);
    }

    template<typename T, typename P, ospf::CharType CharT>
    struct formatter<ospf::NamedType<T, P>, CharT>
        : public formatter<T, CharT>
    {
        template<typename FormatContext>
        inline constexpr decltype(auto) format(const ospf::NamedType<T, P>& value, FormatContext& fc) const
        {
            static const formatter<T, CharT> _formatter{};
            return _formatter.format(value.unwrap(), fc);
        }
    };
};

namespace ospf
{
    template<typename T, typename P>
        requires WithTag<T>
    struct TagValue<NamedType<T, P>>
    {
        using Type = typename TagValue<T>::Type;

        inline constexpr RetType<Type> value(ArgCLRefType<NamedType<T, P>> value) const noexcept
        {
            static constexpr const TagValue<T> extractor{};
            return extractor(value.unwrap());
        }
    };

    template<typename T, typename P>
        requires WithDefault<T>
    struct DefaultValue<NamedType<T, P>>
    {
        inline static constexpr RetType<NamedType<T, P>> value(void) noexcept
        {
            return NamedType<T, P>{};
        }
    };
};

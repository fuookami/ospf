#pragma once

#include <ospf/functional/either.hpp>
#include <ospf/memory/reference.hpp>
#include <ospf/meta_programming/named_flag.hpp>

namespace ospf
{
    inline namespace functional
    {
        OSPF_NAMED_FLAG(CopyOnWrite);

        template<
            typename T, 
            ReferenceCategory cat = ReferenceCategory::Reference,
            CopyOnWrite cow = off
        >
        class ValOrRef
        {
        public:
            using ValueType = OriginType<T>;
            using ReferenceType = reference::Ref<ValueType, cat>;
            using PtrType = PtrType<ValueType>;
            using CPtrType = CPtrType<ValueType>;
            using RefType = LRefType<ValueType>;
            using CRefType = CLRefType<ValueType>;

        private:
            using Either = functional::Either<ValueType, ReferenceType>;

        public:
            inline static constexpr ValOrRef value(ArgCLRefType<ValueType> val) noexcept
            {
                return ValOrRef{ val };
            }

            template<typename = void>
                requires ReferenceFaster<ValueType> && std::movable<ValueType>
            inline static constexpr ValOrRef value(ArgRRefType<ValueType> val) noexcept
            {
                return ValOrRef{ move<ValueType>(val) };
            }

            template<typename... Args>
                requires std::constructible_from<ValueType, Args...>
            inline static constexpr ValOrRef value(Args&&... args) noexcept
            {
                return ValOrRef{ ValueType{ std::forward<Args>(args)... } };
            }

            inline static constexpr ValOrRef ref(CLRefType<ValueType> ref) noexcept
            {
                return ValOrRef{ ReferenceType{ ref } };
            }

            template<typename = void>
                requires std::copyable<ReferenceType>
            inline static constexpr ValOrRef ref(ArgCLRefType<ReferenceType> ref) noexcept
            {
                return ValOrRef{ ref };
            }

            template<typename = void>
                requires ReferenceFaster<ReferenceType> && std::movable<ReferenceType>
            inline static constexpr ValOrRef ref(ArgRRefType<ReferenceType> ref) noexcept
            {
                return ValOrRef{ move<ReferenceType>(ref) };
            }

        public:
            template<typename = void>
                requires WithDefault<ValueType>
            constexpr ValOrRef(void)
                : _either(Either::left(DefaultValue<ValueType>::value())) {}

            template<typename U, CopyOnWrite cw>
                requires std::copyable<ReferenceType> && std::convertible_to<U, ValueType> && std::convertible_to<ospf::PtrType<U>, PtrType>
            constexpr ValOrRef(const ValOrRef<U, cat, cw>& ano)
                : _either(std::visit([](const auto& value)
                    {
                        if constexpr (DecaySameAs<decltype(value), U>)
                        {
                            return Either::left(ValueType{ value });
                        }
                        else if constexpr (DecaySameAs<decltype(value), reference::Ref<U, cat>>)
                        {
                            return Either::right(ReferenceType{ value });
                        }
                    }, ano._either)) {}

            template<typename U, CopyOnWrite cw>
                requires std::convertible_to<U, ValueType> && std::convertible_to<ospf::PtrType<U>, PtrType>
            constexpr ValOrRef(ValOrRef<U, cat, cw>&& ano)
                : _either(std::visit([](auto& value)
                    {
                        if constexpr (DecaySameAs<decltype(value), U>)
                        {
                            return Either::left(ValueType{ std::move(value) });
                        }
                        else if constexpr (DecaySameAs<decltype(value), reference::Ref<U, cat>>)
                        {
                            return Either::right(ReferenceType{ std::move(value) });
                        }
                    }, ano._either)) {}

        public:
            constexpr ValOrRef(ArgCLRefType<ValueType> value)
                : _either(Either::left(value)) {}

            template<typename = void>
                requires ReferenceFaster<ValueType> && std::movable<ValueType>
            constexpr ValOrRef(ArgRRefType<ValueType> value)
                : _either(Either::left(move<ValueType>(value))) {}

        private:
            constexpr ValOrRef(ArgCLRefType<ReferenceType> ref)
                : _either(Either::right(ref)) {}

            template<typename = void>
                requires ReferenceFaster<ReferenceType> && std::movable<ReferenceType>
            constexpr ValOrRef(ArgRRefType<ReferenceType> ref)
                : _either(Either::right(move<ReferenceType>(ref))) {}

        public:
            constexpr ValOrRef(const ValOrRef& ano) = default;
            constexpr ValOrRef(ValOrRef&& ano) noexcept = default;
            constexpr ValOrRef& operator=(const ValOrRef& rhs) = default;
            constexpr ValOrRef& operator=(ValOrRef&& rhs) noexcept = default;
            constexpr ~ValOrRef(void) noexcept = default;

        public:
            inline constexpr const bool is_value(void) const noexcept
            {
                return _either.is_left();
            }

            inline constexpr const bool is_reference(void) const noexcept
            {
                return _either.is_right();
            }

        public:
            inline constexpr const PtrType operator->(void) noexcept
            {
                return &deref();
            }

            inline constexpr const CPtrType operator->(void) const noexcept
            {
                return &deref();
            }

            inline constexpr RefType operator*(void) noexcept
            {
                return deref();
            }

            inline constexpr CRefType operator*(void) const noexcept
            {
                return deref();
            }

        public:
            inline constexpr operator RefType(void) noexcept
            {
                return deref();
            }

            inline constexpr operator CRefType(void) const noexcept
            {
                return deref();
            }

        public:
            template<typename = void>
                requires std::copyable<ValueType>
            inline constexpr void copy_if_reference(void) noexcept
            {
                if (_either.is_right())
                {
                    _either = Either::left(*_either.right());
                }
            }

        public:
            template<typename U, ReferenceCategory c, CopyOnWrite cw>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs == rhs } -> DecaySameAs<bool>; }
            inline constexpr const bool operator==(const ValOrRef<U, c, cw>& rhs) const noexcept
            {
                return &deref() == &*rhs || deref() == *rhs;
            }

            template<typename U, ReferenceCategory c, CopyOnWrite cw>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs == rhs } -> DecayNotSameAs<void, bool>; }
            inline constexpr decltype(auto) operator==(const ValOrRef<U, c, cw>& rhs) const noexcept
            {
                return deref() == *rhs;
            }

            template<typename U, ReferenceCategory c, CopyOnWrite cw>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs != rhs } -> DecaySameAs<bool>; }
            inline constexpr const bool operator!=(const ValOrRef<U, c, cw>& rhs) const noexcept
            {
                return &deref() != &*rhs && deref() != *rhs;
            }

            template<typename U, ReferenceCategory c, CopyOnWrite cw>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs != rhs } -> DecayNotSameAs<void, bool>; }
            inline constexpr decltype(auto) operator!=(const ValOrRef<U, c, cw>& rhs) const noexcept
            {
                return deref() != *rhs;
            }

            template<typename U, ReferenceCategory c>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs == rhs } -> DecaySameAs<bool>; }
            inline constexpr const bool operator==(const reference::Ref<U, c>& rhs) const noexcept
            {
                return &deref() == &*rhs || deref() == *rhs;
            }

            template<typename U, ReferenceCategory c>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs == rhs } -> DecayNotSameAs<void, bool>; }
            inline constexpr decltype(auto) operator==(const reference::Ref<U, c>& rhs) const noexcept
            {
                return deref() == *rhs;
            }

            template<typename U, ReferenceCategory c>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs != rhs } -> DecaySameAs<bool>; }
            inline constexpr const bool operator!=(const reference::Ref<U, c>& rhs) const noexcept
            {
                return &deref() != &*rhs && deref() != *rhs;
            }

            template<typename U, ReferenceCategory c>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs != rhs } -> DecayNotSameAs<void, bool>; }
            inline constexpr decltype(auto) operator!=(const reference::Ref<U, c>& rhs) const noexcept
            {
                return deref() != *rhs;
            }

            template<typename U>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs == rhs } -> DecaySameAs<bool>; }
            inline constexpr const bool operator==(const U& rhs) const noexcept
            {
                return &deref() == &rhs || deref() == rhs;
            }

            template<typename U>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs == rhs } -> DecayNotSameAs<void, bool>; }
            inline constexpr decltype(auto) operator==(const U& rhs) const noexcept
            {
                return deref() == rhs;
            }

            template<typename U>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs != rhs } -> DecaySameAs<bool>; }
            inline constexpr const bool operator!=(const U& rhs) const noexcept
            {
                return &deref() != &rhs && deref() != rhs;
            }

            template<typename U>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs != rhs } -> DecayNotSameAs<void, bool>; }
            inline constexpr decltype(auto) operator!=(const U& rhs) const noexcept
            {
                return deref() != rhs;
            }

            template<typename U>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs == rhs } -> DecaySameAs<bool>; }
            inline constexpr const bool operator==(const std::optional<U>& rhs) const noexcept
            {
                return static_cast<bool>(rhs) && (&deref() == &*rhs || deref() == *rhs);
            }

            template<typename U>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs == rhs } -> DecayNotSameAs<void, bool>; }
            inline constexpr decltype(auto) operator==(const std::optional<U>& rhs) const
            {
                return deref() == *rhs;
            }

            template<typename U>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs != rhs } -> DecaySameAs<bool>; }
            inline constexpr const bool operator!=(const std::optional<U>& rhs) const noexcept
            {
                return !static_cast<bool>(rhs) || (&deref() != &*rhs && deref() != *rhs);
            }

            template<typename U>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs != rhs } -> DecayNotSameAs<void, bool>; }
            inline constexpr decltype(auto) operator!=(const std::optional<U>& rhs) const
            {
                return deref() != *rhs;
            }

        public:
            template<typename U, ReferenceCategory c, CopyOnWrite cw>
            inline constexpr decltype(auto) operator<(const ValOrRef<U, c, cw>& rhs) const noexcept
            {
                return deref() < *rhs;
            }

            template<typename U, ReferenceCategory c, CopyOnWrite cw>
            inline constexpr decltype(auto) operator<=(const ValOrRef<U, c, cw>& rhs) const noexcept
            {
                return deref() <= *rhs;
            }

            template<typename U, ReferenceCategory c, CopyOnWrite cw>
            inline constexpr decltype(auto) operator>(const ValOrRef<U, c, cw>& rhs) const noexcept
            {
                return deref() > *rhs;
            }

            template<typename U, ReferenceCategory c, CopyOnWrite cw>
            inline constexpr decltype(auto) operator>=(const ValOrRef<U, c, cw>& rhs) const noexcept
            {
                return deref() >= *rhs;
            }

            template<typename U, ReferenceCategory c>
            inline constexpr decltype(auto) operator<(const reference::Ref<U, c>& rhs) const noexcept
            {
                return deref() < *rhs;
            }

            template<typename U, ReferenceCategory c>
            inline constexpr decltype(auto) operator<=(const reference::Ref<U, c>& rhs) const noexcept
            {
                return deref() <= *rhs;
            }

            template<typename U, ReferenceCategory c>
            inline constexpr decltype(auto) operator>(const reference::Ref<U, c>& rhs) const noexcept
            {
                return deref() > *rhs;
            }

            template<typename U, ReferenceCategory c>
            inline constexpr decltype(auto) operator>=(const reference::Ref<U, c>& rhs) const noexcept
            {
                return deref() > *rhs;
            }

            template<typename U>
            inline constexpr decltype(auto) operator<(const U& rhs) const noexcept
            {
                return deref() < rhs;
            }

            template<typename U>
            inline constexpr decltype(auto) operator<=(const U& rhs) const noexcept
            {
                return deref() <= rhs;
            }

            template<typename U>
            inline constexpr decltype(auto) operator>(const U& rhs) const noexcept
            {
                return deref() > rhs;
            }

            template<typename U>
            inline constexpr decltype(auto) operator>=(const U& rhs) const noexcept
            {
                return deref() >= rhs;
            }

            template<typename U>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs < rhs } -> DecaySameAs<bool>; }
            inline constexpr const bool operator<(const std::optional<U>& rhs) const noexcept
            {
                return static_cast<bool>(rhs) && deref() < *rhs;
            }

            template<typename U>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs < rhs } -> DecayNotSameAs<void, bool>; }
            inline constexpr decltype(auto) operator<(const std::optional<U>& rhs) const
            {
                return deref() < *rhs;
            }

            template<typename U>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs <= rhs } -> DecaySameAs<bool>; }
            inline constexpr const bool operator<=(const std::optional<U>& rhs) const noexcept
            {
                return static_cast<bool>(rhs) && deref() <= *rhs;
            }

            template<typename U>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs <= rhs } -> DecayNotSameAs<void, bool>; }
            inline constexpr decltype(auto) operator<=(const std::optional<U>& rhs) const
            {
                return deref() <= *rhs;
            }

            template<typename U>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs > rhs } -> DecaySameAs<bool>; }
            inline constexpr const bool operator>(const std::optional<U>& rhs) const noexcept
            {
                return static_cast<bool>(rhs) && deref() > *rhs;
            }

            template<typename U>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs > rhs } -> DecayNotSameAs<void, bool>; }
            inline constexpr decltype(auto) operator>(const std::optional<U>& rhs) const
            {
                return deref() > *rhs;
            }

            template<typename U>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs >= rhs } -> DecaySameAs<bool>; }
            inline constexpr const bool operator>=(const std::optional<U>& rhs) const noexcept
            {
                return static_cast<bool>(rhs) && deref() >= *rhs;
            }

            template<typename U>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs >= rhs } -> DecayNotSameAs<void, bool>; }
            inline constexpr decltype(auto) operator>=(const std::optional<U>& rhs) const
            {
                return deref() >= *rhs;
            }

        public:
            template<typename U, ReferenceCategory c, CopyOnWrite cw>
            inline constexpr decltype(auto) operator<=>(const ValOrRef<U, c, cw>& rhs) const noexcept
            {
                return deref() <=> *rhs;
            }

            template<typename U, ReferenceCategory c>
            inline constexpr decltype(auto) operator<=>(const reference::Ref<U, c>& rhs) const noexcept
            {
                return deref() <=> *rhs;
            }

            template<typename U>
            inline constexpr decltype(auto) operator<=>(const U& rhs) const noexcept
            {
                return deref() <=> rhs;
            }

            template<typename U>
            inline constexpr ospf::RetType<std::compare_three_way_result_t<T, U>> operator<=>(const std::optional<U>& rhs) const noexcept
            {
                if (rhs.has_value())
                {
                    return deref() <=> *rhs;
                }
                else
                {
                    return true <=> static_cast<bool>(rhs);
                }
            }

        private:
            inline constexpr RefType deref(void) noexcept
            {
                if constexpr (cow == on)
                {
                    copy_if_reference();
                }
                return const_cast<RefType>(const_cast<const ValOrRef&>(*this).deref());
            }

            inline constexpr CRefType deref(void) const noexcept
            {
                return std::visit([](const auto& arg)
                    {
                        using ThisType = OriginType<decltype(arg)>;
                        if constexpr (DecaySameAs<ThisType, ValueType>)
                        {
                            return arg;
                        }
                        else if constexpr (DecaySameAs<ThisType, ReferenceType>)
                        {
                            return *arg;
                        }
                        //else
                        //{
                        //    static_assert(false, "non-exhaustive visitor!");
                        //}
                    }, _either);
            }

        private:
            Either _either;
        };

        template<
            typename T, 
            ReferenceCategory cat,
            CopyOnWrite cow
        >
            requires CopyFaster<T>
        class ValOrRef<T, cat, cow>
        {
        public:
            using ValueType = OriginType<T>;
            using ReferenceType = reference::Ref<ValueType, cat>;
            using PtrType = PtrType<ValueType>;
            using CPtrType = CPtrType<ValueType>;
            using RefType = LRefType<ValueType>;
            using CRefType = CLRefType<ValueType>;

        public:
            inline static constexpr ValOrRef value(ArgCLRefType<ValueType> val) noexcept
            {
                return ValOrRef{ val };
            }

            template<typename... Args>
                requires std::constructible_from<ValueType, Args...>
            inline static constexpr ValOrRef value(Args&&... args) noexcept
            {
                return ValOrRef{ ValueType{ std::forward<Args>(args)... } };
            }

            inline static constexpr ValOrRef ref(ArgCLRefType<ValueType> ref) noexcept
            {
                return ValOrRef{ ref };
            }

            inline static constexpr ValOrRef ref(ArgCLRefType<ReferenceType> ref) noexcept
            {
                return ValOrRef{ ref };
            }

        public:
            template<typename = void>
                requires WithDefault<ValueType>
            constexpr ValOrRef(void)
                : _value(DefaultValue<ValueType>::value()) {}

        public:
            template<typename U>
                requires std::convertible_to<U, ValueType>
            constexpr ValOrRef(const ValOrRef<U, cat, cow>& ano)
                : _value(static_cast<ValueType>(*ano)) {}

        public:
            constexpr ValOrRef(ArgCLRefType<ValueType> value)
                : _value(value) {}

        public:
            constexpr ValOrRef(const ValOrRef& ano) = default;
            constexpr ValOrRef(ValOrRef&& ano) noexcept = default;
            constexpr ValOrRef& operator=(const ValOrRef& rhs) = default;
            constexpr ValOrRef& operator=(ValOrRef&& rhs) noexcept = default;
            constexpr ~ValOrRef(void) = default;

        public:
            inline constexpr const bool is_value(void) const noexcept
            {
                return true;
            }

            inline constexpr const bool is_reference(void) const noexcept
            {
                return false;
            }

        public:
            inline constexpr const PtrType operator->(void) noexcept
            {
                return &_value;
            }

            inline constexpr const CPtrType operator->(void) const noexcept
            {
                return &_value;
            }

            inline constexpr RefType operator*(void) noexcept
            {
                return _value;
            }

            inline constexpr CRefType operator*(void) const noexcept
            {
                return _value;
            }

        public:
            inline constexpr operator RefType(void) noexcept
            {
                return _value;
            }

            inline constexpr operator CRefType(void) const noexcept
            {
                return _value;
            }

        public:
            inline constexpr void copy_if_reference(void) noexcept
            {
                // nothing to do
            }

        public:
            template<typename U, ReferenceCategory c, CopyOnWrite cw>
            inline constexpr decltype(auto) operator==(const ValOrRef<U, c, cw>& rhs) const noexcept
            {
                return _value == *rhs;
            }

            template<typename U, ReferenceCategory c, CopyOnWrite cw>
            inline constexpr decltype(auto) operator!=(const ValOrRef<U, c, cw>& rhs) const noexcept
            {
                return _value != *rhs;
            }

            template<typename U, ReferenceCategory c>
            inline constexpr decltype(auto) operator==(const reference::Ref<U, c>& rhs) const noexcept
            {
                return _value == *rhs;
            }

            template<typename U, ReferenceCategory c>
            inline constexpr decltype(auto) operator!=(const reference::Ref<U, c>& rhs) const noexcept
            {
                return _value != *rhs;
            }

            template<typename U>
            inline constexpr decltype(auto) operator==(const U& rhs) const noexcept
            {
                return _value == rhs;
            }

            template<typename U>
            inline constexpr decltype(auto) operator!=(const U& rhs) const noexcept
            {
                return _value != rhs;
            }

            template<typename U>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs == rhs } -> DecaySameAs<bool>; }
            inline constexpr const bool operator==(const std::optional<U>& rhs) const noexcept
            {
                return static_cast<bool>(rhs) && _value == *rhs;
            }

            template<typename U>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs == rhs } -> DecayNotSameAs<void, bool>; }
            inline constexpr decltype(auto) operator==(const std::optional<U>& rhs) const
            {
                return _value == *rhs;
            }

            template<typename U>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs != rhs } -> DecaySameAs<bool>; }
            inline constexpr const bool operator!=(const std::optional<U>& rhs) const noexcept
            {
                return !static_cast<bool>(rhs) || _value != *rhs;
            }

            template<typename U>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs != rhs } -> DecayNotSameAs<void, bool>; }
            inline constexpr decltype(auto) operator!=(const std::optional<U>& rhs) const
            {
                return _value != *rhs;
            }

        public:
            template<typename U, ReferenceCategory c, CopyOnWrite cw>
            inline constexpr decltype(auto) operator<(const ValOrRef<U, c, cw>& rhs) const noexcept
            {
                return _value < *rhs;
            }

            template<typename U, ReferenceCategory c, CopyOnWrite cw>
            inline constexpr decltype(auto) operator<=(const ValOrRef<U, c, cw>& rhs) const noexcept
            {
                return _value <= *rhs;
            }

            template<typename U, ReferenceCategory c, CopyOnWrite cw>
            inline constexpr decltype(auto) operator>(const ValOrRef<U, c, cw>& rhs) const noexcept
            {
                return _value > *rhs;
            }

            template<typename U, ReferenceCategory c, CopyOnWrite cw>
            inline constexpr decltype(auto) operator>=(const ValOrRef<U, c, cw>& rhs) const noexcept
            {
                return _value > *rhs;
            }

            template<typename U, ReferenceCategory c>
            inline constexpr decltype(auto) operator<(const reference::Ref<U, c>& rhs) const noexcept
            {
                return _value < *rhs;
            }

            template<typename U, ReferenceCategory c>
            inline constexpr decltype(auto) operator<=(const reference::Ref<U, c>& rhs) const noexcept
            {
                return _value <= *rhs;
            }

            template<typename U, ReferenceCategory c>
            inline constexpr decltype(auto) operator>(const reference::Ref<U, c>& rhs) const noexcept
            {
                return _value > *rhs;
            }

            template<typename U, ReferenceCategory c>
            inline constexpr decltype(auto) operator>=(const reference::Ref<U, c>& rhs) const noexcept
            {
                return _value > *rhs;
            }

            template<typename U>
            inline constexpr decltype(auto) operator<(const U& rhs) const noexcept
            {
                return _value < rhs;
            }

            template<typename U>
            inline constexpr decltype(auto) operator<=(const U& rhs) const noexcept
            {
                return _value <= rhs;
            }

            template<typename U>
            inline constexpr decltype(auto) operator>(const U& rhs) const noexcept
            {
                return _value > rhs;
            }

            template<typename U>
            inline constexpr const bool operator>=(const U& rhs) const noexcept
            {
                return _value >= rhs;
            }

            template<typename U>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs < rhs } -> DecaySameAs<bool>; }
            inline constexpr const bool operator<(const std::optional<U>& rhs) const noexcept
            {
                return static_cast<bool>(rhs) && _value < *rhs;
            }

            template<typename U>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs <= rhs } -> DecayNotSameAs<void, bool>; }
            inline constexpr decltype(auto) operator<(const std::optional<U>& rhs) const
            {
                return _value < *rhs;
            }

            template<typename U>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs <= rhs } -> DecaySameAs<bool>; }
            inline constexpr const bool operator<=(const std::optional<U>& rhs) const noexcept
            {
                return static_cast<bool>(rhs) && _value <= *rhs;
            }

            template<typename U>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs <= rhs } -> DecayNotSameAs<void, bool>; }
            inline constexpr decltype(auto) operator<=(const std::optional<U>& rhs) const
            {
                return _value <= *rhs;
            }

            template<typename U>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs > rhs } -> DecaySameAs<bool>; }
            inline constexpr const bool operator>(const std::optional<U>& rhs) const noexcept
            {
                return static_cast<bool>(rhs) && _value > *rhs;
            }

            template<typename U>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs > rhs } -> DecayNotSameAs<void, bool>; }
            inline constexpr decltype(auto) operator>(const std::optional<U>& rhs) const
            {
                return _value > *rhs;
            }

            template<typename U>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs >= rhs } -> DecaySameAs<bool>; }
            inline constexpr const bool operator>=(const std::optional<U>& rhs) const noexcept
            {
                return static_cast<bool>(rhs) && _value >= *rhs;
            }

            template<typename U>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs >= rhs } -> DecayNotSameAs<void, bool>; }
            inline constexpr decltype(auto) operator>=(const std::optional<U>& rhs) const
            {
                return _value >= *rhs;
            }

        public:
            template<typename U, ReferenceCategory c, CopyOnWrite cw>
            inline constexpr decltype(auto) operator<=>(const ValOrRef<U, c, cw>& rhs) const noexcept
            {
                return _value <=> *rhs;
            }

            template<typename U, ReferenceCategory c>
            inline constexpr decltype(auto) operator<=>(const reference::Ref<U, c>& rhs) const noexcept
            {
                return _value <=> *rhs;
            }

            template<typename U>
            inline constexpr decltype(auto) operator<=>(const U& rhs) const noexcept
            {
                return _value <=> rhs;
            }

            template<typename U>
            inline constexpr RetType<std::compare_three_way_result_t<T, U>> operator<=>(const std::optional<U>& rhs) const noexcept
            {
                if (rhs.has_value())
                {
                    return _value <=> *rhs;
                }
                else
                {
                    return true <=> static_cast<bool>(rhs);
                }
            }

        private:
            ValueType _value;
        };
    };

    // copy on write
    template<
        typename T,
        ReferenceCategory cat = ReferenceCategory::Reference
    >
    using COW = ValOrRef<T, cat, CopyOnWrite::On>;
};

template<typename T, typename U, ospf::ReferenceCategory cat1, ospf::ReferenceCategory cat2, ospf::CopyOnWrite cow>
inline constexpr decltype(auto) operator==(const ospf::reference::Ref<T, cat1>& lhs, const ospf::ValOrRef<U, cat2, cow>& rhs) noexcept
{
    return *lhs == *rhs;
}

template<typename T, typename U, ospf::ReferenceCategory cat1, ospf::ReferenceCategory cat2, ospf::CopyOnWrite cow>
inline constexpr decltype(auto) operator!=(const ospf::reference::Ref<T, cat1>& lhs, const ospf::ValOrRef<U, cat2, cow>& rhs) noexcept
{
    return *lhs != *rhs;
}

template<typename T, typename U, ospf::ReferenceCategory cat, ospf::CopyOnWrite cow>
inline constexpr decltype(auto) operator==(const T& lhs, const ospf::ValOrRef<U, cat, cow>& rhs) noexcept
{
    return lhs == *rhs;
}

template<typename T, typename U, ospf::ReferenceCategory cat, ospf::CopyOnWrite cow>
inline constexpr decltype(auto) operator!=(const T& lhs, const ospf::ValOrRef<U, cat, cow>& rhs) noexcept
{
    return lhs != *rhs;
}

template<typename T, typename U, ospf::ReferenceCategory cat, ospf::CopyOnWrite cow>
    requires requires (const T& lhs, const U& rhs) { { lhs == rhs } -> ospf::DecaySameAs<bool>; }
inline constexpr const bool operator==(const std::optional<U>& lhs, const ospf::ValOrRef<U, cat, cow>& rhs) noexcept
{
    return static_cast<bool>(lhs) && lhs == *rhs;
}

template<typename T, typename U, ospf::ReferenceCategory cat, ospf::CopyOnWrite cow>
    requires requires (const T& lhs, const U& rhs) { { lhs == rhs } -> ospf::DecayNotSameAs<void, bool>; }
inline constexpr decltype(auto) operator==(const std::optional<U>& lhs, const ospf::ValOrRef<U, cat, cow>& rhs)
{
    return lhs == *rhs;
}

template<typename T, typename U, ospf::ReferenceCategory cat, ospf::CopyOnWrite cow>
    requires requires (const T& lhs, const U& rhs) { { lhs != rhs } -> ospf::DecaySameAs<bool>; }
inline constexpr const bool operator!=(const std::optional<U>& lhs, const ospf::ValOrRef<U, cat, cow>& rhs) noexcept
{
    return !static_cast<bool>(lhs) || lhs != *rhs;
}

template<typename T, typename U, ospf::ReferenceCategory cat, ospf::CopyOnWrite cow>
    requires requires (const T& lhs, const U& rhs) { { lhs != rhs } -> ospf::DecayNotSameAs<void, bool>; }
inline constexpr decltype(auto) operator!=(const std::optional<U>& lhs, const ospf::ValOrRef<U, cat, cow>& rhs)
{
    return lhs != *rhs;
}

template<typename T, typename U, ospf::ReferenceCategory cat1, ospf::ReferenceCategory cat2, ospf::CopyOnWrite cow>
inline constexpr decltype(auto) operator<(const ospf::reference::Ref<T, cat1>& lhs, const ospf::ValOrRef<U, cat2, cow>& rhs) noexcept
{
    return *lhs < *rhs;
}

template<typename T, typename U, ospf::ReferenceCategory cat1, ospf::ReferenceCategory cat2, ospf::CopyOnWrite cow>
inline constexpr decltype(auto) operator<=(const ospf::reference::Ref<T, cat1>& lhs, const ospf::ValOrRef<U, cat2, cow>& rhs) noexcept
{
    return *lhs <= *rhs;
}

template<typename T, typename U, ospf::ReferenceCategory cat1, ospf::ReferenceCategory cat2, ospf::CopyOnWrite cow>
inline constexpr decltype(auto) operator>(const ospf::reference::Ref<T, cat1>& lhs, const ospf::ValOrRef<U, cat2, cow>& rhs) noexcept
{
    return *lhs > *rhs;
}

template<typename T, typename U, ospf::ReferenceCategory cat1, ospf::ReferenceCategory cat2, ospf::CopyOnWrite cow>
inline constexpr decltype(auto) operator>=(const ospf::reference::Ref<T, cat1>& lhs, const ospf::ValOrRef<U, cat2, cow>& rhs) noexcept
{
    return *lhs >= *rhs;
}

template<typename T, typename U, ospf::ReferenceCategory cat, ospf::CopyOnWrite cow>
inline constexpr decltype(auto) operator<(const T& lhs, const ospf::ValOrRef<U, cat, cow>& rhs) noexcept
{
    return lhs < *rhs;
}

template<typename T, typename U, ospf::ReferenceCategory cat, ospf::CopyOnWrite cow>
inline constexpr decltype(auto) operator<=(const T& lhs, const ospf::ValOrRef<U, cat, cow>& rhs) noexcept
{
    return lhs <= *rhs;
}

template<typename T, typename U, ospf::ReferenceCategory cat, ospf::CopyOnWrite cow>
inline constexpr decltype(auto) operator>(const T& lhs, const ospf::ValOrRef<U, cat, cow>& rhs) noexcept
{
    return lhs > *rhs;
}

template<typename T, typename U, ospf::ReferenceCategory cat, ospf::CopyOnWrite cow>
inline constexpr decltype(auto) operator>=(const T& lhs, const ospf::ValOrRef<U, cat, cow>& rhs) noexcept
{
    return lhs >= *rhs;
}

template<typename T, typename U, ospf::ReferenceCategory cat, ospf::CopyOnWrite cow>
    requires requires (const T& lhs, const U& rhs) { { lhs < rhs } -> ospf::DecaySameAs<bool>; }
inline constexpr const bool operator<(const std::optional<T>& lhs, const ospf::ValOrRef<U, cat, cow>& rhs) noexcept
{
    return static_cast<bool>(lhs) && *lhs < *rhs;
}

template<typename T, typename U, ospf::ReferenceCategory cat, ospf::CopyOnWrite cow>
    requires requires (const T& lhs, const U& rhs) { { lhs < rhs } -> ospf::DecayNotSameAs<void, bool>; }
inline constexpr decltype(auto) operator<(const std::optional<T>& lhs, const ospf::ValOrRef<U, cat, cow>& rhs) noexcept
{
    return *lhs < *rhs;
}

template<typename T, typename U, ospf::ReferenceCategory cat, ospf::CopyOnWrite cow>
    requires requires (const T& lhs, const U& rhs) { { lhs <= rhs } -> ospf::DecaySameAs<bool>; }
inline constexpr const bool operator<=(const std::optional<T>& lhs, const ospf::ValOrRef<U, cat, cow>& rhs) noexcept
{
    return static_cast<bool>(lhs) && *lhs <= *rhs;
}

template<typename T, typename U, ospf::ReferenceCategory cat, ospf::CopyOnWrite cow>
    requires requires (const T& lhs, const U& rhs) { { lhs <= rhs } -> ospf::DecayNotSameAs<void, bool>; }
inline constexpr decltype(auto) operator<=(const std::optional<T>& lhs, const ospf::ValOrRef<U, cat, cow>& rhs) noexcept
{
    return *lhs <= *rhs;
}

template<typename T, typename U, ospf::ReferenceCategory cat, ospf::CopyOnWrite cow>
    requires requires (const T& lhs, const U& rhs) { { lhs > rhs } -> ospf::DecaySameAs<bool>; }
inline constexpr const bool operator>(const std::optional<T>& lhs, const ospf::ValOrRef<U, cat, cow>& rhs) noexcept
{
    return static_cast<bool>(lhs) && *lhs > *rhs;
}

template<typename T, typename U, ospf::ReferenceCategory cat, ospf::CopyOnWrite cow>
    requires requires (const T& lhs, const U& rhs) { { lhs > rhs } -> ospf::DecayNotSameAs<void, bool>; }
inline constexpr decltype(auto) operator>(const std::optional<T>& lhs, const ospf::ValOrRef<U, cat, cow>& rhs) noexcept
{
    return *lhs > *rhs;
}

template<typename T, typename U, ospf::ReferenceCategory cat, ospf::CopyOnWrite cow>
    requires requires (const T& lhs, const U& rhs) { { lhs >= rhs } -> ospf::DecaySameAs<bool>; }
inline constexpr const bool operator>=(const std::optional<T>& lhs, const ospf::ValOrRef<U, cat, cow>& rhs) noexcept
{
    return static_cast<bool>(lhs) && *lhs >= *rhs;
}

template<typename T, typename U, ospf::ReferenceCategory cat, ospf::CopyOnWrite cow>
    requires requires (const T& lhs, const U& rhs) { { lhs >= rhs } -> ospf::DecayNotSameAs<void, bool>; }
inline constexpr decltype(auto) operator>=(const std::optional<T>& lhs, const ospf::ValOrRef<U, cat, cow>& rhs) noexcept
{
    return *lhs >= *rhs;
}

template<typename T, typename U, ospf::ReferenceCategory cat1, ospf::ReferenceCategory cat2, ospf::CopyOnWrite cow>
inline constexpr decltype(auto) operator<=>(const ospf::reference::Ref<T, cat1>& lhs, const ospf::ValOrRef<U, cat2, cow>& rhs) noexcept
{
    return *lhs <=> *rhs;
}

template<typename T, typename U, ospf::ReferenceCategory cat, ospf::CopyOnWrite cow>
inline constexpr decltype(auto) operator<=>(const T& lhs, const ospf::ValOrRef<U, cat, cow>& rhs) noexcept
{
    return lhs <=> *rhs;
}

template<typename T, typename U, ospf::ReferenceCategory cat, ospf::CopyOnWrite cow>
inline constexpr ospf::RetType<std::compare_three_way_result_t<T, U>> operator<=>(const std::optional<T>& lhs, const ospf::ValOrRef<U, cat, cow>& rhs) noexcept
{
    if (lhs.has_value())
    {
        return *lhs <=> *rhs;
    }
    else
    {
        return static_cast<bool>(lhs) <=> true;
    }
}

template<typename T, typename U, ospf::ReferenceCategory cat1, ospf::ReferenceCategory cat2, ospf::CopyOnWrite cow>
inline constexpr ospf::RetType<std::compare_three_way_result_t<T, U>> operator<=>(const std::optional<ospf::ValOrRef<T, cat1>>& lhs, const std::optional<ospf::ValOrRef<U, cat2, cow>>& rhs) noexcept
{
    if (lhs.has_value() && rhs.has_value())
    {
        return **lhs <=> **rhs;
    }
    else
    {
        return static_cast<bool>(lhs) <=> static_cast<bool>(rhs);
    }
}

template<typename T, typename U, ospf::ReferenceCategory cat1, ospf::ReferenceCategory cat2, ospf::CopyOnWrite cow>
inline constexpr ospf::RetType<std::compare_three_way_result_t<T, U>> operator<=>(const std::optional<const ospf::ValOrRef<T, cat1>>& lhs, const std::optional<const ospf::ValOrRef<U, cat2, cow>>& rhs) noexcept
{
    if (lhs.has_value() && rhs.has_value())
    {
        return **lhs <=> **rhs;
    }
    else
    {
        return static_cast<bool>(lhs) <=> static_cast<bool>(rhs);
    }
}

namespace std
{
    template<typename T, ospf::ReferenceCategory cat, ospf::CopyOnWrite cow>
    struct hash<ospf::ValOrRef<T, cat, cow>>
    {
        using ValueType = ospf::ValOrRef<T, cat, cow>;

        inline constexpr const ospf::usize operator()(ospf::ArgCLRefType<ValueType> value) const noexcept
        {
            static const hash<T> hasher{};
            return hasher(*value);
        }
    };

    template<typename T, ospf::ReferenceCategory cat, ospf::CopyOnWrite cow, ospf::CharType CharT>
    struct formatter<ospf::ValOrRef<T, cat, cow>, CharT>
        : public formatter<T, CharT>
    {
        using ValueType = ospf::ValOrRef<T, cat, cow>;

        template<typename FormatContext>
        inline constexpr decltype(auto) format(ospf::ArgCLRefType<ValueType> value, FormatContext& fc) const
        {
            static const formatter<T, CharT> _formatter{};
            return _formatter.format(*value, fc);
        }
    };
};

namespace ospf
{
    template<typename T, ReferenceCategory cat, ospf::CopyOnWrite cow>
        requires WithTag<T>
    struct TagValue<ValOrRef<T, cat, cow>>
    {
        using Type = typename TagValue<T>::Type;
        using ValueType = ValOrRef<T, cat, cow>;

        inline constexpr RetType<Type> value(ArgCLRefType<ValueType> value) const noexcept
        {
            static constexpr const TagValue<T> extractor{};
            return extractor(*value);
        }
    };

    template<typename T, ReferenceCategory cat, ospf::CopyOnWrite cow>
        requires WithDefault<T>
    struct DefaultValue<ValOrRef<T, cat, cow>>
    {
        using ValueType = ospf::ValOrRef<T, cat, cow>;

        inline static constexpr ValueType value(void) noexcept
        {
            return ValueType{};
        }
    };
};

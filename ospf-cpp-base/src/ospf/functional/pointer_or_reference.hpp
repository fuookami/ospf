#pragma once

#include <ospf/functional/value_or_reference.hpp>
#include <ospf/memory/pointer.hpp>

namespace ospf
{
    inline namespace functional
    {
        // must not be nullptr
        template<
            typename T,
            PointerCategory pcat = PointerCategory::Shared,
            ReferenceCategory rcat = ReferenceCategory::Reference
        >
        class PtrOrRef
        {
        public:
            using ValueType = OriginType<T>;
            using PointerType = pointer::Ptr<ValueType, pcat>;
            using ReferenceType = reference::Ref<ValueType, rcat>;
            using PtrType = PtrType<ValueType>;
            using CPtrType = CPtrType<ValueType>;
            using RefType = LRefType<ValueType>;
            using CRefType = CLRefType<ValueType>;

        private:
            using Either = functional::Either<PointerType, ReferenceType>;

        public:
            template<typename = void>
                requires std::copyable<PointerType>
            inline static constexpr PtrOrRef ptr(ArgCLRefType<PointerType> ptr)
            {
                return PtrOrRef{ ptr };
            }

            template<typename = void>
                requires ReferenceFaster<PointerType> && std::movable<PointerType>
            inline static constexpr PtrOrRef ptr(ArgRRefType<PointerType> ptr)
            {
                return PtrOrRef{ move<PointerType>(ptr) };
            }

            inline static constexpr PtrOrRef ref(CLRefType<ValueType> ref) noexcept
            {
                return PtrOrRef{ ReferenceType{ ref } };
            }

            template<typename = void>
                requires std::copyable<ReferenceType>
            inline static constexpr PtrOrRef ref(ArgCLRefType<ReferenceType> ref) noexcept
            {
                return PtrOrRef{ ref };
            }

            template<typename = void>
                requires ReferenceFaster<ReferenceType> && std::movable<ReferenceType>
            inline static constexpr PtrOrRef ref(ArgRRefType<ReferenceType> ref) noexcept
            {
                return PtrOrRef{ move<ReferenceType>(ref) };
            }

        public:
            constexpr PtrOrRef(const std::nullptr_t) = delete;

        public:
            template<typename U>
                requires std::copyable<PointerType> && std::convertible_to<ospf::PtrType<U>, PtrType>
            constexpr PtrOrRef(const PtrOrRef<U, pcat, rcat>& ano)
                : _either(std::visit([](const auto& value) 
                    {
                        if constexpr (DecaySameAs<decltype(value), pointer::Ptr<U, pcat>>)
                        {
                            return Either::left(PointerType{ value });
                        }
                        else if constexpr (DecaySameAs<decltype(value), reference::Ref<U, rcat>>)
                        {
                            return Either::right(ReferenceType{ value });
                        }
                    }, ano._either)) {}

            template<typename U>
                requires std::convertible_to<ospf::PtrType<U>, PtrType>
            constexpr PtrOrRef(PtrOrRef<U, pcat, rcat>&& ano)
                : _either(std::visit([](auto& value) 
                    {
                        if constexpr (DecaySameAs<decltype(value), pointer::Ptr<U, pcat>>)
                        {
                            return Either::left(PointerType{ std::move(value) });
                        }
                        else if constexpr (DecaySameAs<decltype(value), reference::Ref<U, rcat>>)
                        {
                            return Either::right(ReferenceType{ std::move(value) });
                        }
                    }, ano._either)) {}

        public:
            template<typename = void>
                requires std::copyable<PointerType>
            constexpr PtrOrRef(ArgCLRefType<PointerType> ptr)
                : _either(Either::left(ptr))
            {
                if (_either.left() == nullptr)
                {
                    throw OSPFException{ OSPFErrCode::ApplicationError, "invalid argument nullptr for PtrOrRef" };
                }
            }

            template<typename = void>
                requires ReferenceFaster<PointerType> && std::movable<PointerType>
            constexpr PtrOrRef(ArgRRefType<PointerType> ptr)
                : _either(Either::left(move<PointerType>(ptr)))
            {
                if (_either.left() == nullptr)
                {
                    throw OSPFException{ OSPFErrCode::ApplicationError, "invalid argument nullptr for PtrOrRef" };
                }
            }

        private:
            constexpr PtrOrRef(ArgCLRefType<ReferenceType> ref)
                : _either(Either::right(ref)) {}

            template<typename = void>
                requires ReferenceFaster<ReferenceType> && std::movable<ReferenceType>
            constexpr PtrOrRef(ArgRRefType<ReferenceType> ref)
                : _either(Either::right(move<ReferenceType>(ref))) {}

        public:
            constexpr PtrOrRef(const PtrOrRef& ano) = default;
            constexpr PtrOrRef(PtrOrRef&& ano) noexcept = default;
            constexpr PtrOrRef& operator=(const PtrOrRef& rhs) = default;
            constexpr PtrOrRef& operator=(PtrOrRef&& rhs) noexcept = default;
            constexpr ~PtrOrRef(void) noexcept = default;

        public:
            inline constexpr const bool is_ptr(void) const noexcept
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
            template<typename U, PointerCategory pc, ReferenceCategory rc>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs == rhs } -> DecaySameAs<bool>; }
            inline constexpr const bool operator==(const PtrOrRef<U, pc, rc>& rhs) const noexcept
            {
                return &deref() == &*rhs || deref() == *rhs;
            }

            template<typename U, PointerCategory pc, ReferenceCategory rc>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs == rhs } -> DecayNotSameAs<void, bool>; }
            inline constexpr decltype(auto) operator==(const PtrOrRef<U, pc, rc>& rhs) const noexcept
            {
                return deref() == *rhs;
            }

            template<typename U, PointerCategory pc, ReferenceCategory rc>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs != rhs } -> DecaySameAs<bool>; }
            inline constexpr const bool operator!=(const PtrOrRef<U, pc, rc>& rhs) const noexcept
            {
                return &deref() != &*rhs && deref() != *rhs;
            }

            template<typename U, PointerCategory pc, ReferenceCategory rc>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs != rhs } -> DecayNotSameAs<void, bool>; }
            inline constexpr decltype(auto) operator!=(const PtrOrRef<U, pc, rc>& rhs) const noexcept
            {
                return deref() != *rhs;
            }

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

            template<typename U, PointerCategory c>
            inline constexpr const bool operator==(const pointer::Ptr<U, c>& rhs) const noexcept
            {
                return rhs != nullptr && &deref() == &*rhs;
            }

            template<typename U, PointerCategory c>
            inline constexpr const bool operator!=(const pointer::Ptr<U, c>& rhs) const noexcept
            {
                return rhs == nullptr || &deref() != &*rhs;
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
                return !static_cast<bool>(rhs) && (&deref() != &*rhs && deref() != *rhs);
            }

            template<typename U>
                requires requires (const ValueType& lhs, const U& rhs) { { lhs != rhs } -> DecayNotSameAs<void, bool>; }
            inline constexpr decltype(auto) operator!=(const std::optional<U>& rhs) const
            {
                return deref() != *rhs;
            }

        public:
            template<typename U, PointerCategory pc, ReferenceCategory rc>
            inline constexpr decltype(auto) operator<(const PtrOrRef<U, pc, rc>& rhs) const noexcept
            {
                return deref() < *rhs;
            }

            template<typename U, PointerCategory pc, ReferenceCategory rc>
            inline constexpr decltype(auto) operator<=(const PtrOrRef<U, pc, rc>& rhs) const noexcept
            {
                return deref() <= *rhs;
            }

            template<typename U, PointerCategory pc, ReferenceCategory rc>
            inline constexpr decltype(auto) operator>(const PtrOrRef<U, pc, rc>& rhs) const noexcept
            {
                return deref() > *rhs;
            }

            template<typename U, PointerCategory pc, ReferenceCategory rc>
            inline constexpr decltype(auto) operator>=(const PtrOrRef<U, pc, rc>& rhs) const noexcept
            {
                return deref() >= *rhs;
            }

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

            template<typename U, PointerCategory c>
            inline constexpr const bool operator<(const pointer::Ptr<U, c>& rhs) const noexcept
            {
                return rhs != nullptr && &deref() < &*rhs;
            }

            template<typename U, PointerCategory c>
            inline constexpr const bool operator<=(const pointer::Ptr<U, c>& rhs) const noexcept
            {
                return rhs != nullptr && &deref() <= &*rhs;
            }

            template<typename U, PointerCategory c>
            inline constexpr const bool operator>(const pointer::Ptr<U, c>& rhs) const noexcept
            {
                return rhs != nullptr && &deref() > &*rhs;
            }

            template<typename U, PointerCategory c>
            inline constexpr const bool operator>=(const pointer::Ptr<U, c>& rhs) const noexcept
            {
                return rhs != nullptr && &deref() >= &*rhs;
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
                return deref() >= *rhs;
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
            template<typename U, PointerCategory pc, ReferenceCategory rc>
            inline constexpr decltype(auto) operator<=>(const PtrOrRef<U, pc, rc>& rhs) const noexcept
            {
                return deref() <=> *rhs;
            }

            template<typename U, ReferenceCategory rc, CopyOnWrite cw>
            inline constexpr decltype(auto) operator<=>(const ValOrRef<U, rc, cw>& rhs) const noexcept
            {
                return deref() <=> *rhs;
            }

            template<typename U, PointerCategory c>
            inline constexpr ospf::RetType<std::compare_three_way_result_t<T, U>> operator<=>(const pointer::Ptr<U, c>& rhs) const noexcept
            {
                if (rhs != nullptr)
                {
                    return deref() <=> *rhs;
                }
                else
                {
                    return true <=> static_cast<bool>(rhs);
                }
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
                return const_cast<RefType>(const_cast<const PtrOrRef&>(*this).deref());
            }

            inline constexpr CRefType deref(void) const noexcept
            {
                return std::visit([](const auto& arg)
                    {
                        using ThisType = OriginType<decltype(arg)>;
                        if constexpr (DecaySameAs<ThisType, PointerType>)
                        {
                            return *arg;
                        }
                        else if constexpr (DecaySameAs<ThisType, ReferenceType>)
                        {
                            return *arg;
                        }
                    }, _either);
            }

        private:
            Either _either;
        };

        template<
            typename T,
            ReferenceCategory cat = ReferenceCategory::Reference
        >
        using UniqueOrRef = PtrOrRef<T, PointerCategory::Unique, cat>;

        template<
            typename T,
            ReferenceCategory cat = ReferenceCategory::Reference
        >
        using SharedOrRef = PtrOrRef<T, PointerCategory::Shared, cat>;
    };
};

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat1, ospf::ReferenceCategory rcat2, ospf::CopyOnWrite cow>
    requires requires (const T& lhs, const U& rhs) { { lhs == rhs } -> ospf::DecaySameAs<bool>; }
inline constexpr const bool operator==(const ospf::ValOrRef<T, rcat1, cow>& lhs, const ospf::PtrOrRef<U, pcat, rcat2>& rhs) noexcept
{
    return &*lhs == &*rhs || *lhs == *rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat1, ospf::ReferenceCategory rcat2, ospf::CopyOnWrite cow>
    requires requires (const T& lhs, const U& rhs) { { lhs == rhs } -> ospf::DecayNotSameAs<void, bool>; }
inline constexpr decltype(auto) operator==(const ospf::ValOrRef<T, rcat1, cow>& lhs, const ospf::PtrOrRef<U, pcat, rcat2>& rhs) noexcept
{
    return *lhs == *rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat1, ospf::ReferenceCategory rcat2, ospf::CopyOnWrite cow>
    requires requires (const T& lhs, const U& rhs) { { lhs != rhs } -> ospf::DecaySameAs<bool>; }
inline constexpr const bool operator!=(const ospf::ValOrRef<T, rcat1, cow>& lhs, const ospf::PtrOrRef<U, pcat, rcat2>& rhs) noexcept
{
    return &*lhs != &*rhs && *lhs != *rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat1, ospf::ReferenceCategory rcat2, ospf::CopyOnWrite cow>
    requires requires (const T& lhs, const U& rhs) { { lhs != rhs } -> ospf::DecayNotSameAs<void, bool>; }
inline constexpr decltype(auto) operator!=(const ospf::ValOrRef<T, rcat1, cow>& lhs, const ospf::PtrOrRef<U, pcat, rcat2>& rhs) noexcept
{
    return *lhs != *rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat1, ospf::PointerCategory pcat2, ospf::ReferenceCategory rcat>
inline constexpr const bool operator==(const ospf::pointer::Ptr<T, pcat1>& lhs, const ospf::PtrOrRef<U, pcat2, rcat>& rhs) noexcept
{
    return &lhs == &*rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat1, ospf::PointerCategory pcat2, ospf::ReferenceCategory rcat>
inline constexpr const bool operator!=(const ospf::pointer::Ptr<T, pcat1>& lhs, const ospf::PtrOrRef<U, pcat2, rcat>& rhs) noexcept
{
    return &lhs != &*rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat1, ospf::ReferenceCategory rcat2>
    requires requires (const T& lhs, const U& rhs) { { lhs == rhs } -> ospf::DecaySameAs<bool>; }
inline constexpr const bool operator==(const ospf::reference::Ref<T, rcat1>& lhs, const ospf::PtrOrRef<U, pcat, rcat2>& rhs) noexcept
{
    return &*lhs == &*rhs || *lhs == *rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat1, ospf::ReferenceCategory rcat2>
    requires requires (const T& lhs, const U& rhs) { { lhs == rhs } -> ospf::DecayNotSameAs<void, bool>; }
inline constexpr decltype(auto) operator==(const ospf::reference::Ref<T, rcat1>& lhs, const ospf::PtrOrRef<U, pcat, rcat2>& rhs) noexcept
{
    return *lhs == *rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat1, ospf::ReferenceCategory rcat2>
    requires requires (const T& lhs, const U& rhs) { { lhs != rhs } -> ospf::DecaySameAs<bool>; }
inline constexpr const bool operator!=(const ospf::reference::Ref<T, rcat1>& lhs, const ospf::PtrOrRef<U, pcat, rcat2>& rhs) noexcept
{
    return &*lhs != &*rhs && *lhs != *rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat1, ospf::ReferenceCategory rcat2>
    requires requires (const T& lhs, const U& rhs) { { lhs != rhs } -> ospf::DecayNotSameAs<void, bool>; }
inline constexpr decltype(auto) operator!=(const ospf::reference::Ref<T, rcat1>& lhs, const ospf::PtrOrRef<U, pcat, rcat2>& rhs) noexcept
{
    return *lhs != *rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat>
    requires requires (const T& lhs, const U& rhs) { { lhs == rhs } -> ospf::DecaySameAs<bool>; }
inline constexpr const bool operator==(const T& lhs, const ospf::PtrOrRef<U, pcat, rcat>& rhs) noexcept
{
    return &lhs == &*rhs || lhs == *rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat>
    requires requires (const T& lhs, const U& rhs) { { lhs == rhs } -> ospf::DecayNotSameAs<void, bool>; }
inline constexpr decltype(auto) operator==(const T& lhs, const ospf::PtrOrRef<U, pcat, rcat>& rhs) noexcept
{
    return lhs == *rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat>
    requires requires (const T& lhs, const U& rhs) { { lhs != rhs } -> ospf::DecaySameAs<bool>; }
inline constexpr const bool operator!=(const T& lhs, const ospf::PtrOrRef<U, pcat, rcat>& rhs) noexcept
{
    return &lhs != &*rhs && lhs != *rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat>
    requires requires (const T& lhs, const U& rhs) { { lhs != rhs } -> ospf::DecayNotSameAs<void, bool>; }
inline constexpr decltype(auto) operator!=(const T& lhs, const ospf::PtrOrRef<U, pcat, rcat>& rhs) noexcept
{
    return lhs != *rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat>
    requires requires (const T& lhs, const U& rhs) { { lhs == rhs } -> ospf::DecaySameAs<bool>; }
inline constexpr const bool operator==(const std::optional<T>& lhs, const ospf::PtrOrRef<U, pcat, rcat>& rhs) noexcept
{
    return static_cast<bool>(lhs) && (&*lhs == &*rhs || *lhs == *rhs);
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat>
    requires requires (const T& lhs, const U& rhs) { { lhs == rhs } -> ospf::DecayNotSameAs<void, bool>; }
inline constexpr decltype(auto) operator==(const std::optional<T>& lhs, const ospf::PtrOrRef<U, pcat, rcat>& rhs)
{
    return *lhs == *rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat>
    requires requires (const T& lhs, const U& rhs) { { lhs != rhs } -> ospf::DecaySameAs<bool>; }
inline constexpr const bool operator!=(const std::optional<T>& lhs, const ospf::PtrOrRef<U, pcat, rcat>& rhs) noexcept
{
    return static_cast<bool>(lhs) && (&*lhs != &*rhs && *lhs != *rhs);
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat>
    requires requires (const T& lhs, const U& rhs) { { lhs != rhs } -> ospf::DecayNotSameAs<void, bool>; }
inline constexpr decltype(auto) operator!=(const std::optional<T>& lhs, const ospf::PtrOrRef<U, pcat, rcat>& rhs)
{
    return *lhs != *rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat1, ospf::ReferenceCategory rcat2, ospf::CopyOnWrite cow>
inline constexpr decltype(auto) operator<(const ospf::ValOrRef<T, rcat1, cow>& lhs, const ospf::PtrOrRef<U, pcat, rcat2>& rhs) noexcept
{
    return *lhs < *rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat1, ospf::ReferenceCategory rcat2, ospf::CopyOnWrite cow>
inline constexpr decltype(auto) operator<=(const ospf::ValOrRef<T, rcat1, cow>& lhs, const ospf::PtrOrRef<U, pcat, rcat2>& rhs) noexcept
{
    return *lhs <= *rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat1, ospf::ReferenceCategory rcat2, ospf::CopyOnWrite cow>
inline constexpr decltype(auto) operator>(const ospf::ValOrRef<T, rcat1, cow>& lhs, const ospf::PtrOrRef<U, pcat, rcat2>& rhs) noexcept
{
    return *lhs > *rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat1, ospf::ReferenceCategory rcat2, ospf::CopyOnWrite cow>
inline constexpr decltype(auto) operator>=(const ospf::ValOrRef<T, rcat1, cow>& lhs, const ospf::PtrOrRef<U, pcat, rcat2>& rhs) noexcept
{
    return *lhs >= *rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat1, ospf::PointerCategory pcat2, ospf::ReferenceCategory rcat>
inline constexpr const bool operator<(const ospf::pointer::Ptr<T, pcat1>& lhs, const ospf::PtrOrRef<U, pcat2, rcat>& rhs) noexcept
{
    return lhs != nullptr && &lhs < &rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat1, ospf::PointerCategory pcat2, ospf::ReferenceCategory rcat>
inline constexpr const bool operator<=(const ospf::pointer::Ptr<T, pcat1>& lhs, const ospf::PtrOrRef<U, pcat2, rcat>& rhs) noexcept
{
    return lhs != nullptr && &lhs <= &rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat1, ospf::PointerCategory pcat2, ospf::ReferenceCategory rcat>
inline constexpr const bool operator>(const ospf::pointer::Ptr<T, pcat1>& lhs, const ospf::PtrOrRef<U, pcat2, rcat>& rhs) noexcept
{
    return lhs != nullptr && &lhs > &rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat1, ospf::PointerCategory pcat2, ospf::ReferenceCategory rcat>
inline constexpr const bool operator>=(const ospf::pointer::Ptr<T, pcat1>& lhs, const ospf::PtrOrRef<U, pcat2, rcat>& rhs) noexcept
{
    return lhs != nullptr && &lhs >= &rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat1, ospf::ReferenceCategory rcat2>
inline constexpr decltype(auto) operator<(const ospf::reference::Ref<T, rcat1>& lhs, const ospf::PtrOrRef<U, pcat, rcat2>& rhs) noexcept
{
    return *lhs < *rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat1, ospf::ReferenceCategory rcat2>
inline constexpr decltype(auto) operator<=(const ospf::reference::Ref<T, rcat1>& lhs, const ospf::PtrOrRef<U, pcat, rcat2>& rhs) noexcept
{
    return *lhs <= *rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat1, ospf::ReferenceCategory rcat2>
inline constexpr decltype(auto) operator>(const ospf::reference::Ref<T, rcat1>& lhs, const ospf::PtrOrRef<U, pcat, rcat2>& rhs) noexcept
{
    return *lhs > *rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat1, ospf::ReferenceCategory rcat2>
inline constexpr decltype(auto) operator>=(const ospf::reference::Ref<T, rcat1>& lhs, const ospf::PtrOrRef<U, pcat, rcat2>& rhs) noexcept
{
    return *lhs >= *rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat>
inline constexpr decltype(auto) operator<(const T& lhs, const ospf::PtrOrRef<U, pcat, rcat>& rhs) noexcept
{
    return lhs < *rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat>
inline constexpr decltype(auto) operator<=(const T& lhs, const ospf::PtrOrRef<U, pcat, rcat>& rhs) noexcept
{
    return lhs <= *rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat>
inline constexpr decltype(auto) operator>(const T& lhs, const ospf::PtrOrRef<U, pcat, rcat>& rhs) noexcept
{
    return lhs > *rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat>
inline constexpr decltype(auto) operator>=(const T& lhs, const ospf::PtrOrRef<U, pcat, rcat>& rhs) noexcept
{
    return lhs >= *rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat>
inline constexpr decltype(auto) operator<(const std::optional<T>& lhs, const ospf::PtrOrRef<U, pcat, rcat>& rhs)
{
    return *lhs < *rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat>
inline constexpr decltype(auto) operator<=(const std::optional<T>& lhs, const ospf::PtrOrRef<U, pcat, rcat>& rhs)
{
    return *lhs <= *rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat>
inline constexpr decltype(auto) operator>(const std::optional<T>& lhs, const ospf::PtrOrRef<U, pcat, rcat>& rhs)
{
    return *lhs > *rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat>
inline constexpr decltype(auto) operator>=(const std::optional<T>& lhs, const ospf::PtrOrRef<U, pcat, rcat>& rhs)
{
    return *lhs >= *rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat1, ospf::ReferenceCategory rcat2, ospf::CopyOnWrite cow>
inline constexpr decltype(auto) operator<=>(const ospf::ValOrRef<T, rcat1, cow>& lhs, const ospf::PtrOrRef<U, pcat, rcat2>& rhs) noexcept
{
    return *lhs <=> *rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat1, ospf::PointerCategory pcat2, ospf::ReferenceCategory rcat>
inline constexpr ospf::RetType<std::compare_three_way_result_t<T, U>> operator<=>(const ospf::pointer::Ptr<T, pcat1>& lhs, const ospf::PtrOrRef<U, pcat2, rcat>& rhs) noexcept
{
    if (lhs != nullptr)
    {
        return *lhs <=> *rhs;
    }
    else
    {
        return static_cast<bool>(lhs) <=> true;
    }
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat1, ospf::ReferenceCategory rcat2>
inline constexpr decltype(auto) operator<=>(const ospf::reference::Ref<T, rcat1>& lhs, const ospf::PtrOrRef<U, pcat, rcat2>& rhs) noexcept
{
    return *lhs <=> *rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat>
inline constexpr decltype(auto) operator<=>(const T& lhs, const ospf::PtrOrRef<U, pcat, rcat>& rhs) noexcept
{
    return lhs <=> *rhs;
}

template<typename T, typename U, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat>
inline constexpr ospf::RetType<std::compare_three_way_result_t<T, U>> operator<=>(const std::optional<T>& lhs, const ospf::PtrOrRef<U, pcat, rcat>& rhs) noexcept
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

template<typename T, typename U, ospf::PointerCategory pcat1, ospf::PointerCategory pcat2, ospf::ReferenceCategory rcat1, ospf::ReferenceCategory rcat2>
inline constexpr ospf::RetType<std::compare_three_way_result_t<T, U>> operator<=>(const std::optional<ospf::PtrOrRef<T, pcat1, rcat1>>& lhs, const std::optional<ospf::PtrOrRef<U, pcat2, rcat2>>& rhs) noexcept
{
    if (lhs.has_value() && rhs.has_value())
    {
        return *lhs <=> *rhs;
    }
    else
    {
        return static_cast<bool>(lhs) <=> static_cast<bool>(rhs);
    }
}

template<typename T, typename U, ospf::PointerCategory pcat1, ospf::PointerCategory pcat2, ospf::ReferenceCategory rcat1, ospf::ReferenceCategory rcat2>
inline constexpr ospf::RetType<std::compare_three_way_result_t<T, U>> operator<=>(const std::optional<const ospf::PtrOrRef<T, pcat1, rcat1>>& lhs, const std::optional<const ospf::PtrOrRef<U, pcat2, rcat2>>& rhs) noexcept
{
    if (lhs.has_value() && rhs.has_value())
    {
        return *lhs <=> *rhs;
    }
    else
    {
        return static_cast<bool>(lhs) <=> static_cast<bool>(rhs);
    }
}

namespace std
{
    template<typename T, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat>
    struct hash<ospf::PtrOrRef<T, pcat, rcat>>
    {
        using ValueType = ospf::PtrOrRef<T, pcat, rcat>;

        inline constexpr const ospf::usize operator()(ospf::ArgCLRefType<ValueType> value) const noexcept
        {
            static const hash<T> hasher{};
            return hasher(*value);
        }
    };

    template<typename T, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat, ospf::CharType CharT>
    struct formatter<ospf::PtrOrRef<T, pcat, rcat>, CharT>
        : public formatter<T, CharT>
    {
        using ValueType = ospf::PtrOrRef<T, pcat, rcat>;

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
    template<typename T, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat>
        requires WithTag<T>
    struct TagValue<PtrOrRef<T, pcat, rcat>>
    {
        using Type = typename TagValue<T>::Type;
        using ValueType = PtrOrRef<T, pcat, rcat>;

        inline constexpr RetType<Type> value(ArgCLRefType<ValueType> value) const noexcept
        {
            static constexpr const TagValue<T> extractor{};
            return extractor(*value);
        }
    };
};

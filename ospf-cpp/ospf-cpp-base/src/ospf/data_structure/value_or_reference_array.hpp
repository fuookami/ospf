#pragma once

#include <ospf/basic_definition.hpp>
#include <ospf/concepts/with_default.hpp>
#include <ospf/functional/value_or_reference.hpp>
#include <ospf/literal_constant.hpp>
#include <ospf/memory/reference.hpp>
#include <ospf/functional/iterator.hpp>
#include <boost/iterator/transform_iterator.hpp>

namespace ospf
{
    inline namespace data_structure
    {
        namespace value_or_reference_array
        {
            template<
                typename T,
                usize len,
                ReferenceCategory cat,
                CopyOnWrite cow,
                template<typename, usize> class C
            >
                requires NotSameAs<T, void>
            class StaticValueOrReferenceArray;

            template<
                typename T,
                ReferenceCategory cat,
                CopyOnWrite cow,
                template<typename> class C
            >
                requires NotSameAs<T, void>
            class DynamicValueOrReferenceArray;

            template<typename T, typename C>
            class ValueOrReferenceArrayConstIterator
                : public RandomIteratorImpl<ConstType<T>, typename OriginType<C>::const_iterator, ValueOrReferenceArrayConstIterator<T, C>>
            {
                template<
                    typename T,
                    usize len,
                    ReferenceCategory cat,
                    CopyOnWrite cow,
                    template<typename, usize> class C
                >
                    requires NotSameAs<T, void>
                friend class StaticValueOrReferenceArray;

                template<
                    typename T,
                    ReferenceCategory cat,
                    CopyOnWrite cow,
                    template<typename> class C
                >
                    requires NotSameAs<T, void>
                friend class DynamicValueOrReferenceArray;

            public:
                using ValueType = OriginType<T>;
                using ContainerType = OriginType<C>;
                using IterType = typename ContainerType::const_iterator;

            private:
                using Impl = RandomIteratorImpl<ConstType<ValueType>, IterType, ValueOrReferenceArrayConstIterator<T, C>>;

            public:
                constexpr ValueOrReferenceArrayConstIterator(ArgCLRefType<IterType> iter)
                    : Impl(iter) {}
                constexpr ValueOrReferenceArrayConstIterator(const ValueOrReferenceArrayConstIterator& ano) = default;
                constexpr ValueOrReferenceArrayConstIterator(ValueOrReferenceArrayConstIterator&& ano) noexcept = default;
                constexpr ValueOrReferenceArrayConstIterator& operator=(const ValueOrReferenceArrayConstIterator& rhs) = default;
                constexpr ValueOrReferenceArrayConstIterator& operator=(ValueOrReferenceArrayConstIterator&& rhs) noexcept = default;
                constexpr ~ValueOrReferenceArrayConstIterator(void) noexcept = default;

            OSPF_CRTP_PERMISSION:
                inline static constexpr CLRefType<ValueType> OSPF_CRTP_FUNCTION(get)(ArgCLRefType<IterType> iter) noexcept
                {
                    return **iter;
                }

                inline static constexpr RetType<ValueOrReferenceArrayConstIterator> OSPF_CRTP_FUNCTION(construct)(ArgCLRefType<IterType> iter) noexcept
                {
                    return ValueOrReferenceArrayConstIterator{ iter };
                }
            };

            template<typename T, typename C>
            class ValueOrReferenceArrayIterator
                : public RandomIteratorImpl<OriginType<T>, typename OriginType<C>::iterator, ValueOrReferenceArrayIterator<T, C>>
            {
                template<
                    typename T,
                    usize len,
                    ReferenceCategory cat,
                    CopyOnWrite cow,
                    template<typename, usize> class C
                >
                    requires NotSameAs<T, void>
                friend class StaticValueOrReferenceArray;

                template<
                    typename T,
                    ReferenceCategory cat,
                    CopyOnWrite cow,
                    template<typename> class C
                >
                    requires NotSameAs<T, void>
                friend class DynamicValueOrReferenceArray;

            public:
                using ValueType = OriginType<T>;
                using ContainerType = OriginType<C>;
                using IterType = typename ContainerType::iterator;

            private:
                using Impl = RandomIteratorImpl<ValueType, IterType, ValueOrReferenceArrayIterator<T, C>>;

            public:
                constexpr ValueOrReferenceArrayIterator(ArgCLRefType<IterType> iter)
                    : Impl(iter) {}
                constexpr ValueOrReferenceArrayIterator(const ValueOrReferenceArrayIterator& ano) = default;
                constexpr ValueOrReferenceArrayIterator(ValueOrReferenceArrayIterator&& ano) noexcept = default;
                constexpr ValueOrReferenceArrayIterator& operator=(const ValueOrReferenceArrayIterator& rhs) = default;
                constexpr ValueOrReferenceArrayIterator& operator=(ValueOrReferenceArrayIterator&& rhs) noexcept = default;
                constexpr ~ValueOrReferenceArrayIterator(void) noexcept = default;

            public:
                inline constexpr operator RetType<ValueOrReferenceArrayIterator<T, C>>(void) const noexcept
                {
                    return ValueOrReferenceArrayIterator<T, C>{ this->_iter };
                }

            OSPF_CRTP_PERMISSION:
                inline static constexpr LRefType<ValueType> OSPF_CRTP_FUNCTION(get)(ArgCLRefType<IterType> iter) noexcept
                {
                    return **iter;
                }

                inline static constexpr RetType<ValueOrReferenceArrayIterator> OSPF_CRTP_FUNCTION(construct)(ArgCLRefType<IterType> iter) noexcept
                {
                    return ValueOrReferenceArrayIterator{ iter };
                }
            };

            template<typename T, typename C>
            class ValueOrReferenceArrayConstReverseIterator
                : public RandomIteratorImpl<ConstType<T>, typename OriginType<C>::const_reverse_iterator, ValueOrReferenceArrayConstReverseIterator<T, C>>
            {
                template<
                    typename T,
                    usize len,
                    ReferenceCategory cat,
                    CopyOnWrite cow,
                    template<typename, usize> class C
                >
                    requires NotSameAs<T, void>
                friend class StaticValueOrReferenceArray;

                template<
                    typename T,
                    ReferenceCategory cat,
                    CopyOnWrite cow,
                    template<typename> class C
                >
                    requires NotSameAs<T, void>
                friend class DynamicValueOrReferenceArray;

            public:
                using ValueType = OriginType<T>;
                using ContainerType = OriginType<C>;
                using IterType = typename ContainerType::const_reverse_iterator;

            private:
                using Impl = RandomIteratorImpl<ConstType<ValueType>, IterType, ValueOrReferenceArrayConstReverseIterator<T, C>>;

            public:
                constexpr ValueOrReferenceArrayConstReverseIterator(ArgCLRefType<IterType> iter)
                    : Impl(iter) {}
                constexpr ValueOrReferenceArrayConstReverseIterator(const ValueOrReferenceArrayConstReverseIterator& ano) = default;
                constexpr ValueOrReferenceArrayConstReverseIterator(ValueOrReferenceArrayConstReverseIterator&& ano) noexcept = default;
                constexpr ValueOrReferenceArrayConstReverseIterator& operator=(const ValueOrReferenceArrayConstReverseIterator& rhs) = default;
                constexpr ValueOrReferenceArrayConstReverseIterator& operator=(ValueOrReferenceArrayConstReverseIterator&& rhs) noexcept = default;
                constexpr ~ValueOrReferenceArrayConstReverseIterator(void) noexcept = default;

            OSPF_CRTP_PERMISSION:
                inline static constexpr CLRefType<ValueType> OSPF_CRTP_FUNCTION(get)(ArgCLRefType<IterType> iter) noexcept
                {
                    return **iter;
                }

                inline static constexpr RetType<ValueOrReferenceArrayConstReverseIterator> OSPF_CRTP_FUNCTION(construct)(ArgCLRefType<IterType> iter) noexcept
                {
                    return ValueOrReferenceArrayConstReverseIterator{ iter };
                }
            };

            template<typename T, typename C>
            class ValueOrReferenceArrayReverseIterator
                : public RandomIteratorImpl<OriginType<T>, typename OriginType<C>::reverse_iterator, ValueOrReferenceArrayReverseIterator<T, C>>
            {
                template<
                    typename T,
                    usize len,
                    ReferenceCategory cat,
                    CopyOnWrite cow,
                    template<typename, usize> class C
                >
                    requires NotSameAs<T, void>
                friend class StaticValueOrReferenceArray;

                template<
                    typename T,
                    ReferenceCategory cat,
                    CopyOnWrite cow,
                    template<typename> class C
                >
                    requires NotSameAs<T, void>
                friend class DynamicValueOrReferenceArray;

            public:
                using ValueType = OriginType<T>;
                using ContainerType = OriginType<C>;
                using IterType = typename ContainerType::reverse_iterator;

            private:
                using Impl = RandomIteratorImpl<ValueType, IterType, ValueOrReferenceArrayReverseIterator<T, C>>;

            public:
                constexpr ValueOrReferenceArrayReverseIterator(ArgCLRefType<IterType> iter)
                    : Impl(iter) {}
                constexpr ValueOrReferenceArrayReverseIterator(const ValueOrReferenceArrayReverseIterator& ano) = default;
                constexpr ValueOrReferenceArrayReverseIterator(ValueOrReferenceArrayReverseIterator&& ano) noexcept = default;
                constexpr ValueOrReferenceArrayReverseIterator& operator=(const ValueOrReferenceArrayReverseIterator& rhs) = default;
                constexpr ValueOrReferenceArrayReverseIterator& operator=(ValueOrReferenceArrayReverseIterator&& rhs) noexcept = default;
                constexpr ~ValueOrReferenceArrayReverseIterator(void) noexcept = default;

            OSPF_CRTP_PERMISSION:
                inline static constexpr LRefType<ValueType> OSPF_CRTP_FUNCTION(get)(ArgCLRefType<IterType> iter) noexcept
                {
                    return **iter;
                }

                inline static constexpr RetType<ValueOrReferenceArrayReverseIterator> OSPF_CRTP_FUNCTION(construct)(ArgCLRefType<IterType> iter) noexcept
                {
                    return ValueOrReferenceArrayReverseIterator{ iter };
                }
            };

            template<typename T, typename C>
            struct ValueOrReferenceArrayAccessPolicy;

            template<
                typename T,
                usize len,
                ReferenceCategory cat,
                CopyOnWrite cow,
                template<typename, usize> class C
            >
            struct ValueOrReferenceArrayAccessPolicy<T, C<ValOrRef<OriginType<T>, cat, cow>, len>>
            {
            public:
                using ValueType = OriginType<T>;
                using ValueOrReferenceType = ValOrRef<ValueType, cat, cow>;
                using ReferenceType = typename ValueOrReferenceType::ReferenceType;
                using ContainerType = C<ValueOrReferenceType, len>;
                using IterType = ValueOrReferenceArrayIterator<ValueType, ContainerType>;
                using ConstIterType = ValueOrReferenceArrayConstIterator<ValueType, ContainerType>;
                using ReverseIterType = ValueOrReferenceArrayReverseIterator<ValueType, ContainerType>;
                using ConstReverseIterType = ValueOrReferenceArrayConstReverseIterator<ValueType, ContainerType>;

            public:
                inline static constexpr const usize size(CLRefType<ContainerType> array) noexcept
                {
                    return len;
                }

                inline static constexpr LRefType<ValueType> get(LRefType<ContainerType> array, const usize i)
                {
                    return *array.at(i);
                }

                inline static constexpr CLRefType<ValueType> get(CLRefType<ContainerType> array, const usize i)
                {
                    return *array.at(i);
                }

                inline static constexpr LRefType<ValueType> get_unchecked(LRefType<ContainerType> array, const usize i)
                {
                    return *array[i];
                }

                inline static constexpr CLRefType<ValueType> get_unchecked(CLRefType<ContainerType> array, const usize i)
                {
                    return *array[i];
                }

            public:
                inline static constexpr RetType<IterType> begin(LRefType<ContainerType> array) noexcept
                {
                    return IterType{ array.begin() };
                }

                inline static constexpr RetType<ConstIterType> cbegin(CLRefType<ContainerType> array) noexcept
                {
                    return ConstIterType{ array.begin() };
                }

            public:
                inline static constexpr RetType<IterType> end(LRefType<ContainerType> array) noexcept
                {
                    return IterType{ array.end() };
                }

                inline static constexpr RetType<ConstIterType> cend(CLRefType<ContainerType> array) noexcept
                {
                    return ConstIterType{ array.end() };
                }

            public:
                inline static constexpr RetType<ReverseIterType> rbegin(LRefType<ContainerType> array) noexcept
                {
                    return ReverseIterType{ array.rbegin() };
                }

                inline static constexpr RetType<ConstReverseIterType> crbegin(CLRefType<ContainerType> array) noexcept
                {
                    return ConstReverseIterType{ array.rbegin() };
                }

            public:
                inline static constexpr RetType<ReverseIterType> rend(LRefType<ContainerType> array) noexcept
                {
                    return ReverseIterType{ array.rend() };
                }

                inline static constexpr RetType<ConstReverseIterType> crend(CLRefType<ContainerType> array) noexcept
                {
                    return ConstReverseIterType{ array.rend() };
                }
            };

            template<
                typename T,
                ReferenceCategory cat,
                CopyOnWrite cow,
                typename Alloc,
                template<typename, typename> class C
            >
            struct ValueOrReferenceArrayAccessPolicy<T, C<ValOrRef<OriginType<T>, cat, cow>, Alloc>>
            {
            public:
                using ValueType = OriginType<T>;
                using ValueOrReferenceType = ValOrRef<ValueType, cat, cow>;
                using ReferenceType = typename ValueOrReferenceType::ReferenceType;
                using ContainerType = C<ValueOrReferenceType, Alloc>;
                using IterType = ValueOrReferenceArrayIterator<ValueType, ContainerType>;
                using ConstIterType = ValueOrReferenceArrayConstIterator<ValueType, ContainerType>;
                using ReverseIterType = ValueOrReferenceArrayReverseIterator<ValueType, ContainerType>;
                using ConstReverseIterType = ValueOrReferenceArrayConstReverseIterator<ValueType, ContainerType>;

            public:
                inline static constexpr const usize size(CLRefType<ContainerType> array) noexcept
                {
                    return array.size();
                }

                inline static constexpr LRefType<ValueType> get(LRefType<ContainerType> array, const usize i)
                {
                    return *array.at(i);
                }

                inline static constexpr CLRefType<ValueType> get(CLRefType<ContainerType> array, const usize i)
                {
                    return *array.at(i);
                }

                inline static constexpr LRefType<ValueType> get_unchecked(LRefType<ContainerType> array, const usize i)
                {
                    return *array[i];
                }

                inline static constexpr CLRefType<ValueType> get_unchecked(CLRefType<ContainerType> array, const usize i)
                {
                    return *array[i];
                }

            public:
                inline static constexpr RetType<IterType> begin(LRefType<ContainerType> array) noexcept
                {
                    return IterType{ array.begin() };
                }

                inline static constexpr RetType<ConstIterType> cbegin(CLRefType<ContainerType> array) noexcept
                {
                    return ConstIterType{ array.cbegin() };
                }

            public:
                inline static constexpr RetType<IterType> end(LRefType<ContainerType> array) noexcept
                {
                    return IterType{ array.end() };
                }

                inline static constexpr RetType<ConstIterType> cend(CLRefType<ContainerType> array) noexcept
                {
                    return ConstIterType{ array.cend() };
                }

            public:
                inline static constexpr RetType<ReverseIterType> rbegin(LRefType<ContainerType> array) noexcept
                {
                    return ReverseIterType{ array.rbegin() };
                }

                inline static constexpr RetType<ConstReverseIterType> crbegin(CLRefType<ContainerType> array) noexcept
                {
                    return ConstReverseIterType{ array.crbegin() };
                }

            public:
                inline static constexpr RetType<ReverseIterType> rend(LRefType<ContainerType> array) noexcept
                {
                    return ReverseIterType{ array.rend() };
                }

                inline static constexpr RetType<ConstReverseIterType> crend(CLRefType<ContainerType> array) noexcept
                {
                    return ConstReverseIterType{ array.crend() };
                }
            };

            template<typename T, typename C, typename Self>
            class ValueOrReferenceArrayImpl
            {
                OSPF_CRTP_IMPL;

            public:
                using ValueType = OriginType<T>;
                using ContainerType = OriginType<C>;
                using AccessPolicyType = ValueOrReferenceArrayAccessPolicy<ValueType, ContainerType>;
                using ValueOrReferenceType = typename AccessPolicyType::ValueOrReferenceType;
                using ReferenceType = typename AccessPolicyType::ReferenceType;
                using IterType = typename AccessPolicyType::IterType;
                using ConstIterType = typename AccessPolicyType::ConstIterType;
                using ReverseIterType = typename AccessPolicyType::ReverseIterType;
                using ConstReverseIterType = typename AccessPolicyType::ConstReverseIterType;

            protected:
                constexpr ValueOrReferenceArrayImpl(void) = default;

            public:
                constexpr ValueOrReferenceArrayImpl(const ValueOrReferenceArrayImpl& ano) = default;
                constexpr ValueOrReferenceArrayImpl(ValueOrReferenceArrayImpl&& ano) noexcept = default;
                constexpr ValueOrReferenceArrayImpl& operator=(const ValueOrReferenceArrayImpl& rhs) = default;
                constexpr ValueOrReferenceArrayImpl& operator=(ValueOrReferenceArrayImpl&& rhs) noexcept = default;
                constexpr ~ValueOrReferenceArrayImpl(void) noexcept = default;

            public:
                inline constexpr LRefType<ValueType> at(const usize i)
                {
                    return AccessPolicyType::get(container(), i);
                }

                inline constexpr CLRefType<ValueType> at(const usize i) const
                {
                    return AccessPolicyType::get(const_container(), i);
                }

                inline constexpr LRefType<ValueType> operator[](const usize i)
                {
                    return AccessPolicyType::get_unchecked(container(), i);
                }

                inline constexpr CLRefType<ValueType> operator[](const usize i) const
                {
                    return AccessPolicyType::get_unchecked(const_container(), i);
                }

            public:
                inline constexpr LRefType<ValueType> front(void)
                {
                    return at(0_uz);
                }

                inline constexpr CLRefType<ValueType> front(void) const
                {
                    return at(0_uz);
                }

                inline constexpr LRefType<ValueType> back(void)
                {
                    return at(size() - 1_uz);
                }

                inline constexpr CLRefType<ValueType> back(void) const
                {
                    return at(size() - 1_uz);
                }

            public:
                inline constexpr RetType<IterType> begin(void) noexcept
                {
                    return AccessPolicyType::begin(container());
                }

                inline constexpr RetType<ConstIterType> begin(void) const noexcept
                {
                    return AccessPolicyType::cbegin(const_container());
                }

                inline constexpr RetType<ConstIterType> cbegin(void) const noexcept
                {
                    return AccessPolicyType::cbegin(const_container());
                }

            public:
                inline constexpr RetType<IterType> end(void) noexcept
                {
                    return AccessPolicyType::end(container());
                }

                inline constexpr RetType<ConstIterType> end(void) const noexcept
                {
                    return AccessPolicyType::cend(const_container());
                }

                inline constexpr RetType<ConstIterType> cend(void) const noexcept
                {
                    return AccessPolicyType::cend(const_container());
                }

            public:
                inline constexpr RetType<ReverseIterType> rbegin(void) noexcept
                {
                    return AccessPolicyType::rbegin(container());
                }

                inline constexpr RetType<ConstReverseIterType> rbegin(void) const noexcept
                {
                    return AccessPolicyType::crbegin(const_container());
                }

                inline constexpr RetType<ConstReverseIterType> crbegin(void) const noexcept
                {
                    return AccessPolicyType::crbegin(const_container());
                }

            public:
                inline constexpr const bool empty(void) const noexcept
                {
                    return size() == 0_uz;
                }

                inline constexpr const usize size(void) const noexcept
                {
                    return AccessPolicyType::size(const_container());
                }

            public:
                inline void swap(ValueOrReferenceArrayImpl& ano) noexcept
                {
                    std::swap(container(), ano.container());
                }

            protected:
                inline constexpr LRefType<ContainerType> container(void) noexcept
                {
                    return Trait::get_container(self());
                }

                inline constexpr CLRefType<ContainerType> const_container(void) const noexcept
                {
                    return Trait::get_const_container(self());
                }

            private:
                struct Trait : public Self
                {
                    inline static constexpr LRefType<ContainerType> get_container(Self& self) noexcept
                    {
                        static const auto get_impl = &Self::OSPF_CRTP_FUNCTION(get_container);
                        return (self.*get_impl)();
                    }

                    inline static constexpr CLRefType<ContainerType> get_const_container(const Self& self) noexcept
                    {
                        static const auto get_impl = &Self::OSPF_CRTP_FUNCTION(get_const_container);
                        return (self.*get_impl)();
                    }
                };
            };

            template<
                typename T,
                usize len,
                ReferenceCategory cat,
                CopyOnWrite cow,
                template<typename, usize> class C
            >
                requires NotSameAs<T, void>
            class StaticValueOrReferenceArray
                : public ValueOrReferenceArrayImpl<T, C<ValOrRef<OriginType<T>, cat, cow>, len>, StaticValueOrReferenceArray<T, len, cat, cow, C>>
            {
                using Impl = ValueOrReferenceArrayImpl<T, C<ValOrRef<OriginType<T>, cat, cow>, len>, StaticValueOrReferenceArray<T, len, cat, cow, C>>;

                template<
                    typename T,
                    ReferenceCategory cat,
                    CopyOnWrite cow,
                    template<typename> class C
                >
                    requires NotSameAs<T, void>
                friend class DynamicValueOrReferenceArray;

            public:
                using typename Impl::ValueType;
                using typename Impl::ReferenceType;
                using typename Impl::ValueOrReferenceType;
                using typename Impl::ContainerType;
                using typename Impl::IterType;
                using typename Impl::ConstIterType;
                using typename Impl::ReverseIterType;
                using typename Impl::ConstReverseIterType;

            public:
                template<typename = void>
                    requires std::default_initializable<ValueOrReferenceType>
                constexpr StaticValueOrReferenceArray(void)
                    : _container(make_array<ValueOrReferenceType, len>([](const usize _) { return ValueOrReferenceType{}; })) {}

                template<typename = void>
                    requires (!std::default_initializable<ValueOrReferenceType> && WithDefault<ValueOrReferenceType> && std::copyable<ValueOrReferenceType>)
                constexpr StaticValueOrReferenceArray(void)
                    : _container(make_array<ValueOrReferenceType, len>(DefaultValue<ValueOrReferenceType>::value())) {}

                constexpr StaticValueOrReferenceArray(ArgRRefType<ContainerType> container)
                    : _container(move<ContainerType>(container)) {}

            public:
                constexpr StaticValueOrReferenceArray(const StaticValueOrReferenceArray& ano) = default;
                constexpr StaticValueOrReferenceArray(StaticValueOrReferenceArray&& ano) noexcept = default;
                constexpr StaticValueOrReferenceArray& operator=(const StaticValueOrReferenceArray& rhs) = default;
                constexpr StaticValueOrReferenceArray& operator=(StaticValueOrReferenceArray&& rhs) noexcept = default;
                constexpr ~StaticValueOrReferenceArray(void) = default;

            public:
                template<typename = void>
                    requires requires (ContainerType& container) { { container.data() } -> DecaySameAs<PtrType<ValueOrReferenceType>>; }
                inline constexpr const PtrType<ValueOrReferenceType> data(void) noexcept
                {
                    return _container.data();
                }

                template<typename = void>
                    requires requires (const ContainerType& container) { { container.data() } -> DecaySameAs<CPtrType<ValueOrReferenceType>>; }
                inline constexpr const CPtrType<ValueOrReferenceType> data(void) const noexcept
                {
                    return _container.data();
                }

            public:
                inline constexpr const usize max_size(void) const noexcept
                {
                    return _container.max_size();
                }

            public:
                template<typename = void>
                    requires std::copyable<ValueOrReferenceType>
                inline constexpr void fill(ArgCLRefType<ValueOrReferenceType> value) noexcept
                {
                    _container.fill(value);
                }

                template<typename = void>
                    requires std::copyable<ValueOrReferenceType>
                inline constexpr void fill_value(ArgCLRefType<ValueType> value) noexcept
                {
                    _container.fill(ValueOrReferenceType::value(value));
                }

                template<typename = void>
                    requires std::copyable<ValueOrReferenceType> && ReferenceFaster<ValueType> && std::movable<ValueType>
                inline constexpr void fill_value(ArgRRefType<ValueType> value) noexcept
                {
                    _container.fill(ValueOrReferenceType::value(move<ValueType>(value)));
                }

                template<typename = void>
                    requires std::copyable<ValueOrReferenceType>
                inline constexpr void fill_reference(CLRefType<ValueType> ref) noexcept
                {
                    _container.fill(ValueOrReferenceType::ref(ref));
                }

                template<typename = void>
                    requires std::copyable<ValueOrReferenceType>
                inline constexpr void fill_reference(ArgCLRefType<ReferenceType> ref) noexcept
                {
                    _container.fill(ValueOrReferenceType::ref(ref));
                }

                template<typename = void>
                    requires std::copyable<ValueOrReferenceType> && ReferenceFaster<ReferenceType> && std::movable<ReferenceType>
                inline constexpr void fill_reference(ArgRRefType<ReferenceType> ref) noexcept
                {
                    _container.fill(ValueOrReferenceType::ref(move<ReferenceType>(ref)));
                }

            public:
                inline constexpr const bool operator==(const StaticValueOrReferenceArray& rhs) const noexcept
                {
                    return _container == rhs._container;
                }

                inline constexpr const bool operator!=(const StaticValueOrReferenceArray& rhs) const noexcept
                {
                    return _container != rhs._container;
                }

            public:
                inline constexpr const bool operator<(const StaticValueOrReferenceArray& rhs) const noexcept
                {
                    return _container < rhs._container;
                }

                inline constexpr const bool operator<=(const StaticValueOrReferenceArray& rhs) const noexcept
                {
                    return _container <= rhs._container;
                }

                inline constexpr const bool operator>(const StaticValueOrReferenceArray& rhs) const noexcept
                {
                    return _container > rhs._container;
                }

                inline constexpr const bool operator>=(const StaticValueOrReferenceArray& rhs) const noexcept
                {
                    return _container >= rhs._container;
                }

            public:
                inline constexpr decltype(auto) operator<=>(const StaticValueOrReferenceArray& rhs) const noexcept
                {
                    return _container <=> rhs._container;
                }

            OSPF_CRTP_PERMISSION:
                inline constexpr LRefType<ContainerType> OSPF_CRTP_FUNCTION(get_container)(void) noexcept
                {
                    return _container;
                }

                inline constexpr CLRefType<ContainerType> OSPF_CRTP_FUNCTION(get_const_container)(void) const noexcept
                {
                    return _container;
                }

            private:
                ContainerType _container;
            };

            template<
                typename T,
                ReferenceCategory cat,
                CopyOnWrite cow,
                template<typename> class C
            >
                requires NotSameAs<T, void>
            class DynamicValueOrReferenceArray
                : public ValueOrReferenceArrayImpl<T, C<ValOrRef<OriginType<T>, cat, cow>>, DynamicValueOrReferenceArray<T, cat, cow, C>>
            {
                using Impl = ValueOrReferenceArrayImpl<T, C<ValOrRef<OriginType<T>, cat, cow>>, DynamicValueOrReferenceArray<T, cat, cow, C>>;

            public:
                using typename Impl::ValueType;
                using typename Impl::ReferenceType;
                using typename Impl::ValueOrReferenceType;
                using typename Impl::ContainerType;
                using typename Impl::IterType;
                using typename Impl::ConstIterType;
                using typename Impl::ReverseIterType;
                using typename Impl::ConstReverseIterType;

            public:
                constexpr DynamicValueOrReferenceArray(void) = default;

                template<typename = void>
                    requires std::default_initializable<ValueOrReferenceType>
                constexpr DynamicValueOrReferenceArray(const usize length)
                    : _container(length) {}

                template<typename = void>
                    requires (!std::default_initializable<ValueOrReferenceType> && WithDefault<ValueOrReferenceType> && std::copyable<ValueOrReferenceType>)
                constexpr DynamicValueOrReferenceArray(const usize length)
                    : _container(length, DefaultValue<ValueOrReferenceType>::value()) {}

                template<typename = void>
                    requires std::copyable<ValueOrReferenceType>
                constexpr DynamicValueOrReferenceArray(const usize length, ArgCLRefType<ValueOrReferenceType> value)
                    : _container(length, value) {}

                template<std::input_iterator It>
                    requires requires (const It it) { { *it } -> DecaySameAs<ValueOrReferenceType>; }
                constexpr DynamicValueOrReferenceArray(It&& first, It&& last)
                    : _container(first, last) {}

                constexpr DynamicValueOrReferenceArray(std::initializer_list<ValueType> values)
                    : _container
                    (
                        boost::make_transform_iterator(values.begin(), [](ValueType& value) { return ValueOrReferenceType::value(move<ValueType>(value)); }),
                        boost::make_transform_iterator(values.end(), [](ValueType& value) { return ValueOrReferenceType::value(move<ValueType>(value)); })
                    ) {}

                constexpr DynamicValueOrReferenceArray(std::initializer_list<ValueOrReferenceType> values)
                    : _container(std::move(values)) {}

                constexpr DynamicValueOrReferenceArray(ArgRRefType<ContainerType> container)
                    : _container(move<ContainerType>(container)) {}

            public:
                constexpr DynamicValueOrReferenceArray(const DynamicValueOrReferenceArray& ano) = default;
                constexpr DynamicValueOrReferenceArray(DynamicValueOrReferenceArray&& ano) noexcept = default;
                constexpr DynamicValueOrReferenceArray& operator=(const DynamicValueOrReferenceArray& rhs) = default;
                constexpr DynamicValueOrReferenceArray& operator=(DynamicValueOrReferenceArray&& rhs) noexcept = default;
                constexpr ~DynamicValueOrReferenceArray(void) noexcept = default;

            public:
                template<typename = void>
                    requires std::default_initializable<ValueOrReferenceType>
                inline constexpr void assign(const usize length)
                {
                    if constexpr (std::copyable<ValueOrReferenceType>)
                    {
                        _container.assign(length, ValueOrReferenceType{});
                    }
                    else
                    {
                        _container.clear();
                        for (auto i{ 0_uz }; i != length; ++i)
                        {
                            _container.push_back(ValueOrReferenceType{});
                        }
                    }
                }

                template<typename = void>
                    requires (!std::default_initializable<ValueOrReferenceType> && WithDefault<ValueOrReferenceType> && std::copyable<ValueOrReferenceType>)
                inline constexpr void assign(const usize length)
                {
                    _container.assign(length, DefaultValue<ValueOrReferenceType>::value());
                }

                template<typename = void>
                    requires std::copyable<ValueOrReferenceType>
                inline constexpr void assign(const usize length, ArgCLRefType<ValueOrReferenceType> value)
                {
                    _container.assign(length, value);
                }

                template<std::input_iterator It>
                    requires requires (const It& it) { { *it } -> DecaySameAs<ValueOrReferenceType>; }
                inline constexpr void assign(It&& first, It&& last)
                {
                    _container.assign(std::forward<It>(first), std::forward<It>(last));
                }

                inline constexpr void assign(std::initializer_list<ValueType> values)
                {
                    _container.assign
                    (
                        boost::make_transform_iterator(values.begin(), [](ValueType& value) { return ValueOrReferenceType::value(move<ValueType>(value)); }),
                        boost::make_transform_iterator(values.end(), [](ValueType& value) { return ValueOrReferenceType::value(move<ValueType>(value)); })
                    );
                }

                inline constexpr void assign(std::initializer_list<ValueOrReferenceType> values)
                {
                    _container.assign(std::move(values));
                }

                template<
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::copyable<ValueOrReferenceType>
                inline constexpr void assign(const StaticValueOrReferenceArray<ValueType, len, cat, cow, C1>& values)
                {
                    assign(values._container.begin(), values._container.end());
                }

                template<
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::movable<ValueOrReferenceType>
                inline constexpr void assign(StaticValueOrReferenceArray<ValueType, len, cat, cow, C1>&& values)
                {
                    assign
                    (
                        boost::make_transform_iterator(values._container.begin(), [](ValueOrReferenceType& value) { return std::move(value); }),
                        boost::make_transform_iterator(values._container.end(), [](ValueOrReferenceType& value) { return std::move(value); })
                    );
                }

                template<
                    typename U,
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::convertible_to<U, ValueType> && std::convertible_to<PtrType<U>, PtrType<ValueType>>
                inline constexpr void assign(const StaticValueOrReferenceArray<U, len, cat, cow, C1>& values)
                {
                    assign
                    (
                        boost::make_transform_iterator(values._container.begin(), [](const ValOrRef<U, cat, cow>& value) { return ValueOrReferenceType{ value }; }),
                        boost::make_transform_iterator(values._container.end(), [](const ValOrRef<U, cat, cow>& value) { return ValueOrReferenceType{ value }; })
                    );
                }

                template<
                    typename U,
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::convertible_to<U, ValueType> && std::convertible_to<PtrType<U>, PtrType<ValueType>>
                inline constexpr void assign(StaticValueOrReferenceArray<U, len, cat, cow, C1>&& values)
                {
                    assign
                    (
                        boost::make_transform_iterator(values._container.begin(), [](ValOrRef<U, cat, cow>& value) { return ValueOrReferenceType{ std::move(value) }; }),
                        boost::make_transform_iterator(values._container.end(), [](ValOrRef<U, cat, cow>& value) { return ValueOrReferenceType{ std::move(value) }; })
                    );
                }

                template<template<typename> class C1>
                    requires std::copyable<ValueOrReferenceType>
                inline constexpr void assign(const DynamicValueOrReferenceArray<ValueType, cat, cow, C1>& values)
                {
                    assign(values._container.begin(), values._container.end());
                }

                template<template<typename> class C1>
                    requires std::movable<ValueOrReferenceType>
                inline constexpr void assign(DynamicValueOrReferenceArray<ValueType, cat, cow, C1>&& values)
                {
                    assign
                    (
                        boost::make_transform_iterator(values._container.begin(), [](ValueOrReferenceType& value) { return std::move(value); }),
                        boost::make_transform_iterator(values._container.end(), [](ValueOrReferenceType& value) { return std::move(value); })
                    );
                }

                template<
                    typename U,
                    usize len,
                    template<typename> class C1
                >
                    requires std::convertible_to<U, ValueType> && std::convertible_to<PtrType<U>, PtrType<ValueType>>
                inline constexpr void assign(const DynamicValueOrReferenceArray<U, cat, cow, C1>& values)
                {
                    assign
                    (
                        boost::make_transform_iterator(values._container.begin(), [](const ValOrRef<U, cat, cow>& value) { return ValueOrReferenceType{ value }; }),
                        boost::make_transform_iterator(values._container.end(), [](const ValOrRef<U, cat, cow>& value) { return ValueOrReferenceType{ value }; })
                    );
                }

                template<
                    typename U,
                    usize len,
                    template<typename> class C1
                >
                    requires std::convertible_to<U, ValueType> && std::convertible_to<PtrType<U>, PtrType<ValueType>>
                inline constexpr void assign(DynamicValueOrReferenceArray<U, cat, cow, C1>&& values)
                {
                    assign
                    (
                        boost::make_transform_iterator(values._container.begin(), [](ValOrRef<U, cat, cow>& value) { return ValueOrReferenceType{ std::move(value) }; }),
                        boost::make_transform_iterator(values._container.end(), [](ValOrRef<U, cat, cow>& value) { return ValueOrReferenceType{ std::move(value) }; })
                    );
                }

                template<typename = void>
                    requires std::copyable<ValueOrReferenceType>
                inline constexpr void assign_value(const usize length, ArgCLRefType<ValueType> value)
                {
                    _container.assign(length, ValueOrReferenceType::value(value));
                }

                template<typename = void>
                    requires std::copyable<ValueOrReferenceType> && ReferenceFaster<ValueType> && std::movable<ValueType>
                inline constexpr void assign_value(const usize length, ArgRRefType<ValueType> value)
                {
                    _container.assign(length, ValueOrReferenceType::value(move<ValueType>(value)));
                }

                template<std::input_iterator It>
                    requires requires (const It it) { { *it } -> DecaySameAs<ValueType>; }
                inline constexpr void assign_value(It&& first, It&& last)
                {
                    if constexpr (std::is_same_v<decltype(*first), CLRefType<ValueType>>)
                    {
                        _container.assign
                        (
                            boost::make_transform_iterator(std::forward<It>(first), [](ArgCLRefType<ValueType> value) { return ValueOrReferenceType::value(value); }),
                            boost::make_transform_iterator(std::forward<It>(last), [](ArgCLRefType<ValueType> value) { return ValueOrReferenceType::value(value); })
                        );
                    }
                    else
                    {
                        _container.assign
                        (
                            boost::make_transform_iterator(std::forward<It>(first), [](ArgLRefType<ValueType> value) { return ValueOrReferenceType::value(move<ValueType>(value)); }),
                            boost::make_transform_iterator(std::forward<It>(last), [](ArgLRefType<ValueType> value) { return ValueOrReferenceType::value(move<ValueType>(value)); })
                        );
                    }
                }

                inline constexpr void assign_value(std::initializer_list<ValueType> values)
                {
                    assign(std::move(values));
                }

                template<typename = void>
                    requires std::copyable<ValueOrReferenceType>
                inline constexpr void assign_reference(const usize length, CLRefType<ValueType> ref)
                {
                    _container.assign(length, ValueOrReferenceType::ref(ReferenceType{ ref }));
                }

                template<typename = void>
                    requires std::copyable<ValueOrReferenceType>
                inline constexpr void assign_reference(const usize length, ArgCLRefType<ReferenceType> ref)
                {
                    _container.assign(length, ValueOrReferenceType::ref(ref));
                }

                template<typename = void>
                    requires std::copyable<ValueOrReferenceType> && ReferenceFaster<ReferenceType> && std::movable<ReferenceType>
                inline constexpr void assign_reference(const usize length, ArgRRefType<ReferenceType> ref)
                {
                    _container.assign(length, ValueOrReferenceType::ref(move<ReferenceType>(ref)));
                }

                template<std::input_iterator It>
                    requires requires (const It it) { { *it } -> DecaySameAs<ValueType>; }
                inline constexpr void assign_reference(It&& first, It&& last)
                {
                    _container.assign
                    (
                        boost::make_transform_iterator(std::forward<It>(first), [](ArgCLRefType<ValueType> value) { return ValueOrReferenceType::ref(value); }),
                        boost::make_transform_iterator(std::forward<It>(last), [](ArgCLRefType<ValueType> value) { return ValueOrReferenceType::ref(value); })
                    );
                }

                template<std::input_iterator It>
                    requires std::copyable<ReferenceType> && requires (const It it) { { *it } -> DecaySameAs<ReferenceType>; }
                inline constexpr void assign_reference(It&& first, It&& last)
                {
                    _container.assign
                    (
                        boost::make_transform_iterator(std::forward<It>(first), [](ArgCLRefType<ReferenceType> ref) { return ValueOrReferenceType::ref(ref); }),
                        boost::make_transform_iterator(std::forward<It>(last), [](ArgCLRefType<ReferenceType> ref) { return ValueOrReferenceType::ref(ref); })
                    );
                }

                inline constexpr void assign_reference(std::initializer_list<ReferenceType> refs)
                {
                    _container.assign
                    (
                        boost::make_transform_iterator(refs.begin(), [](ArgLRefType<ReferenceType> ref) { return ValueOrReferenceType::ref(move<ReferenceType>(ref)); }),
                        boost::make_transform_iterator(refs.end(), [](ArgLRefType<ReferenceType> ref) { return ValueOrReferenceType::ref(move<ReferenceType>(ref)); })
                    );
                }

            public:
                template<typename = void>
                    requires requires (ContainerType& container) { { container.data() } -> DecaySameAs<PtrType<ValueOrReferenceType>>; }
                inline constexpr const PtrType<ValueOrReferenceType> data(void) noexcept
                {
                    return _container.data();
                }

                template<typename = void>
                    requires requires (ContainerType& container) { { container.data() } -> DecaySameAs<CPtrType<ValueOrReferenceType>>; }
                inline constexpr const CPtrType<ValueOrReferenceType> data(void) const noexcept
                {
                    return _container.data();
                }

            public:
                inline constexpr const usize max_size(void) const noexcept
                {
                    return _container.max_size();
                }

                template<typename = void>
                    requires requires (ContainerType& container) { container.reserve(std::declval<usize>()); }
                inline constexpr void reserve(const usize new_capacity)
                {
                    _container.reserve(new_capacity);
                }

                template<typename = void>
                    requires requires (const ContainerType& container) { { container.capacity() } -> DecaySameAs<usize>; }
                inline constexpr const usize capacity(void) const noexcept
                {
                    return _container.capacity();
                }

                template<typename = void>
                    requires requires (ContainerType& container) { container.shrink_to_fit(); }
                inline void shrink_to_fit(void)
                {
                    _container.shrink_to_fit();
                }

            public:
                inline constexpr void clear(void) noexcept
                {
                    _container.clear();
                }

                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, ArgCLRefType<ValueOrReferenceType> value)
                {
                    return IterType{ _container.insert(pos._iter, value) };
                }

                template<typename = void>
                    requires ReferenceFaster<ValueOrReferenceType> && std::movable<ValueOrReferenceType>
                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, ArgRRefType<ValueOrReferenceType> value)
                {
                    return IterType{ _container.insert(pos._iter, move<ValueOrReferenceType>(value)) };
                }

                template<std::input_iterator It>
                    requires requires (const It& it) { { *it } -> DecaySameAs<ValueOrReferenceType>; }
                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, It&& first, It&& last)
                {
                    return IterType{ _container.insert(pos._iter, std::forward<It>(first), std::forward<It>(last)) };
                }

                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, std::initializer_list<ValueType> values)
                {
                    return insert(pos,
                        boost::make_transform_iterator(values.begin(), [](ValueType& value) { return ValueOrReferenceType::value(move<ValueType>(value)); }),
                        boost::make_transform_iterator(values.end(), [](ValueType& value) { return ValueOrReferenceType::value(move<ValueType>(value)); })
                    );
                }

                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, std::initializer_list<ValueOrReferenceType> values)
                {
                    return IterType{ _container.insert(pos._iter, std::move(values)) };
                }

                template<
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::copyable<ValueOrReferenceType>
                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, const StaticValueOrReferenceArray<ValueType, len, cat, cow, C1>& values)
                {
                    return insert(pos, values._container.begin(), values._container.end());
                }

                template<
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::movable<ValueOrReferenceType>
                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, StaticValueOrReferenceArray<ValueType, len, cat, cow, C1>&& values)
                {
                    return IterType{ std::move(values._container.begin(), values._container.end(), std::inserter(_container, pos._iter)) };
                }

                template<
                    typename U,
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::convertible_to<U, ValueType> && std::convertible_to<PtrType<U>, PtrType<ValueType>>
                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, const StaticValueOrReferenceArray<U, len, cat, cow, C1>& values)
                {
                    return insert(pos,
                        boost::make_transform_iterator(values._container.begin(), [](const ValOrRef<U, cat>& value) { return ValueOrReferenceType{ value }; }),
                        boost::make_transform_iterator(values._container.end(), [](const ValOrRef<U, cat>& value) { return ValueOrReferenceType{ value }; })
                    );
                }

                template<
                    typename U,
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::convertible_to<U, ValueType> && std::convertible_to<PtrType<U>, PtrType<ValueType>>
                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, StaticValueOrReferenceArray<U, len, cat, cow, C1>&& values)
                {
                    return insert(pos,
                        boost::make_transform_iterator(values._container.begin(), [](ValOrRef<U, cat>& value) { return ValueOrReferenceType{ std::move(value) }; }),
                        boost::make_transform_iterator(values._container.end(), [](ValOrRef<U, cat>& value) { return ValueOrReferenceType{ std::move(value) }; })
                    );
                }

                template<template<typename> class C1>
                    requires std::copyable<ValueOrReferenceType>
                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, const DynamicValueOrReferenceArray<ValueType, cat, cow, C1>& values)
                {
                    return insert(pos, values._container.begin(), values._container.end());
                }

                template<template<typename> class C1>
                    requires std::movable<ValueOrReferenceType>
                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, DynamicValueOrReferenceArray<ValueType, cat, cow, C1>&& values)
                {
                    return IterType{ std::move(values._container.begin(), values._container.end(), std::inserter(_container, pos._iter)) };
                }

                template<
                    typename U,
                    template<typename> class C1
                >
                    requires std::convertible_to<U, ValueType> && std::convertible_to<PtrType<U>, PtrType<ValueType>>
                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, const DynamicValueOrReferenceArray<U, cat, cow, C1>& values)
                {
                    return insert(pos,
                        boost::make_transform_iterator(values._container.begin(), [](const ValOrRef<U, cat>& value) { return ValueOrReferenceType{ value }; }),
                        boost::make_transform_iterator(values._container.end(), [](const ValOrRef<U, cat>& value) { return ValueOrReferenceType{ value }; })
                    );
                }

                template<
                    typename U,
                    template<typename> class C1
                >
                    requires std::convertible_to<U, ValueType> && std::convertible_to<PtrType<U>, PtrType<ValueType>>
                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, DynamicValueOrReferenceArray<U, cat, cow, C1>&& values)
                {
                    return insert(pos,
                        boost::make_transform_iterator(values._container.begin(), [](ValOrRef<U, cat>& value) { return ValueOrReferenceType{ std::move(value) }; }),
                        boost::make_transform_iterator(values._container.end(), [](ValOrRef<U, cat>& value) { return ValueOrReferenceType{ std::move(value) }; })
                    );
                }

                inline constexpr RetType<IterType> insert_value(ArgCLRefType<ConstIterType> pos, ArgCLRefType<ValueType> value)
                {
                    return insert(pos, ValueOrReferenceType::value(value));
                }

                template<typename = void>
                    requires ReferenceFaster<ValueType> && std::movable<ValueType>
                inline constexpr RetType<IterType> insert_value(ArgCLRefType<ConstIterType> pos, ArgRRefType<ValueType> value)
                {
                    return insert(pos, ValueOrReferenceType::value(move<ValueType>(value)));
                }

                template<std::input_iterator It>
                    requires requires (const It& it) { { *it } -> DecaySameAs<ValueType>; }
                inline constexpr RetType<IterType> insert_value(ArgCLRefType<ConstIterType> pos, It&& first, It&& last)
                {
                    if constexpr (std::is_same_v<decltype(*first), CLRefType<ValueType>>)
                    {
                        return insert(pos._iter,
                            boost::make_transform_iterator(std::forward<It>(first), [](ArgCLRefType<ValueType> value) { return ValueOrReferenceType::value(value); }),
                            boost::make_transform_iterator(std::forward<It>(last), [](ArgCLRefType<ValueType> value) { return ValueOrReferenceType::value(value); })
                        );
                    }
                    else
                    {
                        return insert(pos._iter,
                            boost::make_transform_iterator(std::forward<It>(first), [](ArgLRefType<ValueType> value) { return ValueOrReferenceType::value(move<ValueType>(value)); }),
                            boost::make_transform_iterator(std::forward<It>(last), [](ArgLRefType<ValueType> value) { return ValueOrReferenceType::value(move<ValueType>(value)); })
                        );
                    }
                }

                inline constexpr RetType<IterType> insert_value(ArgCLRefType<ConstIterType> pos, std::initializer_list<ValueType> values)
                {
                    return insert(pos, std::move(values));
                }

                inline constexpr RetType<IterType> insert_reference(ArgCLRefType<ConstIterType> pos, CLRefType<ValueType> value)
                {
                    return insert(pos._iter, ValueOrReferenceType::ref(value));
                }

                inline constexpr RetType<IterType> insert_reference(ArgCLRefType<ConstIterType> pos, ArgRRefType<ReferenceType> ref)
                {
                    return insert(pos._iter, ValueOrReferenceType::ref(move<ReferenceType>(ref)));
                }

                template<std::input_iterator It>
                    requires requires (const It it) { { *it } -> DecaySameAs<ValueType>; }
                inline constexpr RetType<IterType> insert_reference(ArgCLRefType<ConstIterType> pos, It&& first, It&& last)
                {
                    return insert(pos._iter,
                        boost::make_transform_iterator(std::forward<It>(first), [](CLRefType<ValueType> value) { return ValueOrReferenceType::ref(value); }),
                        boost::make_transform_iterator(std::forward<It>(last), [](CLRefType<ValueType> value) { return ValueOrReferenceType::ref(value); })
                    );
                }

                template<std::input_iterator It>
                    requires std::copyable<ReferenceType> && requires (const It it) { { *it } -> DecaySameAs<ReferenceType>; }
                inline constexpr RetType<IterType> insert_reference(ArgCLRefType<ConstIterType> pos, It&& first, It&& last)
                {
                    return insert(pos._iter,
                        boost::make_transform_iterator(std::forward<It>(first), [](ArgCLRefType<ReferenceType> ref) { return ValueOrReferenceType::ref(ref); }),
                        boost::make_transform_iterator(std::forward<It>(last), [](ArgCLRefType<ReferenceType> ref) { return ValueOrReferenceType::ref(ref); })
                    );
                }

                inline constexpr RetType<IterType> insert_reference(ArgCLRefType<ConstIterType> pos, std::initializer_list<ReferenceType> refs)
                {
                    return insert(pos._iter,
                        boost::make_transform_iterator(refs.begin(), [](ArgLRefType<ReferenceType> ref) { return ValueOrReferenceType::ref(ref); }),
                        boost::make_transform_iterator(refs.end(), [](ArgLRefType<ReferenceType> ref) { return ValueOrReferenceType::ref(ref); })
                    );
                }

                inline constexpr RetType<IterType> erase(ArgCLRefType<ConstIterType> pos)
                {
                    return IterType{ _container.erase(pos._iter) };
                }

                inline constexpr RetType<IterType> erase(ArgCLRefType<ConstIterType> first, ArgCLRefType<ConstIterType> last)
                {
                    return IterType{ _container.erase(first._iter, last._iter) };
                }

            public:
                inline constexpr void push_back(ArgCLRefType<ValueOrReferenceType> value)
                {
                    _container.push_back(value);
                }

                template<typename = void>
                    requires ReferenceFaster<ValueOrReferenceType> && std::movable<ValueOrReferenceType>
                inline constexpr void push_back(ArgRRefType<ValueOrReferenceType> value)
                {
                    _container.push_back(move<ValueOrReferenceType>(value));
                }

                template<
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::copyable<ValueOrReferenceType>
                inline constexpr void push_back(const StaticValueOrReferenceArray<ValueType, len, cat, cow, C1>& values)
                {
                    insert(this->end(), values);
                }

                template<
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::movable<ValueOrReferenceType>
                inline constexpr void push_back(StaticValueOrReferenceArray<ValueType, len, cat, cow, C1>&& values)
                {
                    insert(this->end(), std::move(values));
                }

                template<
                    typename U,
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::convertible_to<U, ValueType> && std::convertible_to<PtrType<U>, PtrType<ValueType>>
                inline constexpr void push_back(const StaticValueOrReferenceArray<U, len, cat, cow, C1>& values)
                {
                    insert(this->end(), values);
                }

                template<
                    typename U,
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::convertible_to<U, ValueType> && std::convertible_to<PtrType<U>, PtrType<ValueType>>
                inline constexpr void push_back(StaticValueOrReferenceArray<U, len, cat, cow, C1>&& values)
                {
                    insert(this->end(), std::move(values));
                }

                template<template<typename> class C1>
                    requires std::copyable<ValueOrReferenceType>
                inline constexpr void push_back(const DynamicValueOrReferenceArray<ValueType, cat, cow, C1>& values)
                {
                    insert(this->end(), values);
                }

                template<template<typename> class C1>
                    requires std::movable<ValueOrReferenceType>
                inline constexpr void push_back(DynamicValueOrReferenceArray<ValueType, cat, cow, C1>&& values)
                {
                    insert(this->end(), std::move(values));
                }

                template<
                    typename U,
                    template<typename> class C1
                >
                    requires std::convertible_to<U, ValueType> && std::convertible_to<PtrType<U>, PtrType<ValueType>>
                inline constexpr void push_back(const DynamicValueOrReferenceArray<U, cat, cow, C1>& values)
                {
                    insert(this->end(), values);
                }

                template<
                    typename U,
                    template<typename> class C1
                >
                    requires std::convertible_to<U, ValueType> && std::convertible_to<PtrType<U>, PtrType<ValueType>>
                inline constexpr void push_back(DynamicValueOrReferenceArray<U, cat, cow, C1>&& values)
                {
                    insert(this->end(), std::move(values));
                }

                inline constexpr void push_back_value(ArgCLRefType<ValueType> value)
                {
                    _container.push_back(ValueOrReferenceType::value(value));
                }

                template<typename = void>
                    requires ReferenceFaster<ValueType> && std::movable<ValueType>
                inline constexpr void push_back_value(ArgRRefType<ValueType> value)
                {
                    _container.push_back(ValueOrReferenceType::value(move<ValueType>(value)));
                }

                inline constexpr void push_back_reference(CLRefType<ValueType> ref)
                {
                    _container.push_back(ValueOrReferenceType::ref(ref));
                }

                inline constexpr void push_back_reference(ArgCLRefType<ReferenceType> ref)
                {
                    _container.push_back(ValueOrReferenceType::ref(ref));
                }

                template<typename = void>
                    requires ReferenceFaster<ReferenceType> && std::movable<ReferenceType>
                inline constexpr void push_back_reference(ArgRRefType<ReferenceType> ref)
                {
                    _container.push_back(ValueOrReferenceType::ref(move<ReferenceType>(ref)));
                }

                template<typename... Args>
                    requires std::constructible_from<ValueType, Args...>
                inline constexpr void emplace_back(Args&&... args)
                {
                    _container.push_back(ValueOrReferenceType::value(std::forward<Args>(args)...));
                }

                template<typename... Args>
                    requires std::constructible_from<ReferenceType, Args...>
                inline constexpr void emplace_back(Args&&... args)
                {
                    _container.push_back(ValueOrReferenceType::ref(ReferenceType{ std::forward<Args>(args)... }));
                }

                inline constexpr RetType<ValueOrReferenceType> pop_back(void)
                {
                    auto back = move<ValueOrReferenceType>(this->back());
                    _container.pop_back();
                    return back;
                }

            public:
                inline constexpr void push_front(ArgCLRefType<ValueOrReferenceType> value)
                {
                    insert(this->begin(), value);
                }

                template<typename = void>
                    requires requires (ContainerType& container) { container.push_front(std::declval<ValueOrReferenceType>()); }
                inline constexpr void push_front(ArgCLRefType<ValueOrReferenceType> value)
                {
                    _container.push_front(value);
                }

                template<typename = void>
                    requires ReferenceFaster<ValueOrReferenceType> && std::movable<ValueOrReferenceType>
                inline constexpr void push_front(ArgRRefType<ValueOrReferenceType> value)
                {
                    insert(this->begin(), move<ValueOrReferenceType>(value));
                }

                template<typename = void>
                    requires ReferenceFaster<ValueOrReferenceType> && std::movable<ValueOrReferenceType> && 
                        requires (ContainerType& container) { container.push_front(std::declval<ValueOrReferenceType>()); }
                inline constexpr void push_front(ArgRRefType<ValueOrReferenceType> value)
                {
                    _container.push_front(move<ValueOrReferenceType>(value));
                }

                template<
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::copyable<ValueOrReferenceType>
                inline constexpr void push_front(const StaticValueOrReferenceArray<ValueType, len, cat, cow, C1>& values)
                {
                    insert(this->begin(), values);
                }

                template<
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::movable<ValueOrReferenceType>
                inline constexpr void push_front(StaticValueOrReferenceArray<ValueType, len, cat, cow, C1>&& values)
                {
                    insert(this->begin(), std::move(values));
                }

                template<
                    typename U,
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::convertible_to<U, ValueType> && std::convertible_to<PtrType<U>, PtrType<ValueType>>
                inline constexpr void push_front(const StaticValueOrReferenceArray<U, len, cat, cow, C1>& values)
                {
                    insert(this->begin(), values);
                }

                template<
                    typename U,
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::convertible_to<U, ValueType> && std::convertible_to<PtrType<U>, PtrType<ValueType>>
                inline constexpr void push_front(StaticValueOrReferenceArray<U, len, cat, cow, C1>&& values)
                {
                    insert(this->begin(), std::move(values));
                }

                template<template<typename> class C1>
                    requires std::copyable<ValueOrReferenceType>
                inline constexpr void push_front(const DynamicValueOrReferenceArray<ValueType, cat, cow, C1>& values)
                {
                    insert(this->begin(), values);
                }

                template<template<typename> class C1>
                    requires std::movable<ValueOrReferenceType>
                inline constexpr void push_front(DynamicValueOrReferenceArray<ValueType, cat, cow, C1>&& values)
                {
                    insert(this->begin(), std::move(values));
                }

                template<
                    typename U,
                    template<typename> class C1
                >
                    requires std::convertible_to<U, ValueType> && std::convertible_to<PtrType<U>, PtrType<ValueType>>
                inline constexpr void push_front(const DynamicValueOrReferenceArray<U, cat, cow, C1>& values)
                {
                    insert(this->begin(), values);
                }

                template<
                    typename U,
                    template<typename> class C1
                >
                    requires std::convertible_to<U, ValueType> && std::convertible_to<PtrType<U>, PtrType<ValueType>>
                inline constexpr void push_front(DynamicValueOrReferenceArray<U, cat, cow, C1>&& values)
                {
                    insert(this->begin(), std::move(values));
                }

                inline constexpr void push_front_value(ArgCLRefType<ValueType> value)
                {
                    insert(this->begin(), ValueOrReferenceType::value(value));
                }

                template<typename = void>
                    requires requires (ContainerType& container) { container.push_front(std::declval<ValueOrReferenceType>()); }
                inline constexpr void push_front_value(ArgCLRefType<ValueType> value)
                {
                    _container.push_front(ValueOrReferenceType::value(value));
                }

                template<typename = void>
                    requires ReferenceFaster<ValueType> && std::movable<ValueType>
                inline constexpr void push_front_value(ArgRRefType<ValueType> value)
                {
                    insert(this->begin(), ValueOrReferenceType::value(move<ValueType>(value)));
                }

                template<typename = void>
                    requires ReferenceFaster<ValueType> && std::movable<ValueType> && 
                        requires (ContainerType& container) { container.push_front(std::declval<ValueOrReferenceType>()); }
                inline constexpr void push_front_value(ArgRRefType<ValueType> value)
                {
                    _container.push_front(ValueOrReferenceType::value(move<ValueType>(value)));
                }
                
                inline constexpr void push_front_reference(CLRefType<ValueType> ref)
                {
                    insert(this->begin(), ValueOrReferenceType::ref(ref));
                }

                template<typename = void>
                    requires requires (ContainerType& container) { container.push_front(std::declval<ValueOrReferenceType>()); }
                inline constexpr void push_front_reference(CLRefType<ValueType> ref)
                {
                    _container.push_front(ValueOrReferenceType::ref(ref));
                }

                inline constexpr void push_front_reference(ArgCLRefType<ReferenceType> ref)
                {
                    insert(this->begin(), ValueOrReferenceType::ref(ref));
                }

                template<typename = void>
                    requires requires (ContainerType& container) { container.push_front(std::declval<ValueOrReferenceType>()); }
                inline constexpr void push_front_reference(ArgCLRefType<ReferenceType> ref)
                {
                    _container.push_front(ValueOrReferenceType::ref(ref));
                }

                template<typename = void>
                    requires ReferenceFaster<ReferenceType> && std::movable<ReferenceType>
                inline constexpr void push_front_reference(ArgRRefType<ReferenceType> ref)
                {
                    insert(this->begin(), ValueOrReferenceType::ref(move<ReferenceType>(ref)));
                }

                template<typename = void>
                    requires ReferenceFaster<ReferenceType> && std::movable<ReferenceType> &&
                        requires (ContainerType& container) { container.push_front(std::declval<ValueOrReferenceType>()); }
                inline constexpr void push_front_reference(ArgRRefType<ReferenceType> ref)
                {
                    _container.push_front(ValueOrReferenceType::ref(move<ReferenceType>(ref)));
                }

                template<typename... Args>
                    requires std::constructible_from<ValueType, Args...>
                inline constexpr void emplace_front(Args&&... args)
                {
                    insert(this->begin(), ValueOrReferenceType::value(std::forward<Args>(args)...));
                }

                template<typename... Args>
                    requires std::constructible_from<ValueType, Args...>
                        && requires (ContainerType& container) { container.push_front(std::declval<ValueOrReferenceType>()); }
                inline constexpr void emplace_front(Args&&... args)
                {
                    _container.push_front(ValueOrReferenceType::value(std::forward<Args>(args)...));
                }

                template<typename... Args>
                    requires std::constructible_from<ReferenceType, Args...>
                inline constexpr void emplace_front(Args&&... args)
                {
                    insert(this->begin(), ValueOrReferenceType::ref(ReferenceType{ std::forward<Args>(args)... }));
                }

                template<typename... Args>
                    requires std::constructible_from<ReferenceType, Args...>
                        && requires (ContainerType& container) { container.push_front(std::declval<ValueOrReferenceType>()); }
                inline constexpr void emplace_front(Args&&... args)
                {
                    _container.push_front(ValueOrReferenceType::ref(ReferenceType{ std::forward<Args>(args)... }));
                }

                inline constexpr RetType<ValueOrReferenceType> pop_front(void)
                {
                    auto front = move<ValueOrReferenceType>(this->front());
                    _container.erase(_container.begin());
                    return front;
                }

                template<typename = void>
                    requires requires (ContainerType& container) { container.pop_front(); }
                inline constexpr RetType<ValueOrReferenceType> pop_front(void)
                {
                    auto front = move<ValueOrReferenceType>(this->front());
                    _container.pop_front();
                    return front;
                }

             public:
                template<typename = void>
                    requires std::default_initializable<ValueOrReferenceType>
                inline constexpr void resize(const usize length)
                {
                    _container.resize(length);
                }

                template<typename = void>
                    requires (!std::default_initializable<ValueOrReferenceType> && WithDefault<ValueOrReferenceType> && std::copyable<ValueOrReferenceType>)
                inline constexpr void resize(const usize length)
                {
                    _container.resize(length, DefaultValue<ValueOrReferenceType>::value());
                }

                template<typename = void>
                    requires std::copyable<ValueOrReferenceType>
                inline constexpr void resize(const usize length, ArgCLRefType<ValueOrReferenceType> value)
                {
                    _container.resize(length, value);
                }

            public:
                inline constexpr const bool operator==(const DynamicValueOrReferenceArray& rhs) const noexcept
                {
                    return _container == rhs._container;
                }

                inline constexpr const bool operator!=(const DynamicValueOrReferenceArray& rhs) const noexcept
                {
                    return _container != rhs._container;
                }

            public:
                inline constexpr const bool operator<(const DynamicValueOrReferenceArray& rhs) const noexcept
                {
                    return _container < rhs._container;
                }

                inline constexpr const bool operator<=(const DynamicValueOrReferenceArray& rhs) const noexcept
                {
                    return _container <= rhs._container;
                }

                inline constexpr const bool operator>(const DynamicValueOrReferenceArray& rhs) const noexcept
                {
                    return _container > rhs._container;
                }

                inline constexpr const bool operator>=(const DynamicValueOrReferenceArray& rhs) const noexcept
                {
                    return _container >= rhs._container;
                }

            public:
                inline constexpr decltype(auto) operator<=>(const DynamicValueOrReferenceArray& rhs) const noexcept
                {
                    return _container <=> rhs._container;
                }

            OSPF_CRTP_PERMISSION:
                inline constexpr LRefType<ContainerType> OSPF_CRTP_FUNCTION(get_container)(void) noexcept
                {
                    return _container;
                }

                inline constexpr CLRefType<ContainerType> OSPF_CRTP_FUNCTION(get_const_container)(void) const noexcept
                {
                    return _container;
                }

            private:
                ContainerType _container;
            };
        };

        template<
            typename T,
            usize len,
            ReferenceCategory cat = ReferenceCategory::Reference,
            CopyOnWrite cow = off,
            template<typename, usize> class C = std::array
        >
        using ValOrRefArray = value_or_reference_array::StaticValueOrReferenceArray<T, len, cat, cow, C>;

        template<
            typename T,
            ReferenceCategory cat = ReferenceCategory::Reference,
            CopyOnWrite cow = off,
            template<typename> class C = std::vector
        >
        using DynValOrRefArray = value_or_reference_array::DynamicValueOrReferenceArray<T, cat, cow, C>;
    };
};

namespace std
{
    template<typename T, typename C, typename Self>
    inline void swap(ospf::value_or_reference_array::ValueOrReferenceArrayImpl<T, C, Self>& lhs, ospf::value_or_reference_array::ValueOrReferenceArrayImpl<T, C, Self>& rhs) noexcept
    {
        lhs.swap(rhs);
    }
};

#pragma once

#include <ospf/basic_definition.hpp>
#include <ospf/concepts/with_default.hpp>
#include <ospf/functional/pointer_or_reference.hpp>
#include <ospf/literal_constant.hpp>
#include <ospf/memory/reference.hpp>
#include <ospf/functional/iterator.hpp>
#include <boost/iterator/transform_iterator.hpp>

namespace ospf
{
    inline namespace data_structure
    {
        namespace pointer_or_reference_array
        {
            template<
                typename T,
                usize len,
                PointerCategory pcat,
                ReferenceCategory rcat,
                template<typename, usize> class C
            >
                requires NotSameAs<T, void>
            class StaticPointerOrReferenceArray;

            template<
                typename T,
                PointerCategory pcat,
                ReferenceCategory rcat,
                template<typename> class C
            >
                requires NotSameAs<T, void>
            class DynamicPointerOrReferenceArray;

            template<typename T, typename C>
            class PointerOrReferenceArrayConstIterator
                : public RandomIteratorImpl<ConstType<T>, typename OriginType<C>::const_iterator, PointerOrReferenceArrayConstIterator<T, C>>
            {
                template<
                    typename T,
                    usize len,
                    PointerCategory pcat,
                    ReferenceCategory rcat,
                    template<typename, usize> class C
                >
                    requires NotSameAs<T, void>
                class StaticPointerOrReferenceArray;

                template<
                    typename T,
                    PointerCategory pcat,
                    ReferenceCategory rcat,
                    template<typename> class C
                >
                    requires NotSameAs<T, void>
                class DynamicPointerOrReferenceArray;

            public:
                using ValueType = OriginType<T>;
                using ContainerType = OriginType<C>;
                using IterType = typename ContainerType::const_iterator;

            private:
                using Impl = RandomIteratorImpl<ConstType<ValueType>, IterType, PointerOrReferenceArrayConstIterator<T, C>>;

            public:
                constexpr PointerOrReferenceArrayConstIterator(ArgCLRefType<IterType> iter)
                    : Impl(iter) {}
                constexpr PointerOrReferenceArrayConstIterator(const PointerOrReferenceArrayConstIterator& ano) = default;
                constexpr PointerOrReferenceArrayConstIterator(PointerOrReferenceArrayConstIterator&& ano) noexcept = default;
                constexpr PointerOrReferenceArrayConstIterator& operator=(const PointerOrReferenceArrayConstIterator& rhs) = default;
                constexpr PointerOrReferenceArrayConstIterator& operator=(PointerOrReferenceArrayConstIterator&& rhs) noexcept = default;
                constexpr ~PointerOrReferenceArrayConstIterator(void) noexcept = default;

            OSPF_CRTP_PERMISSION:
                inline static constexpr CLRefType<ValueType> OSPF_CRTP_FUNCTION(get)(ArgCLRefType<IterType> iter) noexcept
                {
                    return **iter;
                }

                inline static constexpr RetType<PointerOrReferenceArrayConstIterator> OSPF_CRTP_FUNCTION(construct)(ArgCLRefType<IterType> iter) noexcept
                {
                    return PointerOrReferenceArrayConstIterator{ iter };
                }
            };

            template<typename T, typename C>
            class PointerOrReferenceArrayIterator
                : public RandomIteratorImpl<OriginType<T>, typename OriginType<C>::iterator, PointerOrReferenceArrayIterator<T, C>>
            {
                template<
                    typename T,
                    usize len,
                    PointerCategory pcat,
                    ReferenceCategory rcat,
                    template<typename, usize> class C
                >
                    requires NotSameAs<T, void>
                class StaticPointerOrReferenceArray;

                template<
                    typename T,
                    PointerCategory pcat,
                    ReferenceCategory rcat,
                    template<typename> class C
                >
                    requires NotSameAs<T, void>
                class DynamicPointerOrReferenceArray;

            public:
                using ValueType = OriginType<T>;
                using ContainerType = OriginType<C>;
                using IterType = typename ContainerType::iterator;

            private:
                using Impl = RandomIteratorImpl<ConstType<ValueType>, IterType, PointerOrReferenceArrayIterator<T, C>>;

            public:
                constexpr PointerOrReferenceArrayIterator(ArgCLRefType<IterType> iter)
                    : Impl(iter) {}
                constexpr PointerOrReferenceArrayIterator(const PointerOrReferenceArrayIterator& ano) = default;
                constexpr PointerOrReferenceArrayIterator(PointerOrReferenceArrayIterator&& ano) noexcept = default;
                constexpr PointerOrReferenceArrayIterator& operator=(const PointerOrReferenceArrayIterator& rhs) = default;
                constexpr PointerOrReferenceArrayIterator& operator=(PointerOrReferenceArrayIterator&& rhs) noexcept = default;
                constexpr ~PointerOrReferenceArrayIterator(void) noexcept = default;

            OSPF_CRTP_PERMISSION:
                inline static constexpr CLRefType<ValueType> OSPF_CRTP_FUNCTION(get)(ArgCLRefType<IterType> iter) noexcept
                {
                    return **iter;
                }

                inline static constexpr RetType<PointerOrReferenceArrayIterator> OSPF_CRTP_FUNCTION(construct)(ArgCLRefType<IterType> iter) noexcept
                {
                    return PointerOrReferenceArrayIterator{ iter };
                }
            };

            template<typename T, typename C>
            class PointerOrReferenceArrayConstReverseIterator
                : public RandomIteratorImpl<ConstType<T>, typename OriginType<C>::const_reverse_iterator, PointerOrReferenceArrayConstReverseIterator<T, C>>
            {
                template<
                    typename T,
                    usize len,
                    PointerCategory pcat,
                    ReferenceCategory rcat,
                    template<typename, usize> class C
                >
                    requires NotSameAs<T, void>
                class StaticPointerOrReferenceArray;

                template<
                    typename T,
                    PointerCategory pcat,
                    ReferenceCategory rcat,
                    template<typename> class C
                >
                    requires NotSameAs<T, void>
                class DynamicPointerOrReferenceArray;

            public:
                using ValueType = OriginType<T>;
                using ContainerType = OriginType<C>;
                using IterType = typename ContainerType::const_reverse_iterator;

            private:
                using Impl = RandomIteratorImpl<ConstType<ValueType>, IterType, PointerOrReferenceArrayConstReverseIterator<T, C>>;

            public:
                constexpr PointerOrReferenceArrayConstReverseIterator(ArgCLRefType<IterType> iter)
                    : Impl(iter) {}
                constexpr PointerOrReferenceArrayConstReverseIterator(const PointerOrReferenceArrayConstReverseIterator& ano) = default;
                constexpr PointerOrReferenceArrayConstReverseIterator(PointerOrReferenceArrayConstReverseIterator&& ano) noexcept = default;
                constexpr PointerOrReferenceArrayConstReverseIterator& operator=(const PointerOrReferenceArrayConstReverseIterator& rhs) = default;
                constexpr PointerOrReferenceArrayConstReverseIterator& operator=(PointerOrReferenceArrayConstReverseIterator&& rhs) noexcept = default;
                constexpr ~PointerOrReferenceArrayConstReverseIterator(void) noexcept = default;

            OSPF_CRTP_PERMISSION:
                inline static constexpr CLRefType<ValueType> OSPF_CRTP_FUNCTION(get)(ArgCLRefType<IterType> iter) noexcept
                {
                    return **iter;
                }

                inline static constexpr RetType<PointerOrReferenceArrayConstReverseIterator> OSPF_CRTP_FUNCTION(construct)(ArgCLRefType<IterType> iter) noexcept
                {
                    return PointerOrReferenceArrayConstReverseIterator{ iter };
                }
            };

            template<typename T, typename C>
            class PointerOrReferenceArrayReverseIterator
                : public RandomIteratorImpl<OriginType<T>, typename OriginType<C>::reverse_iterator, PointerOrReferenceArrayReverseIterator<T, C>>
            {
                template<
                    typename T,
                    usize len,
                    PointerCategory pcat,
                    ReferenceCategory rcat,
                    template<typename, usize> class C
                >
                    requires NotSameAs<T, void>
                class StaticPointerOrReferenceArray;

                template<
                    typename T,
                    PointerCategory pcat,
                    ReferenceCategory rcat,
                    template<typename> class C
                >
                    requires NotSameAs<T, void>
                class DynamicPointerOrReferenceArray;

            public:
                using ValueType = OriginType<T>;
                using ContainerType = OriginType<C>;
                using IterType = typename ContainerType::reverse_iterator;

            private:
                using Impl = RandomIteratorImpl<ConstType<ValueType>, IterType, PointerOrReferenceArrayReverseIterator<T, C>>;

            public:
                constexpr PointerOrReferenceArrayReverseIterator(ArgCLRefType<IterType> iter)
                    : Impl(iter) {}
                constexpr PointerOrReferenceArrayReverseIterator(const PointerOrReferenceArrayReverseIterator& ano) = default;
                constexpr PointerOrReferenceArrayReverseIterator(PointerOrReferenceArrayReverseIterator&& ano) noexcept = default;
                constexpr PointerOrReferenceArrayReverseIterator& operator=(const PointerOrReferenceArrayReverseIterator& rhs) = default;
                constexpr PointerOrReferenceArrayReverseIterator& operator=(PointerOrReferenceArrayReverseIterator&& rhs) noexcept = default;
                constexpr ~PointerOrReferenceArrayReverseIterator(void) noexcept = default;

            OSPF_CRTP_PERMISSION:
                inline static constexpr CLRefType<ValueType> OSPF_CRTP_FUNCTION(get)(ArgCLRefType<IterType> iter) noexcept
                {
                    return **iter;
                }

                inline static constexpr RetType<PointerOrReferenceArrayReverseIterator> OSPF_CRTP_FUNCTION(construct)(ArgCLRefType<IterType> iter) noexcept
                {
                    return PointerOrReferenceArrayReverseIterator{ iter };
                }
            };

            template<typename T, typename C>
            struct PointerOrReferenceArrayAccessPolicy;

            template<
                typename T,
                usize len,
                PointerCategory pcat,
                ReferenceCategory rcat,
                template<typename, usize> class C
            >
            struct PointerOrReferenceArrayAccessPolicy<T, C<PtrOrRef<OriginType<T>, pcat, rcat>, len>>
            {
            public:
                using ValueType = OriginType<T>;
                using PointerOrReferenceType = PtrOrRef<ValueType, pcat, rcat>;
                using PointerType = typename PointerOrReferenceType::PointerType;
                using ReferenceType = typename PointerOrReferenceType::ReferenceType;
                using ContainerType = C<PointerOrReferenceType, len>;
                using IterType = PointerOrReferenceArrayIterator<ValueType, ContainerType>;
                using ConstIterType = PointerOrReferenceArrayConstIterator<ValueType, ContainerType>;
                using ReverseIterType = PointerOrReferenceArrayReverseIterator<ValueType, ContainerType>;
                using ConstReverseIterType = PointerOrReferenceArrayConstReverseIterator<ValueType, ContainerType>;

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
                PointerCategory pcat,
                ReferenceCategory rcat,
                typename Alloc,
                template<typename, typename> class C
            >
            struct PointerOrReferenceArrayAccessPolicy<T, C<PtrOrRef<OriginType<T>, pcat, rcat>, Alloc>>
            {
            public:
                using ValueType = OriginType<T>;
                using PointerOrReferenceType = PtrOrRef<ValueType, pcat, rcat>;
                using PointerType = typename PointerOrReferenceType::PointerType;
                using ReferenceType = typename PointerOrReferenceType::ReferenceType;
                using ContainerType = C<PointerOrReferenceType, Alloc>;
                using IterType = PointerOrReferenceArrayIterator<ValueType, ContainerType>;
                using ConstIterType = PointerOrReferenceArrayConstIterator<ValueType, ContainerType>;
                using ReverseIterType = PointerOrReferenceArrayReverseIterator<ValueType, ContainerType>;
                using ConstReverseIterType = PointerOrReferenceArrayConstReverseIterator<ValueType, ContainerType>;

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
                    return ConstIterType{ array.cebgin() };
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
            class PointerOrReferenceArrayImpl
            {
                OSPF_CRTP_IMPL;
            
            public:
                using ValueType = OriginType<T>;
                using ContainerType = OriginType<C>;
                using AccessPolicyType = PointerOrReferenceArrayAccessPolicy<ValueType, ContainerType>;
                using PointerOrReferenceType = typename AccessPolicyType::PointerOrReferenceType;
                using PointerType = typename AccessPolicyType::PointerType;
                using ReferenceType = typename AccessPolicyType::ReferenceType;
                using IterType = typename AccessPolicyType::IterType;
                using ConstIterType = typename AccessPolicyType::ConstIterType;
                using ReverseIterType = typename AccessPolicyType::ReverseIterType;
                using ConstReverseIterType = typename AccessPolicyType::ConstReverseIterType;

            protected:
                constexpr PointerOrReferenceArrayImpl(void) = default;
            public:
                constexpr PointerOrReferenceArrayImpl(const PointerOrReferenceArrayImpl& ano) = default;
                constexpr PointerOrReferenceArrayImpl(PointerOrReferenceArrayImpl&& ano) noexcept = default;
                constexpr PointerOrReferenceArrayImpl& operator=(const PointerOrReferenceArrayImpl& rhs) = default;
                constexpr PointerOrReferenceArrayImpl& operator=(PointerOrReferenceArrayImpl&& rhs) noexcept = default;
                constexpr ~PointerOrReferenceArrayImpl(void) noexcept = default;

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
                inline void swap(PointerOrReferenceArrayImpl& ano) noexcept
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
                PointerCategory pcat,
                ReferenceCategory rcat,
                template<typename, usize> class C
            >
                requires NotSameAs<T, void>
            class StaticPointerOrReferenceArray
                : public PointerOrReferenceArrayImpl<T, C<PtrOrRef<OriginType<T>, pcat, rcat>, len>, StaticPointerOrReferenceArray<T, len, pcat, rcat, C>>
            {
                using Impl = PointerOrReferenceArrayImpl<T, C<PtrOrRef<OriginType<T>, pcat, rcat>, len>, StaticPointerOrReferenceArray<T, len, pcat, rcat, C>>;

                template<
                    typename T,
                    PointerCategory pcat,
                    ReferenceCategory rcat,
                    template<typename> class C
                >
                    requires NotSameAs<T, void>
                class DynamicPointerOrReferenceArray;

            public:
                using typename Impl::ValueType;
                using typename Impl::PointerType;
                using typename Impl::ReferenceType;
                using typename Impl::PointerOrReferenceType;
                using typename Impl::ContainerType;
                using typename Impl::IterType;
                using typename Impl::ConstIterType;
                using typename Impl::ReverseIterType;
                using typename Impl::ConstReverseIterType;

            public:
                constexpr StaticPointerOrReferenceArray(ArgRRefType<ContainerType> container)
                    : _container(move<ContainerType>(container)) {}

            public:
                constexpr StaticPointerOrReferenceArray(const StaticPointerOrReferenceArray& ano) = default;
                constexpr StaticPointerOrReferenceArray(StaticPointerOrReferenceArray&& ano) noexcept = default;
                constexpr StaticPointerOrReferenceArray& operator=(const StaticPointerOrReferenceArray& rhs) = default;
                constexpr StaticPointerOrReferenceArray& operator=(StaticPointerOrReferenceArray&& rhs) noexcept = default;
                constexpr ~StaticPointerOrReferenceArray(void) = default;

            public:
                template<typename = void>
                    requires requires (ContainerType& container) { { container.data() } -> DecaySameAs<PtrType<PointerOrReferenceType>>; }
                inline constexpr const PtrType<PointerOrReferenceType> data(void) noexcept
                {
                    return _container.data();
                }

                template<typename = void>
                    requires requires (const ContainerType& container) { { container.data() } -> DecaySameAs<CPtrType<PointerOrReferenceType>>; }
                inline constexpr const CPtrType<PointerOrReferenceType> data(void) const noexcept
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
                    requires std::copyable<PointerOrReferenceType>
                inline constexpr void fill(ArgCLRefType<PointerOrReferenceType> value) noexcept
                {
                    _container.fill(value);
                }

                template<typename = void>
                    requires std::copyable<PointerOrReferenceType>
                inline constexpr void fill_pointer(ArgCLRefType<PointerType> ptr) noexcept
                {
                    _container.fill(PointerOrReferenceType::ptr(ptr));
                }

                template<typename = void>
                    requires std::copyable<PointerOrReferenceType>
                inline constexpr void fill_reference(CLRefType<ValueType> ref) noexcept
                {
                    _container.fill(PointerOrReferenceType::ref(ref));
                }

                template<typename = void>
                    requires std::copyable<ValueOrReferenceType>
                inline constexpr void fill_reference(ArgCLRefType<ReferenceType> ref) noexcept
                {
                    _container.fill(PointerOrReferenceType::ref(ref));
                }

                template<typename = void>
                    requires std::copyable<ValueOrReferenceType> && ReferenceFaster<ReferenceType> && std::movable<ReferenceType>
                inline constexpr void fill_reference(ArgRRefType<ReferenceType> ref) noexcept
                {
                    _container.fill(PointerOrReferenceType::ref(move<ReferenceType>(ref)));
                }

            public:
                inline constexpr const bool operator==(const StaticPointerOrReferenceArray& rhs) const noexcept
                {
                    return _container == rhs._container;
                }

                inline constexpr const bool operator!=(const StaticPointerOrReferenceArray& rhs) const noexcept
                {
                    return _container != rhs._container;
                }

            public:
                inline constexpr const bool operator<(const StaticPointerOrReferenceArray& rhs) const noexcept
                {
                    return _container < rhs._container;
                }

                inline constexpr const bool operator<=(const StaticPointerOrReferenceArray& rhs) const noexcept
                {
                    return _container <= rhs._container;
                }

                inline constexpr const bool operator>(const StaticPointerOrReferenceArray& rhs) const noexcept
                {
                    return _container > rhs._container;
                }

                inline constexpr const bool operator>=(const StaticPointerOrReferenceArray& rhs) const noexcept
                {
                    return _container >= rhs._container;
                }

            public:
                inline constexpr decltype(auto) operator<=>(const StaticPointerOrReferenceArray& rhs) const noexcept
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
                PointerCategory pcat,
                ReferenceCategory rcat,
                template<typename> class C
            >
                requires NotSameAs<T, void>
            class DynamicPointerOrReferenceArray
                : public PointerOrReferenceArrayImpl<T, C<PtrOrRef<OriginType<T>, pcat, rcat>>, DynamicPointerOrReferenceArray<T, pcat, rcat, C>>
            {
                using Impl = PointerOrReferenceArrayImpl<T, C<ValOrRef<OriginType<T>, pcat, rcat>>, DynamicPointerOrReferenceArray<T, pcat, rcat, C>>;

            public:
                using typename Impl::ValueType;
                using typename Impl::PointerType;
                using typename Impl::ReferenceType;
                using typename Impl::PointerOrReferenceType;
                using typename Impl::ContainerType;
                using typename Impl::IterType;
                using typename Impl::ConstIterType;
                using typename Impl::ReverseIterType;
                using typename Impl::ConstReverseIterType;

            public:
                constexpr DynamicPointerOrReferenceArray(void) = default;

                template<typename = void>
                    requires std::copyable<PointerOrReferenceType>
                constexpr DynamicPointerOrReferenceArray(const usize length, ArgCLRefType<PointerOrReferenceType> value)
                    : _container(length, value) {}

                template<std::input_iterator It>
                    requires requires (const It it) { { *it } -> DecaySameAs<PointerOrReferenceType>; }
                constexpr DynamicPointerOrReferenceArray(It&& first, It&& last)
                    : _container(first, last) {}

                constexpr DynamicPointerOrReferenceArray(std::initializer_list<PointerType> ptrs)
                    : _container
                    (
                        boost::make_transform_iterator(ptrs.begin(), [](PointerType& ptr) { return PointerOrReferenceType::ptr(move<PointerType>(ptr)); }),
                        boost::make_transform_iterator(ptrs.end(), [](PointerType& ptr) { return PointerOrReferenceType::ptr(move<PointerType>(ptr)); })
                    ) {}

                constexpr DynamicPointerOrReferenceArray(std::initializer_list<PointerOrReferenceType> values)
                    : _container(std::move(values)) {}

                constexpr DynamicPointerOrReferenceArray(ArgRRefType<ContainerType> container)
                    : _container(move<ContainerType>(container)) {}

            public:
                constexpr DynamicPointerOrReferenceArray(const DynamicPointerOrReferenceArray& ano) = default;
                constexpr DynamicPointerOrReferenceArray(DynamicPointerOrReferenceArray&& ano) noexcept = default;
                constexpr DynamicPointerOrReferenceArray& operator=(const DynamicPointerOrReferenceArray& rhs) = default;
                constexpr DynamicPointerOrReferenceArray& operator=(DynamicPointerOrReferenceArray&& rhs) noexcept = default;
                constexpr ~DynamicPointerOrReferenceArray(void) = default;

            public:
                template<typename = void>
                    requires std::copyable<PointerOrReferenceType>
                inline constexpr void assign(const usize length, ArgCLRefType<PointerOrReferenceType> value)
                {
                    _container.assign(length, value);
                }

                template<std::input_iterator It>
                    requires requires (const It& it) { { *it } -> DecaySameAs<PointerOrReferenceType>; }
                inline constexpr void assign(It&& first, It&& last)
                {
                    _container.assign(std::forward<It>(first), std::forward<It>(last));
                }

                inline constexpr void assign(std::initializer_list<PointerType> ptrs)
                {
                    _container.assign
                    (
                        boost::make_transform_iterator(ptrs.begin(), [](PointerType& ptr) { return PointerOrReferenceType::ptr(move<PointerType>(ptr)); }),
                        boost::make_transform_iterator(ptrs.end(), [](PointerType& ptr) { return PointerOrReferenceType::ptr(move<PointerType>(ptr)); })
                    );
                }

                inline constexpr void assign(std::initializer_list<PointerOrReferenceType> values)
                {
                    _container.assign(std::move(values));
                }

                template<
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::copyable<PointerOrReferenceType>
                inline constexpr void assign(const StaticPointerOrReferenceArray<ValueType, len, pcat, rcat, C1>& values)
                {
                    assign(values._container.begin(), values._container.end());
                }

                template<
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::movable<PointerOrReferenceType>
                inline constexpr void assign(StaticPointerOrReferenceArray<ValueType, len, pcat, rcat, C1>&& values)
                {
                    assign
                    (
                        boost::make_transform_iterator(values._container.begin(), [](PointerOrReferenceType& value) { return std::move(value); }),
                        boost::make_transform_iterator(values._container.end(), [](PointerOrReferenceType& value) { return std::move(value); })
                    );
                }

                template<
                    typename U,
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::convertible_to<PtrType<U>, PtrType<ValueType>>
                inline constexpr void assign(const StaticPointerOrReferenceArray<U, len, pcat, rcat, C1>& values)
                {
                    assign
                    (
                        boost::make_transform_iterator(values._container.begin(), [](const PtrOrRef<U, pcat, rcat>& value) { return PointerOrReferenceType{ value }; }),
                        boost::make_transform_iterator(values._container.end(), [](const PtrOrRef<U, pcat, rcat>& value) { return PointerOrReferenceType{ value }; })
                    );
                }

                template<
                    typename U,
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::convertible_to<PtrType<U>, PtrType<ValueType>>
                inline constexpr void assign(StaticPointerOrReferenceArray<U, len, pcat, rcat, C1>&& values)
                {
                    assign
                    (
                        boost::make_transform_iterator(values._container.begin(), [](PtrOrRef<U, pcat, rcat>& value) { return PointerOrReferenceType{ std::move(value) }; }),
                        boost::make_transform_iterator(values._container.end(), [](PtrOrRef<U, pcat, rcat>& value) { return PointerOrReferenceType{ std::move(value) }; })
                    );
                }

                template<template<typename> class C1>
                    requires std::copyable<PointerOrReferenceType>
                inline constexpr void assign(const DynamicPointerOrReferenceArray<ValueType, pcat, rcat, C1>& values)
                {
                    assign(values._container.begin(), values._container.end());
                }

                template<template<typename> class C1>
                    requires std::movable<PointerOrReferenceType>
                inline constexpr void assign(DynamicPointerOrReferenceArray<ValueType, pcat, rcat, C1>&& values)
                {
                    assign
                    (
                        boost::make_transform_iterator(values._container.begin(), [](PointerOrReferenceType& value) { return std::move(value); }),
                        boost::make_transform_iterator(values._container.end(), [](PointerOrReferenceType& value) { return std::move(value); })
                    );
                }

                template<
                    typename U,
                    template<typename> class C1
                >
                    requires std::convertible_to<PtrType<U>, PtrType<ValueType>>
                inline constexpr void assign(const DynamicPointerOrReferenceArray<U, pcat, rcat, C1>& values)
                {
                    assign
                    (
                        boost::make_transform_iterator(values._container.begin(), [](const PtrOrRef<U, pcat, rcat>& value) { return PointerOrReferenceType{ value }; }),
                        boost::make_transform_iterator(values._container.end(), [](const PtrOrRef<U, pcat, rcat>& value) { return PointerOrReferenceType{ value }; })
                    );
                }

                template<
                    typename U,
                    template<typename> class C1
                >
                    requires std::convertible_to<PtrType<U>, PtrType<ValueType>>
                inline constexpr void assign(DynamicPointerOrReferenceArray<U, pcat, rcat, C1>& values)
                {
                    assign
                    (
                        boost::make_transform_iterator(values._container.begin(), [](PtrOrRef<U, pcat, rcat>& value) { return PointerOrReferenceType{ std::move(value) }; }),
                        boost::make_transform_iterator(values._container.end(), [](PtrOrRef<U, pcat, rcat>& value) { return PointerOrReferenceType{ std::move(value) }; })
                    );
                }

                template<typename = void>
                    requires std::copyable<PointerOrReferenceType>
                inline constexpr void assign_pointer(const usize length, ArgCLRefType<PointerType> ptr)
                {
                    _container.assign(length, PointerOrReferenceType::ptr(ptr));
                }

                template<typename = void>
                    requires std::copyable<PointerOrReferenceType> && ReferenceFaster<PointerType> && std::movable<PointerType>
                inline constexpr void assign_pointer(const usize length, ArgRRefType<PointerType> value)
                {
                    _container.assign(length, PointerOrReferenceType::ptr(move<PointerType>(value)));
                }

                template<std::input_iterator It>
                    requires requires (const It it) { { *it } -> DecaySameAs<PointerType>; }
                inline constexpr void assign_pointer(It&& first, It&& last)
                {
                    if constexpr (std::is_same_v<decltype(*first), CLRefType<PointerType>>)
                    {
                        _container.assign
                        (
                            boost::make_transform_iterator(std::forward<It>(first), [](ArgCLRefType<PointerType> ptr) { return PointerOrReferenceType::ptr(ptr); }),
                            boost::make_transform_iterator(std::forward<It>(last), [](ArgCLRefType<PointerType> ptr) { return PointerOrReferenceType::ptr(ptr); })
                        );
                    }
                    else
                    {
                        _container.assign
                        (
                            boost::make_transform_iterator(std::forward<It>(first), [](ArgLRefType<PointerType> ptr) { return PointerOrReferenceType::ptr(move<PointerType>(ptr)); }),
                            boost::make_transform_iterator(std::forward<It>(last), [](ArgLRefType<PointerType> ptr) { return PointerOrReferenceType::ptr(move<PointerType>(ptr)); })
                        );
                    }
                }

                inline constexpr void assign_pointer(std::initializer_list<PointerType> ptrs)
                {
                    _container.assign
                    (
                        boost::make_transform_iterator(ptrs.begin(), [](PointerType& ptr) { return PointerOrReferenceType::ptr(move<PointerType>(ptr)); }),
                        boost::make_transform_iterator(ptrs.end(), [](PointerType& ptr) { return PointerOrReferenceType::ptr(move<PointerType>(ptr)); })
                    );
                }

                template<typename = void>
                    requires std::copyable<PointerOrReferenceType>
                inline constexpr void assign_reference(const usize length, CLRefType<ValueType> ref)
                {
                    _container.assign(length, PointerOrReferenceType::ref(ReferenceType{ ref }));
                }

                template<typename = void>
                    requires std::copyable<PointerOrReferenceType>
                inline constexpr void assign_reference(const usize length, ArgCLRefType<ReferenceType> ref)
                {
                    _container.assign(length, PointerOrReferenceType::ref(ref));
                }

                template<typename = void>
                    requires std::copyable<PointerOrReferenceType> && ReferenceFaster<ReferenceType> && std::movable<ReferenceType>
                inline constexpr void assign_reference(const usize length, ArgRRefType<ReferenceType> ref)
                {
                    _container.assign(length, PointerOrReferenceType::ref(move<ReferenceType>(ref)));
                }

                template<std::input_iterator It>
                    requires requires (const It it) { { *it } -> DecaySameAs<ValueType>; }
                inline constexpr void assign_reference(It&& first, It&& last)
                {
                    _container.assign
                    (
                        boost::make_transform_iterator(std::forward<It>(first), [](CLRefType<ValueType> value) { return PointerOrReferenceType::ref(value); }),
                        boost::make_transform_iterator(std::forward<It>(last), [](CLRefType<ValueType> value) { return PointerOrReferenceType::ref(value); })
                    );
                }

                template<std::input_iterator It>
                    requires std::copyable<ReferenceType> && requires (const It it) { { *it } -> DecaySameAs<ReferenceType>; }
                inline constexpr void assign_reference(It&& first, It&& last)
                {
                    _container.assign
                    (
                        boost::make_transform_iterator(std::forward<It>(first), [](ArgCLRefType<ReferenceType> ref) { return PointerOrReferenceType::ref(ref); }),
                        boost::make_transform_iterator(std::forward<It>(last), [](ArgCLRefType<ReferenceType> ref) { return PointerOrReferenceType::ref(ref); })
                    );
                }

                inline constexpr void assign_reference(std::initializer_list<ReferenceType> refs)
                {
                    _container.assign
                    (
                        boost::make_transform_iterator(refs.begin(), [](LRefType<ReferenceType> ref) { return PointerOrReferenceType::ref(move<ReferenceType>(ref)); }),
                        boost::make_transform_iterator(refs.end(), [](LRefType<ReferenceType> ref) { return PointerOrReferenceType::ref(move<ReferenceType>(ref)); })
                    );
                }

            public:
                template<typename = void>
                    requires requires (ContainerType& container) { { container.data() } -> DecaySameAs<PtrType<PointerOrReferenceType>>; }
                inline constexpr const PtrType<PointerOrReferenceType> data(void) noexcept
                {
                    return _container.data();
                }

                template<typename = void>
                    requires requires (ContainerType& container) { { container.data() } -> DecaySameAs<CPtrType<PointerOrReferenceType>>; }
                inline constexpr const CPtrType<PointerOrReferenceType> data(void) const noexcept
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

                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, ArgCLRefType<PointerOrReferenceType> value)
                {
                    return IterType{ _container.insert(pos._iter, value) };
                }

                template<typename = void>
                    requires ReferenceFaster<PointerOrReferenceType> && std::movable<PointerOrReferenceType>
                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, ArgRRefType<PointerOrReferenceType> value)
                {
                    return IterType{ _container.insert(pos._iter, move<PointerOrReferenceType>(value)) };
                }

                template<std::input_iterator It>
                    requires requires (const It& it) { { *it } -> DecaySameAs<PointerOrReferenceType>; }
                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, It&& first, It&& last)
                {
                    return IterType{ _container.insert(pos._iter, std::forward<It>(first), std::forward<It>(last)) };
                }

                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, std::initializer_list<PointerType> ptrs)
                {
                    return insert(pos, 
                        boost::make_transform_iterator(ptrs.begin(), [](PointerType& ptr) { return PointerOrReferenceType::ptr(std::move(ptr)); }),
                        boost::make_transform_iterator(ptrs.end(), [](PointerType& ptr) { return PointerOrReferenceType::ptr(std::move(ptr)); })
                    );
                }

                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, std::initializer_list<PointerOrReferenceType> values)
                {
                    return IterType{ _container.insert(pos._iter, std::move(values)) };
                }

                template<
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::copyable<PointerOrReferenceType>
                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, const StaticPointerOrReferenceArray<ValueType, len, pcat, rcat, C1>& values)
                {
                    return insert(pos, values._container.begin(), values._container.end());
                }

                template<
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::movable<PointerOrReferenceType>
                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, StaticPointerOrReferenceArray<ValueType, len, pcat, rcat, C1>&& values)
                {
                    return IterType{ std::move(values._container.begin(), values._container.end(), std::inserter(_container, pos._iter)) };
                }

                template<
                    typename U,
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::convertible_to<PtrType<U>, PtrType<ValueType>>
                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, const StaticPointerOrReferenceArray<U, len, pcat, rcat, C1>& values)
                {
                    return insert(pos,
                        boost::make_transform_iterator(values._container.begin(), [](const PtrOrRef<U, pcat, rcat>& value) { return PointerOrReferenceType{ value }; }),
                        boost::make_transform_iterator(values._container.end(), [](const PtrOrRef<U, pcat, rcat>& value) { return PointerOrReferenceType{ value }; })
                    );
                }

                template<
                    typename U,
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::convertible_to<PtrType<U>, PtrType<ValueType>>
                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, StaticPointerOrReferenceArray<U, len, pcat, rcat, C1>&& values)
                {
                    return insert(pos,
                        boost::make_transform_iterator(values._container.begin(), [](PtrOrRef<U, pcat, rcat>& value) { return PointerOrReferenceType{ std::move(value) }; }),
                        boost::make_transform_iterator(values._container.end(), [](PtrOrRef<U, pcat, rcat>& value) { return PointerOrReferenceType{ std::move(value) }; })
                    );
                }

                template<template<typename> class C1>
                    requires std::copyable<PointerOrReferenceType>
                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, const DynamicPointerOrReferenceArray<ValueType, pcat, rcat, C1>& values)
                {
                    return insert(pos, values._container.begin(), values._container.end());
                }

                template<template<typename> class C1>
                    requires std::movable<PointerOrReferenceType>
                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, DynamicPointerOrReferenceArray<ValueType, pcat, rcat, C1>&& values)
                {
                    return IterType{ std::move(values._container.begin(), values._container.end(), std::inserter(_container, pos._iter)) };
                }

                template<
                    typename U, 
                    template<typename> class C1
                >
                    requires std::convertible_to<PtrType<U>, PtrType<ValueType>>
                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, const DynamicPointerOrReferenceArray<U, pcat, rcat, C1>& values)
                {
                    return insert(pos,
                        boost::make_transform_iterator(values._container.begin(), [](const PtrOrRef<U, pcat, rcat>& value) { return PointerOrReferenceType{ value }; }),
                        boost::make_transform_iterator(values._container.end(), [](const PtrOrRef<U, pcat, rcat>& value) { return PointerOrReferenceType{ value }; })
                    );
                }

                template<
                    typename U,
                    template<typename> class C1
                >
                    requires std::convertible_to<PtrType<U>, PtrType<ValueType>>
                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, DynamicPointerOrReferenceArray<U, pcat, rcat, C1>&& values)
                {
                    return insert(pos,
                        boost::make_transform_iterator(values._container.begin(), [](PtrOrRef<U, pcat, rcat>& value) { return PointerOrReferenceType{ std::move(value) }; }),
                        boost::make_transform_iterator(values._container.end(), [](PtrOrRef<U, pcat, rcat>& value) { return PointerOrReferenceType{ std::move(value) }; })
                    );
                }

                inline constexpr RetType<IterType> insert_ptr(ArgCLRefType<ConstIterType> pos, ArgCLRefType<PointerType> ptr)
                {
                    return IterType{ _container.insert(pos._iter, PointerOrReferenceType::ptr(ptr)) };
                }

                template<typename = void>
                    requires ReferenceFaster<PointerType> && std::movable<PointerType>
                inline constexpr RetType<IterType> insert_ptr(ArgCLRefType<ConstIterType> pos, ArgRRefType<PointerType> ptr)
                {
                    return IterType{ _container.insert(pos._iter, PointerOrReferenceType::ptr(std::move(ptr))) };
                }

                template<std::input_iterator It>
                    requires requires (const It& it) { { *it } -> DecaySameAs<PointerType>; }
                inline constexpr RetType<IterType> insert_ptr(ArgCLRefType<ConstIterType> pos, It&& first, It&& last)
                {
                    if constexpr (std::is_same_v<decltype(*first), CLRefType<PointerType>>)
                    {
                        return IterType{ _container.insert(pos._iter, 
                            boost::make_transform_iterator(std::forward<It>(first), [](ArgCLRefType<PointerType> ptr) { return PointerOrReferenceType::ptr(ptr); }),
                            boost::make_transform_iterator(std::forward<It>(last), [](ArgCLRefType<PointerType> ptr) { return PointerOrReferenceType::ptr(ptr); })
                        ) };
                    }
                    else
                    {
                        return IterType{ _container.insert(pos._iter,
                            boost::make_transform_iterator(std::forward<It>(first), [](ArgLRefType<PointerType> ptr) { return PointerOrReferenceType::ptr(move<PointerType>(ptr)); }),
                            boost::make_transform_iterator(std::forward<It>(last), [](ArgLRefType<PointerType> ptr) { return PointerOrReferenceType::ptr(move<PointerType>(ptr)); })
                        ) };
                    }
                }

                inline constexpr RetType<IterType> insert_ptr(ArgCLRefType<ConstIterType> pos, std::initializer_list<PointerType> ptrs)
                {
                    return insert(pos, std::move(ptrs));
                }

                inline constexpr RetType<IterType> insert_reference(ArgCLRefType<ConstIterType> pos, CLRefType<ValueType> value)
                {
                    return insert(pos._iter, PointerOrReferenceType::ref(value));
                }

                inline constexpr RetType<IterType> insert_reference(ArgCLRefType<ConstIterType> pos, ArgRRefType<ReferenceType> ref)
                {
                    return insert(pos._iter, PointerOrReferenceType::ref(move<ReferenceType>(ref)));
                }

                template<std::input_iterator It>
                    requires requires (const It it) { { *it } -> DecaySameAs<ValueType>; }
                inline constexpr RetType<IterType> insert_reference(ArgCLRefType<ConstIterType> pos, It&& first, It&& last)
                {
                    return insert(pos._iter,
                        boost::make_transform_iterator(std::forward<It>(first), [](CLRefType<ValueType> value) { return PointerOrReferenceType::ref(value); }),
                        boost::make_transform_iterator(std::forward<It>(last), [](CLRefType<ValueType> value) { return PointerOrReferenceType::ref(value); })
                    );
                }

                template<std::input_iterator It>
                    requires std::copyable<ReferenceType> && requires (const It it) { { *it } -> DecaySameAs<ReferenceType>; }
                inline constexpr RetType<IterType> insert_reference(ArgCLRefType<ConstIterType> pos, It&& first, It&& last)
                {
                    return insert(pos._iter,
                        boost::make_transform_iterator(std::forward<It>(first), [](ArgCLRefType<ReferenceType> ref) { return PointerOrReferenceType::ref(ref); }),
                        boost::make_transform_iterator(std::forward<It>(last), [](ArgCLRefType<ReferenceType> ref) { return PointerOrReferenceType::ref(ref); })
                    );
                }

                inline constexpr RetType<IterType> insert_reference(ArgCLRefType<ConstIterType> pos, std::initializer_list<ReferenceType> refs)
                {
                    return insert(pos._iter,
                        boost::make_transform_iterator(refs.begin(), [](ArgLRefType<ReferenceType> ref) { return PointerOrReferenceType::ref(ref); }),
                        boost::make_transform_iterator(refs.end(), [](ArgLRefType<ReferenceType> ref) { return PointerOrReferenceType::ref(ref); })
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
                inline constexpr void push_back(ArgCLRefType<PointerOrReferenceType> value)
                {
                    _container.push_back(value);
                }

                template<typename = void>
                    requires ReferenceFaster<PointerOrReferenceType> && std::movable<PointerOrReferenceType>
                inline constexpr void push_back(ArgRRefType<PointerOrReferenceType> value)
                {
                    _container.push_back(move<PointerOrReferenceType>(value));
                }

                template<
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::copyable<PointerOrReferenceType>
                inline constexpr void push_back(const StaticPointerOrReferenceArray<ValueType, len, pcat, rcat, C1>& values)
                {
                    insert(this->end(), values);
                }

                template<
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::movable<PointerOrReferenceType>
                inline constexpr void push_back(StaticPointerOrReferenceArray<ValueType, len, pcat, rcat, C1>&& values)
                {
                    insert(this->end(), std::move(values));
                }

                template<
                    typename U,
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::convertible_to<PtrType<U>, PtrType<ValueType>>
                inline constexpr void push_back(const StaticPointerOrReferenceArray<U, len, pcat, rcat, C1>& values)
                {
                    insert(this->end(), values);
                }

                template<
                    typename U,
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::convertible_to<PtrType<U>, PtrType<ValueType>>
                inline constexpr void push_back(StaticPointerOrReferenceArray<U, len, pcat, rcat, C1>&& values)
                {
                    insert(this->end(), std::move(values));
                }

                template<template<typename> class C1>
                    requires std::copyable<PointerOrReferenceType>
                inline constexpr void push_back(const DynamicPointerOrReferenceArray<ValueType, pcat, rcat, C1>& values)
                {
                    insert(this->end(), values);
                }

                template<template<typename> class C1>
                    requires std::movable<PointerOrReferenceType>
                inline constexpr void push_back(DynamicPointerOrReferenceArray<ValueType, pcat, rcat, C1>&& values)
                {
                    insert(this->end(), std::move(values));
                }

                template<
                    typename U,
                    template<typename> class C1
                >
                    requires std::convertible_to<PtrType<U>, PtrType<ValueType>>
                inline constexpr void push_back(const DynamicPointerOrReferenceArray<U, pcat, rcat, C1>& values)
                {
                    insert(this->end(), values);
                }

                template<
                    typename U,
                    template<typename> class C1
                >
                    requires std::convertible_to<PtrType<U>, PtrType<ValueType>>
                inline constexpr void push_back(DynamicPointerOrReferenceArray<U, pcat, rcat, C1>&& values)
                {
                    insert(this->end(), std::move(values));
                }

                inline constexpr void push_back_pointer(ArgCLRefType<PointerType> ptr)
                {
                    _container.push_back(PointerOrReferenceType::ptr(ptr));
                }

                template<typename = void>
                    requires ReferenceFaster<PointerType> && std::movable<PointerType>
                inline constexpr void push_back_pointer(ArgRRefType<PointerType> ptr)
                {
                    _container.push_back(PointerOrReferenceType::ptr(move<PointerType>(ptr)));
                }

                inline constexpr void push_back_reference(CLRefType<ValueType> ref)
                {
                    _container.push_back(PointerOrReferenceType::ref(ref));
                }

                inline constexpr void push_back_reference(ArgCLRefType<ReferenceType> ref)
                {
                    _container.push_back(PointerOrReferenceType::ref(ref));
                }

                template<typename = void>
                    requires ReferenceFaster<ReferenceType> && std::movable<ReferenceType>
                inline constexpr void push_back_reference(ArgRRefType<ReferenceType> ref)
                {
                    _container.push_back(PointerOrReferenceType::ref(move<ReferenceType>(ref)));
                }

                inline constexpr RetType<PointerOrReferenceType> pop_back(void)
                {
                    auto back = move<PointerOrReferenceType>(this->back());
                    _container.pop_back();
                    return back;
                }

            public:
                inline constexpr void push_front(ArgCLRefType<PointerOrReferenceType> value)
                {
                    insert(this->begin(), value);
                }

                template<typename = void>
                    requires requires (ContainerType& container) { container.push_front(std::declval<PointerOrReferenceType>()); }
                inline constexpr void push_front(ArgCLRefType<PointerOrReferenceType> value)
                {
                    _container.push_front(value);
                }

                template<typename = void>
                    requires ReferenceFaster<PointerOrReferenceType> && std::movable<PointerOrReferenceType>
                inline constexpr void push_front(ArgRRefType<PointerOrReferenceType> value)
                {
                    insert(this->begin(), move<PointerOrReferenceType>(value));
                }

                template<typename = void>
                    requires ReferenceFaster<PointerOrReferenceType> && std::movable<PointerOrReferenceType> &&
                        requires (ContainerType& container) { container.push_front(std::declval<PointerOrReferenceType>()); }
                inline constexpr void push_front(ArgRRefType<PointerOrReferenceType> value)
                {
                    _container.push_front(move<PointerOrReferenceType>(value));
                }

                template<
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::copyable<PointerOrReferenceType>
                inline constexpr void push_front(const StaticPointerOrReferenceArray<ValueType, len, pcat, rcat, C1>& values)
                {
                    insert(this->begin(), values);
                }

                template<
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::movable<PointerOrReferenceType>
                inline constexpr void push_front(StaticPointerOrReferenceArray<ValueType, len, pcat, rcat, C1>&& values)
                {
                    insert(this->begin(), std::move(values));
                }

                template<
                    typename U,
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::convertible_to<PtrType<U>, PtrType<ValueType>>
                inline constexpr void push_front(const StaticPointerOrReferenceArray<U, len, pcat, rcat, C1>& values)
                {
                    insert(this->begin(), values);
                }

                template<
                    typename U,
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::convertible_to<PtrType<U>, PtrType<ValueType>>
                inline constexpr void push_front(StaticPointerOrReferenceArray<U, len, pcat, rcat, C1>&& values)
                {
                    insert(this->begin(), std::move(values));
                }

                template<template<typename> class C1>
                    requires std::copyable<PointerOrReferenceType>
                inline constexpr void push_front(const DynamicPointerOrReferenceArray<ValueType, pcat, rcat, C1>& values)
                {
                    insert(this->begin(), values);
                }

                template<template<typename> class C1>
                    requires std::movable<PointerOrReferenceType>
                inline constexpr void push_front(DynamicPointerOrReferenceArray<ValueType, pcat, rcat, C1>&& values)
                {
                    insert(this->begin(), std::move(values));
                }

                template<
                    typename U,
                    template<typename> class C1
                >
                    requires std::convertible_to<PtrType<U>, PtrType<ValueType>>
                inline constexpr void push_front(const DynamicPointerOrReferenceArray<U, pcat, rcat, C1>& values)
                {
                    insert(this->begin(), values);
                }

                template<
                    typename U,
                    template<typename> class C1
                >
                    requires std::convertible_to<PtrType<U>, PtrType<ValueType>>
                inline constexpr void push_front(DynamicPointerOrReferenceArray<U, pcat, rcat, C1>&& values)
                {
                    insert(this->begin(), std::move(values));
                }

                inline constexpr void push_front_pointer(ArgCLRefType<PointerType> ptr)
                {
                    insert(this->begin(), PointerOrReferenceType::ptr(ptr));
                }

                template<typename = void>
                    requires requires (ContainerType& container) { container.push_front(std::declval<PointerOrReferenceType>()); }
                inline constexpr void push_front_pointer(ArgCLRefType<PointerType> ptr)
                {
                    _container.push_front(PointerOrReferenceType::ptr(ptr));
                }

                template<typename = void>
                    requires ReferenceFaster<PointerType> && std::movable<PointerType>
                inline constexpr void push_front_pointer(ArgRRefType<PointerType> ptr)
                {
                    insert(this->begin(), PointerOrReferenceType::ptr(move<PointerType>(ptr)));
                }

                template<typename = void>
                    requires ReferenceFaster<PointerType> && std::movable<PointerType> &&
                        requires (ContainerType& container) { container.push_front(std::declval<PointerOrReferenceType>()); }
                inline constexpr void push_front_pointer(ArgRRefType<PointerType> ptr)
                {
                    _container.push_front(PointerOrReferenceType::ptr(move<PointerType>(ptr)));
                }

                inline constexpr void push_front_reference(CLRefType<ValueType> ref)
                {
                    insert(this->begin(), PointerOrReferenceType::ref(ref));
                }

                template<typename = void>
                    requires requires (ContainerType& container) { container.push_front(std::declval<PointerOrReferenceType>()); }
                inline constexpr void push_front_reference(CLRefType<ValueType> ref)
                {
                    _container.push_front(PointerOrReferenceType::ref(ref));
                }

                inline constexpr void push_front_reference(ArgCLRefType<ReferenceType> ref)
                {
                    insert(this->begin(), PointerOrReferenceType::ref(ref));
                }

                template<typename = void>
                    requires requires (ContainerType& container) { container.push_front(std::declval<PointerOrReferenceType>()); }
                inline constexpr void push_front_reference(ArgCLRefType<ReferenceType> ref)
                {
                    _container.push_front(PointerOrReferenceType::ref(ref));
                }

                template<typename = void>
                    requires ReferenceFaster<ReferenceType>&& std::movable<ReferenceType>
                inline constexpr void push_front_reference(ArgRRefType<ReferenceType> ref)
                {
                    insert(this->begin(), PointerOrReferenceType::ref(move<ReferenceType>(ref)));
                }

                template<typename = void>
                    requires ReferenceFaster<ReferenceType> && std::movable<ReferenceType> &&
                        requires (ContainerType& container) { container.push_front(std::declval<PointerOrReferenceType>()); }
                inline constexpr void push_front_reference(ArgRRefType<ReferenceType> ref)
                {
                    _container.push_front(PointerOrReferenceType::ref(move<ReferenceType>(ref)));
                }

                inline constexpr RetType<PointerOrReferenceType> pop_front(void)
                {
                    auto front = move<PointerOrReferenceType>(this->front());
                    _container.erase(_container.begin());
                    return front;
                }

                template<typename = void>
                    requires requires (ContainerType& container) { container.pop_front(); }
                inline constexpr RetType<PointerOrReferenceType> pop_front(void)
                {
                    auto back = move<PointerOrReferenceType>(this->front());
                    _container.pop_front();
                    return back;
                }

            public:
                template<typename = void>
                    requires std::copyable<PointerOrReferenceType>
                inline constexpr void resize(const usize length, ArgCLRefType<PointerOrReferenceType> value)
                {
                    _container.resize(length, value);
                }

            public:
                inline constexpr const bool operator==(const DynamicPointerOrReferenceArray& rhs) const noexcept
                {
                    return _container == rhs._container;
                }

                inline constexpr const bool operator!=(const DynamicPointerOrReferenceArray& rhs) const noexcept
                {
                    return _container != rhs._container;
                }

            public:
                inline constexpr const bool operator<(const DynamicPointerOrReferenceArray& rhs) const noexcept
                {
                    return _container < rhs._container;
                }

                inline constexpr const bool operator<=(const DynamicPointerOrReferenceArray& rhs) const noexcept
                {
                    return _container <= rhs._container;
                }

                inline constexpr const bool operator>(const DynamicPointerOrReferenceArray& rhs) const noexcept
                {
                    return _container > rhs._container;
                }

                inline constexpr const bool operator>=(const DynamicPointerOrReferenceArray& rhs) const noexcept
                {
                    return _container >= rhs._container;
                }

            public:
                inline constexpr decltype(auto) operator<=>(const DynamicPointerOrReferenceArray& rhs) const noexcept
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
            PointerCategory pcat = PointerCategory::Shared,
            ReferenceCategory rcat = ReferenceCategory::Reference,
            template<typename, usize> class C = std::array
        >
        using PtrOrRefArray = pointer_or_reference_array::StaticPointerOrReferenceArray<T, len, pcat, rcat, C>;

        template<
            typename T,
            usize len,
            ReferenceCategory rcat = ReferenceCategory::Reference,
            template<typename, usize> class C = std::array
        >
        using SharedOrRefArray = pointer_or_reference_array::StaticPointerOrReferenceArray<T, len, PointerCategory::Shared, rcat, C>;

        template<
            typename T,
            usize len,
            ReferenceCategory rcat = ReferenceCategory::Reference,
            template<typename, usize> class C = std::array
        >
        using UniqueOrRefArray = pointer_or_reference_array::StaticPointerOrReferenceArray<T, len, PointerCategory::Unique, rcat, C>;

        template<
            typename T,
            PointerCategory pcat = PointerCategory::Shared,
            ReferenceCategory rcat = ReferenceCategory::Reference,
            template<typename> class C = std::vector
        >
        using DynPtrOrRefArray = pointer_or_reference_array::DynamicPointerOrReferenceArray<T, pcat, rcat, C>;

        template<
            typename T,
            ReferenceCategory rcat = ReferenceCategory::Reference,
            template<typename> class C = std::vector
        >
        using DynSharedOrRefArray = pointer_or_reference_array::DynamicPointerOrReferenceArray<T, PointerCategory::Shared, rcat, C>;

        template<
            typename T,
            ReferenceCategory rcat = ReferenceCategory::Reference,
            template<typename> class C = std::vector
        >
        using DynUniqueOrRefArray = pointer_or_reference_array::DynamicPointerOrReferenceArray<T, PointerCategory::Unique, rcat, C>;
    };
};

namespace std
{
    template<typename T, typename C, typename Self>
    inline void swap(ospf::pointer_or_reference_array::PointerOrReferenceArrayImpl<T, C, Self>& lhs, ospf::pointer_or_reference_array::PointerOrReferenceArrayImpl<T, C, Self>& rhs) noexcept
    {
        lhs.swap(rhs);
    }
};

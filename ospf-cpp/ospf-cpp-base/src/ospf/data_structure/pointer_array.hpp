#pragma once

#include <ospf/basic_definition.hpp>
#include <ospf/concepts/with_default.hpp>
#include <ospf/literal_constant.hpp>
#include <ospf/memory/pointer.hpp>
#include <ospf/memory/reference.hpp>
#include <ospf/functional/iterator.hpp>
#include <boost/iterator/transform_iterator.hpp>

namespace ospf
{
    inline namespace data_structure
    {
        namespace pointer_array
        {
            template<
                typename T,
                usize len,
                PointerCategory cat,
                template<typename, usize> class C
            >
            class StaticPointerArray;

            template<
                typename T,
                PointerCategory cat,
                template<typename> class C
            >
            class DynamicPointerArray;

            template<typename T, PointerCategory cat, typename C>
            class PointerArrayConstIterator
                : public RandomIteratorImpl<const pointer::Ptr<OriginType<T>, cat>, typename OriginType<C>::const_iterator, PointerArrayConstIterator<T, cat, C>>
            {
                template<
                    typename T,
                    usize len,
                    PointerCategory cat,
                    template<typename, usize> class C
                >
                friend class StaticPointerArray;

                template<
                    typename T,
                    PointerCategory cat,
                    template<typename> class C
                >
                friend class DynamicPointerArray;

            public:
                using ValueType = pointer::Ptr<OriginType<T>, cat>;
                using ContainerType = OriginType<C>;
                using IterType = typename ContainerType::const_iterator;

            private:
                using Impl = RandomIteratorImpl<ConstType<ValueType>, IterType, PointerArrayConstIterator<T, cat, C>>;

            public:
                constexpr PointerArrayConstIterator(ArgCLRefType<IterType> iter)
                    : Impl(iter) {}
                constexpr PointerArrayConstIterator(const PointerArrayConstIterator& ano) = default;
                constexpr PointerArrayConstIterator(PointerArrayConstIterator&& ano) noexcept = default;
                constexpr PointerArrayConstIterator& operator=(const PointerArrayConstIterator& rhs) = default;
                constexpr PointerArrayConstIterator& operator=(PointerArrayConstIterator&& rhs) noexcept = default;
                constexpr ~PointerArrayConstIterator(void) noexcept = default;

            public:
                inline constexpr const bool null(void) const
                {
                    return *this->_iter == nullptr;
                }

            OSPF_CRTP_PERMISSION:
                inline static constexpr CLRefType<ValueType> OSPF_CRTP_FUNCTION(get)(ArgCLRefType<IterType> iter) noexcept
                {
                    return *iter;
                }

                inline static constexpr const PointerArrayConstIterator OSPF_CRTP_FUNCTION(construct)(ArgCLRefType<IterType> iter) noexcept
                {
                    return PointerArrayConstIterator{ iter };
                }
            };

            template<typename T, PointerCategory cat, typename C>
            class PointerArrayIterator
                : public RandomIteratorImpl<pointer::Ptr<OriginType<T>, cat>, typename OriginType<C>::iterator, PointerArrayIterator<T, cat, C>>
            {
                template<
                    typename T,
                    usize len,
                    PointerCategory cat,
                    template<typename, usize> class C
                >
                friend class StaticPointerArray;

                template<
                    typename T,
                    PointerCategory cat,
                    template<typename> class C
                >
                friend class DynamicPointerArray;

            public:
                using ValueType = pointer::Ptr<OriginType<T>, cat>;
                using ContainerType = OriginType<C>;
                using IterType = typename ContainerType::iterator;

            private:
                using Impl = RandomIteratorImpl<ValueType, IterType, PointerArrayIterator<T, cat, C>>;

            public:
                constexpr PointerArrayIterator(ArgCLRefType<IterType> iter)
                    : Impl(iter) {}
                constexpr PointerArrayIterator(const PointerArrayIterator& ano) = default;
                constexpr PointerArrayIterator(PointerArrayIterator&& ano) noexcept = default;
                constexpr PointerArrayIterator& operator=(const PointerArrayIterator& rhs) = default;
                constexpr PointerArrayIterator& operator=(PointerArrayIterator&& rhs) noexcept = default;
                constexpr ~PointerArrayIterator(void) noexcept = default;

            public:
                inline constexpr const bool null(void) const
                {
                    return *this->_iter == nullptr;
                }

            public:
                inline constexpr operator const PointerArrayConstIterator<T, cat, C>(void) const noexcept
                {
                    return PointerArrayConstIterator<T, cat, C>{ this->_iter };
                }

            OSPF_CRTP_PERMISSION:
                inline static constexpr LRefType<ValueType> OSPF_CRTP_FUNCTION(get)(ArgCLRefType<IterType> iter) noexcept
                {
                    return *iter;
                }

                inline static constexpr const PointerArrayIterator OSPF_CRTP_FUNCTION(construct)(ArgCLRefType<IterType> iter) noexcept
                {
                    return PointerArrayIterator{ iter };
                }
            };

            template<typename T, PointerCategory cat, typename C>
            class PointerArrayConstReverseIterator
                : public RandomIteratorImpl<const pointer::Ptr<OriginType<T>, cat>, typename OriginType<C>::const_reverse_iterator, PointerArrayConstReverseIterator<T, cat, C>>
            {
                template<
                    typename T,
                    usize len,
                    PointerCategory cat,
                    template<typename, usize> class C
                >
                friend class StaticPointerArray;

                template<
                    typename T,
                    PointerCategory cat,
                    template<typename> class C
                >
                friend class DynamicPointerArray;

            public:
                using ValueType = pointer::Ptr<OriginType<T>, cat>;
                using ContainerType = OriginType<C>;
                using IterType = typename ContainerType::const_reverse_iterator;

            private:
                using Impl = RandomIteratorImpl<ConstType<ValueType>, IterType, PointerArrayConstReverseIterator<T, cat, C>>;

            public:
                constexpr PointerArrayConstReverseIterator(ArgCLRefType<IterType> iter)
                    : Impl(iter) {}
                constexpr PointerArrayConstReverseIterator(const PointerArrayConstReverseIterator& ano) = default;
                constexpr PointerArrayConstReverseIterator(PointerArrayConstReverseIterator&& ano) noexcept = default;
                constexpr PointerArrayConstReverseIterator& operator=(const PointerArrayConstReverseIterator& rhs) = default;
                constexpr PointerArrayConstReverseIterator& operator=(PointerArrayConstReverseIterator&& rhs) noexcept = default;
                constexpr ~PointerArrayConstReverseIterator(void) = default;

            public:
                inline constexpr const bool null(void) const
                {
                    return *this->_iter == nullptr;
                }

            OSPF_CRTP_PERMISSION:
                inline static constexpr CLRefType<ValueType> OSPF_CRTP_FUNCTION(get)(ArgCLRefType<IterType> iter) noexcept
                {
                    return *iter;
                }

                inline static constexpr const PointerArrayConstReverseIterator OSPF_CRTP_FUNCTION(construct)(ArgCLRefType<IterType> iter) noexcept
                {
                    return PointerArrayConstReverseIterator{ iter };
                }
            };

            template<typename T, PointerCategory cat, typename C>
            class PointerArrayReverseIterator
                : public RandomIteratorImpl<pointer::Ptr<OriginType<T>, cat>, typename OriginType<C>::reverse_iterator, PointerArrayReverseIterator<T, cat, C>>
            {
                template<
                    typename T,
                    usize len,
                    PointerCategory cat,
                    template<typename, usize> class C
                >
                friend class StaticPointerArray;

                template<
                    typename T,
                    PointerCategory cat,
                    template<typename> class C
                >
                friend class DynamicPointerArray;

            public:
                using ValueType = pointer::Ptr<OriginType<T>, cat>;
                using ContainerType = OriginType<C>;
                using IterType = typename ContainerType::reverse_iterator;

            private:
                using Impl = RandomIteratorImpl<ValueType, IterType, PointerArrayReverseIterator<T, cat, C>>;

            public:
                constexpr PointerArrayReverseIterator(ArgCLRefType<IterType> iter)
                    : Impl(iter) {}
                constexpr PointerArrayReverseIterator(const PointerArrayReverseIterator& ano) = default;
                constexpr PointerArrayReverseIterator(PointerArrayReverseIterator&& ano) noexcept = default;
                constexpr PointerArrayReverseIterator& operator=(const PointerArrayReverseIterator& rhs) = default;
                constexpr PointerArrayReverseIterator& operator=(PointerArrayReverseIterator&& rhs) noexcept = default;
                constexpr ~PointerArrayReverseIterator(void) noexcept = default;

            public:
                inline constexpr const bool null(void) const
                {
                    return *this->_iter == nullptr;
                }

            OSPF_CRTP_PERMISSION:
                inline static constexpr LRefType<ValueType> OSPF_CRTP_FUNCTION(get)(ArgCLRefType<IterType> iter) noexcept
                {
                    return *iter;
                }

                inline static constexpr const PointerArrayReverseIterator OSPF_CRTP_FUNCTION(construct)(ArgCLRefType<IterType> iter) noexcept
                {
                    return PointerArrayReverseIterator{ iter };
                }
            };

            template<typename T, PointerCategory cat, typename C>
            class PointerArrayConstUncheckedIterator
                : public RandomIteratorImpl<ConstType<T>, typename OriginType<C>::const_iterator, PointerArrayConstUncheckedIterator<T, cat, C>>
            {
                template<
                    typename T,
                    usize len,
                    PointerCategory cat,
                    template<typename, usize> class C
                >
                friend class StaticPointerArray;

                template<
                    typename T,
                    PointerCategory cat,
                    template<typename> class C
                >
                friend class DynamicPointerArray;

            public:
                using ValueType = OriginType<T>;
                using ContainerType = OriginType<C>;
                using IterType = typename ContainerType::const_iterator;

            private:
                using Impl = RandomIteratorImpl<ConstType<ValueType>, IterType, PointerArrayConstUncheckedIterator<T, cat, C>>;

            public:
                constexpr PointerArrayConstUncheckedIterator(ArgCLRefType<IterType> iter)
                    : Impl(iter) {}
                constexpr PointerArrayConstUncheckedIterator(const PointerArrayConstUncheckedIterator& ano) = default;
                constexpr PointerArrayConstUncheckedIterator(PointerArrayConstUncheckedIterator&& ano) noexcept = default;
                constexpr PointerArrayConstUncheckedIterator& operator=(const PointerArrayConstUncheckedIterator& rhs) = default;
                constexpr PointerArrayConstUncheckedIterator& operator=(PointerArrayConstUncheckedIterator&& rhs) noexcept = default;
                constexpr ~PointerArrayConstUncheckedIterator(void) = default;

            public:
                inline constexpr const bool null(void) const
                {
                    return *this->_iter == nullptr;
                }

            OSPF_CRTP_PERMISSION:
                inline static constexpr CLRefType<ValueType> OSPF_CRTP_FUNCTION(get)(ArgCLRefType<IterType> iter) noexcept
                {
                    return **iter;
                }

                inline static constexpr const PointerArrayConstUncheckedIterator OSPF_CRTP_FUNCTION(construct)(ArgCLRefType<IterType> iter) noexcept
                {
                    return PointerArrayConstUncheckedIterator{ iter };
                }
            };

            template<typename T, PointerCategory cat, typename C>
            class PointerArrayUncheckedIterator
                : public RandomIteratorImpl<OriginType<T>, typename OriginType<C>::iterator, PointerArrayUncheckedIterator<T, cat, C>>
            {
                template<
                    typename T,
                    usize len,
                    PointerCategory cat,
                    template<typename, usize> class C
                >
                friend class StaticPointerArray;

                template<
                    typename T,
                    PointerCategory cat,
                    template<typename> class C
                >
                friend class DynamicPointerArray;

            public:
                using ValueType = OriginType<T>;
                using ContainerType = OriginType<C>;
                using IterType = typename ContainerType::iterator;

            private:
                using Impl = RandomIteratorImpl<ValueType, IterType, PointerArrayUncheckedIterator<T, cat, C>>;

            public:
                constexpr PointerArrayUncheckedIterator(ArgCLRefType<IterType> iter)
                    : Impl(iter) {}
                constexpr PointerArrayUncheckedIterator(const PointerArrayUncheckedIterator& ano) = default;
                constexpr PointerArrayUncheckedIterator(PointerArrayUncheckedIterator&& ano) noexcept = default;
                constexpr PointerArrayUncheckedIterator& operator=(const PointerArrayUncheckedIterator& rhs) = default;
                constexpr PointerArrayUncheckedIterator& operator=(PointerArrayUncheckedIterator&& rhs) noexcept = default;
                constexpr ~PointerArrayUncheckedIterator(void) = default;

            public:
                inline constexpr const bool null(void) const
                {
                    return *this->_iter == nullptr;
                }

            public:
                inline constexpr operator const PointerArrayConstUncheckedIterator<T, cat, C>(void) const noexcept
                {
                    return PointerArrayConstUncheckedIterator<T, cat, C>{ this->_iter };
                }

            OSPF_CRTP_PERMISSION:
                inline static constexpr LRefType<ValueType> OSPF_CRTP_FUNCTION(get)(ArgCLRefType<IterType> iter) noexcept
                {
                    return **iter;
                }

                inline static constexpr const PointerArrayUncheckedIterator OSPF_CRTP_FUNCTION(construct)(ArgCLRefType<IterType> iter) noexcept
                {
                    return PointerArrayUncheckedIterator{ iter };
                }
            };

            template<typename T, PointerCategory cat, typename C>
            class PointerArrayConstUncheckedReverseIterator
                : public RandomIteratorImpl<ConstType<T>, typename OriginType<C>::const_reverse_iterator, PointerArrayConstUncheckedReverseIterator<T, cat, C>>
            {
                template<
                    typename T,
                    usize len,
                    PointerCategory cat,
                    template<typename, usize> class C
                >
                friend class StaticPointerArray;

                template<
                    typename T,
                    PointerCategory cat,
                    template<typename> class C
                >
                friend class DynamicPointerArray;

            public:
                using ValueType = OriginType<T>;
                using ContainerType = OriginType<C>;
                using IterType = typename ContainerType::const_reverse_iterator;

            private:
                using Impl = RandomIteratorImpl<ConstType<ValueType>, IterType, PointerArrayConstUncheckedReverseIterator<T, cat, C>>;

            public:
                constexpr PointerArrayConstUncheckedReverseIterator(ArgCLRefType<IterType> iter)
                    : Impl(iter) {}
                constexpr PointerArrayConstUncheckedReverseIterator(const PointerArrayConstUncheckedReverseIterator& ano) = default;
                constexpr PointerArrayConstUncheckedReverseIterator(PointerArrayConstUncheckedReverseIterator&& ano) noexcept = default;
                constexpr PointerArrayConstUncheckedReverseIterator& operator=(const PointerArrayConstUncheckedReverseIterator& rhs) = default;
                constexpr PointerArrayConstUncheckedReverseIterator& operator=(PointerArrayConstUncheckedReverseIterator&& rhs) noexcept = default;
                constexpr ~PointerArrayConstUncheckedReverseIterator(void) = default;

            public:
                inline constexpr const bool null(void) const
                {
                    return *this->_iter == nullptr;
                }

            OSPF_CRTP_PERMISSION:
                inline static constexpr CLRefType<ValueType> OSPF_CRTP_FUNCTION(get)(ArgCLRefType<IterType> iter) noexcept
                {
                    return **iter;
                }

                inline static constexpr const PointerArrayConstUncheckedReverseIterator OSPF_CRTP_FUNCTION(construct)(ArgCLRefType<IterType> iter) noexcept
                {
                    return PointerArrayConstUncheckedReverseIterator{ iter };
                }
            };

            template<typename T, PointerCategory cat, typename C>
            class PointerArrayUncheckedReverseIterator
                : public RandomIteratorImpl<OriginType<T>, typename OriginType<C>::reverse_iterator, PointerArrayUncheckedReverseIterator<T, cat, C>>
            {
                template<
                    typename T,
                    usize len,
                    PointerCategory cat,
                    template<typename, usize> class C
                >
                friend class StaticPointerArray;

                template<
                    typename T,
                    PointerCategory cat,
                    template<typename> class C
                >
                friend class DynamicPointerArray;

            public:
                using ValueType = OriginType<T>;
                using ContainerType = OriginType<C>;
                using IterType = typename ContainerType::reverse_iterator;

            private:
                using Impl = RandomIteratorImpl<ValueType, IterType, PointerArrayUncheckedReverseIterator<T, cat, C>>;

            public:
                constexpr PointerArrayUncheckedReverseIterator(ArgCLRefType<IterType> iter)
                    : Impl(iter) {}
                constexpr PointerArrayUncheckedReverseIterator(const PointerArrayUncheckedReverseIterator& ano) = default;
                constexpr PointerArrayUncheckedReverseIterator(PointerArrayUncheckedReverseIterator&& ano) noexcept = default;
                constexpr PointerArrayUncheckedReverseIterator& operator=(const PointerArrayUncheckedReverseIterator& rhs) = default;
                constexpr PointerArrayUncheckedReverseIterator& operator=(PointerArrayUncheckedReverseIterator&& rhs) noexcept = default;
                constexpr ~PointerArrayUncheckedReverseIterator(void) = default;

            public:
                inline constexpr const bool null(void) const
                {
                    return *this->_iter == nullptr;
                }

            OSPF_CRTP_PERMISSION:
                inline static constexpr LRefType<ValueType> OSPF_CRTP_FUNCTION(get)(ArgCLRefType<IterType> iter) noexcept
                {
                    return **iter;
                }

                inline static constexpr const PointerArrayUncheckedReverseIterator OSPF_CRTP_FUNCTION(construct)(ArgCLRefType<IterType> iter) noexcept
                {
                    return PointerArrayUncheckedReverseIterator{ iter };
                }
            };

            template<typename T, typename C>
            struct PointerArrayAccessPolicy;

            template<
                typename T,
                usize len,
                PointerCategory cat,
                template<typename, usize> class C
            >
            struct PointerArrayAccessPolicy<T, C<pointer::Ptr<OriginType<T>, cat>, len>>
            {
            public:
                using ValueType = OriginType<T>;
                using PointerType = pointer::Ptr<ValueType, cat>;
                using ContainerType = C<PointerType, len>;
                using IterType = PointerArrayIterator<ValueType, cat, ContainerType>;
                using ConstIterType = PointerArrayConstIterator<ValueType, cat, ContainerType>;
                using ReverseIterType = PointerArrayReverseIterator<ValueType, cat, ContainerType>;
                using ConstReverseIterType = PointerArrayConstReverseIterator<ValueType, cat, ContainerType>;
                using UncheckedIterType = PointerArrayUncheckedIterator<ValueType, cat, ContainerType>;
                using ConstUncheckedIterType = PointerArrayConstUncheckedIterator<ValueType, cat, ContainerType>;
                using UncheckedReverseIterType = PointerArrayUncheckedReverseIterator<ValueType, cat, ContainerType>;
                using ConstUncheckedReverseIterType = PointerArrayConstUncheckedReverseIterator<ValueType, cat, ContainerType>;

            public:
                inline static constexpr const usize size(CLRefType<ContainerType> array) noexcept
                {
                    return len;
                }

                inline static constexpr LRefType<PointerType> get(LRefType<ContainerType> array, const usize i)
                {
                    return array.at(i);
                }

                inline static constexpr CLRefType<PointerType> get(CLRefType<ContainerType> array, const usize i)
                {
                    return array.at(i);
                }

                inline static constexpr LRefType<PointerType> get_unchecked(LRefType<ContainerType> array, const usize i)
                {
                    return array[i];
                }

                inline static constexpr CLRefType<PointerType> get_unchecked(CLRefType<ContainerType> array, const usize i)
                {
                    return array[i];
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

                inline static constexpr RetType<UncheckedIterType> begin_unchecked(LRefType<ContainerType> array) noexcept
                {
                    return UncheckedIterType{ array.begin() };
                }

                inline static constexpr RetType<ConstUncheckedIterType> cbegin_unchecked(CLRefType<ContainerType> array) noexcept
                {
                    return ConstUncheckedIterType{ array.begin() };
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

                inline static constexpr RetType<UncheckedIterType> end_unchecked(LRefType<ContainerType> array) noexcept
                {
                    return UncheckedIterType{ array.end() };
                }

                inline static constexpr RetType<ConstUncheckedIterType> cend_unchecked(CLRefType<ContainerType> array) noexcept
                {
                    return ConstUncheckedIterType{ array.end() };
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

                inline static constexpr RetType<UncheckedReverseIterType> rbegin_unchecked(LRefType<ContainerType> array) noexcept
                {
                    return UncheckedReverseIterType{ array.rbegin() };
                }

                inline static constexpr RetType<ConstUncheckedReverseIterType> crbegin_unchecked(CLRefType<ContainerType> array) noexcept
                {
                    return ConstUncheckedReverseIterType{ array.rbegin() };
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

                inline static constexpr RetType<UncheckedReverseIterType> rend_unchecked(LRefType<ContainerType> array) noexcept
                {
                    return UncheckedReverseIterType{ array.rend() };
                }

                inline static constexpr RetType<ConstUncheckedReverseIterType> crend_unchecked(CLRefType<ContainerType> array) noexcept
                {
                    return ConstUncheckedReverseIterType{ array.rend() };
                }
            };

            template<
                typename T,
                PointerCategory cat,
                typename Alloc,
                template<typename, typename> class C
            >
            struct PointerArrayAccessPolicy<T, C<pointer::Ptr<OriginType<T>, cat>, Alloc>>
            {
            public:
                using ValueType = OriginType<T>;
                using PointerType = pointer::Ptr<ValueType, cat>;
                using ContainerType = C<PointerType, Alloc>;
                using IterType = PointerArrayIterator<ValueType, cat, ContainerType>;
                using ConstIterType = PointerArrayConstIterator<ValueType, cat, ContainerType>;
                using ReverseIterType = PointerArrayReverseIterator<ValueType, cat, ContainerType>;
                using ConstReverseIterType = PointerArrayConstReverseIterator<ValueType, cat, ContainerType>;
                using UncheckedIterType = PointerArrayUncheckedIterator<ValueType, cat, ContainerType>;
                using ConstUncheckedIterType = PointerArrayConstUncheckedIterator<ValueType, cat, ContainerType>;
                using UncheckedReverseIterType = PointerArrayUncheckedReverseIterator<ValueType, cat, ContainerType>;
                using ConstUncheckedReverseIterType = PointerArrayConstUncheckedReverseIterator<ValueType, cat, ContainerType>;

            public:
                inline static constexpr const usize size(CLRefType<ContainerType> array) noexcept
                {
                    return array.size();
                }

                inline static constexpr LRefType<PointerType> get(LRefType<ContainerType> array, const usize i)
                {
                    return array.at(i);
                }

                inline static constexpr CLRefType<PointerType> get(CLRefType<ContainerType> array, const usize i)
                {
                    return array.at(i);
                }

                inline static constexpr LRefType<PointerType> get_unchecked(LRefType<ContainerType> array, const usize i)
                {
                    return array[i];
                }

                inline static constexpr CLRefType<PointerType> get_unchecked(CLRefType<ContainerType> array, const usize i)
                {
                    return array[i];
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

                inline static constexpr RetType<UncheckedIterType> begin_unchecked(LRefType<ContainerType> array) noexcept
                {
                    return UncheckedIterType{ array.begin() };
                }

                inline static constexpr RetType<ConstUncheckedIterType> cbegin_unchecked(CLRefType<ContainerType> array) noexcept
                {
                    return ConstUncheckedIterType{ array.cbegin() };
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

                inline static constexpr RetType<UncheckedIterType> end_unchecked(LRefType<ContainerType> array) noexcept
                {
                    return UncheckedIterType{ array.end() };
                }

                inline static constexpr RetType<ConstUncheckedIterType> cend_unchecked(CLRefType<ContainerType> array) noexcept
                {
                    return ConstUncheckedIterType{ array.cend() };
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

                inline static constexpr RetType<UncheckedReverseIterType> rbegin_unchecked(LRefType<ContainerType> array) noexcept
                {
                    return UncheckedReverseIterType{ array.rbegin() };
                }

                inline static constexpr RetType<ConstUncheckedReverseIterType> crbegin_unchecked(CLRefType<ContainerType> array) noexcept
                {
                    return ConstUncheckedReverseIterType{ array.crbegin() };
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

                inline static constexpr RetType<UncheckedReverseIterType> rend_unchecked(LRefType<ContainerType> array) noexcept
                {
                    return UncheckedReverseIterType{ array.rend() };
                }

                inline static constexpr RetType<ConstUncheckedReverseIterType> crend_unchecked(CLRefType<ContainerType> array) noexcept
                {
                    return ConstUncheckedReverseIterType{ array.crend() };
                }
            };

            template<typename T, typename C, typename Self>
            class PointerArrayImpl
            {
                OSPF_CRTP_IMPL

            public:
                using ValueType = OriginType<T>;
                using ContainerType = OriginType<C>;
                using AccessPolicyType = PointerArrayAccessPolicy<ValueType, ContainerType>;
                using PointerType = typename AccessPolicyType::PointerType;
                using IterType = typename AccessPolicyType::IterType;
                using ConstIterType = typename AccessPolicyType::ConstIterType;
                using ReverseIterType = typename AccessPolicyType::ReverseIterType;
                using ConstReverseIterType = typename AccessPolicyType::ConstReverseIterType;
                using UncheckedIterType = typename AccessPolicyType::UncheckedIterType;
                using ConstUncheckedIterType = typename AccessPolicyType::ConstUncheckedIterType;
                using UncheckedReverseIterType = typename AccessPolicyType::UncheckedReverseIterType;
                using ConstUncheckedReverseIterType = typename AccessPolicyType::ConstUncheckedReverseIterType;

            protected:
                constexpr PointerArrayImpl(void) = default;
            public:
                constexpr PointerArrayImpl(const PointerArrayImpl& ano) = default;
                constexpr PointerArrayImpl(PointerArrayImpl&& ano) noexcept = default;
                constexpr PointerArrayImpl& operator=(const PointerArrayImpl& rhs) = default;
                constexpr PointerArrayImpl& operator=(PointerArrayImpl&& rhs) noexcept = default;
                constexpr ~PointerArrayImpl(void) noexcept = default;

            public:
                inline constexpr const bool null(const usize i) const
                {
                    return at(i) == nullptr;
                }

            public:
                inline constexpr LRefType<PointerType> at(const usize i)
                {
                    return AccessPolicyType::get(container(), i);
                }

                inline constexpr CLRefType<PointerType> at(const usize i) const
                {
                    return AccessPolicyType::get(const_container(), i);
                }

                inline constexpr LRefType<PointerType> operator[](const usize i)
                {
                    return AccessPolicyType::get_unchecked(container(), i);
                }

                inline constexpr CLRefType<PointerType> operator[](const usize i) const
                {
                    return AccessPolicyType::get_unchecked(const_container(), i);
                }

            public:
                inline constexpr LRefType<PointerType> front(void)
                {
                    return at(0_uz);
                }

                inline constexpr CLRefType<PointerType> front(void) const
                {
                    return at(0_uz);
                }

                inline constexpr LRefType<PointerType> back(void)
                {
                    return at(size() - 1_uz);
                }

                inline constexpr CLRefType<PointerType> back(void) const
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

                inline constexpr RetType<UncheckedIterType> begin_unchecked(void) noexcept
                {
                    return AccessPolicyType::begin_unchecked(container());
                }

                inline constexpr RetType<ConstUncheckedIterType> begin_unchecked(void) const noexcept
                {
                    return AccessPolicyType::cbegin_unchecked(const_container());
                }

                inline constexpr RetType<ConstUncheckedIterType> cbegin_unchecked(void) const noexcept
                {
                    return AccessPolicyType::cbegin_unchecked(const_container());
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

                inline constexpr RetType<UncheckedIterType> end_unchecked(void) noexcept
                {
                    return AccessPolicyType::end_unchecked(container());
                }

                inline constexpr RetType<ConstUncheckedIterType> end_unchecked(void) const noexcept
                {
                    return AccessPolicyType::cend_unchecked(const_container());
                }

                inline constexpr RetType<ConstUncheckedIterType> cend_unchecked(void) const noexcept
                {
                    return AccessPolicyType::cend_unchecked(const_container());
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

                inline constexpr RetType<UncheckedReverseIterType> rbegin_unchecked(void) noexcept
                {
                    return AccessPolicyType::rbegin_unchecked(container());
                }

                inline constexpr RetType<ConstUncheckedReverseIterType> rbegin_unchecked(void) const noexcept
                {
                    return AccessPolicyType::crbegin_unchecked(const_container());
                }

                inline constexpr RetType<ConstUncheckedReverseIterType> crbegin_unchecked(void) const noexcept
                {
                    return AccessPolicyType::crbegin_unchecked(const_container());
                }

            public:
                inline constexpr RetType<ReverseIterType> rend(void) noexcept
                {
                    return AccessPolicyType::rend(container());
                }

                inline constexpr RetType<ConstReverseIterType> rend(void) const noexcept
                {
                    return AccessPolicyType::crend(const_container());
                }

                inline constexpr RetType<ConstReverseIterType> crend(void) const noexcept
                {
                    return AccessPolicyType::crend(const_container());
                }

                inline constexpr RetType<UncheckedReverseIterType> rend_unchecked(void) noexcept
                {
                    return AccessPolicyType::rend_unchecked(container());
                }

                inline constexpr RetType<ConstUncheckedReverseIterType> rend_unchecked(void) const noexcept
                {
                    return AccessPolicyType::crend_unchecked(const_container());
                }

                inline constexpr RetType<ConstUncheckedReverseIterType> crend_unchecked(void) const noexcept
                {
                    return AccessPolicyType::crend_unchecked(const_container());
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
                inline void swap(PointerArrayImpl& ano) noexcept
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

                template<typename T, typename C, typename Self>
                class PointerArrayUncheckedAccessorImpl
                {
                    using Impl = PointerArrayImpl<T, C, Self>;

                public:
                    using ValueType = typename Impl::ValueType;
                    using ContainerType = typename Impl::ContainerType;
                    using PointerType = typename Impl::PointerType;
                    using IterType = typename Impl::IterType;
                    using ConstIterType = typename Impl::ConstIterType;
                    using ReverseIterType = typename Impl::ReverseIterType;
                    using ConstReverseIterType = typename Impl::ConstReverseIterType;
                    using UncheckedIterType = typename Impl::UncheckedIterType;
                    using ConstUncheckedIterType = typename Impl::ConstUncheckedIterType;
                    using UncheckedReverseIterType = typename Impl::UncheckedReverseIterType;
                    using ConstUncheckedReverseIterType = typename Impl::ConstUncheckedReverseIterType;

                public:
                    constexpr PointerArrayUncheckedAccessorImpl(const Impl& impl)
                        : _impl(impl) {}
                    constexpr PointerArrayUncheckedAccessorImpl(const PointerArrayUncheckedAccessorImpl& ano) = default;
                    constexpr PointerArrayUncheckedAccessorImpl(PointerArrayUncheckedAccessorImpl&& ano) noexcept = default;
                    constexpr PointerArrayUncheckedAccessorImpl& operator=(const PointerArrayUncheckedAccessorImpl& rhs) = default;
                    constexpr PointerArrayUncheckedAccessorImpl& operator=(PointerArrayUncheckedAccessorImpl&& rhs) noexcept = default;
                    constexpr ~PointerArrayUncheckedAccessorImpl(void) noexcept = default;

                public:
                    inline constexpr const bool null(const usize i) const
                    {
                        return _impl.null(i);
                    }

                public:
                    inline constexpr LRefType<PointerType> at(const usize i)
                    {
                        return *_impl.at(i);
                    }

                    inline constexpr CLRefType<PointerType> at(const usize i) const
                    {
                        return *_impl.at(i);
                    }

                    inline constexpr LRefType<PointerType> operator[](const usize i)
                    {
                        return *_impl[i];
                    }

                    inline constexpr CLRefType<PointerType> operator[](const usize i)
                    {
                        return *_impl[i];
                    }

                public:
                    inline constexpr LRefType<PointerType> front(void)
                    {
                        return *_impl.front();
                    }

                    inline constexpr CLRefType<PointerType> front(void) const
                    {
                        return *_impl.front();
                    }

                    inline constexpr LRefType<PointerType> back(void)
                    {
                        return *_impl.back();
                    }

                    inline constexpr CLRefType<PointerType> back(void) const
                    {
                        return *_impl.back();
                    }

                public:
                    inline constexpr RetType<UncheckedIterType> begin(void) noexcept
                    {
                        return _impl.begin_unchecked();
                    }

                    inline constexpr RetType<ConstUncheckedIterType> begin(void) const noexcept
                    {
                        return _impl.begin_unchecked();
                    }

                    inline constexpr RetType<ConstUncheckedIterType> cbegin(void) const noexcept
                    {
                        return _impl.cbegin_unchecked();
                    }

                public:
                    inline constexpr RetType<UncheckedIterType> end(void) noexcept
                    {
                        return _impl.end_unchecked();
                    }

                    inline constexpr RetType<ConstUncheckedIterType> end(void) const noexcept
                    {
                        return _impl.end_unchecked();
                    }

                    inline constexpr RetType<ConstUncheckedIterType> cend(void) const noexcept
                    {
                        return _impl.cend_unchecked();
                    }

                public:
                    inline constexpr RetType<UncheckedReverseIterType> rbegin(void) noexcept
                    {
                        return _impl.rbegin_unchecked();
                    }

                    inline constexpr RetType<ConstUncheckedReverseIterType> rbegin(void) const noexcept
                    {
                        return _impl.rbegin_unchecked();
                    }

                    inline constexpr RetType<ConstUncheckedReverseIterType> crbegin(void) const noexcept
                    {
                        return _impl.crbegin_unchecked();
                    }

                public:
                    inline constexpr RetType<UncheckedReverseIterType> rend(void) noexcept
                    {
                        return _impl.rend_unchecked();
                    }

                    inline constexpr RetType<ConstUncheckedReverseIterType> rend(void) const noexcept
                    {
                        return _impl.rend_unchecked();
                    }

                    inline constexpr RetType<ConstUncheckedReverseIterType> crend(void) const noexcept
                    {
                        return _impl.crend_unchecked();
                    }

                public:
                    inline constexpr const bool empty(void) const noexcept
                    {
                        return _impl.empty();
                    }

                    inline constexpr const usize size(void) const noexcept
                    {
                        return _impl.size();
                    }

                private:
                    Ref<Impl> _impl;
                };

                template<
                    typename T, 
                    usize len,
                    PointerCategory cat,
                    template<typename, usize> class C
                >
                class StaticPointerArray
                    : public PointerArrayImpl<T, C<pointer::Ptr<OriginType<T>, cat>, len>, StaticPointerArray<T, len, cat, C>>
                {
                    using Impl = PointerArrayImpl<T, C<pointer::Ptr<OriginType<T>, cat>, len>, StaticPointerArray<T, len, cat, C>>;
                    using UncheckedAccessorImpl = PointerArrayUncheckedAccessorImpl<T, C<pointer::Ptr<OriginType<T>, cat>, len>, StaticPointerArray<T, len, cat, C>>;

                    template<
                        typename T,
                        PointerCategory cat,
                        template<typename> class C
                    >
                    friend class DynamicPointerArray;

                public:
                    using typename Impl::ValueType;
                    using typename Impl::PointerType;
                    using typename Impl::ContainerType;
                    using typename Impl::IterType;
                    using typename Impl::ConstIterType;
                    using typename Impl::ReverseIterType;
                    using typename Impl::ConstReverseIterType;
                    using typename Impl::UncheckedIterType;
                    using typename Impl::ConstUncheckedIterType;
                    using typename Impl::UncheckedReverseIterType;
                    using typename Impl::ConstUncheckedReverseIterType;

                public:
                    constexpr StaticPointerArray(void) = default;

                    constexpr StaticPointerArray(ArgRRefType<ContainerType> container)
                        : _container(move<ContainerType>(container)) {}

                    constexpr StaticPointerArray(std::initializer_list<PtrType<ValueType>> ptrs)
                    {
                        for (auto i{ 0_uz }, j{ (std::min)(len, ptrs.size) }; i != j; ++i)
                        {
                            _container[i] = PointerType{ move<ValueType>(ptrs[i]) };
                        }
                    }

                    constexpr StaticPointerArray(std::initializer_list<CPtrType<ValueType>> ptrs)
                    {
                        for (auto i{ 0_uz }, j{ (std::min)(len, ptrs.size) }; i != j; ++i)
                        {
                            _container[i] = PointerType{ move<ValueType>(ptrs[i]) };
                        }
                    }

                    constexpr StaticPointerArray(std::initializer_list<PointerType> ptrs)
                        : _container(std::move(ptrs)) {}

                public:
                    constexpr StaticPointerArray(const StaticPointerArray& ano) = default;
                    constexpr StaticPointerArray(StaticPointerArray&& ano) noexcept = default;
                    constexpr StaticPointerArray& operator=(const StaticPointerArray& rhs) = default;
                    constexpr StaticPointerArray& operator=(StaticPointerArray&& rhs) noexcept = default;
                    constexpr ~StaticPointerArray(void) = default;

                public:
                    inline constexpr const UncheckedAccessorImpl unchecked(void) const noexcept
                    {
                        return UncheckedAccessorImpl{ dynamic_cast<const Impl&>(*this) };
                    }

                    template<typename = void>
                        requires requires (ContainerType& container) { { container.data() } -> DecaySameAs<PtrType<PointerType>>; }
                    inline constexpr const PtrType<PointerType> data(void) noexcept
                    {
                        return _container.data();
                    }

                    template<typename = void>
                        requires requires (const ContainerType& container) { { container.data() } -> DecaySameAs<CPtrType<PointerType>>; }
                    inline constexpr const CPtrType<PointerType> data(void) const noexcept
                    {
                        return _container.data();
                    }

                public:
                    inline constexpr const usize max_size(void) const noexcept
                    {
                        return _container.max_size();
                    }

                public:
                    inline constexpr void clear(void) noexcept
                    {
                        fill(nullptr);
                    }

                    inline constexpr void fill(const std::nullptr_t _ = nullptr) noexcept
                    {
                        if constexpr (std::copyable<PointerType>)
                        {
                            _container.fill(PointerType{ nullptr });
                        }
                        else
                        {
                            for (auto i{ 0_uz }; i != _container.size(); ++i)
                            {
                                _container[i] = PointerType{ nullptr };
                            }
                        }
                    }

                    template<typename = void>
                        requires std::copyable<PointerType>
                    inline constexpr void fill(const PtrType<ValueType> ptr) noexcept
                    {
                        _container.fill(PointerType{ ptr });
                    }

                    template<typename = void>
                        requires std::copyable<PointerType>
                    inline constexpr void fill(const CPtrType<ValueType> ptr) noexcept
                    {
                        _container.fill(PointerType{ ptr });
                    }

                    template<typename = void>
                        requires std::copyable<PointerType>
                    inline constexpr void fill(ArgCLRefType<PointerType> ptr) noexcept
                    {
                        _container.fill(ptr);
                    }

                public:
                    inline constexpr const bool operator==(const StaticPointerArray& rhs) const noexcept
                    {
                        return _container == rhs._container;
                    }

                    inline constexpr const bool operator!=(const StaticPointerArray& rhs) const noexcept
                    {
                        return _container != rhs._container;
                    }

                public:
                    inline constexpr const bool operator<(const StaticPointerArray& rhs) const noexcept
                    {
                        return _container < rhs._container;
                    }

                    inline constexpr const bool operator<=(const StaticPointerArray& rhs) const noexcept
                    {
                        return _container <= rhs._container;
                    }

                    inline constexpr const bool operator>(const StaticPointerArray& rhs) const noexcept
                    {
                        return _container > rhs._container;
                    }

                    inline constexpr const bool operator>=(const StaticPointerArray& rhs) const noexcept
                    {
                        return _container >= rhs._container;
                    }

                public:
                    inline constexpr decltype(auto) operator<=>(const StaticPointerArray& rhs) const noexcept
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
                    PointerCategory cat,
                    template<typename> class C
                >
                class DynamicPointerArray
                    : public PointerArrayImpl<T, C<pointer::Ptr<OriginType<T>, cat>>, DynamicPointerArray<T, cat, C>>
                {
                    using Impl = PointerArrayImpl<T, C<pointer::Ptr<OriginType<T>, cat>>, DynamicPointerArray<T, cat, C>>;
                    using UncheckedAccessorImpl = PointerArrayUncheckedAccessorImpl<T, C<pointer::Ptr<OriginType<T>, cat>>, DynamicPointerArray<T, cat, C>>;

                public:
                    using typename Impl::ValueType;
                    using typename Impl::PointerType;
                    using typename Impl::ContainerType;
                    using typename Impl::IterType;
                    using typename Impl::ConstIterType;
                    using typename Impl::ReverseIterType;
                    using typename Impl::ConstReverseIterType;
                    using typename Impl::UncheckedIterType;
                    using typename Impl::ConstUncheckedIterType;
                    using typename Impl::UncheckedReverseIterType;
                    using typename Impl::ConstUncheckedReverseIterType;

                public:
                    constexpr DynamicPointerArray(void) = default;

                    template<typename = void>
                        requires std::default_initializable<PointerType>
                    constexpr explicit DynamicPointerArray(const usize length)
                        : _container(length) {}

                    template<typename = void>
                        requires (!std::default_initializable<PointerType> && WithDefault<PointerType> && std::copyable<PointerType>)
                    constexpr explicit DynamicPointerArray(const usize length)
                        : _container(length, DefaultValue<PointerType>::value()) {}

                    template<typename = void>
                        requires std::copyable<PointerType>
                    constexpr DynamicPointerArray(const usize length, const PtrType<ValueType> ptr)
                        : _container(length, PointerType{ ptr }) {}

                    template<typename = void>
                        requires std::copyable<PointerType>
                    constexpr DynamicPointerArray(const usize length, const CPtrType<ValueType> ptr)
                        : _container(length, PointerType{ ptr }) {}

                    template<typename = void>
                        requires std::copyable<PointerType>
                    constexpr DynamicPointerArray(const usize length, ArgCLRefType<PointerType> ptr)
                        : _container(length, ptr) {}

                    template<std::input_iterator It>
                        requires requires (const It it) { { *it } -> DecaySameAs<PtrType<ValueType>>; }
                    constexpr DynamicPointerArray(It&& first, It&& last)
                        : _container(
                            boost::make_transform_iterator(std::forward<It>(first), [](const PtrType<ValueType> ptr) { return PointerType{ ptr }; }),
                            boost::make_transform_iterator(std::forward<It>(last), [](const PtrType<ValueType> ptr) { return PointerType{ ptr }; })
                        ) {}

                    template<std::input_iterator It>
                        requires requires (const It it) { { *it } -> DecaySameAs<CPtrType<ValueType>>; }
                    constexpr DynamicPointerArray(It&& first, It&& last)
                        : _container(
                            boost::make_transform_iterator(std::forward<It>(first), [](const CPtrType<ValueType> ptr) { return PointerType{ ptr }; }),
                            boost::make_transform_iterator(std::forward<It>(last), [](const CPtrType<ValueType> ptr) { return PointerType{ ptr }; })
                        ) {}

                    template<std::input_iterator It>
                        requires requires (const It it) { { *it } -> DecaySameAs<PointerType>; }
                    constexpr DynamicPointerArray(It&& first, It&& last)
                        : _container(std::forward<It>(first), std::forward<It>(last)) {}

                    constexpr DynamicPointerArray(const std::initializer_list<PtrType<ValueType>> ptrs)
                        : _container(ptrs.size())
                    {
                        for (auto i{ 0_uz }; i != ptrs.size(); ++i)
                        {
                            _container[i] = PointerType{ ptrs[i] };
                        }
                    }

                    constexpr DynamicPointerArray(const std::initializer_list<CPtrType<ValueType>> ptrs)
                        : _container(ptrs.size())
                    {
                        for (auto i{ 0_uz }; i != ptrs.size(); ++i)
                        {
                            _container[i] = PointerType{ ptrs[i] };
                        }
                    }

                    constexpr DynamicPointerArray(std::initializer_list<PointerType> ptrs)
                        : _container(std::move(ptrs)) {}

                public:
                    constexpr DynamicPointerArray(const DynamicPointerArray& ano) = default;
                    constexpr DynamicPointerArray(DynamicPointerArray&& ano) noexcept = default;
                    constexpr DynamicPointerArray& operator=(const DynamicPointerArray& rhs) = default;
                    constexpr DynamicPointerArray& operator=(DynamicPointerArray&& rhs) noexcept = default;
                    constexpr ~DynamicPointerArray(void) = default;

                public:
                    inline constexpr void assign(const usize length, const std::nullptr_t _ = nullptr)
                    {
                        if constexpr (std::copyable<PointerType>)
                        {
                            _container.assign(length, PointerType{ nullptr });
                        }
                        else
                        {
                            _container.clear();
                            for (auto i{ 0_uz }; i != length; ++i)
                            {
                                _container.push_back(PointerType{ nullptr });
                            }
                        }
                    }

                    template<typename = void>
                        requires std::copyable<PointerType>
                    inline constexpr void assign(const usize length, const PtrType<ValueType> ptr)
                    {
                        _container.assign(length, PointerType{ ptr });
                    }

                    template<typename = void>
                        requires std::copyable<PointerType>
                    inline constexpr void assign(const usize length, const CPtrType<ValueType> ptr)
                    {
                        _container.assign(length, PointerType{ ptr });
                    }

                    template<typename = void>
                        requires std::copyable<PointerType>
                    inline constexpr void assign(const usize length, ArgCLRefType<PointerType> ptr)
                    {
                        _container.assign(length, ptr);
                    }

                    template<std::input_iterator It>
                        requires requires (const It it) { { *it } -> DecaySameAs<PtrType<ValueType>>; }
                    inline constexpr void assign(It&& first, It&& last)
                    {
                        _container.assign(
                            boost::make_transform_iterator(std::forward<It>(first), [](const PtrType<ValueType> ptr) { return PointerType{ ptr }; }),
                            boost::make_transform_iterator(std::forward<It>(last), [](const PtrType<ValueType> ptr) { return PointerType{ ptr }; })
                        );
                    }

                    template<std::input_iterator It>
                        requires requires (const It it) { { *it } -> DecaySameAs<CPtrType<ValueType>>; }
                    inline constexpr void assign(It&& first, It&& last)
                    {
                        _container.assign(
                            boost::make_transform_iterator(std::forward<It>(first), [](const CPtrType<ValueType> ptr) { return PointerType{ ptr }; }),
                            boost::make_transform_iterator(std::forward<It>(last), [](const CPtrType<ValueType> ptr) { return PointerType{ ptr }; })
                        );
                    }

                    template<std::input_iterator It>
                        requires requires (const It it) { { *it } -> DecaySameAs<PointerType>; }
                    inline constexpr void assign(It&& first, It&& last)
                    {
                        _container.assign(std::forward<It>(first), std::forward<It>(last));
                    }

                    inline constexpr void assign(const std::initializer_list<PtrType<ValueType>> ptrs)
                    {
                        _container.assign(ptrs.size());
                        for (auto i{ 0_uz }; i != ptrs.size(); ++i)
                        {
                            _container[i] = PointerType{ ptrs[i] };
                        }
                    }

                    inline constexpr void assign(const std::initializer_list<CPtrType<ValueType>> ptrs)
                    {
                        _container.assign(ptrs.size());
                        for (auto i{ 0_uz }; i != ptrs.size(); ++i)
                        {
                            _container[i] = PointerType{ ptrs[i] };
                        }
                    }

                    inline constexpr void assign(std::initializer_list<PointerType> ptrs)
                    {
                        _container.assign(std::move(ptrs));
                    }

                    template<
                        usize len,
                        template<typename, usize> class C1
                    >
                        requires std::copyable<PointerType>
                    inline constexpr void assign(const StaticPointerArray<ValueType, len, cat, C1>& ptrs)
                    {
                        assign(ptrs._container.begin(), ptrs._container.end());
                    }

                    template<
                        usize len,
                        template<typename, usize> class C1
                    >
                    inline constexpr void assign(StaticPointerArray<ValueType, len, cat, C1>&& ptrs)
                    {
                        _container.clear();
                        std::move(ptrs._container.begin(), ptrs._container.end(), std::back_inserter(_container));
                    }

                    template<
                        typename U,
                        usize len,
                        template<typename, usize> class C1
                    >
                        requires std::copyable<PointerType> && std::convertible_to<PtrType<U>, PointerType>
                    inline constexpr void assign(const StaticPointerArray<U, len, cat, C1>& ptrs)
                    {
                        assign(
                            boost::make_transform_iterator(ptrs._container.begin(), [](const Ptr<U, cat>& ptr) { return PointerType{ ptr }; }),
                            boost::make_transform_iterator(ptrs._container.end(), [](const Ptr<U, cat>& ptr) { return PointerType{ ptr }; })
                        );
                    }

                    template<
                        typename U,
                        usize len,
                        template<typename, usize> class C1
                    >
                        requires std::convertible_to<PtrType<U>, PointerType>
                    inline constexpr void assign(StaticPointerArray<U, len, cat, C1>&& ptrs)
                    {
                        assign(
                            boost::make_transform_iterator(ptrs._container.begin(), [](Ptr<U, cat>& ptr) { return PointerType{ std::move(ptr) }; }),
                            boost::make_transform_iterator(ptrs._container.end(), [](Ptr<U, cat>& ptr) { return PointerType{ std::move(ptr) }; })
                        );
                    }

                    template<template<typename> class C1>
                        requires std::copyable<PointerType>
                    inline constexpr void assign(const DynamicPointerArray<ValueType, cat, C1>& ptrs)
                    {
                        assign(ptrs._container.begin(), ptrs._container.end());
                    }

                    template<template<typename> class C1>
                    inline constexpr void assign(DynamicPointerArray<ValueType, cat, C1>&& ptrs)
                    {
                        _container.clear();
                        std::move(ptrs._container.begin(), ptrs._container.end(), std::back_inserter(_container));
                    }

                    template<
                        typename U,
                        template<typename> class C1
                    >
                        requires std::copyable<PointerType> && std::convertible_to<PtrType<U>, PointerType>
                    inline constexpr void assign(const DynamicPointerArray<U, cat, C1>& ptrs)
                    {
                        assign(
                            boost::make_transform_iterator(ptrs._container.begin(), [](const Ptr<U, cat>& ptr) { return PointerType{ ptr }; }),
                            boost::make_transform_iterator(ptrs._container.end(), [](const Ptr<U, cat>& ptr) { return PointerType{ ptr }; })
                        );
                    }

                    template<
                        typename U,
                        template<typename> class C1
                    >
                        requires std::convertible_to<PtrType<U>, PointerType>
                    inline constexpr void assign(DynamicPointerArray<U, cat, C1>&& ptrs)
                    {
                        assign(
                            boost::make_transform_iterator(ptrs._container.begin(), [](Ptr<U, cat>& ptr) { return PointerType{ std::move(ptr) }; }),
                            boost::make_transform_iterator(ptrs._container.end(), [](Ptr<U, cat>& ptr) { return PointerType{ std::move(ptr) }; })
                        );
                    }

                public:
                    inline constexpr RetType<UncheckedAccessorImpl> unchecked(void) const noexcept
                    {
                        return UncheckedAccessorImpl{ dynamic_cast<const Impl&>(*this) };
                    }

                    template<typename = void>
                        requires requires (ContainerType& container) { { container.data() } -> DecaySameAs<PtrType<PointerType>>; }
                    inline constexpr const PtrType<PointerType> data(void) noexcept
                    {
                        return _container.data();
                    }

                    template<typename = void>
                        requires requires (const ContainerType& container) { { container.data() } -> DecaySameAs<CPtrType<PointerType>>; }
                    inline constexpr const CPtrType<PointerType> data(void) const noexcept
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

                    inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, const std::nullptr_t _)
                    {
                        return IterType{ _container.insert(pos._iter, PointerType{ nullptr }) };
                    }

                    inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, const PtrType<ValueType> ptr)
                    {
                        return IterType{ _container.insert(pos._iter, PointerType{ ptr }) };
                    }

                    inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, const CPtrType<ValueType> ptr)
                    {
                        return IterType{ _container.insert(pos._iter, PointerType{ ptr }) };
                    }

                    inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, ArgRRefType<PointerType> ptr)
                    {
                        return IterType{ _container.insert(pos._iter, move<PointerType>(ptr)) };
                    }

                    inline constexpr RetType<UncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, const std::nullptr_t _)
                    {
                        return UncheckedIterType{ _container.insert(pos._iter, PointerType{ nullptr }) };
                    }

                    inline constexpr RetType<UncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, const PtrType<ValueType> ptr)
                    {
                        return UncheckedIterType{ _container.insert(pos._iter, PointerType{ ptr }) };
                    }

                    inline constexpr RetType<UncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, const CPtrType<ValueType> ptr)
                    {
                        return UncheckedIterType{ _container.insert(pos._iter, PointerType{ ptr }) };
                    }

                    inline constexpr RetType<UncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, ArgRRefType<PointerType> ptr)
                    {
                        return UncheckedIterType{ _container.insert(pos._iter, move<PointerType>(ptr)) };
                    }

                    template<std::input_iterator It>
                        requires requires (const It it) { { *it } -> DecaySameAs<PtrType<ValueType>>; }
                    inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, It&& first, It&& last)
                    {
                        return IterType{ _container.insert(pos._iter,
                            boost::make_transform_iterator(std::forward<It>(first), [](const PtrType<ValueType> ptr) { return PointerType{ ptr }; }),
                            boost::make_transform_iterator(std::forward<It>(last), [](const PtrType<ValueType> ptr) { return PointerType{ ptr }; })
                        ) };
                    }

                    template<std::input_iterator It>
                        requires requires (const It it) { { *it } -> DecaySameAs<CPtrType<ValueType>>; }
                    inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, It&& first, It&& last)
                    {
                        return IterType{ _container.insert(pos._iter,
                            boost::make_transform_iterator(std::forward<It>(first), [](const CPtrType<ValueType> ptr) { return PointerType{ ptr }; }),
                            boost::make_transform_iterator(std::forward<It>(last), [](const CPtrType<ValueType> ptr) { return PointerType{ ptr }; })
                        ) };
                    }

                    template<std::input_iterator It>
                        requires requires (const It it) { { *it } -> DecaySameAs<PointerType>; }
                    inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, It&& first, It&& last)
                    {
                        return IterType{ _container.insert(pos, std::forward<It>(first), std::forward<It>(last)) };
                    }

                    template<std::input_iterator It>
                        requires requires (const It it) { { *it } -> DecaySameAs<PtrType<ValueType>>; }
                    inline constexpr RetType<ConstUncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, It&& first, It&& last)
                    {
                        return ConstUncheckedIterType{ _container.insert(pos._iter,
                            boost::make_transform_iterator(std::forward<It>(first), [](const PtrType<ValueType> ptr) { return PointerType{ ptr }; }),
                            boost::make_transform_iterator(std::forward<It>(last), [](const PtrType<ValueType> ptr) { return PointerType{ ptr }; })
                        ) };
                    }

                    template<std::input_iterator It>
                        requires requires (const It it) { { *it } -> DecaySameAs<CPtrType<ValueType>>; }
                    inline constexpr RetType<ConstUncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, It&& first, It&& last)
                    {
                        return ConstUncheckedIterType{ _container.insert(pos._iter,
                            boost::make_transform_iterator(std::forward<It>(first), [](const CPtrType<ValueType> ptr) { return PointerType{ ptr }; }),
                            boost::make_transform_iterator(std::forward<It>(last), [](const CPtrType<ValueType> ptr) { return PointerType{ ptr }; })
                        ) };
                    }

                    template<std::input_iterator It>
                        requires requires (const It it) { { *it } -> DecaySameAs<PointerType>; }
                    inline constexpr RetType<ConstUncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, It&& first, It&& last)
                    {
                        return ConstUncheckedIterType{ _container.insert(pos, std::forward<It>(first), std::forward<It>(last)) };
                    }

                    inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, const std::initializer_list<PtrType<ValueType>> ptrs)
                    {
                        auto it = pos._iter;
                        for (auto i{ 0_uz }; i != ptrs.size(); ++i)
                        {
                            it = _container.insert(it, PointerType{ ptrs[i] });
                        }
                        return IterType{ it };
                    }

                    inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, const std::initializer_list<CPtrType<ValueType>> ptrs)
                    {
                        auto it = pos._iter;
                        for (auto i{ 0_uz }; i != ptrs.size(); ++i)
                        {
                            it = _container.insert(it, PointerType{ ptrs[i] });
                        }
                        return IterType{ it };
                    }

                    inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, std::initializer_list<PointerType> ptrs)
                    {
                        return IterType{ _container.insert(pos, std::move(ptrs)) };
                    }

                    inline constexpr RetType<UncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, const std::initializer_list<PtrType<ValueType>> ptrs)
                    {
                        auto it = pos._iter;
                        for (auto i{ 0_uz }; i != ptrs.size(); ++i)
                        {
                            it = _container.insert(it, PointerType{ ptrs[i] });
                        }
                        return UncheckedIterType{ it };
                    }

                    inline constexpr RetType<UncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, const std::initializer_list<CPtrType<ValueType>> ptrs)
                    {
                        auto it = pos._iter;
                        for (auto i{ 0_uz }; i != ptrs.size(); ++i)
                        {
                            it = _container.insert(it, PointerType{ ptrs[i] });
                        }
                        return UncheckedIterType{ it };
                    }

                    inline constexpr RetType<UncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, std::initializer_list<PointerType> ptrs)
                    {
                        return UncheckedIterType{ _container.insert(pos._iter, std::move(ptrs)) };
                    }

                    template<
                        usize len,
                        template<typename, usize> class C1
                    >
                        requires std::copyable<PointerType>
                    inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, const StaticPointerArray<ValueType, len, cat, C1>& ptrs)
                    {
                        return insert(pos, ptrs._container.begin(), ptrs._container.end());
                    }

                    template<
                        usize len,
                        template<typename, usize> class C1
                    >
                    inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, StaticPointerArray<ValueType, len, cat, C1>&& ptrs)
                    {
                        return IterType{ std::move(ptrs._container.begin(), ptrs._container.end(), std::inserter(_container, pos._iter)) };
                    }

                    template<
                        typename U,
                        usize len,
                        template<typename, usize> class C1
                    >
                        requires std::copyable<PointerType> && std::convertible_to<PtrType<U>, PointerType>
                    inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, const StaticPointerArray<U, len, cat, C1>& ptrs)
                    {
                        return insert(pos,
                            boost::make_transform_iterator(ptrs._container.begin(), [](const Ptr<U, cat>& ptr) { return PointerType{ ptr }; }),
                            boost::make_transform_iterator(ptrs._container.end(), [](const Ptr<U, cat>& ptr) { return PointerType{ ptr }; })
                        );
                    }

                    template<
                        typename U,
                        usize len,
                        template<typename, usize> class C1
                    >
                        requires std::convertible_to<PtrType<U>, PointerType>
                    inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, StaticPointerArray<U, len, cat, C1>&& ptrs)
                    {
                        return insert(pos,
                            boost::make_transform_iterator(ptrs._container.begin(), [](Ptr<U, cat>& ptr) { return PointerType{ std::move(ptr) }; }),
                            boost::make_transform_iterator(ptrs._container.end(), [](Ptr<U, cat>& ptr) { return PointerType{ std::move(ptr) }; })
                        );
                    }

                    template<
                        usize len,
                        template<typename, usize> class C1
                    >
                        requires std::copyable<PointerType>
                    inline constexpr RetType<UncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, const StaticPointerArray<ValueType, len, cat, C1>& ptrs)
                    {
                        return insert(pos, ptrs._container.begin(), ptrs._container.end());
                    }

                    template<
                        usize len,
                        template<typename, usize> class C1
                    >
                    inline constexpr RetType<UncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, StaticPointerArray<ValueType, len, cat, C1>&& ptrs)
                    {
                        return UncheckedIterType{ std::move(ptrs._container.begin(), ptrs._container.end(), std::inserter(_container, pos._iter)) };
                    }
                    
                    template<
                        typename U,
                        usize len,
                        template<typename, usize> class C1
                    >
                        requires std::copyable<PointerType> && std::convertible_to<PtrType<U>, PointerType>
                    inline constexpr RetType<UncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, const StaticPointerArray<U, len, cat, C1>& ptrs)
                    {
                        return insert(pos, 
                            boost::make_transform_iterator(ptrs._container.begin(), [](const Ptr<U, cat>& ptr) { return PointerType{ ptr }; }),
                            boost::make_transform_iterator(ptrs._container.end(), [](const Ptr<U, cat>& ptr) { return PointerType{ ptr }; })
                        );
                    }

                    template<
                        typename U,
                        usize len,
                        template<typename, usize> class C1
                    >
                        requires std::convertible_to<PtrType<U>, PointerType>
                    inline constexpr RetType<UncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, StaticPointerArray<U, len, cat, C1>&& ptrs)
                    {
                        return insert(pos,
                            boost::make_transform_iterator(ptrs._container.begin(), [](Ptr<U, cat>& ptr) { return PointerType{ std::move(ptr) }; }),
                            boost::make_transform_iterator(ptrs._container.end(), [](Ptr<U, cat>& ptr) { return PointerType{ std::move(ptr) }; })
                        );
                    }

                    template<template<typename> class C1>
                        requires std::copyable<PointerType>
                    inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, const DynamicPointerArray<ValueType, cat, C1>& ptrs)
                    {
                        return insert(pos, ptrs._container.begin(), ptrs._container.end());
                    }

                    template<template<typename> class C1>
                    inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, DynamicPointerArray<ValueType, cat, C1>&& ptrs)
                    {
                        return IterType{ std::move(ptrs._container.begin(), ptrs._container.end(), std::inserter(_container, pos._iter)) };
                    }

                    template<
                        typename U,
                        template<typename> class C1
                    >
                        requires std::copyable<PointerType> && std::convertible_to<PtrType<U>, PointerType>
                    inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, const DynamicPointerArray<U, cat, C1>& ptrs)
                    {
                        return insert(pos,
                            boost::make_transform_iterator(ptrs._container.begin(), [](const Ptr<U, cat>& ptr) { return PointerType{ ptr }; }),
                            boost::make_transform_iterator(ptrs._container.end(), [](const Ptr<U, cat>& ptr) { return PointerType{ ptr }; })
                        );
                    }

                    template<
                        typename U,
                        template<typename> class C1
                    >
                        requires std::convertible_to<PtrType<U>, PointerType>
                    inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, DynamicPointerArray<U, cat, C1>&& ptrs)
                    {
                        return insert(pos,
                            boost::make_transform_iterator(ptrs._container.begin(), [](Ptr<U, cat>& ptr) { return PointerType{ std::move(ptr) }; }),
                            boost::make_transform_iterator(ptrs._container.end(), [](Ptr<U, cat>& ptr) { return PointerType{ std::move(ptr) }; })
                        );
                    }

                    template<template<typename> class C1>
                        requires std::copyable<PointerType>
                    inline constexpr RetType<UncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, const DynamicPointerArray<ValueType, cat, C1>& ptrs)
                    {
                        return insert(pos, ptrs._container.begin(), ptrs._container.end());
                    }

                    template<template<typename> class C1>
                    inline constexpr RetType<UncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, DynamicPointerArray<ValueType, cat, C1>&& ptrs)
                    {
                        return UncheckedIterType{ std::move(ptrs._container.begin(), ptrs._container.end(), std::inserter(_container, pos._iter)) };
                    }

                    template<
                        typename U,
                        template<typename> class C1
                    >
                        requires std::copyable<PointerType> && std::convertible_to<PtrType<U>, PointerType>
                    inline constexpr RetType<UncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, const DynamicPointerArray<U, cat, C1>& ptrs)
                    {
                        return insert(pos,
                            boost::make_transform_iterator(ptrs._container.begin(), [](const Ptr<U, cat>& ptr) { return PointerType{ ptr }; }),
                            boost::make_transform_iterator(ptrs._container.end(), [](const Ptr<U, cat>& ptr) { return PointerType{ ptr }; })
                        );
                    }

                    template<
                        typename U,
                        template<typename> class C1
                    >
                        requires std::convertible_to<PtrType<U>, PointerType>
                    inline constexpr RetType<UncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, DynamicPointerArray<U, cat, C1>&& ptrs)
                    {
                        return insert(pos,
                            boost::make_transform_iterator(ptrs._container.begin(), [](Ptr<U, cat>& ptr) { return PointerType{ std::move(ptr) }; }),
                            boost::make_transform_iterator(ptrs._container.end(), [](Ptr<U, cat>& ptr) { return PointerType{ std::move(ptr) }; })
                        );
                    }

                    inline constexpr RetType<IterType> emplace(ArgCLRefType<ConstIterType> pos, const std::nullptr_t _)
                    {
                        return IterType{ _container.emplace(pos._iter, nullptr) };
                    }

                    inline constexpr RetType<IterType> emplace(ArgCLRefType<ConstIterType> pos, const PtrType<ValueType> ptr)
                    {
                        return IterType{ _container.emplace(pos._iter, ptr) };
                    }

                    inline constexpr RetType<IterType> emplace(ArgCLRefType<ConstIterType> pos, const CPtrType<ValueType> ptr)
                    {
                        return IterType{ _container.emplace(pos._iter, ptr) };
                    }

                    template<typename... Args>
                        requires std::constructible_from<PointerType, Args...>
                    inline constexpr RetType<IterType> emplace(ArgCLRefType<ConstIterType> pos, Args&&... args)
                    {
                        return IterType{ _container.emplace(pos._iter, std::forward<Args>(args)...) };
                    }

                    inline constexpr RetType<UncheckedIterType> emplace(ArgCLRefType<ConstUncheckedIterType> pos, const std::nullptr_t _)
                    {
                        return UncheckedIterType{ _container.emplace(pos._iter, nullptr) };
                    }

                    inline constexpr RetType<UncheckedIterType> emplace(ArgCLRefType<ConstUncheckedIterType> pos, const PtrType<ValueType> ptr)
                    {
                        return UncheckedIterType{ _container.emplace(pos._iter, ptr) };
                    }

                    inline constexpr RetType<UncheckedIterType> emplace(ArgCLRefType<ConstUncheckedIterType> pos, const CPtrType<ValueType> ptr)
                    {
                        return UncheckedIterType{ _container.emplace(pos._iter, ptr) };
                    }

                    template<typename... Args>
                        requires std::constructible_from<PointerType, Args...>
                    inline constexpr RetType<UncheckedIterType> emplace(ArgCLRefType<ConstUncheckedIterType> pos, Args&&... args)
                    {
                        return UncheckedIterType{ _container.emplace(pos._iter, std::forward<Args>(args)...) };
                    }

                    inline constexpr RetType<IterType> erase(ArgCLRefType<ConstIterType> pos)
                    {
                        return IterType{ _container.erase(pos._iter) };
                    }

                    inline constexpr RetType<UncheckedIterType> erase(ArgCLRefType<ConstUncheckedIterType> pos)
                    {
                        return UncheckedIterType{ _container.erase(pos._iter) };
                    }

                    inline constexpr RetType<IterType> erase(ArgCLRefType<ConstIterType> first, ArgCLRefType<ConstIterType> last)
                    {
                        return IterType{ _container.erase(first._iter, last._iter) };
                    }

                    inline constexpr RetType<UncheckedIterType> erase(ArgCLRefType<ConstUncheckedIterType> first, ArgCLRefType<ConstUncheckedIterType> last)
                    {
                        return UncheckedIterType{ _container.erase(first._iter, last._iter) };
                    }

                    inline constexpr void push_back(const std::nullptr_t _)
                    {
                        _container.push_back(PointerType{ nullptr });
                    }

                    inline constexpr void push_back(const PtrType<ValueType> ptr)
                    {
                        _container.push_back(PointerType{ ptr });
                    }

                    inline constexpr void push_back(const CPtrType<ValueType> ptr)
                    {
                        _container.push_back(PointerType{ ptr });
                    }

                    inline constexpr void push_back(ArgRRefType<PointerType> ptr)
                    {
                        _container.push_back(move<PointerType>(ptr));
                    }

                    template<
                        usize len,
                        template<typename, usize> class C1
                    >
                        requires std::copyable<PointerType>
                    inline constexpr void push_back(const StaticPointerArray<ValueType, len, cat, C1>& ptrs)
                    {
                        insert(this->end(), ptrs);
                    }

                    template<
                        usize len,
                        template<typename, usize> class C1
                    >
                    inline constexpr void push_back(StaticPointerArray<ValueType, len, cat, C1>&& ptrs)
                    {
                        insert(this->end(), std::move(ptrs));
                    }

                    template<
                        typename U,
                        usize len,
                        template<typename, usize> class C1
                    >
                        requires std::copyable<PointerType> && std::convertible_to<PtrType<U>, PointerType>
                    inline constexpr void push_back(const StaticPointerArray<U, len, cat, C1>& ptrs)
                    {
                        insert(this->end(), ptrs);
                    }

                    template<
                        typename U,
                        usize len,
                        template<typename, usize> class C1
                    >
                        requires std::convertible_to<PtrType<U>, PointerType>
                    inline constexpr void push_back(StaticPointerArray<U, len, cat, C1>&& ptrs)
                    {
                        insert(this->end(), std::move(ptrs));
                    }

                    template<template<typename> class C1>
                        requires std::copyable<PointerType>
                    inline constexpr void push_back(const DynamicPointerArray<ValueType, cat, C1>& ptrs)
                    {
                        insert(this->end(), ptrs);
                    }

                    template<template<typename> class C1>
                    inline constexpr void push_back(DynamicPointerArray<ValueType, cat, C1>&& ptrs)
                    {
                        insert(this->end(), std::move(ptrs));
                    }

                    template<
                        typename U,
                        template<typename> class C1
                    >
                        requires std::copyable<PointerType> && std::convertible_to<PtrType<U>, PointerType>
                    inline constexpr void push_back(const DynamicPointerArray<U, cat, C1>& ptrs)
                    {
                        insert(this->end(), ptrs);
                    }

                    template<
                        typename U,
                        template<typename> class C1
                    >
                        requires std::convertible_to<PtrType<U>, PointerType>
                    inline constexpr void push_back(DynamicPointerArray<U, cat, C1>&& ptrs)
                    {
                        insert(this->end(), std::move(ptrs));
                    }

                    inline constexpr void emplace_back(const std::nullptr_t _)
                    {
                        _container.emplace_back(nullptr);
                    }

                    inline constexpr void emplace_back(const PtrType<ValueType> ptr)
                    {
                        _container.emplace_back(ptr);
                    }

                    inline constexpr void emplace_back(const CPtrType<ValueType> ptr)
                    {
                        _container.emplace_back(ptr);
                    }

                    template<typename... Args>
                        requires std::constructible_from<PointerType, Args...>
                    inline constexpr void emplace_back(Args&&... args)
                    {
                        _container.emplace_back(std::forward<Args>(args)...);
                    }

                    inline constexpr RetType<PointerType> pop_back(void)
                    {
                        auto back = move<PointerType>(this->back());
                        _container.pop_back();
                        return back;
                    }

                    inline constexpr void push_front(const std::nullptr_t _)
                    {
                        insert(this->begin(), nullptr);
                    }

                    template<typename = void>
                        requires requires (ContainerType& container) { container.push_front(nullptr); }
                    inline constexpr void push_front(const std::nullptr_t _)
                    {
                        _container.push_front(nullptr);
                    }
                    
                    inline constexpr void push_front(const PtrType<ValueType> ptr)
                    {
                        insert(this->begin(), PointerType{ ptr });
                    }

                    template<typename = void>
                        requires requires (ContainerType& container) { container.push_front(std::declval<PointerType>()); }
                    inline constexpr void push_front(const PtrType<ValueType> ptr)
                    {
                        _container.push_front(PointerType{ ptr });
                    }

                    inline constexpr void push_front(const CPtrType<ValueType> ptr)
                    {
                        insert(this->begin(), PointerType{ ptr });
                    }

                    template<typename = void>
                        requires requires (ContainerType& container) { container.push_front(std::declval<PointerType>()); }
                    inline constexpr void push_front(const CPtrType<ValueType> ptr)
                    {
                        _container.push_front(PointerType{ ptr });
                    }

                    inline constexpr void push_front(ArgRRefType<PointerType> ptr)
                    {
                        insert(this->begin(), move<PointerType>(ptr));
                    }

                    template<typename = void>
                        requires requires (ContainerType& container) { container.push_front(std::declval<PointerType>()); }
                    inline constexpr void push_front(ArgRRefType<PointerType> ptr)
                    {
                        _container.push_front(move<PointerType>(ptr));
                    }

                    template<
                        usize len,
                        template<typename, usize> class C1
                    >
                        requires std::copyable<PointerType>
                    inline constexpr void push_front(const StaticPointerArray<ValueType, len, cat, C1>& ptrs)
                    {
                        insert(this->begin(), ptrs);
                    }

                    template<
                        usize len,
                        template<typename, usize> class C1
                    >
                    inline constexpr void push_front(StaticPointerArray<ValueType, len, cat, C1>&& ptrs)
                    {
                        insert(this->begin(), std::move(ptrs));
                    }

                    template<
                        typename U,
                        usize len,
                        template<typename, usize> class C1
                    >
                        requires std::copyable<PointerType> && std::convertible_to<PtrType<U>, PointerType>
                    inline constexpr void push_front(const StaticPointerArray<U, len, cat, C1>& ptrs)
                    {
                        insert(this->begin(), ptrs);
                    }

                    template<
                        typename U,
                        usize len,
                        template<typename, usize> class C1
                    >
                        requires std::convertible_to<PtrType<U>, PointerType>
                    inline constexpr void push_front(StaticPointerArray<U, len, cat, C1>&& ptrs)
                    {
                        insert(this->begin(), std::move(ptrs));
                    }

                    template<template<typename> class C1>
                        requires std::copyable<PointerType>
                    inline constexpr void push_front(const DynamicPointerArray<ValueType, cat, C1>& ptrs)
                    {
                        insert(this->begin(), ptrs);
                    }

                    template<template<typename> class C1>
                    inline constexpr void push_front(DynamicPointerArray<ValueType, cat, C1>&& ptrs)
                    {
                        insert(this->begin(), std::move(ptrs));
                    }

                    template<
                        typename U,
                        template<typename> class C1
                    >
                        requires std::copyable<PointerType> && std::convertible_to<PtrType<U>, PointerType>
                    inline constexpr void push_front(const DynamicPointerArray<U, cat, C1>& ptrs)
                    {
                        insert(this->begin(), ptrs);
                    }

                    template<
                        typename U,
                        template<typename> class C1
                    >
                        requires std::convertible_to<PtrType<U>, PointerType>
                    inline constexpr void push_front(DynamicPointerArray<U, cat, C1>&& ptrs)
                    {
                        insert(this->begin(), std::move(ptrs));
                    }

                    inline constexpr void emplace_front(const std::nullptr_t _)
                    {
                        _container.emplace(_container.begin(), nullptr);
                    }

                    template<typename = void>
                        requires requires (ContainerType& container) { container.emplace_front(nullptr); }
                    inline constexpr void emplace_front(const std::nullptr_t _)
                    {
                        _container.emplace_front(nullptr);
                    }

                    inline constexpr void emplace_front(const PtrType<ValueType> ptr)
                    {
                        _container.emplace(_container.begin(), ptr);
                    }

                    template<typename = void>
                        requires requires (ContainerType& container) { container.emplace_front(std::declval<PtrType<ValueType>>()); }
                    inline constexpr void emplace_back(const PtrType<ValueType> ptr)
                    {
                        _container.emplace_front(ptr);
                    }

                    inline constexpr void emplace_front(const CPtrType<ValueType> ptr)
                    {
                        _container.emplace(_container.begin(), ptr);
                    }

                    template<typename = void>
                        requires requires (ContainerType& container) { container.emplace_front(std::declval<CPtrType<ValueType>>()); }
                    inline constexpr void emplace_front(const CPtrType<ValueType> ptr)
                    {
                        _container.emplace_front(ptr);
                    }

                    template<typename... Args>
                        requires std::constructible_from<PointerType, Args...>
                    inline constexpr void emplace_front(Args&&... args)
                    {
                        _container.emplace(_container.begin(), std::forward<Args>(args)...);
                    }

                    template<typename... Args>
                        requires std::constructible_from<PointerType, Args...>
                            && requires (ContainerType& container) { container.emplace_front(std::declval<Args>()...); }
                    inline constexpr void emplace_front(Args&&... args)
                    {
                        _container.emplace_front(std::forward<Args>(args)...);
                    }

                    inline constexpr RetType<PointerType> pop_front(void)
                    {
                        auto front = move<PointerType>(this->front());
                        _container.pop_front();
                        return front;
                    }

                    template<typename = void>
                        requires std::default_initializable<PointerType>
                    inline constexpr void resize(const usize length)
                    {
                        _container.resize(length);
                    }

                    template<typename = void>
                        requires (!std::default_initializable<PointerType> && WithDefault<PointerType> && std::copyable<PointerType>)
                    inline constexpr void resize(const usize length)
                    {
                        _container.resize(length, DefaultValue<PointerType>::value());
                    }
                    
                    template<typename = void>
                        requires std::copyable<PointerType>
                    inline constexpr void resize(const usize length, const PtrType<ValueType> ptr)
                    {
                        _container.resize(length, PointerType{ ptr });
                    }

                    template<typename = void>
                        requires std::copyable<PointerType>
                    inline constexpr void resize(const usize length, const CPtrType<ValueType> ptr)
                    {
                        _container.resize(length, PointerType{ ptr });
                    }

                    template<typename = void>
                        requires std::copyable<PointerType>
                    inline constexpr void resize(const usize length, ArgCLRefType<PointerType> ptr)
                    {
                        _container.resize(length, ptr);
                    }

                public:
                    inline constexpr const bool operator==(const DynamicPointerArray& rhs) const noexcept
                    {
                        return _container == rhs._container;
                    }

                    inline constexpr const bool operator!=(const DynamicPointerArray& rhs) const noexcept
                    {
                        return _container != rhs._container;
                    }

                public:
                    inline constexpr const bool operator<(const DynamicPointerArray& rhs) const noexcept
                    {
                        return _container < rhs._container;
                    }

                    inline constexpr const bool operator<=(const DynamicPointerArray& rhs) const noexcept
                    {
                        return _container <= rhs._container;
                    }

                    inline constexpr const bool operator>(const DynamicPointerArray& rhs) const noexcept
                    {
                        return _container > rhs._container;
                    }

                    inline constexpr const bool operator>=(const DynamicPointerArray& rhs) const noexcept
                    {
                        return _container >= rhs._container;
                    }

                public:
                    inline constexpr decltype(auto) operator<=>(const DynamicPointerArray& rhs) const noexcept
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
        };

        template<
            typename T,
            usize l,
            PointerCategory cat = PointerCategory::Raw,
            template<typename, usize> class C = std::array
        >
        using PtrArray = pointer_array::StaticPointerArray<T, l, cat, C>;

        template<typename T,
            usize l,
            template<typename, usize> class C = std::array
        >
        using SharedArray = pointer_array::StaticPointerArray<T, l, PointerCategory::Shared, C>;

        template<typename T,
            usize l,
            template<typename, usize> class C = std::array
        >
        using UniqueArray = pointer_array::StaticPointerArray<T, l, PointerCategory::Unique, C>;

        template<
            typename T,
            PointerCategory cat = PointerCategory::Raw,
            template<typename> class C = std::vector
        >
        using DynPtrArray = pointer_array::DynamicPointerArray<T, cat, C>;

        template<
            typename T,
            template<typename> class C = std::vector
        >
        using DynSharedArray = pointer_array::DynamicPointerArray<T, PointerCategory::Shared, C>;

        template<
            typename T,
            template<typename> class C = std::vector
        >
        using DynUniqueArray = pointer_array::DynamicPointerArray<T, PointerCategory::Unique, C>;
    };
};

namespace std
{
    template<typename T, typename C, typename Self>
    inline void swap(ospf::pointer_array::PointerArrayImpl<T, C, Self>& lhs, ospf::pointer_array::PointerArrayImpl<T, C, Self>& rhs) noexcept
    {
        lhs.swap(rhs);
    }
};
 
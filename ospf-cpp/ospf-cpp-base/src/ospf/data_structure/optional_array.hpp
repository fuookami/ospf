#pragma once

#include <ospf/basic_definition.hpp>
#include <ospf/concepts/with_default.hpp>
#include <ospf/literal_constant.hpp>
#include <ospf/memory/reference.hpp>
#include <ospf/functional/iterator.hpp>
#include <ospf/functional/optional.hpp>
#include <boost/iterator/transform_iterator.hpp>
#include <vector>
#include <array>

namespace ospf
{
    inline namespace data_structure
    {
        namespace optional_array
        {
            template<
                typename T,
                usize len,
                template<typename, usize> class C
            >
                requires NotSameAs<T, void>
            class StaticOptionalArray;

            template<
                typename T,
                template<typename> class C
            >
                requires NotSameAs<T, void>
            class DynamicOptionalArray;

            template<typename T, typename C>
            class OptionalArrayConstIterator
                : public RandomIteratorImpl<const std::optional<OriginType<T>>, typename OriginType<C>::const_iterator, OptionalArrayConstIterator<T, C>>
            {
                template<
                    typename T,
                    usize len,
                    template<typename, usize> class C
                >
                    requires NotSameAs<T, void>
                friend class StaticOptionalArray;

                template<
                    typename T,
                    template<typename> class C
                >
                    requires NotSameAs<T, void>
                friend class DynamicOptionalArray;

            public:
                using ValueType = std::optional<OriginType<T>>;
                using ContainerType = OriginType<C>;
                using IterType = typename ContainerType::const_iterator;

            private:
                using Impl = RandomIteratorImpl<ConstType<ValueType>, IterType, OptionalArrayConstIterator<T, C>>;

            public:
                constexpr OptionalArrayConstIterator(ArgCLRefType<IterType> iter)
                    : Impl(iter) {}
                constexpr OptionalArrayConstIterator(const OptionalArrayConstIterator& ano) = default;
                constexpr OptionalArrayConstIterator(OptionalArrayConstIterator&& ano) noexcept = default;
                constexpr OptionalArrayConstIterator& operator=(const OptionalArrayConstIterator& rhs) = default;
                constexpr OptionalArrayConstIterator& operator=(OptionalArrayConstIterator&& rhs) noexcept = default;
                constexpr ~OptionalArrayConstIterator(void) noexcept = default;

            public:
                inline constexpr const bool has_value(void) const
                {
                    return this->_iter->has_value();
                }

            OSPF_CRTP_PERMISSION:
                inline static constexpr CLRefType<ValueType> OSPF_CRTP_FUNCTION(get)(ArgCLRefType<IterType> iter) noexcept
                {
                    return *iter;
                }

                inline static constexpr const OptionalArrayConstIterator OSPF_CRTP_FUNCTION(construct)(ArgCLRefType<IterType> iter) noexcept
                {
                    return OptionalArrayConstIterator{ iter };
                }
            };

            template<typename T, typename C>
            class OptionalArrayIterator
                : public RandomIteratorImpl<std::optional<OriginType<T>>, typename OriginType<C>::iterator, OptionalArrayIterator<T, C>>
            {
                template<
                    typename T,
                    usize len,
                    template<typename, usize> class C
                >
                    requires NotSameAs<T, void>
                friend class StaticOptionalArray;

                template<
                    typename T,
                    template<typename> class C
                >
                    requires NotSameAs<T, void>
                friend class DynamicOptionalArray;

            public:
                using ValueType = std::optional<OriginType<T>>;
                using ContainerType = OriginType<C>;
                using IterType = typename ContainerType::iterator;

            private:
                using Impl = RandomIteratorImpl<ValueType, IterType, OptionalArrayIterator<T, C>>;

            public:
                constexpr OptionalArrayIterator(ArgCLRefType<IterType> iter)
                    : Impl(iter) {}
                constexpr OptionalArrayIterator(const OptionalArrayIterator& ano) = default;
                constexpr OptionalArrayIterator(OptionalArrayIterator&& ano) noexcept = default;
                constexpr OptionalArrayIterator& operator=(const OptionalArrayIterator& rhs) = default;
                constexpr OptionalArrayIterator& operator=(OptionalArrayIterator&& rhs) noexcept = default;
                constexpr ~OptionalArrayIterator(void) noexcept = default;

            public:
                inline constexpr const bool has_value(void) const
                {
                    return this->_iter->has_value();
                }

            public:
                inline constexpr operator const OptionalArrayConstIterator<T, C>(void) const noexcept
                {
                    return OptionalArrayConstIterator<T, C>{ this->_iter };
                }

            OSPF_CRTP_PERMISSION:
                inline static constexpr LRefType<ValueType> OSPF_CRTP_FUNCTION(get)(ArgCLRefType<IterType> iter) noexcept
                {
                    return *iter;
                }

                inline static constexpr const OptionalArrayIterator OSPF_CRTP_FUNCTION(construct)(ArgCLRefType<IterType> iter) noexcept
                {
                    return OptionalArrayIterator{ iter };
                }
            };

            template<typename T, typename C>
            class OptionalArrayConstReverseIterator
                : public RandomIteratorImpl<const std::optional<OriginType<T>>, typename OriginType<C>::const_reverse_iterator, OptionalArrayConstReverseIterator<T, C>>
            {
                template<
                    typename T,
                    usize len,
                    template<typename, usize> class C
                >
                    requires NotSameAs<T, void>
                friend class StaticOptionalArray;

                template<
                    typename T,
                    template<typename> class C
                >
                    requires NotSameAs<T, void>
                friend class DynamicOptionalArray;

            public:
                using ValueType = std::optional<OriginType<T>>;
                using ContainerType = OriginType<C>;
                using IterType = typename ContainerType::const_reverse_iterator;

            private:
                using Impl = RandomIteratorImpl<ConstType<ValueType>, IterType, OptionalArrayConstReverseIterator<T, C>>;

            public:
                constexpr OptionalArrayConstReverseIterator(ArgCLRefType<IterType> iter)
                    : Impl(iter) {}
                constexpr OptionalArrayConstReverseIterator(const OptionalArrayConstReverseIterator& ano) = default;
                constexpr OptionalArrayConstReverseIterator(OptionalArrayConstReverseIterator&& ano) noexcept = default;
                constexpr OptionalArrayConstReverseIterator& operator=(const OptionalArrayConstReverseIterator& rhs) = default;
                constexpr OptionalArrayConstReverseIterator& operator=(OptionalArrayConstReverseIterator&& rhs) noexcept = default;
                constexpr ~OptionalArrayConstReverseIterator(void) noexcept = default;

            public:
                inline constexpr const bool has_value(void) const
                {
                    return this->_iter->has_value();
                }

            OSPF_CRTP_PERMISSION:
                inline static constexpr CLRefType<ValueType> OSPF_CRTP_FUNCTION(get)(ArgCLRefType<IterType> iter) noexcept
                {
                    return *iter;
                }

                inline static constexpr const OptionalArrayConstReverseIterator OSPF_CRTP_FUNCTION(construct)(ArgCLRefType<IterType> iter) noexcept
                {
                    return OptionalArrayConstReverseIterator{ iter };
                }
            };

            template<typename T, typename C>
            class OptionalArrayReverseIterator
                : public RandomIteratorImpl<std::optional<OriginType<T>>, typename OriginType<C>::reverse_iterator, OptionalArrayReverseIterator<T, C>>
            {
                template<
                    typename T,
                    usize len,
                    template<typename, usize> class C
                >
                    requires NotSameAs<T, void>
                friend class StaticOptionalArray;

                template<
                    typename T,
                    template<typename> class C
                >
                    requires NotSameAs<T, void>
                friend class DynamicOptionalArray;

            public:
                using ValueType = std::optional<OriginType<T>>;
                using ContainerType = OriginType<C>;
                using IterType = typename ContainerType::reverse_iterator;

            private:
                using Impl = RandomIteratorImpl<ValueType, IterType, OptionalArrayReverseIterator<T, C>>;

            public:
                constexpr OptionalArrayReverseIterator(ArgCLRefType<IterType> iter)
                    : Impl(iter) {}
                constexpr OptionalArrayReverseIterator(const OptionalArrayReverseIterator& ano) = default;
                constexpr OptionalArrayReverseIterator(OptionalArrayReverseIterator&& ano) noexcept = default;
                constexpr OptionalArrayReverseIterator& operator=(const OptionalArrayReverseIterator& rhs) = default;
                constexpr OptionalArrayReverseIterator& operator=(OptionalArrayReverseIterator&& rhs) noexcept = default;
                constexpr ~OptionalArrayReverseIterator(void) noexcept = default;

            public:
                inline constexpr const bool has_value(void) const
                {
                    return this->_iter->has_value();
                }

            OSPF_CRTP_PERMISSION:
                inline static constexpr LRefType<ValueType> OSPF_CRTP_FUNCTION(get)(ArgCLRefType<IterType> iter) noexcept
                {
                    return *iter;
                }

                inline static constexpr const OptionalArrayReverseIterator OSPF_CRTP_FUNCTION(construct)(ArgCLRefType<IterType> iter) noexcept
                {
                    return OptionalArrayReverseIterator{ iter };
                }
            };

            template<typename T, typename C>
            class OptionalArrayConstUncheckedIterator
                : public RandomIteratorImpl<ConstType<T>, typename OriginType<C>::const_iterator, OptionalArrayConstUncheckedIterator<T, C>>
            {
                template<
                    typename T,
                    usize len,
                    template<typename, usize> class C
                >
                    requires NotSameAs<T, void>
                friend class StaticOptionalArray;

                template<
                    typename T,
                    template<typename> class C
                >
                    requires NotSameAs<T, void>
                friend class DynamicOptionalArray;

            public:
                using ValueType = OriginType<T>;
                using ContainerType = OriginType<C>;
                using IterType = typename ContainerType::const_iterator;

            private:
                using Impl = RandomIteratorImpl<ConstType<ValueType>, IterType, OptionalArrayConstUncheckedIterator<T, C>>;

            public:
                constexpr OptionalArrayConstUncheckedIterator(ArgCLRefType<IterType> iter)
                    : Impl(iter) {}
                constexpr OptionalArrayConstUncheckedIterator(const OptionalArrayConstUncheckedIterator& ano) = default;
                constexpr OptionalArrayConstUncheckedIterator(OptionalArrayConstUncheckedIterator&& ano) noexcept = default;
                constexpr OptionalArrayConstUncheckedIterator& operator=(const OptionalArrayConstUncheckedIterator& rhs) = default;
                constexpr OptionalArrayConstUncheckedIterator& operator=(OptionalArrayConstUncheckedIterator&& rhs) noexcept = default;
                constexpr ~OptionalArrayConstUncheckedIterator(void) noexcept = default;

            public:
                inline constexpr const bool has_value(void) const
                {
                    return this->_iter->has_value();
                }

            OSPF_CRTP_PERMISSION:
                inline static constexpr CLRefType<ValueType> OSPF_CRTP_FUNCTION(get)(ArgCLRefType<IterType> iter) noexcept
                {
                    return **iter;
                }

                inline static constexpr const OptionalArrayConstUncheckedIterator OSPF_CRTP_FUNCTION(construct)(ArgCLRefType<IterType> iter) noexcept
                {
                    return OptionalArrayConstUncheckedIterator{ iter };
                }
            };

            template<typename T, typename C>
            class OptionalArrayUncheckedIterator
                : public RandomIteratorImpl<OriginType<T>, typename OriginType<C>::iterator, OptionalArrayUncheckedIterator<T, C>>
            {
                template<
                    typename T,
                    usize len,
                    template<typename, usize> class C
                >
                    requires NotSameAs<T, void>
                friend class StaticOptionalArray;

                template<
                    typename T,
                    template<typename> class C
                >
                    requires NotSameAs<T, void>
                friend class DynamicOptionalArray;

            public:
                using ValueType = OriginType<T>;
                using ContainerType = OriginType<C>;
                using IterType = typename ContainerType::iterator;

            private:
                using Impl = RandomIteratorImpl<ValueType, IterType, OptionalArrayUncheckedIterator<T, C>>;

            public:
                constexpr OptionalArrayUncheckedIterator(ArgCLRefType<IterType> iter)
                    : Impl(iter) {}
                constexpr OptionalArrayUncheckedIterator(const OptionalArrayUncheckedIterator& ano) = default;
                constexpr OptionalArrayUncheckedIterator(OptionalArrayUncheckedIterator&& ano) noexcept = default;
                constexpr OptionalArrayUncheckedIterator& operator=(const OptionalArrayUncheckedIterator& rhs) = default;
                constexpr OptionalArrayUncheckedIterator& operator=(OptionalArrayUncheckedIterator&& rhs) noexcept = default;
                constexpr ~OptionalArrayUncheckedIterator(void) noexcept = default;

            public:
                inline constexpr const bool has_value(void) const
                {
                    return this->_iter->has_value();
                }

            public:
                inline constexpr operator const OptionalArrayConstUncheckedIterator<T, C>(void) const noexcept
                {
                    return OptionalArrayConstUncheckedIterator<T, C>{ this->_iter };
                }

            OSPF_CRTP_PERMISSION:
                inline static constexpr LRefType<ValueType> OSPF_CRTP_FUNCTION(get)(ArgCLRefType<IterType> iter) noexcept
                {
                    return **iter;
                }

                inline static constexpr const OptionalArrayUncheckedIterator OSPF_CRTP_FUNCTION(construct)(ArgCLRefType<IterType> iter) noexcept
                {
                    return OptionalArrayUncheckedIterator{ iter };
                }
            };

            template<typename T, typename C>
            class OptionalArrayConstUncheckedReverseIterator
                : public RandomIteratorImpl<ConstType<T>, typename OriginType<C>::const_reverse_iterator, OptionalArrayConstUncheckedReverseIterator<T, C>>
            {
                template<
                    typename T,
                    usize len,
                    template<typename, usize> class C
                >
                    requires NotSameAs<T, void>
                friend class StaticOptionalArray;

                template<
                    typename T,
                    template<typename> class C
                >
                    requires NotSameAs<T, void>
                friend class DynamicOptionalArray;

            public:
                using ValueType = OriginType<T>;
                using ContainerType = OriginType<C>;
                using IterType = typename ContainerType::const_reverse_iterator;

            private:
                using Impl = RandomIteratorImpl<ConstType<ValueType>, IterType, OptionalArrayConstUncheckedReverseIterator<T, C>>;

            public:
                constexpr OptionalArrayConstUncheckedReverseIterator(ArgCLRefType<IterType> iter)
                    : Impl(iter) {}
                constexpr OptionalArrayConstUncheckedReverseIterator(const OptionalArrayConstUncheckedReverseIterator& ano) = default;
                constexpr OptionalArrayConstUncheckedReverseIterator(OptionalArrayConstUncheckedReverseIterator&& ano) noexcept = default;
                constexpr OptionalArrayConstUncheckedReverseIterator& operator=(const OptionalArrayConstUncheckedReverseIterator& rhs) = default;
                constexpr OptionalArrayConstUncheckedReverseIterator& operator=(OptionalArrayConstUncheckedReverseIterator&& rhs) noexcept = default;
                constexpr ~OptionalArrayConstUncheckedReverseIterator(void) noexcept = default;

            public:
                inline constexpr const bool has_value(void) const
                {
                    return this->_iter->has_value();
                }

            OSPF_CRTP_PERMISSION:
                inline static constexpr CLRefType<ValueType> OSPF_CRTP_FUNCTION(get)(ArgCLRefType<IterType> iter) noexcept
                {
                    return **iter;
                }

                inline static constexpr const OptionalArrayConstUncheckedReverseIterator OSPF_CRTP_FUNCTION(construct)(ArgCLRefType<IterType> iter) noexcept
                {
                    return OptionalArrayConstUncheckedReverseIterator{ iter };
                }
            };

            template<typename T, typename C>
            class OptionalArrayUncheckedReverseIterator
                : public RandomIteratorImpl<OriginType<T>, typename OriginType<C>::reverse_iterator, OptionalArrayUncheckedReverseIterator<T, C>>
            {
                template<
                    typename T,
                    usize len,
                    template<typename, usize> class C
                >
                    requires NotSameAs<T, void>
                friend class StaticOptionalArray;

                template<
                    typename T,
                    template<typename> class C
                >
                    requires NotSameAs<T, void>
                friend class DynamicOptionalArray;

            public:
                using ValueType = OriginType<T>;
                using ContainerType = OriginType<C>;
                using IterType = typename ContainerType::reverse_iterator;

            private:
                using Impl = RandomIteratorImpl<ValueType, IterType, OptionalArrayUncheckedReverseIterator<T, C>>;

            public:
                constexpr OptionalArrayUncheckedReverseIterator(ArgCLRefType<IterType> iter)
                    : Impl(iter) {}
                constexpr OptionalArrayUncheckedReverseIterator(const OptionalArrayUncheckedReverseIterator& ano) = default;
                constexpr OptionalArrayUncheckedReverseIterator(OptionalArrayUncheckedReverseIterator&& ano) noexcept = default;
                constexpr OptionalArrayUncheckedReverseIterator& operator=(const OptionalArrayUncheckedReverseIterator& rhs) = default;
                constexpr OptionalArrayUncheckedReverseIterator& operator=(OptionalArrayUncheckedReverseIterator&& rhs) noexcept = default;
                constexpr ~OptionalArrayUncheckedReverseIterator(void) noexcept = default;

            public:
                inline constexpr const bool has_value(void) const
                {
                    return this->_iter->has_value();
                }

            OSPF_CRTP_PERMISSION:
                inline static constexpr LRefType<ValueType> OSPF_CRTP_FUNCTION(get)(ArgCLRefType<IterType> iter) noexcept
                {
                    return **iter;
                }

                inline static constexpr const OptionalArrayUncheckedReverseIterator OSPF_CRTP_FUNCTION(construct)(ArgCLRefType<IterType> iter) noexcept
                {
                    return OptionalArrayUncheckedReverseIterator{ iter };
                }
            };

            template<typename T, typename C>
            struct OptionalArrayAccessPolicy;

            template<
                typename T,
                usize len,
                template<typename, usize> class C
            >
            struct OptionalArrayAccessPolicy<T, C<std::optional<OriginType<T>>, len>>
            {
            public:
                using ValueType = OriginType<T>;
                using OptType = std::optional<ValueType>;
                using ContainerType = C<OptType, len>;
                using IterType = OptionalArrayIterator<ValueType, ContainerType>;
                using ConstIterType = OptionalArrayConstIterator<ValueType, ContainerType>;
                using ReverseIterType = OptionalArrayReverseIterator<ValueType, ContainerType>;
                using ConstReverseIterType = OptionalArrayConstReverseIterator<ValueType, ContainerType>;
                using UncheckedIterType = OptionalArrayUncheckedIterator<ValueType, ContainerType>;
                using ConstUncheckedIterType = OptionalArrayConstUncheckedIterator<ValueType, ContainerType>;
                using UncheckedReverseIterType = OptionalArrayUncheckedReverseIterator<ValueType, ContainerType>;
                using ConstUncheckedReverseIterType = OptionalArrayConstUncheckedReverseIterator<ValueType, ContainerType>;

            public:
                inline static constexpr const usize size(CLRefType<ContainerType> array) noexcept
                {
                    return len;
                }

                inline static constexpr LRefType<OptType> get(LRefType<ContainerType> array, const usize i)
                {
                    return array.at(i);
                }

                inline static constexpr CLRefType<OptType> get(CLRefType<ContainerType> array, const usize i)
                {
                    return array.at(i);
                }

                inline static constexpr LRefType<OptType> get_uncheked(LRefType<ContainerType> array, const usize i)
                {
                    return array[i];
                }

                inline static constexpr CLRefType<OptType> get_uncheked(CLRefType<ContainerType> array, const usize i)
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
                typename Alloc,
                template<typename, typename> class C
            >
            struct OptionalArrayAccessPolicy<T, C<std::optional<OriginType<T>>, Alloc>>
            {
            public:
                using ValueType = OriginType<T>;
                using OptType = std::optional<ValueType>;
                using ContainerType = C<OptType, Alloc>;
                using IterType = OptionalArrayIterator<ValueType, ContainerType>;
                using ConstIterType = OptionalArrayConstIterator<ValueType, ContainerType>;
                using ReverseIterType = OptionalArrayReverseIterator<ValueType, ContainerType>;
                using ConstReverseIterType = OptionalArrayConstReverseIterator<ValueType, ContainerType>;
                using UncheckedIterType = OptionalArrayUncheckedIterator<ValueType, ContainerType>;
                using ConstUncheckedIterType = OptionalArrayConstUncheckedIterator<ValueType, ContainerType>;
                using UncheckedReverseIterType = OptionalArrayUncheckedReverseIterator<ValueType, ContainerType>;
                using ConstUncheckedReverseIterType = OptionalArrayConstUncheckedReverseIterator<ValueType, ContainerType>;

            protected:
                OptionalArrayAccessPolicy(void) = default;
            public:
                OptionalArrayAccessPolicy(const OptionalArrayAccessPolicy& ano) = default;
                OptionalArrayAccessPolicy(OptionalArrayAccessPolicy&& ano) noexcept = default;
                OptionalArrayAccessPolicy& operator=(const OptionalArrayAccessPolicy& rhs) = default;
                OptionalArrayAccessPolicy& operator=(OptionalArrayAccessPolicy&& rhs) noexcept = default;
                ~OptionalArrayAccessPolicy(void) = default;

            public:
                inline static constexpr const usize size(CLRefType<ContainerType> array) noexcept
                {
                    return array.size();
                }

            public:
                inline static constexpr LRefType<OptType> get(LRefType<ContainerType> array, const usize i)
                {
                    return array.at(i);
                }

                inline static constexpr CLRefType<OptType> get(CLRefType<ContainerType> array, const usize i)
                {
                    return array.at(i);
                }

                inline static constexpr LRefType<OptType> get_uncheked(LRefType<ContainerType> array, const usize i)
                {
                    return array[i];
                }

                inline static constexpr CLRefType<OptType> get_uncheked(CLRefType<ContainerType> array, const usize i)
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
            class OptionalArrayImpl
            {
                OSPF_CRTP_IMPL

            public:
                using ValueType = OriginType<T>;
                using ContainerType = OriginType<C>;
                using AccessPolicyType = OptionalArrayAccessPolicy<ValueType, ContainerType>;
                using OptType = typename AccessPolicyType::OptType;
                using IterType = typename AccessPolicyType::IterType;
                using ConstIterType = typename AccessPolicyType::ConstIterType;
                using ReverseIterType = typename AccessPolicyType::ReverseIterType;
                using ConstReverseIterType = typename AccessPolicyType::ConstReverseIterType;
                using UncheckedIterType = typename AccessPolicyType::UncheckedIterType;
                using ConstUncheckedIterType = typename AccessPolicyType::ConstUncheckedIterType;
                using UncheckedReverseIterType = typename AccessPolicyType::UncheckedReverseIterType;
                using ConstUncheckedReverseIterType = typename AccessPolicyType::ConstUncheckedReverseIterType;

            protected:
                constexpr OptionalArrayImpl(void) = default;
            public:
                constexpr OptionalArrayImpl(const OptionalArrayImpl& ano) = default;
                constexpr OptionalArrayImpl(OptionalArrayImpl&& ano) noexcept = default;
                constexpr OptionalArrayImpl& operator=(const OptionalArrayImpl& rhs) = default;
                constexpr OptionalArrayImpl& operator=(OptionalArrayImpl&& rhs) noexcept = default;
                constexpr ~OptionalArrayImpl(void) noexcept = default;

            public:
                inline constexpr const bool has_value(const usize i) const
                {
                    return at(i).has_value();
                }

                inline constexpr void set(const usize i, const std::nullopt_t _)
                {
                    AccessPolicyType::get(container(), i) = OptType{ std::nullopt };
                }

                inline constexpr void set(const usize i, ArgCLRefType<ValueType> value)
                {
                    AccessPolicyType::get(container(), i) = OptType{ value };
                }

                template<typename = void>
                    requires ReferenceFaster<ValueType> && std::movable<ValueType>
                inline constexpr void set(const usize i, ArgRRefType<ValueType> value)
                {
                    AccessPolicyType::get(container(), i) = OptType{ move<ValueType>(value) };
                }

                inline constexpr void set(const usize i, ArgCLRefType<OptType> value)
                {
                    AccessPolicyType::get(container(), i) = value;
                }

                template<typename = void>
                    requires ReferenceFaster<OptType> && std::movable<OptType>
                inline constexpr void set(const usize i, ArgRRefType<OptType> value)
                {
                    AccessPolicyType::get(container(), i) = move<OptType>(value);
                }

            public:
                inline constexpr LRefType<OptType> at(const usize i)
                {
                    return AccessPolicyType::get(container(), i);
                }

                inline constexpr CLRefType<OptType> at(const usize i) const
                {
                    return AccessPolicyType::get(const_container(), i);
                }

                inline constexpr LRefType<OptType> operator[](const usize i)
                {
                    return AccessPolicyType::get_unchecked(container(), i);
                }

                inline constexpr CLRefType<OptType> operator[](const usize i) const
                {
                    return AccessPolicyType::get_unchecked(const_container(), i);
                }

            public:
                inline constexpr LRefType<OptType> front(void)
                {
                    return at(0_uz);
                }

                inline constexpr CLRefType<OptType> front(void) const
                {
                    return at(0_uz);
                }

                inline constexpr LRefType<OptType> back(void)
                {
                    return at(size() - 1_uz);
                }

                inline constexpr CLRefType<OptType> back(void) const
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
                inline void swap(OptionalArrayImpl& ano) noexcept
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

            template<typename T, typename C, typename Self>
            class OptionalArrayUncheckedAccessorImpl
            {
                using Impl = OptionalArrayImpl<T, C, Self>;

            public:
                using ValueType = typename Impl::ValueType;
                using ContainerType = typename Impl::ContainerType;
                using OptType = typename Impl::OptType;
                using IterType = typename Impl::IterType;
                using ConstIterType = typename Impl::ConstIterType;
                using ReverseIterType = typename Impl::ReverseIterType;
                using ConstReverseIterType = typename Impl::ConstReverseIterType;
                using UncheckedIterType = typename Impl::UncheckedIterType;
                using ConstUncheckedIterType = typename Impl::ConstUncheckedIterType;
                using UncheckedReverseIterType = typename Impl::UncheckedReverseIterType;
                using ConstUncheckedReverseIterType = typename Impl::ConstUncheckedReverseIterType;

            public:
                constexpr OptionalArrayUncheckedAccessorImpl(const Impl& impl)
                    : _impl(impl) {}
                constexpr OptionalArrayUncheckedAccessorImpl(const OptionalArrayUncheckedAccessorImpl& ano) = default;
                constexpr OptionalArrayUncheckedAccessorImpl(OptionalArrayUncheckedAccessorImpl&& ano) noexcept = default;
                constexpr OptionalArrayUncheckedAccessorImpl& operator=(const OptionalArrayUncheckedAccessorImpl& rhs) = default;
                constexpr OptionalArrayUncheckedAccessorImpl& operator=(OptionalArrayUncheckedAccessorImpl&& rhs) noexcept = default;
                constexpr ~OptionalArrayUncheckedAccessorImpl(void) noexcept = default;

            public:
                inline constexpr const bool has_value(const usize i) const
                {
                    return _impl.has_value(i);
                }

            public:
                inline constexpr LRefType<OptType> at(const usize i)
                {
                    return *_impl.at(i);
                }

                inline constexpr CLRefType<OptType> at(const usize i) const
                {
                    return *_impl.at(i);
                }

                inline constexpr LRefType<OptType> operator[](const usize i)
                {
                    return *_impl[i];
                }

                inline constexpr CLRefType<OptType> operator[](const usize i)
                {
                    return *_impl[i];
                }

            public:
                inline constexpr LRefType<OptType> front(void)
                {
                    return *_impl.front();
                }

                inline constexpr CLRefType<OptType> front(void) const
                {
                    return *_impl.front();
                }

                inline constexpr LRefType<OptType> back(void)
                {
                    return *_impl.back();
                }

                inline constexpr CLRefType<OptType> back(void) const
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
                template<typename, usize> class C
            >
                requires NotSameAs<T, void>
            class StaticOptionalArray
                : public OptionalArrayImpl<T, C<std::optional<OriginType<T>>, len>, StaticOptionalArray<T, len, C>>
            {
                using Impl = OptionalArrayImpl<T, C<std::optional<OriginType<T>>, len>, StaticOptionalArray<T, len, C>>;
                using UncheckedAccessorImpl = OptionalArrayUncheckedAccessorImpl<T, C<std::optional<T>, len>, StaticOptionalArray<T, len, C>>;

                template<
                    typename T,
                    template<typename> class C
                >
                    requires NotSameAs<T, void>
                friend class DynamicOptionalArray;

            public:
                using typename Impl::ValueType;
                using typename Impl::OptType;
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
                StaticOptionalArray(void) = default;

                StaticOptionalArray(ArgRRefType<ContainerType> container)
                    : _container(move<ContainerType>(container)) {}

                StaticOptionalArray(std::initializer_list<ValueType> values)
                {
                    for (auto i{ 0_uz }, j{ (std::min)(len, values.size) }; i != j; ++i)
                    {
                        _container[i] = OptType{ move<ValueType>(values[i]) };
                    }
                }

                StaticOptionalArray(std::initializer_list<OptType> values)
                    : _container(std::move(values)) {}

            public:
                constexpr StaticOptionalArray(const StaticOptionalArray& ano) = default;
                constexpr StaticOptionalArray(StaticOptionalArray&& ano) noexcept = default;
                constexpr StaticOptionalArray& operator=(const StaticOptionalArray& rhs) = default;
                constexpr StaticOptionalArray& operator=(StaticOptionalArray&& rhs) noexcept = default;
                constexpr ~StaticOptionalArray(void) = default;

            public:
                inline constexpr const UncheckedAccessorImpl unchecked(void) const noexcept
                {
                    return UncheckedAccessorImpl{ dynamic_cast<const Impl&>(*this) };
                }

                template<typename = void>
                    requires requires (ContainerType& container) { { container.data() } -> DecaySameAs<PtrType<OptType>>; }
                inline constexpr const PtrType<OptType> data(void) noexcept
                {
                    return _container.data();
                }

                template<typename = void>
                    requires requires (const ContainerType& container) { { container.data() } -> DecaySameAs<CPtrType<OptType>>; }
                inline constexpr const CPtrType<OptType> data(void) const noexcept
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
                    fill(std::nullopt);
                }

                inline constexpr void fill(const std::nullopt_t _ = std::nullopt) noexcept
                {
                    if constexpr (std::copyable<OptType>)
                    {
                        _container.fill(OptType{ std::nullopt });
                    }
                    else
                    {
                        for (auto i{ 0_uz }; i != _container.size(); ++i)
                        {
                            _container[i] = OptType{ std::nullopt };
                        }
                    }
                }

                template<typename = void>
                    requires std::copyable<OptType>
                inline constexpr void fill(ArgCLRefType<ValueType> value) noexcept
                {
                    _container.fill(OptType{ value });
                }

                template<typename = void>
                    requires std::copyable<OptType> && ReferenceFaster<ValueType> && std::movable<ValueType>
                inline void fill(ArgRRefType<ValueType> value) noexcept
                {
                    _container.fill(OptType{ move<ValueType>(value) });
                }

                template<typename = void>
                    requires std::copyable<OptType>
                inline constexpr void fill(ArgCLRefType<OptType> value) noexcept
                {
                    _container.fill(value);
                }

            public:
                inline constexpr const bool operator==(const StaticOptionalArray& rhs) const noexcept
                {
                    return _container == rhs._container;
                }

                inline constexpr const bool operator!=(const StaticOptionalArray& rhs) const noexcept
                {
                    return _container != rhs._container;
                }

            public:
                inline constexpr const bool operator<(const StaticOptionalArray& rhs) const noexcept
                {
                    return _container < rhs._container;
                }

                inline constexpr const bool operator<=(const StaticOptionalArray& rhs) const noexcept
                {
                    return _container <= rhs._container;
                }

                inline constexpr const bool operator>(const StaticOptionalArray& rhs) const noexcept
                {
                    return _container > rhs._container;
                }

                inline constexpr const bool operator>=(const StaticOptionalArray& rhs) const noexcept
                {
                    return _container >= rhs._container;
                }

            public:
                inline constexpr decltype(auto) operator<=>(const StaticOptionalArray& rhs) const noexcept
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
                template<typename> class C
            >
                requires NotSameAs<T, void>
            class DynamicOptionalArray
                : public OptionalArrayImpl<T, C<std::optional<OriginType<T>>>, DynamicOptionalArray<T, C>>
            {
                using Impl = OptionalArrayImpl<T, C<std::optional<OriginType<T>>>, DynamicOptionalArray<T, C>>;
                using UncheckedAccessorImpl = OptionalArrayUncheckedAccessorImpl<T, C<std::optional<OriginType<T>>>, DynamicOptionalArray<T, C>>;

            public:
                using typename Impl::ValueType;
                using typename Impl::OptType;
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
                constexpr DynamicOptionalArray(void) = default;

                template<typename = void>
                    requires std::default_initializable<OptType>
                constexpr explicit DynamicOptionalArray(const usize length)
                    : _container(length) {}

                template<typename = void>
                    requires (!std::default_initializable<OptType> && WithDefault<OptType> && std::copyable<OptType>)
                constexpr explicit DynamicOptionalArray(const usize length)
                    : _container(length, DefaultValue<OptType>::value()) {}

                template<typename = void>
                    requires std::copyable<OptType>
                constexpr DynamicOptionalArray(const usize length, ArgCLRefType<ValueType> value)
                    : _container(length, OptType{ value }) {}

                template<typename = void>
                    requires std::copyable<OptType> && ReferenceFaster<ValueType> && std::movable<ValueType>
                DynamicOptionalArray(const usize length, ArgRRefType<ValueType> value)
                    : _container(length, OptType{ move<ValueType>(value) }) {}

                template<typename = void>
                    requires std::copyable<OptType>
                DynamicOptionalArray(const usize length, ArgCLRefType<OptType> value)
                    : _container(length, value) {}

                template<std::input_iterator It>
                    requires requires (const It it) { { *it } -> DecaySameAs<ValueType>; }
                constexpr DynamicOptionalArray(It&& first, It&& last)
                {
                    if constexpr (std::is_same_v<decltype(*first), CLRefType<ValueType>>)
                    {
                        _container.assign(
                            boost::make_transform_iterator(std::forward<It>(first), [](ArgCLRefType<ValueType> value) { return OptType{ value }; }),
                            boost::make_transform_iterator(std::forward<It>(last), [](ArgCLRefType<ValueType> value) { return OptType{ value }; })
                        );
                    }
                    else
                    {
                        _container.assign(
                            boost::make_transform_iterator(std::forward<It>(first), [](ArgLRefType<ValueType> value) { return OptType{ move<ValueType>(value) }; }),
                            boost::make_transform_iterator(std::forward<It>(last), [](ArgLRefType<ValueType> value) { return OptType{ move<ValueType>(value) }; })
                        );
                    }
                }

                template<std::input_iterator It>
                    requires requires (const It it) { { *it } -> DecaySameAs<OptType>; }
                constexpr DynamicOptionalArray(It&& first, It&& last)
                    : _container(std::forward<It>(first), std::forward<It>(last)) {}

                constexpr DynamicOptionalArray(std::initializer_list<ValueType> values)
                    : _container(values.size())
                {
                    for (auto i{ 0_uz }; i != values.size(); ++i)
                    {
                        _container[i] = move<ValueType>(values[i]);
                    }
                }

                constexpr DynamicOptionalArray(std::initializer_list<OptType> values)
                    : _container(std::move(values)) {}

            public:
                constexpr DynamicOptionalArray(const DynamicOptionalArray& ano) = default;
                constexpr DynamicOptionalArray(DynamicOptionalArray&& ano) noexcept = default;
                constexpr DynamicOptionalArray& operator=(const DynamicOptionalArray& rhs) = default;
                constexpr DynamicOptionalArray& operator=(DynamicOptionalArray&& rhs) noexcept = default;
                constexpr ~DynamicOptionalArray(void) = default;

            public:
                inline constexpr void assign(const usize length, const std::nullopt_t _ = std::nullopt)
                {
                    if constexpr (std::copyable<OptType>)
                    {
                        _container.assign(length, OptType{ std::nullopt });
                    }
                    else
                    {
                        _container.clear();
                        for (auto i{ 0_uz }; i != length; ++i)
                        {
                            _container.push_back(OptType{ std::nullopt });
                        }
                    }
                }

                template<typename = void>
                    requires std::copyable<OptType>
                inline constexpr void assign(const usize length, ArgCLRefType<ValueType> value)
                {
                    _container.assign(length, OptType{ value });
                }

                template<typename = void>
                    requires std::copyable<OptType> && ReferenceFaster<ValueType> && std::movable<ValueType>
                inline constexpr void assign(const usize length, ArgRRefType<ValueType> value)
                {
                    _container.assign(length, OptType{ move<ValueType>(value) });
                }

                template<typename = void>
                    requires std::copyable<OptType>
                inline constexpr void assign(const usize length, ArgCLRefType<OptType> value)
                {
                    _container.assign(length, value);
                }

                template<std::input_iterator It>
                    requires requires (const It it) { { *it } -> DecaySameAs<ValueType>; }
                inline constexpr void assign(It&& first, It&& last)
                {
                    if constexpr (std::is_same_v<decltype(*first), CLRefType<ValueType>>)
                    {
                        _container.assign(
                            boost::make_transform_iterator(std::forward<It>(first), [](ArgCLRefType<ValueType> value) { return OptType{ value }; }),
                            boost::make_transform_iterator(std::forward<It>(last), [](ArgCLRefType<ValueType> value) { return OptType{ value }; })
                        );
                    }
                    else
                    {
                        _container.assign(
                            boost::make_transform_iterator(std::forward<It>(first), [](ArgLRefType<ValueType> value) { return OptType{ move<ValueType>(value) }; }),
                            boost::make_transform_iterator(std::forward<It>(last), [](ArgLRefType<ValueType> value) { return OptType{ move<ValueType>(value) }; })
                        );
                    }
                }

                template<std::input_iterator It>
                    requires requires (const It it) { { *it } -> DecaySameAs<OptType>; }
                inline constexpr void assign(It&& first, It&& last)
                {
                    _container.assign(std::forward<It>(first), std::forward<It>(last));
                }

                inline constexpr void assign(std::initializer_list<ValueType> values)
                {
                    _container.assign(values.size());
                    for (auto i{ 0_uz }; i != values.size(); ++i)
                    {
                        _container[i] = move<ValueType>(values[i]);
                    }
                }

                inline constexpr void assign(std::initializer_list<OptType> values)
                {
                    _container.assign(std::move(values));
                }

                template<
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::copyable<OptType>
                inline constexpr void assign(const StaticOptionalArray<ValueType, len, C1>& values)
                {
                    assign(values._container.begin(), values._container.end());
                }

                template<
                    usize len,
                    template<typename, usize> class C1
                >
                inline constexpr void assign(StaticOptionalArray<ValueType, len, C1>&& values)
                {
                    _container.clear();
                    std::move(values._container.begin(), values._container.end(), std::back_inserter(_container));
                }

                template<
                    typename U,
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::convertible_to<U, ValueType>
                inline constexpr void assign(const StaticOptionalArray<U, len, C1>& values)
                {
                    assign(
                        boost::make_transform_iterator(values._container.begin(), [](const std::optional<U>& opt) { return map<ValueType>(opt); }),
                        boost::make_transform_iterator(values._container.end(), [](const std::optional<U>& opt) { return map<ValueType>(opt); })
                    );
                }

                template<
                    typename U,
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::convertible_to<U, ValueType>
                inline constexpr void assign(StaticOptionalArray<U, len, C1>&& values)
                {
                    assign(
                        boost::make_transform_iterator(values._container.begin(), [](std::optional<U>& opt) { return map<ValueType>(std::move(opt)); }),
                        boost::make_transform_iterator(values._container.end(), [](std::optional<U>& opt) { return map<ValueType>(std::move(opt)); })
                    );
                }

                template<template<typename> class C1>
                    requires std::copyable<OptType>
                inline constexpr void assign(const DynamicOptionalArray<ValueType, C1>& values)
                {
                    assign(values._container.begin(), values._container.end());
                }

                template<template<typename> class C1>
                inline constexpr void assign(DynamicOptionalArray<ValueType, C1>&& values)
                {
                    _container.clear();
                    std::move(values._container.begin(), values._container.end(), std::back_inserter(_container));
                }

                template<
                    typename U,
                    template<typename> class C1
                >
                    requires std::convertible_to<U, ValueType>
                inline constexpr void assign(const DynamicOptionalArray<U, C1>& values)
                {
                    assign(
                        boost::make_transform_iterator(values._container.begin(), [](const std::optional<U>& opt) { return map<ValueType>(opt); }),
                        boost::make_transform_iterator(values._container.end(), [](const std::optional<U>& opt) { return map<ValueType>(opt); })
                    );
                }

                template<
                    typename U,
                    template<typename> class C1
                >
                    requires std::convertible_to<U, ValueType>
                inline constexpr void assign(DynamicOptionalArray<U, C1>&& values)
                {
                    assign(
                        boost::make_transform_iterator(values._container.begin(), [](std::optional<U>& opt) { return map<ValueType>(std::move(opt)); }),
                        boost::make_transform_iterator(values._container.end(), [](std::optional<U>& opt) { return map<ValueType>(std::move(opt)); })
                    );
                }

            public:
                inline constexpr UncheckedAccessorImpl unchecked(void) const noexcept
                {
                    return UncheckedAccessorImpl{ dynamic_cast<const Impl&>(*this) };
                }

                template<typename = void>
                    requires requires (ContainerType& container) { { container.data() } -> DecaySameAs<PtrType<OptType>>; }
                inline constexpr const PtrType<OptType> data(void) noexcept
                {
                    return _container.data();
                }

                template<typename = void>
                    requires requires (const ContainerType& container) { { container.data() } -> DecaySameAs<CPtrType<OptType>>; }
                inline constexpr const CPtrType<OptType> data(void) const noexcept
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

                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, const std::nullopt_t _)
                {
                    return IterType{ _container.insert(pos._iter, OptType{ std::nullopt }) };
                }

                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, ArgCLRefType<ValueType> value)
                {
                    return IterType{ _container.insert(pos._iter, OptType{ value }) };
                }

                template<typename = void>
                    requires ReferenceFaster<ValueType> && std::movable<ValueType>
                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, ArgRRefType<ValueType> value)
                {
                    return IterType{ _container.insert(pos._iter, OptType{ move<ValueType>(value) }) };
                }

                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, ArgCLRefType<OptType> value)
                {
                    return IterType{ _container.insert(pos._iter, value) };
                }

                template<typename = void>
                    requires ReferenceFaster<ValueType> && std::movable<ValueType>
                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, ArgRRefType<OptType> value)
                {
                    return IterType{ _container.insert(pos._iter, move<OptType>(value)) };
                }

                inline constexpr RetType<UncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, const std::nullopt_t _)
                {
                    return UncheckedIterType{ _container.insert(pos._iter, OptType{ std::nullopt }) };
                }

                inline constexpr RetType<UncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, ArgCLRefType<ValueType> value)
                {
                    return UncheckedIterType{ _container.insert(pos._iter, OptType{ value }) };
                }

                template<typename = void>
                    requires ReferenceFaster<ValueType> && std::movable<ValueType>
                inline constexpr RetType<UncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, ArgRRefType<ValueType> value)
                {
                    return UncheckedIterType{ _container.insert(pos._iter, OptType{ move<ValueType>(value) }) };
                }

                inline constexpr RetType<UncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, ArgCLRefType<OptType> value)
                {
                    return UncheckedIterType{ _container.insert(pos._iter, value) };
                }

                template<typename = void>
                    requires ReferenceFaster<OptType> && std::movable<OptType>
                inline constexpr RetType<UncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, ArgRRefType<OptType> value)
                {
                    return UncheckedIterType{ _container.insert(pos._iter, move<OptType>(value)) };
                }

                template<std::input_iterator It>
                    requires requires (const It it) { { *it } -> DecaySameAs<ValueType>; }
                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, It&& first, It&& last)
                {
                    if constexpr (std::is_same_v<decltype(*first), CLRefType<ValueType>>)
                    {
                        return IterType{ _container.insert(pos._iter,
                            boost::make_transform_iterator(std::forward<It>(first), [](ArgCLRefType<ValueType> value) { return OptType{ value }; }),
                            boost::make_transform_iterator(std::forward<It>(last), [](ArgCLRefType<ValueType> value) { return OptType{ value }; })
                        ) };
                    }
                    else
                    {
                        return IterType{ _container.insert(pos._iter,
                            boost::make_transform_iterator(std::forward<It>(first), [](ArgLRefType<ValueType> value) { return OptType{ move<ValueType>(value) }; }),
                            boost::make_transform_iterator(std::forward<It>(last), [](ArgLRefType<ValueType> value) { return OptType{ move<ValueType>(value) }; })
                        ) };
                    }
                }

                template<std::input_iterator It>
                    requires requires (const It it) { { *it } -> DecaySameAs<OptType>; }
                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, It&& first, It&& last)
                {
                    return IterType{ _container.insert(pos._iter, std::forward<It>(first), std::forward<It>(last)) };
                }

                template<std::input_iterator It>
                    requires requires (const It it) { { *it } -> DecaySameAs<ValueType>; }
                inline constexpr RetType<UncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, It&& first, It&& last)
                {
                    if constexpr (std::is_same_v<decltype(*first), CLRefType<ValueType>>)
                    {
                        return UncheckedIterType{ _container.insert(pos._iter,
                            boost::make_transform_iterator(std::forward<It>(first), [](ArgCLRefType<ValueType> value) { return OptType{ value }; }),
                            boost::make_transform_iterator(std::forward<It>(last), [](ArgCLRefType<ValueType> value) { return OptType{ value }; })
                        ) };
                    }
                    else
                    {
                        return UncheckedIterType{ _container.insert(pos._iter,
                            boost::make_transform_iterator(std::forward<It>(first), [](ArgLRefType<ValueType> value) { return OptType{ move<ValueType>(value) }; }),
                            boost::make_transform_iterator(std::forward<It>(last), [](ArgLRefType<ValueType> value) { return OptType{ move<ValueType>(value) }; })
                        ) };
                    }
                }

                template<std::input_iterator It>
                    requires requires (const It it) { { *it } -> DecaySameAs<OptType>; }
                inline constexpr RetType<UncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, It&& first, It&& last)
                {
                    return UncheckedIterType{ _container.insert(pos._iter, std::forward<It>(first), std::forward<It>(last))};
                }

                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, std::initializer_list<ValueType> values)
                {
                    auto it = pos._iter;
                    for (auto i{ 0_uz }; i != values.size(); ++i)
                    {
                        it = _container.insert(it, OptType{ move<ValueType>(values[i]) });
                    }
                    return IterType{ it };
                }

                inline constexpr RetType<UncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, std::initializer_list<ValueType> values)
                {
                    auto it = pos._iter;
                    for (auto i{ 0_uz }; i != values.size(); ++i)
                    {
                        it = _container.insert(it, OptType{ move<ValueType>(values[i]) });
                    }
                    return UncheckedIterType{ it };
                }

                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, std::initializer_list<OptType> values)
                {
                    return IterType{ _container.insert(pos._iter, std::move(values)) };
                }

                inline constexpr RetType<UncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, std::initializer_list<OptType> values)
                {
                    return UncheckedIterType{ _container.insert(pos._iter, std::move(values)) };
                }

                template<
                    usize len,
                    template<typename, usize> class C1
                >
                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, const StaticOptionalArray<ValueType, len, C1>& values)
                {
                    return insert(pos, values._container.begin(), values._container.end());
                }

                template<
                    usize len,
                    template<typename, usize> class C1
                >
                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, StaticOptionalArray<ValueType, len, C1>&& values)
                {
                    return IterType{ std::move(values._container.begin(), values._container.end(), std::inserter(_container, pos._iter)) };
                }

                template<
                    typename U,
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::convertible_to<U, ValueType>
                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, const StaticOptionalArray<U, len, C1>& values)
                {
                    return insert(pos,
                        boost::make_transform_iterator(values._container.begin(), [](const std::optional<U>& opt) { return map<ValueType>(opt); }),
                        boost::make_transform_iterator(values._container.end(), [](const std::optional<U>& opt) { return map<ValueType>(opt); })
                    );
                }

                template<
                    typename U,
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::convertible_to<U, ValueType>
                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, StaticOptionalArray<U, len, C1>&& values)
                {
                    return insert(pos,
                        boost::make_transform_iterator(values._container.begin(), [](std::optional<U>& opt) { return map<ValueType>(std::move(opt)); }),
                        boost::make_transform_iterator(values._container.end(), [](std::optional<U>& opt) { return map<ValueType>(std::move(opt)); })
                    );
                }

                template<
                    usize len,
                    template<typename, usize> class C1
                >
                inline constexpr RetType<UncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, const StaticOptionalArray<ValueType, len, C1>& values)
                {
                    return insert(pos, values._container.begin(), values._container.end());
                }

                template<
                    usize len,
                    template<typename, usize> class C1
                >
                inline constexpr RetType<UncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, StaticOptionalArray<ValueType, len, C1>&& values)
                {
                    return IterType{ std::move(values._container.begin(), values._container.end(), std::inserter(_container, pos._iter)) };
                }

                template<
                    typename U,
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::convertible_to<U, ValueType>
                inline constexpr RetType<UncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, const StaticOptionalArray<U, len, C1>& values)
                {
                    return insert(pos,
                        boost::make_transform_iterator(values._container.begin(), [](const std::optional<U>& opt) { return map<ValueType>(opt); }),
                        boost::make_transform_iterator(values._container.end(), [](const std::optional<U>& opt) { return map<ValueType>(opt); })
                    );
                }
                 
                template<
                    typename U,
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::convertible_to<U, ValueType>
                inline constexpr RetType<UncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, StaticOptionalArray<U, len, C1>&& values)
                {
                    return insert(pos,
                        boost::make_transform_iterator(values._container.begin(), [](std::optional<U>& opt) { return map<ValueType>(std::move(opt)); }),
                        boost::make_transform_iterator(values._container.end(), [](std::optional<U>& opt) { return map<ValueType>(std::move(opt)); })
                    );
                }

                template<template<typename> class C1>
                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, const DynamicOptionalArray<ValueType, C1>& values)
                {
                    return insert(pos, values._container.begin(), values._container.end());
                }

                template<template<typename> class C1>
                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, DynamicOptionalArray<ValueType, C1>&& values)
                {
                    return IterType{ std::move(values._container.begin(), values._container.end(), std::inserter(_container, pos._iter)) };
                }

                template<
                    typename U,
                    template<typename> class C1
                >
                    requires std::convertible_to<U, ValueType>
                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, const DynamicOptionalArray<U, C1>& values)
                {
                    return insert(pos,
                        boost::make_transform_iterator(values._container.begin(), [](const std::optional<U>& opt) { return map<ValueType>(opt); }),
                        boost::make_transform_iterator(values._container.end(), [](const std::optional<U>& opt) { return map<ValueType>(opt); })
                    );
                }

                template<
                    typename U,
                    template<typename> class C1
                >
                    requires std::convertible_to<U, ValueType>
                inline constexpr RetType<IterType> insert(ArgCLRefType<ConstIterType> pos, DynamicOptionalArray<U, C1>&& values)
                {
                    return insert(pos,
                        boost::make_transform_iterator(values._container.begin(), [](std::optional<U>& opt) { return map<ValueType>(std::move(opt)); }),
                        boost::make_transform_iterator(values._container.end(), [](std::optional<U>& opt) { return map<ValueType>(std::move(opt)); })
                    );
                }

                template<template<typename> class C1>
                inline constexpr RetType<UncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, const DynamicOptionalArray<ValueType, C1>& values)
                {
                    return insert(pos, values._container.begin(), values._container.end());
                }

                template<template<typename> class C1>
                inline constexpr RetType<UncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, DynamicOptionalArray<ValueType, C1>&& values)
                {
                    return IterType{ std::move(values._container.begin(), values._container.end(), std::inserter(_container, pos._iter)) };
                }

                template<
                    typename U,
                    template<typename> class C1
                >
                    requires std::convertible_to<U, ValueType>
                inline constexpr RetType<UncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, const DynamicOptionalArray<U, C1>& values)
                {
                    return insert(pos,
                        boost::make_transform_iterator(values._container.begin(), [](const std::optional<U>& opt) { return map<ValueType>(opt); }),
                        boost::make_transform_iterator(values._container.end(), [](const std::optional<U>& opt) { return map<ValueType>(opt); })
                    );
                }

                template<
                    typename U,
                    template<typename> class C1
                >
                    requires std::convertible_to<U, ValueType>
                inline constexpr RetType<UncheckedIterType> insert(ArgCLRefType<ConstUncheckedIterType> pos, DynamicOptionalArray<U, C1>&& values)
                {
                    return insert(pos,
                        boost::make_transform_iterator(values._container.begin(), [](std::optional<U>& opt) { return map<ValueType>(std::move(opt)); }),
                        boost::make_transform_iterator(values._container.end(), [](std::optional<U>& opt) { return map<ValueType>(std::move(opt)); })
                    );
                }

                inline constexpr RetType<IterType> emplace(ArgCLRefType<ConstIterType> pos, const std::nullopt_t _)
                {
                    return IterType{ _container.emplace(pos._iter, std::nullopt) };
                }

                template<typename... Args>
                    requires std::constructible_from<ValueType, Args...>
                inline constexpr RetType<IterType> emplace(ArgCLRefType<ConstIterType> pos, Args&&... args)
                {
                    return IterType{ _container.emplace(pos._iter, ValueType{ std::forward<Args>(args)... }) };
                }

                inline constexpr RetType<UncheckedIterType> emplace(ArgCLRefType<ConstUncheckedIterType> pos, const std::nullopt_t _)
                {
                    return UncheckedIterType{ _container.emplace(pos._iter, std::nullopt) };
                }

                template<typename... Args>
                    requires std::constructible_from<ValueType, Args...>
                inline constexpr RetType<UncheckedIterType> emplace(ArgCLRefType<ConstUncheckedIterType> pos, Args&&... args)
                {
                    return UncheckedIterType{ _container.emplace(pos._iter, ValueType{ std::forward<Args>(args)... }) };
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

                inline constexpr void push_back(const std::nullopt_t _)
                {
                    _container.push_back(std::nullopt);
                }

                inline constexpr void push_back(ArgCLRefType<ValueType> value)
                {
                    _container.push_back(OptType{ value });
                }

                template<typename = void>
                    requires ReferenceFaster<ValueType> && std::movable<ValueType>
                inline constexpr void push_back(ArgRRefType<ValueType> value)
                {
                    _container.push_back(OptType{ move<ValueType>(value) });
                }

                inline constexpr void push_back(ArgCLRefType<OptType> value)
                {
                    _container.push_back(value);
                }

                template<typename = void>
                    requires ReferenceFaster<OptType> && std::movable<OptType>
                inline constexpr void push_back(ArgRRefType<OptType> value)
                {
                    _container.push_back(value);
                }

                template<
                    usize len,
                    template<typename, usize> class C1
                >
                inline constexpr void push_back(const StaticOptionalArray<ValueType, len, C1>& values)
                {
                    insert(this->end(), values);
                }

                template<
                    usize len,
                    template<typename, usize> class C1
                >
                inline constexpr void push_back(StaticOptionalArray<ValueType, len, C1>&& values)
                {
                    insert(this->end(), std::move(values));
                }

                template<
                    typename U,
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::convertible_to<U, ValueType>
                inline constexpr void push_back(const StaticOptionalArray<U, len, C1>& values)
                {
                    insert(this->end(), values);
                }

                template<
                    typename U,
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::convertible_to<U, ValueType>
                inline constexpr void push_back(StaticOptionalArray<U, len, C1>&& values)
                {
                    insert(this->end(), std::move(values));
                }

                template<template<typename> class C1>
                inline constexpr void push_back(const DynamicOptionalArray<ValueType, C1>& values)
                {
                    insert(this->end(), values);
                }

                template<template<typename> class C1>
                inline constexpr void push_back(DynamicOptionalArray<ValueType, C1>&& values)
                {
                    insert(this->end(), std::move(values));
                }

                template<
                    typename U,
                    template<typename> class C1
                >
                    requires std::convertible_to<U, ValueType>
                inline constexpr void push_back(const DynamicOptionalArray<U, C1>& values)
                {
                    insert(this->end(), values);
                }

                template<
                    typename U,
                    template<typename> class C1
                >
                    requires std::convertible_to<U, ValueType>
                inline constexpr void push_back(DynamicOptionalArray<U, C1>&& values)
                {
                    insert(this->end(), std::move(values));
                }

                inline constexpr LRefType<OptType> emplace_back(const std::nullopt_t _)
                {
                    return _container.emplace_back(std::nullopt);
                }

                template<typename... Args>
                    requires std::constructible_from<ValueType, Args...>
                inline constexpr LRefType<ValueType> emplace_back(Args&&... args)
                {
                    return *_container.emplace_back(ValueType{ std::forward<Args>(args)... });
                }

                inline constexpr RetType<OptType> pop_back(void)
                {
                    auto back = move<OptType>(this->back());
                    _container.pop_back();
                    return back;
                }

                inline constexpr void push_front(const std::nullopt_t _)
                {
                    insert(this->begin(), std::nullopt);
                }

                template<typename = void>
                    requires requires (ContainerType& container) { container.push_front(std::nullopt); }
                inline constexpr void push_front(const std::nullopt_t _)
                {
                    _container.push_front(std::nullopt);
                }

                inline constexpr void push_front(ArgCLRefType<ValueType> value)
                {
                    insert(this->begin(), OptType{ value });
                }

                template<typename = void>
                    requires requires (ContainerType& container) { container.push_front(std::declval<OptType>()); }
                inline constexpr void push_front(ArgCLRefType<ValueType> value)
                {
                    _container.push_front(OptType{ value });
                }

                template<typename = void>
                    requires ReferenceFaster<ValueType> && std::movable<ValueType>
                inline constexpr void push_front(ArgRRefType<ValueType> value)
                {
                    insert(this->begin(), OptType{ move<ValueType>(value) });
                }

                template<typename = void>
                    requires ReferenceFaster<ValueType> && std::movable<ValueType> 
                        && requires (ContainerType& container) { container.push_front(std::declval<OptType>()); }
                inline constexpr void push_front(ArgRRefType<ValueType> value)
                {
                    _container.push_front(OptType{ move<ValueType>(value) });
                }

                inline constexpr void push_front(ArgCLRefType<OptType> value)
                {
                    insert(this->begin(), value);
                }

                template<typename = void>
                    requires requires (ContainerType& container) { container.push_front(std::declval<OptType>()); }
                inline constexpr void push_front(ArgCLRefType<OptType> value)
                {
                    _container.push_front(value);
                }

                template<typename = void>
                    requires ReferenceFaster<OptType> && std::movable<OptType>
                inline constexpr void push_front(ArgRRefType<OptType> value)
                {
                    insert(this->begin(), move<OptType>(value));
                }

                template<typename = void>
                    requires ReferenceFaster<OptType> && std::movable<OptType>
                        && requires (ContainerType& container) { container.push_front(std::declval<OptType>()); }
                inline constexpr void push_front(ArgRRefType<OptType> value)
                {
                    _container.push_front(move<OptType>(value));
                }

                template<
                    usize len,
                    template<typename, usize> class C1
                >
                inline constexpr void push_front(const StaticOptionalArray<ValueType, len, C1>& values)
                {
                    insert(this->begin(), values);
                }

                template<
                    usize len,
                    template<typename, usize> class C1
                >
                inline constexpr void push_front(StaticOptionalArray<ValueType, len, C1>&& values)
                {
                    insert(this->begin(), std::move(values));
                }

                template<
                    typename U,
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::convertible_to<U, ValueType>
                inline constexpr void push_front(const StaticOptionalArray<U, len, C1>& values)
                {
                    insert(this->begin(), values);
                }

                template<
                    typename U,
                    usize len,
                    template<typename, usize> class C1
                >
                    requires std::convertible_to<U, ValueType>
                inline constexpr void push_front(StaticOptionalArray<U, len, C1>&& values)
                {
                    insert(this->begin(), std::move(values));
                }

                template<template<typename> class C1>
                inline constexpr void push_front(const DynamicOptionalArray<ValueType, C1>& values)
                {
                    insert(this->begin(), values);
                }

                template<template<typename> class C1>
                inline constexpr void push_front(DynamicOptionalArray<ValueType, C1>&& values)
                {
                    insert(this->begin(), std::move(values));
                }

                template<
                    typename U,
                    template<typename> class C1
                >
                    requires std::convertible_to<U, ValueType>
                inline constexpr void push_front(const DynamicOptionalArray<U, C1>& values)
                {
                    insert(this->begin(), values);
                }

                template<
                    typename U,
                    template<typename> class C1
                >
                    requires std::convertible_to<U, ValueType>
                inline constexpr void push_front(DynamicOptionalArray<U, C1>&& values)
                {
                    insert(this->begin(), std::move(values));
                }

                inline constexpr LRefType<OptType> emplace_front(const std::nullopt_t _)
                {
                    return *_container.emplace(_container.begin(), std::nullopt);
                }

                template<typename = void>
                    requires requires (ContainerType& container) { container.emplace_front(std::nullopt); }
                inline constexpr LRefType<OptType> emplace_front(const std::nullopt_t _)
                {
                    return _container.emplace_front(std::nullopt);
                }
                
                template<typename... Args>
                    requires std::constructible_from<ValueType, Args...>
                inline constexpr LRefType<ValueType> emplace_front(Args&&... args)
                {
                    return **_container.emplace(_container.begin(), ValueType{ std::forward<Args>(args)... });
                }

                template<typename... Args>
                    requires std::constructible_from<ValueType, Args...>
                        && requires (ContainerType& container) { container.emplace_front(std::declval<ValueType>()); }
                inline constexpr LRefType<ValueType> emplace_front(Args&&... args)
                {
                    return *_container.emplace_front(ValueType{ std::forward<Args>(args)... });
                }

                inline constexpr RetType<OptType> pop_front(void)
                {
                    auto value = move<OptType>(this->front());
                    _container.erase(_container.begin());
                    return value;
                }

                template<typename = void>
                    requires requires (ContainerType& container) { container.pop_front(); }
                inline constexpr RetType<OptType> pop_front(void)
                {
                    auto value = move<OptType>(this->front());
                    _container.pop_front();
                    return value;
                }
                
                template<typename = void>
                    requires std::default_initializable<OptType>
                inline constexpr void resize(const usize length)
                {
                    _container.resize(length);
                }

                template<typename = void>
                    requires (!std::default_initializable<OptType> && WithDefault<OptType> && std::copyable<OptType>)
                inline constexpr void resize(const usize length)
                {
                    _container.resize(length, DefaultValue<OptType>::value());
                }

                template<typename = void>
                    requires std::copyable<OptType>
                inline constexpr void resize(const usize length, ArgCLRefType<ValueType> value)
                {
                    _container.resize(length, OptType{ value });
                }

                template<typename = void>
                    requires std::copyable<OptType> && ReferenceFaster<ValueType> && std::movable<ValueType>
                inline constexpr void resize(const usize length, ArgRRefType<ValueType> value)
                {
                    _container.resize(length, OptType{ move<ValueType>(value) });
                }

                template<typename = void>
                    requires std::copyable<OptType>
                inline constexpr void resize(const usize length, ArgCLRefType<OptType> value)
                {
                    _container.resize(length, value);
                }

            public:
                inline constexpr const bool operator==(const DynamicOptionalArray& rhs) const noexcept
                {
                    return _container == rhs._container;
                }

                inline constexpr const bool operator!=(const DynamicOptionalArray& rhs) const noexcept
                {
                    return _container != rhs._container;
                }

            public:
                inline constexpr const bool operator<(const DynamicOptionalArray& rhs) const noexcept
                {
                    return _container < rhs._container;
                }

                inline constexpr const bool operator<=(const DynamicOptionalArray& rhs) const noexcept
                {
                    return _container <= rhs._container;
                }

                inline constexpr const bool operator>(const DynamicOptionalArray& rhs) const noexcept
                {
                    return _container > rhs._container;
                }

                inline constexpr const bool operator>=(const DynamicOptionalArray& rhs) const noexcept
                {
                    return _container >= rhs._container;
                }

            public:
                inline constexpr decltype(auto) operator<=>(const DynamicOptionalArray& rhs) const noexcept
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
            template<typename, usize> class C = std::array
        >
        using OptArray = optional_array::StaticOptionalArray<OriginType<T>, len, C>;

        template<
            typename T,
            template<typename> class C = std::vector
        >
        using DynOptArray = optional_array::DynamicOptionalArray<OriginType<T>, C>;
    };
};

namespace std
{
    template<typename T, typename C, typename Self>
    inline void swap(ospf::optional_array::OptionalArrayImpl<T, C, Self>& lhs, ospf::optional_array::OptionalArrayImpl<T, C, Self>& rhs) noexcept
    {
        lhs.swap(rhs);
    }
};

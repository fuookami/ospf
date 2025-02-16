#pragma once

#include <ospf/type_family.hpp>
#include <ospf/meta_programming/crtp.hpp>
#include <iterator>

namespace ospf
{
    inline namespace meta_programming
    {
        inline namespace iterator
        {
            template<typename T, std::bidirectional_iterator I, typename Self>
            class BidirectionalIteratorImpl;

            template<typename T, std::random_access_iterator I, typename Self>
            class RandomIteratorImpl;

            template<typename T, std::forward_iterator I, typename Self>
            class ForwardIteratorImpl
            {
                OSPF_CRTP_IMPL

                template<typename T, std::bidirectional_iterator I, typename Self>
                friend class BidirectionalIteratorImpl;

                template<typename T, std::random_access_iterator I, typename Self>
                friend class RandomIteratorImpl;

            public:
                using ValueType = OriginType<T>;
                using IterType = OriginType<I>;
                using RefType = std::conditional_t<std::is_const_v<T>, CLRefType<ValueType>, LRefType<ValueType>>;
                using PtrType = std::conditional_t<std::is_const_v<T>, CPtrType<ValueType>, PtrType<ValueType>>;

            protected:
                constexpr ForwardIteratorImpl(ArgCLRefType<IterType> iter)
                    : _iter(iter) {}
            public:
                constexpr ForwardIteratorImpl(const ForwardIteratorImpl& ano) = default;
                constexpr ForwardIteratorImpl(ForwardIteratorImpl&& ano) noexcept = default;
                constexpr ForwardIteratorImpl& operator=(const ForwardIteratorImpl& rhs) = default;
                constexpr ForwardIteratorImpl& operator=(ForwardIteratorImpl&& rhs) noexcept = default;
                constexpr ~ForwardIteratorImpl(void) noexcept = default;

            public:
                inline constexpr RefType operator*(void) const noexcept
                {
                    return Trait::get(_iter);
                }

                inline constexpr const PtrType operator->(void) const noexcept
                {
                    return &Trait::get(_iter);
                }

            public:
                inline constexpr Self& operator++(void) noexcept
                {
                    ++_iter;
                    return self();
                }

                inline constexpr Self operator++(int) noexcept
                {
                    auto ret = Trait::construct(_iter);
                    ++_iter;
                    return ret;
                }

            public:
                inline constexpr void swap(ForwardIteratorImpl& ano) noexcept
                {
                    return std::swap(_iter, ano._iter);
                }

            public:
                template<typename Iter, typename Other>
                inline constexpr const bool operator==(const ForwardIteratorImpl<T, Iter, Other>& ano) const noexcept
                {
                    return _iter == ano._iter;
                }

                template<typename Iter, typename Other>
                inline constexpr const bool operator!=(const ForwardIteratorImpl<T, Iter, Other>& ano) const noexcept
                {
                    return _iter != ano._iter;
                }

            private:
                struct Trait : public Self
                {
                    inline static constexpr decltype(auto) get(ArgCLRefType<IterType> iter) noexcept
                    {
                        static const auto impl = &Self::OSPF_CRTP_FUNCTION(get);
                        return (*impl)(iter);
                    }

                    inline static constexpr Self construct(ArgCLRefType<IterType> iter) noexcept
                    {
                        static const auto impl = &Self::OSPF_CRTP_FUNCTION(construct);
                        return (*impl)(iter);
                    }
                };

            protected:
                IterType _iter;
            };

            template<typename T, std::bidirectional_iterator I, typename Self>
            class BidirectionalIteratorImpl
                : public ForwardIteratorImpl<T, I, Self>
            {
                using Base = ForwardIteratorImpl<T, I, Self>;
                using typename Base::Trait;

                template<typename T, std::random_access_iterator I, typename Self>
                friend class RandomIteratorImpl;

            public:
                using ValueType = typename Base::ValueType;
                using IterType = typename Base::IterType;
                using RefType = typename Base::RefType;
                using PtrType = typename Base::PtrType;

            protected:
                constexpr BidirectionalIteratorImpl(ArgCLRefType<IterType> iter)
                    : Base(iter) {}
            public:
                constexpr BidirectionalIteratorImpl(const BidirectionalIteratorImpl& ano) = default;
                constexpr BidirectionalIteratorImpl(BidirectionalIteratorImpl&& ano) noexcept = default;
                constexpr BidirectionalIteratorImpl& operator=(const BidirectionalIteratorImpl& rhs) = default;
                constexpr BidirectionalIteratorImpl& operator=(BidirectionalIteratorImpl&& rhs) noexcept = default;
                constexpr ~BidirectionalIteratorImpl(void) noexcept = default;

            public:
                inline constexpr Self& operator--(void) noexcept
                {
                    --this->_iter;
                    return this->self();
                }

                inline constexpr Self operator--(int) noexcept
                {
                    auto ret = Trait::construct(this->_iter);
                    --this->_iter;
                    return ret;
                }
            };

            template<typename T, std::random_access_iterator I, typename Self>
            class RandomIteratorImpl
                : public BidirectionalIteratorImpl<T, I, Self>
            {
                using Base = BidirectionalIteratorImpl<T, I, Self>;
                using typename Base::Trait;

            public:
                using ValueType = typename Base::ValueType;
                using IterType = typename Base::IterType;
                using RefType = typename Base::RefType;
                using PtrType = typename Base::PtrType;

            protected:
                constexpr RandomIteratorImpl(ArgCLRefType<IterType> iter)
                    : Base(iter) {}
            public:
                constexpr RandomIteratorImpl(const RandomIteratorImpl& ano) = default;
                constexpr RandomIteratorImpl(RandomIteratorImpl&& ano) noexcept = default;
                constexpr RandomIteratorImpl& operator=(const RandomIteratorImpl& rhs) = default;
                constexpr RandomIteratorImpl& operator=(RandomIteratorImpl&& rhs) noexcept = default;
                constexpr ~RandomIteratorImpl(void) noexcept = default;

            public:
                inline constexpr Self operator+(const ptrdiff diff) const noexcept
                {
                    return Trait::construct(this->_iter + diff);
                }

                inline constexpr Self& operator+=(const ptrdiff diff) noexcept
                {
                    this->_iter += diff;
                    return this->self();
                }

                inline constexpr Self operator-(const ptrdiff diff) const noexcept
                {
                    return Trait::construct(this->_iter - diff);
                }

                inline constexpr Self& operator-=(const ptrdiff diff) noexcept
                {
                    this->_iter -= diff;
                    return this->self();
                }

            public:
                template<typename Iter, typename Other>
                inline constexpr const bool operator<(const RandomIteratorImpl<T, Iter, Other>& ano) const noexcept
                {
                    return this->_iter < ano._iter;
                }

                template<typename Iter, typename Other>
                inline constexpr const bool operator<=(const RandomIteratorImpl<T, Iter, Other>& ano) const noexcept
                {
                    return this->_iter <= ano._iter;
                }

                template<typename Iter, typename Other>
                inline constexpr const bool operator>(const RandomIteratorImpl<T, Iter, Other>& ano) const noexcept
                {
                    return this->_iter > ano._iter;
                }

                template<typename Iter, typename Other>
                inline constexpr const bool operator>=(const RandomIteratorImpl<T, Iter, Other>& ano) const noexcept
                {
                    return this->_iter >= ano._iter;
                }

            public:
                template<typename Iter, typename Other>
                inline constexpr decltype(auto) operator<=>(const RandomIteratorImpl<T, Iter, Other>& ano) const noexcept
                {
                    return this->_iter <=> ano._iter;
                }
            };
        };
    };
};

namespace std
{
    template<typename T, typename I, typename Self>
    inline constexpr void swap(ospf::ForwardIteratorImpl<T, I, Self>& lhs, ospf::ForwardIteratorImpl<T, I, Self>& rhs) noexcept
    {
        return lhs.swap(rhs);
    }
};

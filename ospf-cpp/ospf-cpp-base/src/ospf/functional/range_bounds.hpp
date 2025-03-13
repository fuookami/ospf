#pragma once

#include <ospf/functional/integer_iterator.hpp>
#include <ospf/memory/reference.hpp>
#include <variant>

namespace ospf
{
    inline namespace functional
    {
        struct Unbounded {};
        static constexpr const auto unbounded = Unbounded{};

        template<typename I>
        class Bound
        {
        public:
            using IndexType = OriginType<I>;

            struct Included 
            {
                IndexType value;

                inline constexpr operator LRefType<IndexType>(void) noexcept
                {
                    return value;
                }

                inline constexpr operator CLRefType<IndexType>(void) noexcept
                {
                    return value;
                }
            };

            struct Excluded
            {
                IndexType value;

                inline constexpr operator LRefType<IndexType>(void) noexcept
                {
                    return value;
                }

                inline constexpr operator CLRefType<IndexType>(void) noexcept
                {
                    return value;
                }
            };

            using VariantType = std::variant<Included, Excluded, Unbounded>;

        public:
            constexpr Bound(const Unbounded _ = ospf::unbounded)
                : _variant(ospf::unbounded) {}

            constexpr Bound(ArgCLRefType<Included> included)
                : _variant(included) {}

            template<typename = void>
                requires ReferenceFaster<Included> && std::movable<Included>
            constexpr Bound(ArgRRefType<Included> included)
                : _variant(move<Included>(included)) {}

            constexpr Bound(ArgCLRefType<Excluded> excluded)
                : _variant(excluded) {}

            template<typename = void>
                requires ReferenceFaster<Excluded> && std::movable<Excluded>
            constexpr Bound(ArgRRefType<Excluded> excluded)
                : _variant(move<Excluded>(excluded)) {}

        public:
            constexpr Bound(const Bound& ano) = default;
            constexpr Bound(Bound&& ano) noexcept = default;
            constexpr Bound& operator=(const Bound& rhs) = default;
            constexpr Bound& operator=(Bound&& rhs) noexcept = default;
            constexpr ~Bound(void) noexcept = default;

        public:
            inline constexpr const bool inclusive(void) const noexcept
            {
                return _variant.index() == 0_uz;
            }

            inline constexpr const bool exclusive(void) const noexcept
            {
                return _variant.index() == 1_uz;
            }

            inline constexpr const bool unbounded(void) const noexcept
            {
                return _variant.index() == 2_uz;
            }

        public:
            inline constexpr ArgCLRefType<IndexType> operator*(void) const
            {
                return std::visit([](const auto& arg)
                    {
                        using Type = OriginType<decltype(arg)>;
                        if constexpr (std::is_same_v<Type, Included> || std::is_same_v<Type, Excluded>)
                        {
                            return arg.value;
                        }
                        else
                        {
                            throw OSPFException{ OSPFErrCode::ApplicationError, "unbounded accessed" };
                            return IndexType{ };
                        }
                    }, _variant);
            }

        private:
            VariantType _variant;
        };

        template<typename I>
        class RangeBounds;

        template<typename I>
        class RangeBoundsReverseWrapper;

        template<typename R>
        concept RangeBoundsType = requires (const R& r)
        {
            { r.empty() } -> DecaySameAs<bool>;
            { r.start_bound() } -> DecaySameAs<typename R::BoundType>;
            { r.end_bound() } -> DecaySameAs<typename R::BoundType>;
            { r.contains(std::declval<typename R::IndexType>()) } -> DecaySameAs<bool>;
        };

        template<typename R>
        concept IntegerRangeBoundsType = RangeBoundsType<R> && std::integral<typename R::IndexType>
            && requires (const R& r)
        {
            { r.begin() } -> std::forward_iterator;
            { r.end() } -> std::forward_iterator;
        };

        template<typename I>
            requires std::integral<I>
        class IntegerRangeBoundsWithStepWrapper
        {
            friend class RangeBounds<I>;
            friend class RangeBoundsReverseWrapper<I>;

        public:
            using RangeBoundsType = RangeBounds<I>;
            using IndexType = typename RangeBoundsType::IndexType;
            using BoundType = typename RangeBoundsType::BoundType;
            using IncludedType = typename RangeBoundsType::IncludedType;
            using ExcludedType = typename RangeBoundsType::ExcludedType;

        private:
            constexpr IntegerRangeBoundsWithStepWrapper(const RangeBoundsType& bounds, ArgRRefType<IndexType> step, const bool reverse = false)
                : _reverse(reverse), _bounds(bounds), _step(move<IndexType>(step)) {}

        public:
            constexpr IntegerRangeBoundsWithStepWrapper(const IntegerRangeBoundsWithStepWrapper& ano) = default;
            constexpr IntegerRangeBoundsWithStepWrapper(IntegerRangeBoundsWithStepWrapper&& ano) noexcept = default;
            constexpr IntegerRangeBoundsWithStepWrapper& operator=(const IntegerRangeBoundsWithStepWrapper& rhs) = default;
            constexpr IntegerRangeBoundsWithStepWrapper& operator=(IntegerRangeBoundsWithStepWrapper&& rhs) noexcept = default;
            constexpr ~IntegerRangeBoundsWithStepWrapper(void) noexcept = default;

        public:
            template<typename = void>
                requires requires
                    {
                        { static_cast<IndexType>(0) } -> DecaySameAs<IndexType>;
                        { static_cast<IndexType>(1) } -> DecaySameAs<IndexType>;
                        { static_cast<usize>(std::declval<IndexType>()) } -> DecaySameAs<usize>;
                        { std::declval<IndexType>() - std::declval<IndexType>() } -> DecaySameAs<IndexType>;
                        { std::declval<IndexType>() + std::declval<IndexType>() } -> DecaySameAs<IndexType>;
                    }
            inline constexpr const usize size(void) const
            {
                return _bounds->size() / static_cast<usize>(_step);
            }

            template<typename = void>
                requires requires
                    {
                        { static_cast<IndexType>(0) } -> DecaySameAs<IndexType>;
                        { static_cast<IndexType>(1) } -> DecaySameAs<IndexType>;
                        { static_cast<usize>(std::declval<IndexType>()) } -> DecaySameAs<usize>;
                        { std::declval<IndexType>() - std::declval<IndexType>() } -> DecaySameAs<IndexType>;
                        { std::declval<IndexType>() + std::declval<IndexType>() } -> DecaySameAs<IndexType>;
                    }
            inline constexpr const usize size(ArgCLRefType<IndexType> lb, ArgCLRefType<IndexType> ub) const noexcept
            {
                return _bounds->size(lb, ub) / static_cast<usize>(_step);
            }

        public:
            template<typename = void>
                requires requires
                    {
                        { std::declval<IndexType>() + std::declval<IndexType>() } -> DecaySameAs<IndexType>;
                    }
            inline constexpr RetType<IntegerIterator<IndexType>> begin(void) const noexcept
            {
                return IntegerIterator<IndexType>
                { 
                    _bounds->start_bound().exclusive() ? (*_bounds->start_bound() + _step) : *_bounds->start_bound(),
                    _bounds->end_bound().inclusive() ? (*_bounds->end_bound() + _step) : *_bounds->end_bound(),
                    _step, 
                    _reverse
                };
            }

            template<typename = void>
                requires requires
                    {
                        { std::declval<IndexType>() + std::declval<IndexType>() } -> DecaySameAs<IndexType>;
                    }
            inline constexpr RetType<IntegerIterator<IndexType>> begin(ArgRRefType<IndexType> lb, ArgRRefType<IndexType> ub) const noexcept
            {
                if (_bounds->start_bound().unbounded() && _bounds->end_bound().unbounded())
                {
                    return IntegerIterator<IndexType>{ move<IndexType>(lb), move<IndexType>(ub), _step, _reverse };
                }
                else if (_bounds->start_bound().unbounded())
                {
                    if (ub < *_bounds->end_bound())
                    {
                        return IntegerIterator<IndexType>{ move<IndexType>(lb), move<IndexType>(ub), _step, _reverse };
                    }
                    else
                    {
                        return IntegerIterator<IndexType>{ move<IndexType>(lb), _bounds->end_bound().inclusive() ? (*_bounds->end_bound() + _step) : *_bounds->end_bound(), _step, _reverse };
                    }
                }
                else if (_bounds->end_bound().unbounded())
                {
                    if (lb > *_bounds->start_bound())
                    {
                        return IntegerIterator<IndexType>{ move<IndexType>(lb), move<IndexType>(ub), _step, _reverse };
                    }
                    else
                    {
                        return IntegerIterator<IndexType>{ _bounds->start_bound().exclusive() ? (*_bounds->start_bound() + _step) : *_bounds->start_bound(), move<IndexType>(ub), _step, _reverse };
                    }
                }
                else
                {
                    return IntegerIterator<IndexType>
                    {
                        _bounds->start_bound().exclusive() ? (*_bounds->start_bound() + _step) : *_bounds->start_bound(),
                        _bounds->end_bound().inclusive() ? (*_bounds->end_bound() + _step) : *_bounds->end_bound(),
                        _step,
                        _reverse
                    };
                }
            }

            inline constexpr RetType<IntegerIterator<IndexType>> end(void) const noexcept
            {
                return IntegerIterator<IndexType>{};
            }

            inline constexpr IntegerRangeBoundsWithStepWrapper reverse(void) const noexcept
            {
                return IntegerRangeBoundsWithStepWrapper{ *_bounds, _step, !_reverse };
            }

        private:
            bool _reverse;
            IndexType _step;
            Ref<RangeBoundsType> _bounds;
        };

        template<typename I>
        class RangeBoundsReverseWrapper
        {
            friend class RangeBounds<I>;

        public:
            using RangeBoundsType = RangeBounds<I>;
            using IndexType = typename RangeBoundsType::IndexType;
            using BoundType = typename RangeBoundsType::BoundType;
            using IncludedType = typename RangeBoundsType::IncludedType;
            using ExcludedType = typename RangeBoundsType::ExcludedType;

        private:
            constexpr RangeBoundsReverseWrapper(const RangeBoundsType& bounds)
                : _bounds(bounds) {}

        public:
            constexpr RangeBoundsReverseWrapper(const RangeBoundsReverseWrapper& ano) = default;
            constexpr RangeBoundsReverseWrapper(RangeBoundsReverseWrapper&& ano) noexcept = default;
            constexpr RangeBoundsReverseWrapper& operator=(const RangeBoundsReverseWrapper& rhs) = default;
            constexpr RangeBoundsReverseWrapper& operator=(RangeBoundsReverseWrapper&& rhs) noexcept = default;
            constexpr ~RangeBoundsReverseWrapper(void) noexcept = default;

        public:
            inline constexpr const bool empty(void) const noexcept
            {
                return _bounds->empty();
            }

            inline constexpr ArgCLRefType<BoundType> start_bound(void) const noexcept
            {
                return _bounds->end_bound();
            }

            inline constexpr ArgCLRefType<BoundType> end_bound(void) const noexcept
            {
                return _bounds->start_bound();
            }

            template<typename = void>
                requires std::three_way_comparable<IndexType> || std::totally_ordered<IndexType>
            inline constexpr const bool contains(ArgCLRefType<IndexType> value) const noexcept
            {
                return _bounds.contains(value);
            }
        
        public:
            template<typename = void>
                requires std::integral<IndexType>
                    && requires
                    {
                        { static_cast<IndexType>(0) } -> DecaySameAs<IndexType>;
                        { static_cast<IndexType>(1) } -> DecaySameAs<IndexType>;
                        { static_cast<usize>(std::declval<IndexType>()) } -> DecaySameAs<usize>;
                        { std::declval<IndexType>() - std::declval<IndexType>() } -> DecaySameAs<IndexType>;
                        { std::declval<IndexType>() + std::declval<IndexType>() } -> DecaySameAs<IndexType>;
                    }
            inline constexpr const usize size(void) const
            {
                return _bounds->size();
            }

            template<typename = void>
                requires std::integral<IndexType>
                    && requires
                    {
                        { static_cast<IndexType>(0) } -> DecaySameAs<IndexType>;
                        { static_cast<IndexType>(1) } -> DecaySameAs<IndexType>;
                        { static_cast<usize>(std::declval<IndexType>()) } -> DecaySameAs<usize>;
                        { std::declval<IndexType>() - std::declval<IndexType>() } -> DecaySameAs<IndexType>;
                        { std::declval<IndexType>() + std::declval<IndexType>() } -> DecaySameAs<IndexType>;
                    }
            inline constexpr const usize size(ArgCLRefType<IndexType> lb, ArgCLRefType<IndexType> ub) const noexcept
            {
                return _bounds->size(lb, ub);
            }

        public:
            template<typename = void>
                requires std::integral<IndexType>
                    && requires
                    {
                        { static_cast<IndexType>(1) } -> DecaySameAs<IndexType>;
                    }
            inline constexpr RetType<IntegerIterator<IndexType>> begin(void) const noexcept
            {
                static const auto one = static_cast<IndexType>(1);

                return IntegerIterator<IndexType>
                { 
                    _bounds->start_bound().exclusive() ? (*_bounds->start_bound() + one) : *_bounds->start_bound(),
                    _bounds->end_bound().inclusive() ? (*_bounds->end_bound() + one) : *_bounds->end_bound(),
                    one,
                    true
                };
            }

            template<typename = void>
                requires std::integral<IndexType>
                    && requires
                    {
                        { static_cast<IndexType>(1) } -> DecaySameAs<IndexType>;
                    }
            inline constexpr RetType<IntegerIterator<IndexType>> begin(ArgRRefType<IndexType> lb, ArgRRefType<IndexType> ub) const noexcept
            {
                static const auto one = static_cast<IndexType>(1);

                if (_bounds->start_bound().unbounded() && _bounds->end_bound().unbounded())
                {
                    return IntegerIterator<IndexType>{ move<IndexType>(lb), move<IndexType>(ub), one, true };
                }
                else if (_bounds->start_bound().unbounded())
                {
                    if (ub < *_bounds->end_bound())
                    {
                        return IntegerIterator<IndexType>{ move<IndexType>(lb), move<IndexType>(ub), one, true };
                    }
                    else
                    {
                        return IntegerIterator<IndexType>{ move<IndexType>(lb), _bounds->end_bound().inclusive() ? (*_bounds->end_bound() + one) : *_bounds->end_bound(), one, true };
                    }
                }
                else if (_bounds->end_bound().unbounded())
                {
                    if (lb > *_bounds->start_bound())
                    {
                        return IntegerIterator<IndexType>{ move<IndexType>(lb), move<IndexType>(ub), one, true };
                    }
                    else
                    {
                        return IntegerIterator<IndexType>{ _bounds->start_bound().exclusive() ? (*_bounds->start_bound() + one) : *_bounds->start_bound(), move<IndexType>(ub), one, true };
                    }
                }
                else
                {
                    return IntegerIterator<IndexType>
                    {
                        _bounds->start_bound().exclusive() ? (*_bounds->start_bound() + one) : *_bounds->start_bound(),
                        _bounds->end_bound().inclusive() ? (*_bounds->end_bound() + one) : *_bounds->end_bound(),
                        one,
                        true
                    };
                }
            }

            template<typename = void>
                requires std::integral<IndexType>
            inline constexpr RetType<IntegerIterator<IndexType>> end(void) const noexcept
            {
                return IntegerIterator<IndexType>{ };
            }

        public:
            inline constexpr RetType<IntegerRangeBoundsWithStepWrapper<IndexType>> step(ArgRRefType<IndexType> step) const noexcept
            {
                return IntegerRangeBoundsWithStepWrapper<IndexType>{ *this, move<IndexType>(step), true };
            }

        private:
            Ref<RangeBoundsType> _bounds;
        };

        template<typename I>
        class RangeBounds
        {
        public:
            using IndexType = OriginType<I>;
            using BoundType = Bound<IndexType>;
            using IncludedType = typename BoundType::Included;
            using ExcludedType = typename BoundType::Excluded;

            //tex:$(- \infty , \infty)$
            inline static constexpr RangeBounds full(void) noexcept
            {
                return RangeBounds{ BoundType{ functional::unbounded }, BoundType{ functional::unbounded } };
            }

            //tex:$[L, \infty )$
            inline static constexpr RangeBounds from(ArgRRefType<IndexType> start_bound) noexcept
            {
                return RangeBounds{ BoundType{ IncludedType{ move<IndexType>(start_bound) } }, BoundType{ functional::unbounded } };
            }

            //tex:$(- \infty , R)$
            inline static constexpr RangeBounds to(ArgRRefType<IndexType> end_bound) noexcept
            {
                return RangeBounds{ BoundType{ functional::unbounded }, BoundType{ ExcludedType{ move<IndexType>(end_bound) } } };
            }

            //tex:$[L, R)$
            inline static constexpr RangeBounds range(ArgRRefType<IndexType> start_bound, ArgRRefType<IndexType> end_bound) noexcept
            {
                return RangeBounds{ BoundType{ IncludedType{ move<IndexType>(start_bound) } }, BoundType{ ExcludedType{ move<IndexType>(end_bound) } } };
            }

            //tex:$[L, R]$
            inline static constexpr RangeBounds inclusive(ArgRRefType<IndexType> start_bound, ArgRRefType<IndexType> end_bound) noexcept
            {
                return RangeBounds{ BoundType{ IncludedType{ move<IndexType>(start_bound) } }, BoundType{ IncludedType{ move<IndexType>(end_bound) } } };
            }

            //tex:$(- \infty, R]$
            inline static constexpr RangeBounds to_inclusive(ArgRRefType<IndexType> end_bound) noexcept
            {
                return RangeBounds{ BoundType{ functional::unbounded }, BoundType{ IncludedType{ move<IndexType>(end_bound) } } };
            }

        private:
            constexpr RangeBounds(ArgRRefType<BoundType> start_bound, ArgRRefType<BoundType> end_bound)
                : _start_bound(move<BoundType>(start_bound)), _end_bound(move<BoundType>(end_bound)) {}

        public:
            constexpr RangeBounds(const RangeBounds& ano) = default;
            constexpr RangeBounds(RangeBounds&& ano) noexcept = default;
            constexpr RangeBounds& operator=(const RangeBounds& rhs) = default;
            constexpr RangeBounds& operator=(RangeBounds&& rhs) noexcept = default;
            constexpr ~RangeBounds(void) noexcept = default;

        public:
            inline constexpr const bool unbounded(void) const noexcept
            {
                return _start_bound.unbounded() || _end_bound.unbounded();
            }

            inline constexpr const bool empty(void) const noexcept
            {
                if (unbounded())
                {
                    return false;
                }
                else if (_start_bound.inclusive() && _start_bound.inclusive())
                {
                    return *_start_bound > *_end_bound;
                }
                else
                {
                    return *_start_bound >= *_end_bound;
                }
            }

            inline constexpr ArgCLRefType<Bound<IndexType>> start_bound(void) const noexcept
            {
                return _start_bound;
            }

            inline constexpr ArgCLRefType<Bound<IndexType>> end_bound(void) const noexcept
            {
                return _end_bound;
            }

            template<typename = void>
                requires std::three_way_comparable<IndexType> || std::totally_ordered<IndexType>
            inline constexpr const bool contains(ArgCLRefType<IndexType> value) const noexcept
            {
                return in_start_bound(value) && in_end_bound(value);
            }

            template<typename = void>
                requires std::integral<IndexType>
                    && requires
                    {
                        { static_cast<IndexType>(0) } -> DecaySameAs<IndexType>;
                        { static_cast<IndexType>(1) } -> DecaySameAs<IndexType>;
                        { static_cast<usize>(std::declval<IndexType>()) } -> DecaySameAs<usize>;
                        { std::declval<IndexType>() - std::declval<IndexType>() } -> DecaySameAs<IndexType>;
                        { std::declval<IndexType>() + std::declval<IndexType>() } -> DecaySameAs<IndexType>;
                    }
            inline constexpr const usize size(void) const
            {
                return static_cast<usize>(*_end_bound - *_start_bound - static_cast<IndexType>(static_cast<int>(_start_bound.exclusive())) + static_cast<IndexType>(static_cast<int>(_end_bound.inclusive())));
            }

            template<typename = void>
                requires std::integral<IndexType>
                    && requires
                    {
                        { static_cast<IndexType>(0) } -> DecaySameAs<IndexType>;
                        { static_cast<IndexType>(1) } -> DecaySameAs<IndexType>;
                        { static_cast<usize>(std::declval<IndexType>()) } -> DecaySameAs<usize>;
                        { std::declval<IndexType>() - std::declval<IndexType>() } -> DecaySameAs<IndexType>;
                        { std::declval<IndexType>() + std::declval<IndexType>() } -> DecaySameAs<IndexType>;
                    }
            inline constexpr const usize size(ArgCLRefType<IndexType> lb, ArgCLRefType<IndexType> ub) const noexcept
            {
                if (_start_bound.unbounded() && _end_bound.unbounded())
                {
                    return ub - lb;
                }
                else if (_start_bound.unbounded())
                {
                    if (ub < *_end_bound)
                    {
                        return ub - lb;
                    }
                    else
                    {
                        return *_end_bound - lb 
                            + static_cast<IndexType>(static_cast<int>(_end_bound.inclusive()));
                    }
                }
                else if (_end_bound.unbounded())
                {
                    if (lb > *_start_bound)
                    {
                        return ub - lb;
                    }
                    else
                    {
                        return ub - *_start_bound 
                            - static_cast<IndexType>(static_cast<int>(_start_bound.exclusive()));
                    }
                }
                else
                {
                    return *_end_bound - *_start_bound
                        - static_cast<IndexType>(static_cast<int>(_start_bound.exclusive())) 
                        + static_cast<IndexType>(static_cast<int>(_end_bound.inclusive()));
                }
            }

        public:
            inline constexpr RangeBoundsReverseWrapper<IndexType> reverse(void) const noexcept
            {
                return RangeBoundsReverseWrapper<IndexType>{ *this };
            }

            inline constexpr IntegerRangeBoundsWithStepWrapper<IndexType> step(ArgRRefType<IndexType> step) const noexcept
            {
                return IntegerRangeBoundsWithStepWrapper<IndexType>{ *this, move<IndexType>(step) };
            }

        public:
            template<typename = void>
                requires std::integral<IndexType>
                    && requires
                    {
                        { static_cast<IndexType>(1) } -> DecaySameAs<IndexType>;
                        { std::declval<IndexType>() + std::declval<IndexType>() } -> DecaySameAs<IndexType>;
                    }
            inline constexpr RetType<IntegerIterator<IndexType>> begin(void) const noexcept
            {
                static const auto one = static_cast<IndexType>(1);

                return IntegerIterator<IndexType>
                { 
                    _start_bound.exclusive() ? (*_start_bound + one) : *_start_bound,
                    _end_bound.inclusive() ? (*_end_bound + one) : *_end_bound,
                    one
                };
            }

            template<typename = void>
                requires std::integral<IndexType>
                    && requires
                    {
                        { static_cast<IndexType>(1) } -> DecaySameAs<IndexType>;
                        { std::declval<IndexType>() + std::declval<IndexType>() } -> DecaySameAs<IndexType>;
                    }
            inline constexpr RetType<IntegerIterator<IndexType>> begin(ArgRRefType<IndexType> lb, ArgRRefType<IndexType> ub) const noexcept
            {
                static const auto one = static_cast<IndexType>(1);

                if (_start_bound.unbounded() && _end_bound.unbounded())
                {
                    return IntegerIterator<IndexType>{ move<IndexType>(lb), move<IndexType>(ub), one };
                }
                else if (_start_bound.unbounded())
                {
                    if (ub < *_end_bound)
                    {
                        return IntegerIterator<IndexType>{ move<IndexType>(lb), move<IndexType>(ub), one };
                    }
                    else
                    {
                        return IntegerIterator<IndexType>{ move<IndexType>(lb), _end_bound.inclusive() ? (*_end_bound + one) : *_end_bound, one };
                    }
                }
                else if (_end_bound.unbounded())
                {
                    if (lb > *_start_bound)
                    {
                        return IntegerIterator<IndexType>{ move<IndexType>(lb), move<IndexType>(ub), one };
                    }
                    else
                    {
                        return IntegerIterator<IndexType>{ _start_bound.exclusive() ? (*_start_bound + one) : *_start_bound, move<IndexType>(ub), one };
                    }
                }
                else
                {
                    return IntegerIterator<IndexType>
                    {
                        _start_bound.exclusive() ? (*_start_bound + one) : *_start_bound,
                        _end_bound.inclusive() ? (*_end_bound + one) : *_end_bound,
                        one
                    };
                }
            }

            template<typename = void>
                requires std::integral<IndexType>
            inline constexpr RetType<IntegerIterator<IndexType>> end(void) const noexcept
            {
                return IntegerIterator<IndexType>{ };
            }

        private:
            inline constexpr const bool in_start_bound(ArgCLRefType<IndexType> value) const noexcept
            {
                if (_start_bound.inclusive())
                {
                    return *_start_bound <= value;
                }
                else if (_start_bound.exclusive())
                {
                    return *_start_bound < value;
                }
                else
                {
                    return true;
                }
            }

            inline constexpr const bool in_end_bound(ArgCLRefType<IndexType> value) const noexcept
            {
                if (_end_bound.inclusive())
                {
                    return value <= *_end_bound;
                }
                else if (_end_bound.exclusive())
                {
                    return value < *_end_bound;
                }
                else
                {
                    return true;
                }
            }

        private:
            BoundType _start_bound;
            BoundType _end_bound;
        };

        extern template class Bound<i32>;
        extern template class Bound<u32>;
        extern template class Bound<usize>;
        extern template class Bound<isize>;
        extern template class RangeBounds<i32>;
        extern template class RangeBounds<u32>;
        extern template class RangeBounds<usize>;
        extern template class RangeBounds<isize>;

        // DSL:
        // 1.full:         _ To _    -> (-¡Þ, ¡Þ)
        // 2.from:         a To _    -> [a,   ¡Þ)
        // 3.to:           _ To b    -> (-¡Þ, b)
        // 4.range:        a To b    -> [a,   b)
        // 5.inclusive:    a Until b -> [a,   b]
        // 6.to_inclusive: _ Until b -> (-¡Þ, b]

        namespace range_bounds
        {
            struct RangeFull {};

            struct RangeTo
            {
                template<typename T>
                inline constexpr decltype(auto) operator()(const Unbounded lhs, T&& rhs) const noexcept
                {
                    if constexpr (DecaySameAs<T, Unbounded>)
                    {
                        return RangeBounds<usize>::full();
                    }
                    else
                    {
                        return RangeBounds<OriginType<T>>::to(std::forward<T>(rhs));
                    }
                }
            };

            struct RangeFrom
            {
                template<typename T, typename U>
                    requires DecayNotSameAs<T, Unbounded> && (DecaySameAs<U, Unbounded> || DecaySameAs<U, T>)
                inline constexpr RetType<RangeBounds<OriginType<T>>> operator()(T&& lhs, U&& rhs) const noexcept
                {
                    if constexpr (DecaySameAs<U, Unbounded>)
                    {
                        return RangeBounds<OriginType<T>>::from(std::forward<T>(lhs));
                    }
                    else if constexpr (DecaySameAs<U, T>)
                    {
                        if constexpr (CopyFaster<T>)
                        {
                            return RangeBounds<OriginType<T>>::range(lhs, rhs);
                        }
                        else
                        {
                            return RangeBounds<OriginType<T>>::range(move<T>(lhs), move<T>(rhs));
                        }
                    }
                }
            };
            static constexpr const auto from = ospf::range_bounds::RangeFrom{};

            struct RangeUntil
            {
                template<typename T>
                    requires DecayNotSameAs<T, Unbounded>
                inline constexpr RetType<RangeBounds<OriginType<T>>> operator()(const Unbounded lhs, T&& rhs) const noexcept
                {
                    return RangeBounds<OriginType<T>>::to_inclusive(std::forward<T>(rhs));
                }
            };

            struct RangeIn
            {
                template<typename T>
                    requires DecayNotSameAs<T, Unbounded>
                inline constexpr RetType<RangeBounds<OriginType<T>>> operator()(T&& lhs, T&& rhs) const noexcept
                {
                    return RangeBounds<OriginType<T>>::inclusive(std::forward<T>(lhs), std::forward<T>(rhs));
                }
            };
            static constexpr const auto in = ospf::range_bounds::RangeIn{};
        };
    };
};

static constexpr const auto _ = ospf::Unbounded{};
static constexpr const auto __ = ospf::range_bounds::RangeFull{};
static constexpr const auto to = ospf::range_bounds::RangeTo{};
static constexpr const auto until = ospf::range_bounds::RangeUntil{};

inline static constexpr decltype(auto) operator<(const ospf::Unbounded lhs, const ospf::range_bounds::RangeTo rhs)
{
    return [](auto&& rhs)
    {
        return to(_, rhs);
    };
}

template<typename T>
inline static constexpr decltype(auto) operator<(T&& lhs, const ospf::range_bounds::RangeTo rhs)
{
    return [&lhs](auto&& rhs)
    {
        return ospf::range_bounds::from(lhs, rhs);
    };
}

inline static constexpr decltype(auto) operator<(const ospf::Unbounded lhs, const ospf::range_bounds::RangeUntil rhs)
{
    return [](auto&& rhs)
    {
        return until(_, rhs);
    };
}

template<typename T>
inline static constexpr decltype(auto) operator<(T&& lhs, const ospf::range_bounds::RangeUntil rhs)
{
    return [&lhs](auto&& rhs)
    {
        return ospf::range_bounds::in(lhs, rhs);
    };
}

template<typename F>
    requires std::invocable<F, ospf::Unbounded>
inline static constexpr decltype(auto) operator>(F&& lhs, const ospf::Unbounded rhs)
{
    return lhs(_);
}

template<typename F, typename T>
    requires std::invocable<F, T>
inline static constexpr decltype(auto) operator>(F&& lhs, T&& rhs)
{
    return lhs(rhs);
}

#ifndef RTo
#define RTo <to>
#endif

#ifndef RUntil
#define RUntil <until>
#endif

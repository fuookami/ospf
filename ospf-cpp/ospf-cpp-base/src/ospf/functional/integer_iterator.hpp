#pragma once

#include <ospf/concepts/base.hpp>
#include <ospf/exception.hpp>
#include <ospf/type_family.hpp>

namespace ospf
{
    inline namespace functional
    {
        template<typename T>
            requires std::integral<T>
                && requires
                { 
                    { static_cast<T>(0) } -> DecaySameAs<T>;
                    { static_cast<T>(1) } -> DecaySameAs<T>;
                    { -std::declval<T>() } -> DecaySameAs<T>;
                }
        class IntegerIterator
        {
        public:
            using ValueType = OriginType<T>;

        public:
            constexpr IntegerIterator(void)
                : _has_next(false), _curr(static_cast<ValueType>(0)), _last(static_cast<ValueType>(0)), _step(static_cast<ValueType>(0)) {}

            constexpr IntegerIterator(ArgRRefType<ValueType> curr, ArgRRefType<ValueType> last, ArgRRefType<ValueType> step, const bool reverse = false)
                : _has_next(true), _curr(static_cast<ValueType>(0)), _last(static_cast<ValueType>(0)), _step(reverse ? move<ValueType>(step) : -move<ValueType>(step))
            {
                static const auto zero = static_cast<ValueType>(0);

                if (curr == last)
                {
                    _has_next = false;
                }
                else
                {
                    if (step > zero)
                    {
                        if (curr > last)
                        {
                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid integer range for iterator" };
                        }
                    }
                    else if (step < zero)
                    {
                        if (curr < last)
                        {
                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid integer range for iterator" };
                        }
                    }
                    else
                    {
                        throw OSPFException{ OSPFErrCode::ApplicationError, "invalid step 0 for integer iterator" };
                    }

                    if (reverse)
                    {
                        _curr = move<ValueType>(last) - step;
                        _last = move<ValueType>(curr) - step;
                        _step = -move<ValueType>(step);
                    }
                    else
                    {
                        _curr = move<ValueType>(curr);
                        _last = move<ValueType>(last);
                        _step = move<ValueType>(step);
                    }
                }
            }

        public:
            constexpr IntegerIterator(const IntegerIterator& ano) = default;
            constexpr IntegerIterator(IntegerIterator&& ano) noexcept = default;
            constexpr IntegerIterator& operator=(const IntegerIterator& rhs) = default;
            constexpr IntegerIterator& operator=(IntegerIterator&& rhs) noexcept = default;
            constexpr ~IntegerIterator(void) noexcept = default;

        public:
            inline constexpr operator const bool(void) const noexcept
            {
                return _has_next;
            }

        public:
            inline constexpr ArgCLRefType<ValueType> operator*(void) const noexcept
            {
                return _curr;
            }

            inline constexpr const CPtrType<ValueType> operator->(void) const noexcept
            {
                return &_curr;
            }

        public:
            inline constexpr IntegerIterator& operator++(void) noexcept
            {
                next();
                return *this;
            }

            inline constexpr const IntegerIterator operator++(int) noexcept
            {
                auto ret = *this;
                next();
                return ret;
            }

        public:
            inline constexpr const bool operator==(const IntegerIterator& ano) const noexcept
            {
                if (!_has_next && !ano._has_next)
                {
                    return true;
                }
                else if (_has_next && ano._has_next)
                {
                    return _curr == ano._curr
                        && _last == ano._last
                        && _step == ano._step;
                }
                else
                {
                    return false;
                }
            }

            inline constexpr const bool operator!=(const IntegerIterator& ano) const noexcept
            {
                if (!_has_next && !ano._has_next)
                {
                    return false;
                }
                else if (_has_next && ano._has_next)
                {
                    return _curr != ano._curr
                        || _last != ano._last
                        || _step != ano._step;
                }
                else
                {
                    return true;
                }
            }

        private:
            inline constexpr void next(void) noexcept
            {
                static const auto zero = static_cast<T>(0);

                assert(_step != zero);
                assert(_has_next == true);

                if (_step > zero)
                {
                    _curr = std::min(_curr + _step, _last);
                }
                else
                {
                    _curr = std::max(_curr + _step, _last);
                }
                if (_curr == _last)
                {
                    _has_next = false;
                }
            }

        private:
            bool _has_next;
            ValueType _curr;
            ValueType _last;
            ValueType _step;
        };

        template<typename T>
            requires std::unsigned_integral<T>
                && requires
                {
                    { static_cast<T>(0) } -> DecaySameAs<T>;
                    { static_cast<T>(1) } -> DecaySameAs<T>;
                }
        class IntegerIterator<T>
        {
        public:
            using ValueType = OriginType<T>;

        public:
            constexpr IntegerIterator(void)
                : _has_next(false), _curr(static_cast<ValueType>(0)), _last(static_cast<ValueType>(0)), _step(static_cast<ValueType>(0)), _reverse(false) {}

            constexpr IntegerIterator(ArgRRefType<ValueType> curr, ArgRRefType<ValueType> last, ArgRRefType<ValueType> step, const bool reverse = false)
                : _has_next(true), _curr(static_cast<ValueType>(0)), _last(static_cast<ValueType>(0)), _step(static_cast<ValueType>(0)), _reverse(reverse)
            {
                static const auto zero = static_cast<ValueType>(0);

                if (curr == last)
                {
                    _has_next = false;
                }
                else
                {
                    if (step == zero)
                    {
                        throw OSPFException{ OSPFErrCode::ApplicationError, "invalid step 0 for integer iterator" };
                    }
                    else if (reverse)
                    {
                        if (curr < last)
                        {
                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid integer range for iterator" };
                        }
                    }
                    else
                    {
                        if (curr > last)
                        {
                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid integer range for iterator" };
                        }
                    }

                    if (reverse)
                    {
                        _curr = move<ValueType>(last) - step;
                        _last = move<ValueType>(curr);
                        _step = move<ValueType>(step);
                    }
                    else
                    {
                        _curr = move<ValueType>(curr);
                        _last = move<ValueType>(last);
                        _step = move<ValueType>(step);
                    }
                }
            }

        public:
            constexpr IntegerIterator(const IntegerIterator& ano) = default;
            constexpr IntegerIterator(IntegerIterator&& ano) noexcept = default;
            constexpr IntegerIterator& operator=(const IntegerIterator& rhs) = default;
            constexpr IntegerIterator& operator=(IntegerIterator&& rhs) noexcept = default;
            constexpr ~IntegerIterator(void) noexcept = default;

        public:
            inline constexpr operator const bool(void) const noexcept
            {
                return _has_next;
            }

        public:
            inline constexpr ArgCLRefType<ValueType> operator*(void) const noexcept
            {
                return _curr;
            }

            inline constexpr const CPtrType<ValueType> operator->(void) const noexcept
            {
                return &_curr;
            }

        public:
            inline constexpr IntegerIterator& operator++(void) noexcept
            {
                next();
                return *this;
            }

            inline constexpr const IntegerIterator operator++(int) noexcept
            {
                auto ret = *this;
                next();
                return ret;
            }

        public:
            inline constexpr const bool operator==(const IntegerIterator& rhs) const noexcept
            {
                if (!_has_next && !rhs._has_next)
                {
                    return true;
                }
                else if (_has_next && rhs._has_next)
                {
                    return _reverse == rhs._reverse
                        && _curr == rhs._curr
                        && _last == rhs._last
                        && _step == rhs._step;
                }
                else
                {
                    return false;
                }
            }

            template<typename I>
            inline constexpr const bool operator!=(const IntegerIterator& rhs) const noexcept
            {
                if (!_has_next && !rhs._has_next)
                {
                    return false;
                }
                else if (_has_next && rhs._has_next)
                {
                    return _reverse != rhs._reverse
                        || _curr != rhs._curr
                        || _last != rhs._last
                        || _step != rhs._step;
                }
                else
                {
                    return true;
                }
            }

        private:
            inline constexpr void next(void) noexcept
            {
                static const auto zero = static_cast<T>(0);

                assert(_step != zero);
                assert(_has_next == true);

                if (_reverse)
                {
                    if (_curr < _step)
                    {
                        _has_next = false;
                    }
                    else
                    {
                        _curr = std::max(_curr - _step, _last);

                        if (_curr == _last)
                        {
                            _has_next = false;
                        }
                    }
                }
                else
                {
                    _curr = std::min(_curr + _step, _last);

                    if (_curr == _last)
                    {
                        _has_next = false;
                    }
                }
            }

        private:
            bool _has_next;
            bool _reverse;
            ValueType _curr;
            ValueType _last;
            ValueType _step;
        };

        extern template class IntegerIterator<i32>;
        extern template class IntegerIterator<u32>;
        extern template class IntegerIterator<usize>;
        extern template class IntegerIterator<isize>;
    };
};

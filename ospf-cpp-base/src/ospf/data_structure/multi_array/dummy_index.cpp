#include <ospf/data_structure/multi_array/dummy_index.hpp>

namespace ospf::data_structure::multi_array::dummy_index
{
    constexpr DummyIndexEnumerator::DummyIndexEnumerator(ArgRRefType<Continuous1> first, ArgRRefType<Continuous1> last)
        : _next(npos), _has_next(true), _curr(std::in_place_index<0_uz>, move<Continuous1>(first)), _end(std::in_place_index<0_uz>, move<Continuous1>(last))
    {
        if (_curr == _end)
        {
            _has_next = false;
        }
        _next = *std::get<0_uz>(_curr);
    }

    constexpr DummyIndexEnumerator::DummyIndexEnumerator(ArgRRefType<Continuous2> first, ArgRRefType<Continuous2> last, ArgRRefType<Transformer> transformer)
        : _next(npos), _has_next(true), _curr(std::in_place_index<1_uz>, move<Continuous2>(first)), _end(std::in_place_index<1_uz>, move<Continuous2>(last)), _transformer(move<Transformer>(transformer))
    {
        if (_curr == _end)
        {
            _has_next = false;
        }
        while (_curr != _end)
        {
            const auto index = (*_transformer)(*std::get<1_uz>(_curr));
            if (index.has_value())
            {
                _next = *index;
                break;
            }
            ++std::get<1_uz>(_curr);
        }
        if (_curr == _end)
        {
            _has_next = false;
        }
    }

    constexpr DummyIndexEnumerator::DummyIndexEnumerator(ArgRRefType<Discrete1> first, ArgRRefType<Discrete1> last)
        : _next(npos), _has_next(true), _curr(std::in_place_index<2_uz>, move<Discrete1>(first)), _end(std::in_place_index<2_uz>, move<Discrete1>(last))
    {
        if (_curr == _end)
        {
            _has_next = false;
        }
        _next = *std::get<0_uz>(_curr);
    }

    constexpr DummyIndexEnumerator::DummyIndexEnumerator(ArgRRefType<Discrete2> first, ArgRRefType<Discrete2> last, ArgRRefType<Transformer> transformer)
        : _next(npos), _has_next(true), _curr(std::in_place_index<3_uz>, move<Discrete2>(first)), _end(std::in_place_index<3_uz>, move<Discrete2>(last)), _transformer(move<Transformer>(transformer))
    {
        if (_curr == _end)
        {
            _has_next = false;
        }
        while (_curr != _end)
        {
            const auto index = (*_transformer)(*std::get<1_uz>(_curr));
            if (index.has_value())
            {
                _next = *index;
                break;
            }
            ++std::get<1_uz>(_curr);
        }
        if (_curr == _end)
        {
            _has_next = false;
        }
    }

    constexpr void DummyIndexEnumerator::next(void) noexcept
    {
        assert(_has_next);

        std::visit([this](auto& iter)
            {
                if constexpr (DecaySameAs<decltype(*iter), usize>)
                {
                    ++iter;
                    if (this->_curr == this->_end)
                    {
                        this->_has_next = false;
                    }
                    else
                    {
                        this->_next = *iter;
                    }
                }
                else if constexpr (DecaySameAs<decltype(*iter), isize>)
                {
                    while (true)
                    {
                        ++iter;
                        if (this->_curr == this->_end)
                        {
                            this->_has_next = false;
                            break;
                        }
                        else
                        {
                            const auto index = (*this->_transformer)(*iter);
                            if (index != std::nullopt)
                            {
                                this->_next = *index;
                                break;
                            }
                        }
                    }
                }
                //else
                //{
                //    static_assert(false, "non-exhaustive visitor!");
                //}
            }, _curr);
    }
};
